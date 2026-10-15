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
fn production_build_trusts_no_key_yet() {
    let (mut app, _) = App::open_in_memory(NOW).unwrap();
    let (file, sig) = signed(NOW, &["net.dns.flush_cache"]);
    let err = app.apply_kill_switch(&file, &sig, NOW).unwrap_err();
    assert!(matches!(err, AppError::KillSwitch(_)));
    assert_eq!(err.reason_code(), "KILLSWITCH_NO_TRUSTED_KEY");
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
