// SPDX-License-Identifier: GPL-3.0-or-later
//! What a playbook or the AI may propose (spec 002, FR-003). By construction there are no fields
//! for risk, approval, elevation or verification: those can only come from the catalog.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedStep {
    pub step_id: String,
    pub capability: String,
    pub version: String,
    pub params: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionPlanProposal {
    pub session_id: String,
    pub steps: Vec<ProposedStep>,
}
