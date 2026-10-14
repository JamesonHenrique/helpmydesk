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
}
