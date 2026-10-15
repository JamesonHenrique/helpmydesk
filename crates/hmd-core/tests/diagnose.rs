// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 006 acceptance tests: deterministic playbook evaluation with fake probes, and real
//! diagnoses for every family on the CI runners.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};

use hmd_catalog::embedded;
use hmd_core::{DiagnoseOptions, ProbeOutcome, diagnose, diagnose_with};
use hmd_domain::ProblemFamily;
use hmd_policy::{ApprovalRequirement, Confidence, RiskLevel, StepVerdict};
use hmd_probes::{Finding, ProbeError, ProbeOutput, Severity};

/// Fake runner: `findings` maps probe ID → finding codes; `failing` probes return TIMEOUT.
fn fake<'a>(
    findings: &'a [(&'a str, &'a [&'static str])],
    failing: &'a [&'a str],
) -> impl Fn(&str) -> Result<ProbeOutput, ProbeError> + 'a {
    move |id: &str| {
        if failing.contains(&id) {
            return Err(ProbeError::Timeout);
        }
        let codes = findings
            .iter()
            .find(|(p, _)| *p == id)
            .map(|(_, c)| *c)
            .unwrap_or(&[]);
        Ok(ProbeOutput {
            probe_id: "fake",
            findings: codes
                .iter()
                .map(|code| Finding {
                    code,
                    severity: Severity::Warning,
                })
                .collect(),
            evidence: BTreeMap::new(),
        })
    }
}

fn run(
    family: ProblemFamily,
    findings: &[(&str, &[&'static str])],
    failing: &[&str],
    opts: &DiagnoseOptions,
) -> hmd_core::DiagnosisReport {
    diagnose_with(embedded().unwrap(), family, opts, fake(findings, failing)).unwrap()
}

#[test]
fn us1_dns_diagnosis_proposes_flush_with_per_action_approval() {
    let r = run(
        ProblemFamily::Connectivity,
        &[("net.dns.resolve", &["net.dns.resolution_failing"])],
        &[],
        &DiagnoseOptions::default(),
    );
    assert_eq!(r.diagnosis_id, "net.dns");
    let decision = r.fix_decision.unwrap();
    match &decision.verdicts[0] {
        StepVerdict::Accepted {
            capability,
            approval,
            risk_level,
            ..
        } => {
            assert_eq!(capability, "net.dns.flush_cache");
            assert_eq!(*approval, ApprovalRequirement::PerAction);
            assert_eq!(*risk_level, RiskLevel::N3);
        }
        other => panic!("expected accepted, got {other:?}"),
    }
}

#[test]
fn us1_gateway_down_is_guidance_only() {
    let r = run(
        ProblemFamily::Connectivity,
        &[("net.gateway.reach", &["net.gateway.unreachable"])],
        &[],
        &DiagnoseOptions::default(),
    );
    assert_eq!(r.diagnosis_id, "net.gateway_down");
    assert!(r.fix_decision.is_none());
    assert!(!r.copy.steps.is_empty());
}

#[test]
fn us1_internet_down_wins_over_dns_failure() {
    // When the internet is down DNS also fails; flushing DNS would not help.
    let r = run(
        ProblemFamily::Connectivity,
        &[
            ("net.dns.resolve", &["net.dns.resolution_failing"]),
            ("net.http.check", &["net.internet.unreachable"]),
        ],
        &[],
        &DiagnoseOptions::default(),
    );
    assert_eq!(r.diagnosis_id, "net.internet_down");
    assert!(r.fix_decision.is_none());
}

#[test]
fn us1_nothing_found_uses_fallback_with_report() {
    let r = run(
        ProblemFamily::Connectivity,
        &[],
        &[],
        &DiagnoseOptions::default(),
    );
    assert!(r.is_fallback);
    assert_eq!(r.diagnosis_id, "net.ok");
    assert_eq!(r.report.map(|t| t.code()), Some("REP-01"));
}

#[test]
fn us1_family_without_probes_is_guidance_plus_report() {
    {
        let family = ProblemFamily::AppError;
        let r = run(family, &[], &[], &DiagnoseOptions::default());
        assert!(r.probes.is_empty(), "{family:?} must not fake checks");
        assert!(r.is_fallback);
        assert_eq!(r.report.map(|t| t.code()), Some("REP-01"));
        assert!(!r.copy.steps.is_empty());
    }
}

#[test]
fn us2_failed_probe_lowers_confidence_and_blocks_n3() {
    let r = run(
        ProblemFamily::Connectivity,
        &[("net.dns.resolve", &["net.dns.resolution_failing"])],
        &["net.http.check"],
        &DiagnoseOptions::default(),
    );
    assert_eq!(r.diagnosis_id, "net.dns");
    assert_eq!(r.confidence, Confidence::Unsure);
    assert!(
        r.probes
            .iter()
            .any(|p| p.outcome == ProbeOutcome::Failed("TIMEOUT"))
    );
    match &r.fix_decision.unwrap().verdicts[0] {
        StepVerdict::Rejected { reason, .. } => assert_eq!(reason.reason_code(), "LOW_CONFIDENCE"),
        other => panic!("expected LOW_CONFIDENCE, got {other:?}"),
    }
}

#[test]
fn us3_sensitive_probe_needs_consent() {
    let without = run(
        ProblemFamily::Performance,
        &[],
        &[],
        &DiagnoseOptions::default(),
    );
    let startup = without
        .probes
        .iter()
        .find(|p| p.probe_id == "startup.apps.list")
        .unwrap();
    assert_eq!(
        startup.outcome,
        ProbeOutcome::NotAuthorized("CONSENT_REQUIRED")
    );
    assert!(
        without
            .probes
            .iter()
            .filter(|p| p.probe_id != "startup.apps.list")
            .all(|p| matches!(p.outcome, ProbeOutcome::Ran(_)))
    );

    let opts = DiagnoseOptions {
        consents: BTreeSet::from(["startup_apps".to_string()]),
        ..DiagnoseOptions::default()
    };
    let with = run(
        ProblemFamily::Performance,
        &[("startup.apps.list", &["startup.many_apps"])],
        &[],
        &opts,
    );
    assert_eq!(with.diagnosis_id, "perf.startup");
}

/// SC-002: real diagnoses for every family run on Linux and Windows without elevation.
#[test]
fn sc002_real_diagnosis_for_every_family() {
    for family in ProblemFamily::ALL {
        let r = diagnose(family, &DiagnoseOptions::default())
            .unwrap_or_else(|e| panic!("{family:?}: {e}"));
        assert!(!r.copy.title.is_empty(), "{family:?}");
    }
}

/// Spec 010: sound, microphone and camera diagnoses, most certain first.
#[test]
fn spec010_audio_video_diagnoses() {
    let opts = DiagnoseOptions {
        platform: hmd_policy::Platform::Windows,
        ..DiagnoseOptions::default()
    };
    let r = run(
        ProblemFamily::AudioVideo,
        &[
            ("av.privacy.mic_camera", &["av.mic.blocked_by_privacy"]),
            ("av.audio.devices", &["av.no_output_device"]),
        ],
        &[],
        &opts,
    );
    assert_eq!(r.diagnosis_id, "av.mic_blocked");
    assert_eq!(r.confidence, Confidence::AlmostCertain);
    assert!(r.fix_decision.is_none(), "guidance only");

    let linux = DiagnoseOptions {
        platform: hmd_policy::Platform::Linux,
        ..DiagnoseOptions::default()
    };
    let r = run(
        ProblemFamily::AudioVideo,
        &[
            ("av.audio.devices", &["av.no_output_device"]),
            ("av.camera.present", &["av.no_camera"]),
        ],
        &[],
        &linux,
    );
    assert_eq!(r.diagnosis_id, "av.no_output", "sound before camera");
    assert!(
        r.probes
            .iter()
            .any(|p| p.probe_id == "av.privacy.mic_camera"
                && p.outcome == ProbeOutcome::NotAuthorized("PLATFORM_UNSUPPORTED")),
        "Windows-only probe is skipped on Linux, not faked"
    );

    let r = run(ProblemFamily::AudioVideo, &[], &[], &linux);
    assert!(r.is_fallback);
    assert_eq!(r.report.map(|t| t.code()), Some("REP-01"));
}
