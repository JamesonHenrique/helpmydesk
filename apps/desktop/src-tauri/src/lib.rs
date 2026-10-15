// SPDX-License-Identifier: GPL-3.0-or-later
//! Desktop shell (spec 013): one Tauri command per `hmd_ipc::Backend` method, nothing else.
//! Commands are `async` so long checks (network probes) never block the window.
//! Which window may call what is decided by `capabilities/*.json` (INV-04).

use std::sync::Mutex;

use hmd_ipc::{
    ApprovalDto, Backend, DiagnosisDto, IpcError, IpcResult, OutcomeDto, PlanDto, ReportDto,
    SettingsDto, StartDto, StartupDto, TicketDto, system_clock,
};
use serde::Serialize;
use tauri::{
    AppHandle, Builder, Emitter, Manager, Runtime, State, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};

/// Event the approval window's decision (or its closing) is announced with.
const APPROVAL_EVENT: &str = "hmd://approval-decided";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApprovalDecision {
    ticket_id: String,
    plan_id: i64,
    decision: &'static str,
}

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
async fn open_approval<R: Runtime>(
    app: AppHandle<R>,
    ticket_id: String,
    plan_id: i64,
) -> IpcResult<()> {
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
    let window = WebviewWindowBuilder::new(&app, "approval", url)
        .title("HelpMyDesk: aprovar correção")
        .inner_size(560.0, 680.0)
        .resizable(true)
        .build()
        .map_err(|e| IpcError {
            code: "WINDOW_FAILED".into(),
            message: e.to_string(),
        })?;
    // Closing the window without deciding counts as "no" (the main window must not wait forever).
    let handle = app.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            let _ = handle.emit(
                APPROVAL_EVENT,
                ApprovalDecision {
                    ticket_id: ticket_id.clone(),
                    plan_id,
                    decision: "closed",
                },
            );
        }
    });
    Ok(())
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

/// Saves the report into the person's Downloads folder (or Documents, or home) and returns the
/// path, so the screen can say where it is.
#[tauri::command]
async fn save_report<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, Shared>,
    report_id: i64,
    text: String,
) -> IpcResult<String> {
    let paths = app.path();
    let dir = paths
        .download_dir()
        .or_else(|_| paths.document_dir())
        .or_else(|_| paths.home_dir())
        .map_err(|e| IpcError {
            code: "NO_FOLDER".into(),
            message: e.to_string(),
        })?;
    with(&state, |b| b.save_report(report_id, &text, &dir))
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

/// Registers every command (the app and the IPC tests use the same list).
fn commands<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
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
        save_report,
        history,
        settings,
        set_autonomy,
        set_consent,
        export_all,
        erase_all,
    ])
}

/// Builds and runs the app. The history lives in the per-user app data directory.
pub fn run() {
    let result = commands(tauri::Builder::default())
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
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("HelpMyDesk could not start: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    //! IPC with the real capability files (INV-04) on Tauri's mock runtime.
    use std::collections::BTreeMap;

    use hmd_ipc::Backend;
    use hmd_probes::ProbeOutput;
    use serde_json::{Value, json};
    use tauri::test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder};
    use tauri::webview::InvokeRequest;
    use tauri::{App, Manager, WebviewWindow, WebviewWindowBuilder};

    use super::{Shared, commands};

    fn app() -> App<MockRuntime> {
        let app = commands(mock_builder())
            .build(tauri::generate_context!())
            .unwrap();
        let backend = Backend::in_memory(
            Box::new(|_: &str| {
                Ok(ProbeOutput {
                    probe_id: "fake",
                    findings: vec![],
                    evidence: BTreeMap::new(),
                })
            }),
            Box::new(|| 1_760_400_000),
        )
        .unwrap();
        app.manage(Shared {
            backend: std::sync::Mutex::new(backend),
            startup: hmd_ipc::StartupDto {
                opened: "created".into(),
                interrupted: 0,
                housekept: 0,
            },
        });
        app
    }

    fn window(app: &App<MockRuntime>, label: &str) -> WebviewWindow<MockRuntime> {
        WebviewWindowBuilder::new(app, label, Default::default())
            .build()
            .unwrap()
    }

    fn call(w: &WebviewWindow<MockRuntime>, cmd: &str, args: Value) -> Result<Value, Value> {
        get_ipc_response(
            w,
            InvokeRequest {
                cmd: cmd.into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "tauri://localhost".parse().unwrap(),
                body: tauri::ipc::InvokeBody::Json(args),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
        .map(|b| b.deserialize::<Value>().unwrap())
    }

    #[test]
    fn main_window_runs_use_cases_with_camel_case_arguments() {
        let app = app();
        let main = window(&app, "main");
        let started = call(&main, "start_ticket", json!({"family": "storage"})).unwrap();
        assert_eq!(started["ticket"]["familyTitle"], "Sem espaço");
        let id = started["ticket"]["id"].as_str().unwrap().to_string();
        let d = call(&main, "diagnose", json!({"ticketId": id})).unwrap();
        assert_eq!(d["ticket"]["state"], "in_progress");
        let history = call(&main, "history", json!({})).unwrap();
        assert_eq!(history.as_array().unwrap().len(), 1);
    }

    /// INV-04: the main window is refused `approve`/`decline`/`plan` by the ACL itself.
    #[test]
    fn main_window_cannot_approve() {
        let app = app();
        let main = window(&app, "main");
        for cmd in ["approve", "decline", "plan"] {
            let err = call(
                &main,
                cmd,
                json!({"ticketId": "1", "planId": 1, "stepIds": ["s1"]}),
            )
            .unwrap_err();
            let text = err.to_string();
            assert!(
                !text.contains("NO_PENDING_PLAN") && !text.contains("NO_SUCH_TICKET"),
                "{cmd} reached the command from the main window: {text}"
            );
        }
    }

    /// The approval window reaches `plan`/`approve` (the core then answers: nothing pending) but
    /// not the other use cases.
    #[test]
    fn approval_window_only_approves() {
        let app = app();
        let approval = window(&app, "approval");
        let err = call(&approval, "plan", json!({"ticketId": "1", "planId": 1})).unwrap_err();
        assert_eq!(err["code"], "NO_PENDING_PLAN", "{err}");
        let err = call(&approval, "start_ticket", json!({"family": "storage"})).unwrap_err();
        assert!(
            err.get("code").is_none(),
            "start_ticket reached the command: {err}"
        );
    }
}
