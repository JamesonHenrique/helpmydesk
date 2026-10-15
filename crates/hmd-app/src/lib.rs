// SPDX-License-Identifier: GPL-3.0-or-later
//! Application service (spec 012): the use cases the desktop UI calls, one method each. It ties
//! the catalog, diagnosis, local history, executor and report together and owns the lifecycle
//! rules that span them (deduplication, reopening, REP-08, approval before execution, proof after
//! execution). The UI never talks to the store or the executor directly.
//!
//! Probe runners are injected (`run_probe`), so every use case is testable with fakes; the app
//! passes [`hmd_probes::run`].
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use hmd_catalog::{LoadError, LoadedCatalog};
use hmd_core::report::{ReportInput, redact_free_text, render, system_summary_with};
use hmd_core::{
    CoreError, DiagnoseOptions, DiagnosisReport, LOCALE, current_platform, diagnose_with,
    run_authorized_probe,
};
use hmd_domain::{
    HousekeepingConfig, ProblemFamily, ReportTrigger, RetentionConfig, SessionPhase, Ticket,
    TicketEvent, TicketState, Timestamp, due_event, repeated_reopen_trigger,
};
use hmd_executor::{
    ActionApproval, EvaluatedPlan, ExecError, ExecuteInput, Registry, execute_step, verify_step,
};
use hmd_policy::{
    ActionPlanProposal, ApprovalRequirement, AutonomyMode, Platform, PolicyDecision, StepVerdict,
};
use hmd_probes::{ProbeError, ProbeOutput};
use hmd_store::{OpenOutcome, PurgeStats, Store, StoreError};

/// Setting key for the autonomy mode.
pub const AUTONOMY_KEY: &str = "autonomy_mode";

#[derive(Debug)]
pub enum AppError {
    /// The embedded catalog is invalid: the app must stay in guidance-only mode.
    Catalog(usize),
    Store(StoreError),
    Core(CoreError),
    Exec(ExecError),
    NoSuchTicket,
    /// No plan waiting for approval/execution in this run of the app (plans are re-proposed after
    /// a restart; approvals expire in 10 minutes anyway).
    NoPendingPlan,
    /// A report needs a diagnosis made in this run of the app.
    NoDiagnosis,
    /// The ticket's state does not allow this use case.
    InvalidState(&'static str),
}

impl AppError {
    pub fn reason_code(&self) -> &'static str {
        match self {
            AppError::Catalog(_) => "INVALID_CATALOG",
            AppError::Store(e) => e.reason_code(),
            AppError::Core(_) => "CORE_ERROR",
            AppError::Exec(e) => e.reason_code(),
            AppError::NoSuchTicket => "NO_SUCH_TICKET",
            AppError::NoPendingPlan => "NO_PENDING_PLAN",
            AppError::NoDiagnosis => "NO_DIAGNOSIS",
            AppError::InvalidState(_) => "INVALID_STATE",
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Store(e) => write!(f, "{e}"),
            AppError::Core(e) => write!(f, "{e}"),
            AppError::Exec(e) => write!(f, "{e}"),
            AppError::Catalog(n) => write!(f, "INVALID_CATALOG: {n} error(s)"),
            AppError::InvalidState(state) => write!(f, "INVALID_STATE: {state}"),
            other => write!(f, "{}", other.reason_code()),
        }
    }
}

impl std::error::Error for AppError {}

impl From<StoreError> for AppError {
    fn from(e: StoreError) -> Self {
        AppError::Store(e)
    }
}

impl From<CoreError> for AppError {
    fn from(e: CoreError) -> Self {
        AppError::Core(e)
    }
}

impl From<ExecError> for AppError {
    fn from(e: ExecError) -> Self {
        AppError::Exec(e)
    }
}

impl From<hmd_domain::DomainError> for AppError {
    fn from(e: hmd_domain::DomainError) -> Self {
        AppError::Store(StoreError::Domain(e))
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

/// What happened at startup; the UI tells the person about recovery and migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Startup {
    pub open: OpenOutcome,
    /// Executions interrupted by a crash, now `unknown` (never re-run automatically).
    pub interrupted: usize,
    /// Tickets closed or abandoned by the time rules.
    pub housekept: usize,
    pub purged: PurgeStats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketStart {
    pub ticket: Ticket,
    /// An open ticket of the same family was continued instead of creating another.
    pub reused: bool,
    /// The ticket was reopened (counts towards REP-08).
    pub reopened: bool,
}

/// One proposed step as the approval screen shows it: texts from the catalog only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepView {
    pub step_id: String,
    pub capability: String,
    pub title: String,
    pub risk: String,
    pub undo: String,
    pub requires_elevation: bool,
    pub per_action_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanView {
    pub plan_id: i64,
    pub plan_hash: String,
    pub steps: Vec<StepView>,
}

#[derive(Debug, Clone)]
pub struct DiagnosisView {
    pub ticket: Ticket,
    pub session_id: i64,
    pub diagnosis: DiagnosisReport,
    /// Why a report would be offered (REP-08 wins over the playbook's trigger).
    pub trigger: Option<ReportTrigger>,
    pub plan: Option<PlanView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionOutcome {
    /// Every approved step ran and its verification probe proved it.
    Verified,
    /// A step has no implementation in this build: the person gets the guidance instead.
    NotAvailable { step_id: String },
    /// A step failed, was not proven, or ended `unknown`: the ticket goes to the report.
    NotResolved { trigger: ReportTrigger },
}

struct Pending {
    ticket_id: String,
    view: PlanView,
    proposal: ActionPlanProposal,
    decision: PolicyDecision,
    approval: Option<ActionApproval>,
}

fn autonomy_code(m: AutonomyMode) -> &'static str {
    match m {
        AutonomyMode::ObserveOnly => "observe_only",
        AutonomyMode::AskAlways => "ask_always",
        AutonomyMode::AutoSimple => "auto_simple",
    }
}

/// Use cases over one local history file.
pub struct App {
    store: Store,
    catalog: &'static LoadedCatalog,
    platform: Platform,
    registry: Registry,
    pending: BTreeMap<i64, Pending>,
    last_diagnosis: BTreeMap<String, DiagnosisReport>,
}

fn catalog() -> Result<&'static LoadedCatalog> {
    hmd_catalog::embedded().map_err(|errors: &[LoadError]| AppError::Catalog(errors.len()))
}

impl App {
    /// Opens the history, then: crashed executions → `unknown`, time rules, retention.
    pub fn open(path: &Path, now: Timestamp) -> Result<(App, Startup)> {
        let (store, open) = Store::open(path, now)?;
        App::start(store, open, now)
    }

    pub fn open_in_memory(now: Timestamp) -> Result<(App, Startup)> {
        App::start(Store::open_in_memory()?, OpenOutcome::Created, now)
    }

    fn start(store: Store, open: OpenOutcome, now: Timestamp) -> Result<(App, Startup)> {
        let mut app = App {
            store,
            catalog: catalog()?,
            platform: current_platform(),
            registry: Registry::production(),
            pending: BTreeMap::new(),
            last_diagnosis: BTreeMap::new(),
        };
        let interrupted = app.store.recover_interrupted(now)?;
        let housekept = app.housekeeping(now)?;
        let purged = app.store.purge(now, &RetentionConfig::default())?;
        Ok((
            app,
            Startup {
                open,
                interrupted,
                housekept,
                purged,
            },
        ))
    }

    /// Tests and the lab inject fake implementations and platforms; production never does.
    pub fn with_registry(mut self, registry: Registry) -> App {
        self.registry = registry;
        self
    }

    pub fn with_platform(mut self, platform: Platform) -> App {
        self.platform = platform;
        self
    }

    fn housekeeping(&mut self, now: Timestamp) -> Result<usize> {
        let cfg = HousekeepingConfig::default();
        let mut applied = 0;
        for ticket in self.store.tickets()? {
            let mut current = ticket;
            // Abandoned tickets close in a second step.
            while let Some(event) = due_event(&current, now, &cfg) {
                current = self.store.apply(&current.id, event, now)?;
                applied += 1;
            }
        }
        Ok(applied)
    }

    fn ticket(&self, id: &str) -> Result<Ticket> {
        self.store.ticket(id)?.ok_or(AppError::NoSuchTicket)
    }

    // ----- settings and consent -----

    pub fn autonomy(&self) -> Result<AutonomyMode> {
        let code = self.store.setting(AUTONOMY_KEY)?;
        Ok([
            AutonomyMode::ObserveOnly,
            AutonomyMode::AskAlways,
            AutonomyMode::AutoSimple,
        ]
        .into_iter()
        .find(|m| code.as_deref() == Some(autonomy_code(*m)))
        .unwrap_or_default())
    }

    pub fn set_autonomy(&mut self, mode: AutonomyMode, now: Timestamp) -> Result<()> {
        Ok(self
            .store
            .set_setting(AUTONOMY_KEY, autonomy_code(mode), now)?)
    }

    pub fn set_consent(&mut self, scope: &str, granted: bool, now: Timestamp) -> Result<()> {
        Ok(self.store.set_consent(self.catalog, scope, granted, now)?)
    }

    pub fn consents(&self) -> Result<BTreeSet<String>> {
        Ok(self.store.consents()?)
    }

    // ----- tickets -----

    /// Opens a ticket for `family`, continuing an open one of the same family instead of
    /// duplicating it, and reopening an unresolved or resolved one (Plan v1 VII.2).
    pub fn start_ticket(&mut self, family: ProblemFamily, now: Timestamp) -> Result<TicketStart> {
        let latest = self
            .store
            .tickets()?
            .into_iter()
            .filter(|t| t.family == family)
            .max_by_key(|t| (t.last_changed_at, t.id.parse::<i64>().unwrap_or(0)));
        match latest {
            Some(t)
                if matches!(
                    t.state,
                    TicketState::Open
                        | TicketState::InProgress
                        | TicketState::AwaitingUser
                        | TicketState::Reopened
                        | TicketState::ResolvedPendingConfirmation
                ) =>
            {
                Ok(TicketStart {
                    ticket: t,
                    reused: true,
                    reopened: false,
                })
            }
            Some(t) if matches!(t.state, TicketState::Unresolved | TicketState::Resolved) => {
                let ticket = self.store.apply(&t.id, TicketEvent::Reopen, now)?;
                Ok(TicketStart {
                    ticket,
                    reused: true,
                    reopened: true,
                })
            }
            _ => Ok(TicketStart {
                ticket: self.store.create_ticket(family, now)?,
                reused: false,
                reopened: false,
            }),
        }
    }

    pub fn history(&self) -> Result<Vec<Ticket>> {
        Ok(self.store.tickets()?)
    }

    /// Brings the ticket to `in_progress` for a new diagnosis.
    fn begin_session(&mut self, ticket: &Ticket, now: Timestamp) -> Result<Ticket> {
        let event = match ticket.state {
            TicketState::Open | TicketState::Reopened => Some(TicketEvent::StartSession),
            TicketState::AwaitingUser => Some(TicketEvent::UserResponded),
            TicketState::ResolvedPendingConfirmation => Some(TicketEvent::UserNotFixed),
            TicketState::InProgress => None,
            other => return Err(AppError::InvalidState(other.code())),
        };
        match event {
            Some(e) => Ok(self.store.apply(&ticket.id, e, now)?),
            None => Ok(ticket.clone()),
        }
    }

    fn options(&self, session_ref: String) -> Result<DiagnoseOptions> {
        Ok(DiagnoseOptions {
            platform: self.platform,
            mode: self.autonomy()?,
            consents: self.store.consents()?,
            session_ref,
            unknown_executions: self.store.unknown_capabilities()?,
        })
    }

    fn step_view(&self, verdict: &StepVerdict) -> Option<StepView> {
        let StepVerdict::Accepted {
            step_id,
            capability,
            approval,
            requires_elevation,
            ..
        } = verdict
        else {
            return None;
        };
        let copy = self
            .catalog
            .meta
            .get(capability)
            .and_then(|m| m.copy.get(LOCALE));
        Some(StepView {
            step_id: step_id.clone(),
            capability: capability.clone(),
            title: copy.map_or_else(|| capability.clone(), |c| c.title.clone()),
            risk: copy.and_then(|c| c.risk.clone()).unwrap_or_default(),
            undo: copy.and_then(|c| c.undo.clone()).unwrap_or_default(),
            requires_elevation: *requires_elevation,
            per_action_approval: *approval == ApprovalRequirement::PerAction,
        })
    }

    /// Runs the family's playbook, records the session, and prepares the plan when the Policy
    /// Engine accepts at least one fix (the ticket then waits for the person).
    pub fn diagnose(
        &mut self,
        ticket_id: &str,
        now: Timestamp,
        run_probe: impl Fn(&str) -> std::result::Result<ProbeOutput, ProbeError>,
    ) -> Result<DiagnosisView> {
        let ticket = self.ticket(ticket_id)?;
        let ticket = self.begin_session(&ticket, now)?;
        let opts = self.options(format!("ticket:{}@{now}", ticket.id))?;
        let diagnosis = diagnose_with(self.catalog, ticket.family, &opts, run_probe)?;

        let accepted: Vec<StepView> = diagnosis
            .fix_decision
            .iter()
            .flat_map(|d| d.verdicts.iter())
            .filter_map(|v| self.step_view(v))
            .collect();
        let phase = if accepted.is_empty() {
            SessionPhase::Done
        } else {
            SessionPhase::AwaitingApproval
        };
        let session_id = self.store.record_diagnosis(
            &ticket.id,
            self.catalog,
            &diagnosis,
            self.platform,
            phase,
            now,
            now,
        )?;

        let reopen = repeated_reopen_trigger(&self.store.reopen_times(ticket.family)?, now);
        let trigger = reopen.or(diagnosis.report);

        let mut plan = None;
        let mut ticket = ticket;
        if let (false, Some(proposal), Some(decision)) = (
            accepted.is_empty(),
            diagnosis.fix_proposal.clone(),
            diagnosis.fix_decision.clone(),
        ) {
            let evaluated = EvaluatedPlan {
                proposal: &proposal,
                decision: &decision,
                catalog_version: &self.catalog.version,
            };
            let plan_id = self.store.save_plan(&ticket.id, &evaluated, now)?;
            let view = PlanView {
                plan_id,
                plan_hash: evaluated.hash(),
                steps: accepted,
            };
            plan = Some(view.clone());
            self.pending.insert(
                plan_id,
                Pending {
                    ticket_id: ticket.id.clone(),
                    view,
                    proposal,
                    decision,
                    approval: None,
                },
            );
            ticket = self
                .store
                .apply(&ticket.id, TicketEvent::NeedUserInput, now)?;
        }
        self.last_diagnosis
            .insert(ticket.id.clone(), diagnosis.clone());
        Ok(DiagnosisView {
            ticket,
            session_id,
            diagnosis,
            trigger,
            plan,
        })
    }

    /// The plan waiting for approval, as the approval window shows it.
    pub fn pending_plan(&self, ticket_id: &str, plan_id: i64) -> Result<PlanView> {
        self.pending
            .get(&plan_id)
            .filter(|p| p.ticket_id == ticket_id && p.approval.is_none())
            .map(|p| p.view.clone())
            .ok_or(AppError::NoPendingPlan)
    }

    /// The person approved `step_ids` of the plan on the approval screen (INV-04).
    pub fn approve(
        &mut self,
        ticket_id: &str,
        plan_id: i64,
        step_ids: &[&str],
        now: Timestamp,
    ) -> Result<ActionApproval> {
        let ticket = self.ticket(ticket_id)?;
        let mode = self.autonomy()?;
        let pending = self
            .pending
            .get_mut(&plan_id)
            .filter(|p| p.ticket_id == ticket.id)
            .ok_or(AppError::NoPendingPlan)?;
        let plan = EvaluatedPlan {
            proposal: &pending.proposal,
            decision: &pending.decision,
            catalog_version: &self.catalog.version,
        };
        let approval = self.store.approve(plan_id, &plan, step_ids, mode, now)?;
        pending.approval = Some(approval.clone());
        if ticket.state == TicketState::AwaitingUser {
            self.store
                .apply(&ticket.id, TicketEvent::UserResponded, now)?;
        }
        Ok(approval)
    }

    /// "Não, obrigado": the plan is dropped; the person keeps the guidance and the report option.
    pub fn decline(&mut self, ticket_id: &str, plan_id: i64, now: Timestamp) -> Result<()> {
        let ticket = self.ticket(ticket_id)?;
        self.pending
            .remove(&plan_id)
            .ok_or(AppError::NoPendingPlan)?;
        if ticket.state == TicketState::AwaitingUser {
            self.store
                .apply(&ticket.id, TicketEvent::UserResponded, now)?;
        }
        Ok(())
    }

    /// Runs the approved steps in plan order and proves each one. Stops at the first step that
    /// cannot run or is not proven.
    pub fn execute(
        &mut self,
        ticket_id: &str,
        plan_id: i64,
        now: Timestamp,
        run_probe: impl Fn(&str) -> std::result::Result<ProbeOutput, ProbeError>,
    ) -> Result<ExecutionOutcome> {
        let ticket = self.ticket(ticket_id)?;
        if ticket.state != TicketState::InProgress {
            return Err(AppError::InvalidState(ticket.state.code()));
        }
        let pending = self
            .pending
            .remove(&plan_id)
            .ok_or(AppError::NoPendingPlan)?;
        let approval = pending.approval.as_ref().ok_or(AppError::NoPendingPlan)?;
        let plan = EvaluatedPlan {
            proposal: &pending.proposal,
            decision: &pending.decision,
            catalog_version: &self.catalog.version,
        };
        let opts = self.options(pending.proposal.session_id.clone())?;
        let mut outcome = ExecutionOutcome::Verified;
        for step in pending
            .proposal
            .steps
            .iter()
            .filter(|s| approval.approved_step_ids.contains(&s.step_id))
        {
            let input = ExecuteInput {
                plan,
                plan_ref: plan_id,
                approval,
                step_id: &step.step_id,
                platform: self.platform,
                now,
            };
            let run = match execute_step(&input, &self.registry, &mut self.store) {
                Ok(run) => run,
                Err(ExecError::NotImplemented) => {
                    outcome = ExecutionOutcome::NotAvailable {
                        step_id: step.step_id.clone(),
                    };
                    break;
                }
                Err(e) => return Err(e.into()),
            };
            let trigger = match run.state {
                hmd_domain::ExecutionState::Succeeded => {
                    let variant = self
                        .catalog
                        .catalog
                        .get(&step.capability)
                        .and_then(|c| c.variants.get(&self.platform))
                        .ok_or(AppError::InvalidState("no_variant"))?;
                    let catalog = self.catalog;
                    let verification = verify_step(
                        run.execution,
                        step,
                        variant,
                        &self.registry,
                        &mut self.store,
                        |probe| run_authorized_probe(catalog, &opts, probe, &run_probe),
                        now,
                    )?;
                    verification.report
                }
                hmd_domain::ExecutionState::Unknown => Some(ReportTrigger::UnknownExecutionState),
                _ => Some(ReportTrigger::VerificationFailed),
            };
            if let Some(trigger) = trigger {
                outcome = ExecutionOutcome::NotResolved { trigger };
                break;
            }
        }
        match &outcome {
            ExecutionOutcome::Verified => {
                self.store
                    .apply(&ticket.id, TicketEvent::VerificationPassed, now)?;
            }
            ExecutionOutcome::NotResolved { trigger } => {
                self.store
                    .apply(&ticket.id, TicketEvent::CannotResolve(*trigger), now)?;
            }
            ExecutionOutcome::NotAvailable { .. } => {}
        }
        Ok(outcome)
    }

    /// "A internet voltou para você também?" Only the person moves a ticket to `resolved`.
    pub fn confirm(&mut self, ticket_id: &str, fixed: bool, now: Timestamp) -> Result<Ticket> {
        let event = if fixed {
            TicketEvent::UserConfirmed
        } else {
            TicketEvent::UserNotFixed
        };
        Ok(self.store.apply(ticket_id, event, now)?)
    }

    // ----- reports -----

    /// Renders and stores the "não consegui" report for the last diagnosis of this ticket. The
    /// ticket goes to `unresolved` when it was in progress.
    pub fn make_report(
        &mut self,
        ticket_id: &str,
        symptom: Option<&str>,
        trigger: Option<ReportTrigger>,
        now: Timestamp,
        run_probe: impl Fn(&str) -> std::result::Result<ProbeOutput, ProbeError>,
    ) -> Result<(i64, String)> {
        let ticket = self.ticket(ticket_id)?;
        let diagnosis = self
            .last_diagnosis
            .get(&ticket.id)
            .ok_or(AppError::NoDiagnosis)?;
        let trigger = trigger
            .or(diagnosis.report)
            .unwrap_or(ReportTrigger::PersonRequested);
        let opts = self.options(String::from("report"))?;
        let system = system_summary_with(self.catalog, &opts, run_probe);
        let symptom = symptom.map(redact_free_text);
        let text = render(&ReportInput {
            catalog: self.catalog,
            diagnosis,
            trigger,
            symptom: symptom.as_deref(),
            system: system.as_ref(),
            created_at: now,
        });
        let id = self.store.save_report(&ticket.id, trigger, &text, now)?;
        if ticket.state == TicketState::InProgress {
            self.store
                .apply(&ticket.id, TicketEvent::CannotResolve(trigger), now)?;
        }
        Ok((id, text))
    }

    pub fn edit_report(&mut self, report_id: i64, text: &str, now: Timestamp) -> Result<()> {
        Ok(self.store.edit_report(report_id, text, now)?)
    }

    /// Call after the person saved the file.
    pub fn report_exported(&mut self, report_id: i64, now: Timestamp) -> Result<()> {
        Ok(self.store.mark_exported(report_id, now)?)
    }

    // ----- privacy -----

    pub fn export_all(&self, now: Timestamp) -> Result<String> {
        Ok(self.store.export_markdown(now)?)
    }

    /// "Apagar tudo": closes the history and removes its files.
    pub fn erase_all(self, path: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
        drop(self.store);
        hmd_store::erase_all(path)
    }

    pub fn audit_intact(&self) -> Result<bool> {
        Ok(self.store.verify_audit_chain()?.broken_at.is_none())
    }
}
