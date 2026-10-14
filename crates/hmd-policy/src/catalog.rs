// SPDX-License-Identifier: GPL-3.0-or-later
//! Typed catalog entries, minimum risk level and catalog validation (spec 002, FR-003, FR-011).

use std::collections::BTreeMap;
use std::fmt;

use hmd_domain::ProblemFamily;

/// Risk levels of Plan v1 V.2, ordered from least to most risky.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RiskLevel {
    N0,
    N1A,
    N1B,
    N1S,
    N2,
    N3,
    N4,
}

impl RiskLevel {
    pub const ALL: [RiskLevel; 7] = [
        RiskLevel::N0,
        RiskLevel::N1A,
        RiskLevel::N1B,
        RiskLevel::N1S,
        RiskLevel::N2,
        RiskLevel::N3,
        RiskLevel::N4,
    ];

    pub fn code(self) -> &'static str {
        match self {
            RiskLevel::N0 => "N0",
            RiskLevel::N1A => "N1-A",
            RiskLevel::N1B => "N1-B",
            RiskLevel::N1S => "N1-S",
            RiskLevel::N2 => "N2",
            RiskLevel::N3 => "N3",
            RiskLevel::N4 => "N4",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Platform {
    Windows,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityKind {
    Probe,
    Fix,
    Guide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataLoss {
    None,
    Some,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disruption {
    None,
    Brief,
    Noticeable,
}

/// Scope of a persistent change, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    None,
    User,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rollback {
    NotNeeded,
    Implemented,
    Impossible,
}

/// Findings that must be present (`all_of`) or absent (`none_of`) in the session.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Preconditions {
    pub all_of: Vec<String>,
    pub none_of: Vec<String>,
}

/// Per-platform data of a capability; this is where every safety property lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformVariant {
    pub risk_level: RiskLevel,
    pub requires_elevation: bool,
    pub data_loss: DataLoss,
    pub disruption: Disruption,
    pub persistent_change: Scope,
    pub rollback: Rollback,
    /// Probe that proves the post-condition; mandatory for N2/N3 fixes (INV-05).
    pub verification: Option<String>,
    pub preconditions: Preconditions,
    /// Allowed parameter names; anything else is rejected.
    pub params: Vec<String>,
    pub max_per_session: u32,
    /// Data category consent needed for N1-S probes (e.g. `installed_apps`).
    pub consent_category: Option<String>,
    /// May run without approval in `AutoSimple` mode (gate G-AUTO passed).
    pub auto_eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    pub id: String,
    pub version: String,
    pub kind: CapabilityKind,
    pub family: Option<ProblemFamily>,
    pub variants: BTreeMap<Platform, PlatformVariant>,
    /// Capability IDs that must not appear in the same plan.
    pub exclusive_with: Vec<String>,
}

/// The loaded, signature-verified catalog (loading/verification is a later feature).
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    capabilities: BTreeMap<String, Capability>,
}

impl Catalog {
    pub fn new(capabilities: impl IntoIterator<Item = Capability>) -> Self {
        Catalog {
            capabilities: capabilities
                .into_iter()
                .map(|c| (c.id.clone(), c))
                .collect(),
        }
    }

    pub fn get(&self, id: &str) -> Option<&Capability> {
        self.capabilities.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        self.capabilities.values()
    }
}

/// FR-011: lowest risk level the effects of a variant allow (SEC-012 of the plan).
pub fn min_level(kind: CapabilityKind, v: &PlatformVariant) -> RiskLevel {
    let base = match kind {
        CapabilityKind::Guide => RiskLevel::N0,
        CapabilityKind::Probe => RiskLevel::N1A,
        CapabilityKind::Fix => RiskLevel::N2,
    };
    let elevation = if v.requires_elevation {
        RiskLevel::N3
    } else {
        RiskLevel::N0
    };
    let data_loss = match v.data_loss {
        DataLoss::None => RiskLevel::N0,
        DataLoss::Some => RiskLevel::N3,
    };
    let disruption = match v.disruption {
        Disruption::None => RiskLevel::N0,
        Disruption::Brief => RiskLevel::N2,
        Disruption::Noticeable => RiskLevel::N3,
    };
    let persistent = match v.persistent_change {
        Scope::None => RiskLevel::N0,
        Scope::User => RiskLevel::N2,
        Scope::System => RiskLevel::N3,
    };
    base.max(elevation)
        .max(data_loss)
        .max(disruption)
        .max(persistent)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    LevelBelowMinimum {
        capability: String,
        platform: Platform,
        declared: RiskLevel,
        minimum: RiskLevel,
    },
    FixWithoutVerification {
        capability: String,
        platform: Platform,
    },
    NoVariants {
        capability: String,
    },
}

impl CatalogError {
    pub fn reason_code(&self) -> &'static str {
        match self {
            CatalogError::LevelBelowMinimum { .. } => "LEVEL_BELOW_MINIMUM",
            CatalogError::FixWithoutVerification { .. } => "FIX_WITHOUT_VERIFICATION",
            CatalogError::NoVariants { .. } => "NO_VARIANTS",
        }
    }
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatalogError::LevelBelowMinimum {
                capability,
                platform,
                declared,
                minimum,
            } => write!(
                f,
                "{}: {capability} ({platform:?}) declares {} but effects require {}",
                self.reason_code(),
                declared.code(),
                minimum.code()
            ),
            CatalogError::FixWithoutVerification {
                capability,
                platform,
            } => write!(f, "{}: {capability} ({platform:?})", self.reason_code()),
            CatalogError::NoVariants { capability } => {
                write!(f, "{}: {capability}", self.reason_code())
            }
        }
    }
}

impl std::error::Error for CatalogError {}

/// FR-011 / US5: returns every problem found; an empty vector means the catalog is acceptable.
pub fn validate_catalog(catalog: &Catalog) -> Vec<CatalogError> {
    let mut errors = Vec::new();
    for cap in catalog.iter() {
        if cap.variants.is_empty() {
            errors.push(CatalogError::NoVariants {
                capability: cap.id.clone(),
            });
        }
        for (&platform, v) in &cap.variants {
            let minimum = min_level(cap.kind, v);
            if v.risk_level < minimum {
                errors.push(CatalogError::LevelBelowMinimum {
                    capability: cap.id.clone(),
                    platform,
                    declared: v.risk_level,
                    minimum,
                });
            }
            let executable = matches!(v.risk_level, RiskLevel::N2 | RiskLevel::N3);
            if cap.kind == CapabilityKind::Fix && executable && v.verification.is_none() {
                errors.push(CatalogError::FixWithoutVerification {
                    capability: cap.id.clone(),
                    platform,
                });
            }
        }
    }
    errors
}
