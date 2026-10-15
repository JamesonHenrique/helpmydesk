// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 008 with the real SQLite journal: migration v1 → v2, approvals, write-ahead journal,
//! interrupted executions and the full approve → run → verify → ticket flow.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::PathBuf;

use hmd_catalog::embedded;
use hmd_core::ProbeOutcome;
use hmd_domain::{
    DAY, ExecutionState, ProblemFamily, ReportTrigger, RetentionConfig, TicketEvent, TicketState,
    Timestamp,
};
use hmd_executor::{
    EvaluatedPlan, ExecError, ExecuteInput, ExecutionStart, FixImpl, FixOutcome, Journal, Registry,
    execute_step, idempotency_key, verify_step,
};
use hmd_policy::{
    ActionPlanProposal, AutonomyMode, Confidence, Platform, PolicyContext, PolicyDecision,
    ProposedStep, RejectReason, StepVerdict, evaluate,
};
use hmd_probes::{Finding, ProbeOutput, Severity};
use hmd_store::{OpenOutcome, SCHEMA_VERSION, Store};

const NOW: Timestamp = 1_760_400_000;
const FIX: &str = "net.dns.flush_cache";

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("hmd-journal-{}-{name}", std::process::id()));
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

struct Fake(FixOutcome);

impl FixImpl for Fake {
    fn run(&self, _step: &ProposedStep) -> FixOutcome {
        self.0
    }
}

fn ctx(findings: &[&str]) -> PolicyContext {
    let mut c = PolicyContext::new(Platform::Linux);
    c.findings = findings.iter().map(|f| (*f).to_string()).collect();
    c.confidence = Confidence::Probable;
    c
}

fn plan_for(session: &str, c: &PolicyContext) -> (ActionPlanProposal, PolicyDecision) {
    let catalog = embedded().unwrap();
    let proposal = ActionPlanProposal {
        session_id: session.into(),
        steps: vec![ProposedStep {
            step_id: "s1".into(),
            capability: FIX.into(),
            version: catalog.catalog.get(FIX).unwrap().version.clone(),
            params: vec![],
        }],
    };
    let decision = evaluate(&proposal, &catalog.catalog, c).unwrap();
    (proposal, decision)
}

fn evaluated<'a>(p: &'a ActionPlanProposal, d: &'a PolicyDecision) -> EvaluatedPlan<'a> {
    EvaluatedPlan {
        proposal: p,
        decision: d,
        catalog_version: &embedded().unwrap().version,
    }
}

fn dns(still_failing: bool) -> ProbeOutcome {
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

/// SC-002: a database written with schema v1 migrates to v2, keeping data and a backup copy.
#[test]
fn sc002_v1_database_migrates_with_backup() {
    let dir = TempDir::new("migrate");
    {
        let conn = rusqlite::Connection::open(dir.db()).unwrap();
        conn.execute_batch(hmd_store::schema::V1).unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        conn.execute(
            "INSERT INTO ticket (family, state, reopen_count, created_at, last_changed_at)
             VALUES ('storage', 'open', 0, 1, 1)",
            [],
        )
        .unwrap();
    }
    let (store, outcome) = Store::open(&dir.db(), NOW).unwrap();
    let OpenOutcome::Migrated { from, backup } = outcome else {
        panic!("expected migration, got {outcome:?}");
    };
    assert_eq!(from, 1);
    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(store.tickets().unwrap().len(), 1, "data kept");
    let old = rusqlite::Connection::open(&backup).unwrap();
    let v: i32 = old
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(v, 1, "backup is the v1 file");
    let n: i64 = old
        .query_row("SELECT count(*) FROM ticket", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1);
}

#[test]
fn approval_is_stored_and_bound_to_the_stored_plan() {
    let mut s = Store::open_in_memory().unwrap();
    let t = s.create_ticket(ProblemFamily::Connectivity, NOW).unwrap();
    let (p, d) = plan_for("sess", &ctx(&["net.dns.resolution_failing"]));
    let plan = evaluated(&p, &d);
    let id = s.save_plan(&t.id, &plan, NOW).unwrap();
    assert_eq!(s.save_plan(&t.id, &plan, NOW).unwrap(), id, "stored once");
    let a1 = s
        .approve(id, &plan, &["s1"], AutonomyMode::AskAlways, NOW)
        .unwrap();
    let a2 = s
        .approve(id, &plan, &["s1"], AutonomyMode::AskAlways, NOW)
        .unwrap();
    assert_ne!(a1.nonce, a2.nonce, "nonce unique per approval");
    assert_eq!(s.approval(a1.nonce).unwrap(), Some(a1.clone()));

    let (other_p, other_d) = plan_for("other", &ctx(&["net.dns.resolution_failing"]));
    assert!(
        s.approve(
            id,
            &evaluated(&other_p, &other_d),
            &["s1"],
            AutonomyMode::AskAlways,
            NOW
        )
        .is_err(),
        "an approval must be of the stored plan"
    );
}

#[test]
fn us2_interrupted_execution_becomes_unknown_and_blocks_the_capability() {
    let mut s = Store::open_in_memory().unwrap();
    let t = s.create_ticket(ProblemFamily::Connectivity, NOW).unwrap();
    let c = ctx(&["net.dns.resolution_failing"]);
    let (p, d) = plan_for("sess", &c);
    let plan = evaluated(&p, &d);
    let plan_id = s.save_plan(&t.id, &plan, NOW).unwrap();
    let a = s
        .approve(plan_id, &plan, &["s1"], AutonomyMode::AskAlways, NOW)
        .unwrap();
    // The app "dies" right after the write-ahead record.
    let key = idempotency_key("sess", "s1", a.nonce);
    let exec = s
        .begin(&ExecutionStart {
            plan_ref: plan_id,
            approval_nonce: a.nonce,
            step_id: "s1",
            capability: format!("{FIX}@1.0.0"),
            platform: Platform::Linux,
            idempotency_key: key.clone(),
            at: NOW,
        })
        .unwrap();
    assert!(s.key_used(&key).unwrap());

    assert_eq!(s.recover_interrupted(NOW + 60).unwrap(), 1);
    let rows = s.executions(plan_id).unwrap();
    assert_eq!(rows[0].state, ExecutionState::Unknown);
    assert_eq!(rows[0].result.as_deref(), Some("INTERRUPTED"));

    // US2.3: the Policy Engine now refuses the same capability.
    let unknown = s.unknown_capabilities().unwrap();
    assert!(unknown.contains(FIX));
    let mut blocked = c.clone();
    blocked.counters.unknown_executions = unknown;
    let (_, d2) = plan_for("sess-2", &blocked);
    assert!(matches!(
        &d2.verdicts[0],
        StepVerdict::Rejected {
            reason: RejectReason::PendingUnknownExecution,
            ..
        }
    ));

    // The same approval cannot run the step again, even through the executor.
    let registry = Registry::default().with(FIX, Box::new(Fake(FixOutcome::Succeeded)));
    let input = ExecuteInput {
        plan,
        plan_ref: plan_id,
        approval: &a,
        step_id: "s1",
        platform: Platform::Linux,
        now: NOW + 61,
    };
    assert_eq!(
        execute_step(&input, &registry, &mut s).unwrap_err(),
        ExecError::DuplicateExecution
    );

    // Only a clarification leaves `unknown`.
    s.clarify_unknown(exec, false, NOW + 120).unwrap();
    assert_eq!(
        s.executions(plan_id).unwrap()[0].state,
        ExecutionState::Failed
    );
    assert!(s.unknown_capabilities().unwrap().is_empty());
    assert!(s.clarify_unknown(exec, true, NOW + 121).is_err());
}

#[test]
fn idempotency_key_is_unique_in_the_database() {
    let mut s = Store::open_in_memory().unwrap();
    let t = s.create_ticket(ProblemFamily::Connectivity, NOW).unwrap();
    let (p, d) = plan_for("sess", &ctx(&["net.dns.resolution_failing"]));
    let plan = evaluated(&p, &d);
    let plan_id = s.save_plan(&t.id, &plan, NOW).unwrap();
    let a = s
        .approve(plan_id, &plan, &["s1"], AutonomyMode::AskAlways, NOW)
        .unwrap();
    let start = ExecutionStart {
        plan_ref: plan_id,
        approval_nonce: a.nonce,
        step_id: "s1",
        capability: format!("{FIX}@1.0.0"),
        platform: Platform::Linux,
        idempotency_key: "k".into(),
        at: NOW,
    };
    s.begin(&start).unwrap();
    assert!(s.begin(&start).is_err(), "UNIQUE backstop");
}

fn full_flow(still_failing: bool) -> (Store, String, i64) {
    let catalog = embedded().unwrap();
    let mut s = Store::open_in_memory().unwrap();
    let t = s.create_ticket(ProblemFamily::Connectivity, NOW).unwrap();
    s.apply(&t.id, TicketEvent::StartSession, NOW).unwrap();
    let (p, d) = plan_for("sess", &ctx(&["net.dns.resolution_failing"]));
    let plan = evaluated(&p, &d);
    let plan_id = s.save_plan(&t.id, &plan, NOW).unwrap();
    let a = s
        .approve(plan_id, &plan, &["s1"], AutonomyMode::AskAlways, NOW + 10)
        .unwrap();
    let registry = Registry::default().with(FIX, Box::new(Fake(FixOutcome::Succeeded)));
    let input = ExecuteInput {
        plan,
        plan_ref: plan_id,
        approval: &a,
        step_id: "s1",
        platform: Platform::Linux,
        now: NOW + 20,
    };
    let run = execute_step(&input, &registry, &mut s).unwrap();
    assert_eq!(run.state, ExecutionState::Succeeded);
    let variant = &catalog.catalog.get(FIX).unwrap().variants[&Platform::Linux];
    let asked = Cell::new(false);
    let v = verify_step(
        run.execution,
        &p.steps[0],
        variant,
        &registry,
        &mut s,
        |probe| {
            asked.set(probe == "net.dns.resolve");
            dns(still_failing)
        },
        NOW + 30,
    )
    .unwrap();
    assert!(asked.get());
    let event = match v.report {
        None => TicketEvent::VerificationPassed,
        Some(trigger) => TicketEvent::CannotResolve(trigger),
    };
    s.apply(&t.id, event, NOW + 31).unwrap();
    (s, t.id, plan_id)
}

#[test]
fn end_to_end_verified_fix_waits_for_the_person() {
    let (s, ticket, plan_id) = full_flow(false);
    let rows = s.executions(plan_id).unwrap();
    assert_eq!(rows[0].state, ExecutionState::VerifiedOk);
    assert_eq!(rows[0].verification.as_deref(), Some("verified_ok"));
    assert_eq!(
        s.ticket(&ticket).unwrap().unwrap().state,
        TicketState::ResolvedPendingConfirmation,
        "resolved only after the person confirms"
    );
}

#[test]
fn end_to_end_failed_verification_is_rep03() {
    let (s, ticket, plan_id) = full_flow(true);
    assert_eq!(
        s.executions(plan_id).unwrap()[0].state,
        ExecutionState::VerifiedFailed
    );
    let t = s.ticket(&ticket).unwrap().unwrap();
    assert_eq!(t.state, TicketState::Unresolved);
    assert_eq!(t.last_trigger, Some(ReportTrigger::VerificationFailed));
}

#[test]
fn retention_removes_plans_and_journal_with_the_ticket() {
    let (mut s, ticket, plan_id) = full_flow(true);
    s.apply(&ticket, TicketEvent::Close, NOW + 40).unwrap();
    s.purge(NOW + 40 + 180 * DAY, &RetentionConfig::default())
        .unwrap();
    assert!(s.executions(plan_id).unwrap().is_empty());
}
