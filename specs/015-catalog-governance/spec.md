# Feature Specification: Catalog contribution governance

**Feature Branch**: `015-catalog-governance`
**Created**: 2026-10-15
**Status**: Approved (owner approval, 2026-10-14: "aprovo tudo")
**Input**: Plan v1 SPEC-016, ADR-019, TH-12 (malicious contribution), gates G-SEC/G-AUTO

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A contributor knows what a good capability card is (Priority: P1)

1. **Given** someone wants to add a check or a fix, **Then** `docs/contributing/catalog.md` says
   what CI checks automatically, what only review checks, what is always blocked (N4) and how the
   G-SEC/G-AUTO gates work; the issue and PR templates ask for the same things.

### User Story 2 - Sensitive paths need the maintainer (Priority: P1)

1. **Given** a change to the catalog, policy, executor, probes, kill switch, window permissions,
   CSP, dependency policy or workflows, **Then** CODEOWNERS requires the maintainer.

### User Story 3 - No fix runs without lab evidence (Priority: P1)

1. **Given** the production executor registry, **Then** a test fails if it contains a fix that is
   not in the reviewed release list (G-SEC), or the reverse.

## Requirements *(mandatory)*

- **FR-001**: Guide, issue template and PR template aligned with the automatic catalog checks.
- **FR-002**: CODEOWNERS covers every protected path listed in the guide.
- **FR-003**: `crates/hmd-executor/tests/release_gate.rs` ties `Registry::production()` to the
  release list.

## Success Criteria *(mandatory)*

- **SC-001**: The release-gate test runs in CI.

## Assumptions

- CODEOWNERS is enforced only with branch protection, which is a GitHub setting the owner decides
  (open P2). Semgrep rules and manual CI approval for first-time contributors also depend on
  repository settings.
