// SPDX-License-Identifier: GPL-3.0-or-later
//! OS, memory and disk probes via `sysinfo` (safe API, no processes, no elevation).

use std::path::{Path, PathBuf};

use sysinfo::{Disks, System};

use crate::{Evidence, ProbeError, ProbeOutput, rules};

/// `sys.info.basic`: OS name/version, kernel/build, architecture, uptime.
/// Deliberately omits `System::host_name()` and anything user-related (FR-005).
pub(crate) fn basic_info() -> ProbeOutput {
    let mut out = ProbeOutput::new("sys.info.basic");
    let text = |v: Option<String>| v.map(Evidence::Text);
    if let Some(v) = text(System::name()) {
        out.evidence.insert("os_name", v);
    }
    if let Some(v) = text(System::os_version()) {
        out.evidence.insert("os_version", v);
    }
    if let Some(v) = text(System::kernel_version()) {
        out.evidence.insert("kernel_version", v);
    }
    out.evidence
        .insert("arch", Evidence::Text(std::env::consts::ARCH.to_string()));
    out.evidence
        .insert("uptime_seconds", Evidence::Uint(System::uptime()));
    out
}

/// `perf.memory.snapshot`: total and available memory in bytes.
pub(crate) fn memory() -> Result<ProbeOutput, ProbeError> {
    let mut sys = System::new();
    sys.refresh_memory();
    let (total, available) = (sys.total_memory(), sys.available_memory());
    if total == 0 {
        return Err(ProbeError::NoData);
    }
    let mut out = ProbeOutput::new("perf.memory.snapshot");
    out.evidence.insert("total_bytes", Evidence::Uint(total));
    out.evidence
        .insert("available_bytes", Evidence::Uint(available));
    out.evidence.insert(
        "available_pct",
        Evidence::Float((available as f64 / total as f64 * 1000.0).round() / 10.0),
    );
    out.findings = rules::memory_findings(total, available);
    Ok(out)
}

/// Mount point of the system volume: `/` on Linux, `%SystemDrive%\` on Windows.
fn system_mount() -> PathBuf {
    if cfg!(windows) {
        let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
        PathBuf::from(format!("{drive}\\"))
    } else {
        PathBuf::from("/")
    }
}

/// `storage.volumes`: total/free space of the system volume. Never volume labels (FR-005).
pub(crate) fn volumes() -> Result<ProbeOutput, ProbeError> {
    let disks = Disks::new_with_refreshed_list();
    let target = system_mount();
    let disk = disks
        .list()
        .iter()
        .find(|d| same_mount(d.mount_point(), &target))
        .ok_or(ProbeError::NoData)?;
    let (total, free) = (disk.total_space(), disk.available_space());
    if total == 0 {
        return Err(ProbeError::NoData);
    }
    let mut out = ProbeOutput::new("storage.volumes");
    out.evidence.insert(
        "mount",
        Evidence::Text(target.to_string_lossy().into_owned()),
    );
    out.evidence.insert("total_bytes", Evidence::Uint(total));
    out.evidence.insert("free_bytes", Evidence::Uint(free));
    out.findings = rules::disk_findings(total, free);
    Ok(out)
}

fn same_mount(a: &Path, b: &Path) -> bool {
    if cfg!(windows) {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    } else {
        a == b
    }
}
