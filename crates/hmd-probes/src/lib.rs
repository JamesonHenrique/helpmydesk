// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only probes (spec 004). Probes never need elevation, never spawn processes and never
//! collect identifying data (host name, user name, serials, MACs, SSIDs, volume labels).
//! Thresholds live in [`rules`] as pure functions.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod av;
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
    (
        "av.audio.devices",
        &[
            rules::NO_OUTPUT_DEVICE,
            rules::NO_INPUT_DEVICE,
            rules::NO_SOUND_SERVER,
        ],
    ),
    ("av.camera.present", &[rules::NO_CAMERA]),
    (
        "av.privacy.mic_camera",
        &[rules::MIC_BLOCKED, rules::CAMERA_BLOCKED],
    ),
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

/// Replaces the user name and the host name inside untrusted text (process and app names can
/// contain them) with neutral placeholders. Case-insensitive; names shorter than 3 characters are
/// replaced only as whole words (e.g. a two-letter host name in "o PC ab"), so they do not mangle
/// unrelated words. The real names are read only to be removed.
pub fn redact_identity(text: &str) -> String {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    let host = sysinfo::System::host_name().unwrap_or_default();
    redact_names(
        text,
        &[
            (user.as_str(), "[usuário]"),
            (host.as_str(), "[computador]"),
        ],
    )
}

/// Separators around a short name for it to count as a whole word.
fn is_word_gap(c: Option<char>) -> bool {
    c.is_none_or(|c| {
        c.is_whitespace()
            || matches!(
                c,
                '.' | ',' | ';' | ':' | '!' | '?' | '(' | ')' | '"' | '\''
            )
    })
}

/// Pure part of [`redact_identity`], testable with fixed names.
pub fn redact_names(text: &str, names: &[(&str, &str)]) -> String {
    let mut out = text.to_string();
    for (name, placeholder) in names {
        let short = name.chars().count() < 3;
        if name.is_empty() {
            continue;
        }
        let lower_name = name.to_lowercase();
        let mut result = String::with_capacity(out.len());
        let mut rest = out.as_str();
        while let Some(pos) = rest.to_lowercase().find(&lower_name) {
            let end = pos + lower_name.len();
            // `to_lowercase` can change byte lengths for some scripts; only cut on char boundaries.
            if !rest.is_char_boundary(pos) || !rest.is_char_boundary(end) {
                break;
            }
            let whole_word = is_word_gap(rest[..pos].chars().next_back())
                && is_word_gap(rest[end..].chars().next());
            result.push_str(&rest[..pos]);
            if short && !whole_word {
                result.push_str(&rest[pos..end]);
            } else {
                result.push_str(placeholder);
            }
            rest = &rest[end..];
        }
        result.push_str(rest);
        out = result;
    }
    out
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
        "av.audio.devices" => av::audio_devices(),
        "av.camera.present" => av::camera(),
        "av.privacy.mic_camera" => av::privacy(),
        _ => Err(ProbeError::UnknownProbe),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fr005_redacts_user_and_host_case_insensitively() {
        let names = [("runner", "[usuário]"), ("fv-az123", "[computador]")];
        assert_eq!(
            redact_names("Runner.Worker on fv-AZ123", &names),
            "[usuário].Worker on [computador]"
        );
        assert_eq!(redact_names("chrome", &names), "chrome");
        assert_eq!(
            redact_names("ab-tool", &[("ab", "[x]")]),
            "ab-tool",
            "short names only as whole words"
        );
        assert_eq!(
            redact_names("o PC ab (casa) e o abacate, AB. ab-1", &[("ab", "[c]")]),
            "o PC [c] (casa) e o abacate, [c]. ab-1"
        );
        assert_eq!(redact_names("ab", &[("ab", "[c]")]), "[c]");
        assert_eq!(redact_names("abc", &[("", "[c]")]), "abc");
    }

    /// Spec 016: characters whose lowercase has another byte length (Turkish İ, the Kelvin sign)
    /// must not shift the match and leak the name.
    #[test]
    fn redaction_survives_case_mappings_that_change_length() {
        let names = [("james", "[usuário]")];
        assert_eq!(
            redact_names("\u{130}\u{130} james", &names),
            "\u{130}\u{130} [usuário]"
        );
        assert_eq!(
            redact_names("\u{212A}\u{212A}-JAMES-james", &names),
            "\u{212A}\u{212A}-[usuário]-[usuário]"
        );
        let turkish = [("\u{130}smail", "[usuário]")];
        assert_eq!(
            redact_names("/home/\u{130}smail/x \u{130}SMAIL", &turkish),
            "/home/[usuário]/x [usuário]"
        );
    }

    /// Every mix of tricky characters around the name: the name never survives.
    #[test]
    fn redaction_never_leaks_the_name() {
        let pieces = [
            "james", "JAMES", "\u{130}", "\u{212A}", "\u{2126}", "\u{1E9E}", "ß", "é", " ", "-",
            "a", "\u{3A3}",
        ];
        let mut seed: u64 = 0x5EED;
        for _ in 0..5000 {
            let mut text = String::new();
            for _ in 0..8 {
                seed = seed
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                text.push_str(pieces[(seed >> 33) as usize % pieces.len()]);
            }
            let out = redact_names(&text, &[("james", "[u]")]);
            assert!(!out.to_lowercase().contains("james"), "{text:?} -> {out:?}");
        }
    }
}
