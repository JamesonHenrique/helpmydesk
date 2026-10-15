// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only probes (spec 004). Probes never need elevation, never spawn processes and never
//! collect identifying data (host name, user name, serials, MACs, SSIDs, volume labels).
//! Thresholds live in [`rules`] as pure functions.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod rules;

mod dns;
mod net;
mod startup;
mod system;

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

/// A finding code (declared in the catalog entry's `produces`) with its severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Finding {
    pub code: &'static str,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Evidence {
    Bool(bool),
    Uint(u64),
    Float(f64),
    Text(String),
}

/// Structured output of one probe run (FR-003).
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeOutput {
    pub probe_id: &'static str,
    pub findings: Vec<Finding>,
    pub evidence: BTreeMap<&'static str, Evidence>,
}

impl ProbeOutput {
    fn new(probe_id: &'static str) -> Self {
        ProbeOutput {
            probe_id,
            findings: Vec::new(),
            evidence: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeError {
    /// The probe ID is not implemented here.
    UnknownProbe,
    /// The OS did not provide the data the probe needs.
    NoData,
    /// The probe did not finish within its time limit.
    Timeout,
}

impl ProbeError {
    /// Stable reason code; the orchestrator records it as a `probe_failed` finding.
    pub fn reason_code(self) -> &'static str {
        match self {
            ProbeError::UnknownProbe => "UNKNOWN_PROBE",
            ProbeError::NoData => "NO_DATA",
            ProbeError::Timeout => "TIMEOUT",
        }
    }
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason_code())
    }
}

impl std::error::Error for ProbeError {}

/// Implemented probes and the findings each one may emit (FR-002, checked against the catalog).
pub const PROBES: &[(&str, &[&str])] = &[
    ("sys.info.basic", &[]),
    ("perf.memory.snapshot", &[rules::MEMORY_HIGH]),
    ("storage.volumes", &[rules::LOW_SPACE]),
    ("net.dns.resolve", &[rules::DNS_FAILING]),
    (
        "net.adapters.status",
        &[rules::NO_ACTIVE_ADAPTER, rules::NO_VALID_IP],
    ),
    (
        "net.gateway.reach",
        &[rules::GATEWAY_MISSING, rules::GATEWAY_UNREACHABLE],
    ),
    (
        "net.http.check",
        &[rules::INTERNET_UNREACHABLE, rules::CAPTIVE_PORTAL],
    ),
    ("perf.cpu.snapshot", &[rules::CPU_HIGH]),
    ("perf.top_processes", &[]),
    ("startup.apps.list", &[rules::MANY_STARTUP]),
];

/// Untrusted text from the system (app names, process names) is truncated and stripped of
/// control and invisible characters before it can reach the UI or the AI (Plan v1 V.9).
pub fn sanitize_untrusted(text: &str, max_chars: usize) -> String {
    text.chars()
        .filter(|c| !c.is_control() && !matches!(*c, '\u{200B}'..='\u{200F}' | '\u{2028}'..='\u{202E}' | '\u{2060}'..='\u{2069}' | '\u{FEFF}'))
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Runs a probe by catalog ID. Unknown IDs are rejected, never interpreted.
pub fn run(probe_id: &str) -> Result<ProbeOutput, ProbeError> {
    match probe_id {
        "sys.info.basic" => Ok(system::basic_info()),
        "perf.memory.snapshot" => system::memory(),
        "storage.volumes" => system::volumes(),
        "net.dns.resolve" => dns::resolve(dns::NAMES, dns::TIMEOUT),
        "net.adapters.status" => Ok(net::adapters()),
        "net.gateway.reach" => Ok(net::gateway_reach()),
        "net.http.check" => net::http_check(),
        "perf.cpu.snapshot" => system::cpu(),
        "perf.top_processes" => system::top_processes(),
        "startup.apps.list" => startup::list(),
        _ => Err(ProbeError::UnknownProbe),
    }
}
