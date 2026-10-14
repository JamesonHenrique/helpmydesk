// SPDX-License-Identifier: GPL-3.0-or-later
//! Domain errors with stable, machine-readable reason codes (FR-009).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// The (state, event) pair is not in the allowed transition table.
    InvalidTransition {
        from: &'static str,
        event: &'static str,
    },
}

impl DomainError {
    /// Stable reason code; never renamed once released (the enum only grows).
    pub fn reason_code(&self) -> &'static str {
        match self {
            DomainError::InvalidTransition { .. } => "INVALID_TRANSITION",
        }
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::InvalidTransition { from, event } => {
                write!(
                    f,
                    "{}: event `{event}` not allowed in `{from}`",
                    self.reason_code()
                )
            }
        }
    }
}

impl std::error::Error for DomainError {}
