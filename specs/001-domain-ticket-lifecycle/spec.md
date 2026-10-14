# Feature Specification: Domain core — ticket lifecycle, session phases and report triggers

**Feature Branch**: `001-domain-ticket-lifecycle`
**Created**: 2026-10-14
**Status**: Approved (owner pre-approved development start, ADR-020)
**Input**: Plan v1, Part VII.2 — "Pure domain core: local ticket lifecycle, diagnostic session
phases and 'could not resolve' report triggers"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A problem report has an honest, predictable lifecycle (Priority: P1)

A person reports a problem; the app tracks it from open to resolved, unresolved, cancelled or
closed. The person can always cancel, can reopen when the problem comes back, and is never told
"resolved" before confirming it.

**Why this priority**: every other feature (probes, AI, fixes, reports) attaches to the ticket;
an inconsistent lifecycle would make history, undo and honesty guarantees unreliable.

**Independent Test**: exercise the pure transition function with every (state, event) pair and
compare against the allowed-transition table.

**Acceptance Scenarios**:

1. **Given** an `open` ticket, **When** a session starts, **Then** the ticket is `in_progress`.
2. **Given** an `in_progress` ticket, **When** verification passes, **Then** the ticket is
   `resolved_pending_confirmation` (never directly `resolved`).
3. **Given** a `resolved_pending_confirmation` ticket, **When** the person confirms, **Then** it
   is `resolved`; **When** the person says it is not fixed, **Then** it goes back to
   `in_progress`.
4. **Given** a `resolved` ticket, **When** the person reopens it, **Then** it is `reopened` and
   its reopen count increases by one.
5. **Given** any non-terminal ticket, **When** an event not allowed in the table arrives,
   **Then** the transition is rejected with reason code `INVALID_TRANSITION` and the state is
   unchanged.

---

### User Story 2 - "I could not fix it" is a first-class outcome (Priority: P1)

When the assistant cannot resolve the problem safely, the ticket moves to `unresolved` with the
specific reason (REP-01…REP-08), so the app can generate the report for the person.

**Why this priority**: honesty about failure is a core product promise (constitution III).

**Independent Test**: transition to `unresolved` requires a report trigger; check that the
trigger is recorded and that the same family reopened twice within 7 days produces REP-08.

**Acceptance Scenarios**:

1. **Given** an `in_progress` ticket, **When** event `cannot_resolve(REP-03)` arrives, **Then**
   the ticket is `unresolved` with trigger `REP-03`.
2. **Given** a ticket of family `connectivity` reopened twice within 7 days, **When** the
   repeated-reopen rule is evaluated, **Then** it yields `REP-08`.
3. **Given** an `unresolved` ticket, **When** the person tries again, **Then** it is `reopened`.

---

### User Story 3 - Diagnostic sessions follow fixed phases (Priority: P2)

Each attempt to diagnose is a session that moves through
`triage → probing → planning → awaiting_approval → executing → verifying → done`, and can end in
`failed` or `aborted` at any non-terminal phase.

**Why this priority**: the orchestrator and the UI depend on a predictable phase model; the
approval window is only meaningful in `awaiting_approval`.

**Independent Test**: exhaustive (phase, event) table test.

**Acceptance Scenarios**:

1. **Given** a session in `planning`, **When** a plan needs approval, **Then** the phase is
   `awaiting_approval`; **When** no step needs approval, **Then** the phase is `executing`.
2. **Given** a session in `awaiting_approval`, **When** the person refuses, **Then** the session
   is `done` without execution.
3. **Given** a session in any non-terminal phase, **When** the person stops it, **Then** it is
   `aborted`.

---

### User Story 4 - Time-based housekeeping (Priority: P3)

Tickets waiting for the person are abandoned after 14 days; resolved tickets close after 7 days;
unresolved tickets close after 30 days; abandoned tickets close.

**Independent Test**: pure function `due_event(ticket, now)` with crafted timestamps.

**Acceptance Scenarios**:

1. **Given** an `awaiting_user` ticket idle for 14 days, **When** housekeeping runs, **Then** the
   due event is `abandon`.
2. **Given** a `resolved` ticket resolved 6 days ago, **When** housekeeping runs, **Then** no
   event is due.
3. **Given** a `resolved_pending_confirmation` ticket idle for 14 days, **When** housekeeping
   runs, **Then** the due event is `abandon` (never an automatic `resolved`).

### Edge Cases

- Terminal states (`cancelled`, `closed`) accept no events.
- Clock going backwards (`now` earlier than the last change): no time-based event is due.
- Reopen count saturates instead of overflowing.
- Cancelling while a session is executing: the ticket can be cancelled; the session decides
  separately whether the running step finishes (out of scope here, handled by the executor spec).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The domain MUST define the ticket states `open`, `in_progress`, `awaiting_user`,
  `resolved_pending_confirmation`, `resolved`, `unresolved`, `reopened`, `cancelled`,
  `abandoned`, `closed`.
- **FR-002**: Ticket transitions MUST follow exactly the table in `plan.md` (Plan v1 VII.2 plus two
  documented additions: `reopened` can be cancelled, and `resolved_pending_confirmation` is
  abandoned after the idle timeout); any other (state, event) pair MUST be rejected with
  `INVALID_TRANSITION`.
- **FR-003**: `resolved` MUST only be reachable from `resolved_pending_confirmation` by explicit
  person confirmation.
- **FR-004**: Entering `unresolved` MUST carry a report trigger REP-01…REP-08, recorded on the
  ticket.
- **FR-005**: Reopening (`resolved` or `unresolved` → `reopened`) MUST increment a saturating
  reopen counter.
- **FR-006**: The domain MUST expose the repeated-reopen rule: given the reopen timestamps of the
  same family, two or more reopens within 7 days yield REP-08.
- **FR-007**: The domain MUST define session phases and their transitions as in User Story 3;
  invalid pairs MUST be rejected with `INVALID_TRANSITION`.
- **FR-008**: The domain MUST provide `due_event(ticket, now)` implementing User Story 4 with
  configurable durations (defaults 14/7/30 days).
- **FR-009**: Every rejection MUST expose a stable machine-readable reason code string.

### Non-Functional Requirements

- **NFR-001**: The crate MUST be pure: no I/O, no clock access, no randomness, no global state.
- **NFR-002**: The crate MUST have no runtime dependencies outside the Rust standard library.
- **NFR-003**: Production code MUST NOT use `unwrap`, `expect` or `panic!` (lint-enforced).

### Key Entities

- **Ticket**: id (opaque string), family, state, reopen count, last report trigger,
  timestamps of last state change.
- **TicketEvent**: input that requests a state change.
- **SessionPhase / SessionEvent**: phases and inputs of one diagnostic attempt.
- **ReportTrigger**: REP-01…REP-08 (Plan v1 VII.2).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of (state, event) pairs for tickets and sessions are covered by an exhaustive
  test that compares against the allowed table.
- **SC-002**: `cargo clippy -- -D warnings` and `cargo test` pass on Linux and Windows in CI.
- **SC-003**: Zero runtime dependencies in `hmd-domain` (`cargo tree` shows only the crate).

## Assumptions

- Durations are defaults from the plan and will be configurable by the application layer.
- Persistence (SQLite) and the orchestrator are separate features; this crate only decides.
- Timestamps are Unix seconds (`i64`) supplied by the caller.
