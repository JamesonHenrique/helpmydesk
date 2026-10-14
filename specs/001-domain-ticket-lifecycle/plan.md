# Implementation Plan: Domain core — ticket lifecycle, session phases and report triggers

**Branch**: `001-domain-ticket-lifecycle` | **Date**: 2026-10-14 | **Spec**: [spec.md](spec.md)

## Summary

A pure Rust crate `hmd-domain` with: ticket states and an explicit transition table, diagnostic
session phases, report triggers REP-01…REP-08, the repeated-reopen rule and time-based
housekeeping. No I/O, no dependencies.

## Technical Context

**Language/Version**: Rust (stable toolchain pinned in `rust-toolchain.toml`), edition 2024
**Primary Dependencies**: none (std only)
**Storage**: N/A (persistence is a later feature)
**Testing**: `cargo test` (exhaustive table tests); `cargo clippy -D warnings`
**Target Platform**: Windows x64 and Linux x64 (pure code; CI on both)
**Project Type**: library crate inside a Cargo workspace (`crates/hmd-domain`)
**Performance Goals**: N/A (constant-time table lookups)
**Constraints**: no `unwrap`/`expect`/`panic!` in production code (workspace lints)
**Scale/Scope**: ~10 ticket states, ~11 ticket events, 9 session phases

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I. AI proposes, code decides | Domain decides transitions deterministically; no model input | PASS |
| II. Least privilege | No privileged code | PASS (N/A) |
| III. Proof of resolution | `resolved` only via person confirmation after verification (FR-003) | PASS |
| IV. Privacy | No personal data in the crate (ticket id is opaque; text lives elsewhere) | PASS |
| V. No AI outside session | No network | PASS (N/A) |
| VI. Tests and traceability | Tests cite FR IDs; exhaustive tables (SC-001) | PASS |
| VII. Simplicity | Zero dependencies | PASS |
| Security constraints | `unwrap_used`/`expect_used`/`panic` denied via workspace lints | PASS |

## Ticket transition table (authoritative)

| From \ Event | start_session | need_user_input | user_responded | verification_passed | user_confirmed | user_not_fixed | cannot_resolve(REP) | reopen | cancel | abandon | close |
|---|---|---|---|---|---|---|---|---|---|---|---|
| open | in_progress | | | | | | | | cancelled | | |
| in_progress | | awaiting_user | | resolved_pending_confirmation | | | unresolved | | cancelled | | |
| awaiting_user | | | in_progress | | | | | | cancelled | abandoned | |
| resolved_pending_confirmation | | | | | resolved | in_progress | | | | abandoned¹ | |
| resolved | | | | | | | | reopened | | | closed |
| unresolved | | | | | | | | reopened | | | closed |
| reopened | in_progress | | | | | | | | cancelled¹ | | |
| abandoned | | | | | | | | | | | closed |
| cancelled | — terminal — |
| closed | — terminal — |

¹ Documented additions to Plan v1 VII.2 (see spec FR-002).

## Session phase table (authoritative)

| From \ Event | triage_completed | probes_completed | plan_requires_approval | plan_ready | no_actionable_plan | approved | refused | execution_finished | verification_finished | fail | stop |
|---|---|---|---|---|---|---|---|---|---|---|---|
| triage | probing | | | | | | | | | failed | aborted |
| probing | | planning | | | | | | | | failed | aborted |
| planning | | | awaiting_approval | executing | done | | | | | failed | aborted |
| awaiting_approval | | | | | | executing | done | | | failed | aborted |
| executing | | | | | | | | verifying | | failed | aborted |
| verifying | | | | | | | | | done | failed | aborted |
| done / failed / aborted | — terminal — |

## Housekeeping (`due_event`)

| State | Condition (`now - last_changed_at`) | Due event |
|---|---|---|
| awaiting_user | ≥ 14 days | abandon |
| resolved_pending_confirmation | ≥ 14 days | abandon |
| resolved | ≥ 7 days | close |
| unresolved | ≥ 30 days | close |
| abandoned | always | close |
| any | `now < last_changed_at` | none |

## Project Structure

```text
Cargo.toml                 # workspace, shared lints
rust-toolchain.toml
crates/hmd-domain/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── error.rs           # DomainError + reason codes (FR-009)
    ├── family.rs          # ProblemFamily
    ├── report.rs          # ReportTrigger REP-01..08, repeated-reopen rule (FR-006)
    ├── ticket.rs          # states, events, transition table, Ticket (FR-001..005)
    ├── session.rs         # phases, events, table (FR-007)
    └── housekeeping.rs    # due_event (FR-008)
```

## Complexity Tracking

No violations.
