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
/// The platform these tests pretend to be running on. It is a constant, not the real one, so the
/// same test runs on any machine: what is under test is the helper's decision, not the host.
const HOST: Platform = Platform::Linux;

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
        host: HOST,
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
        validate(&request, embedded().unwrap(), &allowlist, NOW, HOST).unwrap_err(),
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
        validate(&wrong_hash, catalog, &allowlist, NOW, HOST).unwrap_err(),
        ElevatedError::PlanMismatch
    );

    let mut edited_plan = plan.request();
    edited_plan.plan = edited_plan.plan.replace("catalog=", "catalog=x");
    assert_eq!(
        validate(&edited_plan, catalog, &allowlist, NOW, HOST).unwrap_err(),
        ElevatedError::PlanMismatch
    );

    // The request's parameters have to be the parameters of that step in the plan.
    let mut extra_param = plan.request();
    extra_param
        .params
        .insert("dns.server".into(), "1.1.1.1".into());
    assert_eq!(
        validate(&extra_param, catalog, &allowlist, NOW, HOST).unwrap_err(),
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
        validate(&request, catalog, &allowlist, NOW, HOST).unwrap_err(),
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
    // A verdict that does not ask for elevation is not a platform problem: the plan itself is not
    // what was approved for a privileged run.
    assert_eq!(
        validate(&demoted, catalog, &allowlist, NOW, HOST).unwrap_err(),
        ElevatedError::PlanMismatch
    );
}

/// US2.3: the platform in the request also has to be the platform this helper is running on. A
/// token built for another system cannot pick the variant whose rules suit it better (and the
/// allowlist, keyed by capability and version, stays unambiguous).
#[test]
fn us2_3_a_request_for_another_platform_is_refused() {
    let plan = Plan::new();
    let catalog = embedded().unwrap();
    let (allowlist, _) = spy_allowlist();
    let other = if HOST == Platform::Windows {
        Platform::Linux
    } else {
        Platform::Windows
    };

    let mut request = plan.request();
    request.platform = if other == Platform::Windows {
        "windows"
    } else {
        "linux"
    }
    .into();
    assert_eq!(
        validate(&request, catalog, &allowlist, NOW, HOST).unwrap_err(),
        ElevatedError::PlatformNotSupported
    );
}

/// SEC-028: the token does not get to decide how long it stays alive. An approval is valid for ten
/// minutes, so a request that claims more than that is refused however it was built.
#[test]
fn sec_028_a_request_cannot_buy_itself_more_than_an_approval() {
    let plan = Plan::new();
    let catalog = embedded().unwrap();
    let (allowlist, _) = spy_allowlist();

    let mut request = plan.request();
    request.expires_at = NOW + 86_400;
    assert_eq!(
        validate(&request, catalog, &allowlist, NOW, HOST).unwrap_err(),
        ElevatedError::DeadlineTooLong
    );

    let mut reopened = plan.request();
    reopened.expires_at = NOW + 86_400;
    let token = encode_token(&reopened).unwrap();
    let (exe, dir) = installed();
    let (code, result) = hmd_elevated::run_request(&token, &context(&allowlist, &exe, &dir), NOW);
    assert_eq!(code, 18);
    assert_eq!(
        result.reason_code.as_deref(),
        Some("REQUEST_DEADLINE_TOO_LONG")
    );
}

/// SEC-023: the person approved the risks and the verification the catalog said at the time. A plan
/// written against another catalog version is not that plan.
#[test]
fn sec_023_a_plan_from_another_catalog_version_is_refused() {
    let plan = Plan::new();
    let catalog = embedded().unwrap();
    let (allowlist, _) = spy_allowlist();

    let mut other_version = plan.request();
    other_version.plan = other_version
        .plan
        .lines()
        .map(|line| {
            if line.starts_with("catalog=") {
                "catalog=0.0.0-de-outra-versao".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
        + "\n";
    other_version.plan_hash = hmd_canonical::sha256_tagged(&other_version.plan);
    assert_eq!(
        validate(&other_version, catalog, &allowlist, NOW, HOST).unwrap_err(),
        ElevatedError::CatalogMismatch
    );
}

/// SEC-022: the transport the launcher may ask for is part of the protocol and is checked, so a
/// token cannot point the result of a privileged run somewhere else.
#[test]
fn sec_022_the_result_pipe_of_the_request_has_one_shape() {
    let plan = Plan::new();
    let catalog = embedded().unwrap();
    let (allowlist, _) = spy_allowlist();

    let mut request = plan.request();
    request.result_pipe = Some(r"\\.\pipe\helpmydesk-7f3a".into());
    assert!(
        validate(&request, catalog, &allowlist, NOW, HOST).is_ok(),
        "the shape the launcher will use is accepted"
    );

    for pipe in [r"\\.\pipe\outro", r"\\\\servidor\pipe\x", r"\\.\pipe\a b"] {
        let mut request = plan.request();
        request.result_pipe = Some(pipe.into());
        assert_eq!(
            validate(&request, catalog, &allowlist, NOW, HOST).unwrap_err(),
            ElevatedError::BadRequest,
            "{pipe}"
        );
    }
}

/// FR-004/SEC-022: one request, one token. A token that decodes to the same bytes but was not
/// written the way this helper writes them is refused, so nothing downstream can be fooled by two
/// spellings of the same request.
#[test]
fn fr_004_one_request_has_one_token() {
    let plan = Plan::new();
    let token = encode_token(&plan.request()).unwrap();
    assert!(decode_token(&token).is_ok());
    // `YQ==` and `YR==` are the same byte; only the first is the spelling this helper writes.
    assert!(hmd_elevated::base64url_decode("YR==").is_err());
    assert!(hmd_elevated::base64url_decode("YQ==").is_ok());
}

/// FR-014: the result goes to the journal, so it has a size limit that actually applies.
#[test]
fn fr_014_a_result_over_the_cap_is_replaced_by_a_code_that_fits() {
    let huge = hmd_elevated::ElevatedResult {
        protocol: hmd_elevated::PROTOCOL.into(),
        outcome: Outcome::Failed,
        reason_code: Some("FIX_FAILED".into()),
        detail: Some("x".repeat(hmd_elevated::MAX_RESULT_BYTES * 2)),
    };
    let json = result_json(&huge);
    assert!(json.len() <= hmd_elevated::MAX_RESULT_BYTES, "{json}");
    assert!(
        json.contains(r#""reason_code":"RESULT_TOO_LARGE""#),
        "{json}"
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
        validate(
            &decode_token(&token).unwrap(),
            catalog,
            &allowlist,
            NOW,
            HOST
        )
        .unwrap_err(),
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
        validate(&late, catalog, &allowlist, NOW, HOST).unwrap_err(),
        ElevatedError::Expired
    );
    // One second earlier it is still alive: the deadline is exclusive.
    assert!(validate(&plan.request(), catalog, &allowlist, DEADLINE - 1, HOST).is_ok());
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

/// SEC-027: the helper is ephemeral — one action per invocation, and the process holds nothing
/// between them. What this checks is that two invocations are two independent actions (the request
/// is not consumed by a resident daemon); it is not a claim that a token cannot be replayed, which
/// SEC-028 bounds by time and the person's click, not by the token.
#[test]
fn sec_027_two_invocations_are_two_independent_actions() {
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

/// SEC-025: a refusal from the helper carries its own code, and that code is what the journal
/// keeps. Flattening every refusal into "elevation not available" would hide a security event.
#[test]
fn sec_025_a_refusal_from_the_helper_reaches_the_journal_with_its_own_code() {
    struct Refuses(&'static str);
    impl hmd_elevated::Launcher for Refuses {
        fn available(&self) -> bool {
            true
        }
        fn platform(&self) -> Platform {
            Platform::Linux
        }
        fn launch(
            &self,
            _request: &ElevatedRequest,
            _timeout: std::time::Duration,
        ) -> Result<hmd_elevated::ElevatedResult, hmd_elevated::LaunchError> {
            Err(hmd_elevated::LaunchError::Refused(self.0))
        }
    }

    let plan = Plan::new();
    for code in [
        "HELPER_PATH_NOT_TRUSTED",
        "NOT_IN_ALLOWLIST",
        "REQUEST_EXPIRED",
    ] {
        let fix = ElevatedFix::new(Box::new(Refuses(code)));
        assert_eq!(
            fix.run(plan.step(), &plan.evaluated(), Platform::Linux, DEADLINE)
                .unwrap_err(),
            ExecError::ElevationRefused(code),
            "{code}"
        );
        assert_eq!(ExecError::ElevationRefused(code).reason_code(), code);
    }
}

/// FR-015: on a platform with no privileged path, every elevated step is refused.
#[test]
fn fr_015_no_elevation_path_on_this_platform() {
    let plan = Plan::new();
    let launcher = hmd_elevated::production_launcher();
    assert!(!launcher.available(), "no UAC or pkexec path in this build");
    assert_eq!(
        launcher
            .launch(&plan.request(), std::time::Duration::from_millis(1))
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
