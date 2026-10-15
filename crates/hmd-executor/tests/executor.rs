// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 008 acceptance tests with a fake fix implementation and an in-memory journal that applies
//! the domain transition rules.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use hmd_catalog::embedded;
use hmd_core::ProbeOutcome;
use hmd_domain::{ExecutionState, ReportTrigger, Timestamp, execution};
use hmd_executor::{
    ActionApproval, EvaluatedPlan, ExecError, ExecuteInput, ExecutionStart, FixImpl, FixOutcome,
    Journal, JournalError, Registry, execute_step, plan_hash, verify_step,
};
use hmd_policy::{
    ActionPlanProposal, AutonomyMode, Confidence, Platform, PolicyContext, PolicyDecision,
    ProposedStep, Rollback, evaluate,
};
use hmd_probes::{Finding, ProbeOutput, Severity};

const NOW: Timestamp = 1_760_400_000;
const FIX: &str = "net.dns.flush_cache";

#[derive(Default)]
struct MemJournal {
    rows: BTreeMap<i64, (String, ExecutionState)>,
    history: Vec<(i64, ExecutionState)>,
}

impl Journal for MemJournal {
    fn key_used(&self, key: &str) -> Result<bool, JournalError> {
        Ok(self.rows.values().any(|(k, _)| k == key))
    }
    fn begin(&mut self, start: &ExecutionStart<'_>) -> Result<i64, JournalError> {
        let id = i64::try_from(self.rows.len()).unwrap() + 1;
        self.rows
            .insert(id, (start.idempotency_key.clone(), ExecutionState::Started));
        self.history.push((id, ExecutionState::Started));
        Ok(id)
    }
    fn advance(
        &mut self,
        id: i64,
        to: ExecutionState,
        _note: Option<&str>,
        _at: Timestamp,
    ) -> Result<(), JournalError> {
        let row = self
            .rows
            .get_mut(&id)
            .ok_or(JournalError("missing".into()))?;
        row.1 = execution::transition(row.1, to).map_err(|e| JournalError(e.to_string()))?;
        self.history.push((id, to));
        Ok(())
    }
}

type Shared = Arc<Mutex<MemJournal>>;

/// Journal handle shared with the fake implementation (to observe write-ahead).
struct SharedJournal(Shared);

impl Journal for SharedJournal {
    fn key_used(&self, key: &str) -> Result<bool, JournalError> {
        self.0.lock().unwrap().key_used(key)
    }
    fn begin(&mut self, start: &ExecutionStart<'_>) -> Result<i64, JournalError> {
        self.0.lock().unwrap().begin(start)
    }
    fn advance(
        &mut self,
        id: i64,
        to: ExecutionState,
        note: Option<&str>,
        at: Timestamp,
    ) -> Result<(), JournalError> {
        self.0.lock().unwrap().advance(id, to, note, at)
    }
}

struct Fake {
    outcome: FixOutcome,
    rollback: FixOutcome,
    /// When set, `run` asserts the journal already says `started` (US2.1).
    watch: Option<Shared>,
    runs: Arc<Mutex<u32>>,
}

impl Fake {
    fn new(outcome: FixOutcome) -> Fake {
        Fake {
            outcome,
            rollback: FixOutcome::Succeeded,
            watch: None,
            runs: Arc::new(Mutex::new(0)),
        }
    }
}

impl FixImpl for Fake {
    fn run(&self, _step: &ProposedStep) -> FixOutcome {
        if let Some(j) = &self.watch {
            let j = j.lock().unwrap();
            assert!(
                j.rows.values().any(|(_, s)| *s == ExecutionState::Started),
                "started must be written before the implementation runs"
            );
        }
        *self.runs.lock().unwrap() += 1;
        self.outcome
    }
    fn rollback(&self, _step: &ProposedStep) -> FixOutcome {
        self.rollback
    }
}

fn proposal(steps: &[&str]) -> ActionPlanProposal {
    let catalog = embedded().unwrap();
    ActionPlanProposal {
        session_id: "s-1".into(),
        steps: steps
            .iter()
            .enumerate()
            .map(|(i, cap)| ProposedStep {
                step_id: format!("s{}", i + 1),
                capability: (*cap).to_string(),
                version: catalog
                    .catalog
                    .get(cap)
                    .map(|c| c.version.clone())
                    .unwrap_or_default(),
                params: vec![],
            })
            .collect(),
    }
}

fn decide(p: &ActionPlanProposal) -> PolicyDecision {
    let catalog = embedded().unwrap();
    let mut ctx = PolicyContext::new(Platform::Linux);
    ctx.findings.insert("net.dns.resolution_failing".into());
    ctx.confidence = Confidence::Probable;
    evaluate(p, &catalog.catalog, &ctx).unwrap()
}

struct Fixture {
    proposal: ActionPlanProposal,
    decision: PolicyDecision,
}

impl Fixture {
    fn new() -> Fixture {
        let proposal = proposal(&[FIX]);
        let decision = decide(&proposal);
        assert!(decision.verdicts[0].is_accepted(), "{decision:?}");
        Fixture { proposal, decision }
    }
    fn plan(&self) -> EvaluatedPlan<'_> {
        EvaluatedPlan {
            proposal: &self.proposal,
            decision: &self.decision,
            catalog_version: &embedded().unwrap().version,
        }
    }
    fn approve(&self, nonce: i64) -> ActionApproval {
        ActionApproval::new(
            &self.plan(),
            ["s1".to_string()],
            AutonomyMode::AskAlways,
            NOW,
            nonce,
        )
    }
}

fn input<'a>(
    plan: EvaluatedPlan<'a>,
    approval: &'a ActionApproval,
    step: &'a str,
    now: Timestamp,
) -> ExecuteInput<'a> {
    ExecuteInput {
        plan,
        plan_ref: 1,
        approval,
        step_id: step,
        platform: Platform::Linux,
        now,
    }
}

#[test]
fn us1_1_hash_is_stable_and_sensitive_to_every_change() {
    let f = Fixture::new();
    let v = &embedded().unwrap().version;
    let h = plan_hash(&f.proposal, &f.decision, v);
    assert!(h.starts_with("sha256:") && h.len() == 7 + 64);
    assert_eq!(h, plan_hash(&f.proposal, &f.decision, v));
    assert_ne!(h, plan_hash(&f.proposal, &f.decision, "2099.01.0"));

    let mut other = f.proposal.clone();
    other.steps[0].params.push(("x".into(), "1".into()));
    assert_ne!(h, plan_hash(&other, &f.decision, v));

    let two = proposal(&[FIX, "net.dns.resolve"]);
    let mut swapped = two.clone();
    swapped.steps.swap(0, 1);
    assert_ne!(
        plan_hash(&two, &decide(&two), v),
        plan_hash(&swapped, &decide(&swapped), v),
        "reordering changes the hash"
    );
}

#[test]
fn us1_2_other_plan_is_refused_and_nothing_runs() {
    let f = Fixture::new();
    let approval = f.approve(1);
    let mut changed = f.proposal.clone();
    changed.session_id = "s-2".into();
    let plan = EvaluatedPlan {
        proposal: &changed,
        ..f.plan()
    };
    let fake = Fake::new(FixOutcome::Succeeded);
    let runs = fake.runs.clone();
    let registry = Registry::default().with(FIX, Box::new(fake));
    let mut j = MemJournal::default();
    let err = execute_step(&input(plan, &approval, "s1", NOW), &registry, &mut j).unwrap_err();
    assert_eq!(err, ExecError::PlanHashMismatch);
    assert!(j.rows.is_empty());
    assert_eq!(*runs.lock().unwrap(), 0);
}

#[test]
fn us1_3_expired_approval() {
    let f = Fixture::new();
    let approval = f.approve(1);
    let registry = Registry::default().with(FIX, Box::new(Fake::new(FixOutcome::Succeeded)));
    let mut j = MemJournal::default();
    let late = input(f.plan(), &approval, "s1", NOW + 600);
    assert_eq!(
        execute_step(&late, &registry, &mut j).unwrap_err(),
        ExecError::ApprovalExpired
    );
    let early = input(f.plan(), &approval, "s1", NOW - 1);
    assert_eq!(
        execute_step(&early, &registry, &mut j).unwrap_err(),
        ExecError::ApprovalExpired,
        "clock before the approval is not trusted"
    );
    assert!(j.rows.is_empty());
}

#[test]
fn us1_4_step_not_approved_or_not_accepted() {
    let f = Fixture::new();
    let none = ActionApproval::new(&f.plan(), [], AutonomyMode::AskAlways, NOW, 1);
    let registry = Registry::default().with(FIX, Box::new(Fake::new(FixOutcome::Succeeded)));
    let mut j = MemJournal::default();
    assert_eq!(
        execute_step(&input(f.plan(), &none, "s1", NOW), &registry, &mut j).unwrap_err(),
        ExecError::StepNotApproved
    );

    // A step the Policy Engine rejected cannot be approved into running.
    let p = proposal(&[FIX]);
    let catalog = embedded().unwrap();
    let ctx = PolicyContext::new(Platform::Linux); // no findings → precondition unmet
    let rejected = evaluate(&p, &catalog.catalog, &ctx).unwrap();
    assert!(!rejected.verdicts[0].is_accepted());
    let plan = EvaluatedPlan {
        proposal: &p,
        decision: &rejected,
        catalog_version: &catalog.version,
    };
    let approval = ActionApproval::new(&plan, ["s1".into()], AutonomyMode::AskAlways, NOW, 2);
    assert_eq!(
        execute_step(&input(plan, &approval, "s1", NOW), &registry, &mut j).unwrap_err(),
        ExecError::StepNotApproved
    );
    assert!(j.rows.is_empty());
}

#[test]
fn us1_5_same_approval_runs_a_step_once() {
    let f = Fixture::new();
    let approval = f.approve(7);
    let fake = Fake::new(FixOutcome::Succeeded);
    let runs = fake.runs.clone();
    let registry = Registry::default().with(FIX, Box::new(fake));
    let mut j = MemJournal::default();
    let run = execute_step(&input(f.plan(), &approval, "s1", NOW), &registry, &mut j).unwrap();
    assert_eq!(run.state, ExecutionState::Succeeded);
    assert_eq!(
        execute_step(
            &input(f.plan(), &approval, "s1", NOW + 1),
            &registry,
            &mut j
        )
        .unwrap_err(),
        ExecError::DuplicateExecution
    );
    assert_eq!(*runs.lock().unwrap(), 1);
    // A new approval (new nonce) is a new execution.
    let again = f.approve(8);
    execute_step(&input(f.plan(), &again, "s1", NOW + 2), &registry, &mut j).unwrap();
    assert_eq!(*runs.lock().unwrap(), 2);
}

#[test]
fn production_registry_has_no_system_changing_fix() {
    let f = Fixture::new();
    let approval = f.approve(1);
    let mut j = MemJournal::default();
    assert_eq!(
        execute_step(
            &input(f.plan(), &approval, "s1", NOW),
            &Registry::production(),
            &mut j
        )
        .unwrap_err(),
        ExecError::NotImplemented
    );
    assert!(
        j.rows.is_empty(),
        "NOT_IMPLEMENTED is decided before `started`"
    );
}

#[test]
fn us2_1_started_is_written_before_the_fix_runs() {
    let f = Fixture::new();
    let approval = f.approve(1);
    let shared: Shared = Arc::new(Mutex::new(MemJournal::default()));
    let mut fake = Fake::new(FixOutcome::Unknown);
    fake.watch = Some(shared.clone());
    let registry = Registry::default().with(FIX, Box::new(fake));
    let mut j = SharedJournal(shared.clone());
    let run = execute_step(&input(f.plan(), &approval, "s1", NOW), &registry, &mut j).unwrap();
    assert_eq!(run.state, ExecutionState::Unknown);
    assert_eq!(
        shared.lock().unwrap().history,
        vec![(1, ExecutionState::Started), (1, ExecutionState::Unknown)]
    );
}

fn variant() -> hmd_policy::PlatformVariant {
    embedded().unwrap().catalog.get(FIX).unwrap().variants[&Platform::Linux].clone()
}

fn dns_probe(still_failing: bool) -> ProbeOutcome {
    ProbeOutcome::Ran(ProbeOutput {
        probe_id: "net.dns.resolve",
        findings: if still_failing {
            vec![Finding {
                code: "net.dns.resolution_failing",
                severity: Severity::Warning,
            }]
        } else {
            vec![]
        },
        evidence: BTreeMap::new(),
    })
}

fn succeeded_run(registry: &Registry, j: &mut MemJournal, f: &Fixture) -> i64 {
    let approval = f.approve(1);
    execute_step(&input(f.plan(), &approval, "s1", NOW), registry, j)
        .unwrap()
        .execution
}

#[test]
fn us3_2_verified_when_the_problem_finding_is_gone() {
    let f = Fixture::new();
    let registry = Registry::default().with(FIX, Box::new(Fake::new(FixOutcome::Succeeded)));
    let mut j = MemJournal::default();
    let id = succeeded_run(&registry, &mut j, &f);
    let mut asked = String::new();
    let v = verify_step(
        id,
        &f.proposal.steps[0],
        &variant(),
        &registry,
        &mut j,
        |probe| {
            asked = probe.to_string();
            dns_probe(false)
        },
        NOW + 5,
    )
    .unwrap();
    assert_eq!(asked, "net.dns.resolve", "the catalog verification probe");
    assert_eq!(v.state, ExecutionState::VerifiedOk);
    assert_eq!(v.report, None);
}

#[test]
fn us3_3_still_failing_or_probe_failed_is_rep03() {
    let f = Fixture::new();
    let registry = Registry::default().with(FIX, Box::new(Fake::new(FixOutcome::Succeeded)));
    for outcome in [dns_probe(true), ProbeOutcome::Failed("TIMEOUT")] {
        let mut j = MemJournal::default();
        let id = succeeded_run(&registry, &mut j, &f);
        let v = verify_step(
            id,
            &f.proposal.steps[0],
            &variant(),
            &registry,
            &mut j,
            |_| outcome.clone(),
            NOW + 5,
        )
        .unwrap();
        // DNS flush declares rollback "not_needed": nothing to undo.
        assert_eq!(v.state, ExecutionState::VerifiedFailed);
        assert_eq!(v.report, Some(ReportTrigger::VerificationFailed));
    }
}

#[test]
fn us3_3_rollback_when_the_catalog_implements_it() {
    let f = Fixture::new();
    let registry = Registry::default().with(FIX, Box::new(Fake::new(FixOutcome::Succeeded)));
    let mut j = MemJournal::default();
    let id = succeeded_run(&registry, &mut j, &f);
    let mut v = variant();
    v.rollback = Rollback::Implemented;
    let result = verify_step(
        id,
        &f.proposal.steps[0],
        &v,
        &registry,
        &mut j,
        |_| dns_probe(true),
        NOW + 5,
    )
    .unwrap();
    assert_eq!(result.state, ExecutionState::RolledBack);
    assert_eq!(result.report, Some(ReportTrigger::VerificationFailed));
}

#[test]
fn verification_of_a_failed_execution_is_refused_by_the_journal() {
    let f = Fixture::new();
    let registry = Registry::default().with(FIX, Box::new(Fake::new(FixOutcome::Failed("X"))));
    let mut j = MemJournal::default();
    let id = succeeded_run(&registry, &mut j, &f);
    let err = verify_step(
        id,
        &f.proposal.steps[0],
        &variant(),
        &registry,
        &mut j,
        |_| dns_probe(false),
        NOW,
    )
    .unwrap_err();
    assert_eq!(err.reason_code(), "JOURNAL_ERROR");
}
