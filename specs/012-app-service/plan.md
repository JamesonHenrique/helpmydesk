# Implementation Plan: Application service

**Branch**: `012-app-service` | **Date**: 2026-10-15 | **Spec**: [spec.md](spec.md)

## Technical Context

`crates/hmd-app` (leaf; depends on store, executor, core, catalog, policy, probes, domain).
`App::{open, start_ticket, diagnose, approve, decline, execute, confirm, make_report,
edit_report, report_exported, set_autonomy, set_consent, export_all, erase_all, history}`.
Store schema v4 (`setting`). `DiagnoseOptions { session_ref, unknown_executions }` and
`DiagnosisReport::fix_proposal` in `hmd-core`.

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I | Only policy-accepted, approved catalog steps run; proof by probe | PASS |
| III | REP-03/04/08; resolved only by the person | PASS |
| VII | No new dependency | PASS |
