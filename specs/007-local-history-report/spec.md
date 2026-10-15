# Feature Specification: Local history and "não consegui" report

**Feature Branch**: `007-local-history-report`
**Created**: 2026-10-14
**Status**: Approved (owner approval, 2026-10-14)
**Input**: Plan v1 VII.2 (lifecycle, REP triggers), VII.3 (report contract), VII.4 (local data model, retention)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The app remembers problems on this computer only (Priority: P1)

Tickets, their state changes, diagnosis sessions with findings, consents and reports are kept in a
local SQLite file. Nothing leaves the computer.

**Independent Test**: the store runs against temporary files; no network.

**Acceptance Scenarios**:

1. **Given** a new ticket, **When** it goes through `start_session` and `cannot_resolve(REP-01)`,
   **Then** the stored ticket is `unresolved` with trigger REP-01 and both events are in its history.
2. **Given** an invalid transition (e.g. `closed` + `start_session`), **Then** the store returns the
   domain error and the stored ticket is unchanged.
3. **Given** a finished diagnosis, **When** the session is recorded, **Then** playbook@version,
   diagnosis, confidence and each finding (code, severity, probe@version) are stored.
4. **Given** a family reopened twice within 7 days, **Then** the stored reopen times make
   `repeated_reopen_trigger` return REP-08.
5. **Given** the file is not a valid database, **When** it is opened, **Then** it is renamed aside
   (`*.corrupt-<unix-seconds>`), a new empty database is created and the caller is told.

### User Story 2 - A person can take an honest report to someone (Priority: P1)

When the app cannot resolve (any REP trigger, including REP-06 "a pessoa pediu"), it generates a
pt-BR Markdown report the person can read, edit and save.

**Acceptance Scenarios**:

1. **Given** a diagnosis, **Then** the report has the header `report@1` and the fixed sections
   Sintoma, O que verifiquei, Diagnóstico, O que já foi tentado, Sistema, Para quem for ajudar.
2. **Given** no fix was executed (execution is a later spec), **Then** "O que já foi tentado" says
   plainly that the HelpMyDesk changed nothing.
3. **Given** a symptom text with the user or computer name, **Then** the report has
   `[usuário]`/`[computador]` instead.
4. **Given** any diagnosis, **Then** the report contains no process names, startup app names, IPs
   or other probe evidence text — only finding codes, verdicts and the OS summary.
5. **Given** the person edited the report, **Then** the edited text is stored; `exported_at` is set
   only when the file is saved.

### User Story 3 - Old data goes away by itself (Priority: P2)

1. **Given** a `closed` or `cancelled` ticket last changed 180+ days ago, **When** retention runs,
   **Then** the ticket and everything attached (events, sessions, findings, messages, reports) are deleted.
2. **Given** a ticket message created 90+ days ago, **Then** it is deleted even if its ticket stays.
3. Consent records are kept (append-only history of what the person allowed).

### Edge Cases

- Stored codes that this version does not know (state, family, trigger) are an error, never a panic.
- Messages are sanitized and identity-redacted before storage and limited to 4000 characters.
- The store never reads the clock: every operation takes `now` (spec 001 NFR-001).
- A newer database (`user_version` above what this build knows) is not touched (`SCHEMA_TOO_NEW`).

## Requirements *(mandatory)*

- **FR-001**: Ticket writes MUST go only through `Ticket::apply`; the update is conditional on the
  previously read state.
- **FR-002**: Every applied event MUST be appended to the ticket history.
- **FR-003**: The schema MUST be versioned with `PRAGMA user_version`; migrations run at open, with
  a copy of the file made first (`*.bak-v<old>`) when an existing database is migrated.
- **FR-004**: SQLite MUST run with WAL and foreign keys on; child rows cascade on ticket deletion.
- **FR-005**: The report MUST be a pure function of the diagnosis, the trigger, the redacted
  symptom and the OS summary (`report@1`).
- **FR-006**: The OS summary MUST come from `sys.info.basic` authorized by the Policy Engine.
- **FR-007**: Retention decisions MUST be pure functions in `hmd-domain` (defaults 180/90 days).
- **NFR-001**: New dependency `rusqlite` with bundled SQLite, justified in ADR-023.

## Success Criteria *(mandatory)*

- **SC-001**: Each acceptance scenario has a test.
- **SC-002**: CI generates a real report on Linux and Windows and checks the user and host names
  are absent.

## Assumptions

- IDs are SQLite integer row IDs, not ULIDs (deviation from VII.4): the data never leaves the
  computer or merges with other databases, so global uniqueness buys nothing now.
- ActionPlan, ActionApproval, ActionExecution, AuditEvent, AiCallLog and KillSwitchState arrive
  with the specs that write them, as new migrations.
- `cancelled` never becomes `closed`; its retention counts from its last change, like `closed`.
- `hmd-store` needs a C compiler: MSVC on CI Windows, MinGW for local `x86_64-pc-windows-gnu`
  checks (installed on the dev machine 2026-10-15).
