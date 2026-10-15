// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 009 US4: everything the app keeps about the person, readable, and a way to erase it all.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::{Result, Store};

/// Format tag in the export header.
pub const EXPORT_FORMAT: &str = "export@1";

fn date(ts: i64) -> String {
    hmd_core::report::utc_date(ts)
}

impl Store {
    /// A Markdown file with every ticket (history, sessions, findings, messages, reports, plans
    /// and executions), consents and the audit trail. Nothing is sent anywhere.
    pub fn export_markdown(&self, now: i64) -> Result<String> {
        let mut s = String::new();
        // Writing into a String cannot fail.
        let _ = writeln!(s, "<!-- {EXPORT_FORMAT} -->");
        let _ = writeln!(s, "# Meus dados no HelpMyDesk\n");
        let _ = writeln!(
            s,
            "Exportado em {} (UTC). Tudo isto fica só neste computador.\n",
            date(now)
        );

        let _ = writeln!(s, "## Permissões\n");
        let consents = self.consent_history()?;
        if consents.is_empty() {
            let _ = writeln!(s, "Nenhuma permissão registrada.\n");
        } else {
            for (scope, granted, version, at) in consents {
                let _ = writeln!(
                    s,
                    "- {}: `{scope}` {} (aviso versão {version})",
                    date(at),
                    if granted { "concedida" } else { "retirada" }
                );
            }
            s.push('\n');
        }

        let _ = writeln!(s, "## Chamados\n");
        let tickets = self.tickets()?;
        if tickets.is_empty() {
            let _ = writeln!(s, "Nenhum chamado.\n");
        }
        for t in tickets {
            let _ = writeln!(
                s,
                "### Chamado {} · {} · {}\n",
                t.id,
                t.family.code(),
                t.state.code()
            );
            let _ = writeln!(s, "- Aberto em {}", date(t.created_at));
            for h in self.history(&t.id)? {
                let _ = writeln!(
                    s,
                    "- {}: `{}` → `{}`{}",
                    date(h.at),
                    h.event,
                    h.to.code(),
                    h.trigger
                        .map_or(String::new(), |r| format!(" ({})", r.code()))
                );
            }
            for session in self.sessions(&t.id)? {
                let codes: Vec<String> = session.findings.iter().map(|f| f.code.clone()).collect();
                let _ = writeln!(
                    s,
                    "- Diagnóstico em {}: `{}` com `{}` ({}); achados: {}",
                    date(session.started_at),
                    session.diagnosis_id,
                    session.playbook,
                    session.confidence,
                    if codes.is_empty() {
                        "nenhum".to_string()
                    } else {
                        codes.join(", ")
                    }
                );
            }
            for plan in self.plans(&t.id)? {
                for e in self.executions(plan)? {
                    let _ = writeln!(
                        s,
                        "- Correção `{}` em {}: `{}`",
                        e.capability,
                        date(e.started_at),
                        e.state.code()
                    );
                }
            }
            let messages = self.messages(&t.id)?;
            if !messages.is_empty() {
                let _ = writeln!(s, "\nMensagens:\n");
                for m in messages {
                    let _ = writeln!(s, "> {} · {}:", date(m.created_at), m.author);
                    for line in m.body.lines() {
                        let _ = writeln!(s, "> {line}");
                    }
                    s.push('\n');
                }
            }
            for r in self.reports(&t.id)? {
                let _ = writeln!(
                    s,
                    "\nRelatório {} ({}), criado em {}:\n",
                    r.id,
                    r.trigger.code(),
                    date(r.created_at)
                );
                let _ = writeln!(s, "````markdown\n{}\n````", r.content.trim_end());
            }
            s.push('\n');
        }

        let _ = writeln!(s, "## Registro de auditoria\n");
        let check = self.verify_audit_chain()?;
        let _ = writeln!(
            s,
            "{} eventos; cadeia {}.\n",
            check.events,
            match check.broken_at {
                None => "íntegra".to_string(),
                Some(id) => format!("quebrada no evento {id}"),
            }
        );
        for e in self.audit_events()? {
            let _ = writeln!(
                s,
                "- {} {} `{}` {} → {}",
                date(e.occurred_at),
                e.actor,
                e.action,
                e.subject_ref,
                e.outcome
            );
        }
        Ok(s)
    }

    fn consent_history(&self) -> Result<Vec<(String, bool, String, i64)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT scope, granted, notice_version, at FROM consent_record ORDER BY id")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    fn plans(&self, ticket: &str) -> Result<Vec<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM action_plan WHERE ticket_id = ?1 ORDER BY id")?;
        let rows = stmt.query_map([crate::ticket_id(ticket)?], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

/// Spec 009 US4 "apagar tudo": removes the database at `path` and every file the store made next
/// to it (`-wal`, `-shm`, `.bak-v*`, `.corrupt-*`). Close the [`Store`] first. Returns what was
/// removed. Files that are not the store's are never touched.
pub fn erase_all(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Ok(Vec::new());
    };
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut removed = Vec::new();
    if !dir.exists() {
        return Ok(removed);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let Some(file) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let ours = file == name
            || file == format!("{name}-wal")
            || file == format!("{name}-shm")
            || file.starts_with(&format!("{name}.bak-v"))
            || file.starts_with(&format!("{name}.corrupt-"));
        if ours && entry.file_type()?.is_file() {
            std::fs::remove_file(entry.path())?;
            removed.push(entry.path());
        }
    }
    removed.sort();
    Ok(removed)
}
