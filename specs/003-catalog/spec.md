# Feature Specification: Capability catalog (format, loading, validation, embedding)

**Feature Branch**: `003-catalog`
**Created**: 2026-10-14
**Status**: Approved (owner approval, 2026-10-14)
**Input**: Plan v1 V.5, VII.3, ADR-017, ADR-021

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Contributors describe capabilities in reviewable files (Priority: P1)

A contributor adds a capability as one human-readable file in `catalog/capabilities/`, with every
safety property per platform and the plain-language texts. Mistakes are caught by CI, not by users.

**Independent Test**: load the real `catalog/` directory in a test; any invalid file fails CI with
a precise error.

**Acceptance Scenarios**:

1. **Given** a file with an unknown field, **When** loaded, **Then** error `UNKNOWN_FIELD`
   (strict parsing, no silent ignore).
2. **Given** a file whose `id` differs from its file name, **Then** `ID_FILENAME_MISMATCH`.
3. **Given** a fix declared below its computed minimum level, **Then** `LEVEL_BELOW_MINIMUM`
   (reusing `hmd_policy::validate_catalog`).
4. **Given** a precondition that names a finding no probe declares in `produces`, **Then**
   `UNKNOWN_FINDING`.
5. **Given** a verification that names a capability that is not a probe, **Then**
   `INVALID_VERIFICATION`.
6. **Given** `exclusive_with` naming a missing capability, **Then** `UNKNOWN_REFERENCE`.
7. **Given** a fix or guide without pt-BR `title`, `risk` and `undo` texts, **Then** `MISSING_COPY`.

### User Story 2 - The app ships exactly the reviewed catalog (Priority: P1)

The catalog is embedded in the binary at build time (no runtime file or download), so the
Authenticode/updater signature of the binary covers it (ADR-017).

**Acceptance Scenarios**:

1. **Given** the build, **When** `embedded()` is called, **Then** it returns the same catalog as
   loading `catalog/` from disk, already validated.
2. **Given** an invalid catalog in the repository, **Then** the build's test suite fails.

### Edge Cases

- Duplicate IDs across files → `DUPLICATE_ID`.
- Unknown enum values (e.g. `risk_level = "N9"`) → `PARSE_ERROR` with file name.
- Probes may have no copy texts; fixes and guides must.

## Requirements *(mandatory)*

- **FR-001**: Catalog files MUST be TOML, one capability per file, parsed with unknown fields denied.
- **FR-002**: Each file MUST map to `hmd_policy::Capability` plus `produces` (finding codes, probes
  only) and `copy` (texts per locale).
- **FR-003**: Loading MUST run `hmd_policy::validate_catalog` and the cross-reference checks of
  User Story 1 and return all errors with file names.
- **FR-004**: `catalog/catalog.toml` MUST hold the catalog version (`AAAA.MM.patch`).
- **FR-005**: The crate MUST expose `embedded()` built from files included at compile time.
- **FR-006**: The real repository catalog MUST be loaded and validated by a test in CI.
- **NFR-001**: Dependencies limited to `serde` and `toml` (MIT/Apache-2.0), justified in ADR-021.

## Success Criteria *(mandatory)*

- **SC-001**: Each error code of User Story 1 and the edge cases has a failing-fixture test.
- **SC-002**: CI green on Linux and Windows with the real catalog.

## Assumptions

- Kill switch (remote, signed) is a separate spec.
- Only pt-BR copy is required in the MVP.
