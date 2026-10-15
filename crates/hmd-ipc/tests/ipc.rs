// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 013: the UI bridge with fake probes, checked as the JSON the UI receives.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering::SeqCst};

use hmd_executor::{FixImpl, FixOutcome, Registry};
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

fn backend(fixed: &Arc<AtomicBool>, with_fix: bool) -> Backend {
    let (c, _) = clock();
    let mut b = Backend::in_memory(runner(fixed.clone()), c).unwrap();
    let f = fixed.clone();
    b.map_app(move |app| {
        let app = app.with_platform(Platform::Linux);
        if with_fix {
            app.with_registry(Registry::default().with("net.dns.flush_cache", Box::new(Fix(f))))
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
        serde_json::json!({"kind": "not_available", "stepId": "s1", "trigger": null})
    );
    let report = b.make_report(&id, Some("não abre nada")).unwrap();
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
    let report = b.make_report(&id, None).unwrap();
    let first = b.save_report(report.id, "texto editado", &dir).unwrap();
    assert!(
        first.ends_with("relatorio-helpmydesk-2025-10-14.md"),
        "{first}"
    );
    assert_eq!(std::fs::read_to_string(&first).unwrap(), "texto editado");
    let second = b.save_report(report.id, "outra versão", &dir).unwrap();
    assert!(
        second.ends_with("relatorio-helpmydesk-2025-10-14-1.md"),
        "never overwrites"
    );
    assert_eq!(std::fs::read_to_string(&first).unwrap(), "texto editado");
    let missing = dir.join("nao-existe");
    assert_eq!(
        b.save_report(report.id, "x", &missing).unwrap_err().code,
        "SAVE_FAILED"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
