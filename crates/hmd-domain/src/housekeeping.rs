// SPDX-License-Identifier: GPL-3.0-or-later
//! Time-based housekeeping (spec 001, FR-008). Pure: the caller supplies `now`.

use crate::{DAY, Ticket, TicketEvent, TicketState, Timestamp};

/// Idle durations in seconds; defaults follow Plan v1 VII.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HousekeepingConfig {
    pub abandon_after: i64,
    pub close_resolved_after: i64,
    pub close_unresolved_after: i64,
}

impl Default for HousekeepingConfig {
    fn default() -> Self {
        HousekeepingConfig {
            abandon_after: 14 * DAY,
            close_resolved_after: 7 * DAY,
            close_unresolved_after: 30 * DAY,
        }
    }
}

/// Returns the event that housekeeping should apply now, if any. A clock that went backwards
/// (`now < last_changed_at`) never produces an event.
pub fn due_event(ticket: &Ticket, now: Timestamp, cfg: &HousekeepingConfig) -> Option<TicketEvent> {
    let idle = now.checked_sub(ticket.last_changed_at)?;
    if idle < 0 {
        return None;
    }
    match ticket.state {
        TicketState::AwaitingUser | TicketState::ResolvedPendingConfirmation
            if idle >= cfg.abandon_after =>
        {
            Some(TicketEvent::Abandon)
        }
        TicketState::Resolved if idle >= cfg.close_resolved_after => Some(TicketEvent::Close),
        TicketState::Unresolved if idle >= cfg.close_unresolved_after => Some(TicketEvent::Close),
        TicketState::Abandoned => Some(TicketEvent::Close),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProblemFamily, ticket::transition};

    fn ticket_in(state: TicketState, changed_at: Timestamp) -> Ticket {
        let mut t = Ticket::new("t", ProblemFamily::Connectivity, changed_at);
        t.state = state;
        t
    }

    #[test]
    fn fr008_defaults() {
        let cfg = HousekeepingConfig::default();
        let at = 1_000 * DAY;
        let cases = [
            (
                TicketState::AwaitingUser,
                14 * DAY,
                Some(TicketEvent::Abandon),
            ),
            (TicketState::AwaitingUser, 13 * DAY, None),
            (
                TicketState::ResolvedPendingConfirmation,
                14 * DAY,
                Some(TicketEvent::Abandon),
            ),
            (TicketState::Resolved, 6 * DAY, None),
            (TicketState::Resolved, 7 * DAY, Some(TicketEvent::Close)),
            (TicketState::Unresolved, 29 * DAY, None),
            (TicketState::Unresolved, 30 * DAY, Some(TicketEvent::Close)),
            (TicketState::Abandoned, 0, Some(TicketEvent::Close)),
            (TicketState::InProgress, 365 * DAY, None),
            (TicketState::Closed, 365 * DAY, None),
        ];
        for (state, idle, expected) in cases {
            assert_eq!(
                due_event(&ticket_in(state, at), at + idle, &cfg),
                expected,
                "{state:?}"
            );
        }
    }

    #[test]
    fn fr008_due_events_are_always_valid_transitions() {
        let cfg = HousekeepingConfig::default();
        for state in TicketState::ALL {
            if let Some(event) = due_event(&ticket_in(state, 0), 365 * DAY, &cfg) {
                assert!(transition(state, event).is_ok(), "{state:?} + {event:?}");
            }
        }
    }

    #[test]
    fn fr008_clock_backwards_yields_nothing() {
        let cfg = HousekeepingConfig::default();
        let t = ticket_in(TicketState::Abandoned, 100 * DAY);
        assert_eq!(due_event(&t, 99 * DAY, &cfg), None);
    }

    #[test]
    fn us4_pending_confirmation_never_auto_resolves() {
        let cfg = HousekeepingConfig::default();
        let t = ticket_in(TicketState::ResolvedPendingConfirmation, 0);
        assert_ne!(
            due_event(&t, 365 * DAY, &cfg),
            Some(TicketEvent::UserConfirmed)
        );
    }
}
