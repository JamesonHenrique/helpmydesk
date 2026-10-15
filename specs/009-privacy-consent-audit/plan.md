# Implementation Plan: Privacy disclosure, consent, audit trail, export and erase

**Branch**: `009-privacy-consent-audit` | **Date**: 2026-10-15 | **Spec**: [spec.md](spec.md)

## Technical Context

- `hmd-catalog`: `looks_at`/`never` in probe copy; `[consents.<id>]` in `catalog.toml`;
  `LoadedCatalog::disclosures` and `consents`; codes `PROBE_MISSING_DISCLOSURE`,
  `UNKNOWN_CONSENT`, `CONSENT_MISSING_COPY`.
- `hmd-domain::retention`: `audit_after` (180 days) and `audit_expired`.
- `hmd-store`: schema v3 `audit_event`; `audit` module (canonical text, chain, verify, prefix
  purge); audit appends inside the existing transactions; `set_consent`; `export_markdown`;
  `erase_all`.
- No new dependency (`sha2` comes through `hmd-executor`).

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| IV | Collection declared per probe; consent informed and versioned | PASS |
| II | Export and erase are local; nothing is sent | PASS |
| VII | No new dependency | PASS |
