// SPDX-License-Identifier: GPL-3.0-or-later
//! "Could not resolve" report triggers REP-01..REP-08 (Plan v1 VII.2, FR-004, FR-006).

use crate::{DAY, Timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReportTrigger {
    /// REP-01: playbook ended without an applicable fix.
    NoApplicableFix,
    /// REP-02: low confidence after exhausting the playbook probes.
    LowConfidence,
    /// REP-03: verification failed after the fix (rolled back when possible).
    VerificationFailed,
    /// REP-04: an execution stayed `unknown` and the state probe could not clarify it.
    UnknownExecutionState,
    /// REP-05: hardware, security or failing-disk finding.
    OutOfScopeFinding,
    /// REP-06: the person asked for a report.
    PersonRequested,
    /// REP-07: a blocked (N4) action was requested.
    BlockedRequest,
    /// REP-08: same family reopened repeatedly within the window.
    RepeatedReopen,
}

impl ReportTrigger {
    pub const ALL: [ReportTrigger; 8] = [
        ReportTrigger::NoApplicableFix,
        ReportTrigger::LowConfidence,
        ReportTrigger::VerificationFailed,
        ReportTrigger::UnknownExecutionState,
        ReportTrigger::OutOfScopeFinding,
        ReportTrigger::PersonRequested,
        ReportTrigger::BlockedRequest,
        ReportTrigger::RepeatedReopen,
    ];

    pub fn code(self) -> &'static str {
        match self {
            ReportTrigger::NoApplicableFix => "REP-01",
            ReportTrigger::LowConfidence => "REP-02",
            ReportTrigger::VerificationFailed => "REP-03",
            ReportTrigger::UnknownExecutionState => "REP-04",
            ReportTrigger::OutOfScopeFinding => "REP-05",
            ReportTrigger::PersonRequested => "REP-06",
            ReportTrigger::BlockedRequest => "REP-07",
            ReportTrigger::RepeatedReopen => "REP-08",
        }
    }
}

/// Default window for the repeated-reopen rule.
pub const REOPEN_WINDOW: i64 = 7 * DAY;
/// Reopens of the same family within the window that trigger REP-08.
pub const REOPEN_THRESHOLD: usize = 2;

/// FR-006: given the reopen timestamps of one problem family, returns REP-08 when at least
/// [`REOPEN_THRESHOLD`] reopens happened within [`REOPEN_WINDOW`] before `now`.
/// Timestamps in the future (clock skew) are ignored.
pub fn repeated_reopen_trigger(reopened_at: &[Timestamp], now: Timestamp) -> Option<ReportTrigger> {
    let recent = reopened_at
        .iter()
        .filter(|&&t| t <= now && now.saturating_sub(t) <= REOPEN_WINDOW)
        .count();
    (recent >= REOPEN_THRESHOLD).then_some(ReportTrigger::RepeatedReopen)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn codes_are_unique_and_sequential() {
        let codes: Vec<_> = ReportTrigger::ALL.iter().map(|t| t.code()).collect();
        assert_eq!(codes.iter().collect::<HashSet<_>>().len(), 8);
        for (i, code) in codes.iter().enumerate() {
            assert_eq!(*code, format!("REP-{:02}", i + 1));
        }
    }

    #[test]
    fn fr006_two_reopens_within_seven_days_trigger_rep08() {
        let now = 100 * DAY;
        assert_eq!(
            repeated_reopen_trigger(&[now - 6 * DAY, now - DAY], now),
            Some(ReportTrigger::RepeatedReopen)
        );
    }

    #[test]
    fn fr006_one_reopen_or_old_reopens_do_not_trigger() {
        let now = 100 * DAY;
        assert_eq!(repeated_reopen_trigger(&[now - DAY], now), None);
        assert_eq!(
            repeated_reopen_trigger(&[now - 8 * DAY, now - DAY], now),
            None
        );
        assert_eq!(repeated_reopen_trigger(&[], now), None);
    }

    #[test]
    fn fr006_future_timestamps_are_ignored() {
        let now = 100 * DAY;
        assert_eq!(repeated_reopen_trigger(&[now + DAY, now - DAY], now), None);
    }
}
