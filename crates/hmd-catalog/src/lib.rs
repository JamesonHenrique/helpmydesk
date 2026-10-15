// SPDX-License-Identifier: GPL-3.0-or-later
//! Capability catalog (spec 003): parses the TOML files of `catalog/`, maps them to
//! `hmd_policy` types, validates everything and exposes the copy embedded at build time.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod playbook;
mod raw;

pub use playbook::{DiagnosisCopy, DiagnosisRule, Playbook, check_family_coverage};

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;
use std::sync::OnceLock;

use hmd_domain::ProblemFamily;
use hmd_policy::{
    Capability, CapabilityKind, Catalog, DataLoss, Disruption, Platform, PlatformVariant,
    Preconditions, RiskLevel, Rollback, Scope, validate_catalog,
};
use raw::*;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

/// Locale required for every fix and guide in the MVP.
pub const REQUIRED_LOCALE: &str = "pt-BR";

/// Human-authored texts of a capability for one locale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copy {
    pub title: String,
    pub risk: Option<String>,
    pub undo: Option<String>,
    /// Probes only: what is read (spec 009).
    pub looks_at: Option<String>,
    /// Probes only: what is never kept or sent.
    pub never: Option<String>,
}

/// Text the person reads before granting a consent category (spec 009).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentNotice {
    /// Stored with every consent record (`notice_version`).
    pub version: String,
    pub copy: BTreeMap<String, ConsentCopy>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentCopy {
    pub title: String,
    pub text: String,
}

/// One line of the "o que eu olho" screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeDisclosure<'a> {
    pub probe_id: &'a str,
    pub title: &'a str,
    pub looks_at: &'a str,
    pub never: &'a str,
    /// Consent category needed on the given platform, if any.
    pub consent: Option<&'a str>,
}

/// Catalog data that the Policy Engine does not need but the app does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntryMeta {
    pub produces: Vec<String>,
    pub copy: BTreeMap<String, Copy>,
}

#[derive(Debug, Clone)]
pub struct LoadedCatalog {
    pub version: String,
    pub catalog: Catalog,
    pub meta: BTreeMap<String, EntryMeta>,
    pub playbooks: BTreeMap<String, Playbook>,
    pub consents: BTreeMap<String, ConsentNotice>,
}

impl LoadedCatalog {
    /// The playbook of a problem family (one per family, spec 006).
    pub fn playbook_for(&self, family: ProblemFamily) -> Option<&Playbook> {
        self.playbooks.values().find(|p| p.family == family)
    }

    /// Spec 009 "o que eu olho": every probe available on `platform`, in catalog order, with its
    /// pt-BR disclosure (guaranteed present by validation).
    pub fn disclosures(&self, platform: Platform) -> Vec<ProbeDisclosure<'_>> {
        self.catalog
            .iter()
            .filter(|c| c.kind == CapabilityKind::Probe)
            .filter_map(|c| {
                let variant = c.variants.get(&platform)?;
                let copy = self.meta.get(&c.id)?.copy.get(REQUIRED_LOCALE)?;
                Some(ProbeDisclosure {
                    probe_id: &c.id,
                    title: &copy.title,
                    looks_at: copy.looks_at.as_deref()?,
                    never: copy.never.as_deref()?,
                    consent: variant.consent_category.as_deref(),
                })
            })
            .collect()
    }

    /// Every finding code some probe declares in `produces`.
    pub fn known_findings(&self) -> BTreeSet<&str> {
        self.meta
            .values()
            .flat_map(|m| m.produces.iter().map(String::as_str))
            .collect()
    }
}

/// A problem found while loading; `file` is the catalog file name (or `catalog.toml`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    pub file: String,
    pub code: &'static str,
    pub message: String,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}: {}", self.file, self.code, self.message)
    }
}

impl std::error::Error for LoadError {}

pub(crate) fn err(file: &str, code: &'static str, message: impl Into<String>) -> LoadError {
    LoadError {
        file: file.to_string(),
        code,
        message: message.into(),
    }
}

pub(crate) fn parse_error(file: &str, e: toml::de::Error) -> LoadError {
    let message = e.to_string();
    let code = if message.contains("unknown field") {
        "UNKNOWN_FIELD"
    } else {
        "PARSE_ERROR"
    };
    err(file, code, message)
}

/// FR-001..FR-004: loads a catalog from the manifest text and `(file name, content)` pairs.
/// Returns every error found, not only the first.
pub fn load_from_sources(
    manifest: &str,
    files: &[(&str, &str)],
) -> Result<LoadedCatalog, Vec<LoadError>> {
    load(manifest, files, &[])
}

/// Loads capabilities and playbooks (spec 006 FR-001) from in-memory sources.
pub fn load(
    manifest: &str,
    files: &[(&str, &str)],
    playbook_files: &[(&str, &str)],
) -> Result<LoadedCatalog, Vec<LoadError>> {
    let mut errors = Vec::new();
    let (version, consents) = match toml::from_str::<RawManifest>(manifest) {
        Ok(m) => {
            if !is_catalog_version(&m.version) {
                errors.push(err(
                    "catalog.toml",
                    "INVALID_MANIFEST",
                    format!("version `{}` is not YYYY.MM.patch", m.version),
                ));
            }
            (m.version, consents(m.consents, &mut errors))
        }
        Err(e) => {
            errors.push(parse_error("catalog.toml", e));
            (String::new(), BTreeMap::new())
        }
    };

    let mut caps = Vec::new();
    let mut meta = BTreeMap::new();
    let mut origin: BTreeMap<String, String> = BTreeMap::new();
    for (file, content) in files {
        let raw = match toml::from_str::<RawCapability>(content) {
            Ok(raw) => raw,
            Err(e) => {
                errors.push(parse_error(file, e));
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
        if let Some(first) = origin.insert(raw.id.clone(), file.to_string()) {
            errors.push(err(
                file,
                "DUPLICATE_ID",
                format!("`{}` already defined in {first}", raw.id),
            ));
            continue;
        }
        let (cap, entry) = convert(raw);
        meta.insert(cap.id.clone(), entry);
        caps.push(cap);
    }

    let catalog = Catalog::new(caps);
    let file_of = |id: &str| origin.get(id).cloned().unwrap_or_else(|| id.to_string());

    // Effects vs declared level, fixes without verification, empty variants (hmd-policy).
    for e in validate_catalog(&catalog) {
        let id = match &e {
            hmd_policy::CatalogError::LevelBelowMinimum { capability, .. }
            | hmd_policy::CatalogError::FixWithoutVerification { capability, .. }
            | hmd_policy::CatalogError::NoVariants { capability } => capability.clone(),
        };
        errors.push(err(&file_of(&id), e.reason_code(), e.to_string()));
    }

    let known: BTreeSet<&str> = meta
        .values()
        .flat_map(|m: &EntryMeta| m.produces.iter().map(String::as_str))
        .collect();
    for cap in catalog.iter() {
        let file = file_of(&cap.id);
        cross_check(cap, &catalog, &known, &meta, &file, &mut errors);
        for v in cap.variants.values() {
            if let Some(category) = &v.consent_category
                && !consents.contains_key(category)
            {
                errors.push(err(
                    &file,
                    "UNKNOWN_CONSENT",
                    format!("consent category `{category}` is not declared in catalog.toml"),
                ));
            }
        }
    }

    let mut loaded = LoadedCatalog {
        version,
        catalog,
        meta,
        playbooks: BTreeMap::new(),
        consents,
    };
    let playbooks = playbook::parse(playbook_files, &mut errors);
    playbook::validate(&playbooks, &loaded, &mut errors);
    loaded.playbooks = playbooks;

    if errors.is_empty() {
        Ok(loaded)
    } else {
        Err(errors)
    }
}

/// User Story 1 cross-reference checks.
fn cross_check(
    cap: &Capability,
    catalog: &Catalog,
    known: &BTreeSet<&str>,
    meta: &BTreeMap<String, EntryMeta>,
    file: &str,
    errors: &mut Vec<LoadError>,
) {
    let entry = meta.get(&cap.id);
    if cap.kind != CapabilityKind::Probe && entry.is_some_and(|m| !m.produces.is_empty()) {
        errors.push(err(
            file,
            "INVALID_PRODUCES",
            "only probes may declare `produces`",
        ));
    }
    for other in &cap.exclusive_with {
        if catalog.get(other).is_none() {
            errors.push(err(
                file,
                "UNKNOWN_REFERENCE",
                format!("exclusive_with `{other}`"),
            ));
        }
    }
    for (platform, v) in &cap.variants {
        for finding in v
            .preconditions
            .all_of
            .iter()
            .chain(&v.preconditions.none_of)
        {
            if !known.contains(finding.as_str()) {
                errors.push(err(
                    file,
                    "UNKNOWN_FINDING",
                    format!("{platform:?}: no probe produces `{finding}`"),
                ));
            }
        }
        if let Some(probe) = &v.verification {
            let ok = catalog.get(probe).is_some_and(|p| {
                p.kind == CapabilityKind::Probe && p.variants.contains_key(platform)
            });
            if !ok {
                errors.push(err(
                    file,
                    "INVALID_VERIFICATION",
                    format!("{platform:?}: `{probe}` is not a probe available on this platform"),
                ));
            }
            // Spec 016: "verified" means the verification probe no longer reports the problem,
            // so it must be able to report it at all.
            let seen = meta.get(probe.as_str()).map(|m| &m.produces);
            let all_of = &v.preconditions.all_of;
            let checkable = ok && all_of.iter().all(|f| known.contains(f.as_str()));
            if checkable
                && (all_of.is_empty()
                    || !all_of
                        .iter()
                        .all(|f| seen.is_some_and(|p| p.iter().any(|x| x == f))))
            {
                errors.push(err(
                    file,
                    "VACUOUS_VERIFICATION",
                    format!(
                        "{platform:?}: `{probe}` cannot report every `all_of` finding, so it would \
                         prove any fix"
                    ),
                ));
            }
        }
    }
    if cap.kind == CapabilityKind::Probe {
        let disclosed = entry
            .and_then(|m| m.copy.get(REQUIRED_LOCALE))
            .is_some_and(|c| {
                !c.title.trim().is_empty()
                    && c.looks_at.as_deref().is_some_and(|t| !t.trim().is_empty())
                    && c.never.as_deref().is_some_and(|t| !t.trim().is_empty())
            });
        if !disclosed {
            errors.push(err(
                file,
                "PROBE_MISSING_DISCLOSURE",
                format!("[copy.{REQUIRED_LOCALE}] of a probe needs title, looks_at and never"),
            ));
        }
    }
    if cap.kind != CapabilityKind::Probe {
        let complete = entry
            .and_then(|m| m.copy.get(REQUIRED_LOCALE))
            .is_some_and(|c| !c.title.trim().is_empty() && c.risk.is_some() && c.undo.is_some());
        if !complete {
            errors.push(err(
                file,
                "MISSING_COPY",
                format!("[copy.{REQUIRED_LOCALE}] needs title, risk and undo"),
            ));
        }
    }
}

/// Spec 009: consent notices from the manifest; each needs a version and pt-BR title and text.
fn consents(
    raw: BTreeMap<String, crate::raw::RawConsent>,
    errors: &mut Vec<LoadError>,
) -> BTreeMap<String, ConsentNotice> {
    raw.into_iter()
        .map(|(id, c)| {
            let complete = !c.version.trim().is_empty()
                && c.copy
                    .get(REQUIRED_LOCALE)
                    .is_some_and(|t| !t.title.trim().is_empty() && !t.text.trim().is_empty());
            if !complete {
                errors.push(err(
                    "catalog.toml",
                    "CONSENT_MISSING_COPY",
                    format!(
                        "consent `{id}` needs version and [copy.{REQUIRED_LOCALE}] title and text"
                    ),
                ));
            }
            let notice = ConsentNotice {
                version: c.version,
                copy: c
                    .copy
                    .into_iter()
                    .map(|(l, t)| {
                        (
                            l,
                            ConsentCopy {
                                title: t.title,
                                text: t.text,
                            },
                        )
                    })
                    .collect(),
            };
            (id, notice)
        })
        .collect()
}

fn is_catalog_version(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    parts.len() == 3
        && parts[0].len() == 4
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        && parts[1].parse::<u8>().is_ok_and(|m| (1..=12).contains(&m))
}

fn convert(raw: RawCapability) -> (Capability, EntryMeta) {
    let variants = raw
        .variants
        .into_iter()
        .map(|(p, v)| (platform(p), variant(v)))
        .collect();
    let cap = Capability {
        id: raw.id,
        version: raw.version,
        kind: match raw.kind {
            RawKind::Probe => CapabilityKind::Probe,
            RawKind::Fix => CapabilityKind::Fix,
            RawKind::Guide => CapabilityKind::Guide,
        },
        family: raw.family.map(family),
        variants,
        exclusive_with: raw.exclusive_with,
    };
    let meta = EntryMeta {
        produces: raw.produces,
        copy: raw
            .copy
            .into_iter()
            .map(|(locale, c)| {
                (
                    locale,
                    Copy {
                        title: c.title,
                        risk: c.risk,
                        undo: c.undo,
                        looks_at: c.looks_at,
                        never: c.never,
                    },
                )
            })
            .collect(),
    };
    (cap, meta)
}

pub(crate) fn family(f: RawFamily) -> ProblemFamily {
    match f {
        RawFamily::Connectivity => ProblemFamily::Connectivity,
        RawFamily::Performance => ProblemFamily::Performance,
        RawFamily::Storage => ProblemFamily::Storage,
        RawFamily::AudioVideo => ProblemFamily::AudioVideo,
        RawFamily::AppError => ProblemFamily::AppError,
    }
}

fn platform(p: RawPlatform) -> Platform {
    match p {
        RawPlatform::Windows => Platform::Windows,
        RawPlatform::Linux => Platform::Linux,
    }
}

fn variant(v: RawVariant) -> PlatformVariant {
    PlatformVariant {
        risk_level: match v.risk_level {
            RawLevel::N0 => RiskLevel::N0,
            RawLevel::N1A => RiskLevel::N1A,
            RawLevel::N1B => RiskLevel::N1B,
            RawLevel::N1S => RiskLevel::N1S,
            RawLevel::N2 => RiskLevel::N2,
            RawLevel::N3 => RiskLevel::N3,
            RawLevel::N4 => RiskLevel::N4,
        },
        requires_elevation: v.requires_elevation,
        data_loss: match v.data_loss {
            RawDataLoss::None => DataLoss::None,
            RawDataLoss::Some => DataLoss::Some,
        },
        disruption: match v.disruption {
            RawDisruption::None => Disruption::None,
            RawDisruption::Brief => Disruption::Brief,
            RawDisruption::Noticeable => Disruption::Noticeable,
        },
        persistent_change: match v.persistent_change {
            RawScope::None => Scope::None,
            RawScope::User => Scope::User,
            RawScope::System => Scope::System,
        },
        rollback: match v.rollback {
            RawRollback::NotNeeded => Rollback::NotNeeded,
            RawRollback::Implemented => Rollback::Implemented,
            RawRollback::Impossible => Rollback::Impossible,
        },
        verification: v.verification,
        preconditions: Preconditions {
            all_of: v.preconditions.all_of,
            none_of: v.preconditions.none_of,
        },
        params: v.params,
        max_per_session: v.max_per_session,
        consent_category: v.consent_category,
        auto_eligible: v.auto_eligible,
    }
}

/// Loads `catalog.toml` and `capabilities/*.toml` from a directory (development and tests).
pub fn load_from_dir(dir: &Path) -> Result<LoadedCatalog, Vec<LoadError>> {
    let io = |file: &str, e: std::io::Error| vec![err(file, "IO_ERROR", e.to_string())];
    let manifest =
        std::fs::read_to_string(dir.join("catalog.toml")).map_err(|e| io("catalog.toml", e))?;
    let mut owned = Vec::new();
    let entries = std::fs::read_dir(dir.join("capabilities")).map_err(|e| io("capabilities", e))?;
    for entry in entries {
        let path = entry.map_err(|e| io("capabilities", e))?.path();
        if path.extension().is_some_and(|x| x == "toml") {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            let content = std::fs::read_to_string(&path).map_err(|e| io(&name, e))?;
            owned.push((name, content));
        }
    }
    owned.sort();
    let playbooks = read_toml_dir(&dir.join("playbooks"))?;
    let files: Vec<(&str, &str)> = owned
        .iter()
        .map(|(n, c)| (n.as_str(), c.as_str()))
        .collect();
    let pb: Vec<(&str, &str)> = playbooks
        .iter()
        .map(|(n, c)| (n.as_str(), c.as_str()))
        .collect();
    with_coverage(load(&manifest, &files, &pb))
}

fn read_toml_dir(dir: &Path) -> Result<Vec<(String, String)>, Vec<LoadError>> {
    let io = |file: &str, e: std::io::Error| vec![err(file, "IO_ERROR", e.to_string())];
    let mut owned = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| io("playbooks", e))? {
        let path = entry.map_err(|e| io("playbooks", e))?.path();
        if path.extension().is_some_and(|x| x == "toml") {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            let content = std::fs::read_to_string(&path).map_err(|e| io(&name, e))?;
            owned.push((name, content));
        }
    }
    owned.sort();
    Ok(owned)
}

fn with_coverage(
    result: Result<LoadedCatalog, Vec<LoadError>>,
) -> Result<LoadedCatalog, Vec<LoadError>> {
    let loaded = result?;
    let missing = check_family_coverage(&loaded);
    if missing.is_empty() {
        Ok(loaded)
    } else {
        Err(missing)
    }
}

/// FR-005: the catalog compiled into this binary, validated once.
pub fn embedded() -> Result<&'static LoadedCatalog, &'static [LoadError]> {
    static CELL: OnceLock<Result<LoadedCatalog, Vec<LoadError>>> = OnceLock::new();
    match CELL.get_or_init(|| {
        with_coverage(load(
            embedded::MANIFEST,
            embedded::FILES,
            embedded::PLAYBOOK_FILES,
        ))
    }) {
        Ok(c) => Ok(c),
        Err(e) => Err(e.as_slice()),
    }
}
