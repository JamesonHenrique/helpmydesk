// SPDX-License-Identifier: GPL-3.0-or-later
//! Pure threshold rules (spec 004 FR-004). Inputs are numbers; outputs are findings.

use crate::{Finding, Severity};

pub const MEMORY_HIGH: &str = "perf.memory.high";
pub const LOW_SPACE: &str = "storage.low_space";
pub const DNS_FAILING: &str = "net.dns.resolution_failing";

/// Memory available below this percentage of total → `perf.memory.high`.
pub const MEMORY_AVAILABLE_MIN_PCT: u64 = 10;
/// System volume free below this percentage of total → `storage.low_space`.
pub const DISK_FREE_MIN_PCT: u64 = 10;
/// System volume free below this many bytes (5 GiB) → `storage.low_space`.
pub const DISK_FREE_MIN_BYTES: u64 = 5 * 1024 * 1024 * 1024;

pub fn memory_findings(total: u64, available: u64) -> Vec<Finding> {
    if total > 0 && (available as u128) * 100 < (total as u128) * MEMORY_AVAILABLE_MIN_PCT as u128 {
        vec![Finding {
            code: MEMORY_HIGH,
            severity: Severity::Warning,
        }]
    } else {
        vec![]
    }
}

pub fn disk_findings(total: u64, free: u64) -> Vec<Finding> {
    let low_pct = total > 0 && (free as u128) * 100 < (total as u128) * DISK_FREE_MIN_PCT as u128;
    if total > 0 && (low_pct || free < DISK_FREE_MIN_BYTES) {
        vec![Finding {
            code: LOW_SPACE,
            severity: Severity::Warning,
        }]
    } else {
        vec![]
    }
}

pub fn dns_findings(resolved: usize, attempted: usize) -> Vec<Finding> {
    if attempted > 0 && resolved == 0 {
        vec![Finding {
            code: DNS_FAILING,
            severity: Severity::Critical,
        }]
    } else {
        vec![]
    }
}

pub const NO_ACTIVE_ADAPTER: &str = "net.no_active_adapter";
pub const NO_VALID_IP: &str = "net.no_valid_ip";
pub const GATEWAY_MISSING: &str = "net.gateway.missing";
pub const GATEWAY_UNREACHABLE: &str = "net.gateway.unreachable";
pub const INTERNET_UNREACHABLE: &str = "net.internet.unreachable";
pub const CAPTIVE_PORTAL: &str = "net.captive_portal_suspected";
pub const CPU_HIGH: &str = "perf.cpu.high";
pub const MANY_STARTUP: &str = "startup.many_apps";

/// Global CPU usage at or above this percentage → `perf.cpu.high`.
pub const CPU_HIGH_PCT: f32 = 90.0;
/// Enabled startup entries at or above this count → `startup.many_apps`.
pub const STARTUP_MANY: usize = 8;

fn finding(code: &'static str, severity: Severity) -> Vec<Finding> {
    vec![Finding { code, severity }]
}

/// A usable IPv4 address: not loopback, link-local (169.254/16, APIPA) or unspecified.
pub fn is_valid_ipv4(ip: std::net::Ipv4Addr) -> bool {
    !(ip.is_loopback() || ip.is_link_local() || ip.is_unspecified())
}

pub fn adapter_findings(active_adapters: usize, default_has_valid_ipv4: bool) -> Vec<Finding> {
    if active_adapters == 0 {
        finding(NO_ACTIVE_ADAPTER, Severity::Critical)
    } else if !default_has_valid_ipv4 {
        finding(NO_VALID_IP, Severity::Critical)
    } else {
        vec![]
    }
}

pub fn gateway_findings(gateway_present: bool, answered: bool) -> Vec<Finding> {
    match (gateway_present, answered) {
        (false, _) => finding(GATEWAY_MISSING, Severity::Critical),
        (true, false) => finding(GATEWAY_UNREACHABLE, Severity::Critical),
        (true, true) => vec![],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpOutcome {
    /// The endpoint answered exactly as expected: there is internet.
    Expected,
    /// Something answered, but not the endpoint (typically a captive portal redirect).
    Unexpected,
    NoResponse,
}

/// Classifies a raw HTTP/1.x response against the expected status and optional body marker.
pub fn classify_http(
    response: &[u8],
    expect_status: u16,
    expect_body: Option<&str>,
) -> HttpOutcome {
    if response.is_empty() {
        return HttpOutcome::NoResponse;
    }
    let text = String::from_utf8_lossy(response);
    let status = text
        .lines()
        .next()
        .filter(|l| l.starts_with("HTTP/1."))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok());
    let body_ok = match expect_body {
        None => true,
        Some(marker) => text
            .split_once("\r\n\r\n")
            .is_some_and(|(_, body)| body.trim() == marker),
    };
    if status == Some(expect_status) && body_ok {
        HttpOutcome::Expected
    } else {
        HttpOutcome::Unexpected
    }
}

pub fn http_findings(outcomes: &[HttpOutcome]) -> Vec<Finding> {
    if outcomes.is_empty() || outcomes.contains(&HttpOutcome::Expected) {
        vec![]
    } else if outcomes.contains(&HttpOutcome::Unexpected) {
        finding(CAPTIVE_PORTAL, Severity::Warning)
    } else {
        finding(INTERNET_UNREACHABLE, Severity::Critical)
    }
}

pub fn cpu_findings(usage_pct: f32) -> Vec<Finding> {
    if usage_pct >= CPU_HIGH_PCT {
        finding(CPU_HIGH, Severity::Warning)
    } else {
        vec![]
    }
}

pub fn startup_findings(enabled: usize) -> Vec<Finding> {
    if enabled >= STARTUP_MANY {
        finding(MANY_STARTUP, Severity::Info)
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn codes(f: Vec<Finding>) -> Vec<&'static str> {
        f.into_iter().map(|f| f.code).collect()
    }

    #[test]
    fn us1_memory_high_below_ten_percent() {
        assert_eq!(codes(memory_findings(16 * GIB, GIB)), vec![MEMORY_HIGH]);
        assert!(memory_findings(16 * GIB, 2 * GIB).is_empty());
        assert!(memory_findings(0, 0).is_empty());
        assert!(memory_findings(u64::MAX, 0).len() == 1, "no overflow");
    }

    #[test]
    fn us1_low_space_by_percent_or_absolute() {
        assert_eq!(codes(disk_findings(1000 * GIB, 50 * GIB)), vec![LOW_SPACE]);
        assert_eq!(codes(disk_findings(30 * GIB, 4 * GIB)), vec![LOW_SPACE]);
        assert!(disk_findings(500 * GIB, 100 * GIB).is_empty());
        assert!(disk_findings(0, 0).is_empty());
    }

    #[test]
    fn us2_dns_failing_only_when_all_fail() {
        assert_eq!(codes(dns_findings(0, 3)), vec![DNS_FAILING]);
        assert!(dns_findings(1, 3).is_empty());
        assert!(dns_findings(0, 0).is_empty());
    }

    #[test]
    fn batch2_us1_adapter_rules() {
        assert_eq!(codes(adapter_findings(0, false)), vec![NO_ACTIVE_ADAPTER]);
        assert_eq!(codes(adapter_findings(1, false)), vec![NO_VALID_IP]);
        assert!(adapter_findings(2, true).is_empty());
        assert!(!is_valid_ipv4("169.254.10.2".parse().unwrap()));
        assert!(!is_valid_ipv4("127.0.0.1".parse().unwrap()));
        assert!(is_valid_ipv4("192.168.0.8".parse().unwrap()));
    }

    #[test]
    fn batch2_us1_gateway_rules() {
        assert_eq!(codes(gateway_findings(false, false)), vec![GATEWAY_MISSING]);
        assert_eq!(
            codes(gateway_findings(true, false)),
            vec![GATEWAY_UNREACHABLE]
        );
        assert!(gateway_findings(true, true).is_empty());
    }

    #[test]
    fn batch2_us1_http_classification() {
        let ok204 = b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n";
        let ms = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nMicrosoft Connect Test";
        let portal = b"HTTP/1.1 302 Found\r\nLocation: http://login.hotel/\r\n\r\n";
        let fake200 = b"HTTP/1.1 200 OK\r\n\r\n<html>Login</html>";
        assert_eq!(classify_http(ok204, 204, None), HttpOutcome::Expected);
        assert_eq!(
            classify_http(ms, 200, Some("Microsoft Connect Test")),
            HttpOutcome::Expected
        );
        assert_eq!(classify_http(portal, 204, None), HttpOutcome::Unexpected);
        assert_eq!(
            classify_http(fake200, 200, Some("Microsoft Connect Test")),
            HttpOutcome::Unexpected
        );
        assert_eq!(classify_http(b"", 204, None), HttpOutcome::NoResponse);
        assert_eq!(
            classify_http(b"garbage", 204, None),
            HttpOutcome::Unexpected
        );
    }

    #[test]
    fn batch2_us1_http_findings() {
        use HttpOutcome::*;
        assert!(http_findings(&[Expected, NoResponse]).is_empty());
        assert_eq!(
            codes(http_findings(&[Unexpected, NoResponse])),
            vec![CAPTIVE_PORTAL]
        );
        assert_eq!(
            codes(http_findings(&[NoResponse, NoResponse])),
            vec![INTERNET_UNREACHABLE]
        );
        assert!(http_findings(&[]).is_empty());
    }

    #[test]
    fn batch2_us2_cpu_and_startup_rules() {
        assert_eq!(codes(cpu_findings(95.0)), vec![CPU_HIGH]);
        assert!(cpu_findings(40.0).is_empty());
        assert_eq!(codes(startup_findings(8)), vec![MANY_STARTUP]);
        assert!(startup_findings(7).is_empty());
    }
}
