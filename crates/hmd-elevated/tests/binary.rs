// SPDX-License-Identifier: GPL-3.0-or-later
//! The helper as a process: the contract the user-level side depends on (spec 020 SEC-022, SEC-025)
//! — the request arrives in the command line, the answer is one JSON line on stdout and the exit
//! code says what happened. Nothing here needs administrator rights.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

fn helper() -> Command {
    Command::new(env!("CARGO_BIN_EXE_helpmydesk-elevated"))
}

fn run(args: &[&str]) -> (i32, String) {
    let output = helper()
        .args(args)
        .output()
        .expect("the helper binary runs");
    (
        output.status.code().expect("the helper exits with a code"),
        String::from_utf8_lossy(&output.stdout).to_string(),
    )
}

/// SEC-025: no request at all is a refusal with a stable code, not a crash.
#[test]
fn missing_request_is_refused_with_a_stable_code() {
    let (code, stdout) = run(&[]);
    assert_eq!(code, 11, "BAD_REQUEST exit code (spec 020 plan)");
    assert!(
        stdout.contains(r#""reason_code":"BAD_REQUEST""#),
        "{stdout}"
    );
    assert!(stdout.ends_with('\n'), "one JSON line: {stdout}");
}

/// SEC-020 comes before SEC-022 on purpose: a helper that is not in the per-machine install folder
/// refuses without even reading the request, so nothing outside the folder can be talked into
/// running.
#[test]
fn forged_token_is_refused_before_it_is_read() {
    let (code, stdout) = run(&["nao-e-um-token"]);
    assert_eq!(code, 16, "HELPER_PATH_NOT_TRUSTED");
    assert!(
        stdout.contains(r#""reason_code":"HELPER_PATH_NOT_TRUSTED""#),
        "{stdout}"
    );
}

/// SEC-020: even with a well-formed request, a helper that is not installed per machine refuses to
/// do anything. On a developer machine (and on CI) the running binary is never in the install
/// folder, so this is the path every test of the binary can rely on.
#[test]
fn helper_outside_the_install_folder_refuses_before_anything_else() {
    let token = hmd_elevated::base64url_encode(
        br#"{"protocol":"hmd-elevated@1","plan":"hmd-plan@1\n","plan_hash":"sha256:x","step_id":"s1","capability":"net.dns.flush_cache","capability_version":"1.0.0","platform":"linux","params":{}}"#,
    );
    let (code, stdout) = run(&[&token]);
    assert_eq!(code, 16, "HELPER_PATH_NOT_TRUSTED exit code");
    assert!(
        stdout.contains(r#""reason_code":"HELPER_PATH_NOT_TRUSTED""#),
        "{stdout}"
    );
}

/// FR-014: nothing but the JSON goes to stdout. No probe output, no path, no name of the person.
#[test]
fn stdout_carries_only_the_result() {
    let (code, stdout) = run(&["nao-e-um-token"]);
    assert_eq!(code, 16);
    assert_eq!(stdout.trim().lines().count(), 1, "{stdout}");
    let json: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    assert_eq!(json["protocol"], "hmd-elevated@1");
    assert!(json["detail"].is_null(), "no free text: {stdout}");
}
