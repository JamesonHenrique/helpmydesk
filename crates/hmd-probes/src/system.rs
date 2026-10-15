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

/// Sampling window for CPU usage (must be ≥ `sysinfo::MINIMUM_CPU_UPDATE_INTERVAL`).
fn cpu_sample_window() -> std::time::Duration {
    sysinfo::MINIMUM_CPU_UPDATE_INTERVAL.max(std::time::Duration::from_millis(500))
}

/// `perf.cpu.snapshot`: global CPU usage over a short sample.
pub(crate) fn cpu() -> Result<ProbeOutput, ProbeError> {
    let mut sys = System::new();
    sys.refresh_cpu_usage();
    std::thread::sleep(cpu_sample_window());
    sys.refresh_cpu_usage();
    let cpus = sys.cpus().len();
    if cpus == 0 {
        return Err(ProbeError::NoData);
    }
    let usage = sys.global_cpu_usage();
    let mut out = ProbeOutput::new("perf.cpu.snapshot");
    out.evidence.insert(
        "cpu_usage_pct",
        Evidence::Float((f64::from(usage) * 10.0).round() / 10.0),
    );
    out.evidence
        .insert("logical_cpus", Evidence::Uint(cpus as u64));
    out.findings = rules::cpu_findings(usage);
    Ok(out)
}

const TOP_KEYS: [&str; 5] = [
    "process_1",
    "process_2",
    "process_3",
    "process_4",
    "process_5",
];

/// `perf.top_processes`: the 5 processes using the most memory. Only executable name, CPU and
/// memory are refreshed: no command line, path, environment or user (spec 005 FR-005).
pub(crate) fn top_processes() -> Result<ProbeOutput, ProbeError> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
    // `without_tasks`: on Linux threads would otherwise be listed as separate processes.
    let kind = ProcessRefreshKind::nothing()
        .with_memory()
        .with_cpu()
        .without_tasks();
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
    std::thread::sleep(cpu_sample_window());
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);

    let mut procs: Vec<(String, f32, u64)> = sys
        .processes()
        .values()
        .filter(|p| p.thread_kind().is_none())
        .map(|p| {
            (
                crate::redact_identity(&crate::sanitize_untrusted(&p.name().to_string_lossy(), 60)),
                p.cpu_usage(),
                p.memory(),
            )
        })
        .collect();
    if procs.is_empty() {
        return Err(ProbeError::NoData);
    }
    procs.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    let mut out = ProbeOutput::new("perf.top_processes");
    for (key, (name, cpu, mem)) in TOP_KEYS.iter().zip(procs) {
        out.evidence.insert(
            key,
            Evidence::Text(format!(
                "{name} — {:.1} MB, {:.0}% CPU",
                mem as f64 / 1_048_576.0,
                cpu
            )),
        );
    }
    Ok(out)
}
