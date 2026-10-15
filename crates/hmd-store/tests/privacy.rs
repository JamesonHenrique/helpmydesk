// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 009: audit trail with hash chain, consent with notice version, export and erase all.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use hmd_catalog::embedded;
use hmd_domain::{DAY, ProblemFamily, ReportTrigger, RetentionConfig, TicketEvent};
use hmd_store::{Author, Store, StoreError, erase_all};

const NOW: i64 = 1_760_400_000;

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("hmd-privacy-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }
    fn db(&self) -> PathBuf {
        self.0.join("history.sqlite3")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn some_activity(s: &mut Store) -> String {
    let t = s.create_ticket(ProblemFamily::Connectivity, NOW).unwrap();
    s.apply(&t.id, TicketEvent::StartSession, NOW).unwrap();
    s.add_message(&t.id, Author::Person, "segredo-da-pessoa: a rede caiu", NOW)
        .unwrap();
    let r = s
        .save_report(
            &t.id,
            ReportTrigger::PersonRequested,
            "texto do relatório",
            NOW,
        )
        .unwrap();
    s.mark_exported(r, NOW + 1).unwrap();
    s.apply(
        &t.id,
        TicketEvent::CannotResolve(ReportTrigger::PersonRequested),
        NOW + 2,
    )
    .unwrap();
    s.set_consent(embedded().unwrap(), "startup_apps", true, NOW + 3)
        .unwrap();
    t.id
}

#[test]
fn us3_actions_are_audited_with_references_only() {
    let mut s = Store::open_in_memory().unwrap();
    some_activity(&mut s);
    let events = s.audit_events().unwrap();
    let actions: Vec<&str> = events.iter().map(|e| e.action.as_str()).collect();
    assert_eq!(
        actions,
        vec![
            "ticket.start_session",
            "report.exported",
            "ticket.cannot_resolve",
            "consent.changed"
        ]
    );
    assert_eq!(events[2].reason_code.as_deref(), Some("REP-06"));
    assert_eq!(events[3].outcome, "granted@1");
    for e in &events {
        let all = format!("{e:?}");
        assert!(!all.contains("segredo"), "no personal content in the audit");
        assert!(!all.contains("texto do relatório"));
    }
    let check = s.verify_audit_chain().unwrap();
    assert_eq!((check.events, check.broken_at), (4, None));
    assert_eq!(events[0].prev_hash, hmd_store::audit::GENESIS);
}

#[test]
fn us3_tampering_is_detected() {
    let dir = TempDir::new("tamper");
    let (mut s, _) = Store::open(&dir.db(), NOW).unwrap();
    some_activity(&mut s);
    drop(s);
    let conn = rusqlite::Connection::open(dir.db()).unwrap();
    assert!(
        conn.execute("UPDATE audit_event SET outcome = 'x' WHERE id = 2", [])
            .is_err(),
        "append-only trigger"
    );
    // Someone with the file deletes an event in the middle.
    conn.execute("DELETE FROM audit_event WHERE id = 2", [])
        .unwrap();
    drop(conn);
    let (s, _) = Store::open(&dir.db(), NOW).unwrap();
    assert_eq!(s.verify_audit_chain().unwrap().broken_at, Some(3));

    // Or bypasses the trigger to rewrite one.
    drop(s);
    let conn = rusqlite::Connection::open(dir.db()).unwrap();
    conn.execute_batch(
        "DROP TRIGGER audit_event_no_update; UPDATE audit_event SET outcome = 'resolved' WHERE id = 3;",
    )
    .unwrap();
    drop(conn);
    let (s, _) = Store::open(&dir.db(), NOW).unwrap();
    assert!(s.verify_audit_chain().unwrap().broken_at.is_some());
}

#[test]
fn us3_retention_removes_only_an_old_prefix_and_the_chain_still_verifies() {
    let mut s = Store::open_in_memory().unwrap();
    let id = some_activity(&mut s);
    s.apply(&id, TicketEvent::Close, NOW + 100 * DAY).unwrap();
    let stats = s
        .purge(NOW + 181 * DAY, &RetentionConfig::default())
        .unwrap();
    assert_eq!(stats.audit_events, 4, "only the events older than 180 days");
    let left = s.audit_events().unwrap();
    assert_eq!(
        left.iter().map(|e| e.action.as_str()).collect::<Vec<_>>(),
        vec!["ticket.close", "retention.purged"]
    );
    assert_eq!(s.verify_audit_chain().unwrap().broken_at, None);
}

/// Spec 016 (P2): a clock that jumps more than the retention period past the last recorded
/// event does not wipe the history; the purge waits for a session with a sane clock.
#[test]
fn clock_jump_does_not_wipe_the_history() {
    let mut s = Store::open_in_memory().unwrap();
    let id = some_activity(&mut s);
    let before = s.audit_events().unwrap().len();
    let stats = s
        .purge(NOW + 400 * DAY, &RetentionConfig::default())
        .unwrap();
    assert_eq!(stats.audit_events, 0);
    assert_eq!(stats.tickets, 0);
    assert_eq!(s.audit_events().unwrap().len(), before);
    // Normal use at the new time, then the next start purges as usual.
    s.apply(&id, TicketEvent::Close, NOW + 400 * DAY).unwrap();
    let stats = s
        .purge(NOW + 400 * DAY + 1, &RetentionConfig::default())
        .unwrap();
    assert!(stats.audit_events > 0);
}

#[test]
fn us2_consent_records_the_notice_version() {
    let mut s = Store::open_in_memory().unwrap();
    let catalog = embedded().unwrap();
    s.set_consent(catalog, "ai_opencode", true, NOW).unwrap();
    s.set_consent(catalog, "ai_opencode", false, NOW + 1)
        .unwrap();
    assert!(!s.consents().unwrap().contains("ai_opencode"));
    assert!(matches!(
        s.set_consent(catalog, "everything", true, NOW),
        Err(StoreError::UnknownCode {
            what: "consent",
            ..
        })
    ));
    let export = s.export_markdown(NOW).unwrap();
    assert!(export.contains("`ai_opencode` concedida (aviso versão 2)"));
    assert!(export.contains("`ai_opencode` retirada (aviso versão 2)"));
}

#[test]
fn us4_export_has_everything_kept() {
    let mut s = Store::open_in_memory().unwrap();
    some_activity(&mut s);
    let text = s.export_markdown(NOW).unwrap();
    assert!(text.starts_with("<!-- export@1 -->"));
    for needle in [
        "## Permissões",
        "## Chamados",
        "segredo-da-pessoa: a rede caiu",
        "texto do relatório",
        "`ticket.cannot_resolve`",
        "cadeia íntegra",
    ] {
        assert!(text.contains(needle), "missing {needle}");
    }
}

#[test]
fn us4_erase_all_removes_only_the_store_files() {
    let dir = TempDir::new("erase");
    let (mut s, _) = Store::open(&dir.db(), NOW).unwrap();
    some_activity(&mut s);
    drop(s);
    let name = dir.db().file_name().unwrap().to_string_lossy().to_string();
    for extra in [
        format!("{name}.bak-v1"),
        format!("{name}.corrupt-{NOW}"),
        "notes.txt".to_string(),
        format!("other-{name}"),
    ] {
        std::fs::write(dir.0.join(extra), b"x").unwrap();
    }
    let removed = erase_all(&dir.db()).unwrap();
    assert!(removed.len() >= 3, "{removed:?}");
    assert!(!dir.db().exists());
    assert!(dir.0.join("notes.txt").exists(), "unrelated file kept");
    assert!(dir.0.join(format!("other-{name}")).exists());
    let (s, outcome) = Store::open(&dir.db(), NOW).unwrap();
    assert_eq!(outcome, hmd_store::OpenOutcome::Created);
    assert!(s.tickets().unwrap().is_empty());
}

/// SC-002: a v2 database (spec 008) gains the audit table.
#[test]
fn sc002_v2_database_migrates_to_v3() {
    let dir = TempDir::new("v2");
    {
        let conn = rusqlite::Connection::open(dir.db()).unwrap();
        conn.execute_batch(hmd_store::schema::V1).unwrap();
        conn.execute_batch(hmd_store::schema::V2).unwrap();
        conn.pragma_update(None, "user_version", 2).unwrap();
    }
    let (mut s, outcome) = Store::open(&dir.db(), NOW).unwrap();
    assert!(matches!(
        outcome,
        hmd_store::OpenOutcome::Migrated { from: 2, .. }
    ));
    some_activity(&mut s);
    assert_eq!(s.verify_audit_chain().unwrap().broken_at, None);
}
