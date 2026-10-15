// SPDX-License-Identifier: GPL-3.0-or-later
//! On-disk TOML shape (spec 003 FR-001). Every struct denies unknown fields.

use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawManifest {
    pub version: String,
    /// Consent notices by category (spec 009).
    #[serde(default)]
    pub consents: BTreeMap<String, RawConsent>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawConsent {
    pub version: String,
    #[serde(default)]
    pub copy: BTreeMap<String, RawConsentCopy>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawConsentCopy {
    pub title: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawCapability {
    pub id: String,
    pub version: String,
    pub kind: RawKind,
    #[serde(default)]
    pub family: Option<RawFamily>,
    #[serde(default)]
    pub exclusive_with: Vec<String>,
    #[serde(default)]
    pub produces: Vec<String>,
    #[serde(default)]
    pub variants: BTreeMap<RawPlatform, RawVariant>,
    #[serde(default)]
    pub copy: BTreeMap<String, RawCopy>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RawKind {
    Probe,
    Fix,
    Guide,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RawFamily {
    Connectivity,
    Performance,
    Storage,
    AudioVideo,
    AppError,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RawPlatform {
    Windows,
    Linux,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) enum RawLevel {
    N0,
    #[serde(rename = "N1-A")]
    N1A,
    #[serde(rename = "N1-B")]
    N1B,
    #[serde(rename = "N1-S")]
    N1S,
    N2,
    N3,
    N4,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RawDataLoss {
    #[default]
    None,
    Some,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RawDisruption {
    #[default]
    None,
    Brief,
    Noticeable,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RawScope {
    #[default]
    None,
    User,
    System,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RawRollback {
    #[default]
    NotNeeded,
    Implemented,
    Impossible,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawPreconditions {
    #[serde(default)]
    pub all_of: Vec<String>,
    #[serde(default)]
    pub none_of: Vec<String>,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawVariant {
    pub risk_level: RawLevel,
    #[serde(default)]
    pub requires_elevation: bool,
    #[serde(default)]
    pub data_loss: RawDataLoss,
    #[serde(default)]
    pub disruption: RawDisruption,
    #[serde(default)]
    pub persistent_change: RawScope,
    #[serde(default)]
    pub rollback: RawRollback,
    #[serde(default)]
    pub verification: Option<String>,
    #[serde(default)]
    pub preconditions: RawPreconditions,
    #[serde(default)]
    pub params: Vec<String>,
    #[serde(default = "one")]
    pub max_per_session: u32,
    #[serde(default)]
    pub consent_category: Option<String>,
    #[serde(default)]
    pub auto_eligible: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawCopy {
    pub title: String,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub undo: Option<String>,
    /// Probes: what is read, in lay words (spec 009 "o que eu olho").
    #[serde(default)]
    pub looks_at: Option<String>,
    /// Probes: what is never kept or sent (it may be read only to decide, e.g. an IP).
    #[serde(default)]
    pub never: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RawConfidence {
    Unsure,
    Probable,
    AlmostCertain,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawDiagnosisCopy {
    pub title: String,
    pub explanation: String,
    #[serde(default)]
    pub steps: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawDiagnosis {
    pub id: String,
    #[serde(default)]
    pub when_all: Vec<String>,
    #[serde(default)]
    pub when_any: Vec<String>,
    #[serde(default)]
    pub when_none: Vec<String>,
    pub confidence: RawConfidence,
    #[serde(default)]
    pub fixes: Vec<String>,
    #[serde(default)]
    pub report: Option<String>,
    #[serde(default)]
    pub copy: BTreeMap<String, RawDiagnosisCopy>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawPlaybookCopy {
    pub title: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawPlaybook {
    pub id: String,
    pub version: String,
    pub family: RawFamily,
    #[serde(default)]
    pub probes: Vec<String>,
    #[serde(default)]
    pub diagnoses: Vec<RawDiagnosis>,
    pub fallback: RawDiagnosis,
    #[serde(default)]
    pub copy: BTreeMap<String, RawPlaybookCopy>,
}
