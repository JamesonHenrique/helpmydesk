// SPDX-License-Identifier: GPL-3.0-or-later
//! SQL schema, one entry per `user_version` (spec 007 FR-003). Never edit a released entry:
//! changes are new migrations.

pub const MIGRATIONS: &[&str] = &[V1, V2, V3, V4, V5];

/// Schema version this build writes.
pub const SCHEMA_VERSION: i32 = 5;

#[doc(hidden)]
pub const V1: &str = r#"
CREATE TABLE ticket (
    id              INTEGER PRIMARY KEY,
    family          TEXT    NOT NULL,
    state           TEXT    NOT NULL,
    reopen_count    INTEGER NOT NULL DEFAULT 0,
    last_trigger    TEXT,
    created_at      INTEGER NOT NULL,
    last_changed_at INTEGER NOT NULL
) STRICT;

CREATE TABLE ticket_event (
    id         INTEGER PRIMARY KEY,
    ticket_id  INTEGER NOT NULL REFERENCES ticket(id) ON DELETE CASCADE,
    event      TEXT    NOT NULL,
    trigger    TEXT,
    from_state TEXT    NOT NULL,
    to_state   TEXT    NOT NULL,
    at         INTEGER NOT NULL
) STRICT;
CREATE INDEX ticket_event_by_ticket ON ticket_event(ticket_id);

CREATE TABLE diagnostic_session (
    id              INTEGER PRIMARY KEY,
    ticket_id       INTEGER NOT NULL REFERENCES ticket(id) ON DELETE CASCADE,
    phase           TEXT    NOT NULL,
    playbook        TEXT    NOT NULL,
    diagnosis_id    TEXT    NOT NULL,
    confidence      TEXT    NOT NULL CHECK (confidence IN ('unsure', 'probable', 'almost_certain')),
    ai_mode         TEXT    NOT NULL DEFAULT 'none' CHECK (ai_mode IN ('none', 'opencode')),
    platform        TEXT    NOT NULL CHECK (platform IN ('windows', 'linux')),
    catalog_version TEXT    NOT NULL,
    started_at      INTEGER NOT NULL,
    ended_at        INTEGER
) STRICT;
CREATE INDEX diagnostic_session_by_ticket ON diagnostic_session(ticket_id);

CREATE TABLE diagnostic_finding (
    id           INTEGER PRIMARY KEY,
    session_id   INTEGER NOT NULL REFERENCES diagnostic_session(id) ON DELETE CASCADE,
    code         TEXT    NOT NULL,
    severity     TEXT    NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),
    source_probe TEXT    NOT NULL
) STRICT;
CREATE INDEX diagnostic_finding_by_session ON diagnostic_finding(session_id);

CREATE TABLE ticket_message (
    id            INTEGER PRIMARY KEY,
    ticket_id     INTEGER NOT NULL REFERENCES ticket(id) ON DELETE CASCADE,
    author_type   TEXT    NOT NULL CHECK (author_type IN ('person', 'assistant', 'system')),
    body_redacted TEXT    NOT NULL CHECK (length(body_redacted) <= 4000),
    created_at    INTEGER NOT NULL
) STRICT;
CREATE INDEX ticket_message_by_ticket ON ticket_message(ticket_id);

CREATE TABLE consent_record (
    id             INTEGER PRIMARY KEY,
    scope          TEXT    NOT NULL,
    granted        INTEGER NOT NULL CHECK (granted IN (0, 1)),
    notice_version TEXT    NOT NULL,
    at             INTEGER NOT NULL
) STRICT;
CREATE TRIGGER consent_record_no_update BEFORE UPDATE ON consent_record
BEGIN SELECT RAISE(ABORT, 'consent_record is append-only'); END;
CREATE TRIGGER consent_record_no_delete BEFORE DELETE ON consent_record
BEGIN SELECT RAISE(ABORT, 'consent_record is append-only'); END;

CREATE TABLE report (
    id          INTEGER PRIMARY KEY,
    ticket_id   INTEGER NOT NULL REFERENCES ticket(id) ON DELETE CASCADE,
    trigger     TEXT    NOT NULL,
    content     TEXT    NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    exported_at INTEGER
) STRICT;
CREATE INDEX report_by_ticket ON report(ticket_id);
"#;

/// Spec 008: plans, approvals and the write-ahead execution journal.
#[doc(hidden)]
pub const V2: &str = r#"
CREATE TABLE action_plan (
    id              INTEGER PRIMARY KEY,
    ticket_id       INTEGER NOT NULL REFERENCES ticket(id) ON DELETE CASCADE,
    session_ref     TEXT    NOT NULL,
    plan_hash       TEXT    NOT NULL UNIQUE,
    catalog_version TEXT    NOT NULL,
    canonical       TEXT    NOT NULL,
    created_at      INTEGER NOT NULL
) STRICT;
CREATE INDEX action_plan_by_ticket ON action_plan(ticket_id);

-- The row id is the approval nonce (unique by construction).
CREATE TABLE action_approval (
    id         INTEGER PRIMARY KEY,
    plan_id    INTEGER NOT NULL REFERENCES action_plan(id) ON DELETE CASCADE,
    plan_hash  TEXT    NOT NULL,
    step_ids   TEXT    NOT NULL,
    mode       TEXT    NOT NULL CHECK (mode IN ('observe_only', 'ask_always', 'auto_simple')),
    decided_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
) STRICT;
CREATE INDEX action_approval_by_plan ON action_approval(plan_id);

CREATE TABLE action_execution (
    id              INTEGER PRIMARY KEY,
    plan_id         INTEGER NOT NULL REFERENCES action_plan(id) ON DELETE CASCADE,
    approval_id     INTEGER NOT NULL REFERENCES action_approval(id) ON DELETE CASCADE,
    step_id         TEXT    NOT NULL,
    capability      TEXT    NOT NULL,
    platform        TEXT    NOT NULL CHECK (platform IN ('windows', 'linux')),
    idempotency_key TEXT    NOT NULL UNIQUE,
    state           TEXT    NOT NULL,
    started_at      INTEGER NOT NULL,
    finished_at     INTEGER,
    result          TEXT,
    verification    TEXT,
    rollback_state  TEXT
) STRICT;
CREATE INDEX action_execution_by_plan ON action_execution(plan_id);
CREATE INDEX action_execution_by_state ON action_execution(state);
"#;

/// Spec 009: local audit trail with a hash chain (references only, no personal content).
#[doc(hidden)]
pub const V3: &str = r#"
CREATE TABLE audit_event (
    id          INTEGER PRIMARY KEY,
    occurred_at INTEGER NOT NULL,
    actor       TEXT    NOT NULL CHECK (actor IN ('user', 'agent', 'system')),
    action      TEXT    NOT NULL,
    subject_ref TEXT    NOT NULL,
    outcome     TEXT    NOT NULL,
    reason_code TEXT,
    prev_hash   TEXT    NOT NULL,
    hash        TEXT    NOT NULL UNIQUE
) STRICT;
CREATE TRIGGER audit_event_no_update BEFORE UPDATE ON audit_event
BEGIN SELECT RAISE(ABORT, 'audit_event is append-only'); END;
"#;

/// Spec 012: the person's settings (e.g. autonomy mode). Values are codes, never free text.
const V4: &str = r#"
CREATE TABLE setting (
    key        TEXT    PRIMARY KEY,
    value      TEXT    NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;
"#;

/// Spec 014: the last accepted kill switch (one row), kept so it applies offline.
const V5: &str = r#"
CREATE TABLE kill_switch_state (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    issued_at   INTEGER NOT NULL,
    payload     BLOB    NOT NULL,
    signature   TEXT    NOT NULL,
    accepted_at INTEGER NOT NULL
) STRICT;
"#;
