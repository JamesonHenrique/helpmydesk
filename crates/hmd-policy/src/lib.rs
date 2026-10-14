// SPDX-License-Identifier: GPL-3.0-or-later
//! Deterministic Policy Engine (spec 002). "The AI proposes, code decides": proposals carry only
//! capability IDs, versions and parameters; everything that matters for safety comes from the
//! catalog and from evidence supplied by the caller. Pure: no I/O, no clock.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod catalog;
pub mod context;
pub mod engine;
pub mod proposal;

pub use catalog::{
    Capability, CapabilityKind, Catalog, CatalogError, DataLoss, Disruption, Platform,
    PlatformVariant, Preconditions, RiskLevel, Rollback, Scope, min_level, validate_catalog,
};
pub use context::{AutonomyMode, Confidence, PolicyContext, SessionCounters};
pub use engine::{
    ApprovalRequirement, MAX_STEPS, PlanRejection, PolicyDecision, RejectReason, StepVerdict,
    evaluate,
};
pub use proposal::{ActionPlanProposal, ProposedStep};
