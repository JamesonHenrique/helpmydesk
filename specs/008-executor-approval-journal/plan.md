# Implementation Plan: Executor, plan-hash approval and execution journal

**Branch**: `008-executor-approval-journal` | **Date**: 2026-10-15 | **Spec**: [spec.md](spec.md)

## Technical Context

- `hmd-domain::execution`: `ExecutionState`, pure `transition`.
- New crate `crates/hmd-executor` (`sha2`): canonical encoding, `plan_hash`, `idempotency_key`,
  approval checks, `FixImpl` trait + registry (empty in production), `execute_step`, `verify_step`.
- `hmd-store`: migration v2 and journal API (`save_plan`, `approve`, `begin`, `finish`,
  `recover_interrupted`, `unknown_capabilities` for the Policy Engine counters).
- The executor writes through a small `Journal` trait implemented by `hmd-store`, so the
  executor stays independent of SQLite.

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I | Only accepted, approved catalog steps run | PASS |
| II | No elevation and no system-changing implementation in this spec | PASS |
| III | `unknown` never re-runs; verification by probe; REP-03/04 | PASS |
| VII | One new dependency (`sha2`), ADR-024 | PASS |
