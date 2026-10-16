// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 020 acceptance tests for the elevated helper. Everything runs as a normal user: the
//! security decisions of the helper are pure functions, so they are exercised directly with
//! forged requests, a broken allowlist and an untrusted path.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use hmd_canonical::{canonical, plan_hash};
use hmd_catalog::embedded;
use hmd_elevated::{
    ActionResult, Allowlist, ElevatedError, ElevatedFix, ElevatedRequest, FakeLauncher,
    HelperContext, Outcome, PROTOCOL, decode_token, encode_token, result_json, trusted_path,
    validate,
};
use hmd_executor::{Elevation, ExecError, FixOutcome, Registry};
use hmd_policy::{
    ActionPlanProposal, Confidence, Platform, PolicyContext, PolicyDecision, ProposedStep, evaluate,
};

/// The only `fix` in the catalog, and the only one that declares `requires_elevation`.
const FIX: &str = "net.dns.flush_cache";
/// The clock of the user-level side, fixed so the tests do not depend on the wall clock.
const NOW: hmd_domain::Timestamp = 1_760_400_000;
/// The request carries the deadline of the approval; the helper compares it with its own clock.
const DEADLINE: hmd_domain::Timestamp = NOW + 300;

/// An action that reports what it was asked to do, so the tests can see that exactly one action ran
/// with the validated step. Shared with the allowlist through an `Arc`, the way the real allowlist
/// holds its implementations.
#[derive(Default)]
struct Spy {
    runs: std::sync::Mutex<Vec<(String, BTreeMap<String, String>)>>,
}

impl Spy {
    fn new() -> std::sync::Arc<Spy> {
        std::sync::Arc::new(Spy::default())
    }
    fn seen(&self) -> Vec<(String, BTreeMap<String, String>)> {
        self.runs.lock().unwrap().clone()
    }
}

impl hmd_elevated::ElevatedAction for Spy {
    fn run(&self, step: &hmd_elevated::ValidatedStep) -> ActionResult {
        self.runs
            .lock()
            .unwrap()
            .push((step.capability.clone(), step.params.clone()));
        ActionResult {
            outcome: Outcome::Succeeded,
            reason_code: None,
        }
    }
}

/// A spy that always fails, to check the `failed` path and its exit code.
struct AlwaysFails;

impl hmd_elevated::ElevatedAction for AlwaysFails {
    fn run(&self, _step: &hmd_elevated::ValidatedStep) -> ActionResult {
        ActionResult {
            outcome: Outcome::Failed,
            reason_code: Some("FIX_FAILED"),
        }
    }
}

struct Plan {
    proposal: ActionPlanProposal,
    decision: PolicyDecision,
}

impl Plan {
    fn new() -> Plan {
        let catalog = embedded().unwrap();
        let proposal = ActionPlanProposal {
            session_id: "s-1".into(),
            steps: vec![ProposedStep {
                step_id: "s1".into(),
                capability: FIX.into(),
                version: catalog.catalog.get(FIX).unwrap().version.clone(),
                params: vec![],
            }],
        };
        let mut ctx = PolicyContext::new(Platform::Linux);
        ctx.findings.insert("net.dns.resolution_failing".into());
        ctx.confidence = Confidence::Probable;
        let decision = evaluate(&proposal, &catalog.catalog, &ctx).unwrap();
        assert!(decision.verdicts[0].is_accepted());
        Plan { proposal, decision }
    }

    fn evaluated(&self) -> hmd_executor::EvaluatedPlan<'_> {
        hmd_executor::EvaluatedPlan {
            proposal: &self.proposal,
            decision: &self.decision,
            catalog_version: &embedded().unwrap().version,
        }
    }

    fn request(&self) -> ElevatedRequest {
        hmd_elevated::build_request(
            &self.evaluated(),
            &self.proposal.steps[0],
            Platform::Linux,
            None,
            DEADLINE,
        )
        .unwrap()
    }

    fn step(&self) -> &ProposedStep {
        &self.proposal.steps[0]
    }
}

/// An allowlist with exactly one allowlisted action, and the handle to read what it ran.
fn spy_allowlist() -> (Allowlist, std::sync::Arc<Spy>) {
    let spy = Spy::new();
    let allowlist = Allowlist::production().with(FIX, "1.0.0", Box::new(spy.clone()));
    (allowlist, spy)
}

fn context<'a>(
    allowlist: &'a Allowlist,
    exe: &'a std::path::Path,
    dir: &'a std::path::Path,
) -> HelperContext<'a> {
    HelperContext {
        exe,
        install_dir: dir,
        catalog: embedded().unwrap(),
        allowlist,
    }
}

fn installed() -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = std::path::PathBuf::from("/opt/HelpMyDesk");
    let exe = dir.join("helpmydesk-elevated");
    (exe, dir)
}

/// US2.1 / SEC-021: an action that is not in the compiled allowlist of this build never runs, no
/// matter how well-formed the request is.
#[test]
fn us2_1_only_the_compiled_allowlist_runs() {
    let plan = Plan::new();
    let request = plan.request();
    let allowlist = Allowlist::production();
    assert!(
        allowlist.is_empty(),
        "production allowlist starts empty (S-02)"
    );
    assert_eq!(
        validate(&request, embedded().unwrap(), &allowlist, NOW).unwrap_err(),
        ElevatedError::NotInAllowlist
    );
}

/// US2.2 / FR-006: the helper recalculates the hash, so a plan edited after the approval is refused.
#[test]
fn us2_2_tampered_plan_or_hash_is_refused() {
    let plan = Plan::new();
    let catalog = embedded().unwrap();
    let (allowlist, _) = spy_allowlist();

    let mut wrong_hash = plan.request();
    wrong_hash.plan_hash = "sha256:".to_string() + &"0".repeat(64);
    assert_eq!(
        validate(&wrong_hash, catalog, &allowlist, NOW).unwrap_err(),
        ElevatedError::PlanMismatch
    );

    let mut edited_plan = plan.request();
    edited_plan.plan = edited_plan.plan.replace("catalog=", "catalog=x");
    assert_eq!(
        validate(&edited_plan, catalog, &allowlist, NOW).unwrap_err(),
        ElevatedError::PlanMismatch
    );

    // The request's parameters have to be the parameters of that step in the plan.
    let mut extra_param = plan.request();
    extra_param
        .params
        .insert("dns.server".into(), "1.1.1.1".into());
    assert_eq!(
        validate(&extra_param, catalog, &allowlist, NOW).unwrap_err(),
        ElevatedError::PlanMismatch
    );
}

/// US2.3: the platform has to be one the catalog knows, and the catalog has to declare that this
/// variant needs elevation. A request that claims otherwise is refused.
#[test]
fn us2_3_unknown_platform_or_elevation_not_declared_is_refused() {
    let plan = Plan::new();
    let catalog = embedded().unwrap();
    let (allowlist, _) = spy_allowlist();

    let mut request = plan.request();
    request.platform = "macos".into();
    assert_eq!(
        validate(&request, catalog, &allowlist, NOW).unwrap_err(),
        ElevatedError::PlatformNotSupported
    );

    // Same request, but with the elevation flag of the verdict set to 0 and the hash recomputed:
    // only the declaration of the plan can decide this, and the catalog says the step needs it.
    let mut demoted = plan.request();
    demoted.plan = demoted
        .plan
        .lines()
        .map(|line| {
            if line.starts_with("verdict=s1|accepted|") {
                let mut fields: Vec<&str> = line.split('|').collect();
                fields[4] = "0";
                fields.join("|")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
        + "\n";
    demoted.plan_hash = hmd_canonical::sha256_tagged(&demoted.plan);
    assert_eq!(
        validate(&demoted, catalog, &allowlist, NOW).unwrap_err(),
        ElevatedError::PlatformNotSupported
    );
}

/// US2.4 / SEC-026: one action per invocation, with the step the helper validated itself.
#[test]
fn us2_4_one_action_per_invocation_with_the_validated_step() {
    let plan = Plan::new();
    let (allowlist, spy) = spy_allowlist();
    let (exe, dir) = installed();
    let token = encode_token(&plan.request()).unwrap();
    let (code, result) = hmd_elevated::run_request(&token, &context(&allowlist, &exe, &dir), NOW);
    assert_eq!(code, 0);
    assert_eq!(result.outcome, Outcome::Succeeded);
    assert_eq!(spy.seen(), vec![(FIX.to_string(), BTreeMap::new())]);
}

/// SEC-025: the result is JSON with the protocol version, and a failing action has its own exit code.
#[test]
fn sec_025_result_is_json_with_a_stable_exit_code() {
    let plan = Plan::new();
    let allowlist = Allowlist::production().with(FIX, "1.0.0", Box::new(AlwaysFails));
    let (exe, dir) = installed();
    let token = encode_token(&plan.request()).unwrap();
    let (code, result) = hmd_elevated::run_request(&token, &context(&allowlist, &exe, &dir), NOW);
    assert_eq!(code, ElevatedError::Failed.exit_code());
    assert_eq!(result.reason_code.as_deref(), Some("FIX_FAILED"));
    let json = result_json(&result);
    assert!(json.contains(r#""protocol":"hmd-elevated@1""#), "{json}");
    assert!(json.contains(r#""outcome":"failed""#), "{json}");
    assert!(
        json.len() <= hmd_elevated::MAX_RESULT_BYTES,
        "the result stays small: {json}"
    );
    // Every refusal has an exit code the launcher can translate back.
    for error in [
        ElevatedError::RequestTooLarge,
        ElevatedError::BadRequest,
        ElevatedError::NotInAllowlist,
        ElevatedError::PlanMismatch,
        ElevatedError::PlatformNotSupported,
        ElevatedError::InvalidParams,
        ElevatedError::HelperPathNotTrusted,
    ] {
        assert_eq!(
            ElevatedError::from_exit_code(error.exit_code()),
            Some(error.reason_code()),
            "{error:?}"
        );
    }
}

/// US3.1 / SEC-020: the helper refuses to run from anywhere but the per-machine install folder.
#[test]
fn us3_1_helper_refuses_to_run_outside_the_install_folder() {
    let plan = Plan::new();
    let (allowlist, _) = spy_allowlist();
    let (_, dir) = installed();
    let elsewhere = std::path::PathBuf::from("/home/pessoa/Downloads/helpmydesk-elevated");
    let token = encode_token(&plan.request()).unwrap();
    let (code, result) =
        hmd_elevated::run_request(&token, &context(&allowlist, &elsewhere, &dir), NOW);
    assert_eq!(code, ElevatedError::HelperPathNotTrusted.exit_code());
    assert_eq!(
        result.reason_code.as_deref(),
        Some("HELPER_PATH_NOT_TRUSTED")
    );
    assert!(!trusted_path(&elsewhere, &dir));
    let (exe, dir) = installed();
    assert!(trusted_path(&exe, &dir));
}

/// FR-003/FR-004/SEC-022: the request is one base64url token with a closed schema and a size cap.
#[test]
fn fr_003_request_is_one_closed_base64url_token() {
    let plan = Plan::new();
    let request = plan.request();
    let token = encode_token(&request).unwrap();
    assert!(
        token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'='),
        "no space, quote or backslash for a command-line parser to reinterpret"
    );
    assert_eq!(decode_token(&token).unwrap(), request);

    // Unknown field: the schema is closed.
    let mut extra = request.clone();
    extra.result_pipe = Some(r"\\.\pipe\x".into());
    let json = serde_json::to_string(&extra).unwrap();
    let with_unknown = json.replace(r#""result_pipe""#, r#""secrets":1,"result_pipe""#);
    assert_eq!(
        decode_token(&hmd_elevated::base64url_encode(with_unknown.as_bytes())).unwrap_err(),
        ElevatedError::BadRequest
    );

    // Unknown protocol version: refused (contract versioning).
    let mut wrong = request.clone();
    wrong.protocol = "hmd-elevated@2".into();
    let token = encode_token(&wrong).unwrap();
    let catalog = embedded().unwrap();
    let allowlist = Allowlist::production();
    assert_eq!(
        validate(&decode_token(&token).unwrap(), catalog, &allowlist, NOW).unwrap_err(),
        ElevatedError::ProtocolMismatch
    );

    // Size cap (FR-004).
    let mut big = request.clone();
    big.plan
        .push_str(&"x".repeat(hmd_elevated::MAX_REQUEST_BYTES));
    assert_eq!(
        encode_token(&big).unwrap_err(),
        ElevatedError::RequestTooLarge
    );
}

/// FR-006: the hash the helper recomputes is the same one the executor produced, because both come
/// from the same encoding.
#[test]
fn fr_006_helper_and_executor_agree_on_the_hash() {
    let plan = Plan::new();
    let version = embedded().unwrap().version.clone();
    assert_eq!(
        plan.request().plan_hash,
        plan_hash(&plan.proposal, &plan.decision, &version)
    );
    assert_eq!(
        plan.request().plan,
        canonical(&plan.proposal, &plan.decision, &version)
    );
}

/// FR-003 / US4: a UAC dialog answered late runs nothing. The request carries the second after
/// which it is dead, and the helper compares it with its own clock before anything else.
#[test]
fn fr_003_a_request_that_expired_runs_nothing() {
    let plan = Plan::new();
    let catalog = embedded().unwrap();
    let (allowlist, spy) = spy_allowlist();
    let (exe, dir) = installed();

    let mut late = plan.request();
    late.expires_at = NOW;
    let (code, result) = hmd_elevated::run_request(
        &encode_token(&late).unwrap(),
        &context(&allowlist, &exe, &dir),
        NOW,
    );
    assert_eq!(code, ElevatedError::Expired.exit_code());
    assert_eq!(result.reason_code.as_deref(), Some("REQUEST_EXPIRED"));
    assert!(spy.seen().is_empty(), "nothing ran");

    assert_eq!(
        validate(&late, catalog, &allowlist, NOW).unwrap_err(),
        ElevatedError::Expired
    );
    // One second earlier it is still alive: the deadline is exclusive.
    assert!(validate(&plan.request(), catalog, &allowlist, DEADLINE - 1).is_ok());
}

/// US1.1/US1.3/FR-010 through the executor: no elevation path in this build means refusal before the
/// journal; a refused dialog is `failed`; a timeout is `unknown`.
#[test]
fn us1_elevation_outcomes_through_the_executor() {
    let plan = Plan::new();
    let step = plan.step();
    let evaluated = plan.evaluated();

    // US1.1: the default registry has no elevation path.
    assert!(
        !Registry::production()
            .elevation()
            .available(step, Platform::Linux)
    );
    assert_eq!(
        Registry::production()
            .elevation()
            .run(step, &evaluated, Platform::Linux, DEADLINE)
            .unwrap_err(),
        ExecError::ElevationNotAvailable
    );

    // US1.3: the person refused the dialog.
    let mut launcher = FakeLauncher::refusing();
    launcher.elapsed = Some(std::time::Duration::from_millis(1));
    let fix = ElevatedFix::new(Box::new(launcher));
    assert_eq!(
        fix.run(step, &evaluated, Platform::Linux, DEADLINE)
            .unwrap_err(),
        ExecError::ElevationDenied
    );

    // FR-010: the deadline expired with the dialog open.
    let fix = ElevatedFix::new(Box::new(FakeLauncher::refusing()));
    assert_eq!(
        fix.run(step, &evaluated, Platform::Linux, DEADLINE)
            .unwrap_err(),
        ExecError::ElevationTimeout
    );

    // A refusal from the helper is reported as not available, never as success.
    let mut launcher = FakeLauncher::refusing();
    launcher.elapsed = Some(std::time::Duration::from_millis(1));
    launcher.answer = Some(hmd_elevated::ElevatedResult {
        protocol: PROTOCOL.into(),
        outcome: Outcome::Failed,
        reason_code: Some("NOT_IN_ALLOWLIST".into()),
        detail: None,
    });
    let fix = ElevatedFix::new(Box::new(launcher));
    assert_eq!(
        fix.run(step, &evaluated, Platform::Linux, DEADLINE)
            .unwrap(),
        FixOutcome::Failed("NOT_IN_ALLOWLIST")
    );
}

/// SEC-027: the helper is ephemeral — one action, then the process is gone; nothing keeps running
/// with administrator rights.
#[test]
fn sec_027_nothing_resident_after_the_invocation() {
    let plan = Plan::new();
    let (allowlist, spy) = spy_allowlist();
    let (exe, dir) = installed();
    let token = encode_token(&plan.request()).unwrap();
    let ctx = context(&allowlist, &exe, &dir);
    assert_eq!(hmd_elevated::run_request(&token, &ctx, NOW).0, 0);
    assert_eq!(hmd_elevated::run_request(&token, &ctx, NOW).0, 0);
    // Two invocations, one action each: the process holds nothing between them.
    assert_eq!(spy.seen().len(), 2);
}

/// FR-015: on a platform with no privileged path, every elevated step is refused.
#[test]
fn fr_015_no_elevation_path_on_this_platform() {
    let plan = Plan::new();
    let launcher = hmd_elevated::production_launcher();
    assert!(!launcher.available(), "no UAC or pkexec path in this build");
    assert_eq!(
        launcher
            .launch("whatever", std::time::Duration::from_millis(1))
            .unwrap_err()
            .reason_code(),
        "ELEVATION_NOT_AVAILABLE"
    );
    let fix = ElevatedFix::new(hmd_elevated::production_launcher());
    assert_eq!(
        fix.run(plan.step(), &plan.evaluated(), Platform::Linux, DEADLINE)
            .unwrap_err(),
        ExecError::ElevationNotAvailable
    );
}
