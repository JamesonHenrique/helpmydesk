// SPDX-License-Identifier: GPL-3.0-or-later
//! Plans, approvals and the write-ahead execution journal (spec 008 FR-005).

use std::collections::BTreeSet;

use hmd_domain::{ExecutionState, Timestamp, execution};
use hmd_executor::{ActionApproval, EvaluatedPlan, ExecutionStart, Journal, JournalError};
use hmd_policy::AutonomyMode;
use rusqlite::{OptionalExtension, params};

use crate::{Result, Store, StoreError, parse, platform_code, ticket_id};

fn mode_code(m: AutonomyMode) -> &'static str {
    match m {
        AutonomyMode::ObserveOnly => "observe_only",
        AutonomyMode::AskAlways => "ask_always",
        AutonomyMode::AutoSimple => "auto_simple",
    }
}

fn mode_from(code: &str) -> Option<AutonomyMode> {
    [
        AutonomyMode::ObserveOnly,
        AutonomyMode::AskAlways,
        AutonomyMode::AutoSimple,
    ]
    .into_iter()
    .find(|m| mode_code(*m) == code)
}

/// One row of the journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredExecution {
    pub id: i64,
    pub approval_nonce: i64,
    pub step_id: String,
    pub capability: String,
    pub platform: String,
    pub state: ExecutionState,
    pub started_at: Timestamp,
    pub finished_at: Option<Timestamp>,
    pub result: Option<String>,
    pub verification: Option<String>,
    pub rollback_state: Option<String>,
}

impl Store {
    /// Stores the evaluated plan the person is about to see; the same plan is stored once.
    pub fn save_plan(
        &mut self,
        ticket: &str,
        plan: &EvaluatedPlan<'_>,
        now: Timestamp,
    ) -> Result<i64> {
        let hash = plan.hash();
        let row_id = ticket_id(ticket)?;
        if let Some(id) = self
            .conn
            .query_row(
                "SELECT id FROM action_plan WHERE plan_hash = ?1 AND ticket_id = ?2",
                params![hash, row_id],
                |r| r.get(0),
            )
            .optional()?
        {
            return Ok(id);
        }
        self.conn.execute(
            "INSERT INTO action_plan (ticket_id, session_ref, plan_hash, catalog_version, canonical,
                 created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                row_id,
                plan.proposal.session_id,
                hash,
                plan.catalog_version,
                hmd_executor::canonical(plan.proposal, plan.decision, plan.catalog_version),
                now
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Records the person's approval of `step_ids`; the row ID becomes the nonce (FR-002).
    /// Refused when `plan` is not the stored plan `plan_id`.
    pub fn approve(
        &mut self,
        plan_id: i64,
        plan: &EvaluatedPlan<'_>,
        step_ids: &[&str],
        mode: AutonomyMode,
        now: Timestamp,
    ) -> Result<ActionApproval> {
        let stored: Option<String> = self
            .conn
            .query_row(
                "SELECT plan_hash FROM action_plan WHERE id = ?1",
                [plan_id],
                |r| r.get(0),
            )
            .optional()?;
        let hash = plan.hash();
        if stored.as_deref() != Some(hash.as_str()) {
            return Err(StoreError::NotFound);
        }
        let draft = ActionApproval::new(
            plan,
            step_ids.iter().map(|s| (*s).to_string()),
            mode,
            now,
            0,
        );
        let joined = draft
            .approved_step_ids
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",");
        self.conn.execute(
            "INSERT INTO action_approval (plan_id, plan_hash, step_ids, mode, decided_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                plan_id,
                draft.plan_hash,
                joined,
                mode_code(mode),
                draft.decided_at,
                draft.expires_at
            ],
        )?;
        Ok(ActionApproval {
            nonce: self.conn.last_insert_rowid(),
            ..draft
        })
    }

    pub fn approval(&self, nonce: i64) -> Result<Option<ActionApproval>> {
        let row = self
            .conn
            .query_row(
                "SELECT plan_hash, step_ids, mode, decided_at, expires_at FROM action_approval
                 WHERE id = ?1",
                [nonce],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, i64>(4)?,
                    ))
                },
            )
            .optional()?;
        row.map(|(plan_hash, steps, mode, decided_at, expires_at)| {
            Ok(ActionApproval {
                plan_hash,
                approved_step_ids: steps
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect(),
                mode: parse("mode", mode, mode_from)?,
                decided_at,
                expires_at,
                nonce,
            })
        })
        .transpose()
    }

    pub fn executions(&self, plan_id: i64) -> Result<Vec<StoredExecution>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, approval_id, step_id, capability, platform, state, started_at, finished_at,
                    result, verification, rollback_state
             FROM action_execution WHERE plan_id = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([plan_id], |r| {
            Ok((
                (
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                ),
                (
                    r.get::<_, i64>(6)?,
                    r.get::<_, Option<i64>>(7)?,
                    r.get::<_, Option<String>>(8)?,
                    r.get::<_, Option<String>>(9)?,
                    r.get::<_, Option<String>>(10)?,
                ),
            ))
        })?;
        rows.map(|row| {
            let ((id, approval_nonce, step_id, capability, platform, state), rest) = row?;
            let (started_at, finished_at, result, verification, rollback_state) = rest;
            Ok(StoredExecution {
                id,
                approval_nonce,
                step_id,
                capability,
                platform,
                state: parse("execution_state", state, ExecutionState::from_code)?,
                started_at,
                finished_at,
                result,
                verification,
                rollback_state,
            })
        })
        .collect()
    }

    /// US2.2: at app start, every `started` execution becomes `unknown`. Returns how many.
    pub fn recover_interrupted(&mut self, now: Timestamp) -> Result<usize> {
        let to = execution::on_startup(ExecutionState::Started);
        Ok(self.conn.execute(
            "UPDATE action_execution SET state = ?1, finished_at = ?2, result = 'INTERRUPTED'
             WHERE state = ?3",
            params![to.code(), now, ExecutionState::Started.code()],
        )?)
    }

    /// US2.3: capability IDs (without version) with an execution in `unknown`, for
    /// `SessionCounters::unknown_executions`.
    pub fn unknown_capabilities(&self) -> Result<BTreeSet<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT capability FROM action_execution WHERE state = ?1")?;
        let rows = stmt.query_map([ExecutionState::Unknown.code()], |r| r.get::<_, String>(0))?;
        let mut out = BTreeSet::new();
        for cap in rows {
            let cap = cap?;
            let id = cap.split_once('@').map_or(cap.as_str(), |(id, _)| id);
            out.insert(id.to_string());
        }
        Ok(out)
    }

    /// Plan v1 V.10: the capability's state probe said whether the effect happened. This is the
    /// only way out of `unknown`; it never re-runs anything.
    pub fn clarify_unknown(
        &mut self,
        execution: i64,
        effect_happened: bool,
        now: Timestamp,
    ) -> Result<()> {
        let to = if effect_happened {
            ExecutionState::Succeeded
        } else {
            ExecutionState::Failed
        };
        self.advance_state(execution, to, Some("CLARIFIED_BY_STATE_PROBE"), now)
    }

    fn advance_state(
        &mut self,
        execution: i64,
        to: ExecutionState,
        note: Option<&str>,
        at: Timestamp,
    ) -> Result<()> {
        let current: String = self
            .conn
            .query_row(
                "SELECT state FROM action_execution WHERE id = ?1",
                [execution],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(StoreError::NotFound)?;
        let from = parse("execution_state", current, ExecutionState::from_code)?;
        let to = execution::transition(from, to)?;
        let column = match to {
            ExecutionState::VerifiedOk | ExecutionState::VerifiedFailed => "verification",
            ExecutionState::RolledBack => "rollback_state",
            _ => "result",
        };
        let note = note.map(str::to_string).or_else(|| match to {
            ExecutionState::VerifiedOk
            | ExecutionState::VerifiedFailed
            | ExecutionState::RolledBack => Some(to.code().to_string()),
            _ => None,
        });
        let changed = self.conn.execute(
            &format!(
                "UPDATE action_execution SET state = ?1, finished_at = ?2,
                     {column} = coalesce(?3, {column})
                 WHERE id = ?4 AND state = ?5"
            ),
            params![to.code(), at, note, execution, from.code()],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::ConcurrentChange)
        }
    }
}

fn journal_error(e: StoreError) -> JournalError {
    JournalError(e.to_string())
}

impl Journal for Store {
    fn key_used(&self, idempotency_key: &str) -> std::result::Result<bool, JournalError> {
        self.conn
            .query_row(
                "SELECT 1 FROM action_execution WHERE idempotency_key = ?1",
                [idempotency_key],
                |_| Ok(()),
            )
            .optional()
            .map(|r| r.is_some())
            .map_err(|e| journal_error(e.into()))
    }

    fn begin(&mut self, start: &ExecutionStart<'_>) -> std::result::Result<i64, JournalError> {
        self.conn
            .execute(
                "INSERT INTO action_execution (plan_id, approval_id, step_id, capability, platform,
                     idempotency_key, state, started_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    start.plan_ref,
                    start.approval_nonce,
                    start.step_id,
                    start.capability,
                    platform_code(start.platform),
                    start.idempotency_key,
                    ExecutionState::Started.code(),
                    start.at
                ],
            )
            .map_err(|e| journal_error(e.into()))?;
        Ok(self.conn.last_insert_rowid())
    }

    fn advance(
        &mut self,
        execution: i64,
        to: ExecutionState,
        note: Option<&str>,
        at: Timestamp,
    ) -> std::result::Result<(), JournalError> {
        self.advance_state(execution, to, note, at)
            .map_err(journal_error)
    }
}
