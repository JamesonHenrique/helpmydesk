// SPDX-License-Identifier: GPL-3.0-or-later
//! Orchestration of one diagnosis (spec 006): authorize the playbook's probes through the Policy
//! Engine, run them, select a diagnosis deterministically and evaluate the proposed fixes.
//! Nothing is executed on the system here; execution is a later spec. [`report`] renders the
//! "não consegui" report (spec 007).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::BTreeSet;
use std::fmt;

use hmd_catalog::{DiagnosisCopy, DiagnosisRule, LoadError, LoadedCatalog, Playbook};
use hmd_domain::{ProblemFamily, ReportTrigger};
use hmd_policy::{
    ActionPlanProposal, AutonomyMode, Confidence, Platform, PolicyContext, PolicyDecision,
    ProposedStep, StepVerdict, evaluate,
};
use hmd_probes::{ProbeError, ProbeOutput};

pub mod report;

/// Locale used for texts in the MVP.
pub const LOCALE: &str = "pt-BR";

/// The platform this binary runs on.
pub fn current_platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else {
        Platform::Linux
    }
}

/// FR-004: one level less sure when any probe of the playbook failed.
pub fn lower(c: Confidence) -> Confidence {
    match c {
        Confidence::AlmostCertain => Confidence::Probable,
        Confidence::Probable | Confidence::Unsure => Confidence::Unsure,
    }
}

/// How sure the app is, in words for the person.
pub fn confidence_phrase(c: Confidence) -> &'static str {
    match c {
        Confidence::AlmostCertain => "Tenho quase certeza",
        Confidence::Probable => "Acho provável",
        Confidence::Unsure => "Não tenho certeza",
    }
}

/// Result of the pure playbook evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection<'a> {
    pub rule: &'a DiagnosisRule,
    pub confidence: Confidence,
    pub is_fallback: bool,
}

fn holds(rule: &DiagnosisRule, findings: &BTreeSet<String>) -> bool {
    rule.when_all.iter().all(|f| findings.contains(f))
        && (rule.when_any.is_empty() || rule.when_any.iter().any(|f| findings.contains(f)))
        && !rule.when_none.iter().any(|f| findings.contains(f))
}

/// FR-002: first diagnosis whose conditions hold, otherwise the fallback. Pure.
pub fn select<'a>(
    playbook: &'a Playbook,
    findings: &BTreeSet<String>,
    failed_probes: usize,
) -> Selection<'a> {
    let (rule, is_fallback) = playbook
        .diagnoses
        .iter()
        .find(|r| holds(r, findings))
        .map_or((&playbook.fallback, true), |r| (r, false));
    let confidence = if failed_probes > 0 {
        lower(rule.confidence)
    } else {
        rule.confidence
    };
    Selection {
        rule,
        confidence,
        is_fallback,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProbeOutcome {
    Ran(ProbeOutput),
    /// The probe ran and failed (`NO_DATA`, `TIMEOUT`…); never an invented result.
    Failed(&'static str),
    /// The Policy Engine did not authorize it (e.g. `CONSENT_REQUIRED`).
    NotAuthorized(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProbeRun {
    pub probe_id: String,
    pub outcome: ProbeOutcome,
}

#[derive(Debug, Clone)]
pub struct DiagnosisReport {
    pub family: ProblemFamily,
    pub playbook_id: String,
    pub probes: Vec<ProbeRun>,
    pub findings: BTreeSet<String>,
    pub diagnosis_id: String,
    pub confidence: Confidence,
    pub is_fallback: bool,
    pub copy: DiagnosisCopy,
    pub report: Option<ReportTrigger>,
    /// Policy decision on the proposed fixes, if the diagnosis proposes any (FR-005).
    pub fix_decision: Option<PolicyDecision>,
}

#[derive(Debug, Clone)]
pub struct DiagnoseOptions {
    pub platform: Platform,
    pub mode: AutonomyMode,
    pub consents: BTreeSet<String>,
}

impl Default for DiagnoseOptions {
    fn default() -> Self {
        DiagnoseOptions {
            platform: current_platform(),
            mode: AutonomyMode::default(),
            consents: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    NoPlaybook(ProblemFamily),
    /// The embedded catalog is invalid: the app must stay in read-only/guidance mode.
    Catalog(Vec<LoadError>),
    MissingCopy(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::NoPlaybook(family) => write!(f, "NO_PLAYBOOK: {}", family.code()),
            CoreError::Catalog(errors) => write!(f, "INVALID_CATALOG: {} error(s)", errors.len()),
            CoreError::MissingCopy(id) => write!(f, "MISSING_COPY: {id}"),
        }
    }
}

impl std::error::Error for CoreError {}

fn proposal(catalog: &LoadedCatalog, ids: &[String], prefix: &str) -> ActionPlanProposal {
    ActionPlanProposal {
        session_id: "local".to_string(),
        steps: ids
            .iter()
            .enumerate()
            .map(|(i, id)| ProposedStep {
                step_id: format!("{prefix}{}", i + 1),
                capability: id.clone(),
                version: catalog
                    .catalog
                    .get(id)
                    .map(|c| c.version.clone())
                    .unwrap_or_default(),
                params: vec![],
            })
            .collect(),
    }
}

/// Runs a diagnosis with an injectable probe runner (tests use fakes; the app uses
/// [`hmd_probes::run`]).
pub fn diagnose_with(
    catalog: &LoadedCatalog,
    family: ProblemFamily,
    opts: &DiagnoseOptions,
    run_probe: impl Fn(&str) -> Result<ProbeOutput, ProbeError>,
) -> Result<DiagnosisReport, CoreError> {
    let playbook = catalog
        .playbook_for(family)
        .ok_or(CoreError::NoPlaybook(family))?;

    // FR-003: probes are capabilities; the Policy Engine authorizes them.
    let mut probes = Vec::new();
    let mut findings = BTreeSet::new();
    let mut failed = 0usize;
    if !playbook.probes.is_empty() {
        let mut ctx = PolicyContext::new(opts.platform);
        ctx.mode = opts.mode;
        ctx.consents = opts.consents.clone();
        let decision = evaluate(
            &proposal(catalog, &playbook.probes, "p"),
            &catalog.catalog,
            &ctx,
        )
        .map_err(|_| CoreError::NoPlaybook(family))?;
        for (probe_id, verdict) in playbook.probes.iter().zip(&decision.verdicts) {
            let outcome = match verdict {
                StepVerdict::Accepted { .. } => match run_probe(probe_id) {
                    Ok(out) => {
                        findings.extend(out.findings.iter().map(|f| f.code.to_string()));
                        ProbeOutcome::Ran(out)
                    }
                    Err(e) => {
                        failed += 1;
                        ProbeOutcome::Failed(e.reason_code())
                    }
                },
                StepVerdict::Rejected { reason, .. } | StepVerdict::Downgraded { reason, .. } => {
                    ProbeOutcome::NotAuthorized(reason.reason_code())
                }
            };
            probes.push(ProbeRun {
                probe_id: probe_id.clone(),
                outcome,
            });
        }
    }

    let selection = select(playbook, &findings, failed);
    let copy = selection
        .rule
        .copy
        .get(LOCALE)
        .cloned()
        .ok_or_else(|| CoreError::MissingCopy(selection.rule.id.clone()))?;

    // FR-005: proposed fixes evaluated with the diagnosis confidence; nothing executes.
    let fix_decision = if selection.rule.fixes.is_empty() {
        None
    } else {
        let mut ctx = PolicyContext::new(opts.platform);
        ctx.mode = opts.mode;
        ctx.consents = opts.consents.clone();
        ctx.findings = findings.clone();
        ctx.confidence = selection.confidence;
        evaluate(
            &proposal(catalog, &selection.rule.fixes, "s"),
            &catalog.catalog,
            &ctx,
        )
        .ok()
    };

    Ok(DiagnosisReport {
        family,
        playbook_id: playbook.id.clone(),
        probes,
        findings,
        diagnosis_id: selection.rule.id.clone(),
        confidence: selection.confidence,
        is_fallback: selection.is_fallback,
        copy,
        report: selection.rule.report,
        fix_decision,
    })
}

/// Diagnoses a family with the embedded catalog and the real probes.
pub fn diagnose(
    family: ProblemFamily,
    opts: &DiagnoseOptions,
) -> Result<DiagnosisReport, CoreError> {
    let catalog = hmd_catalog::embedded().map_err(|e| CoreError::Catalog(e.to_vec()))?;
    diagnose_with(catalog, family, opts, hmd_probes::run)
}
