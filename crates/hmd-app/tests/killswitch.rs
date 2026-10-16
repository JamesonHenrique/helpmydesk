// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 014: the kill switch through the app (signature, monotonic, applied, kept offline).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::Path;

use ed25519_dalek::{Signer, SigningKey};
use hmd_app::{App, AppError};
use hmd_domain::ProblemFamily;
use hmd_policy::Platform;
use hmd_probes::{Finding, ProbeError, ProbeOutput, Severity};

const NOW: i64 = 1_760_400_000;

fn key() -> SigningKey {
    SigningKey::from_bytes(&[42; 32])
}

fn signed(issued_at: i64, disabled: &[&str]) -> (Vec<u8>, String) {
    let list = disabled
        .iter()
        .map(|d| format!("\"{d}\""))
        .collect::<Vec<_>>()
        .join(",");
    let file = format!(
        r#"{{"format":1,"issued_at":{issued_at},"expires_at":{},"disabled_capabilities":[{list}],"disabled_models":[]}}"#,
        issued_at + 30 * 86_400
    )
    .into_bytes();
    let sig = key()
        .sign(&file)
        .to_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    (file, sig)
}

fn dns_failing(id: &str) -> Result<ProbeOutput, ProbeError> {
    let findings = if id == "net.dns.resolve" {
        vec![Finding {
            code: "net.dns.resolution_failing",
            severity: Severity::Critical,
        }]
    } else {
        vec![]
    };
    Ok(ProbeOutput {
        probe_id: "fake",
        findings,
        evidence: BTreeMap::new(),
    })
}

fn open(path: &Path, now: i64) -> App {
    App::open(path, now)
        .unwrap()
        .0
        .with_platform(Platform::Linux)
        .with_trusted_keys(vec![key().verifying_key().to_bytes()])
        .unwrap()
}

#[test]
fn production_build_trusts_only_the_project_key() {
    let (mut app, _) = App::open_in_memory(NOW).unwrap();
    let (file, sig) = signed(NOW, &["net.dns.flush_cache"]);
    let err = app.apply_kill_switch(&file, &sig, NOW).unwrap_err();
    assert!(matches!(err, AppError::KillSwitch(_)));
    assert_eq!(err.reason_code(), "KILLSWITCH_BAD_SIGNATURE");

    // A file really signed with the project key (fixture made with docs/ops/kill-switch.md).
    let file = include_bytes!("fixtures/killswitch-empty.json");
    let sig = include_str!("fixtures/killswitch-empty.json.sig");
    let disabled = app.apply_kill_switch(file, sig, NOW).unwrap();
    assert!(disabled.is_empty());
}

#[test]
fn disabled_fix_is_not_offered_and_survives_restart_offline() {
    let dir = std::env::temp_dir().join(format!("hmd-ks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("history.sqlite3");
    {
        let mut app = open(&path, NOW);
        let t = app
            .start_ticket(ProblemFamily::Connectivity, NOW)
            .unwrap()
            .ticket;
        assert!(
            app.diagnose(&t.id, NOW, dns_failing)
                .unwrap()
                .plan
                .is_some()
        );

        let (file, sig) = signed(NOW, &["net.dns.flush_cache"]);
        let disabled = app.apply_kill_switch(&file, &sig, NOW).unwrap();
        assert!(disabled.contains("net.dns.flush_cache"));
        let view = app.diagnose(&t.id, NOW + 1, dns_failing).unwrap();
        assert!(view.plan.is_none(), "kill switch turned the fix off");

        // Replaying an older (more permissive) file is refused.
        let (old, old_sig) = signed(NOW - 100, &[]);
        assert_eq!(
            app.apply_kill_switch(&old, &old_sig, NOW + 2)
                .unwrap_err()
                .reason_code(),
            "KILLSWITCH_NOT_NEWER"
        );
        assert!(app.audit_intact().unwrap());
    }
    // Offline after a restart: the last accepted file still applies.
    let app = open(&path, NOW + 10);
    assert!(app.disabled_capabilities().contains("net.dns.flush_cache"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Spec 020: the only fix in the catalog is elevated, so it runs through a fake elevation path.
struct CountingElevation(std::sync::Arc<std::sync::atomic::AtomicUsize>);

impl hmd_executor::Elevation for CountingElevation {
    fn available(&self, _: &hmd_policy::ProposedStep, _: hmd_policy::Platform) -> bool {
        true
    }
    fn run(
        &self,
        _: &hmd_policy::ProposedStep,
        _: &hmd_executor::EvaluatedPlan<'_>,
        _: hmd_policy::Platform,
    ) -> Result<hmd_executor::FixOutcome, hmd_executor::ExecError> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(hmd_executor::FixOutcome::Succeeded)
    }
}

/// Spec 016: a kill switch that arrives between approval and execution wins.
#[test]
fn kill_switch_after_approval_blocks_the_approved_fix() {
    let runs = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (app, _) = App::open_in_memory(NOW).unwrap();
    let mut app = app
        .with_platform(Platform::Linux)
        .with_trusted_keys(vec![key().verifying_key().to_bytes()])
        .unwrap()
        .with_registry(
            hmd_executor::Registry::default()
                .with_elevation(Box::new(CountingElevation(runs.clone()))),
        );
    let t = app
        .start_ticket(ProblemFamily::Connectivity, NOW)
        .unwrap()
        .ticket;
    let plan = app.diagnose(&t.id, NOW, dns_failing).unwrap().plan.unwrap();
    app.approve(&t.id, plan.plan_id, &["s1"], NOW).unwrap();
    let (file, sig) = signed(NOW, &["net.dns.flush_cache"]);
    app.apply_kill_switch(&file, &sig, NOW).unwrap();
    assert!(app.execute(&t.id, plan.plan_id, NOW, dns_failing).is_err());
    assert_eq!(runs.load(std::sync::atomic::Ordering::SeqCst), 0);
}

/// Independent review, M4: "Apagar tudo" keeps the kill switch (with its anti-rollback) and the
/// blocked fixes; everything else goes.
#[test]
fn erase_all_keeps_the_safety_state() {
    let dir = std::env::temp_dir().join(format!("hmd-ks-erase-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("history.sqlite3");
    let mut app = open(&path, NOW);
    let (file, sig) = signed(NOW, &["net.dns.flush_cache"]);
    app.apply_kill_switch(&file, &sig, NOW).unwrap();
    let mut state = app.safety_state().unwrap();
    state.blocked.insert("x.blocked.fix".into());
    app.start_ticket(ProblemFamily::Connectivity, NOW).unwrap();
    app.erase_all(&path).unwrap();

    let mut fresh = open(&path, NOW + 1);
    assert!(fresh.disabled_capabilities().is_empty(), "fresh history");
    fresh.restore_safety_state(state, NOW + 1).unwrap();
    assert!(
        fresh
            .disabled_capabilities()
            .contains("net.dns.flush_cache")
    );
    assert!(
        fresh
            .blocked_capabilities()
            .unwrap()
            .contains("x.blocked.fix")
    );
    let (old, old_sig) = signed(NOW - 100, &[]);
    assert_eq!(
        fresh
            .apply_kill_switch(&old, &old_sig, NOW + 2)
            .unwrap_err()
            .reason_code(),
        "KILLSWITCH_NOT_NEWER",
        "anti-rollback survives the erase"
    );
    assert!(fresh.history().unwrap().is_empty(), "tickets are gone");
    // Survives a restart too (it lives in the new history).
    drop(fresh);
    let again = open(&path, NOW + 3);
    assert!(
        again
            .disabled_capabilities()
            .contains("net.dns.flush_cache")
    );
    assert!(
        again
            .blocked_capabilities()
            .unwrap()
            .contains("x.blocked.fix")
    );
    let _ = std::fs::remove_dir_all(&dir);
}
