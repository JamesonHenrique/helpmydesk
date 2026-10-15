# Feature Specification: Privacy disclosure, consent, audit trail, export and erase

**Feature Branch**: `009-privacy-consent-audit`
**Created**: 2026-10-15
**Status**: Approved (owner approval, 2026-10-14: "aprovo tudo")
**Input**: Plan v1 SPEC-011, IX.2 ("o que eu olho", exportar/apagar tudo), VII.3/VII.4 (`AuditEvent`, `ConsentRecord`), TH-16

## User Scenarios & Testing *(mandatory)*

### User Story 1 - "O que eu olho" (Priority: P1)

1. **Given** any probe in the catalog, **Then** its pt-BR copy states `looks_at` (what it reads)
   and `never` (what it never keeps or sends); a probe without them fails catalog validation
   (`PROBE_MISSING_DISCLOSURE`).
2. **Given** a platform, **Then** `disclosures(platform)` lists every probe available there,
   with the consent category it needs, if any.

### User Story 2 - Consent is informed and versioned (Priority: P1)

1. **Given** a consent category used by a probe, **Then** it is declared in `catalog.toml` with a
   version and a pt-BR title and text (`UNKNOWN_CONSENT`, `CONSENT_MISSING_COPY` otherwise).
2. **Given** the person grants or revokes a category, **Then** the record keeps the version of the
   notice they read; undeclared categories are refused.

### User Story 3 - Tampering with the history is detectable (Priority: P2)

1. **Given** ticket events, plan evaluation and approval, execution steps, report export, consent
   changes and retention, **Then** each appends an `AuditEvent` in the same transaction, holding
   only codes and references (`ticket:3`), never message or report text.
2. **Given** the chain `hash = sha256(prev_hash ‖ "\n" ‖ canonical(event))`, **When** an event in
   the middle is changed or removed, **Then** `verify_audit_chain` reports where it breaks.
3. **Given** retention (180 days), **Then** only an expired prefix of the chain is removed and the
   rest still verifies.

### User Story 4 - The person can see and erase everything (Priority: P1)

1. **Given** stored data, **Then** `export_markdown` lists consents, tickets with history,
   diagnoses, fixes, messages, reports and the audit trail with its integrity.
2. **Given** "apagar tudo", **Then** the database and every file the store made next to it
   (`-wal`, `-shm`, `.bak-v*`, `.corrupt-*`) are removed, and nothing else.

## Requirements *(mandatory)*

- **FR-001**: Disclosure and consent notices MUST be catalog data (reviewed like any catalog
  change), validated at load.
- **FR-002**: Audit appends MUST share the transaction of the change they record.
- **FR-003**: `audit_event` is append-only (update blocked by trigger); deletion only by retention
  (prefix) or "apagar tudo".
- **FR-004**: The text runner shows the disclosure (`basic_mode --o-que-eu-olho`).

## Success Criteria *(mandatory)*

- **SC-001**: Each acceptance scenario has a test.
- **SC-002**: Store schema v3 migrates from v1 and v2.

## Assumptions

- The screen "o que vai para a IA" comes with the AI client (SPEC-009 of the plan).
- The audit trail is best effort: the owner of the computer can delete the file; removing the
  newest events cannot be detected.
- Audit retention counts from the event, not from the ticket's closing (simpler, keeps the chain
  verifiable).
