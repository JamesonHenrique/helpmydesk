// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 020 T007, end to end on a real Windows: the UAC launcher starts the installed helper through
//! PowerShell `RunAs`, the request crosses the command line intact, and the helper's exit code comes
//! back as the contract says.
//!
//! It writes to `C:\Program Files`, so it only runs where it is asked to (`HMD_UAC_E2E=1`): the
//! Windows CI runner, which is an administrator account without the UAC dialog. On a real machine
//! the dialog would wait for a click; that part is the manual test (T012).
#![cfg(windows)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use hmd_elevated::uac::{HELPER_EXE, UacLauncher};
use hmd_elevated::{
    ElevatedRequest, INSTALL_DIR, LaunchError, Launcher, MAX_REQUEST_BYTES, PROTOCOL, encode_token,
    system_now,
};

fn install_helper() {
    let dir = Path::new(INSTALL_DIR);
    std::fs::create_dir_all(dir).expect("the CI runner can write to Program Files");
    std::fs::copy(
        env!("CARGO_BIN_EXE_helpmydesk-elevated"),
        dir.join(HELPER_EXE),
    )
    .expect("the helper is copied to the per-machine folder");
}

/// A request as large as the protocol allows, so the test also proves the command line of
/// `Start-Process` carries the biggest token the helper accepts.
fn largest_request() -> ElevatedRequest {
    let mut request = ElevatedRequest {
        protocol: PROTOCOL.to_string(),
        plan: String::new(),
        plan_hash: format!("sha256:{}", "0".repeat(64)),
        step_id: "s1".to_string(),
        capability: "net.dns.flush_cache".to_string(),
        capability_version: "1".to_string(),
        platform: "windows".to_string(),
        params: BTreeMap::new(),
        result_pipe: None,
        expires_at: system_now().unwrap() + 60,
    };
    let room = MAX_REQUEST_BYTES - serde_json::to_string(&request).unwrap().len();
    request.plan = "x".repeat(room);
    assert!(encode_token(&request).is_ok());
    request
}

#[test]
fn t007_uac_launcher_round_trip_with_the_largest_token() {
    if std::env::var("HMD_UAC_E2E").as_deref() != Ok("1") {
        eprintln!("skipped: set HMD_UAC_E2E=1 (writes to Program Files)");
        return;
    }
    install_helper();
    let launcher = UacLauncher::installed();
    assert!(launcher.available());
    // The hash is wrong on purpose: the helper decodes the whole token, parses the JSON and only
    // then refuses. Any truncation or quoting damage would end as BAD_REQUEST instead.
    let answer = launcher.launch(&largest_request(), Duration::from_secs(90));
    assert_eq!(answer, Err(LaunchError::Refused("PLAN_HASH_MISMATCH")));
}
