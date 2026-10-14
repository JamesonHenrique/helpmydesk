// SPDX-License-Identifier: GPL-3.0-or-later
//! Evidence and settings the caller supplies to the engine (spec 002, FR-004..FR-009).

use std::collections::{BTreeMap, BTreeSet};

use crate::Platform;

/// Ceiling chosen by the person (Plan v1 V.2). `AskAlways` is the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutonomyMode {
    ObserveOnly,
    #[default]
    AskAlways,
    AutoSimple,
}

/// Diagnosis confidence computed by the playbook rules (never the model's own claim).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    Unsure,
    Probable,
    AlmostCertain,
}

/// Execution history of the current session.
#[derive(Debug, Clone, Default)]
pub struct SessionCounters {
    /// Completed executions per capability ID.
    pub executions: BTreeMap<String, u32>,
    /// Capabilities with an execution in state `unknown`.
    pub unknown_executions: BTreeSet<String>,
    /// Fixes executed in this session (all capabilities).
    pub total_fixes: u32,
}

#[derive(Debug, Clone)]
pub struct PolicyContext {
    pub platform: Platform,
    pub mode: AutonomyMode,
    /// Finding codes produced by probes in this session.
    pub findings: BTreeSet<String>,
    /// Data categories the person consented to (for N1-S probes).
    pub consents: BTreeSet<String>,
    pub confidence: Confidence,
    /// Capability IDs disabled by the signed kill switch.
    pub disabled: BTreeSet<String>,
    pub counters: SessionCounters,
    /// Maximum fixes per session across all capabilities.
    pub max_fixes_per_session: u32,
    /// Maximum length of a parameter value.
    pub max_param_len: usize,
}

impl PolicyContext {
    pub fn new(platform: Platform) -> Self {
        PolicyContext {
            platform,
            mode: AutonomyMode::default(),
            findings: BTreeSet::new(),
            consents: BTreeSet::new(),
            confidence: Confidence::Unsure,
            disabled: BTreeSet::new(),
            counters: SessionCounters::default(),
            max_fixes_per_session: 5,
            max_param_len: 256,
        }
    }
}
