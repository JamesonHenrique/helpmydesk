// SPDX-License-Identifier: GPL-3.0-or-later
//! Local audit trail with a hash chain (spec 009, Plan v1 VII.3 `AuditEvent`).
//!
//! Each event holds only references (`ticket:3`, `execution:7`), never personal content.
//! `hash = sha256(prev_hash ‖ "\n" ‖ canonical(event))`, so editing, removing or reordering an
//! event in the middle is detectable. It is best effort: the owner of the computer can delete the
//! whole file, and removing the newest events cannot be told apart from them never existing.

use hmd_domain::{RetentionConfig, Timestamp, audit_expired};
use rusqlite::{Connection, OptionalExtension, params};

use crate::{Result, Store};

/// `prev_hash` of the first event ever written.
pub const GENESIS: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// Format tag inside the hashed text.
pub const AUDIT_FORMAT: &str = "audit@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    /// The person, through the app.
    User,
    /// The AI (a proposal it made).
    Agent,
    /// The app itself (housekeeping, recovery, retention).
    System,
}

impl Actor {
    pub fn code(self) -> &'static str {
        match self {
            Actor::User => "user",
            Actor::Agent => "agent",
            Actor::System => "system",
        }
    }
}

/// What is appended; every field is a code or a reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry<'a> {
    pub actor: Actor,
    pub action: &'a str,
    pub subject_ref: String,
    pub outcome: String,
    pub reason_code: Option<&'a str>,
}

/// One stored event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    pub id: i64,
    pub occurred_at: Timestamp,
    pub actor: String,
    pub action: String,
    pub subject_ref: String,
    pub outcome: String,
    pub reason_code: Option<String>,
    pub prev_hash: String,
    pub hash: String,
}

/// Result of [`Store::verify_audit_chain`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditCheck {
    pub events: usize,
    /// ID of the first event whose hash or link does not match.
    pub broken_at: Option<i64>,
}

fn field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\p")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Canonical text of an event (without its own hash). Pure.
#[allow(clippy::too_many_arguments)]
pub fn canonical_event(
    occurred_at: Timestamp,
    actor: &str,
    action: &str,
    subject_ref: &str,
    outcome: &str,
    reason_code: Option<&str>,
) -> String {
    format!(
        "{AUDIT_FORMAT}|{occurred_at}|{}|{}|{}|{}|{}",
        field(actor),
        field(action),
        field(subject_ref),
        field(outcome),
        reason_code.map_or_else(|| "-".to_string(), field)
    )
}

/// `sha256(prev_hash ‖ "\n" ‖ canonical)`.
pub fn chain_hash(prev_hash: &str, canonical: &str) -> String {
    hmd_executor::sha256_tagged(&format!("{prev_hash}\n{canonical}"))
}

/// Appends one event inside the caller's transaction (so the change and its audit commit
/// together).
pub(crate) fn append(conn: &Connection, entry: &AuditEntry<'_>, now: Timestamp) -> Result<()> {
    let prev: String = conn
        .query_row(
            "SELECT hash FROM audit_event ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or_else(|| GENESIS.to_string());
    let text = canonical_event(
        now,
        entry.actor.code(),
        entry.action,
        &entry.subject_ref,
        &entry.outcome,
        entry.reason_code,
    );
    let hash = chain_hash(&prev, &text);
    conn.execute(
        "INSERT INTO audit_event (occurred_at, actor, action, subject_ref, outcome, reason_code,
             prev_hash, hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            now,
            entry.actor.code(),
            entry.action,
            entry.subject_ref,
            entry.outcome,
            entry.reason_code,
            prev,
            hash
        ],
    )?;
    Ok(())
}

/// Removes the expired prefix of the chain; returns how many events went away.
pub(crate) fn purge(conn: &Connection, now: Timestamp, cfg: &RetentionConfig) -> Result<usize> {
    let mut stmt = conn.prepare("SELECT id, occurred_at FROM audit_event ORDER BY id")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
    let mut cut = None;
    for row in rows {
        let (id, at) = row?;
        if audit_expired(at, now, cfg) {
            cut = Some(id);
        } else {
            break;
        }
    }
    Ok(match cut {
        Some(id) => conn.execute("DELETE FROM audit_event WHERE id <= ?1", [id])?,
        None => 0,
    })
}

impl Store {
    pub fn audit_events(&self) -> Result<Vec<AuditEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, occurred_at, actor, action, subject_ref, outcome, reason_code, prev_hash,
                    hash
             FROM audit_event ORDER BY id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(AuditEvent {
                id: r.get(0)?,
                occurred_at: r.get(1)?,
                actor: r.get(2)?,
                action: r.get(3)?,
                subject_ref: r.get(4)?,
                outcome: r.get(5)?,
                reason_code: r.get(6)?,
                prev_hash: r.get(7)?,
                hash: r.get(8)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Recomputes every hash and link. The first remaining event's `prev_hash` is the anchor (an
    /// expired prefix may have been removed by retention).
    pub fn verify_audit_chain(&self) -> Result<AuditCheck> {
        let events = self.audit_events()?;
        let mut prev: Option<&str> = None;
        for e in &events {
            let linked = prev.is_none_or(|p| p == e.prev_hash);
            let text = canonical_event(
                e.occurred_at,
                &e.actor,
                &e.action,
                &e.subject_ref,
                &e.outcome,
                e.reason_code.as_deref(),
            );
            if !linked || chain_hash(&e.prev_hash, &text) != e.hash {
                return Ok(AuditCheck {
                    events: events.len(),
                    broken_at: Some(e.id),
                });
            }
            prev = Some(&e.hash);
        }
        Ok(AuditCheck {
            events: events.len(),
            broken_at: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_escapes_separators() {
        let a = canonical_event(1, "user", "x", "a|b", "ok", None);
        let b = canonical_event(1, "user", "x", "a", "b|ok", None);
        assert_ne!(a, b);
        assert!(a.starts_with("audit@1|1|user|x|a\\pb|ok|-"));
    }
}
