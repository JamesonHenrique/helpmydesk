// SPDX-License-Identifier: GPL-3.0-or-later
//! Execution journal states (spec 008 FR-004, Plan v1 V.10). Pure.

use crate::DomainError;

/// State of one step execution in the local journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionState {
    Planned,
    Approved,
    /// Written before the implementation is called (write-ahead).
    Started,
    Succeeded,
    Failed,
    /// Interrupted or unclear outcome; never re-run automatically.
    Unknown,
    VerifiedOk,
    VerifiedFailed,
    RolledBack,
}

impl ExecutionState {
    pub const ALL: [ExecutionState; 9] = [
        ExecutionState::Planned,
        ExecutionState::Approved,
        ExecutionState::Started,
        ExecutionState::Succeeded,
        ExecutionState::Failed,
        ExecutionState::Unknown,
        ExecutionState::VerifiedOk,
        ExecutionState::VerifiedFailed,
        ExecutionState::RolledBack,
    ];

    pub fn code(self) -> &'static str {
        match self {
            ExecutionState::Planned => "planned",
            ExecutionState::Approved => "approved",
            ExecutionState::Started => "started",
            ExecutionState::Succeeded => "succeeded",
            ExecutionState::Failed => "failed",
            ExecutionState::Unknown => "unknown",
            ExecutionState::VerifiedOk => "verified_ok",
            ExecutionState::VerifiedFailed => "verified_failed",
            ExecutionState::RolledBack => "rolled_back",
        }
    }

    /// Parses a stored [`ExecutionState::code`]; unknown codes return `None`.
    pub fn from_code(code: &str) -> Option<ExecutionState> {
        ExecutionState::ALL.into_iter().find(|s| s.code() == code)
    }

    /// No further journal change is expected (`verified_failed` may still be rolled back).
    pub fn is_final(self) -> bool {
        matches!(
            self,
            ExecutionState::Failed | ExecutionState::VerifiedOk | ExecutionState::RolledBack
        )
    }
}

/// FR-004: the allowed journal moves. `unknown` leaves only through an explicit clarification
/// (the capability's state probe says whether the effect happened), never by re-running.
pub fn transition(from: ExecutionState, to: ExecutionState) -> Result<ExecutionState, DomainError> {
    use ExecutionState as S;
    let allowed = matches!(
        (from, to),
        (S::Planned, S::Approved)
            | (S::Approved, S::Started)
            | (S::Started, S::Succeeded | S::Failed | S::Unknown)
            | (S::Unknown, S::Succeeded | S::Failed)
            | (S::Succeeded, S::VerifiedOk | S::VerifiedFailed)
            | (S::VerifiedFailed, S::RolledBack)
    );
    if allowed {
        Ok(to)
    } else {
        Err(DomainError::InvalidTransition {
            from: from.code(),
            event: to.code(),
        })
    }
}

/// US2.2: what an execution becomes when the app starts and finds it in this state.
pub fn on_startup(state: ExecutionState) -> ExecutionState {
    match state {
        ExecutionState::Started => ExecutionState::Unknown,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ExecutionState as S;

    /// Independent copy of the table in spec 008 US2.4.
    const ALLOWED: [(S, S); 10] = [
        (S::Planned, S::Approved),
        (S::Approved, S::Started),
        (S::Started, S::Succeeded),
        (S::Started, S::Failed),
        (S::Started, S::Unknown),
        (S::Unknown, S::Succeeded),
        (S::Unknown, S::Failed),
        (S::Succeeded, S::VerifiedOk),
        (S::Succeeded, S::VerifiedFailed),
        (S::VerifiedFailed, S::RolledBack),
    ];

    #[test]
    fn us2_4_full_transition_table() {
        for from in S::ALL {
            for to in S::ALL {
                let expected = ALLOWED.contains(&(from, to));
                assert_eq!(transition(from, to).is_ok(), expected, "{from:?} -> {to:?}");
            }
        }
    }

    #[test]
    fn us2_3_unknown_is_never_restarted() {
        assert!(transition(S::Unknown, S::Started).is_err());
        assert!(transition(S::Unknown, S::Approved).is_err());
    }

    #[test]
    fn us2_2_started_becomes_unknown_on_startup() {
        assert_eq!(on_startup(S::Started), S::Unknown);
        for s in S::ALL.into_iter().filter(|s| *s != S::Started) {
            assert_eq!(on_startup(s), s);
        }
    }

    #[test]
    fn codes_round_trip() {
        for s in S::ALL {
            assert_eq!(S::from_code(s.code()), Some(s));
        }
        assert_eq!(S::from_code("running"), None);
    }
}
