// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 012 acceptance tests: the use cases end to end with fake probes and fake fixes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

use hmd_app::{App, AppError, ExecutionOutcome};
use hmd_domain::{DAY, ProblemFamily, ReportTrigger, TicketState, Timestamp};
use hmd_executor::{FixImpl, FixOutcome, Registry};
use hmd_policy::{AutonomyMode, Platform, ProposedStep};
use hmd_probes::{Finding, ProbeError, ProbeOutput, Severity};

const NOW: Timestamp = 1_760_400_000;
const FIX: &str = "net.dns.flush_cache";

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("hmd-app-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }
    fn db(&self) -> PathBuf {
        self.0.join("history.sqlite3")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// DNS fails until `fixed` is set; everything else is fine.
fn dns_probe(fixed: &AtomicBool) -> impl Fn(&str) -> Result<ProbeOutput, ProbeError> + '_ {
    move |id: &str| {
        let findings = if id == "net.dns.resolve" && !fixed.load(SeqCst) {
            vec![Finding {
                code: "net.dns.resolution_failing",
                severity: Severity::Critical,
            }]
        } else {
            vec![]
        };
        Ok(ProbeOutput {
            probe_id: "fake",
            findings,
            evidence: BTreeMap::new(),
        })
    }
}

fn quiet(_: &str) -> Result<ProbeOutput, ProbeError> {
    Ok(ProbeOutput {
        probe_id: "fake",
        findings: vec![],
        evidence: BTreeMap::new(),
    })
}

struct Fake<'a> {
    outcome: FixOutcome,
    fixes: Option<&'a AtomicBool>,
}

impl FixImpl for Fake<'static> {
    fn run(&self, _: &ProposedStep) -> FixOutcome {
        if let Some(f) = self.fixes {
            f.store(true, SeqCst);
        }
        self.outcome
    }
}

fn app() -> App {
    App::open_in_memory(NOW)
        .unwrap()
        .0
        .with_platform(Platform::Linux)
}

fn leak(b: bool) -> &'static AtomicBool {
    Box::leak(Box::new(AtomicBool::new(b)))
}

#[test]
fn us1_same_family_ticket_is_continued_not_duplicated() {
    let mut a = app();
    let first = a.start_ticket(ProblemFamily::Storage, NOW).unwrap();
    assert!(!first.reused);
    let again = a.start_ticket(ProblemFamily::Storage, NOW + 1).unwrap();
    assert!(again.reused && !again.reopened);
    assert_eq!(again.ticket.id, first.ticket.id);
    let other = a.start_ticket(ProblemFamily::Connectivity, NOW).unwrap();
    assert_ne!(other.ticket.id, first.ticket.id);
    assert_eq!(a.history().unwrap().len(), 2);
}

#[test]
fn us2_dns_problem_proposes_a_plan_and_waits_for_the_person() {
    let mut a = app();
    let t = a
        .start_ticket(ProblemFamily::Connectivity, NOW)
        .unwrap()
        .ticket;
    let fixed = AtomicBool::new(false);
    let view = a.diagnose(&t.id, NOW, dns_probe(&fixed)).unwrap();
    assert_eq!(view.diagnosis.diagnosis_id, "net.dns");
    let plan = view.plan.expect("plan");
    assert!(plan.plan_hash.starts_with("sha256:"));
    assert_eq!(plan.steps.len(), 1);
    let step = &plan.steps[0];
    assert_eq!(step.capability, FIX);
    assert_eq!(step.title, "Limpar a memória de endereços da internet");
    assert!(step.per_action_approval && step.requires_elevation);
    assert!(!step.risk.is_empty() && !step.undo.is_empty());
    assert_eq!(view.ticket.state, TicketState::AwaitingUser);
}

#[test]
fn us3_nothing_runs_without_approval_and_production_has_no_fix_yet() {
    let mut a = app();
    let t = a
        .start_ticket(ProblemFamily::Connectivity, NOW)
        .unwrap()
        .ticket;
    let fixed = AtomicBool::new(false);
    let plan = a
        .diagnose(&t.id, NOW, dns_probe(&fixed))
        .unwrap()
        .plan
        .unwrap();
    // The ticket is awaiting the person: executing now is refused.
    assert!(matches!(
        a.execute(&t.id, plan.plan_id, NOW, dns_probe(&fixed)),
        Err(AppError::InvalidState("awaiting_user"))
    ));
    a.approve(&t.id, plan.plan_id, &["s1"], NOW + 5).unwrap();
    let outcome = a
        .execute(&t.id, plan.plan_id, NOW + 6, dns_probe(&fixed))
        .unwrap();
    assert_eq!(
        outcome,
        ExecutionOutcome::NotAvailable {
            step_id: "s1".into()
        }
    );
    assert!(!fixed.load(SeqCst));
    // Guidance + report remain.
    let (_, text) = a
        .make_report(&t.id, Some("sem internet"), None, NOW + 7, quiet)
        .unwrap();
    assert!(text.contains("O HelpMyDesk não alterou nada"));
    let after = a
        .history()
        .unwrap()
        .into_iter()
        .find(|x| x.id == t.id)
        .unwrap();
    assert_eq!(after.state, TicketState::Unresolved);
}

#[test]
fn us3_verified_fix_needs_the_persons_confirmation() {
    let fixed = leak(false);
    let mut a = app().with_registry(Registry::default().with(
        FIX,
        Box::new(Fake {
            outcome: FixOutcome::Succeeded,
            fixes: Some(fixed),
        }),
    ));
    let t = a
        .start_ticket(ProblemFamily::Connectivity, NOW)
        .unwrap()
        .ticket;
    let plan = a
        .diagnose(&t.id, NOW, dns_probe(fixed))
        .unwrap()
        .plan
        .unwrap();
    a.approve(&t.id, plan.plan_id, &["s1"], NOW + 5).unwrap();
    let outcome = a
        .execute(&t.id, plan.plan_id, NOW + 6, dns_probe(fixed))
        .unwrap();
    assert_eq!(outcome, ExecutionOutcome::Verified);
    let t2 = a
        .history()
        .unwrap()
        .into_iter()
        .find(|x| x.id == t.id)
        .unwrap();
    assert_eq!(t2.state, TicketState::ResolvedPendingConfirmation);
    let done = a.confirm(&t.id, true, NOW + 10).unwrap();
    assert_eq!(done.state, TicketState::Resolved);
    assert!(a.audit_intact().unwrap());
}

#[test]
fn us3_fix_that_does_not_help_is_rep03() {
    let mut a = app().with_registry(Registry::default().with(
        FIX,
        Box::new(Fake {
            outcome: FixOutcome::Succeeded,
            fixes: None,
        }),
    ));
    let t = a
        .start_ticket(ProblemFamily::Connectivity, NOW)
        .unwrap()
        .ticket;
    let still = AtomicBool::new(false);
    let plan = a
        .diagnose(&t.id, NOW, dns_probe(&still))
        .unwrap()
        .plan
        .unwrap();
    a.approve(&t.id, plan.plan_id, &["s1"], NOW + 5).unwrap();
    let outcome = a
        .execute(&t.id, plan.plan_id, NOW + 6, dns_probe(&still))
        .unwrap();
    assert_eq!(
        outcome,
        ExecutionOutcome::NotResolved {
            trigger: ReportTrigger::VerificationFailed
        }
    );
    let t2 = a
        .history()
        .unwrap()
        .into_iter()
        .find(|x| x.id == t.id)
        .unwrap();
    assert_eq!(t2.state, TicketState::Unresolved);
    assert_eq!(t2.last_trigger, Some(ReportTrigger::VerificationFailed));
    // Spec 016: the report must not claim that nothing was changed.
    let (_, text) = a
        .make_report(&t.id, None, None, NOW + 7, dns_probe(&still))
        .unwrap();
    assert!(!text.contains("não alterou nada"), "{text}");
    assert!(
        text.contains("aplicou 1 correção aprovada pela pessoa"),
        "{text}"
    );
}

#[test]
fn us3_unknown_outcome_is_rep04_and_blocks_the_fix_next_time() {
    let mut a = app().with_registry(Registry::default().with(
        FIX,
        Box::new(Fake {
            outcome: FixOutcome::Unknown,
            fixes: None,
        }),
    ));
    let t = a
        .start_ticket(ProblemFamily::Connectivity, NOW)
        .unwrap()
        .ticket;
    let still = AtomicBool::new(false);
    let plan = a
        .diagnose(&t.id, NOW, dns_probe(&still))
        .unwrap()
        .plan
        .unwrap();
    a.approve(&t.id, plan.plan_id, &["s1"], NOW + 5).unwrap();
    assert_eq!(
        a.execute(&t.id, plan.plan_id, NOW + 6, dns_probe(&still))
            .unwrap(),
        ExecutionOutcome::NotResolved {
            trigger: ReportTrigger::UnknownExecutionState
        }
    );
    // Trying again: the same fix is not offered while its last run is `unknown`.
    let again = a
        .start_ticket(ProblemFamily::Connectivity, NOW + 7)
        .unwrap();
    assert!(again.reopened);
    let view = a
        .diagnose(&again.ticket.id, NOW + 8, dns_probe(&still))
        .unwrap();
    assert!(view.plan.is_none());
}

#[test]
fn us4_repeated_reopening_offers_the_report_rep08() {
    let mut a = app();
    let t = a
        .start_ticket(ProblemFamily::Performance, NOW)
        .unwrap()
        .ticket;
    for i in 0..2 {
        a.diagnose(&t.id, NOW + i, quiet).unwrap();
        a.make_report(&t.id, None, None, NOW + i, quiet).unwrap();
        let again = a
            .start_ticket(ProblemFamily::Performance, NOW + i + DAY)
            .unwrap();
        assert!(again.reopened);
    }
    let view = a.diagnose(&t.id, NOW + 2 * DAY, quiet).unwrap();
    assert_eq!(view.trigger, Some(ReportTrigger::RepeatedReopen));
}

#[test]
fn decline_keeps_the_ticket_open_for_guidance() {
    let mut a = app();
    let t = a
        .start_ticket(ProblemFamily::Connectivity, NOW)
        .unwrap()
        .ticket;
    let still = AtomicBool::new(false);
    let plan = a
        .diagnose(&t.id, NOW, dns_probe(&still))
        .unwrap()
        .plan
        .unwrap();
    a.decline(&t.id, plan.plan_id, NOW + 1).unwrap();
    assert!(matches!(
        a.approve(&t.id, plan.plan_id, &["s1"], NOW + 2),
        Err(AppError::NoPendingPlan)
    ));
    let t2 = a
        .history()
        .unwrap()
        .into_iter()
        .find(|x| x.id == t.id)
        .unwrap();
    assert_eq!(t2.state, TicketState::InProgress);
}

#[test]
fn startup_applies_time_rules() {
    let dir = TempDir::new("startup");
    {
        let (mut a, s) = App::open(&dir.db(), NOW).unwrap();
        assert_eq!(s.housekept, 0);
        let t = a.start_ticket(ProblemFamily::Storage, NOW).unwrap().ticket;
        a.diagnose(&t.id, NOW, quiet).unwrap();
        a.make_report(&t.id, None, None, NOW, quiet).unwrap();
    }
    // 30 days later the unresolved ticket closes by itself.
    let (a, s) = App::open(&dir.db(), NOW + 31 * DAY).unwrap();
    assert_eq!(s.housekept, 1);
    assert_eq!(a.history().unwrap()[0].state, TicketState::Closed);
}

#[test]
fn settings_consent_export_and_erase() {
    let dir = TempDir::new("privacy");
    let (mut a, _) = App::open(&dir.db(), NOW).unwrap();
    assert_eq!(a.autonomy().unwrap(), AutonomyMode::AskAlways);
    a.set_autonomy(AutonomyMode::ObserveOnly, NOW).unwrap();
    assert_eq!(a.autonomy().unwrap(), AutonomyMode::ObserveOnly);
    a.set_consent("startup_apps", true, NOW).unwrap();
    assert!(a.consents().unwrap().contains("startup_apps"));
    assert!(a.set_consent("everything", true, NOW).is_err());
    assert!(
        a.export_all(NOW)
            .unwrap()
            .contains("`startup_apps` concedida")
    );
    let removed = a.erase_all(&dir.db()).unwrap();
    assert!(!removed.is_empty());
    assert!(!dir.db().exists());
}

#[test]
fn observe_only_never_proposes_fixes() {
    let mut a = app();
    a.set_autonomy(AutonomyMode::ObserveOnly, NOW).unwrap();
    let t = a
        .start_ticket(ProblemFamily::Connectivity, NOW)
        .unwrap()
        .ticket;
    let still = AtomicBool::new(false);
    let view = a.diagnose(&t.id, NOW, dns_probe(&still)).unwrap();
    assert!(view.plan.is_none(), "LEVEL_EXCEEDS_MODE → guidance only");
    assert_ne!(view.ticket.state, TicketState::AwaitingUser);
}
