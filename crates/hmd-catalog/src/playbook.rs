// SPDX-License-Identifier: GPL-3.0-or-later
//! Deterministic playbooks per problem family (spec 006 FR-001): parsing and validation.

use std::collections::{BTreeMap, BTreeSet};

use hmd_domain::{ProblemFamily, ReportTrigger};
use hmd_policy::{CapabilityKind, Confidence};

use crate::raw::{RawConfidence, RawDiagnosis, RawPlaybook};
use crate::{LoadError, LoadedCatalog, REQUIRED_LOCALE, err, family};

/// Human-written texts of a diagnosis for one locale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosisCopy {
    pub title: String,
    pub explanation: String,
    /// Guidance the person can follow (N0); never executed by the app.
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosisRule {
    pub id: String,
    pub when_all: Vec<String>,
    pub when_any: Vec<String>,
    pub when_none: Vec<String>,
    pub confidence: Confidence,
    /// Catalog fix IDs proposed for this diagnosis (still validated by the Policy Engine).
    pub fixes: Vec<String>,
    pub report: Option<ReportTrigger>,
    pub copy: BTreeMap<String, DiagnosisCopy>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playbook {
    pub id: String,
    pub version: String,
    pub family: ProblemFamily,
    /// Probes run in this order.
    pub probes: Vec<String>,
    /// Evaluated in order; the first whose conditions hold wins.
    pub diagnoses: Vec<DiagnosisRule>,
    /// Used when no diagnosis matches.
    pub fallback: DiagnosisRule,
    /// Menu label per locale.
    pub title: BTreeMap<String, String>,
}

fn confidence(c: RawConfidence) -> Confidence {
    match c {
        RawConfidence::Unsure => Confidence::Unsure,
        RawConfidence::Probable => Confidence::Probable,
        RawConfidence::AlmostCertain => Confidence::AlmostCertain,
    }
}

fn report(code: &Option<String>) -> Result<Option<ReportTrigger>, String> {
    match code {
        None => Ok(None),
        Some(c) => ReportTrigger::ALL
            .into_iter()
            .find(|t| t.code() == c)
            .map(Some)
            .ok_or_else(|| format!("unknown report code `{c}`")),
    }
}

fn rule(raw: RawDiagnosis, file: &str, errors: &mut Vec<LoadError>) -> DiagnosisRule {
    let report = report(&raw.report).unwrap_or_else(|m| {
        errors.push(err(file, "PLAYBOOK_INVALID_REPORT", m));
        None
    });
    DiagnosisRule {
        id: raw.id,
        when_all: raw.when_all,
        when_any: raw.when_any,
        when_none: raw.when_none,
        confidence: confidence(raw.confidence),
        fixes: raw.fixes,
        report,
        copy: raw
            .copy
            .into_iter()
            .map(|(locale, c)| {
                (
                    locale,
                    DiagnosisCopy {
                        title: c.title,
                        explanation: c.explanation,
                        steps: c.steps,
                    },
                )
            })
            .collect(),
    }
}

/// Parses playbook files; structural errors only (cross-checks happen in [`validate`]).
pub(crate) fn parse(
    files: &[(&str, &str)],
    errors: &mut Vec<LoadError>,
) -> BTreeMap<String, Playbook> {
    let mut out = BTreeMap::new();
    for (file, content) in files {
        let raw: RawPlaybook = match toml::from_str(content) {
            Ok(raw) => raw,
            Err(e) => {
                errors.push(crate::parse_error(file, e));
                continue;
            }
        };
        if file.strip_suffix(".toml") != Some(raw.id.as_str()) {
            errors.push(err(
                file,
                "ID_FILENAME_MISMATCH",
                format!("id `{}` must match the file name", raw.id),
            ));
        }
        let playbook = Playbook {
            id: raw.id.clone(),
            version: raw.version,
            family: family(raw.family),
            probes: raw.probes,
            diagnoses: raw
                .diagnoses
                .into_iter()
                .map(|d| rule(d, file, errors))
                .collect(),
            fallback: rule(raw.fallback, file, errors),
            title: raw.copy.into_iter().map(|(l, c)| (l, c.title)).collect(),
        };
        if out.insert(raw.id.clone(), playbook).is_some() {
            errors.push(err(
                file,
                "DUPLICATE_ID",
                format!("`{}` defined twice", raw.id),
            ));
        }
    }
    out
}

/// FR-001 cross-checks against the capability catalog.
pub(crate) fn validate(
    playbooks: &BTreeMap<String, Playbook>,
    catalog: &LoadedCatalog,
    errors: &mut Vec<LoadError>,
) {
    let mut families = BTreeSet::new();
    for pb in playbooks.values() {
        let file = format!("{}.toml", pb.id);
        if !families.insert(pb.family.code()) {
            errors.push(err(
                &file,
                "PLAYBOOK_DUPLICATE_FAMILY",
                format!("family `{}` already has a playbook", pb.family.code()),
            ));
        }
        if !pb
            .title
            .get(REQUIRED_LOCALE)
            .is_some_and(|t| !t.trim().is_empty())
        {
            errors.push(err(
                &file,
                "PLAYBOOK_MISSING_COPY",
                "menu [copy.pt-BR] title",
            ));
        }
        let mut produced = BTreeSet::new();
        for probe in &pb.probes {
            match catalog.catalog.get(probe) {
                Some(c) if c.kind == CapabilityKind::Probe => {
                    if let Some(meta) = catalog.meta.get(probe) {
                        produced.extend(meta.produces.iter().map(String::as_str));
                    }
                }
                _ => errors.push(err(
                    &file,
                    "PLAYBOOK_UNKNOWN_PROBE",
                    format!("`{probe}` is not a catalog probe"),
                )),
            }
        }
        let mut ids = BTreeSet::new();
        for d in pb.diagnoses.iter().chain(std::iter::once(&pb.fallback)) {
            if !ids.insert(d.id.as_str()) {
                errors.push(err(&file, "DUPLICATE_ID", format!("diagnosis `{}`", d.id)));
            }
            for f in d.when_all.iter().chain(&d.when_any).chain(&d.when_none) {
                if !produced.contains(f.as_str()) {
                    errors.push(err(
                        &file,
                        "PLAYBOOK_UNKNOWN_FINDING",
                        format!("`{}`: no probe of this playbook produces `{f}`", d.id),
                    ));
                }
            }
            for fix in &d.fixes {
                if !catalog
                    .catalog
                    .get(fix)
                    .is_some_and(|c| c.kind == CapabilityKind::Fix)
                {
                    errors.push(err(
                        &file,
                        "PLAYBOOK_INVALID_FIX",
                        format!("`{}`: `{fix}` is not a catalog fix", d.id),
                    ));
                }
            }
            let complete = d
                .copy
                .get(REQUIRED_LOCALE)
                .is_some_and(|c| !c.title.trim().is_empty() && !c.explanation.trim().is_empty());
            if !complete {
                errors.push(err(
                    &file,
                    "PLAYBOOK_MISSING_COPY",
                    format!(
                        "`{}`: [copy.{REQUIRED_LOCALE}] needs title and explanation",
                        d.id
                    ),
                ));
            }
        }
        let fallback = &pb.fallback;
        if !(fallback.when_all.is_empty()
            && fallback.when_any.is_empty()
            && fallback.when_none.is_empty())
        {
            errors.push(err(
                &file,
                "PLAYBOOK_INVALID_FALLBACK",
                "fallback cannot have conditions",
            ));
        }
    }
}

/// FR-001: every family must have a playbook (menu completeness).
pub fn check_family_coverage(catalog: &LoadedCatalog) -> Vec<LoadError> {
    ProblemFamily::ALL
        .iter()
        .filter(|f| catalog.playbook_for(**f).is_none())
        .map(|f| {
            err(
                "playbooks",
                "MISSING_FAMILY_PLAYBOOK",
                format!("no playbook for `{}`", f.code()),
            )
        })
        .collect()
}
