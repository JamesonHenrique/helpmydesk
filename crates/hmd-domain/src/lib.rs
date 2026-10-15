// SPDX-License-Identifier: GPL-3.0-or-later
//! Pure domain core of HelpMyDesk (spec 001).
//!
//! This crate only *decides*: ticket lifecycle, diagnostic session phases, report triggers,
//! time-based housekeeping and local retention (spec 007). It performs no I/O, reads no clock and
//! has no dependencies; callers pass timestamps (Unix seconds) explicitly.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod error;
pub mod family;
pub mod housekeeping;
pub mod report;
pub mod retention;
pub mod session;
pub mod ticket;

pub use error::DomainError;
pub use family::ProblemFamily;
pub use housekeeping::{HousekeepingConfig, due_event};
pub use report::{ReportTrigger, repeated_reopen_trigger};
pub use retention::{RetentionConfig, message_expired, ticket_expired};
pub use session::{SessionEvent, SessionPhase};
pub use ticket::{Ticket, TicketEvent, TicketState};

/// Unix timestamp in seconds, always supplied by the caller (NFR-001).
pub type Timestamp = i64;

/// Seconds in one day, used by the default durations.
pub const DAY: i64 = 24 * 60 * 60;
