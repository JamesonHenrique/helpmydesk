// SPDX-License-Identifier: GPL-3.0-or-later
//! Local ticket lifecycle (spec 001, FR-001..FR-005). The transition table mirrors
//! `specs/001-domain-ticket-lifecycle/plan.md` and is the single source of truth in code.

use crate::{DomainError, ProblemFamily, ReportTrigger, Timestamp};

/// FR-001: ticket states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TicketState {
    Open,
    InProgress,
    AwaitingUser,
    ResolvedPendingConfirmation,
    Resolved,
    Unresolved,
    Reopened,
    Cancelled,
    Abandoned,
    Closed,
}

impl TicketState {
    pub const ALL: [TicketState; 10] = [
        TicketState::Open,
        TicketState::InProgress,
        TicketState::AwaitingUser,
        TicketState::ResolvedPendingConfirmation,
        TicketState::Resolved,
        TicketState::Unresolved,
        TicketState::Reopened,
        TicketState::Cancelled,
        TicketState::Abandoned,
        TicketState::Closed,
    ];

    pub fn code(self) -> &'static str {
        match self {
            TicketState::Open => "open",
            TicketState::InProgress => "in_progress",
            TicketState::AwaitingUser => "awaiting_user",
            TicketState::ResolvedPendingConfirmation => "resolved_pending_confirmation",
            TicketState::Resolved => "resolved",
            TicketState::Unresolved => "unresolved",
            TicketState::Reopened => "reopened",
            TicketState::Cancelled => "cancelled",
            TicketState::Abandoned => "abandoned",
            TicketState::Closed => "closed",
        }
    }

    /// Terminal states accept no events.
    pub fn is_terminal(self) -> bool {
        matches!(self, TicketState::Cancelled | TicketState::Closed)
    }
}

/// Inputs that request a ticket state change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketEvent {
    StartSession,
    NeedUserInput,
    UserResponded,
    VerificationPassed,
    UserConfirmed,
    UserNotFixed,
    CannotResolve(ReportTrigger),
    Reopen,
    Cancel,
    Abandon,
    Close,
}

impl TicketEvent {
    pub fn code(self) -> &'static str {
        match self {
            TicketEvent::StartSession => "start_session",
            TicketEvent::NeedUserInput => "need_user_input",
            TicketEvent::UserResponded => "user_responded",
            TicketEvent::VerificationPassed => "verification_passed",
            TicketEvent::UserConfirmed => "user_confirmed",
            TicketEvent::UserNotFixed => "user_not_fixed",
            TicketEvent::CannotResolve(_) => "cannot_resolve",
            TicketEvent::Reopen => "reopen",
            TicketEvent::Cancel => "cancel",
            TicketEvent::Abandon => "abandon",
            TicketEvent::Close => "close",
        }
    }
}

/// FR-002: the allowed-transition table. Any pair not listed is rejected.
pub fn transition(from: TicketState, event: TicketEvent) -> Result<TicketState, DomainError> {
    use TicketEvent as E;
    use TicketState as S;
    let to = match (from, event) {
        (S::Open, E::StartSession) => S::InProgress,
        (S::Open, E::Cancel) => S::Cancelled,

        (S::InProgress, E::NeedUserInput) => S::AwaitingUser,
        (S::InProgress, E::VerificationPassed) => S::ResolvedPendingConfirmation,
        (S::InProgress, E::CannotResolve(_)) => S::Unresolved,
        (S::InProgress, E::Cancel) => S::Cancelled,

        (S::AwaitingUser, E::UserResponded) => S::InProgress,
        (S::AwaitingUser, E::Abandon) => S::Abandoned,
        (S::AwaitingUser, E::Cancel) => S::Cancelled,

        // FR-003: `resolved` only by explicit confirmation of the person.
        (S::ResolvedPendingConfirmation, E::UserConfirmed) => S::Resolved,
        (S::ResolvedPendingConfirmation, E::UserNotFixed) => S::InProgress,
        (S::ResolvedPendingConfirmation, E::Abandon) => S::Abandoned,

        (S::Resolved, E::Reopen) => S::Reopened,
        (S::Resolved, E::Close) => S::Closed,

        (S::Unresolved, E::Reopen) => S::Reopened,
        (S::Unresolved, E::Close) => S::Closed,

        (S::Reopened, E::StartSession) => S::InProgress,
        (S::Reopened, E::Cancel) => S::Cancelled,

        (S::Abandoned, E::Close) => S::Closed,

        (from, event) => {
            return Err(DomainError::InvalidTransition {
                from: from.code(),
                event: event.code(),
            });
        }
    };
    Ok(to)
}

/// A locally tracked problem report. Free text lives in messages, not here (privacy).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket {
    pub id: String,
    pub family: ProblemFamily,
    pub state: TicketState,
    pub reopen_count: u32,
    pub last_trigger: Option<ReportTrigger>,
    pub created_at: Timestamp,
    pub last_changed_at: Timestamp,
}

impl Ticket {
    pub fn new(id: impl Into<String>, family: ProblemFamily, now: Timestamp) -> Self {
        Ticket {
            id: id.into(),
            family,
            state: TicketState::Open,
            reopen_count: 0,
            last_trigger: None,
            created_at: now,
            last_changed_at: now,
        }
    }

    /// Applies an event and returns the updated ticket; `self` is left untouched on error.
    pub fn apply(&self, event: TicketEvent, now: Timestamp) -> Result<Ticket, DomainError> {
        let state = transition(self.state, event)?;
        let mut next = self.clone();
        next.state = state;
        next.last_changed_at = now;
        match event {
            // FR-004: entering `unresolved` records why.
            TicketEvent::CannotResolve(trigger) => next.last_trigger = Some(trigger),
            // FR-005: saturating counter.
            TicketEvent::Reopen => next.reopen_count = next.reopen_count.saturating_add(1),
            _ => {}
        }
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TicketEvent as E;
    use TicketState as S;

    const EVENTS: [TicketEvent; 11] = [
        E::StartSession,
        E::NeedUserInput,
        E::UserResponded,
        E::VerificationPassed,
        E::UserConfirmed,
        E::UserNotFixed,
        E::CannotResolve(ReportTrigger::NoApplicableFix),
        E::Reopen,
        E::Cancel,
        E::Abandon,
        E::Close,
    ];

    /// Independent copy of the table in plan.md, used to check `transition` (SC-001).
    const EXPECTED: [(TicketState, &str, TicketState); 19] = [
        (S::Open, "start_session", S::InProgress),
        (S::Open, "cancel", S::Cancelled),
        (S::InProgress, "need_user_input", S::AwaitingUser),
        (
            S::InProgress,
            "verification_passed",
            S::ResolvedPendingConfirmation,
        ),
        (S::InProgress, "cannot_resolve", S::Unresolved),
        (S::InProgress, "cancel", S::Cancelled),
        (S::AwaitingUser, "user_responded", S::InProgress),
        (S::AwaitingUser, "abandon", S::Abandoned),
        (S::AwaitingUser, "cancel", S::Cancelled),
        (
            S::ResolvedPendingConfirmation,
            "user_confirmed",
            S::Resolved,
        ),
        (
            S::ResolvedPendingConfirmation,
            "user_not_fixed",
            S::InProgress,
        ),
        (S::ResolvedPendingConfirmation, "abandon", S::Abandoned),
        (S::Resolved, "reopen", S::Reopened),
        (S::Resolved, "close", S::Closed),
        (S::Unresolved, "reopen", S::Reopened),
        (S::Unresolved, "close", S::Closed),
        (S::Reopened, "start_session", S::InProgress),
        (S::Reopened, "cancel", S::Cancelled),
        (S::Abandoned, "close", S::Closed),
    ];

    #[test]
    fn fr002_sc001_exhaustive_table() {
        for from in S::ALL {
            for event in EVENTS {
                let expected = EXPECTED
                    .iter()
                    .find(|(f, e, _)| *f == from && *e == event.code())
                    .map(|(_, _, to)| *to);
                match (transition(from, event), expected) {
                    (Ok(to), Some(exp)) => assert_eq!(to, exp, "{from:?} + {event:?}"),
                    (Err(err), None) => assert_eq!(err.reason_code(), "INVALID_TRANSITION"),
                    (got, exp) => panic!("{from:?} + {event:?}: got {got:?}, expected {exp:?}"),
                }
            }
        }
    }

    #[test]
    fn fr001_terminal_states_accept_nothing() {
        for state in S::ALL.into_iter().filter(|s| s.is_terminal()) {
            assert!(EVENTS.iter().all(|e| transition(state, *e).is_err()));
        }
    }

    #[test]
    fn fr003_resolved_only_reachable_by_confirmation() {
        for from in S::ALL {
            for event in EVENTS {
                if transition(from, event) == Ok(S::Resolved) {
                    assert_eq!(
                        (from, event),
                        (S::ResolvedPendingConfirmation, E::UserConfirmed)
                    );
                }
            }
        }
    }

    #[test]
    fn us1_happy_path_and_reopen() {
        let t = Ticket::new("t1", ProblemFamily::Connectivity, 10);
        let t = t.apply(E::StartSession, 11).unwrap();
        let t = t.apply(E::VerificationPassed, 12).unwrap();
        assert_eq!(t.state, S::ResolvedPendingConfirmation);
        let t = t.apply(E::UserConfirmed, 13).unwrap();
        assert_eq!(t.state, S::Resolved);
        let t = t.apply(E::Reopen, 14).unwrap();
        assert_eq!(
            (t.state, t.reopen_count, t.last_changed_at),
            (S::Reopened, 1, 14)
        );
    }

    #[test]
    fn us1_invalid_event_leaves_ticket_unchanged() {
        let t = Ticket::new("t1", ProblemFamily::Storage, 10);
        let err = t.apply(E::UserConfirmed, 11).unwrap_err();
        assert_eq!(err.reason_code(), "INVALID_TRANSITION");
        assert_eq!(t.state, S::Open);
        assert_eq!(t.last_changed_at, 10);
    }

    #[test]
    fn fr004_unresolved_records_trigger() {
        let t = Ticket::new("t1", ProblemFamily::AudioVideo, 0)
            .apply(E::StartSession, 1)
            .unwrap()
            .apply(E::CannotResolve(ReportTrigger::VerificationFailed), 2)
            .unwrap();
        assert_eq!(t.state, S::Unresolved);
        assert_eq!(t.last_trigger.map(|r| r.code()), Some("REP-03"));
        assert_eq!(t.apply(E::Reopen, 3).unwrap().state, S::Reopened);
    }

    #[test]
    fn fr005_reopen_counter_saturates() {
        let mut t = Ticket::new("t1", ProblemFamily::Performance, 0);
        t.state = S::Resolved;
        t.reopen_count = u32::MAX;
        assert_eq!(t.apply(E::Reopen, 1).unwrap().reopen_count, u32::MAX);
    }
}
