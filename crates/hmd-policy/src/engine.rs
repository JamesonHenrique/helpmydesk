// SPDX-License-Identifier: GPL-3.0-or-later
//! `evaluate`: applies the checks of Plan v1 V.6 in the fixed order documented in
//! `specs/002-policy-engine/plan.md` (first failure wins).

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ActionPlanProposal, AutonomyMode, Capability, CapabilityKind, Catalog, Confidence,
    PlatformVariant, PolicyContext, ProposedStep, RiskLevel, Rollback,
};

/// FR-010: maximum steps in one plan.
pub const MAX_STEPS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RejectReason {
    DuplicateStep,
    UnknownCapability,
    VersionMismatch,
    PlatformUnsupported,
    CapabilityDisabled,
    BlockedCapability,
    InvalidParams,
    PreconditionUnmet,
    ConsentRequired,
    PendingUnknownExecution,
    RateLimit,
    ExclusiveConflict,
    LowConfidence,
    /// The step is turned into guidance instead of being executed.
    LevelExceedsMode,
}

impl RejectReason {
    pub fn reason_code(self) -> &'static str {
        match self {
            RejectReason::DuplicateStep => "DUPLICATE_STEP",
            RejectReason::UnknownCapability => "UNKNOWN_CAPABILITY",
            RejectReason::VersionMismatch => "VERSION_MISMATCH",
            RejectReason::PlatformUnsupported => "PLATFORM_UNSUPPORTED",
            RejectReason::CapabilityDisabled => "CAPABILITY_DISABLED",
            RejectReason::BlockedCapability => "BLOCKED_CAPABILITY",
            RejectReason::InvalidParams => "INVALID_PARAMS",
            RejectReason::PreconditionUnmet => "PRECONDITION_UNMET",
            RejectReason::ConsentRequired => "CONSENT_REQUIRED",
            RejectReason::PendingUnknownExecution => "PENDING_UNKNOWN_EXECUTION",
            RejectReason::RateLimit => "RATE_LIMIT",
            RejectReason::ExclusiveConflict => "EXCLUSIVE_CONFLICT",
            RejectReason::LowConfidence => "LOW_CONFIDENCE",
            RejectReason::LevelExceedsMode => "LEVEL_EXCEEDS_MODE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanRejection {
    EmptyPlan,
    TooManySteps,
}

impl PlanRejection {
    pub fn reason_code(self) -> &'static str {
        match self {
            PlanRejection::EmptyPlan => "EMPTY_PLAN",
            PlanRejection::TooManySteps => "TOO_MANY_STEPS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalRequirement {
    /// Runs without asking (probes, guides, auto-eligible N2 in `AutoSimple`).
    None,
    /// Covered by one approval of the whole plan.
    Plan,
    /// Needs its own approval (and UAC/polkit when elevated).
    PerAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepVerdict {
    Accepted {
        step_id: String,
        capability: String,
        risk_level: RiskLevel,
        approval: ApprovalRequirement,
        requires_elevation: bool,
        verification: Option<String>,
        rollback: Rollback,
    },
    /// Not executed; the orchestrator shows the catalog guidance instead.
    Downgraded {
        step_id: String,
        capability: String,
        reason: RejectReason,
    },
    Rejected {
        step_id: String,
        reason: RejectReason,
    },
}

impl StepVerdict {
    pub fn step_id(&self) -> &str {
        match self {
            StepVerdict::Accepted { step_id, .. }
            | StepVerdict::Downgraded { step_id, .. }
            | StepVerdict::Rejected { step_id, .. } => step_id,
        }
    }

    pub fn is_accepted(&self) -> bool {
        matches!(self, StepVerdict::Accepted { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDecision {
    pub verdicts: Vec<StepVerdict>,
}

impl PolicyDecision {
    pub fn accepted(&self) -> impl Iterator<Item = &StepVerdict> {
        self.verdicts.iter().filter(|v| v.is_accepted())
    }
}

/// FR-006: approval needed for an accepted level under a mode; `None` means "downgrade".
pub(crate) fn approval_for(
    mode: AutonomyMode,
    level: RiskLevel,
    auto_eligible: bool,
) -> Option<ApprovalRequirement> {
    match (mode, level) {
        (_, RiskLevel::N0 | RiskLevel::N1A | RiskLevel::N1B | RiskLevel::N1S) => {
            Some(ApprovalRequirement::None)
        }
        (AutonomyMode::ObserveOnly, _) | (_, RiskLevel::N4) => None,
        (AutonomyMode::AskAlways, RiskLevel::N2) => Some(ApprovalRequirement::Plan),
        (AutonomyMode::AutoSimple, RiskLevel::N2) if auto_eligible => {
            Some(ApprovalRequirement::None)
        }
        (AutonomyMode::AutoSimple, RiskLevel::N2) => Some(ApprovalRequirement::Plan),
        (_, RiskLevel::N3) => Some(ApprovalRequirement::PerAction),
    }
}

/// Evaluates a proposal. Deterministic: equal inputs give equal decisions (NFR-001).
pub fn evaluate(
    proposal: &ActionPlanProposal,
    catalog: &Catalog,
    ctx: &PolicyContext,
) -> Result<PolicyDecision, PlanRejection> {
    if proposal.steps.is_empty() {
        return Err(PlanRejection::EmptyPlan);
    }
    if proposal.steps.len() > MAX_STEPS {
        return Err(PlanRejection::TooManySteps);
    }

    let mut seen_steps = BTreeSet::new();
    let mut accepted_caps: BTreeSet<String> = BTreeSet::new();
    let mut planned_runs: BTreeMap<String, u32> = BTreeMap::new();
    let mut planned_fixes: u32 = 0;
    let mut verdicts = Vec::with_capacity(proposal.steps.len());

    for step in &proposal.steps {
        if !seen_steps.insert(step.step_id.clone()) {
            verdicts.push(rejected(step, RejectReason::DuplicateStep));
            continue;
        }
        let (cap, variant) = match resolve(step, catalog, ctx) {
            Ok(found) => found,
            Err(reason) => {
                verdicts.push(rejected(step, reason));
                continue;
            }
        };
        if let Err(reason) = check_step(step, cap, variant, ctx) {
            verdicts.push(rejected(step, reason));
            continue;
        }

        // FR-008: limits, counting what this plan already accepted.
        let done = ctx.counters.executions.get(&cap.id).copied().unwrap_or(0);
        let planned = planned_runs.get(&cap.id).copied().unwrap_or(0);
        let is_fix = cap.kind == CapabilityKind::Fix;
        if done.saturating_add(planned) >= variant.max_per_session
            || (is_fix
                && ctx.counters.total_fixes.saturating_add(planned_fixes)
                    >= ctx.max_fixes_per_session)
        {
            verdicts.push(rejected(step, RejectReason::RateLimit));
            continue;
        }
        if cap
            .exclusive_with
            .iter()
            .any(|other| accepted_caps.contains(other))
            || accepted_caps
                .iter()
                .filter_map(|id| catalog.get(id))
                .any(|c| c.exclusive_with.contains(&cap.id))
        {
            verdicts.push(rejected(step, RejectReason::ExclusiveConflict));
            continue;
        }
        // FR-009
        if variant.risk_level == RiskLevel::N3 && ctx.confidence < Confidence::Probable {
            verdicts.push(rejected(step, RejectReason::LowConfidence));
            continue;
        }

        match approval_for(ctx.mode, variant.risk_level, variant.auto_eligible) {
            Some(approval) => {
                accepted_caps.insert(cap.id.clone());
                *planned_runs.entry(cap.id.clone()).or_insert(0) += 1;
                if is_fix {
                    planned_fixes += 1;
                }
                verdicts.push(StepVerdict::Accepted {
                    step_id: step.step_id.clone(),
                    capability: cap.id.clone(),
                    risk_level: variant.risk_level,
                    approval,
                    requires_elevation: variant.requires_elevation,
                    verification: variant.verification.clone(),
                    rollback: variant.rollback,
                });
            }
            None => verdicts.push(StepVerdict::Downgraded {
                step_id: step.step_id.clone(),
                capability: cap.id.clone(),
                reason: RejectReason::LevelExceedsMode,
            }),
        }
    }
    Ok(PolicyDecision { verdicts })
}

/// Checks 2–5: existence, version, platform, kill switch, N4 (FR-001, FR-002).
fn resolve<'a>(
    step: &ProposedStep,
    catalog: &'a Catalog,
    ctx: &PolicyContext,
) -> Result<(&'a Capability, &'a PlatformVariant), RejectReason> {
    let cap = catalog
        .get(&step.capability)
        .ok_or(RejectReason::UnknownCapability)?;
    if cap.version != step.version {
        return Err(RejectReason::VersionMismatch);
    }
    let variant = cap
        .variants
        .get(&ctx.platform)
        .ok_or(RejectReason::PlatformUnsupported)?;
    if ctx.disabled.contains(&cap.id) {
        return Err(RejectReason::CapabilityDisabled);
    }
    if variant.risk_level == RiskLevel::N4 {
        return Err(RejectReason::BlockedCapability);
    }
    Ok((cap, variant))
}

/// Checks 6–9: params, preconditions, consent, pending unknown (FR-004, FR-005, FR-007, FR-008).
fn check_step(
    step: &ProposedStep,
    cap: &Capability,
    variant: &PlatformVariant,
    ctx: &PolicyContext,
) -> Result<(), RejectReason> {
    let mut names = BTreeSet::new();
    for (name, value) in &step.params {
        if !variant.params.contains(name)
            || value.len() > ctx.max_param_len
            || !names.insert(name.as_str())
        {
            return Err(RejectReason::InvalidParams);
        }
    }
    let pre = &variant.preconditions;
    if !pre.all_of.iter().all(|f| ctx.findings.contains(f))
        || pre.none_of.iter().any(|f| ctx.findings.contains(f))
    {
        return Err(RejectReason::PreconditionUnmet);
    }
    if variant.risk_level == RiskLevel::N1S {
        match &variant.consent_category {
            Some(category) if ctx.consents.contains(category) => {}
            _ => return Err(RejectReason::ConsentRequired),
        }
    }
    if ctx.counters.unknown_executions.contains(&cap.id) {
        return Err(RejectReason::PendingUnknownExecution);
    }
    Ok(())
}

fn rejected(step: &ProposedStep, reason: RejectReason) -> StepVerdict {
    StepVerdict::Rejected {
        step_id: step.step_id.clone(),
        reason,
    }
}
