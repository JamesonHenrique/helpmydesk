// SPDX-License-Identifier: GPL-3.0-or-later
//! The Windows UAC launcher (spec 020 T007, ADR-031 "Lançador").
//!
//! The workspace has no `unsafe` and no dependency for `ShellExecuteExW`, so the launcher asks
//! Windows PowerShell for it: `Start-Process -Verb RunAs` is the same "run as administrator" call,
//! behind a managed API. The price is the result channel. A process started through the UAC cannot
//! hand its stdout back to a non-elevated parent, so only the helper's exit code comes back. That
//! code is already the stable part of the contract (SEC-025); the JSON line stays for tests and for
//! a future pipe.
//!
//! What keeps this from being "run a command" (constitution, forbidden actions):
//! * the script is fixed text with exactly two holes, the helper path (a constant of the build) and
//!   the token (base64url, no quote, space or backslash); a hole with any other byte is refused
//!   before PowerShell starts;
//! * the script travels as `-EncodedCommand` (UTF-16LE in base64), so no quoting rule of the command
//!   line applies to it;
//! * PowerShell is taken from the Windows folder by absolute path, with no profile, so nothing the
//!   person installed in `PATH` or in a profile runs in its place.

use std::path::{Path, PathBuf};
use std::time::Duration;

use hmd_policy::Platform;

use crate::{
    ElevatedError, ElevatedRequest, ElevatedResult, INSTALL_DIR, LaunchError, Launcher, Outcome,
    PROTOCOL, base64url_encode, encode_token,
};

/// File name of the helper inside [`INSTALL_DIR`].
pub const HELPER_EXE: &str = "helpmydesk-elevated.exe";

/// What Windows reports when the person refuses or closes the UAC dialog (`ERROR_CANCELLED`).
pub const SCRIPT_DENIED: i32 = 1223;
/// The script could not start the helper for any other reason (file missing, blocked by policy).
/// Outside the helper's range (0, 10–21), so it cannot be mistaken for a helper answer.
pub const SCRIPT_START_FAILED: i32 = 201;
/// The helper ran but Windows gave no exit code. Never read as success.
pub const SCRIPT_NO_EXIT_CODE: i32 = 202;

/// How often the launcher looks at the PowerShell process while it waits.
const POLL: Duration = Duration::from_millis(100);

/// The launch script, or `None` when a hole carries a byte that could leave its single-quoted
/// string. `$p.ExitCode` is checked for `$null` because `exit $null` would end the script with 0,
/// which is the code of a fix that worked.
pub fn launch_script(helper: &str, token: &str) -> Option<String> {
    let path_ok = !helper.is_empty()
        && helper
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b" :\\._-()".contains(&b));
    let token_ok = !token.is_empty()
        && token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'=');
    if !path_ok || !token_ok {
        return None;
    }
    Some(format!(
        "$ErrorActionPreference = 'Stop'\n\
         try {{\n\
         \x20 $p = Start-Process -FilePath '{helper}' -ArgumentList '{token}' -Verb RunAs -WindowStyle Hidden -Wait -PassThru\n\
         \x20 if ($null -eq $p.ExitCode) {{ exit {SCRIPT_NO_EXIT_CODE} }}\n\
         \x20 exit $p.ExitCode\n\
         }} catch {{\n\
         \x20 $e = $_.Exception\n\
         \x20 while ($e -and -not ($e -is [System.ComponentModel.Win32Exception])) {{ $e = $e.InnerException }}\n\
         \x20 if ($e -and $e.NativeErrorCode -eq {SCRIPT_DENIED}) {{ exit {SCRIPT_DENIED} }}\n\
         \x20 exit {SCRIPT_START_FAILED}\n\
         }}\n"
    ))
}

/// `-EncodedCommand` takes the script as UTF-16LE in standard base64.
pub fn encode_command(script: &str) -> String {
    let utf16 = script
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<u8>>();
    base64url_encode(&utf16)
        .chars()
        .map(|c| match c {
            '-' => '+',
            '_' => '/',
            other => other,
        })
        .collect()
}

/// Reads the exit code of the launch script. Anything the contract does not name ends as
/// `unknown`: the helper may have run, and an `unknown` step is never run again (FR-010).
pub fn interpret_exit(code: Option<i32>) -> Result<ElevatedResult, LaunchError> {
    let answer = |outcome: Outcome, reason: Option<&str>| ElevatedResult {
        protocol: PROTOCOL.to_string(),
        outcome,
        reason_code: reason.map(str::to_string),
        detail: None,
    };
    match code {
        Some(0) => Ok(answer(Outcome::Succeeded, None)),
        Some(code) if code == ElevatedError::Failed.exit_code() => Ok(answer(
            Outcome::Failed,
            Some(ElevatedError::Failed.reason_code()),
        )),
        Some(SCRIPT_DENIED) => Err(LaunchError::Denied),
        Some(SCRIPT_START_FAILED) => Err(LaunchError::NotAvailable),
        Some(code) => match ElevatedError::from_exit_code(code) {
            Some(reason) => Err(LaunchError::Refused(reason)),
            None => Ok(answer(
                Outcome::Unknown,
                Some(ElevatedError::Unknown.reason_code()),
            )),
        },
        None => Ok(answer(
            Outcome::Unknown,
            Some(ElevatedError::Unknown.reason_code()),
        )),
    }
}

/// Windows PowerShell 5.1, which ships with every Windows 10 and 11, from the Windows folder.
fn powershell() -> PathBuf {
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    Path::new(&root).join(r"System32\WindowsPowerShell\v1.0\powershell.exe")
}

/// The launcher of the Windows build: the helper installed per machine, started through the UAC.
pub struct UacLauncher {
    helper: PathBuf,
}

impl UacLauncher {
    pub fn installed() -> UacLauncher {
        UacLauncher {
            helper: Path::new(INSTALL_DIR).join(HELPER_EXE),
        }
    }
}

impl Launcher for UacLauncher {
    /// Only on Windows, and only when the per-machine installer put the helper in place: a portable
    /// or development copy has no trusted helper, and says so before anything is written.
    fn available(&self) -> bool {
        cfg!(windows) && self.helper.is_file()
    }

    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn launch(
        &self,
        request: &ElevatedRequest,
        timeout: Duration,
    ) -> Result<ElevatedResult, LaunchError> {
        if !self.available() {
            return Err(LaunchError::NotAvailable);
        }
        let token = encode_token(request).map_err(|_| LaunchError::Malformed)?;
        let helper = self.helper.to_str().ok_or(LaunchError::Malformed)?;
        let script = launch_script(helper, &token).ok_or(LaunchError::Malformed)?;
        let mut command = std::process::Command::new(powershell());
        command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-EncodedCommand",
                &encode_command(&script),
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt as _;
            // CREATE_NO_WINDOW: the UAC dialog is the only thing the person sees.
            command.creation_flags(0x0800_0000);
        }
        let mut child = command.spawn().map_err(|_| LaunchError::NotAvailable)?;
        let started = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return interpret_exit(status.code()),
                Ok(None) if started.elapsed() < timeout => std::thread::sleep(POLL),
                // FR-010: past the deadline the step is `unknown` and never run again. Killing
                // PowerShell stops the wait; an elevated helper that already started cannot be
                // stopped from here, which is why the outcome is not `failed`.
                Ok(None) | Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(LaunchError::Timeout);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELPER: &str = r"C:\Program Files\HelpMyDesk\helpmydesk-elevated.exe";

    #[test]
    fn sec_022_script_holes_only_take_path_and_base64url() {
        assert!(launch_script(HELPER, "eyJhIjoxfQ==").is_some());
        assert!(launch_script(HELPER, "a-b_c").is_some());
        for token in [
            "", "a'b", "a b", "a;b", "a`b", "a$b", "a\"b", "a\nb", "a\\b", "á",
        ] {
            assert!(launch_script(HELPER, token).is_none(), "{token:?}");
        }
        for helper in [
            "",
            r"C:\x'y.exe",
            r"C:\x$y.exe",
            "C:\\x\n.exe",
            r"C:\x`.exe",
        ] {
            assert!(launch_script(helper, "abc").is_none(), "{helper:?}");
        }
    }

    #[test]
    fn sec_022_script_names_the_helper_and_the_token_once() {
        let script = launch_script(HELPER, "dG9rZW4").unwrap_or_default();
        assert_eq!(script.matches(HELPER).count(), 1);
        assert_eq!(script.matches("'dG9rZW4'").count(), 1);
        assert!(script.contains("-Verb RunAs"));
        assert!(script.contains("-Wait -PassThru"));
    }

    #[test]
    fn encoded_command_is_utf16le_standard_base64() {
        assert_eq!(encode_command("a"), "YQA=");
        assert_eq!(encode_command("exit 0"), "ZQB4AGkAdAAgADAA");
        // Bytes that land on 62 and 63 use the standard alphabet, not base64url.
        assert_eq!(encode_command("\u{fbff}"), "//s=");
        assert_eq!(encode_command("\u{fb3e}"), "Pvs=");
    }

    #[test]
    fn sec_025_exit_codes_map_to_the_contract() {
        let ok = interpret_exit(Some(0)).unwrap();
        assert_eq!(ok.outcome, Outcome::Succeeded);
        let failed = interpret_exit(Some(20)).unwrap();
        assert_eq!(failed.outcome, Outcome::Failed);
        assert_eq!(failed.reason_code.as_deref(), Some("FIX_FAILED"));
        assert_eq!(interpret_exit(Some(21)).unwrap().outcome, Outcome::Unknown);
        assert_eq!(
            interpret_exit(Some(16)),
            Err(LaunchError::Refused("HELPER_PATH_NOT_TRUSTED"))
        );
        assert_eq!(
            interpret_exit(Some(13)),
            Err(LaunchError::Refused("PLAN_HASH_MISMATCH"))
        );
        assert_eq!(
            interpret_exit(Some(SCRIPT_DENIED)),
            Err(LaunchError::Denied)
        );
        assert_eq!(
            interpret_exit(Some(SCRIPT_START_FAILED)),
            Err(LaunchError::NotAvailable)
        );
    }

    #[test]
    fn fr_010_codes_outside_the_contract_are_unknown_never_success() {
        for code in [
            None,
            Some(1),
            Some(SCRIPT_NO_EXIT_CODE),
            Some(-1),
            Some(255),
        ] {
            let result = interpret_exit(code).unwrap();
            assert_eq!(result.outcome, Outcome::Unknown, "{code:?}");
            assert_eq!(result.reason_code.as_deref(), Some("FIX_UNKNOWN"));
        }
    }

    #[test]
    fn script_exit_codes_are_outside_the_helper_range() {
        for code in [SCRIPT_DENIED, SCRIPT_START_FAILED, SCRIPT_NO_EXIT_CODE] {
            assert!(ElevatedError::from_exit_code(code).is_none());
            assert!(!(0..=21).contains(&code));
        }
    }

    #[test]
    fn fr_015_not_available_off_windows_or_without_the_installed_helper() {
        let launcher = UacLauncher {
            helper: PathBuf::from("/nao/existe/helpmydesk-elevated.exe"),
        };
        assert!(!launcher.available());
        assert_eq!(launcher.platform(), Platform::Windows);
    }
}
