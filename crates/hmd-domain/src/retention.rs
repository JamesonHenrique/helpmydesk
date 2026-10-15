// SPDX-License-Identifier: GPL-3.0-or-later
//! Local data retention (spec 007 FR-007, Plan v1 VII.4). Pure: the caller supplies `now`.

use crate::{DAY, Ticket, Timestamp};

/// Retention periods in seconds; the person can change them or erase everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionConfig {
    /// Tickets and everything attached, counted from the last change of a terminal ticket.
    pub ticket_after_end: i64,
    /// Free-text messages, counted from their creation.
    pub message_after: i64,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        RetentionConfig {
            ticket_after_end: 180 * DAY,
            message_after: 90 * DAY,
        }
    }
}

/// A `closed` or `cancelled` ticket expires `ticket_after_end` after its last change. A clock that
/// went backwards never expires anything.
pub fn ticket_expired(ticket: &Ticket, now: Timestamp, cfg: &RetentionConfig) -> bool {
    ticket.state.is_terminal()
        && now
            .checked_sub(ticket.last_changed_at)
            .is_some_and(|age| age >= cfg.ticket_after_end)
}

/// A message expires `message_after` after it was written.
pub fn message_expired(created_at: Timestamp, now: Timestamp, cfg: &RetentionConfig) -> bool {
    now.checked_sub(created_at)
        .is_some_and(|age| age >= cfg.message_after)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProblemFamily, TicketState};

    fn ticket(state: TicketState, last_changed_at: Timestamp) -> Ticket {
        let mut t = Ticket::new("1", ProblemFamily::Storage, 0);
        t.state = state;
        t.last_changed_at = last_changed_at;
        t
    }

    #[test]
    fn us3_terminal_tickets_expire_after_180_days() {
        let cfg = RetentionConfig::default();
        let now = 1_000 * DAY;
        for state in [TicketState::Closed, TicketState::Cancelled] {
            assert!(ticket_expired(&ticket(state, now - 180 * DAY), now, &cfg));
            assert!(!ticket_expired(&ticket(state, now - 179 * DAY), now, &cfg));
        }
    }

    #[test]
    fn us3_open_tickets_never_expire() {
        let cfg = RetentionConfig::default();
        for state in TicketState::ALL.into_iter().filter(|s| !s.is_terminal()) {
            assert!(!ticket_expired(&ticket(state, 0), 10_000 * DAY, &cfg));
        }
    }

    #[test]
    fn us3_messages_expire_after_90_days() {
        let cfg = RetentionConfig::default();
        let now = 1_000 * DAY;
        assert!(message_expired(now - 90 * DAY, now, &cfg));
        assert!(!message_expired(now - 89 * DAY, now, &cfg));
    }

    #[test]
    fn clock_backwards_expires_nothing() {
        let cfg = RetentionConfig::default();
        assert!(!ticket_expired(
            &ticket(TicketState::Closed, 500 * DAY),
            0,
            &cfg
        ));
        assert!(!message_expired(500 * DAY, 0, &cfg));
    }

    #[test]
    fn t001_codes_round_trip() {
        for v in TicketState::ALL {
            assert_eq!(TicketState::from_code(v.code()), Some(v));
        }
        for v in ProblemFamily::ALL {
            assert_eq!(ProblemFamily::from_code(v.code()), Some(v));
        }
        for v in crate::ReportTrigger::ALL {
            assert_eq!(crate::ReportTrigger::from_code(v.code()), Some(v));
        }
        for v in crate::SessionPhase::ALL {
            assert_eq!(crate::SessionPhase::from_code(v.code()), Some(v));
        }
        assert_eq!(TicketState::from_code("escalated"), None);
        assert_eq!(ProblemFamily::from_code(""), None);
    }
}
