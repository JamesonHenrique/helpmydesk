// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 020: the installer and the helper have to agree on where the helper lives. SEC-020 only
//! holds if the Windows installer is the per-machine one *and* that is the folder `trusted_path`
//! accepts, so both facts are read from the real files instead of being assumed here.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use hmd_elevated::{INSTALL_DIR, trusted_path};
use serde_json::Value;

fn desktop_config() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/desktop/src-tauri/tauri.conf.json");
    let text = std::fs::read_to_string(&path).expect("tauri.conf.json is readable");
    serde_json::from_str(&text).expect("tauri.conf.json is JSON")
}

#[test]
fn sec_020_the_windows_installer_is_per_machine() {
    let nsis = &desktop_config()["bundle"]["windows"]["nsis"];
    assert_eq!(
        nsis["installMode"], "perMachine",
        "SEC-020: the helper only runs from an admin-only folder, so the installer must be per machine"
    );
}

#[test]
fn fr_011_the_helper_only_runs_from_the_per_machine_folder() {
    assert!(
        INSTALL_DIR.starts_with(r"C:\Program Files"),
        "FR-011: {INSTALL_DIR:?} is not a folder only the administrator can write to"
    );
    // The folder the installer creates is the folder the helper trusts, by name: `productName` is
    // what the NSIS template puts inside `Program Files`.
    let product = desktop_config()["productName"].clone();
    let product = product.as_str().expect("productName is a string");
    assert!(
        INSTALL_DIR.ends_with(product),
        "FR-011: INSTALL_DIR {INSTALL_DIR:?} is not the per-machine folder of {product:?}"
    );
    let install_dir = Path::new(INSTALL_DIR);
    assert!(trusted_path(
        &install_dir.join("helpmydesk-elevated.exe"),
        install_dir
    ));
    assert!(!trusted_path(
        Path::new(r"C:\Users\Pessoa\AppData\Local\HelpMyDesk\helpmydesk-elevated.exe"),
        install_dir
    ));
    assert!(!trusted_path(
        Path::new("/tmp/helpmydesk-elevated"),
        install_dir
    ));
}

#[test]
fn fr_012_the_uninstaller_takes_the_install_folder_with_it() {
    // The helper lives inside $INSTDIR, so the NSIS template that removes that folder removes the
    // helper with it. Nothing else is ours under Program Files, and the helper writes no registry
    // key, so the hook only has to cover the per-user history (F2-UNINST).
    let hooks = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/desktop/src-tauri/windows/hooks.nsh"),
    )
    .expect("the installer hook file is readable");
    assert!(hooks.contains("NSIS_HOOK_POSTUNINSTALL"));
    assert!(
        !hooks.contains(r"DeleteRegKey HKLM"),
        "FR-012: the helper creates no registry key; a key removal here would be unexplained"
    );
}

/// F2-UNINST with the per-machine installer: the template runs with `SetShellVarContext all`, so
/// `$APPDATA` would be ProgramData and the history of the person would stay. The hook switches to
/// `current` before deleting and back to `all` after.
#[test]
fn f2_uninst_history_is_deleted_in_the_person_profile() {
    let hooks = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/desktop/src-tauri/windows/hooks.nsh"),
    )
    .expect("the installer hook file is readable");
    let current = hooks
        .find("SetShellVarContext current")
        .expect("context switch");
    let delete = hooks.find(r#"RMDir /r "$APPDATA"#).expect("history delete");
    let back = hooks
        .rfind("SetShellVarContext all")
        .expect("context restored");
    assert!(current < delete && delete < back, "{hooks}");
}
