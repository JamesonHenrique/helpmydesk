// SPDX-License-Identifier: GPL-3.0-or-later
//! UI bridge (spec 013): the desktop shell exposes one Tauri command per [`Backend`] method and
//! passes the result to the Angular UI as JSON. Everything here is plain Rust with serde, so it is
//! tested without a webview. Views carry catalog texts and codes only; no probe evidence text.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::path::{Path, PathBuf};

use hmd_app::{App, AppError, DiagnosisView, ExecutionOutcome, PlanView, Startup, StepView};
use hmd_core::{LOCALE, ProbeOutcome, confidence_phrase};
use hmd_domain::{ProblemFamily, Ticket, Timestamp};
use hmd_policy::{AutonomyMode, Confidence};
use hmd_probes::{ProbeError, ProbeOutput};
use hmd_store::OpenOutcome;
use serde::Serialize;

/// Error sent to the UI: a stable code plus a technical message (the UI shows its own text).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IpcError {
    pub code: String,
    pub message: String,
}

impl From<AppError> for IpcError {
    fn from(e: AppError) -> Self {
        IpcError {
            code: e.reason_code().to_string(),
            message: e.to_string(),
        }
    }
}

impl IpcError {
    fn new(code: &str, message: impl Into<String>) -> IpcError {
        IpcError {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

pub type IpcResult<T> = Result<T, IpcError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketDto {
    pub id: String,
    pub family: String,
    pub family_title: String,
    pub state: String,
    pub reopen_count: u32,
    pub last_trigger: Option<String>,
    pub created_at: Timestamp,
    pub last_changed_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartDto {
    pub ticket: TicketDto,
    pub reused: bool,
    pub reopened: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeRunDto {
    pub probe_id: String,
    pub title: String,
    /// `ok`, `found`, `failed` or `skipped`.
    pub status: String,
    pub findings: Vec<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepDto {
    pub step_id: String,
    pub capability: String,
    pub title: String,
    pub risk: String,
    pub undo: String,
    pub requires_elevation: bool,
    pub per_action_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDto {
    pub plan_id: i64,
    pub plan_hash: String,
    pub steps: Vec<StepDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosisDto {
    pub ticket: TicketDto,
    pub session_id: i64,
    pub diagnosis_id: String,
    pub confidence: String,
    pub confidence_phrase: String,
    pub title: String,
    pub explanation: String,
    pub steps: Vec<String>,
    pub is_fallback: bool,
    pub trigger: Option<String>,
    pub probes: Vec<ProbeRunDto>,
    pub plan: Option<PlanDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalDto {
    pub nonce: i64,
    pub plan_hash: String,
    pub step_ids: Vec<String>,
    pub expires_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutcomeDto {
    /// `verified`, `not_available` or `not_resolved`.
    pub kind: String,
    pub step_id: Option<String>,
    pub trigger: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportDto {
    pub id: i64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupDto {
    /// `created`, `opened`, `migrated` or `recreated` (the old file was moved aside).
    pub opened: String,
    pub interrupted: usize,
    pub housekept: usize,
}

/// "Meu computador": OS facts only (never host or user names), from the policy-authorized
/// `sys.info.basic` probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemDto {
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub arch: Option<String>,
    pub catalog_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub autonomy: String,
    pub consents: Vec<String>,
}

fn family_title(f: ProblemFamily) -> String {
    hmd_catalog::embedded()
        .ok()
        .and_then(|c| c.playbook_for(f))
        .and_then(|p| p.title.get(LOCALE))
        .cloned()
        .unwrap_or_else(|| f.code().to_string())
}

fn probe_title(id: &str) -> String {
    hmd_catalog::embedded()
        .ok()
        .and_then(|c| c.meta.get(id))
        .and_then(|m| m.copy.get(LOCALE))
        .map_or_else(|| id.to_string(), |c| c.title.clone())
}

pub fn ticket_dto(t: &Ticket) -> TicketDto {
    TicketDto {
        id: t.id.clone(),
        family: t.family.code().to_string(),
        family_title: family_title(t.family),
        state: t.state.code().to_string(),
        reopen_count: t.reopen_count,
        last_trigger: t.last_trigger.map(|r| r.code().to_string()),
        created_at: t.created_at,
        last_changed_at: t.last_changed_at,
    }
}

fn confidence_code(c: Confidence) -> &'static str {
    match c {
        Confidence::Unsure => "unsure",
        Confidence::Probable => "probable",
        Confidence::AlmostCertain => "almost_certain",
    }
}

fn step_dto(s: &StepView) -> StepDto {
    StepDto {
        step_id: s.step_id.clone(),
        capability: s.capability.clone(),
        title: s.title.clone(),
        risk: s.risk.clone(),
        undo: s.undo.clone(),
        requires_elevation: s.requires_elevation,
        per_action_approval: s.per_action_approval,
    }
}

fn plan_dto(p: &PlanView) -> PlanDto {
    PlanDto {
        plan_id: p.plan_id,
        plan_hash: p.plan_hash.clone(),
        steps: p.steps.iter().map(step_dto).collect(),
    }
}

pub fn diagnosis_dto(v: &DiagnosisView) -> DiagnosisDto {
    let d = &v.diagnosis;
    DiagnosisDto {
        ticket: ticket_dto(&v.ticket),
        session_id: v.session_id,
        diagnosis_id: d.diagnosis_id.clone(),
        confidence: confidence_code(d.confidence).to_string(),
        confidence_phrase: confidence_phrase(d.confidence).to_string(),
        title: d.copy.title.clone(),
        explanation: d.copy.explanation.clone(),
        steps: d.copy.steps.clone(),
        is_fallback: d.is_fallback,
        trigger: v.trigger.map(|t| t.code().to_string()),
        probes: d
            .probes
            .iter()
            .map(|p| {
                let (status, findings, reason) = match &p.outcome {
                    ProbeOutcome::Ran(out) if out.findings.is_empty() => ("ok", vec![], None),
                    ProbeOutcome::Ran(out) => (
                        "found",
                        out.findings.iter().map(|f| f.code.to_string()).collect(),
                        None,
                    ),
                    ProbeOutcome::Failed(code) => ("failed", vec![], Some(code.to_string())),
                    ProbeOutcome::NotAuthorized(code) => {
                        ("skipped", vec![], Some(code.to_string()))
                    }
                };
                ProbeRunDto {
                    probe_id: p.probe_id.clone(),
                    title: probe_title(&p.probe_id),
                    status: status.to_string(),
                    findings,
                    reason,
                }
            })
            .collect(),
        plan: v.plan.as_ref().map(plan_dto),
    }
}

fn startup_dto(s: &Startup) -> StartupDto {
    StartupDto {
        opened: match s.open {
            OpenOutcome::Created => "created",
            OpenOutcome::Opened => "opened",
            OpenOutcome::Migrated { .. } => "migrated",
            OpenOutcome::RecreatedAfterCorruption { .. } => "recreated",
        }
        .to_string(),
        interrupted: s.interrupted,
        housekept: s.housekept,
    }
}

fn autonomy_from(code: &str) -> Option<AutonomyMode> {
    match code {
        "observe_only" => Some(AutonomyMode::ObserveOnly),
        "ask_always" => Some(AutonomyMode::AskAlways),
        "auto_simple" => Some(AutonomyMode::AutoSimple),
        _ => None,
    }
}

fn autonomy_code(m: AutonomyMode) -> &'static str {
    match m {
        AutonomyMode::ObserveOnly => "observe_only",
        AutonomyMode::AskAlways => "ask_always",
        AutonomyMode::AutoSimple => "auto_simple",
    }
}

/// Runs a probe by catalog ID (production: [`hmd_probes::run`]).
pub type ProbeRunner = Box<dyn Fn(&str) -> Result<ProbeOutput, ProbeError> + Send + Sync>;
/// Current time in Unix seconds (production: the system clock).
pub type Clock = Box<dyn Fn() -> Timestamp + Send + Sync>;

/// One instance per running app, behind a mutex in the shell.
pub struct Backend {
    app: Option<App>,
    path: Option<PathBuf>,
    runner: ProbeRunner,
    clock: Clock,
}

/// The system clock as Unix seconds (0 if it is before 1970).
pub fn system_clock() -> Timestamp {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

impl Backend {
    /// Opens the history file (created if needed).
    pub fn open(
        path: PathBuf,
        runner: ProbeRunner,
        clock: Clock,
    ) -> IpcResult<(Backend, StartupDto)> {
        let (app, startup) = App::open(&path, clock())?;
        Ok((
            Backend {
                app: Some(app),
                path: Some(path),
                runner,
                clock,
            },
            startup_dto(&startup),
        ))
    }

    /// An in-memory history (tests, "não guardar histórico").
    pub fn in_memory(runner: ProbeRunner, clock: Clock) -> IpcResult<Backend> {
        let (app, _) = App::open_in_memory(clock())?;
        Ok(Backend {
            app: Some(app),
            path: None,
            runner,
            clock,
        })
    }

    /// Tests inject fake fixes and platforms; the shell never calls this.
    pub fn map_app(&mut self, f: impl FnOnce(App) -> App) {
        self.app = self.app.take().map(f);
    }

    /// After a failed "Apagar tudo" the history may be closed; the next command reopens it, so a
    /// passing failure does not need an app restart (spec 016, P2).
    fn reopen_if_closed(&mut self) {
        if self.app.is_none()
            && let Some(path) = &self.path
        {
            self.app = App::open(path, (self.clock)()).ok().map(|(app, _)| app);
        }
    }

    fn app(&mut self) -> IpcResult<&mut App> {
        self.reopen_if_closed();
        self.app
            .as_mut()
            .ok_or_else(|| IpcError::new("NOT_OPEN", "history is closed"))
    }

    fn family(code: &str) -> IpcResult<ProblemFamily> {
        ProblemFamily::from_code(code)
            .ok_or_else(|| IpcError::new("UNKNOWN_FAMILY", format!("`{code}`")))
    }

    pub fn start_ticket(&mut self, family: &str) -> IpcResult<StartDto> {
        let family = Backend::family(family)?;
        let now = (self.clock)();
        let s = self.app()?.start_ticket(family, now)?;
        Ok(StartDto {
            ticket: ticket_dto(&s.ticket),
            reused: s.reused,
            reopened: s.reopened,
        })
    }

    pub fn diagnose(&mut self, ticket_id: &str) -> IpcResult<DiagnosisDto> {
        let now = (self.clock)();
        self.reopen_if_closed();
        let app = self
            .app
            .as_mut()
            .ok_or_else(|| IpcError::new("NOT_OPEN", "history is closed"))?;
        let runner = &self.runner;
        let view = app.diagnose(ticket_id, now, |id| runner(id))?;
        Ok(diagnosis_dto(&view))
    }

    pub fn plan(&mut self, ticket_id: &str, plan_id: i64) -> IpcResult<PlanDto> {
        Ok(plan_dto(&self.app()?.pending_plan(ticket_id, plan_id)?))
    }

    pub fn approve(
        &mut self,
        ticket_id: &str,
        plan_id: i64,
        step_ids: &[String],
    ) -> IpcResult<ApprovalDto> {
        let now = (self.clock)();
        let ids: Vec<&str> = step_ids.iter().map(String::as_str).collect();
        let a = self.app()?.approve(ticket_id, plan_id, &ids, now)?;
        Ok(ApprovalDto {
            nonce: a.nonce,
            plan_hash: a.plan_hash,
            step_ids: a.approved_step_ids.into_iter().collect(),
            expires_at: a.expires_at,
        })
    }

    pub fn decline(&mut self, ticket_id: &str, plan_id: i64) -> IpcResult<()> {
        let now = (self.clock)();
        Ok(self.app()?.decline(ticket_id, plan_id, now)?)
    }

    pub fn execute(&mut self, ticket_id: &str, plan_id: i64) -> IpcResult<OutcomeDto> {
        let now = (self.clock)();
        self.reopen_if_closed();
        let app = self
            .app
            .as_mut()
            .ok_or_else(|| IpcError::new("NOT_OPEN", "history is closed"))?;
        let runner = &self.runner;
        let outcome = app.execute(ticket_id, plan_id, now, |id| runner(id))?;
        Ok(match outcome {
            ExecutionOutcome::Verified => OutcomeDto {
                kind: "verified".into(),
                step_id: None,
                trigger: None,
            },
            ExecutionOutcome::NotAvailable { step_id } => OutcomeDto {
                kind: "not_available".into(),
                step_id: Some(step_id),
                trigger: None,
            },
            ExecutionOutcome::NotResolved { trigger } => OutcomeDto {
                kind: "not_resolved".into(),
                step_id: None,
                trigger: Some(trigger.code().to_string()),
            },
        })
    }

    pub fn confirm(&mut self, ticket_id: &str, fixed: bool) -> IpcResult<TicketDto> {
        let now = (self.clock)();
        Ok(ticket_dto(&self.app()?.confirm(ticket_id, fixed, now)?))
    }

    pub fn make_report(&mut self, ticket_id: &str, symptom: Option<&str>) -> IpcResult<ReportDto> {
        let now = (self.clock)();
        self.reopen_if_closed();
        let app = self
            .app
            .as_mut()
            .ok_or_else(|| IpcError::new("NOT_OPEN", "history is closed"))?;
        let runner = &self.runner;
        let (id, text) = app.make_report(ticket_id, symptom, None, now, |p| runner(p))?;
        Ok(ReportDto { id, text })
    }

    pub fn edit_report(&mut self, report_id: i64, text: &str) -> IpcResult<()> {
        let now = (self.clock)();
        Ok(self.app()?.edit_report(report_id, text, now)?)
    }

    /// "Salvar arquivo": writes the report the person sees (with their edits) into `dir`
    /// (the shell passes the Downloads folder), never overwriting a file, and marks it exported.
    pub fn save_report(&mut self, report_id: i64, text: &str, dir: &Path) -> IpcResult<String> {
        let now = (self.clock)();
        let app = self.app()?;
        app.edit_report(report_id, text, now)?;
        let date = hmd_core::report::utc_date(now);
        let path = (0..100)
            .map(|n| {
                let suffix = if n == 0 {
                    String::new()
                } else {
                    format!("-{n}")
                };
                dir.join(format!("relatorio-helpmydesk-{date}{suffix}.md"))
            })
            .find(|p| !p.exists())
            .ok_or_else(|| IpcError::new("SAVE_FAILED", "too many reports with this date"))?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| IpcError::new("SAVE_FAILED", e.to_string()))?;
        std::io::Write::write_all(&mut file, text.as_bytes())
            .map_err(|e| IpcError::new("SAVE_FAILED", e.to_string()))?;
        self.app()?.report_exported(report_id, now)?;
        Ok(path.display().to_string())
    }

    pub fn history(&mut self) -> IpcResult<Vec<TicketDto>> {
        Ok(self.app()?.history()?.iter().map(ticket_dto).collect())
    }

    pub fn system(&mut self) -> IpcResult<SystemDto> {
        let catalog = hmd_catalog::embedded()
            .map_err(|e| IpcError::new("INVALID_CATALOG", format!("{} error(s)", e.len())))?;
        let runner = &self.runner;
        let summary = hmd_core::report::system_summary_with(
            catalog,
            &hmd_core::DiagnoseOptions::default(),
            |id| runner(id),
        )
        .unwrap_or_default();
        Ok(SystemDto {
            os_name: summary.os_name,
            os_version: summary.os_version,
            kernel_version: summary.kernel_version,
            arch: summary.arch,
            catalog_version: catalog.version.clone(),
        })
    }

    pub fn settings(&mut self) -> IpcResult<SettingsDto> {
        let app = self.app()?;
        Ok(SettingsDto {
            autonomy: autonomy_code(app.autonomy()?).to_string(),
            consents: app.consents()?.into_iter().collect(),
        })
    }

    pub fn set_autonomy(&mut self, code: &str) -> IpcResult<()> {
        let mode = autonomy_from(code)
            .ok_or_else(|| IpcError::new("UNKNOWN_MODE", format!("`{code}`")))?;
        let now = (self.clock)();
        Ok(self.app()?.set_autonomy(mode, now)?)
    }

    pub fn set_consent(&mut self, scope: &str, granted: bool) -> IpcResult<()> {
        let now = (self.clock)();
        Ok(self.app()?.set_consent(scope, granted, now)?)
    }

    pub fn export_all(&mut self) -> IpcResult<String> {
        let now = (self.clock)();
        Ok(self.app()?.export_all(now)?)
    }

    /// "Apagar tudo": removes the history files and starts an empty history.
    pub fn erase_all(&mut self) -> IpcResult<usize> {
        self.reopen_if_closed();
        let app = self
            .app
            .take()
            .ok_or_else(|| IpcError::new("NOT_OPEN", "history is closed"))?;
        let now = (self.clock)();
        match self.path.clone() {
            Some(path) => {
                let removed = app
                    .erase_all(&path)
                    .map_err(|e| IpcError::new("ERASE_FAILED", e.to_string()))?;
                let (fresh, _) = App::open(&path, now)?;
                self.app = Some(fresh);
                Ok(removed.len())
            }
            None => {
                drop(app);
                self.app = Some(App::open_in_memory(now)?.0);
                Ok(0)
            }
        }
    }
}
