// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 004 acceptance tests. Real probes run here on the CI runners (Linux and Windows) as a
//! normal user (SC-001). The DNS probe against real names is `#[ignore]` (needs network).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use hmd_catalog::embedded;
use hmd_policy::{CapabilityKind, Platform};
use hmd_probes::{Evidence, PROBES, ProbeError, run};

fn platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else {
        Platform::Linux
    }
}

/// FR-002 / SC-002: every implemented probe is a catalog probe for this platform and every
/// finding it can emit is declared in `produces`.
#[test]
fn fr002_probes_match_catalog() {
    let cat = embedded().unwrap();
    for (id, emits) in PROBES {
        let cap = cat
            .catalog
            .get(id)
            .unwrap_or_else(|| panic!("{id} not in catalog"));
        assert_eq!(cap.kind, CapabilityKind::Probe, "{id}");
        if !cap.variants.contains_key(&platform()) {
            // Platform-specific probes (spec 010) have no data elsewhere, never fake results.
            assert_eq!(run(id).unwrap_err(), ProbeError::NoData, "{id}");
            continue;
        }
        let produces = &cat.meta[*id].produces;
        for f in *emits {
            assert!(produces.iter().any(|p| p == f), "{id} emits undeclared {f}");
        }
    }
}

#[test]
fn fr002_unknown_probe_is_rejected() {
    assert_eq!(run("shell.exec").unwrap_err(), ProbeError::UnknownProbe);
}

#[test]
fn us1_basic_info_runs_without_privileges() {
    let out = run("sys.info.basic").unwrap();
    assert!(matches!(out.evidence["uptime_seconds"], Evidence::Uint(_)));
    assert!(out.evidence.contains_key("arch"));
}

#[test]
fn us1_memory_snapshot_runs() {
    let out = run("perf.memory.snapshot").unwrap();
    match (
        &out.evidence["total_bytes"],
        &out.evidence["available_bytes"],
    ) {
        (Evidence::Uint(t), Evidence::Uint(a)) => assert!(*t > 0 && a <= t),
        other => panic!("unexpected evidence {other:?}"),
    }
}

#[test]
fn us1_system_volume_runs() {
    let out = run("storage.volumes").unwrap();
    match (&out.evidence["total_bytes"], &out.evidence["free_bytes"]) {
        (Evidence::Uint(t), Evidence::Uint(f)) => assert!(*t > 0 && f <= t),
        other => panic!("unexpected evidence {other:?}"),
    }
}

/// FR-005 / SC-003: no identifying keys, and the host name never appears in any value.
#[test]
fn fr005_no_identifying_data() {
    const DENY: &[&str] = &["host", "user", "serial", "mac", "ssid", "label", "name"];
    let host = sysinfo::System::host_name()
        .unwrap_or_default()
        .to_lowercase();
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default()
        .to_lowercase();
    for id in [
        "sys.info.basic",
        "perf.memory.snapshot",
        "storage.volumes",
        "net.adapters.status",
        "perf.cpu.snapshot",
        "perf.top_processes",
        "startup.apps.list",
        "av.audio.devices",
        "av.camera.present",
        "av.privacy.mic_camera",
    ] {
        let out = match run(id) {
            Ok(out) => out,
            // Sound probes have no data on machines without audio (CI runners).
            Err(ProbeError::NoData) if id.starts_with("av.") => continue,
            Err(e) => panic!("{id}: {e}"),
        };
        for (key, value) in &out.evidence {
            // `os_name` is the OS product name (e.g. "Windows", "Debian GNU/Linux"), allowed.
            if *key != "os_name" {
                assert!(!DENY.iter().any(|d| key.contains(d)), "{id}: key {key}");
            }
            if let Evidence::Text(t) = value {
                let t = t.to_lowercase();
                for secret in [&host, &user] {
                    if secret.len() >= 3 {
                        assert!(!t.contains(secret.as_str()), "{id}: {key} leaks identity");
                    }
                }
            }
        }
    }
}

#[test]
fn batch2_local_probes_run_without_privileges() {
    for id in [
        "net.adapters.status",
        "perf.cpu.snapshot",
        "perf.top_processes",
        "startup.apps.list",
    ] {
        let out = run(id).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(out.probe_id, id);
    }
}

/// Network probes must always finish and return structured output (or TIMEOUT), even on CI
/// runners whose gateway ignores TCP; findings are not asserted here.
#[test]
fn batch2_network_probes_terminate() {
    for id in ["net.gateway.reach", "net.http.check"] {
        match run(id) {
            Ok(out) => assert_eq!(out.probe_id, id),
            Err(e) => assert_eq!(e, ProbeError::Timeout, "{id}"),
        }
    }
}

#[test]
#[ignore = "needs network; run with --ignored"]
fn us2_real_dns_resolves() {
    let out = run("net.dns.resolve").unwrap();
    assert!(out.findings.is_empty(), "{out:?}");
}
