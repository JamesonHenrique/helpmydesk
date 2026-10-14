// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 003 acceptance tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use hmd_catalog::{LoadError, embedded, load_from_dir, load_from_sources};
use hmd_policy::{CapabilityKind, Platform, RiskLevel};

const MANIFEST: &str = r#"version = "2026.10.0""#;

const PROBE: &str = r#"
id = "net.dns.resolve"
version = "1.0.0"
kind = "probe"
produces = ["net.dns.resolution_failing"]
[variants.linux]
risk_level = "N1-B"
"#;

const FIX: &str = r#"
id = "net.dns.flush_cache"
version = "1.0.0"
kind = "fix"
[variants.linux]
risk_level = "N3"
requires_elevation = true
verification = "net.dns.resolve"
[variants.linux.preconditions]
all_of = ["net.dns.resolution_failing"]
[copy.pt-BR]
title = "Limpar"
risk = "Baixo"
undo = "Não precisa"
"#;

fn codes(result: Result<hmd_catalog::LoadedCatalog, Vec<LoadError>>) -> Vec<&'static str> {
    match result {
        Ok(_) => vec![],
        Err(errors) => errors.into_iter().map(|e| e.code).collect(),
    }
}

fn with_fix(fix: &str) -> Vec<&'static str> {
    codes(load_from_sources(
        MANIFEST,
        &[
            ("net.dns.resolve.toml", PROBE),
            ("net.dns.flush_cache.toml", fix),
        ],
    ))
}

#[test]
fn fr002_valid_sources_map_to_policy_types() {
    let loaded = load_from_sources(
        MANIFEST,
        &[
            ("net.dns.resolve.toml", PROBE),
            ("net.dns.flush_cache.toml", FIX),
        ],
    )
    .unwrap();
    assert_eq!(loaded.version, "2026.10.0");
    let fix = loaded.catalog.get("net.dns.flush_cache").unwrap();
    assert_eq!(fix.kind, CapabilityKind::Fix);
    let v = &fix.variants[&Platform::Linux];
    assert_eq!(v.risk_level, RiskLevel::N3);
    assert!(v.requires_elevation);
    assert_eq!(v.max_per_session, 1, "default max_per_session");
    assert_eq!(
        loaded.meta["net.dns.flush_cache"].copy["pt-BR"].title,
        "Limpar"
    );
    assert!(
        loaded
            .known_findings()
            .contains("net.dns.resolution_failing")
    );
}

#[test]
fn us1_unknown_field_is_rejected() {
    let fix = FIX.replace("kind = \"fix\"", "kind = \"fix\"\nrun = \"rm -rf /\"");
    assert_eq!(with_fix(&fix), vec!["UNKNOWN_FIELD"]);
}

#[test]
fn us1_id_must_match_file_name() {
    let c = codes(load_from_sources(MANIFEST, &[("other.toml", PROBE)]));
    assert_eq!(c, vec!["ID_FILENAME_MISMATCH"]);
}

#[test]
fn us1_level_below_minimum() {
    assert_eq!(
        with_fix(&FIX.replace("\"N3\"", "\"N2\"")),
        vec!["LEVEL_BELOW_MINIMUM"]
    );
}

#[test]
fn us1_fix_without_verification() {
    let fix = FIX.replace("verification = \"net.dns.resolve\"\n", "");
    assert_eq!(with_fix(&fix), vec!["FIX_WITHOUT_VERIFICATION"]);
}

#[test]
fn us1_unknown_finding() {
    let fix = FIX.replace(
        "all_of = [\"net.dns.resolution_failing\"]",
        "all_of = [\"made.up\"]",
    );
    assert_eq!(with_fix(&fix), vec!["UNKNOWN_FINDING"]);
}

#[test]
fn us1_verification_must_be_probe() {
    let fix = FIX.replace(
        "verification = \"net.dns.resolve\"",
        "verification = \"net.dns.flush_cache\"",
    );
    assert_eq!(with_fix(&fix), vec!["INVALID_VERIFICATION"]);
}

#[test]
fn us1_unknown_exclusive_reference() {
    let fix = FIX.replace(
        "kind = \"fix\"",
        "kind = \"fix\"\nexclusive_with = [\"nope\"]",
    );
    assert_eq!(with_fix(&fix), vec!["UNKNOWN_REFERENCE"]);
}

#[test]
fn us1_missing_copy() {
    let fix = FIX.replace("undo = \"Não precisa\"\n", "");
    assert_eq!(with_fix(&fix), vec!["MISSING_COPY"]);
}

#[test]
fn edge_duplicate_id_and_bad_enum_and_manifest() {
    let c = codes(load_from_sources(
        MANIFEST,
        &[
            ("net.dns.resolve.toml", PROBE),
            ("net.dns.resolve.toml", PROBE),
        ],
    ));
    assert!(c.contains(&"DUPLICATE_ID"));

    let bad_level = PROBE.replace("N1-B", "N9");
    let c = codes(load_from_sources(
        MANIFEST,
        &[("net.dns.resolve.toml", &bad_level)],
    ));
    assert_eq!(c, vec!["PARSE_ERROR"]);

    let c = codes(load_from_sources(
        r#"version = "2026.13.0""#,
        &[("net.dns.resolve.toml", PROBE)],
    ));
    assert_eq!(c, vec!["INVALID_MANIFEST"]);
}

#[test]
fn edge_only_probes_produce_findings() {
    let fix = FIX.replace("kind = \"fix\"", "kind = \"fix\"\nproduces = [\"x\"]");
    assert_eq!(with_fix(&fix), vec!["INVALID_PRODUCES"]);
}

#[test]
fn errors_name_the_file() {
    let fix = FIX.replace("\"N3\"", "\"N2\"");
    let errs = load_from_sources(
        MANIFEST,
        &[
            ("net.dns.resolve.toml", PROBE),
            ("net.dns.flush_cache.toml", &fix),
        ],
    )
    .unwrap_err();
    assert_eq!(errs[0].file, "net.dns.flush_cache.toml");
}

/// FR-006 / SC-002: the real repository catalog is valid.
#[test]
fn fr006_repository_catalog_is_valid() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../catalog");
    if let Err(errors) = load_from_dir(&dir) {
        panic!(
            "catalog/ is invalid:\n{}",
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// US2: the embedded catalog equals the directory catalog.
#[test]
fn us2_embedded_matches_directory() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../catalog");
    let disk = load_from_dir(&dir).unwrap();
    let emb = embedded().unwrap();
    assert_eq!(emb.version, disk.version);
    assert_eq!(emb.meta, disk.meta);
    let ids = |c: &hmd_policy::Catalog| c.iter().cloned().collect::<Vec<_>>();
    assert_eq!(ids(&emb.catalog), ids(&disk.catalog));
}
