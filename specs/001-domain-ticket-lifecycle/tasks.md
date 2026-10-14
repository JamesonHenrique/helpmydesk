# Tasks: Domain core — ticket lifecycle, session phases and report triggers

**Input**: [spec.md](spec.md), [plan.md](plan.md)

## Phase 1: Setup

- [x] T001 Create Cargo workspace with shared lints denying `unwrap_used`, `expect_used`, `panic` (NFR-003) in `Cargo.toml`
- [x] T002 Pin toolchain in `rust-toolchain.toml` (rustfmt, clippy)
- [x] T003 Create crate `crates/hmd-domain` with zero dependencies (NFR-001, NFR-002)

## Phase 2: Foundational

- [x] T004 [P] `DomainError` with stable reason codes in `crates/hmd-domain/src/error.rs` (FR-009)
- [x] T005 [P] `ProblemFamily` in `crates/hmd-domain/src/family.rs`
- [x] T006 [P] `ReportTrigger` REP-01..08 with codes in `crates/hmd-domain/src/report.rs` (FR-004)

## Phase 3: User Story 1 — ticket lifecycle (P1)

- [x] T007 [US1] Ticket states/events and transition table in `crates/hmd-domain/src/ticket.rs` (FR-001, FR-002, FR-003)
- [x] T008 [US1] `Ticket::apply` returning a new ticket; reopen counter saturating (FR-005)
- [x] T009 [US1] Exhaustive (state, event) test against the plan table (SC-001)

## Phase 4: User Story 2 — unresolved and repeated reopen (P1)

- [x] T010 [US2] `cannot_resolve(trigger)` records the trigger (FR-004)
- [x] T011 [US2] `repeated_reopen_trigger` rule (FR-006) with tests

## Phase 5: User Story 3 — session phases (P2)

- [x] T012 [US3] Session phases/events and table in `crates/hmd-domain/src/session.rs` (FR-007)
- [x] T013 [US3] Exhaustive session table test (SC-001)

## Phase 6: User Story 4 — housekeeping (P3)

- [x] T014 [US4] `due_event` with configurable durations in `crates/hmd-domain/src/housekeeping.rs` (FR-008)
- [x] T015 [US4] Tests incl. clock going backwards

## Phase 7: Polish

- [x] T016 CI workflow running fmt, clippy and tests on `ubuntu-latest` and `windows-latest` (SC-002)
- [x] T017 Verify zero runtime dependencies with `cargo tree` (SC-003)
