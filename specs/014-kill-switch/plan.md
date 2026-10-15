# Implementation Plan: Signed kill switch

**Branch**: `014-kill-switch` | **Date**: 2026-10-15 | **Spec**: [spec.md](spec.md)

`crates/hmd-killswitch` (pure) → `hmd-core::DiagnoseOptions::disabled` → `hmd-store` v5 →
`hmd-app::{apply_kill_switch, with_trusted_keys, disabled_capabilities}`.

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I | Disables only narrow what runs; nothing can be enabled by the file | PASS |
| II | No key material in the repository | PASS |
| VII | ADR-026 | PASS |
