# Implementation Plan: Capability catalog

**Branch**: `003-catalog` | **Date**: 2026-10-14 | **Spec**: [spec.md](spec.md)

## Technical Context

Rust 1.99 · crate `crates/hmd-catalog` · deps: `serde` 1.0, `toml` 1.1 (ADR-021), `hmd-policy`,
`hmd-domain` · `build.rs` generates `include_str!` entries for `catalog/` (FR-005).

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I | Safety properties only in reviewed files; strict parsing (`deny_unknown_fields`) | PASS |
| III | `FIX_WITHOUT_VERIFICATION`, `INVALID_VERIFICATION` enforced | PASS |
| VII | Two small, maintained dependencies, justified in ADR-021 | PASS (justified) |
| Protected path | `catalog/` is CODEOWNER-protected | Noted |

## Design

- `raw.rs`: on-disk TOML structs; `lib.rs`: conversion to `hmd_policy::Capability` +
  `EntryMeta { produces, copy }`; validation = `hmd_policy::validate_catalog` + cross-references.
- Errors carry the file name and a stable code: `PARSE_ERROR`, `UNKNOWN_FIELD`,
  `ID_FILENAME_MISMATCH`, `DUPLICATE_ID`, `LEVEL_BELOW_MINIMUM`, `FIX_WITHOUT_VERIFICATION`,
  `NO_VARIANTS`, `UNKNOWN_FINDING`, `INVALID_VERIFICATION`, `UNKNOWN_REFERENCE`, `MISSING_COPY`,
  `INVALID_PRODUCES`, `INVALID_MANIFEST`, `IO_ERROR`.
- `embedded()` validates once (`OnceLock`); a test proves embedded == directory.
