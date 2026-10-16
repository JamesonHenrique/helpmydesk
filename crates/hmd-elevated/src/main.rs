// SPDX-License-Identifier: GPL-3.0-or-later
//! `helpmydesk-elevated`: the privileged side of spec 020. One action per invocation, then out.
//!
//! The whole request arrives as a single base64url token in the command line (SEC-022): nothing is
//! read from a file, from an environment variable or from stdin, so malware running as the person
//! cannot swap the request between the approval and the click in the UAC dialog. Everything the
//! helper needs to decide is either compiled into this build or recalculated from what it received.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::io::Write as _;
use std::process::ExitCode;

use hmd_catalog::embedded;
use hmd_elevated::{
    Allowlist, ElevatedError, HelperContext, INSTALL_DIR, host_platform, result_json, run_request,
    system_now, trusted_path,
};

fn main() -> ExitCode {
    let allowlist = Allowlist::production();
    let Some(catalog) = embedded().ok() else {
        return refuse(&ElevatedError::BadRequest);
    };
    let exe = std::env::current_exe().unwrap_or_default();
    let install_dir = std::path::Path::new(INSTALL_DIR);
    // SEC-020: where this process is running is checked before anything it was asked to do, before
    // reading even the command line. A helper in a folder the person can write to answers
    // `HELPER_PATH_NOT_TRUSTED` whatever the token says.
    if !trusted_path(&exe, install_dir) {
        return refuse(&ElevatedError::HelperPathNotTrusted);
    }
    let context = HelperContext {
        exe: &exe,
        install_dir,
        host: host_platform(),
        catalog,
        allowlist: &allowlist,
    };
    let Some(token) = std::env::args().nth(1) else {
        return refuse(&ElevatedError::BadRequest);
    };
    let Some(now) = system_now() else {
        return refuse(&ElevatedError::BadRequest);
    };
    let (code, result) = run_request(&token, &context, now);
    // SEC-025: JSON on stdout plus the exit code; the user-level process is the one that writes to
    // the journal. Nothing else is printed: no probe output, no path, no name.
    let line = format!("{}\n", result_json(&result));
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(line.as_bytes());
    let _ = stdout.flush();
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}

fn refuse(error: &ElevatedError) -> ExitCode {
    let line = format!("{}\n", result_json(&error.result()));
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(line.as_bytes());
    let _ = stdout.flush();
    ExitCode::from(u8::try_from(error.exit_code()).unwrap_or(1))
}
