// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 007 US2: the "não consegui" report (`report@1`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use hmd_catalog::embedded;
use hmd_core::report::{
    ReportInput, SystemSummary, redact_free_text, render, system_summary_with, utc_date,
};
use hmd_core::{DiagnoseOptions, DiagnosisReport, diagnose, diagnose_with};
use hmd_domain::{ProblemFamily, ReportTrigger};
use hmd_probes::{Evidence, Finding, ProbeError, ProbeOutput, Severity};

const NOW: i64 = 1_760_400_000; // 2025-10-14

/// Fake runner whose evidence carries text that must never reach the report.
fn leaky(id: &str) -> Result<ProbeOutput, ProbeError> {
    let mut evidence = BTreeMap::new();
    evidence.insert("top", Evidence::Text("secret-process.exe".to_string()));
    evidence.insert("os_name", Evidence::Text("TestOS".to_string()));
    let findings = match id {
        "startup.apps.list" => vec![Finding {
            code: "startup.many_apps",
            severity: Severity::Warning,
        }],
        _ => vec![],
    };
    Ok(ProbeOutput {
        probe_id: "fake",
        findings,
        evidence,
    })
}

fn consented() -> DiagnoseOptions {
    let mut opts = DiagnoseOptions::default();
    opts.consents.insert("startup_apps".to_string());
    opts
}

fn report_for(d: &DiagnosisReport, trigger: ReportTrigger, symptom: Option<&str>) -> String {
    render(&ReportInput {
        catalog: embedded().unwrap(),
        diagnosis: d,
        trigger,
        symptom,
        system: Some(&SystemSummary {
            os_name: Some("TestOS".into()),
            os_version: Some("1".into()),
            kernel_version: None,
            arch: Some("x86_64".into()),
        }),
        created_at: NOW,
    })
}

#[test]
fn us2_1_header_and_fixed_sections_in_order() {
    let d = diagnose_with(
        embedded().unwrap(),
        ProblemFamily::Storage,
        &consented(),
        leaky,
    )
    .unwrap();
    let text = report_for(&d, ReportTrigger::NoApplicableFix, None);
    assert!(text.starts_with("<!-- report@1 -->\n"));
    let mut last = 0;
    for section in [
        "## Sintoma",
        "## O que verifiquei",
        "## Diagnóstico",
        "## O que já foi tentado",
        "## Sistema",
        "## Para quem for ajudar",
    ] {
        let at = text
            .find(section)
            .unwrap_or_else(|| panic!("missing {section}"));
        assert!(at > last, "{section} out of order");
        last = at;
    }
    assert!(text.contains("2025-10-14"));
    assert!(text.contains("(REP-01)"));
    assert!(text.contains("Espaço no disco"), "probe title in pt-BR");
}

#[test]
fn us2_2_says_nothing_was_changed() {
    let d = diagnose_with(
        embedded().unwrap(),
        ProblemFamily::Connectivity,
        &consented(),
        leaky,
    )
    .unwrap();
    let text = report_for(&d, ReportTrigger::NoApplicableFix, None);
    assert!(text.contains("O HelpMyDesk não alterou nada neste computador"));
}

#[test]
fn us2_person_requested_works_without_diagnosis_trigger() {
    let d = diagnose_with(
        embedded().unwrap(),
        ProblemFamily::Storage,
        &consented(),
        leaky,
    )
    .unwrap();
    assert_eq!(d.report, None, "storage ok has no trigger of its own");
    let text = report_for(&d, d.report.unwrap_or(ReportTrigger::PersonRequested), None);
    assert!(text.contains("(REP-06)"));
    assert!(text.contains("(não informado)"));
}

#[test]
fn us2_4_no_probe_evidence_text_and_no_app_names() {
    let d = diagnose_with(
        embedded().unwrap(),
        ProblemFamily::Performance,
        &consented(),
        leaky,
    )
    .unwrap();
    let text = report_for(&d, ReportTrigger::LowConfidence, None);
    assert!(!text.contains("secret-process"), "{text}");
    assert!(
        text.contains("`startup.many_apps`"),
        "finding codes are kept"
    );
}

#[test]
fn us2_3_symptom_is_quoted_and_sanitized() {
    let symptom = redact_free_text("linha 1\u{202E}\nlinha 2\u{0007}");
    assert_eq!(symptom, "linha 1\nlinha 2");
    let d = diagnose_with(
        embedded().unwrap(),
        ProblemFamily::AppError,
        &consented(),
        leaky,
    )
    .unwrap();
    let text = report_for(&d, ReportTrigger::NoApplicableFix, Some(&symptom));
    assert!(text.contains("> linha 1\n> linha 2\n"));
    let long = redact_free_text(&"a".repeat(10_000));
    assert_eq!(long.chars().count(), hmd_core::report::MAX_FREE_TEXT);
}

#[test]
fn system_summary_comes_from_authorized_probe() {
    let sys = system_summary_with(embedded().unwrap(), &DiagnoseOptions::default(), leaky).unwrap();
    assert_eq!(sys.os_name.as_deref(), Some("TestOS"));
}

#[test]
fn utc_dates() {
    assert_eq!(utc_date(0), "1970-01-01");
    assert_eq!(utc_date(951_782_400), "2000-02-29");
    assert_eq!(utc_date(-86_400), "1969-12-31");
    assert_eq!(utc_date(NOW), "2025-10-14");
}

fn is_whole_word(text: &str, name: &str) -> bool {
    text.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .any(|w| w == name)
}

/// SC-002: real reports on the CI runners never contain the user or host name.
#[test]
fn sc002_real_report_has_no_identity() {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default()
        .to_lowercase();
    let host = sysinfo::System::host_name()
        .unwrap_or_default()
        .to_lowercase();
    let symptom = redact_free_text(&format!("o computador {host} do {user} travou"));
    let catalog = embedded().unwrap();
    let opts = DiagnoseOptions::default();
    let system = system_summary_with(catalog, &opts, hmd_probes::run);
    assert!(
        system.is_some(),
        "sys.info.basic is N1-A and needs no consent"
    );
    for family in ProblemFamily::ALL {
        let d = diagnose(family, &opts).unwrap();
        let text = render(&ReportInput {
            catalog,
            diagnosis: &d,
            trigger: d.report.unwrap_or(ReportTrigger::PersonRequested),
            symptom: Some(&symptom),
            system: system.as_ref(),
            created_at: NOW,
        })
        .to_lowercase();
        for name in [&user, &host] {
            if name.is_empty() {
                continue;
            }
            let leaked = if name.chars().count() >= 3 {
                text.contains(name.as_str())
            } else {
                is_whole_word(&text, name)
            };
            assert!(!leaked, "{family:?}: report leaks identity");
        }
    }
}
