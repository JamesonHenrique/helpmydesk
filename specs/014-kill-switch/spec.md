# Feature Specification: Signed kill switch

**Feature Branch**: `014-kill-switch`
**Created**: 2026-10-15
**Status**: Approved (owner approval, 2026-10-14: "aprovo tudo")
**Input**: Plan v1 VII.3 (`disabled-capabilities.json`), VII.4 (`KillSwitchState`), IX.2, SPEC-014 (part)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Maintainers can turn a capability off without a release (Priority: P1)

1. **Given** a file signed by a trusted key, newer than the last one and not expired, **Then** it
   is accepted, kept in the history (audited) and its capabilities are neither run nor proposed.
2. **Given** a restart without internet, **Then** the last accepted file still applies.

### User Story 2 - Nobody else can (Priority: P1)

1. **Given** no trusted key in the build, **Then** every file is refused.
2. **Given** a wrong key, a changed byte, a bad signature, an unknown format, extra fields or a
   file over 64 KiB, **Then** it is refused before anything in it is used.
3. **Given** a file not newer than the last accepted (replay), already expired, or issued more than
   a day in the future, **Then** it is refused.

## Requirements *(mandatory)*

- **FR-001**: Pure crate `hmd-killswitch` (`verify`, `accept`); Ed25519 over the exact bytes,
  signature as hex.
- **FR-002**: `DiagnoseOptions::disabled` feeds every Policy Engine evaluation (probes, fixes,
  verification probes).
- **FR-003**: Store schema v5 `kill_switch_state`; `App::apply_kill_switch`; re-verification at
  load.
- **FR-004**: Maintainer procedure with OpenSSL (`docs/ops/kill-switch.md`) and an example
  `verify`.
- **NFR-001**: `ed25519-dalek` 3.0.0, `serde_json` 1.0.151 (ADR-026).

## Success Criteria *(mandatory)*

- **SC-001**: Each scenario has a test; the OpenSSL procedure was run end to end once.

## Assumptions

- Downloading from GitHub Releases and disabling AI models come with the updater and the AI client.
