// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 016 (F5-SEC): adversarial suite at the UI boundary. Every case is a sequence of calls a
//! compromised or buggy webview could make; none of them may run the fix. The fake fix counts its
//! runs, so "blocked" means the counter stayed at zero, not only that an error came back.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering::SeqCst};

use hmd_executor::{APPROVAL_TTL, FixImpl, FixOutcome, Registry};
use hmd_ipc::{Backend, ProbeRunner};
use hmd_policy::{Platform, ProposedStep};
use hmd_probes::{Finding, ProbeOutput, Severity};

const NOW: i64 = 1_760_400_000;

struct Fix(Arc<AtomicUsize>);

impl FixImpl for Fix {
    fn run(&self, _: &ProposedStep) -> FixOutcome {
        self.0.fetch_add(1, SeqCst);
        FixOutcome::Succeeded
    }
}

/// DNS keeps failing until the fix ran once or the problem went away by itself.
fn runner(runs: Arc<AtomicUsize>, healthy: Arc<AtomicBool>) -> ProbeRunner {
    Box::new(move |id: &str| {
        let failing = runs.load(SeqCst) == 0 && !healthy.load(SeqCst);
        let findings = if id == "net.dns.resolve" && failing {
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
            evidence: BTreeMap::new(),
        })
    })
}

struct Lab {
    b: Backend,
    runs: Arc<AtomicUsize>,
    healthy: Arc<AtomicBool>,
    time: Arc<AtomicI64>,
}

impl Lab {
    fn new() -> Lab {
        let runs = Arc::new(AtomicUsize::new(0));
        let healthy = Arc::new(AtomicBool::new(false));
        let time = Arc::new(AtomicI64::new(NOW));
        let t = time.clone();
        let mut b = Backend::in_memory(
            runner(runs.clone(), healthy.clone()),
            Box::new(move || t.load(SeqCst)),
        )
        .unwrap();
        let r = runs.clone();
        b.map_app(move |app| {
            app.with_platform(Platform::Linux)
                .with_registry(Registry::default().with("net.dns.flush_cache", Box::new(Fix(r))))
        });
        Lab {
            b,
            runs,
            healthy,
            time,
        }
    }

    /// A DNS ticket with a pending plan: (ticket id, plan id).
    fn dns_plan(&mut self) -> (String, i64) {
        let id = self.b.start_ticket("connectivity").unwrap().ticket.id;
        let plan = self.b.diagnose(&id).unwrap().plan.expect("DNS plan");
        (id, plan.plan_id)
    }

    fn s1() -> Vec<String> {
        vec!["s1".to_string()]
    }
}

type Attack = fn(&mut Lab) -> Result<(), String>;

fn err<T>(r: hmd_ipc::IpcResult<T>) -> Result<(), String> {
    match r {
        Ok(_) => Ok(()),
        Err(e) => Err(e.code),
    }
}

/// Each attack returns `Err(code)` when the backend refused its last step.
const ATTACKS: &[(&str, Attack)] = &[
    ("execute without approval", |l| {
        let (id, plan) = l.dns_plan();
        err(l.b.execute(&id, plan))
    }),
    ("approve a step that is not in the plan", |l| {
        let (id, plan) = l.dns_plan();
        err(l.b.approve(&id, plan, &["s99".to_string()]))?;
        err(l.b.execute(&id, plan))
    }),
    ("approve nothing, then execute", |l| {
        let (id, plan) = l.dns_plan();
        err(l.b.approve(&id, plan, &[]))?;
        err(l.b.execute(&id, plan))
    }),
    ("approve a plan id that does not exist", |l| {
        let (id, plan) = l.dns_plan();
        err(l.b.approve(&id, plan + 1, &Lab::s1()))
    }),
    ("approve with extreme plan ids", |l| {
        let (id, _) = l.dns_plan();
        for p in [i64::MIN, -1, 0, i64::MAX] {
            if l.b.approve(&id, p, &Lab::s1()).is_ok() {
                return Ok(());
            }
        }
        Err("all refused".into())
    }),
    ("approve another ticket's plan", |l| {
        let (_, plan) = l.dns_plan();
        let other = l.b.start_ticket("storage").unwrap().ticket.id;
        l.b.diagnose(&other).unwrap();
        err(l.b.approve(&other, plan, &Lab::s1()))
    }),
    ("execute another ticket's approved plan", |l| {
        let (id, plan) = l.dns_plan();
        l.b.approve(&id, plan, &Lab::s1()).unwrap();
        let other = l.b.start_ticket("storage").unwrap().ticket.id;
        l.b.diagnose(&other).unwrap();
        err(l.b.execute(&other, plan))
    }),
    ("forged ticket ids", |l| {
        let (_, plan) = l.dns_plan();
        for id in [
            "",
            "0",
            "-1",
            "1 OR 1=1",
            "1; DROP TABLE ticket",
            "../1",
            "１",
        ] {
            if l.b.approve(id, plan, &Lab::s1()).is_ok() || l.b.execute(id, plan).is_ok() {
                return Ok(());
            }
        }
        Err("all refused".into())
    }),
    ("approve after declining", |l| {
        let (id, plan) = l.dns_plan();
        l.b.decline(&id, plan).unwrap();
        err(l.b.approve(&id, plan, &Lab::s1()))
    }),
    ("execute an expired approval", |l| {
        let (id, plan) = l.dns_plan();
        l.b.approve(&id, plan, &Lab::s1()).unwrap();
        l.time.fetch_add(APPROVAL_TTL + 1, SeqCst);
        err(l.b.execute(&id, plan))
    }),
    ("observe only: nothing to approve", |l| {
        l.b.set_autonomy("observe_only").unwrap();
        let id = l.b.start_ticket("connectivity").unwrap().ticket.id;
        let d = l.b.diagnose(&id).unwrap();
        if d.plan.as_ref().is_some_and(|p| !p.steps.is_empty()) {
            return Ok(());
        }
        for p in [0, 1, 2] {
            if l.b.approve(&id, p, &Lab::s1()).is_ok() {
                return Ok(());
            }
        }
        Err("no fix offered".into())
    }),
    ("switch to observe only after approving", |l| {
        let (id, plan) = l.dns_plan();
        l.b.approve(&id, plan, &Lab::s1()).unwrap();
        l.b.set_autonomy("observe_only").unwrap();
        err(l.b.execute(&id, plan))
    }),
    ("old plan after a new diagnosis found nothing", |l| {
        let (id, old) = l.dns_plan();
        l.healthy.store(true, SeqCst);
        assert!(l.b.diagnose(&id).unwrap().plan.is_none());
        err(l.b.approve(&id, old, &Lab::s1()))?;
        err(l.b.execute(&id, old))
    }),
    (
        "approved old plan after a new diagnosis found nothing",
        |l| {
            let (id, old) = l.dns_plan();
            l.b.approve(&id, old, &Lab::s1()).unwrap();
            l.healthy.store(true, SeqCst);
            l.b.diagnose(&id).map_err(|e| e.code)?;
            err(l.b.execute(&id, old))
        },
    ),
    ("approved plan after a new diagnosis", |l| {
        let (id, old) = l.dns_plan();
        l.b.approve(&id, old, &Lab::s1()).unwrap();
        l.b.diagnose(&id).map_err(|e| e.code)?;
        err(l.b.execute(&id, old))
    }),
    ("plans of an erased history", |l| {
        let (id, plan) = l.dns_plan();
        l.b.approve(&id, plan, &Lab::s1()).unwrap();
        l.b.erase_all().unwrap();
        err(l.b.execute(&id, plan))
    }),
    ("auto simple: execute without approval", |l| {
        l.b.set_autonomy("auto_simple").unwrap();
        let (id, plan) = l.dns_plan();
        err(l.b.execute(&id, plan))
    }),
    ("auto simple: guessed plan ids without approval", |l| {
        l.b.set_autonomy("auto_simple").unwrap();
        let id = l.b.start_ticket("connectivity").unwrap().ticket.id;
        l.b.diagnose(&id).unwrap();
        for p in [0, 1, 2] {
            if l.b.execute(&id, p).is_ok() {
                return Ok(());
            }
        }
        Err("all refused".into())
    }),
    ("save a huge report into Downloads", |l| {
        let (id, _) = l.dns_plan();
        let report = l.b.make_report(&id, None).unwrap();
        let dir = std::env::temp_dir().join(format!("hmd-adv-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let huge = "x".repeat(hmd_app::MAX_REPORT_BYTES + 1);
        let result = err(l.b.save_report(report.id, &huge, &dir));
        let written = std::fs::read_dir(&dir).unwrap().count();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(written, 0, "nothing reaches the disk");
        result
    }),
];

#[test]
fn every_attack_is_blocked_and_the_fix_never_runs() {
    let mut escaped = Vec::new();
    for (name, attack) in ATTACKS {
        let mut lab = Lab::new();
        let refused = attack(&mut lab).is_err();
        let runs = lab.runs.load(SeqCst);
        if !refused || runs != 0 {
            escaped.push(format!("{name}: refused={refused} fix_runs={runs}"));
        }
    }
    assert!(
        escaped.is_empty(),
        "{} of {} attacks escaped:\n{}",
        escaped.len(),
        ATTACKS.len(),
        escaped.join("\n")
    );
    assert_eq!(
        ATTACKS.len(),
        19,
        "update the count in docs when adding cases"
    );
}

#[test]
fn control_the_lab_really_runs_the_fix_once() {
    let mut lab = Lab::new();
    let (id, plan) = lab.dns_plan();
    lab.b.approve(&id, plan, &Lab::s1()).unwrap();
    assert_eq!(lab.b.execute(&id, plan).unwrap().kind, "verified");
    assert_eq!(lab.runs.load(SeqCst), 1);
    assert!(
        lab.b.execute(&id, plan).is_err(),
        "an approval is used once"
    );
    assert_eq!(lab.runs.load(SeqCst), 1);
}
