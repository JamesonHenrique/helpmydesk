// SPDX-License-Identifier: GPL-3.0-or-later
//! Desktop shell (spec 013): one Tauri command per `hmd_ipc::Backend` method, nothing else.
//! Commands are `async` so long checks (network probes) never block the window.
//! Which window may call what is decided by `capabilities/*.json` (INV-04).

use std::sync::Mutex;

use hmd_ipc::{
    ApprovalDto, Backend, DiagnosisDto, IpcError, IpcResult, OutcomeDto, PlanDto, ReportDto,
    SettingsDto, StartDto, StartupDto, TicketDto, system_clock,
};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

struct Shared {
    backend: Mutex<Backend>,
    startup: StartupDto,
}

fn lock_error() -> IpcError {
    IpcError {
        code: "BUSY".into(),
        message: "backend lock poisoned".into(),
    }
}

fn with<T>(
    state: &State<'_, Shared>,
    f: impl FnOnce(&mut Backend) -> IpcResult<T>,
) -> IpcResult<T> {
    let mut backend = state.backend.lock().map_err(|_| lock_error())?;
    f(&mut backend)
}

#[tauri::command]
async fn startup(state: State<'_, Shared>) -> IpcResult<StartupDto> {
    Ok(state.startup.clone())
}

#[tauri::command]
async fn start_ticket(state: State<'_, Shared>, family: String) -> IpcResult<StartDto> {
    with(&state, |b| b.start_ticket(&family))
}

#[tauri::command]
async fn diagnose(state: State<'_, Shared>, ticket_id: String) -> IpcResult<DiagnosisDto> {
    with(&state, |b| b.diagnose(&ticket_id))
}

/// Opens the approval window (its own capability) for a pending plan.
#[tauri::command]
async fn open_approval(app: AppHandle, ticket_id: String, plan_id: i64) -> IpcResult<()> {
    if ticket_id.is_empty() || !ticket_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(IpcError {
            code: "INVALID_TICKET".into(),
            message: "ticket id must be digits".into(),
        });
    }
    if let Some(old) = app.get_webview_window("approval") {
        let _ = old.close();
    }
    let url = WebviewUrl::App(format!("index.html#/aprovacao/{ticket_id}/{plan_id}").into());
    WebviewWindowBuilder::new(&app, "approval", url)
        .title("HelpMyDesk: aprovar correção")
        .inner_size(560.0, 680.0)
        .resizable(true)
        .build()
        .map(|_| ())
        .map_err(|e| IpcError {
            code: "WINDOW_FAILED".into(),
            message: e.to_string(),
        })
}

#[tauri::command]
async fn plan(state: State<'_, Shared>, ticket_id: String, plan_id: i64) -> IpcResult<PlanDto> {
    with(&state, |b| b.plan(&ticket_id, plan_id))
}

#[tauri::command]
async fn approve(
    state: State<'_, Shared>,
    ticket_id: String,
    plan_id: i64,
    step_ids: Vec<String>,
) -> IpcResult<ApprovalDto> {
    with(&state, |b| b.approve(&ticket_id, plan_id, &step_ids))
}

#[tauri::command]
async fn decline(state: State<'_, Shared>, ticket_id: String, plan_id: i64) -> IpcResult<()> {
    with(&state, |b| b.decline(&ticket_id, plan_id))
}

#[tauri::command]
async fn execute(
    state: State<'_, Shared>,
    ticket_id: String,
    plan_id: i64,
) -> IpcResult<OutcomeDto> {
    with(&state, |b| b.execute(&ticket_id, plan_id))
}

#[tauri::command]
async fn confirm(state: State<'_, Shared>, ticket_id: String, fixed: bool) -> IpcResult<TicketDto> {
    with(&state, |b| b.confirm(&ticket_id, fixed))
}

#[tauri::command]
async fn make_report(
    state: State<'_, Shared>,
    ticket_id: String,
    symptom: Option<String>,
) -> IpcResult<ReportDto> {
    with(&state, |b| b.make_report(&ticket_id, symptom.as_deref()))
}

#[tauri::command]
async fn edit_report(state: State<'_, Shared>, report_id: i64, text: String) -> IpcResult<()> {
    with(&state, |b| b.edit_report(report_id, &text))
}

#[tauri::command]
async fn report_exported(state: State<'_, Shared>, report_id: i64) -> IpcResult<()> {
    with(&state, |b| b.report_exported(report_id))
}

#[tauri::command]
async fn history(state: State<'_, Shared>) -> IpcResult<Vec<TicketDto>> {
    with(&state, Backend::history)
}

#[tauri::command]
async fn settings(state: State<'_, Shared>) -> IpcResult<SettingsDto> {
    with(&state, Backend::settings)
}

#[tauri::command]
async fn set_autonomy(state: State<'_, Shared>, mode: String) -> IpcResult<()> {
    with(&state, |b| b.set_autonomy(&mode))
}

#[tauri::command]
async fn set_consent(state: State<'_, Shared>, scope: String, granted: bool) -> IpcResult<()> {
    with(&state, |b| b.set_consent(&scope, granted))
}

#[tauri::command]
async fn export_all(state: State<'_, Shared>) -> IpcResult<String> {
    with(&state, Backend::export_all)
}

#[tauri::command]
async fn erase_all(state: State<'_, Shared>) -> IpcResult<usize> {
    with(&state, Backend::erase_all)
}

/// Builds and runs the app. The history lives in the per-user app data directory.
pub fn run() {
    let result = tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let (backend, startup) = Backend::open(
                dir.join("history.sqlite3"),
                Box::new(hmd_probes::run),
                Box::new(system_clock),
            )
            .map_err(|e| format!("{}: {}", e.code, e.message))?;
            app.manage(Shared {
                backend: Mutex::new(backend),
                startup,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            startup,
            start_ticket,
            diagnose,
            open_approval,
            plan,
            approve,
            decline,
            execute,
            confirm,
            make_report,
            edit_report,
            report_exported,
            history,
            settings,
            set_autonomy,
            set_consent,
            export_all,
            erase_all,
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("HelpMyDesk could not start: {e}");
        std::process::exit(1);
    }
}
