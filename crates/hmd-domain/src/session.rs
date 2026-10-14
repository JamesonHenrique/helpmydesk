// SPDX-License-Identifier: GPL-3.0-or-later
//! Diagnostic session phases (spec 001, FR-007). Table mirrors plan.md.

use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionPhase {
    Triage,
    Probing,
    Planning,
    AwaitingApproval,
    Executing,
    Verifying,
    Done,
    Failed,
    Aborted,
}

impl SessionPhase {
    pub const ALL: [SessionPhase; 9] = [
        SessionPhase::Triage,
        SessionPhase::Probing,
        SessionPhase::Planning,
        SessionPhase::AwaitingApproval,
        SessionPhase::Executing,
        SessionPhase::Verifying,
        SessionPhase::Done,
        SessionPhase::Failed,
        SessionPhase::Aborted,
    ];

    pub fn code(self) -> &'static str {
        match self {
            SessionPhase::Triage => "triage",
            SessionPhase::Probing => "probing",
            SessionPhase::Planning => "planning",
            SessionPhase::AwaitingApproval => "awaiting_approval",
            SessionPhase::Executing => "executing",
            SessionPhase::Verifying => "verifying",
            SessionPhase::Done => "done",
            SessionPhase::Failed => "failed",
            SessionPhase::Aborted => "aborted",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            SessionPhase::Done | SessionPhase::Failed | SessionPhase::Aborted
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    TriageCompleted,
    ProbesCompleted,
    PlanRequiresApproval,
    PlanReady,
    NoActionablePlan,
    Approved,
    Refused,
    ExecutionFinished,
    VerificationFinished,
    Fail,
    Stop,
}

impl SessionEvent {
    pub const ALL: [SessionEvent; 11] = [
        SessionEvent::TriageCompleted,
        SessionEvent::ProbesCompleted,
        SessionEvent::PlanRequiresApproval,
        SessionEvent::PlanReady,
        SessionEvent::NoActionablePlan,
        SessionEvent::Approved,
        SessionEvent::Refused,
        SessionEvent::ExecutionFinished,
        SessionEvent::VerificationFinished,
        SessionEvent::Fail,
        SessionEvent::Stop,
    ];

    pub fn code(self) -> &'static str {
        match self {
            SessionEvent::TriageCompleted => "triage_completed",
            SessionEvent::ProbesCompleted => "probes_completed",
            SessionEvent::PlanRequiresApproval => "plan_requires_approval",
            SessionEvent::PlanReady => "plan_ready",
            SessionEvent::NoActionablePlan => "no_actionable_plan",
            SessionEvent::Approved => "approved",
            SessionEvent::Refused => "refused",
            SessionEvent::ExecutionFinished => "execution_finished",
            SessionEvent::VerificationFinished => "verification_finished",
            SessionEvent::Fail => "fail",
            SessionEvent::Stop => "stop",
        }
    }
}

/// FR-007: allowed phase transitions; `fail` and `stop` apply to every non-terminal phase.
pub fn transition(from: SessionPhase, event: SessionEvent) -> Result<SessionPhase, DomainError> {
    use SessionEvent as E;
    use SessionPhase as P;
    let to = match (from, event) {
        (from, E::Fail) if !from.is_terminal() => P::Failed,
        (from, E::Stop) if !from.is_terminal() => P::Aborted,
        (P::Triage, E::TriageCompleted) => P::Probing,
        (P::Probing, E::ProbesCompleted) => P::Planning,
        (P::Planning, E::PlanRequiresApproval) => P::AwaitingApproval,
        (P::Planning, E::PlanReady) => P::Executing,
        (P::Planning, E::NoActionablePlan) => P::Done,
        (P::AwaitingApproval, E::Approved) => P::Executing,
        (P::AwaitingApproval, E::Refused) => P::Done,
        (P::Executing, E::ExecutionFinished) => P::Verifying,
        (P::Verifying, E::VerificationFinished) => P::Done,
        (from, event) => {
            return Err(DomainError::InvalidTransition {
                from: from.code(),
                event: event.code(),
            });
        }
    };
    Ok(to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use SessionEvent as E;
    use SessionPhase as P;

    /// Independent copy of the plan.md table without the `fail`/`stop` columns (SC-001).
    const EXPECTED: [(SessionPhase, SessionEvent, SessionPhase); 9] = [
        (P::Triage, E::TriageCompleted, P::Probing),
        (P::Probing, E::ProbesCompleted, P::Planning),
        (P::Planning, E::PlanRequiresApproval, P::AwaitingApproval),
        (P::Planning, E::PlanReady, P::Executing),
        (P::Planning, E::NoActionablePlan, P::Done),
        (P::AwaitingApproval, E::Approved, P::Executing),
        (P::AwaitingApproval, E::Refused, P::Done),
        (P::Executing, E::ExecutionFinished, P::Verifying),
        (P::Verifying, E::VerificationFinished, P::Done),
    ];

    #[test]
    fn fr007_sc001_exhaustive_table() {
        for from in P::ALL {
            for event in E::ALL {
                let expected = match event {
                    E::Fail if !from.is_terminal() => Some(P::Failed),
                    E::Stop if !from.is_terminal() => Some(P::Aborted),
                    _ => EXPECTED
                        .iter()
                        .find(|(f, e, _)| *f == from && *e == event)
                        .map(|(_, _, to)| *to),
                };
                match (transition(from, event), expected) {
                    (Ok(to), Some(exp)) => assert_eq!(to, exp, "{from:?} + {event:?}"),
                    (Err(err), None) => assert_eq!(err.reason_code(), "INVALID_TRANSITION"),
                    (got, exp) => panic!("{from:?} + {event:?}: got {got:?}, expected {exp:?}"),
                }
            }
        }
    }

    #[test]
    fn us3_execution_only_after_approval_or_plan_without_approval() {
        for from in P::ALL {
            for event in E::ALL {
                if transition(from, event) == Ok(P::Executing) {
                    assert!(matches!(
                        (from, event),
                        (P::AwaitingApproval, E::Approved) | (P::Planning, E::PlanReady)
                    ));
                }
            }
        }
    }
}
