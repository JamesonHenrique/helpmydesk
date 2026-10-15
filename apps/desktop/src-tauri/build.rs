// SPDX-License-Identifier: GPL-3.0-or-later
//! Declares the app commands so each window only gets the ones its capability file lists
//! (INV-04: only the approval window may approve).

/// Every command in `src/lib.rs`, in the order they are registered.
const COMMANDS: &[&str] = &[
    "startup",
    "start_ticket",
    "diagnose",
    "open_approval",
    "plan",
    "approve",
    "decline",
    "execute",
    "confirm",
    "make_report",
    "edit_report",
    "report_exported",
    "save_report",
    "history",
    "settings",
    "system",
    "set_autonomy",
    "set_consent",
    "export_all",
    "erase_all",
];

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )?;
    Ok(())
}
