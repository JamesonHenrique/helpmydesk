// SPDX-License-Identifier: GPL-3.0-or-later
//! Executor (spec 008, Plan v1 V.7/V.10): runs one approved catalog step at a time, records it in
//! a write-ahead journal and proves the result with the catalog's verification probe.
//!
//! The executor never takes commands, paths or shell text from a plan: a step names a catalog
//! capability, and only an implementation compiled into this build can run it. This build ships
//! no implementation that changes the system ([`Registry::production`] is empty) until spike S-02
//! measures the privileges each fix needs.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use hmd_core::ProbeOutcome;
use hmd_domain::{ExecutionState, ReportTrigger, Timestamp};
use hmd_policy::{
    ActionPlanProposal, AutonomyMode, Platform, PlatformVariant, PolicyDecision, ProposedStep,
    Rollback, StepVerdict,
};

// Spec 020: the canonical encoding lives in `hmd-canonical` so the elevated helper recalculates
// the plan hash with this same code. The re-exports keep `hmd-store` and `hmd-app` unchanged.
pub use hmd_canonical::{PLAN_FORMAT, canonical, idempotency_key, plan_hash, sha256_tagged};

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
    /// This build has no elevated implementation for the step (spec 020 US1.1): nothing ran,
    /// nothing was written to the journal.
    ElevationNotAvailable,
    /// The person refused the system permission dialog (spec 020 US1.3).
    ElevationDenied,
    /// The hard deadline expired with the dialog or the action still open (spec 020 FR-010).
    ElevationTimeout,
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
            ExecError::ElevationNotAvailable => "ELEVATION_NOT_AVAILABLE",
            ExecError::ElevationDenied => "ELEVATION_DENIED",
            ExecError::ElevationTimeout => "ELEVATION_TIMEOUT",
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

use std::time::Duration;

/// Hard deadline for one elevated invocation, counting the wait in the system permission dialog
/// (spec 020 FR-010). When it expires the step is `unknown`, never re-run automatically.
pub const ELEVATION_TIMEOUT: Duration = Duration::from_secs(120);

/// Upper bound for a parameter value, re-checked by the helper (spec 020 FR-007). Same number the
/// Policy Engine applies to proposals (`max_param_len`), repeated on the other side of the trust
/// boundary.
pub const MAX_PARAM_LEN: usize = 256;

/// How a step whose catalog variant declares `requires_elevation` actually gets privileged
/// execution (spec 020 FR-008). `hmd-elevated` implements this; the executor itself never spawns
/// a process, which keeps it pure and testable without privileges.
pub trait Elevation: Send + Sync {
    /// Whether this build can elevate `step` on `platform`. False means the production allowlist
    /// has no implementation yet (spike S-02).
    fn available(&self, step: &ProposedStep, platform: Platform) -> bool;
    /// Runs one approved step elevated. The implementation revalidates the plan by itself; the
    /// executor does not assume anything about what happened.
    fn run(
        &self,
        step: &ProposedStep,
        plan: &EvaluatedPlan<'_>,
        platform: Platform,
    ) -> Result<FixOutcome, ExecError>;

    /// Undo of an elevated step, called only when the catalog declares
    /// `rollback = "implemented"` (spec 008 US3, spec 020 FR-009). `None` means this build has no
    /// undo for that action, which leaves the execution at `verified_failed`.
    fn rollback(
        &self,
        _step: &ProposedStep,
        _plan: &EvaluatedPlan<'_>,
        _platform: Platform,
    ) -> Option<FixOutcome> {
        None
    }
}

/// The default of this build: no elevation path, so every elevated step is refused before
/// anything is written to the journal (spec 020 US1.1, `Registry::production` stays empty until
/// spike S-02 measures the privileges each fix needs).
pub struct NoElevation;

impl Elevation for NoElevation {
    fn available(&self, _step: &ProposedStep, _platform: Platform) -> bool {
        false
    }

    fn run(
        &self,
        _step: &ProposedStep,
        _plan: &EvaluatedPlan<'_>,
        _platform: Platform,
    ) -> Result<FixOutcome, ExecError> {
        Err(ExecError::ElevationNotAvailable)
    }
}

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
pub trait FixImpl: Send + Sync {
    fn run(&self, step: &ProposedStep) -> FixOutcome;

    /// Called only when the catalog declares `rollback = "implemented"`.
    fn rollback(&self, _step: &ProposedStep) -> FixOutcome {
        FixOutcome::Failed("ROLLBACK_NOT_IMPLEMENTED")
    }
}

/// Implementations compiled into this build, by capability ID, plus the elevation path used by
/// steps whose catalog variant declares `requires_elevation` (spec 020).
pub struct Registry {
    impls: BTreeMap<String, Box<dyn FixImpl>>,
    elevation: Box<dyn Elevation>,
}

impl Default for Registry {
    fn default() -> Registry {
        Registry {
            impls: BTreeMap::new(),
            elevation: Box::new(NoElevation),
        }
    }
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

    /// Wires the elevation path (spec 020). Without this, elevated steps are refused.
    pub fn with_elevation(mut self, elevation: Box<dyn Elevation>) -> Registry {
        self.elevation = elevation;
        self
    }

    pub fn elevation(&self) -> &dyn Elevation {
        self.elevation.as_ref()
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

    /// The canonical text itself, so the elevation path can put it in the request and the helper
    /// can recalculate the hash from what it received (spec 020 FR-006).
    pub fn canonical(&self) -> String {
        canonical(self.proposal, self.decision, self.catalog_version)
    }
}

/// Whether the Policy Engine accepted this step with `requires_elevation`.
fn verdict_requires_elevation(verdict: &StepVerdict) -> bool {
    matches!(
        verdict,
        StepVerdict::Accepted {
            requires_elevation: true,
            ..
        }
    )
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
    let (verdict, step) = check_approval(&input.plan, input.approval, input.step_id, input.now)?;
    let key = idempotency_key(
        &input.plan.proposal.session_id,
        input.step_id,
        input.approval.nonce,
    );
    if journal_impl.key_used(&key).map_err(journal)? {
        return Err(ExecError::DuplicateExecution);
    }
    // Spec 020: an elevated step runs only through the elevation path; refusing it happens before
    // any journal write, so nothing is left half-executed.
    let elevated = verdict_requires_elevation(verdict);
    let implementation = if elevated {
        if !registry.elevation().available(step, input.platform) {
            return Err(ExecError::ElevationNotAvailable);
        }
        None
    } else {
        Some(
            registry
                .get(&step.capability)
                .ok_or(ExecError::NotImplemented)?,
        )
    };
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
    // Spec 016: a panic inside a fix must not skip the journal update (the row would stay
    // `started`) nor unwind into the caller, which holds the backend lock.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match implementation {
        Some(implementation) => Ok(implementation.run(step)),
        None => registry.elevation().run(step, &input.plan, input.platform),
    }));
    let (state, reason) = match outcome {
        Ok(Ok(FixOutcome::Succeeded)) => (ExecutionState::Succeeded, None),
        Ok(Ok(FixOutcome::Failed(code))) => (ExecutionState::Failed, Some(code)),
        Ok(Ok(FixOutcome::Unknown)) => (ExecutionState::Unknown, None),
        // After `started` an error is uncertainty, not success: `unknown` with the reason code.
        Ok(Err(e)) => (
            match e {
                ExecError::ElevationDenied => ExecutionState::Failed,
                _ => ExecutionState::Unknown,
            },
            Some(e.reason_code()),
        ),
        Err(_) => (ExecutionState::Unknown, Some("PANICKED")),
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

/// Everything [`verify_step`] needs besides the registry and the journal. The plan and the platform
/// travel with it because the undo of an elevated step goes through the elevation path (spec 020).
#[derive(Debug, Clone, Copy)]
pub struct VerifyInput<'a> {
    pub execution: i64,
    pub step: &'a ProposedStep,
    pub variant: &'a PlatformVariant,
    pub plan: EvaluatedPlan<'a>,
    pub platform: Platform,
    pub now: Timestamp,
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
    input: &VerifyInput<'_>,
    registry: &Registry,
    journal_impl: &mut dyn Journal,
    probe: impl FnOnce(&str) -> ProbeOutcome,
) -> Result<Verification, ExecError> {
    let VerifyInput {
        execution,
        step,
        variant,
        plan,
        platform,
        now,
    } = *input;
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
    // Spec 020: an elevated step has no implementation in the plain registry; its undo goes
    // through the same elevation path that ran it.
    let rolled_back = if variant.rollback == Rollback::Implemented {
        if variant.requires_elevation {
            registry.elevation().rollback(step, &plan, platform)
        } else {
            registry.get(&step.capability).map(|i| i.rollback(step))
        }
    } else {
        None
    };
    if rolled_back == Some(FixOutcome::Succeeded) {
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
