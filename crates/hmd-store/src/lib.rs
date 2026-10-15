// SPDX-License-Identifier: GPL-3.0-or-later
//! Local history of HelpMyDesk (spec 007, Plan v1 VII.4): tickets and their events, diagnosis
//! sessions with findings, messages, consents and "não consegui" reports, in one SQLite file on
//! this computer. Nothing here talks to the network. Every operation takes `now`: the store never
//! reads the clock (spec 001 NFR-001).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod audit;
mod export;
mod journal;
#[doc(hidden)]
pub mod schema;

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use hmd_catalog::LoadedCatalog;
use hmd_core::{DiagnosisReport, ProbeOutcome};
use hmd_domain::{
    DomainError, ProblemFamily, ReportTrigger, RetentionConfig, SessionPhase, Ticket, TicketEvent,
    TicketState, Timestamp, message_expired, ticket_expired,
};
use hmd_policy::{Confidence, Platform};
use hmd_probes::Severity;
use rusqlite::{Connection, ErrorCode, OptionalExtension, Row, params};

pub use audit::{Actor, AuditCheck, AuditEvent};
pub use export::{EXPORT_FORMAT, erase_all};
pub use journal::StoredExecution;
pub use schema::SCHEMA_VERSION;

#[derive(Debug)]
pub enum StoreError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    Domain(DomainError),
    /// The file was written by a newer HelpMyDesk; it is left untouched.
    SchemaTooNew(i32),
    /// A stored code this build does not know (never a panic).
    UnknownCode {
        what: &'static str,
        code: String,
    },
    NotFound,
    /// The ticket changed between read and write.
    ConcurrentChange,
}

impl StoreError {
    pub fn reason_code(&self) -> &'static str {
        match self {
            StoreError::Sqlite(_) => "STORE_SQLITE",
            StoreError::Io(_) => "STORE_IO",
            StoreError::Domain(e) => e.reason_code(),
            StoreError::SchemaTooNew(_) => "SCHEMA_TOO_NEW",
            StoreError::UnknownCode { .. } => "UNKNOWN_CODE",
            StoreError::NotFound => "NOT_FOUND",
            StoreError::ConcurrentChange => "CONCURRENT_CHANGE",
        }
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Sqlite(e) => write!(f, "STORE_SQLITE: {e}"),
            StoreError::Io(e) => write!(f, "STORE_IO: {e}"),
            StoreError::Domain(e) => write!(f, "{}", e.reason_code()),
            StoreError::SchemaTooNew(v) => write!(f, "SCHEMA_TOO_NEW: {v} > {SCHEMA_VERSION}"),
            StoreError::UnknownCode { what, code } => write!(f, "UNKNOWN_CODE: {what} `{code}`"),
            StoreError::NotFound => write!(f, "NOT_FOUND"),
            StoreError::ConcurrentChange => write!(f, "CONCURRENT_CHANGE"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError::Sqlite(e)
    }
}

impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        StoreError::Io(e)
    }
}

impl From<DomainError> for StoreError {
    fn from(e: DomainError) -> Self {
        StoreError::Domain(e)
    }
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// What happened when the file was opened; the app tells the person about the last two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenOutcome {
    Created,
    Opened,
    /// FR-003: an older schema was migrated after copying the file to `backup`.
    Migrated {
        from: i32,
        backup: PathBuf,
    },
    /// US1.5: the file was not a usable database; it was moved to `moved_to` and a new one created.
    RecreatedAfterCorruption {
        moved_to: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Author {
    Person,
    Assistant,
    System,
}

impl Author {
    fn code(self) -> &'static str {
        match self {
            Author::Person => "person",
            Author::Assistant => "assistant",
            Author::System => "system",
        }
    }
}

/// One applied ticket event (FR-002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub event: String,
    pub trigger: Option<ReportTrigger>,
    pub from: TicketState,
    pub to: TicketState,
    pub at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredFinding {
    pub code: String,
    pub severity: String,
    pub source_probe: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSession {
    pub id: i64,
    pub phase: SessionPhase,
    pub playbook: String,
    pub diagnosis_id: String,
    pub confidence: String,
    pub platform: String,
    pub catalog_version: String,
    pub started_at: Timestamp,
    pub ended_at: Option<Timestamp>,
    pub findings: Vec<StoredFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredMessage {
    pub author: String,
    pub body: String,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredReport {
    pub id: i64,
    pub ticket_id: String,
    pub trigger: ReportTrigger,
    pub content: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub exported_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PurgeStats {
    pub tickets: usize,
    pub messages: usize,
    pub audit_events: usize,
}

fn confidence_code(c: Confidence) -> &'static str {
    match c {
        Confidence::Unsure => "unsure",
        Confidence::Probable => "probable",
        Confidence::AlmostCertain => "almost_certain",
    }
}

fn platform_code(p: Platform) -> &'static str {
    match p {
        Platform::Windows => "windows",
        Platform::Linux => "linux",
    }
}

fn severity_code(s: Severity) -> &'static str {
    match s {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Critical => "critical",
    }
}

fn parse<T>(what: &'static str, code: String, f: impl Fn(&str) -> Option<T>) -> Result<T> {
    f(&code).ok_or(StoreError::UnknownCode { what, code })
}

fn parse_trigger(code: Option<String>) -> Result<Option<ReportTrigger>> {
    code.map(|c| parse("trigger", c, ReportTrigger::from_code))
        .transpose()
}

fn ticket_id(id: &str) -> Result<i64> {
    id.parse().map_err(|_| StoreError::NotFound)
}

fn is_corruption(e: &StoreError) -> bool {
    matches!(
        e,
        StoreError::Sqlite(rusqlite::Error::SqliteFailure(f, _))
            if matches!(f.code, ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt)
    )
}

/// `<file>.<suffix>` next to `path`.
fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".");
    name.push(suffix);
    path.with_file_name(name)
}

const TICKET_COLUMNS: &str =
    "id, family, state, reopen_count, last_trigger, created_at, last_changed_at";

/// Raw ticket row; codes are parsed outside the rusqlite callback so unknown codes become
/// [`StoreError::UnknownCode`].
type TicketRow = (i64, String, String, u32, Option<String>, i64, i64);

fn ticket_row(row: &Row<'_>) -> rusqlite::Result<TicketRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
    ))
}

fn ticket_from(row: TicketRow) -> Result<Ticket> {
    let (id, family, state, reopen_count, last_trigger, created_at, last_changed_at) = row;
    Ok(Ticket {
        id: id.to_string(),
        family: parse("family", family, ProblemFamily::from_code)?,
        state: parse("state", state, TicketState::from_code)?,
        reopen_count,
        last_trigger: parse_trigger(last_trigger)?,
        created_at,
        last_changed_at,
    })
}

pub struct Store {
    conn: Connection,
}

impl Store {
    /// Opens (or creates) the database at `path`, migrating and recovering as needed.
    pub fn open(path: &Path, now: Timestamp) -> Result<(Store, OpenOutcome)> {
        match Store::open_existing(path) {
            Err(e) if is_corruption(&e) => {
                let moved_to = sibling(path, &format!("corrupt-{now}"));
                std::fs::rename(path, &moved_to)?;
                // Stale WAL/SHM files belong to the bad file; keep them next to it.
                for ext in ["-wal", "-shm"] {
                    let mut side = path.as_os_str().to_os_string();
                    side.push(ext);
                    if Path::new(&side).exists() {
                        let mut target = moved_to.as_os_str().to_os_string();
                        target.push(ext);
                        std::fs::rename(&side, &target)?;
                    }
                }
                let (store, _) = Store::open_existing(path)?;
                Ok((store, OpenOutcome::RecreatedAfterCorruption { moved_to }))
            }
            other => other,
        }
    }

    fn open_existing(path: &Path) -> Result<(Store, OpenOutcome)> {
        let conn = Connection::open(path)?;
        let version = Store::configure(&conn)?;
        if version > 0 {
            let check: String = conn.pragma_query_value(None, "quick_check", |r| r.get(0))?;
            if check != "ok" {
                return Err(StoreError::Sqlite(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CORRUPT),
                    Some(check),
                )));
            }
        }
        let outcome = match version {
            0 => OpenOutcome::Created,
            v if v == SCHEMA_VERSION => OpenOutcome::Opened,
            from => {
                let backup = sibling(path, &format!("bak-v{from}"));
                if backup.exists() {
                    std::fs::remove_file(&backup)?;
                }
                conn.execute("VACUUM INTO ?1", [backup.to_string_lossy()])?;
                OpenOutcome::Migrated { from, backup }
            }
        };
        let mut store = Store { conn };
        store.migrate(version)?;
        Ok((store, outcome))
    }

    /// A private in-memory database (tests and "não guardar histórico" mode).
    pub fn open_in_memory() -> Result<Store> {
        let conn = Connection::open_in_memory()?;
        let version = Store::configure(&conn)?;
        let mut store = Store { conn };
        store.migrate(version)?;
        Ok(store)
    }

    /// FR-004: WAL and foreign keys; returns the current `user_version`. The version is checked
    /// first so a newer file is never modified (not even its journal mode).
    fn configure(conn: &Connection) -> Result<i32> {
        let version: i32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(StoreError::SchemaTooNew(version));
        }
        let _mode: String =
            conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get(0))?;
        conn.pragma_update(None, "foreign_keys", true)?;
        Ok(version)
    }

    fn migrate(&mut self, from: i32) -> Result<()> {
        let tx = self.conn.transaction()?;
        for (i, sql) in schema::MIGRATIONS.iter().enumerate() {
            let target = i32::try_from(i + 1).unwrap_or(i32::MAX);
            if target > from {
                tx.execute_batch(sql)?;
                tx.pragma_update(None, "user_version", target)?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn schema_version(&self) -> Result<i32> {
        Ok(self
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))?)
    }

    // ----- tickets -----

    pub fn create_ticket(&mut self, family: ProblemFamily, now: Timestamp) -> Result<Ticket> {
        let draft = Ticket::new("", family, now);
        self.conn.execute(
            "INSERT INTO ticket (family, state, reopen_count, created_at, last_changed_at)
             VALUES (?1, ?2, 0, ?3, ?3)",
            params![family.code(), draft.state.code(), now],
        )?;
        Ok(Ticket {
            id: self.conn.last_insert_rowid().to_string(),
            ..draft
        })
    }

    pub fn ticket(&self, id: &str) -> Result<Option<Ticket>> {
        let Ok(id) = id.parse::<i64>() else {
            return Ok(None);
        };
        self.conn
            .query_row(
                &format!("SELECT {TICKET_COLUMNS} FROM ticket WHERE id = ?1"),
                [id],
                ticket_row,
            )
            .optional()?
            .map(ticket_from)
            .transpose()
    }

    pub fn tickets(&self) -> Result<Vec<Ticket>> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {TICKET_COLUMNS} FROM ticket ORDER BY id"))?;
        let rows = stmt.query_map([], ticket_row)?;
        rows.map(|r| ticket_from(r?)).collect()
    }

    /// FR-001/FR-002: the only way to change a ticket. The domain decides; the update applies only
    /// if the row is still what was read, and the event is appended to the history.
    pub fn apply(&mut self, id: &str, event: TicketEvent, now: Timestamp) -> Result<Ticket> {
        let current = self.ticket(id)?.ok_or(StoreError::NotFound)?;
        let next = current.apply(event, now)?;
        let row_id = ticket_id(id)?;
        let trigger = match event {
            TicketEvent::CannotResolve(t) => Some(t.code()),
            _ => None,
        };
        let tx = self.conn.transaction()?;
        let changed = tx.execute(
            "UPDATE ticket SET state = ?1, reopen_count = ?2, last_trigger = ?3, last_changed_at = ?4
             WHERE id = ?5 AND state = ?6 AND last_changed_at = ?7",
            params![
                next.state.code(),
                next.reopen_count,
                next.last_trigger.map(ReportTrigger::code),
                next.last_changed_at,
                row_id,
                current.state.code(),
                current.last_changed_at,
            ],
        )?;
        if changed != 1 {
            return Err(StoreError::ConcurrentChange);
        }
        tx.execute(
            "INSERT INTO ticket_event (ticket_id, event, trigger, from_state, to_state, at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                row_id,
                event.code(),
                trigger,
                current.state.code(),
                next.state.code(),
                now
            ],
        )?;
        let actor = match event {
            // Time-based housekeeping, not the person.
            TicketEvent::Abandon | TicketEvent::Close => Actor::System,
            _ => Actor::User,
        };
        audit::append(
            &tx,
            &audit::AuditEntry {
                actor,
                action: &format!("ticket.{}", event.code()),
                subject_ref: format!("ticket:{row_id}"),
                outcome: next.state.code().to_string(),
                reason_code: trigger,
            },
            now,
        )?;
        tx.commit()?;
        Ok(next)
    }

    pub fn history(&self, id: &str) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT event, trigger, from_state, to_state, at FROM ticket_event
             WHERE ticket_id = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([ticket_id(id)?], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })?;
        rows.map(|r| {
            let (event, trigger, from, to, at) = r?;
            Ok(HistoryEntry {
                event,
                trigger: parse_trigger(trigger)?,
                from: parse("state", from, TicketState::from_code)?,
                to: parse("state", to, TicketState::from_code)?,
                at,
            })
        })
        .collect()
    }

    /// Times when tickets of `family` were reopened, oldest first, for REP-08
    /// ([`hmd_domain::repeated_reopen_trigger`]).
    pub fn reopen_times(&self, family: ProblemFamily) -> Result<Vec<Timestamp>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.at FROM ticket_event e JOIN ticket t ON t.id = e.ticket_id
             WHERE t.family = ?1 AND e.event = ?2 ORDER BY e.at",
        )?;
        let rows = stmt.query_map(params![family.code(), TicketEvent::Reopen.code()], |r| {
            r.get(0)
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ----- sessions and findings -----

    /// US1.3: stores a finished diagnosis (playbook@version, diagnosis, confidence, findings).
    #[allow(clippy::too_many_arguments)]
    pub fn record_diagnosis(
        &mut self,
        ticket: &str,
        catalog: &LoadedCatalog,
        report: &DiagnosisReport,
        platform: Platform,
        phase: SessionPhase,
        started_at: Timestamp,
        ended_at: Timestamp,
    ) -> Result<i64> {
        let row_id = ticket_id(ticket)?;
        let version = |id: &str| -> String {
            catalog
                .catalog
                .get(id)
                .map(|c| c.version.clone())
                .or_else(|| catalog.playbooks.get(id).map(|p| p.version.clone()))
                .unwrap_or_else(|| "?".to_string())
        };
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO diagnostic_session (ticket_id, phase, playbook, diagnosis_id, confidence,
                 platform, catalog_version, started_at, ended_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                row_id,
                phase.code(),
                format!("{}@{}", report.playbook_id, version(&report.playbook_id)),
                report.diagnosis_id,
                confidence_code(report.confidence),
                platform_code(platform),
                catalog.version,
                started_at,
                ended_at
            ],
        )?;
        let session = tx.last_insert_rowid();
        for probe in &report.probes {
            if let ProbeOutcome::Ran(out) = &probe.outcome {
                for finding in &out.findings {
                    tx.execute(
                        "INSERT INTO diagnostic_finding (session_id, code, severity, source_probe)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![
                            session,
                            finding.code,
                            severity_code(finding.severity),
                            format!("{}@{}", probe.probe_id, version(&probe.probe_id))
                        ],
                    )?;
                }
            }
        }
        tx.commit()?;
        Ok(session)
    }

    pub fn sessions(&self, ticket: &str) -> Result<Vec<StoredSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, phase, playbook, diagnosis_id, confidence, platform, catalog_version,
                    started_at, ended_at
             FROM diagnostic_session WHERE ticket_id = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([ticket_id(ticket)?], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, i64>(7)?,
                r.get::<_, Option<i64>>(8)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, phase, playbook, diagnosis_id, confidence, platform, catalog_version, s, e) =
                row?;
            let mut fstmt = self.conn.prepare(
                "SELECT code, severity, source_probe FROM diagnostic_finding
                 WHERE session_id = ?1 ORDER BY id",
            )?;
            let findings = fstmt
                .query_map([id], |r| {
                    Ok(StoredFinding {
                        code: r.get(0)?,
                        severity: r.get(1)?,
                        source_probe: r.get(2)?,
                    })
                })?
                .collect::<rusqlite::Result<_>>()?;
            out.push(StoredSession {
                id,
                phase: parse("phase", phase, SessionPhase::from_code)?,
                playbook,
                diagnosis_id,
                confidence,
                platform,
                catalog_version,
                started_at: s,
                ended_at: e,
                findings,
            });
        }
        Ok(out)
    }

    // ----- messages -----

    /// Free text is sanitized, identity-redacted and limited before it is stored.
    pub fn add_message(
        &mut self,
        ticket: &str,
        author: Author,
        body: &str,
        now: Timestamp,
    ) -> Result<i64> {
        let body = hmd_core::report::redact_free_text(body);
        self.conn.execute(
            "INSERT INTO ticket_message (ticket_id, author_type, body_redacted, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![ticket_id(ticket)?, author.code(), body, now],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn messages(&self, ticket: &str) -> Result<Vec<StoredMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT author_type, body_redacted, created_at FROM ticket_message
             WHERE ticket_id = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([ticket_id(ticket)?], |r| {
            Ok(StoredMessage {
                author: r.get(0)?,
                body: r.get(1)?,
                created_at: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ----- consents -----

    /// Append-only (US3.3): a change of mind is a new record.
    pub fn record_consent(
        &mut self,
        scope: &str,
        granted: bool,
        notice_version: &str,
        now: Timestamp,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO consent_record (scope, granted, notice_version, at) VALUES (?1, ?2, ?3, ?4)",
            params![scope, granted, notice_version, now],
        )?;
        audit::append(
            &tx,
            &audit::AuditEntry {
                actor: Actor::User,
                action: "consent.changed",
                subject_ref: format!("consent:{scope}"),
                outcome: format!(
                    "{}@{notice_version}",
                    if granted { "granted" } else { "revoked" }
                ),
                reason_code: None,
            },
            now,
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Spec 009 US2: grants or revokes a catalog consent category, recording the version of the
    /// notice the person read. Unknown categories are refused.
    pub fn set_consent(
        &mut self,
        catalog: &LoadedCatalog,
        scope: &str,
        granted: bool,
        now: Timestamp,
    ) -> Result<()> {
        let notice = catalog
            .consents
            .get(scope)
            .ok_or_else(|| StoreError::UnknownCode {
                what: "consent",
                code: scope.to_string(),
            })?;
        let version = notice.version.clone();
        self.record_consent(scope, granted, &version, now)
    }

    /// Scopes whose latest record grants consent.
    pub fn consents(&self) -> Result<BTreeSet<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT scope FROM consent_record c
             WHERE granted = 1 AND id = (SELECT max(id) FROM consent_record WHERE scope = c.scope)",
        )?;
        let rows = stmt.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ----- settings -----

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM setting WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    /// Stores a setting and audits the change (codes only).
    pub fn set_setting(&mut self, key: &str, value: &str, now: Timestamp) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO setting (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, now],
        )?;
        audit::append(
            &tx,
            &audit::AuditEntry {
                actor: Actor::User,
                action: "setting.changed",
                subject_ref: format!("setting:{key}"),
                outcome: value.to_string(),
                reason_code: None,
            },
            now,
        )?;
        tx.commit()?;
        Ok(())
    }

    // ----- kill switch -----

    /// The last accepted kill switch file and its signature, if any.
    pub fn kill_switch(&self) -> Result<Option<(Vec<u8>, String)>> {
        Ok(self
            .conn
            .query_row(
                "SELECT payload, signature FROM kill_switch_state WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?)
    }

    /// Replaces the stored kill switch (the caller verified and accepted it) and audits it.
    pub fn set_kill_switch(
        &mut self,
        issued_at: Timestamp,
        payload: &[u8],
        signature: &str,
        now: Timestamp,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO kill_switch_state (id, issued_at, payload, signature, accepted_at)
             VALUES (1, ?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET issued_at = excluded.issued_at,
                 payload = excluded.payload, signature = excluded.signature,
                 accepted_at = excluded.accepted_at",
            params![issued_at, payload, signature, now],
        )?;
        audit::append(
            &tx,
            &audit::AuditEntry {
                actor: Actor::System,
                action: "killswitch.accepted",
                subject_ref: "killswitch".to_string(),
                outcome: format!("issued_at={issued_at}"),
                reason_code: None,
            },
            now,
        )?;
        tx.commit()?;
        Ok(())
    }

    // ----- reports -----

    pub fn save_report(
        &mut self,
        ticket: &str,
        trigger: ReportTrigger,
        content: &str,
        now: Timestamp,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO report (ticket_id, trigger, content, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![ticket_id(ticket)?, trigger.code(), content, now],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// US2.5: keeps what the person decided to keep.
    pub fn edit_report(&mut self, report: i64, content: &str, now: Timestamp) -> Result<()> {
        let changed = self.conn.execute(
            "UPDATE report SET content = ?1, updated_at = ?2 WHERE id = ?3",
            params![content, now, report],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::NotFound)
        }
    }

    /// Called only after the file was saved by the person.
    pub fn mark_exported(&mut self, report: i64, now: Timestamp) -> Result<()> {
        let tx = self.conn.transaction()?;
        let changed = tx.execute(
            "UPDATE report SET exported_at = ?1 WHERE id = ?2",
            params![now, report],
        )?;
        if changed != 1 {
            return Err(StoreError::NotFound);
        }
        audit::append(
            &tx,
            &audit::AuditEntry {
                actor: Actor::User,
                action: "report.exported",
                subject_ref: format!("report:{report}"),
                outcome: "ok".to_string(),
                reason_code: None,
            },
            now,
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn reports(&self, ticket: &str) -> Result<Vec<StoredReport>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ticket_id, trigger, content, created_at, updated_at, exported_at
             FROM report WHERE ticket_id = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([ticket_id(ticket)?], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, Option<i64>>(6)?,
            ))
        })?;
        rows.map(|r| {
            let (id, ticket_id, trigger, content, created_at, updated_at, exported_at) = r?;
            Ok(StoredReport {
                id,
                ticket_id: ticket_id.to_string(),
                trigger: parse("trigger", trigger, ReportTrigger::from_code)?,
                content,
                created_at,
                updated_at,
                exported_at,
            })
        })
        .collect()
    }

    // ----- retention -----

    /// US3: deletes expired tickets (cascading to everything attached) and expired messages.
    /// The decisions are the pure rules of [`hmd_domain::retention`].
    pub fn purge(&mut self, now: Timestamp, cfg: &RetentionConfig) -> Result<PurgeStats> {
        let expired: Vec<String> = self
            .tickets()?
            .into_iter()
            .filter(|t| ticket_expired(t, now, cfg))
            .map(|t| t.id)
            .collect();
        let old_messages: Vec<i64> = {
            let mut stmt = self
                .conn
                .prepare("SELECT id, created_at FROM ticket_message")?;
            let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
            let mut ids = Vec::new();
            for row in rows {
                let (id, created_at) = row?;
                if message_expired(created_at, now, cfg) {
                    ids.push(id);
                }
            }
            ids
        };
        let tx = self.conn.transaction()?;
        let mut stats = PurgeStats::default();
        for id in &old_messages {
            stats.messages += tx.execute("DELETE FROM ticket_message WHERE id = ?1", [id])?;
        }
        for id in &expired {
            stats.tickets += tx.execute("DELETE FROM ticket WHERE id = ?1", [ticket_id(id)?])?;
        }
        stats.audit_events = audit::purge(&tx, now, cfg)?;
        if stats != PurgeStats::default() {
            audit::append(
                &tx,
                &audit::AuditEntry {
                    actor: Actor::System,
                    action: "retention.purged",
                    subject_ref: "store".to_string(),
                    outcome: format!(
                        "tickets={} messages={} audit_events={}",
                        stats.tickets, stats.messages, stats.audit_events
                    ),
                    reason_code: None,
                },
                now,
            )?;
        }
        tx.commit()?;
        Ok(stats)
    }
}
