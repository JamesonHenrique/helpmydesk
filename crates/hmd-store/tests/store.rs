// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 007 acceptance tests for the local history.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use hmd_catalog::embedded;
use hmd_core::report::{ReportInput, redact_free_text, render, system_summary_with};
use hmd_core::{DiagnoseOptions, current_platform, diagnose};
use hmd_domain::{
    DAY, DomainError, ProblemFamily, ReportTrigger, RetentionConfig, SessionPhase, TicketEvent,
    TicketState, repeated_reopen_trigger,
};
use hmd_store::{Author, OpenOutcome, SCHEMA_VERSION, Store, StoreError};

const NOW: i64 = 1_760_400_000;

/// A fresh directory per test under the system temp dir (no extra dependency).
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("hmd-store-{}-{name}", std::process::id()));
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

#[test]
fn us1_1_lifecycle_is_stored_with_history() {
    let mut s = Store::open_in_memory().unwrap();
    let t = s.create_ticket(ProblemFamily::Connectivity, NOW).unwrap();
    assert_eq!(t.state, TicketState::Open);
    s.apply(&t.id, TicketEvent::StartSession, NOW + 1).unwrap();
    s.apply(
        &t.id,
        TicketEvent::CannotResolve(ReportTrigger::NoApplicableFix),
        NOW + 2,
    )
    .unwrap();
    let stored = s.ticket(&t.id).unwrap().unwrap();
    assert_eq!(stored.state, TicketState::Unresolved);
    assert_eq!(stored.last_trigger, Some(ReportTrigger::NoApplicableFix));
    assert_eq!(stored.last_changed_at, NOW + 2);
    let history = s.history(&t.id).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].event, "start_session");
    assert_eq!(history[1].trigger, Some(ReportTrigger::NoApplicableFix));
    assert_eq!(
        (history[1].from, history[1].to),
        (TicketState::InProgress, TicketState::Unresolved)
    );
}

#[test]
fn us1_2_invalid_transition_leaves_ticket_unchanged() {
    let mut s = Store::open_in_memory().unwrap();
    let t = s.create_ticket(ProblemFamily::Storage, NOW).unwrap();
    s.apply(&t.id, TicketEvent::Cancel, NOW + 1).unwrap();
    let err = s
        .apply(&t.id, TicketEvent::StartSession, NOW + 2)
        .unwrap_err();
    assert!(matches!(
        err,
        StoreError::Domain(DomainError::InvalidTransition { .. })
    ));
    let stored = s.ticket(&t.id).unwrap().unwrap();
    assert_eq!(stored.state, TicketState::Cancelled);
    assert_eq!(s.history(&t.id).unwrap().len(), 1);
    assert!(matches!(
        s.apply("999", TicketEvent::Cancel, NOW),
        Err(StoreError::NotFound)
    ));
}

#[test]
fn us1_4_reopen_times_feed_rep08() {
    let mut s = Store::open_in_memory().unwrap();
    for i in 0..2 {
        let t = s.create_ticket(ProblemFamily::Performance, NOW).unwrap();
        s.apply(&t.id, TicketEvent::StartSession, NOW).unwrap();
        s.apply(
            &t.id,
            TicketEvent::CannotResolve(ReportTrigger::LowConfidence),
            NOW,
        )
        .unwrap();
        s.apply(&t.id, TicketEvent::Reopen, NOW + i * DAY).unwrap();
    }
    let times = s.reopen_times(ProblemFamily::Performance).unwrap();
    assert_eq!(times, vec![NOW, NOW + DAY]);
    assert_eq!(
        repeated_reopen_trigger(&times, NOW + 2 * DAY),
        Some(ReportTrigger::RepeatedReopen)
    );
    assert!(s.reopen_times(ProblemFamily::Storage).unwrap().is_empty());
}

#[test]
fn us1_3_and_sc002_real_diagnosis_session_and_report() {
    let catalog = embedded().unwrap();
    let opts = DiagnoseOptions::default();
    let mut s = Store::open_in_memory().unwrap();
    for family in ProblemFamily::ALL {
        let t = s.create_ticket(family, NOW).unwrap();
        s.apply(&t.id, TicketEvent::StartSession, NOW).unwrap();
        let d = diagnose(family, &opts).unwrap();
        let session = s
            .record_diagnosis(
                &t.id,
                catalog,
                &d,
                current_platform(),
                SessionPhase::Done,
                NOW,
                NOW + 5,
            )
            .unwrap();
        let sessions = s.sessions(&t.id).unwrap();
        assert_eq!(sessions.len(), 1);
        let stored = &sessions[0];
        assert_eq!(stored.id, session);
        assert_eq!(stored.diagnosis_id, d.diagnosis_id);
        assert!(stored.playbook.starts_with(&format!("{}@", d.playbook_id)));
        assert!(!stored.playbook.ends_with("@?"));
        assert_eq!(stored.catalog_version, catalog.version);
        let codes: Vec<&str> = stored.findings.iter().map(|f| f.code.as_str()).collect();
        let expected: Vec<&str> = d.findings.iter().map(String::as_str).collect();
        let mut sorted = codes.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, expected, "{family:?}");
        assert!(
            stored
                .findings
                .iter()
                .all(|f| !f.source_probe.ends_with("@?"))
        );

        let trigger = d.report.unwrap_or(ReportTrigger::PersonRequested);
        let system = system_summary_with(catalog, &opts, hmd_probes::run);
        let text = render(&ReportInput {
            catalog,
            diagnosis: &d,
            trigger,
            symptom: Some(&redact_free_text("não funciona")),
            system: system.as_ref(),
            created_at: NOW,
        });
        let report = s.save_report(&t.id, trigger, &text, NOW).unwrap();
        s.apply(&t.id, TicketEvent::CannotResolve(trigger), NOW + 6)
            .unwrap();
        assert_eq!(s.reports(&t.id).unwrap()[0].id, report);
    }
}

#[test]
fn us2_5_edited_report_is_kept_and_export_is_explicit() {
    let mut s = Store::open_in_memory().unwrap();
    let t = s.create_ticket(ProblemFamily::AppError, NOW).unwrap();
    let id = s
        .save_report(&t.id, ReportTrigger::PersonRequested, "rascunho", NOW)
        .unwrap();
    assert_eq!(s.reports(&t.id).unwrap()[0].exported_at, None);
    s.edit_report(id, "texto que a pessoa editou", NOW + 10)
        .unwrap();
    let r = &s.reports(&t.id).unwrap()[0];
    assert_eq!(r.content, "texto que a pessoa editou");
    assert_eq!(r.updated_at, NOW + 10);
    assert_eq!(r.exported_at, None);
    s.mark_exported(id, NOW + 20).unwrap();
    assert_eq!(s.reports(&t.id).unwrap()[0].exported_at, Some(NOW + 20));
    assert!(matches!(
        s.mark_exported(id + 1, NOW),
        Err(StoreError::NotFound)
    ));
}

#[test]
fn messages_are_redacted_and_limited() {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    let mut s = Store::open_in_memory().unwrap();
    let t = s.create_ticket(ProblemFamily::Connectivity, NOW).unwrap();
    s.add_message(
        &t.id,
        Author::Person,
        &format!("sou {user}\u{202E} e a rede caiu"),
        NOW,
    )
    .unwrap();
    s.add_message(&t.id, Author::Person, &"x".repeat(9_000), NOW)
        .unwrap();
    let m = s.messages(&t.id).unwrap();
    assert!(!m[0].body.contains('\u{202E}'));
    if user.chars().count() >= 3 {
        assert!(!m[0].body.contains(&user), "user name stored");
    }
    assert_eq!(m[1].body.chars().count(), 4000);
    assert_eq!(m[0].author, "person");
}

#[test]
fn consents_are_append_only_latest_wins() {
    let mut s = Store::open_in_memory().unwrap();
    s.record_consent("startup_apps", true, "1", NOW).unwrap();
    s.record_consent("ai_opencode", true, "1", NOW).unwrap();
    s.record_consent("ai_opencode", false, "1", NOW + 1)
        .unwrap();
    let granted = s.consents().unwrap();
    assert!(granted.contains("startup_apps"));
    assert!(!granted.contains("ai_opencode"));
}

#[test]
fn us3_retention_purges_old_terminal_tickets_and_old_messages() {
    let mut s = Store::open_in_memory().unwrap();
    let cfg = RetentionConfig::default();
    let old = s.create_ticket(ProblemFamily::Storage, NOW).unwrap();
    s.add_message(&old.id, Author::Person, "antigo", NOW)
        .unwrap();
    s.save_report(&old.id, ReportTrigger::PersonRequested, "r", NOW)
        .unwrap();
    s.apply(&old.id, TicketEvent::Cancel, NOW).unwrap();

    let open = s.create_ticket(ProblemFamily::Storage, NOW).unwrap();
    s.add_message(&open.id, Author::Person, "velho", NOW)
        .unwrap();
    s.add_message(&open.id, Author::Person, "novo", NOW + 100 * DAY)
        .unwrap();

    let stats = s.purge(NOW + 180 * DAY, &cfg).unwrap();
    assert_eq!(stats.tickets, 1);
    assert!(s.ticket(&old.id).unwrap().is_none());
    assert!(s.reports(&old.id).unwrap().is_empty(), "cascade");
    assert!(s.history(&old.id).unwrap().is_empty(), "cascade");
    let left: Vec<String> = s
        .messages(&open.id)
        .unwrap()
        .into_iter()
        .map(|m| m.body)
        .collect();
    assert_eq!(left, vec!["novo"]);
    assert!(s.ticket(&open.id).unwrap().is_some(), "open tickets stay");
    // Consent history survives retention.
    s.record_consent("startup_apps", true, "1", NOW).unwrap();
    s.purge(NOW + 1_000 * DAY, &cfg).unwrap();
    assert!(s.consents().unwrap().contains("startup_apps"));
}

#[test]
fn fr003_file_is_created_reopened_and_uses_wal() {
    let dir = TempDir::new("reopen");
    let (mut s, outcome) = Store::open(&dir.db(), NOW).unwrap();
    assert_eq!(outcome, OpenOutcome::Created);
    let t = s.create_ticket(ProblemFamily::Connectivity, NOW).unwrap();
    drop(s);
    let (s, outcome) = Store::open(&dir.db(), NOW).unwrap();
    assert_eq!(outcome, OpenOutcome::Opened);
    assert_eq!(s.schema_version().unwrap(), SCHEMA_VERSION);
    assert!(s.ticket(&t.id).unwrap().is_some());
    drop(s);
    let mode: String = rusqlite_conn(&dir.db())
        .pragma_query_value(None, "journal_mode", |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
}

#[test]
fn us1_5_corrupted_file_is_moved_aside_and_recreated() {
    let dir = TempDir::new("corrupt");
    std::fs::write(
        dir.db(),
        b"this is not a database, just garbage bytes......",
    )
    .unwrap();
    let (mut s, outcome) = Store::open(&dir.db(), NOW).unwrap();
    let OpenOutcome::RecreatedAfterCorruption { moved_to } = outcome else {
        panic!("expected recovery, got {outcome:?}");
    };
    assert!(
        moved_to
            .to_string_lossy()
            .ends_with(&format!(".corrupt-{NOW}"))
    );
    assert_eq!(
        std::fs::read(&moved_to).unwrap(),
        b"this is not a database, just garbage bytes......",
        "bad file kept, not deleted"
    );
    s.create_ticket(ProblemFamily::Storage, NOW).unwrap();
}

#[test]
fn schema_too_new_is_left_untouched() {
    let dir = TempDir::new("newer");
    {
        let conn = rusqlite_conn(&dir.db());
        conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
    }
    let before = std::fs::read(dir.db()).unwrap();
    assert!(matches!(
        Store::open(&dir.db(), NOW),
        Err(StoreError::SchemaTooNew(_))
    ));
    assert_eq!(std::fs::read(dir.db()).unwrap(), before);
}

#[test]
fn unknown_stored_code_is_an_error_not_a_panic() {
    let dir = TempDir::new("unknown");
    let (mut s, _) = Store::open(&dir.db(), NOW).unwrap();
    let t = s.create_ticket(ProblemFamily::Storage, NOW).unwrap();
    drop(s);
    {
        let conn = rusqlite_conn(&dir.db());
        conn.execute("UPDATE ticket SET state = 'escalated'", [])
            .unwrap();
    }
    let (s, _) = Store::open(&dir.db(), NOW).unwrap();
    assert!(matches!(
        s.ticket(&t.id),
        Err(StoreError::UnknownCode { what: "state", .. })
    ));
}

fn rusqlite_conn(path: &std::path::Path) -> rusqlite::Connection {
    rusqlite::Connection::open(path).unwrap()
}
