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
fn fr_012_the_uninstaller_is_the_template_alone() {
    // The template deletes every file it installed, sidecars included, and then the folder. With no
    // hook of ours there is nothing custom in the uninstaller: no registry key, no extra question.
    let nsis = &desktop_config()["bundle"]["windows"]["nsis"];
    assert!(
        nsis.get("installerHooks").is_none(),
        "FR-012: an uninstaller hook would need its own review"
    );
}

/// F2-UNINST: one question about the history, the template's "Remover dados do programa" box,
/// which deletes in the profile of whoever uninstalls (`SetShellVarContext current`). Portuguese
/// first, so the box speaks the language of the person.
#[test]
fn f2_uninst_the_history_question_is_the_template_box_in_portuguese() {
    let languages = desktop_config()["bundle"]["windows"]["nsis"]["languages"].clone();
    assert_eq!(languages[0], "PortugueseBR", "{languages}");
}

/// Spec 020 T015: the Windows installer build merges the sidecar config, and the sidecar is the
/// helper with the file name the launcher looks for in the install folder.
#[test]
fn t015_the_windows_installer_carries_the_helper() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text =
        std::fs::read_to_string(root.join("apps/desktop/src-tauri/windows/helper.conf.json"))
            .expect("helper.conf.json is readable");
    let config: Value = serde_json::from_str(&text).expect("helper.conf.json is JSON");
    let bins = config["bundle"]["externalBin"]
        .as_array()
        .expect("externalBin is a list");
    assert_eq!(bins.len(), 1);
    let name = Path::new(bins[0].as_str().expect("a path"))
        .file_name()
        .expect("a file name")
        .to_string_lossy()
        .to_string();
    assert_eq!(
        format!("{name}.exe"),
        hmd_elevated::uac::HELPER_EXE,
        "the sidecar has to land where UacLauncher looks"
    );
    let ci = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).expect("ci.yml");
    assert!(
        ci.contains("--config src-tauri/windows/helper.conf.json"),
        "the installer build has to merge the sidecar config"
    );
}
