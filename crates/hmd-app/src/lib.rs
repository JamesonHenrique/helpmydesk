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
    ActionApproval, EvaluatedPlan, ExecError, ExecuteInput, Registry, execute_step,
    verification_passed, verify_step,
};
use hmd_policy::{
    ActionPlanProposal, ApprovalRequirement, AutonomyMode, Platform, PolicyDecision, StepVerdict,
};
use hmd_probes::{ProbeError, ProbeOutput};
use hmd_store::{OpenOutcome, PurgeStats, Store, StoreError};

/// Setting key for the autonomy mode.
pub const AUTONOMY_KEY: &str = "autonomy_mode";

/// Largest kill switch file accepted (re-exported for the downloader).
pub const KILL_SWITCH_MAX_BYTES: usize = hmd_killswitch::MAX_BYTES;

/// Setting with the fixes still blocked when the history was erased (comma-separated IDs).
const CARRIED_BLOCKS_KEY: &str = "carried_blocks";

/// Kept across "Apagar tudo": see [`App::safety_state`].
#[derive(Debug, Clone, Default)]
pub struct SafetyState {
    pub kill_switch: Option<(Vec<u8>, String)>,
    pub blocked: BTreeSet<String>,
}

/// Largest edited report kept or saved (a generated one is a few KiB; spec 016).
pub const MAX_REPORT_BYTES: usize = 64 * 1024;

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
    KillSwitch(hmd_killswitch::KillSwitchError),
    /// The AI is not connected (no consent or no key in the vault).
    AiNotConnected,
    /// The AI call failed or its answer was refused.
    Ai(hmd_ai::AiError),
    /// The system vault could not store or read the key; nothing is kept in plain text.
    Vault(String),
    /// The approval names no step, or a step the plan does not offer.
    StepNotInPlan,
    /// "Só observar" is on: nothing that changes the system runs, even if approved before.
    ObserveOnly,
    /// A kill switch turned off a capability of the plan after it was proposed.
    CapabilityDisabled,
    /// An edited report above [`MAX_REPORT_BYTES`].
    ReportTooLarge,
    /// The fix has a run in `unknown` (or still `started`): it cannot run again yet.
    PendingUnknown,
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
            AppError::KillSwitch(e) => e.reason_code(),
            AppError::AiNotConnected => "AI_NOT_CONNECTED",
            AppError::Ai(e) => e.reason_code(),
            AppError::Vault(_) => "AI_KEY_STORE_FAILED",
            AppError::StepNotInPlan => "STEP_NOT_IN_PLAN",
            AppError::ObserveOnly => "OBSERVE_ONLY",
            AppError::CapabilityDisabled => "CAPABILITY_DISABLED",
            AppError::ReportTooLarge => "REPORT_TOO_LARGE",
            AppError::PendingUnknown => "PENDING_UNKNOWN_EXECUTION",
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
            AppError::KillSwitch(e) => write!(f, "{e}"),
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
    /// Monotonic time of the approval: moving the wall clock back cannot stretch the
    /// 10-minute validity (spec 016, P2).
    approved_at: Option<std::time::Instant>,
}

/// How a verification probe that ran resolves an `unknown` fix: `Some(true)` applied (problem
/// gone), `Some(false)` not applied (problem still there; only for idempotent fixes, which may be
/// proposed again), `None` stays `unknown` (a non-idempotent fix is never risked twice).
pub fn unknown_resolution(problem_gone: bool, idempotent: bool) -> Option<bool> {
    match (problem_gone, idempotent) {
        (true, _) => Some(true),
        (false, true) => Some(false),
        (false, false) => None,
    }
}

/// Whether an approval given `elapsed` ago (monotonic) is past [`hmd_executor::APPROVAL_TTL`].
pub fn approval_too_old(elapsed: std::time::Duration) -> bool {
    elapsed.as_secs() > u64::try_from(hmd_executor::APPROVAL_TTL).unwrap_or(0)
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
    /// Release keys the kill switch must be signed with (spec 014).
    trusted_keys: Vec<[u8; 32]>,
    /// Capabilities turned off by the last accepted kill switch.
    disabled: BTreeSet<String>,
    /// AI models turned off by the last accepted kill switch.
    disabled_models: BTreeSet<String>,
    last_kill_switch: Option<i64>,
    catalog: &'static LoadedCatalog,
    platform: Platform,
    registry: Registry,
    pending: BTreeMap<i64, Pending>,
    last_diagnosis: BTreeMap<String, DiagnosisReport>,
    /// Offset of the computer's clock, told by the interface; reports show local dates.
    utc_offset_minutes: Option<i32>,
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
            trusted_keys: hmd_killswitch::TRUSTED_KEYS.to_vec(),
            disabled: BTreeSet::new(),
            disabled_models: BTreeSet::new(),
            last_kill_switch: None,
            catalog: catalog()?,
            platform: current_platform(),
            registry: Registry::production(),
            pending: BTreeMap::new(),
            last_diagnosis: BTreeMap::new(),
            utc_offset_minutes: None,
        };
        app.load_kill_switch()?;
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

    /// Tests use their own release key; the app ships with [`hmd_killswitch::TRUSTED_KEYS`].
    pub fn with_trusted_keys(mut self, keys: Vec<[u8; 32]>) -> Result<App> {
        self.trusted_keys = keys;
        self.load_kill_switch()?;
        Ok(self)
    }

    /// Re-verifies the stored kill switch with the trusted keys; a file that no longer verifies
    /// is ignored.
    fn load_kill_switch(&mut self) -> Result<()> {
        self.disabled.clear();
        self.disabled_models.clear();
        self.last_kill_switch = None;
        if let Some((payload, signature)) = self.store.kill_switch()?
            && let Ok(ks) = hmd_killswitch::verify(&payload, &signature, &self.trusted_keys)
        {
            self.disabled = ks.disabled_capabilities;
            self.disabled_models = ks.disabled_models;
            self.last_kill_switch = Some(ks.issued_at);
        }
        Ok(())
    }

    /// Spec 014: a newly downloaded kill switch. Applied only if signed by a trusted key, newer
    /// than the last one and not expired; then it is kept for offline use.
    pub fn apply_kill_switch(
        &mut self,
        file: &[u8],
        signature_hex: &str,
        now: Timestamp,
    ) -> Result<BTreeSet<String>> {
        let ks = hmd_killswitch::verify(file, signature_hex, &self.trusted_keys)
            .and_then(|ks| hmd_killswitch::accept(ks, self.last_kill_switch, now))
            .map_err(AppError::KillSwitch)?;
        self.store
            .set_kill_switch(ks.issued_at, file, signature_hex, now)?;
        self.last_kill_switch = Some(ks.issued_at);
        self.disabled = ks.disabled_capabilities;
        self.disabled_models = ks.disabled_models;
        let disabled = &self.disabled;
        self.pending.retain(|_, p| {
            !p.proposal
                .steps
                .iter()
                .any(|s| disabled.contains(&s.capability))
        });
        Ok(self.disabled.clone())
    }

    pub fn disabled_capabilities(&self) -> &BTreeSet<String> {
        &self.disabled
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
        self.store
            .set_setting(AUTONOMY_KEY, autonomy_code(mode), now)?;
        if mode == AutonomyMode::ObserveOnly {
            // "Só observar" cancels everything proposed or approved before.
            self.pending.clear();
        }
        Ok(())
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
            unknown_executions: self.blocked_capabilities()?,
            disabled: self.disabled.clone(),
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
        // A new diagnosis replaces every plan proposed before for this ticket: an old plan may no
        // longer match the system (spec 016).
        self.pending.retain(|_, p| p.ticket_id != ticket.id);
        let ticket = self.begin_session(&ticket, now)?;
        self.clarify_unknowns(now, &run_probe)?;
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
                    approved_at: None,
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
            // An approved plan cannot be approved again (it would renew the validity and change
            // the steps; independent review).
            .filter(|p| p.ticket_id == ticket.id && p.approval.is_none())
            .ok_or(AppError::NoPendingPlan)?;
        let offered = |id: &&str| pending.view.steps.iter().any(|s| s.step_id == *id);
        if step_ids.is_empty() || !step_ids.iter().all(offered) {
            return Err(AppError::StepNotInPlan);
        }
        let plan = EvaluatedPlan {
            proposal: &pending.proposal,
            decision: &pending.decision,
            catalog_version: &self.catalog.version,
        };
        let approval = self.store.approve(plan_id, &plan, step_ids, mode, now)?;
        pending.approval = Some(approval.clone());
        pending.approved_at = Some(std::time::Instant::now());
        if ticket.state == TicketState::AwaitingUser {
            self.store
                .apply(&ticket.id, TicketEvent::UserResponded, now)?;
        }
        Ok(approval)
    }

    /// The way out of `unknown` (spec 016, P2): the fix's verification probe runs. Problem gone →
    /// the fix counts as applied; problem still there → it counts as not applied, so the same
    /// fix can be proposed again and needs a new approval. A probe that fails or is not
    /// authorized leaves the execution `unknown` (still blocked). Never re-runs a fix.
    fn clarify_unknowns(
        &mut self,
        now: Timestamp,
        run_probe: &impl Fn(&str) -> std::result::Result<ProbeOutput, ProbeError>,
    ) -> Result<usize> {
        let mut clarified = 0;
        for (execution, capability) in self.store.unknown_executions()? {
            let (id, version) = capability
                .split_once('@')
                .map_or((capability.as_str(), None), |(id, v)| (id, Some(v)));
            let tag = format!("clarify:{execution}@{now}");
            if let Some(effect) = self.resolve_blocked(id, version, tag, run_probe)? {
                self.store.clarify_unknown(execution, effect, now)?;
                clarified += 1;
            }
        }
        // Blocks carried over an "Apagar tudo" (no execution row left): same rule.
        let mut carried = self.carried_blocks()?;
        let before = carried.len();
        for id in carried.clone() {
            let tag = format!("clarify:carried@{now}");
            if self.resolve_blocked(&id, None, tag, run_probe)?.is_some() {
                carried.remove(&id);
            }
        }
        if carried.len() != before {
            self.set_carried_blocks(&carried, now)?;
            clarified += before - carried.len();
        }
        Ok(clarified)
    }

    /// Runs the fix's verification probe for a blocked capability; `None` keeps it blocked.
    /// A recorded version different from the current catalog's also keeps it blocked: the
    /// catalog's `idempotent` may not describe the version that ran (independent review).
    fn resolve_blocked(
        &self,
        id: &str,
        version: Option<&str>,
        tag: String,
        run_probe: &impl Fn(&str) -> std::result::Result<ProbeOutput, ProbeError>,
    ) -> Result<Option<bool>> {
        let Some(capability) = self.catalog.catalog.get(id) else {
            return Ok(None);
        };
        if version.is_some_and(|v| v != capability.version) {
            return Ok(None);
        }
        let Some(variant) = capability.variants.get(&self.platform) else {
            return Ok(None);
        };
        let Some(probe) = variant.verification.as_deref() else {
            return Ok(None);
        };
        let opts = self.options(tag)?;
        let outcome = run_authorized_probe(self.catalog, &opts, probe, run_probe);
        if !matches!(outcome, hmd_core::ProbeOutcome::Ran(_)) {
            return Ok(None);
        }
        let idempotent = self.catalog.meta.get(id).is_some_and(|m| m.idempotent);
        Ok(unknown_resolution(
            verification_passed(variant, &outcome),
            idempotent,
        ))
    }

    /// OS facts for "Meu computador", through the Policy Engine with the app's options.
    pub fn system_summary(
        &self,
        run_probe: impl Fn(&str) -> std::result::Result<ProbeOutput, ProbeError>,
    ) -> Result<Option<hmd_core::report::SystemSummary>> {
        let opts = self.options(String::from("system"))?;
        Ok(system_summary_with(self.catalog, &opts, run_probe))
    }

    /// Capability IDs that cannot run: an execution in `unknown`/`started`, or a block carried
    /// over an "Apagar tudo".
    pub fn blocked_capabilities(&self) -> Result<BTreeSet<String>> {
        let mut out = self.store.unknown_capabilities()?;
        out.extend(self.carried_blocks()?);
        Ok(out)
    }

    fn carried_blocks(&self) -> Result<BTreeSet<String>> {
        Ok(self
            .store
            .setting(CARRIED_BLOCKS_KEY)?
            .map(|v| {
                v.split(',')
                    .filter(|x| !x.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default())
    }

    fn set_carried_blocks(&mut self, ids: &BTreeSet<String>, now: Timestamp) -> Result<()> {
        let joined = ids.iter().cloned().collect::<Vec<_>>().join(",");
        Ok(self.store.set_setting(CARRIED_BLOCKS_KEY, &joined, now)?)
    }

    /// What "Apagar tudo" must not erase (independent review, M4): the last accepted kill switch
    /// (its anti-rollback included) and the fixes that are blocked.
    pub fn safety_state(&self) -> Result<SafetyState> {
        Ok(SafetyState {
            kill_switch: self.store.kill_switch()?,
            blocked: self.blocked_capabilities()?,
        })
    }

    /// Puts the safety state back into a fresh history. The kill switch is verified again with
    /// the trusted keys; one that no longer verifies is dropped.
    pub fn restore_safety_state(&mut self, state: SafetyState, now: Timestamp) -> Result<()> {
        if let Some((payload, signature)) = state.kill_switch
            && let Ok(ks) = hmd_killswitch::verify(&payload, &signature, &self.trusted_keys)
        {
            self.store
                .set_kill_switch(ks.issued_at, &payload, &signature, now)?;
            self.load_kill_switch()?;
        }
        if !state.blocked.is_empty() {
            self.set_carried_blocks(&state.blocked, now)?;
        }
        Ok(())
    }

    /// The interface tells the computer's UTC offset (minutes) so reports show local dates;
    /// implausible values are ignored (UTC).
    pub fn set_utc_offset(&mut self, minutes: Option<i32>) {
        self.utc_offset_minutes = minutes.and_then(hmd_core::report::valid_utc_offset);
    }

    /// Removes the plan only when it belongs to `ticket_id`.
    fn take_pending(&mut self, ticket_id: &str, plan_id: i64) -> Result<Pending> {
        if self
            .pending
            .get(&plan_id)
            .is_none_or(|p| p.ticket_id != ticket_id)
        {
            return Err(AppError::NoPendingPlan);
        }
        self.pending.remove(&plan_id).ok_or(AppError::NoPendingPlan)
    }

    /// "Não, obrigado": the plan is dropped; the person keeps the guidance and the report option.
    pub fn decline(&mut self, ticket_id: &str, plan_id: i64, now: Timestamp) -> Result<()> {
        let ticket = self.ticket(ticket_id)?;
        self.take_pending(&ticket.id, plan_id)?;
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
        // Taken first: any refusal below consumes the plan, so it cannot be retried later
        // (independent review, M1).
        let pending = self.take_pending(&ticket.id, plan_id)?;
        if self.autonomy()? == AutonomyMode::ObserveOnly {
            return Err(AppError::ObserveOnly);
        }
        let approval = pending.approval.as_ref().ok_or(AppError::NoPendingPlan)?;
        if pending
            .approved_at
            .is_none_or(|at| approval_too_old(at.elapsed()))
        {
            return Err(AppError::Exec(ExecError::ApprovalExpired));
        }
        if pending.proposal.steps.iter().any(|s| {
            approval.approved_step_ids.contains(&s.step_id) && self.disabled.contains(&s.capability)
        }) {
            return Err(AppError::CapabilityDisabled);
        }
        // Independent review, M3: an `unknown` run of the same fix (e.g. from another ticket)
        // that appeared after this plan was proposed still blocks it.
        let blocked = self.blocked_capabilities()?;
        if pending.proposal.steps.iter().any(|s| {
            approval.approved_step_ids.contains(&s.step_id) && blocked.contains(&s.capability)
        }) {
            return Err(AppError::PendingUnknown);
        }
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
            fixes_attempted: self.store.fix_attempts(&ticket.id)?,
            utc_offset_minutes: self.utc_offset_minutes,
        });
        let id = self.store.save_report(&ticket.id, trigger, &text, now)?;
        if ticket.state == TicketState::InProgress {
            self.store
                .apply(&ticket.id, TicketEvent::CannotResolve(trigger), now)?;
        }
        Ok((id, text))
    }

    pub fn edit_report(&mut self, report_id: i64, text: &str, now: Timestamp) -> Result<()> {
        if text.len() > MAX_REPORT_BYTES {
            return Err(AppError::ReportTooLarge);
        }
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

pub mod ai;
