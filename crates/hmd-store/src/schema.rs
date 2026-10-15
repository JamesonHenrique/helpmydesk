// SPDX-License-Identifier: GPL-3.0-or-later
//! SQL schema, one entry per `user_version` (spec 007 FR-003). Never edit a released entry:
//! changes are new migrations.

pub(crate) const MIGRATIONS: &[&str] = &[V1];

/// Schema version this build writes.
pub const SCHEMA_VERSION: i32 = 1;

const V1: &str = r#"
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
