// SPDX-License-Identifier: GPL-3.0-or-later
//! Executor (spec 008, Plan v1 V.7/V.10): runs one approved catalog step at a time, records it in
//! a write-ahead journal and proves the result with the catalog's verification probe.
//!
//! The executor never takes commands, paths or shell text from a plan: a step names a catalog
//! capability, and only an implementation compiled into this build can run it. This build ships
//! no implementation that changes the system ([`Registry::production`] is empty) until spike S-02
//! measures the privileges each fix needs.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod canonical;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use hmd_core::ProbeOutcome;
use hmd_domain::{ExecutionState, ReportTrigger, Timestamp};
use hmd_policy::{
    ActionPlanProposal, AutonomyMode, Platform, PlatformVariant, PolicyDecision, ProposedStep,
    Rollback, StepVerdict,
};

pub use canonical::{PLAN_FORMAT, canonical, idempotency_key, plan_hash, sha256_tagged};

/// Default lifetime of an approval (Plan v1 V.7).
pub const APPROVAL_TTL: i64 = 10 * 60;

/// Proof that the person approved exactly this plan (Plan v1 VII.3 `ActionApproval`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionApproval {
    pub plan_hash: String,
    pub approved_step_ids: BTreeSet<String>,
    pub mode: AutonomyMode,
    pub decided_at: Timestamp,
    pub expires_at: Timestamp,
    /// Unique per approval (FR-002); uniqueness, not secrecy, is what matters locally.
    pub nonce: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecError {
    PlanHashMismatch,
    ApprovalExpired,
    StepNotApproved,
    DuplicateExecution,
    NotImplemented,
    /// The journal refused or failed; nothing was run after it.
    Journal(String),
}

impl ExecError {
    pub fn reason_code(&self) -> &'static str {
        match self {
            ExecError::PlanHashMismatch => "PLAN_HASH_MISMATCH",
            ExecError::ApprovalExpired => "APPROVAL_EXPIRED",
            ExecError::StepNotApproved => "STEP_NOT_APPROVED",
            ExecError::DuplicateExecution => "DUPLICATE_EXECUTION",
            ExecError::NotImplemented => "NOT_IMPLEMENTED",
            ExecError::Journal(_) => "JOURNAL_ERROR",
        }
    }
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecError::Journal(m) => write!(f, "JOURNAL_ERROR: {m}"),
            other => write!(f, "{}", other.reason_code()),
        }
    }
}

impl std::error::Error for ExecError {}

/// What an implementation reports back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixOutcome {
    Succeeded,
    Failed(&'static str),
    /// The implementation cannot tell whether the change happened.
    Unknown,
}

/// A native, fixed implementation of one catalog fix. It receives only the catalog step (ID,
/// version and catalog-validated params), never free text.
pub trait FixImpl {
    fn run(&self, step: &ProposedStep) -> FixOutcome;

    /// Called only when the catalog declares `rollback = "implemented"`.
    fn rollback(&self, _step: &ProposedStep) -> FixOutcome {
        FixOutcome::Failed("ROLLBACK_NOT_IMPLEMENTED")
    }
}

/// Implementations compiled into this build, by capability ID.
#[derive(Default)]
pub struct Registry {
    impls: BTreeMap<String, Box<dyn FixImpl>>,
}

impl Registry {
    /// The production registry: empty until spike S-02 (spec 008 edge cases).
    pub fn production() -> Registry {
        Registry::default()
    }

    pub fn with(mut self, capability: &str, implementation: Box<dyn FixImpl>) -> Registry {
        self.impls.insert(capability.to_string(), implementation);
        self
    }

    pub fn get(&self, capability: &str) -> Option<&dyn FixImpl> {
        self.impls.get(capability).map(|b| b.as_ref())
    }

    pub fn contains(&self, capability: &str) -> bool {
        self.impls.contains_key(capability)
    }
}

/// Error text from a journal implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalError(pub String);

/// What is written when a step starts (before the implementation runs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStart<'a> {
    pub plan_ref: i64,
    pub approval_nonce: i64,
    pub step_id: &'a str,
    /// `id@version`.
    pub capability: String,
    pub platform: Platform,
    pub idempotency_key: String,
    pub at: Timestamp,
}

/// Write-ahead execution journal (implemented by `hmd-store`). `advance` MUST enforce
/// [`hmd_domain::execution::transition`].
pub trait Journal {
    fn key_used(&self, idempotency_key: &str) -> Result<bool, JournalError>;
    /// Records the execution as `started` and returns its ID.
    fn begin(&mut self, start: &ExecutionStart<'_>) -> Result<i64, JournalError>;
    fn advance(
        &mut self,
        execution: i64,
        to: ExecutionState,
        note: Option<&str>,
        at: Timestamp,
    ) -> Result<(), JournalError>;
}

/// The evaluated plan the person saw.
#[derive(Debug, Clone, Copy)]
pub struct EvaluatedPlan<'a> {
    pub proposal: &'a ActionPlanProposal,
    pub decision: &'a PolicyDecision,
    pub catalog_version: &'a str,
}

impl EvaluatedPlan<'_> {
    pub fn hash(&self) -> String {
        plan_hash(self.proposal, self.decision, self.catalog_version)
    }
}

impl ActionApproval {
    /// Approval of `step_ids` of `plan`, valid for [`APPROVAL_TTL`].
    pub fn new(
        plan: &EvaluatedPlan<'_>,
        step_ids: impl IntoIterator<Item = String>,
        mode: AutonomyMode,
        now: Timestamp,
        nonce: i64,
    ) -> ActionApproval {
        ActionApproval {
            plan_hash: plan.hash(),
            approved_step_ids: step_ids.into_iter().collect(),
            mode,
            decided_at: now,
            expires_at: now.saturating_add(APPROVAL_TTL),
            nonce,
        }
    }
}

/// FR-003 checks 1–3, pure: hash, expiry, step approved and accepted by the Policy Engine.
/// Returns the accepted verdict and the proposed step.
pub fn check_approval<'a>(
    plan: &EvaluatedPlan<'a>,
    approval: &ActionApproval,
    step_id: &str,
    now: Timestamp,
) -> Result<(&'a StepVerdict, &'a ProposedStep), ExecError> {
    if plan.hash() != approval.plan_hash {
        return Err(ExecError::PlanHashMismatch);
    }
    if now >= approval.expires_at || now < approval.decided_at {
        return Err(ExecError::ApprovalExpired);
    }
    if !approval.approved_step_ids.contains(step_id) {
        return Err(ExecError::StepNotApproved);
    }
    let verdict = plan
        .decision
        .verdicts
        .iter()
        .find(|v| v.step_id() == step_id)
        .filter(|v| v.is_accepted())
        .ok_or(ExecError::StepNotApproved)?;
    let step = plan
        .proposal
        .steps
        .iter()
        .find(|s| s.step_id == step_id)
        .ok_or(ExecError::StepNotApproved)?;
    Ok((verdict, step))
}

/// Result of running one step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepRun {
    pub execution: i64,
    pub state: ExecutionState,
    pub reason: Option<&'static str>,
}

/// Everything [`execute_step`] needs besides the registry and the journal.
#[derive(Debug, Clone, Copy)]
pub struct ExecuteInput<'a> {
    pub plan: EvaluatedPlan<'a>,
    pub plan_ref: i64,
    pub approval: &'a ActionApproval,
    pub step_id: &'a str,
    pub platform: Platform,
    pub now: Timestamp,
}

fn journal(e: JournalError) -> ExecError {
    ExecError::Journal(e.0)
}

/// FR-003: checks in order, then writes `started`, runs the implementation and records the
/// outcome. A panic-free implementation that cannot tell the outcome yields `unknown`.
pub fn execute_step(
    input: &ExecuteInput<'_>,
    registry: &Registry,
    journal_impl: &mut dyn Journal,
) -> Result<StepRun, ExecError> {
    let (_, step) = check_approval(&input.plan, input.approval, input.step_id, input.now)?;
    let key = idempotency_key(
        &input.plan.proposal.session_id,
        input.step_id,
        input.approval.nonce,
    );
    if journal_impl.key_used(&key).map_err(journal)? {
        return Err(ExecError::DuplicateExecution);
    }
    let implementation = registry
        .get(&step.capability)
        .ok_or(ExecError::NotImplemented)?;
    // US2.1: write-ahead. If the app dies after this line, startup turns it into `unknown`.
    let execution = journal_impl
        .begin(&ExecutionStart {
            plan_ref: input.plan_ref,
            approval_nonce: input.approval.nonce,
            step_id: input.step_id,
            capability: format!("{}@{}", step.capability, step.version),
            platform: input.platform,
            idempotency_key: key,
            at: input.now,
        })
        .map_err(journal)?;
    let (state, reason) = match implementation.run(step) {
        FixOutcome::Succeeded => (ExecutionState::Succeeded, None),
        FixOutcome::Failed(code) => (ExecutionState::Failed, Some(code)),
        FixOutcome::Unknown => (ExecutionState::Unknown, None),
    };
    journal_impl
        .advance(execution, state, reason, input.now)
        .map_err(journal)?;
    Ok(StepRun {
        execution,
        state,
        reason,
    })
}

/// Result of verifying one step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    pub state: ExecutionState,
    /// REP-03 when the verification did not prove the fix.
    pub report: Option<ReportTrigger>,
    pub reason: Option<&'static str>,
}

/// Pure US3 rule: verified when the probe ran and none of the step's `all_of` precondition
/// findings is still produced.
pub fn verification_passed(variant: &PlatformVariant, outcome: &ProbeOutcome) -> bool {
    match outcome {
        ProbeOutcome::Ran(out) => !out
            .findings
            .iter()
            .any(|f| variant.preconditions.all_of.iter().any(|p| p == f.code)),
        _ => false,
    }
}

/// US3: runs the catalog verification probe through `probe` (which MUST authorize it with the
/// Policy Engine, e.g. [`hmd_core::run_authorized_probe`]), records `verified_ok` or
/// `verified_failed`, and rolls back when the catalog says the capability can.
pub fn verify_step(
    execution: i64,
    step: &ProposedStep,
    variant: &PlatformVariant,
    registry: &Registry,
    journal_impl: &mut dyn Journal,
    probe: impl FnOnce(&str) -> ProbeOutcome,
    now: Timestamp,
) -> Result<Verification, ExecError> {
    let (passed, reason) = match variant.verification.as_deref() {
        Some(probe_id) => {
            let outcome = probe(probe_id);
            let reason = match &outcome {
                ProbeOutcome::Failed(code) | ProbeOutcome::NotAuthorized(code) => Some(*code),
                ProbeOutcome::Ran(_) => None,
            };
            (verification_passed(variant, &outcome), reason)
        }
        None => (false, Some("NO_VERIFICATION")),
    };
    if passed {
        journal_impl
            .advance(execution, ExecutionState::VerifiedOk, None, now)
            .map_err(journal)?;
        return Ok(Verification {
            state: ExecutionState::VerifiedOk,
            report: None,
            reason: None,
        });
    }
    journal_impl
        .advance(execution, ExecutionState::VerifiedFailed, reason, now)
        .map_err(journal)?;
    let mut state = ExecutionState::VerifiedFailed;
    if variant.rollback == Rollback::Implemented
        && let Some(implementation) = registry.get(&step.capability)
        && implementation.rollback(step) == FixOutcome::Succeeded
    {
        journal_impl
            .advance(execution, ExecutionState::RolledBack, None, now)
            .map_err(journal)?;
        state = ExecutionState::RolledBack;
    }
    Ok(Verification {
        state,
        report: Some(ReportTrigger::VerificationFailed),
        reason,
    })
}
