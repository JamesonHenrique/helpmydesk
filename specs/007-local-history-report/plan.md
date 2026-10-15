# Implementation Plan: Local history and "não consegui" report

**Branch**: `007-local-history-report` | **Date**: 2026-10-14 | **Spec**: [spec.md](spec.md)

## Technical Context

- `hmd-domain`: `from_code` parsers (state, family, trigger, phase) and `retention.rs` (pure).
- `hmd-core::report`: pure `render` (`report@1` Markdown) + `system_summary_with` (policy-authorized
  `sys.info.basic`) + `redact_free_text`.
- New leaf crate `crates/hmd-store` (`rusqlite`, feature `bundled`): schema v1, migrations,
  corruption recovery, tickets/events/sessions/findings/messages/consents/reports, retention purge.
  No other crate depends on it, so local Windows checks can exclude it.
- `basic_mode` gains `--relatorio "<sintoma>"` to print the report.

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| II | Local only; report leaves the computer only if the person sends it | PASS |
| IV | Redaction before storage and report; no evidence text in report | PASS |
| VII | One new dependency (`rusqlite`), ADR-023 | PASS |

## Schema v1

`ticket`, `ticket_event`, `diagnostic_session`, `diagnostic_finding`, `ticket_message`,
`consent_record` (update/delete blocked by triggers), `report`. Text codes are the domain `code()`
values; timestamps are Unix seconds.
