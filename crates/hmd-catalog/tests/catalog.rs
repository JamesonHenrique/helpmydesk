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
[copy.pt-BR]
title = "DNS"
looks_at = "Nomes fixos"
never = "Histórico"
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

/// Spec 016: a verification that cannot see the problem would "prove" any fix.
#[test]
fn verification_must_be_able_to_see_the_problem() {
    let empty = FIX.replace("all_of = [\"net.dns.resolution_failing\"]", "all_of = []");
    assert_eq!(with_fix(&empty), vec!["VACUOUS_VERIFICATION"]);
    let other_probe = FIX.replace(
        "all_of = [\"net.dns.resolution_failing\"]",
        "all_of = [\"x.other\"]",
    );
    let other = r#"
id = "x.other.probe"
version = "1.0.0"
kind = "probe"
produces = ["x.other"]
[variants.linux]
risk_level = "N1-B"
[copy.pt-BR]
title = "Outra"
looks_at = "Algo"
never = "Nada"
"#;
    let c = codes(load_from_sources(
        MANIFEST,
        &[
            ("net.dns.resolve.toml", PROBE),
            ("x.other.probe.toml", other),
            ("net.dns.flush_cache.toml", &other_probe),
        ],
    ));
    assert_eq!(c, vec!["VACUOUS_VERIFICATION"]);
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

// ---- Spec 006: playbook validation ----

const PB_OK: &str = r#"
id = "PB-NET-001"
version = "1.0.0"
family = "connectivity"
probes = ["net.dns.resolve"]
[copy.pt-BR]
title = "Internet"
[[diagnoses]]
id = "net.dns"
when_all = ["net.dns.resolution_failing"]
confidence = "probable"
fixes = ["net.dns.flush_cache"]
[diagnoses.copy.pt-BR]
title = "T"
explanation = "E"
[fallback]
id = "net.ok"
confidence = "probable"
report = "REP-01"
[fallback.copy.pt-BR]
title = "T"
explanation = "E"
"#;

fn pb_codes(playbooks: &[(&str, &str)]) -> Vec<&'static str> {
    codes(hmd_catalog::load(
        MANIFEST,
        &[
            ("net.dns.resolve.toml", PROBE),
            ("net.dns.flush_cache.toml", FIX),
        ],
        playbooks,
    ))
}

fn pb_with(from: &str, to: &str) -> Vec<&'static str> {
    let pb = PB_OK.replacen(from, to, 1);
    assert_ne!(pb, PB_OK, "fixture replacement did not apply: {from}");
    pb_codes(&[("PB-NET-001.toml", &pb)])
}

#[test]
fn fr001_valid_playbook_loads() {
    assert_eq!(pb_codes(&[("PB-NET-001.toml", PB_OK)]), Vec::<&str>::new());
}

#[test]
fn fr001_playbook_errors() {
    assert_eq!(
        pb_with(
            "probes = [\"net.dns.resolve\"]",
            "probes = [\"net.dns.flush_cache\"]"
        )[0],
        "PLAYBOOK_UNKNOWN_PROBE"
    );
    assert_eq!(
        pb_with(
            "when_all = [\"net.dns.resolution_failing\"]",
            "when_all = [\"made.up\"]"
        ),
        vec!["PLAYBOOK_UNKNOWN_FINDING"]
    );
    assert_eq!(
        pb_with(
            "fixes = [\"net.dns.flush_cache\"]",
            "fixes = [\"net.dns.resolve\"]"
        ),
        vec!["PLAYBOOK_INVALID_FIX"]
    );
    assert_eq!(
        pb_with("report = \"REP-01\"", "report = \"REP-99\""),
        vec!["PLAYBOOK_INVALID_REPORT"]
    );
    assert_eq!(
        pb_with("[copy.pt-BR]\ntitle = \"Internet\"", ""),
        vec!["PLAYBOOK_MISSING_COPY"]
    );
    assert_eq!(
        pb_with(
            "[fallback]\nid = \"net.ok\"",
            "[fallback]\nid = \"net.ok\"\nwhen_all = [\"net.dns.resolution_failing\"]"
        ),
        vec!["PLAYBOOK_INVALID_FALLBACK"]
    );
    assert_eq!(
        pb_with(
            "family = \"connectivity\"\nprobes",
            "family = \"connectivity\"\nrun = 1\nprobes"
        ),
        vec!["UNKNOWN_FIELD"]
    );
}

#[test]
fn fr001_one_playbook_per_family_and_coverage() {
    let second = PB_OK.replace("PB-NET-001", "PB-NET-002");
    let c = pb_codes(&[("PB-NET-001.toml", PB_OK), ("PB-NET-002.toml", &second)]);
    assert_eq!(c, vec!["PLAYBOOK_DUPLICATE_FAMILY"]);

    let loaded = hmd_catalog::load(
        MANIFEST,
        &[
            ("net.dns.resolve.toml", PROBE),
            ("net.dns.flush_cache.toml", FIX),
        ],
        &[("PB-NET-001.toml", PB_OK)],
    )
    .unwrap();
    let missing: Vec<_> = hmd_catalog::check_family_coverage(&loaded)
        .into_iter()
        .map(|e| e.code)
        .collect();
    assert_eq!(missing, vec!["MISSING_FAMILY_PLAYBOOK"; 4]);
}

const CONSENT_MANIFEST: &str = r#"
version = "2026.10.0"
[consents.startup_apps]
version = "1"
[consents.startup_apps.copy.pt-BR]
title = "T"
text = "X"
"#;

const SENSITIVE: &str = r#"
id = "startup.apps.list"
version = "1.0.0"
kind = "probe"
[variants.linux]
risk_level = "N1-S"
consent_category = "startup_apps"
[copy.pt-BR]
title = "Inicialização"
looks_at = "Nomes"
never = "Arquivos"
"#;

/// Spec 009 US1: every probe says what it looks at and what it never reads.
#[test]
fn us1_probe_without_disclosure_is_rejected() {
    let bare = PROBE.replace("never = \"Histórico\"\n", "");
    assert_ne!(bare, PROBE);
    let c = codes(load_from_sources(
        MANIFEST,
        &[("net.dns.resolve.toml", &bare)],
    ));
    assert_eq!(c, vec!["PROBE_MISSING_DISCLOSURE"]);
}

/// Spec 009 US2: consent categories must be declared with a versioned pt-BR notice.
#[test]
fn us2_consent_category_must_be_declared() {
    let c = codes(load_from_sources(
        MANIFEST,
        &[("startup.apps.list.toml", SENSITIVE)],
    ));
    assert_eq!(c, vec!["UNKNOWN_CONSENT"]);
    let ok = load_from_sources(CONSENT_MANIFEST, &[("startup.apps.list.toml", SENSITIVE)]).unwrap();
    assert_eq!(ok.consents["startup_apps"].version, "1");
    let no_text = CONSENT_MANIFEST.replace("text = \"X\"", "text = \" \"");
    let c = codes(load_from_sources(
        &no_text,
        &[("startup.apps.list.toml", SENSITIVE)],
    ));
    assert_eq!(c, vec!["CONSENT_MISSING_COPY"]);
}

#[test]
fn us1_real_catalog_discloses_every_probe_on_both_platforms() {
    let catalog = embedded().unwrap();
    for platform in [Platform::Linux, Platform::Windows] {
        let list = catalog.disclosures(platform);
        let probes = catalog
            .catalog
            .iter()
            .filter(|c| c.kind == CapabilityKind::Probe && c.variants.contains_key(&platform))
            .count();
        assert_eq!(list.len(), probes, "{platform:?}");
        let startup = list
            .iter()
            .find(|d| d.probe_id == "startup.apps.list")
            .unwrap();
        assert_eq!(startup.consent, Some("startup_apps"));
    }
    for id in ["startup_apps", "ai_opencode"] {
        assert!(catalog.consents.contains_key(id), "{id}");
    }
}

/// Plan v1 III.13: no jargon in the first layer (titles, explanations, steps, risk, undo,
/// consent notices). The "o que eu olho" details (`looks_at`, `never`) may name technical terms.
#[test]
fn copy_has_no_jargon_in_the_first_layer() {
    const JARGON: &[&str] = &[
        "DNS",
        "DHCP",
        "IP",
        "IPv4",
        "TCP",
        "HTTP",
        "gateway",
        "kernel",
        "driver",
        "cache",
        "proxy",
        "SSID",
        "MAC",
        "PipeWire",
        "PulseAudio",
        "ALSA",
        "UAC",
        "polkit",
        "processo",
        "serviço",
    ];
    let has = |text: &str| -> Option<&str> {
        JARGON.iter().copied().find(|j| {
            text.match_indices(j).any(|(i, _)| {
                let before = text[..i].chars().next_back();
                let after = text[i + j.len()..].chars().next();
                let edge = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric() && c != '-');
                edge(before) && edge(after)
            })
        })
    };
    let catalog = embedded().unwrap();
    let mut found = Vec::new();
    for (id, meta) in &catalog.meta {
        if let Some(c) = meta.copy.get("pt-BR") {
            for text in [Some(&c.title), c.risk.as_ref(), c.undo.as_ref()]
                .into_iter()
                .flatten()
            {
                if let Some(j) = has(text) {
                    found.push(format!("{id}: `{j}` in {text}"));
                }
            }
        }
    }
    for pb in catalog.playbooks.values() {
        for title in pb.title.values() {
            if let Some(j) = has(title) {
                found.push(format!("{}: `{j}` in {title}", pb.id));
            }
        }
        for d in pb.diagnoses.iter().chain(std::iter::once(&pb.fallback)) {
            if let Some(c) = d.copy.get("pt-BR") {
                for text in [&c.title, &c.explanation].into_iter().chain(&c.steps) {
                    if let Some(j) = has(text) {
                        found.push(format!("{}/{}: `{j}` in {text}", pb.id, d.id));
                    }
                }
            }
        }
    }
    for (id, notice) in &catalog.consents {
        if let Some(c) = notice.copy.get("pt-BR") {
            for text in [&c.title, &c.text] {
                if let Some(j) = has(text) {
                    found.push(format!("consent {id}: `{j}` in {text}"));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "jargon in the first layer:\n{}",
        found.join("\n")
    );
}

#[test]
fn only_fixes_may_be_idempotent() {
    let probe = PROBE.replace("kind = \"probe\"", "kind = \"probe\"\nidempotent = true");
    let c = codes(load_from_sources(
        MANIFEST,
        &[
            ("net.dns.resolve.toml", &probe),
            ("net.dns.flush_cache.toml", FIX),
        ],
    ));
    assert_eq!(c, vec!["INVALID_IDEMPOTENT"]);
    assert!(
        embedded().unwrap().meta["net.dns.flush_cache"].idempotent,
        "flushing the DNS cache twice is harmless"
    );
}
