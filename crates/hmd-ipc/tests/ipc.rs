// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 013: the UI bridge with fake probes, checked as the JSON the UI receives.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering::SeqCst};

use hmd_executor::{Elevation, EvaluatedPlan, ExecError, FixImpl, FixOutcome, Registry};
use hmd_ipc::{Backend, Clock, ProbeRunner};
use hmd_policy::{Platform, ProposedStep};
use hmd_probes::{Evidence, Finding, ProbeOutput, Severity};

const NOW: i64 = 1_760_400_000;

fn assert_send<T: Send>() {}

#[test]
fn backend_can_live_in_the_shell_state() {
    assert_send::<Backend>();
}

/// DNS fails until `fixed`; evidence carries a text that must never reach the UI.
fn runner(fixed: Arc<AtomicBool>) -> ProbeRunner {
    Box::new(move |id: &str| {
        let mut evidence = BTreeMap::new();
        evidence.insert("names", Evidence::Text("secret-host-name".into()));
        let findings = if id == "net.dns.resolve" && !fixed.load(SeqCst) {
            vec![Finding {
                code: "net.dns.resolution_failing",
                severity: Severity::Critical,
            }]
        } else {
            vec![]
        };
        Ok(ProbeOutput {
            probe_id: "fake",
            findings,
            evidence,
        })
    })
}

fn clock() -> (Clock, Arc<AtomicI64>) {
    let t = Arc::new(AtomicI64::new(NOW));
    let c = t.clone();
    (Box::new(move || c.load(SeqCst)), t)
}

struct Fix(Arc<AtomicBool>);

impl FixImpl for Fix {
    fn run(&self, _: &ProposedStep) -> FixOutcome {
        self.0.store(true, SeqCst);
        FixOutcome::Succeeded
    }
}

/// Spec 020: `net.dns.flush_cache` declares `requires_elevation`, so the executor routes it to the
/// elevation path. The tests keep their own fake and wrap it here.
struct FakeElevation<T>(T);

impl<T: FixImpl> Elevation for FakeElevation<T> {
    fn available(&self, _step: &ProposedStep, _platform: Platform) -> bool {
        true
    }
    fn run(
        &self,
        step: &ProposedStep,
        _plan: &EvaluatedPlan<'_>,
        _platform: Platform,
    ) -> Result<FixOutcome, ExecError> {
        Ok(self.0.run(step))
    }
    fn rollback(
        &self,
        step: &ProposedStep,
        _plan: &EvaluatedPlan<'_>,
        _platform: Platform,
    ) -> Option<FixOutcome> {
        Some(self.0.rollback(step))
    }
}

fn backend(fixed: &Arc<AtomicBool>, with_fix: bool) -> Backend {
    let (c, _) = clock();
    let mut b = Backend::in_memory(runner(fixed.clone()), c).unwrap();
    let f = fixed.clone();
    b.map_app(move |app| {
        let app = app.with_platform(Platform::Linux);
        if with_fix {
            app.with_registry(Registry::default().with_elevation(Box::new(FakeElevation(Fix(f)))))
        } else {
            app
        }
    });
    b
}

#[test]
fn full_flow_as_json() {
    let fixed = Arc::new(AtomicBool::new(false));
    let mut b = backend(&fixed, true);
    let start = b.start_ticket("connectivity").unwrap();
    assert_eq!(start.ticket.family_title, "Internet");
    let d = b.diagnose(&start.ticket.id).unwrap();
    let json = serde_json::to_value(&d).unwrap();
    assert_eq!(json["diagnosisId"], "net.dns");
    assert_eq!(json["confidencePhrase"], "Acho provável");
    assert_eq!(json["ticket"]["state"], "awaiting_user");
    assert_eq!(
        json["plan"]["steps"][0]["title"],
        "Limpar a memória de endereços da internet"
    );
    assert_eq!(json["plan"]["steps"][0]["requiresElevation"], true);
    let dns = json["probes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["probeId"] == "net.dns.resolve")
        .unwrap();
    assert_eq!(dns["status"], "found");
    assert_eq!(dns["title"], "Tradução de nomes de sites");
    assert!(
        !json.to_string().contains("secret-host-name"),
        "probe evidence never reaches the UI"
    );

    let plan = d.plan.unwrap();
    assert_eq!(b.plan(&start.ticket.id, plan.plan_id).unwrap(), plan);
    assert_eq!(
        b.plan("999", plan.plan_id).unwrap_err().code,
        "NO_PENDING_PLAN"
    );
    let approval = b
        .approve(&start.ticket.id, plan.plan_id, &["s1".to_string()])
        .unwrap();
    assert_eq!(approval.plan_hash, plan.plan_hash);
    assert!(
        b.plan(&start.ticket.id, plan.plan_id).is_err(),
        "an approved plan is no longer offered"
    );
    let outcome = b.execute(&start.ticket.id, plan.plan_id).unwrap();
    assert_eq!(outcome.kind, "verified");
    let t = b.confirm(&start.ticket.id, true).unwrap();
    assert_eq!(t.state, "resolved");
}

#[test]
fn production_build_cannot_run_the_fix_yet() {
    let fixed = Arc::new(AtomicBool::new(false));
    let mut b = backend(&fixed, false);
    let id = b.start_ticket("connectivity").unwrap().ticket.id;
    let plan = b.diagnose(&id).unwrap().plan.unwrap();
    b.approve(&id, plan.plan_id, &["s1".to_string()]).unwrap();
    let out = b.execute(&id, plan.plan_id).unwrap();
    assert_eq!(
        serde_json::to_value(&out).unwrap(),
        serde_json::json!({"kind": "elevation_not_available", "stepId": "s1", "trigger": null})
    );
    let report = b.make_report(&id, Some("não abre nada"), None).unwrap();
    assert!(report.text.starts_with("<!-- report@1 -->"));
    assert_eq!(b.history().unwrap()[0].state, "unresolved");
}

#[test]
fn errors_are_codes() {
    let fixed = Arc::new(AtomicBool::new(false));
    let mut b = backend(&fixed, false);
    assert_eq!(
        b.start_ticket("printer").unwrap_err().code,
        "UNKNOWN_FAMILY"
    );
    assert_eq!(b.diagnose("999").unwrap_err().code, "NO_SUCH_TICKET");
    assert_eq!(b.set_autonomy("god_mode").unwrap_err().code, "UNKNOWN_MODE");
    assert_eq!(
        b.set_consent("everything", true).unwrap_err().code,
        "UNKNOWN_CODE"
    );
    let id = b.start_ticket("storage").unwrap().ticket.id;
    assert_eq!(b.execute(&id, 1).unwrap_err().code, "INVALID_STATE");
}

#[test]
fn settings_and_erase_all_on_disk() {
    let dir = std::env::temp_dir().join(format!("hmd-ipc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path: PathBuf = dir.join("history.sqlite3");
    let fixed = Arc::new(AtomicBool::new(false));
    let (c, _) = clock();
    let (mut b, startup) = Backend::open(path.clone(), runner(fixed), c).unwrap();
    assert_eq!(startup.opened, "created");
    b.set_autonomy("observe_only").unwrap();
    b.set_consent("startup_apps", true).unwrap();
    let s = b.settings().unwrap();
    assert_eq!(s.autonomy, "observe_only");
    assert_eq!(s.consents, vec!["startup_apps".to_string()]);
    assert!(b.export_all().unwrap().contains("startup_apps"));
    b.start_ticket("storage").unwrap();
    assert!(b.erase_all().unwrap() >= 1);
    assert!(b.history().unwrap().is_empty(), "fresh history after erase");
    assert_eq!(b.settings().unwrap().autonomy, "ask_always");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_report_writes_a_new_file_and_marks_it_exported() {
    let dir = std::env::temp_dir().join(format!("hmd-ipc-save-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let fixed = Arc::new(AtomicBool::new(false));
    let mut b = backend(&fixed, false);
    let id = b.start_ticket("storage").unwrap().ticket.id;
    b.diagnose(&id).unwrap();
    let report = b.make_report(&id, None, None).unwrap();
    let first = b
        .save_report(report.id, "texto editado", &dir, None)
        .unwrap();
    assert!(
        first.ends_with("relatorio-helpmydesk-2025-10-14.md"),
        "{first}"
    );
    assert!(
        !first.contains(&*dir.parent().unwrap().to_string_lossy()),
        "no full path: {first}"
    );
    let first_file = dir.join(first.rsplit('/').next().unwrap());
    assert_eq!(
        std::fs::read_to_string(&first_file).unwrap(),
        "texto editado"
    );
    let second = b
        .save_report(report.id, "outra versão", &dir, None)
        .unwrap();
    assert!(
        second.ends_with("relatorio-helpmydesk-2025-10-14-1.md"),
        "never overwrites"
    );
    assert!(
        !first.contains(&*dir.parent().unwrap().to_string_lossy()),
        "no full path: {first}"
    );
    let first_file = dir.join(first.rsplit('/').next().unwrap());
    assert_eq!(
        std::fs::read_to_string(&first_file).unwrap(),
        "texto editado"
    );
    let missing = dir.join("nao-existe");
    assert_eq!(
        b.save_report(report.id, "x", &missing, None)
            .unwrap_err()
            .code,
        "SAVE_FAILED"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn system_view_has_only_os_facts() {
    let fixed = Arc::new(AtomicBool::new(false));
    let mut b = backend(&fixed, false);
    let json = serde_json::to_value(b.system().unwrap()).unwrap();
    assert!(
        json["catalogVersion"]
            .as_str()
            .unwrap()
            .starts_with("2026.")
    );
    assert!(!json.to_string().contains("secret-host-name"));
}

/// Spec 016 (P2): a failed "Apagar tudo" does not leave the app unusable until a restart.
#[cfg(unix)]
#[test]
fn failed_erase_recovers_on_the_next_command() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("hmd-ipc-erase-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let fixed = Arc::new(AtomicBool::new(false));
    let (c, _) = clock();
    let (mut b, _) = Backend::open(dir.join("history.sqlite3"), runner(fixed), c).unwrap();
    b.start_ticket("storage").unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let failed = b.erase_all();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    if failed.is_ok() {
        // Running as root: permissions do not stop the deletion; nothing to check here.
        let _ = std::fs::remove_dir_all(&dir);
        return;
    }
    assert_eq!(failed.unwrap_err().code, "ERASE_FAILED");
    assert!(b.history().is_ok(), "the history is reopened");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Spec 016 (P2): reports show the computer's date, not UTC; implausible offsets fall back to UTC.
#[test]
fn report_dates_follow_the_computer_clock() {
    let fixed = Arc::new(AtomicBool::new(false));
    let mut b = backend(&fixed, false);
    let id = b.start_ticket("storage").unwrap().ticket.id;
    b.diagnose(&id).unwrap();
    // NOW is midnight UTC on 2025-10-14: still the 13th in Brasília (UTC-3).
    let local = b.make_report(&id, None, Some(-180)).unwrap();
    assert!(
        local.text.contains("- Data: 2025-10-13\n"),
        "{}",
        local.text
    );
    let bogus = b.make_report(&id, None, Some(100_000)).unwrap();
    assert!(
        bogus.text.contains("- Data: 2025-10-14 (UTC)"),
        "{}",
        bogus.text
    );

    let dir = std::env::temp_dir().join(format!("hmd-ipc-tz-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = b.save_report(local.id, "x", &dir, Some(-180)).unwrap();
    assert!(
        path.ends_with("relatorio-helpmydesk-2025-10-13.md"),
        "{path}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

struct NetAnswer;

impl hmd_ai::Transport for NetAnswer {
    fn post(&self, _: &str, _: &str) -> Result<(u16, String), hmd_ai::AiError> {
        let content = r#"{"family": "connectivity", "confidence": "medium", "question": "Acontece no Wi-Fi e no cabo?"}"#;
        Ok((
            200,
            serde_json::json!({"choices": [{"message": {"content": content}}]}).to_string(),
        ))
    }
}

/// Spec 018: the AI use cases as the JSON the screens receive.
#[test]
fn ai_connect_and_classify_as_json() {
    let fixed = Arc::new(AtomicBool::new(false));
    let mut b = backend(&fixed, false);
    b.set_transport(Box::new(NetAnswer));
    let status = serde_json::to_value(b.ai_status().unwrap()).unwrap();
    assert_eq!(status["connected"], false);
    assert_eq!(status["consoleUrl"], hmd_ai::CONSOLE_URL);
    assert_eq!(b.ai_classify("x").unwrap_err().code, "AI_NOT_CONNECTED");
    assert!(b.ai_connect("k").unwrap().connected);
    let c = serde_json::to_value(b.ai_classify("o wifi caiu").unwrap()).unwrap();
    assert_eq!(c["family"], "connectivity");
    assert_eq!(c["confidence"], "medium");
    assert_eq!(c["question"], "Acontece no Wi-Fi e no cabo?");
    assert!(!b.ai_disconnect().unwrap().connected);
}

/// The shell's path: prepare under the lock, call outside it, finish under the lock.
#[test]
fn ai_call_in_three_steps_never_prints_the_key() {
    let fixed = Arc::new(AtomicBool::new(false));
    let mut b = backend(&fixed, false);
    assert_eq!(b.ai_prepare("x").unwrap_err().code, "AI_NOT_CONNECTED");
    let test = Backend::ai_test_key(&NetAnswer, " sk-secret-123 ");
    assert!(
        b.ai_connect_finish("sk-secret-123", test)
            .unwrap()
            .connected
    );
    let call = b.ai_prepare("o wifi caiu").unwrap();
    assert!(
        !format!("{call:?}").contains("sk-secret-123"),
        "Debug hides the key"
    );
    // The fake transport (the backend's own is the real network; tests never use it).
    let result = call.run(&NetAnswer);
    let dto = b.ai_finish(call.model, result).unwrap();
    assert_eq!(dto.family.as_deref(), Some("connectivity"));
}

/// "Apagar tudo" also removes the AI key from the vault (it is personal data too).
#[test]
fn erase_all_also_removes_the_ai_key() {
    let fixed = Arc::new(AtomicBool::new(false));
    let mut b = backend(&fixed, false);
    b.set_transport(Box::new(NetAnswer));
    assert!(b.ai_connect("sk-secret").unwrap().connected);
    b.erase_all().unwrap();
    assert!(!b.ai_status().unwrap().connected);
    // Even granting the consent again does not bring the old key back.
    b.set_consent("ai_opencode", true).unwrap();
    assert!(
        !b.ai_status().unwrap().connected,
        "the key is gone from the vault"
    );
}

/// F3-TRANSP: the preview shows the redacted text; the log has codes only, newest first.
#[test]
fn ai_preview_and_log() {
    let fixed = Arc::new(AtomicBool::new(false));
    let mut b = backend(&fixed, false);
    b.set_transport(Box::new(NetAnswer));
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    if user.chars().count() >= 3 {
        assert!(
            !b.ai_preview(&format!("o PC do {user} travou"))
                .contains(&user)
        );
    }
    b.ai_connect("k").unwrap();
    b.ai_classify("o wifi caiu de novo").unwrap();
    let log = serde_json::to_value(b.ai_log().unwrap()).unwrap();
    assert_eq!(log[0]["outcome"], "ok:connectivity");
    assert_eq!(log[1]["outcome"], "ok:connection_test");
    assert_eq!(log[0]["model"], hmd_ai::DEFAULT_MODEL);
    assert!(!log.to_string().contains("wifi"), "never the text");
}

/// Spec 019: a downloaded kill switch, signed with the project key, is applied; the same file
/// again is fine; a bad signature is an error.
#[test]
fn downloaded_kill_switch_is_applied() {
    let fixed = Arc::new(AtomicBool::new(false));
    let mut b = backend(&fixed, false);
    let file = include_bytes!("../../hmd-app/tests/fixtures/killswitch-empty.json");
    let sig = include_str!("../../hmd-app/tests/fixtures/killswitch-empty.json.sig");
    assert_eq!(
        b.apply_kill_switch(file, sig).unwrap(),
        Vec::<String>::new()
    );
    assert!(b.apply_kill_switch(file, sig).is_ok(), "already have it");
    let forged = "0".repeat(128);
    assert_eq!(
        b.apply_kill_switch(file, &forged).unwrap_err().code,
        "KILLSWITCH_BAD_SIGNATURE"
    );
}
