// SPDX-License-Identifier: GPL-3.0-or-later
//! HelpMyDesk elevated helper (spec 020, Plan v1 V.4 alternative B): one privileged action per
//! invocation, an allowlist compiled into this build, and a request that arrives on the command
//! line of the elevated process.
//!
//! Two sides live here on purpose, because the protocol is the trust boundary:
//!
//! * the **helper** side ([`run_request`], the `helpmydesk-elevated` binary) is the only code in
//!   the project that runs as administrator. It trusts nothing it receives: it re-reads the
//!   canonical plan, recalculates the hash, revalidates the step against the embedded catalog and
//!   runs at most one action of its own compiled allowlist;
//! * the **launcher** side ([`Launcher`], [`FakeLauncher`], and the Windows UAC launcher) is the
//!   user-level process that asks the operating system for permission and collects the result.
//!
//! The declared limit, accepted in Plan v1: malware running as the person can start the helper with
//! any allowlisted action and rely on one click in the UAC dialog, which shows the publisher and not
//! the action. What bounds the damage is the allowlist: small, harmless by design, and it only grows
//! with a security review (spike S-02 decides the first entries).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use hmd_canonical::{PLAN_FORMAT, parse_line, sha256_tagged};
use hmd_catalog::LoadedCatalog;
use hmd_executor::{
    ELEVATION_TIMEOUT, Elevation, EvaluatedPlan, ExecError, FixOutcome, MAX_PARAM_LEN, Registry,
};
use hmd_policy::{Platform, ProposedStep, RiskLevel};
use serde::{Deserialize, Serialize};

/// Version of the request/result contract (spec 020 FR-002). Unknown versions are refused.
pub const PROTOCOL: &str = "hmd-elevated@1";

/// FR-004: the decoded request may not exceed this size.
pub const MAX_REQUEST_BYTES: usize = 4 * 1024;

/// FR-014: the result the helper writes back is small on purpose; it goes to a journal that people
/// can read.
pub const MAX_RESULT_BYTES: usize = 512;

/// How long the elevated process may take, counting the wait in the system dialog (FR-010).
pub const TIMEOUT: std::time::Duration = ELEVATION_TIMEOUT;

/// The request, as it travels in the command line of the elevated process (SEC-022). Closed schema:
/// an unknown field is a refusal, and the size is capped before anything is parsed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElevatedRequest {
    pub protocol: String,
    /// The canonical text of the plan the person approved (`hmd-plan@1`).
    pub plan: String,
    /// `sha256:<64 hex>` of `plan`, recomputed by the helper (FR-006).
    pub plan_hash: String,
    pub step_id: String,
    pub capability: String,
    pub capability_version: String,
    pub platform: String,
    /// Already validated against the catalog's parameter list; re-checked anyway (FR-007).
    pub params: BTreeMap<String, String>,
    /// Where to write the result (Windows named pipe name). Absent means stdout only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_pipe: Option<String>,
}

/// What the helper reports back (SEC-025).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElevatedResult {
    pub protocol: String,
    pub outcome: Outcome,
    /// Stable code: `ELEVATION_DENIED`, `INVALID_PARAMS`, and so on.
    pub reason_code: Option<String>,
    /// Short and sanitized: no probe output, no path, no name of the person (FR-014).
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Succeeded,
    Failed,
    Unknown,
}

/// Every refusal the helper can make, with the exit code that goes with it (SEC-025).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElevatedError {
    /// The helper is not running from the per-machine install folder (SEC-020).
    HelperPathNotTrusted,
    /// No token on the command line, or it is not base64url.
    BadRequest,
    /// Decoded request above [`MAX_REQUEST_BYTES`] (FR-004).
    RequestTooLarge,
    /// `protocol` is not [`PROTOCOL`].
    ProtocolMismatch,
    /// The canonical text does not match the declared hash, or does not contain the step (FR-006).
    PlanMismatch,
    /// Not in the compiled allowlist of this build (SEC-021).
    NotInAllowlist,
    /// The capability has no variant for this platform, or is not declared as elevated (FR-007).
    PlatformNotSupported,
    /// Parameters outside what the catalog allows (FR-007).
    InvalidParams,
    /// The action ran and did not work.
    Failed,
    /// The action cannot tell whether it happened.
    Unknown,
}

impl ElevatedError {
    pub fn reason_code(&self) -> &'static str {
        match self {
            ElevatedError::HelperPathNotTrusted => "HELPER_PATH_NOT_TRUSTED",
            ElevatedError::BadRequest => "BAD_REQUEST",
            ElevatedError::RequestTooLarge => "REQUEST_TOO_LARGE",
            ElevatedError::ProtocolMismatch => "PROTOCOL_MISMATCH",
            ElevatedError::PlanMismatch => "PLAN_HASH_MISMATCH",
            ElevatedError::NotInAllowlist => "NOT_IN_ALLOWLIST",
            ElevatedError::PlatformNotSupported => "PLATFORM_NOT_SUPPORTED",
            ElevatedError::InvalidParams => "INVALID_PARAMS",
            ElevatedError::Failed => "FIX_FAILED",
            ElevatedError::Unknown => "FIX_UNKNOWN",
        }
    }

    /// Process exit code (SEC-025). The launcher maps it back when the result never arrives.
    pub fn exit_code(&self) -> i32 {
        match self {
            ElevatedError::HelperPathNotTrusted => 16,
            ElevatedError::BadRequest | ElevatedError::ProtocolMismatch => 11,
            ElevatedError::PlanMismatch => 13,
            ElevatedError::RequestTooLarge => 10,
            ElevatedError::NotInAllowlist => 12,
            ElevatedError::PlatformNotSupported => 14,
            ElevatedError::InvalidParams => 15,
            ElevatedError::Failed => 20,
            ElevatedError::Unknown => 21,
        }
    }

    /// The same code as a refusal without a process: what the launcher reports when the request
    /// never reached the helper.
    pub fn from_exit_code(code: i32) -> Option<&'static str> {
        Some(match code {
            10 => "REQUEST_TOO_LARGE",
            11 => "BAD_REQUEST",
            12 => "NOT_IN_ALLOWLIST",
            13 => "PLAN_HASH_MISMATCH",
            14 => "PLATFORM_NOT_SUPPORTED",
            15 => "INVALID_PARAMS",
            16 => "HELPER_PATH_NOT_TRUSTED",
            _ => return None,
        })
    }
}

impl fmt::Display for ElevatedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason_code())
    }
}

impl std::error::Error for ElevatedError {}

impl ElevatedError {
    /// The result the helper writes when it refuses.
    pub fn result(&self) -> ElevatedResult {
        ElevatedResult {
            protocol: PROTOCOL.to_string(),
            outcome: Outcome::Failed,
            reason_code: Some(self.reason_code().to_string()),
            detail: None,
        }
    }
}

/// A step the helper revalidated from the request and from the catalog of this build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedStep {
    pub step_id: String,
    pub capability: String,
    pub version: String,
    pub platform: Platform,
    pub params: BTreeMap<String, String>,
}

/// What an allowlisted elevated action reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionResult {
    pub outcome: Outcome,
    pub reason_code: Option<&'static str>,
}

/// One elevated action, compiled into the helper. Implementations use native APIs only: no shell,
/// no path or command text from outside, no network (SEC-024, SEC-026).
pub trait ElevatedAction: Send + Sync {
    fn run(&self, step: &ValidatedStep) -> ActionResult;
}

/// An allowlist can hold shared implementations (the tests count what ran; the app keeps one
/// instance per action).
impl<T: ElevatedAction + ?Sized> ElevatedAction for std::sync::Arc<T> {
    fn run(&self, step: &ValidatedStep) -> ActionResult {
        ElevatedAction::run(&**self, step)
    }
}

/// The allowlist compiled into this build (SEC-021). The production one is empty until spike S-02
/// measures the privileges each fix needs; it only grows with a security review.
#[derive(Default)]
pub struct Allowlist {
    actions: BTreeMap<(String, String), Box<dyn ElevatedAction>>,
}

impl Allowlist {
    /// Empty by design: no production action runs elevated until S-02 (spec 020 FR-005).
    pub fn production() -> Allowlist {
        Allowlist::default()
    }

    pub fn with(
        mut self,
        capability: &str,
        version: &str,
        action: Box<dyn ElevatedAction>,
    ) -> Allowlist {
        self.actions
            .insert((capability.to_string(), version.to_string()), action);
        self
    }

    pub fn get(&self, capability: &str, version: &str) -> Option<&dyn ElevatedAction> {
        self.actions
            .get(&(capability.to_string(), version.to_string()))
            .map(|b| b.as_ref())
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

pub fn platform_code(platform: Platform) -> &'static str {
    match platform {
        Platform::Windows => "windows",
        Platform::Linux => "linux",
    }
}

fn platform_from_code(code: &str) -> Result<Platform, ElevatedError> {
    match code {
        "windows" => Ok(Platform::Windows),
        "linux" => Ok(Platform::Linux),
        _ => Err(ElevatedError::PlatformNotSupported),
    }
}

/// SEC-021/SEC-023: the helper revalidates the request against the catalog of this build and
/// against the canonical text it received. Nothing is taken on faith: the hash is recalculated, the
/// step is looked up in the plan, the verdict must say `accepted` with elevation `1`, and the
/// variant must exist for the platform and declare `requires_elevation`.
pub fn validate(
    request: &ElevatedRequest,
    catalog: &LoadedCatalog,
    allowlist: &Allowlist,
) -> Result<ValidatedStep, ElevatedError> {
    if request.protocol != PROTOCOL {
        return Err(ElevatedError::ProtocolMismatch);
    }
    // FR-006: recalculate the hash of what arrived.
    if sha256_tagged(&request.plan) != request.plan_hash {
        return Err(ElevatedError::PlanMismatch);
    }
    let plan = PlanText::parse(&request.plan)?;
    let entry = plan
        .step(&request.step_id)
        .ok_or(ElevatedError::PlanMismatch)?;
    if entry.capability != request.capability || entry.version != request.capability_version {
        return Err(ElevatedError::PlanMismatch);
    }
    // The parameters in the request are exactly the parameters of that step in the plan.
    if entry.params != request.params {
        return Err(ElevatedError::PlanMismatch);
    }
    let verdict = plan
        .verdict(&request.step_id)
        .ok_or(ElevatedError::PlanMismatch)?;
    if verdict.state != "accepted" {
        return Err(ElevatedError::PlanMismatch);
    }
    // Constitution II: an N3 action needs the approval bound to the exact plan hash, and the
    // catalog is what declares that elevation is needed at all.
    if !verdict.elevation {
        return Err(ElevatedError::PlatformNotSupported);
    }

    let capability = catalog
        .catalog
        .get(&request.capability)
        .ok_or(ElevatedError::NotInAllowlist)?;
    if capability.version != request.capability_version {
        return Err(ElevatedError::NotInAllowlist);
    }
    let platform = platform_from_code(&request.platform)?;
    let variant = capability
        .variants
        .get(&platform)
        .ok_or(ElevatedError::PlatformNotSupported)?;
    if !variant.requires_elevation || variant.risk_level == RiskLevel::N4 {
        return Err(ElevatedError::PlatformNotSupported);
    }
    // FR-007: same parameter rules the Policy Engine applies, repeated on the other side.
    let allowed: BTreeSet<&str> = variant.params.iter().map(String::as_str).collect();
    for (name, value) in &request.params {
        if !allowed.contains(name.as_str()) || value.len() > MAX_PARAM_LEN {
            return Err(ElevatedError::InvalidParams);
        }
    }
    // SEC-021: the compiled allowlist decides, the catalog only says it would need elevation.
    if allowlist
        .get(&request.capability, &request.capability_version)
        .is_none()
    {
        return Err(ElevatedError::NotInAllowlist);
    }
    Ok(ValidatedStep {
        step_id: request.step_id.clone(),
        capability: request.capability.clone(),
        version: request.capability_version.clone(),
        platform,
        params: request.params.clone(),
    })
}

/// The pieces of a canonical plan the helper needs: the step it was asked to run and its verdict.
struct PlanText {
    steps: BTreeMap<String, PlanStep>,
    verdicts: BTreeMap<String, PlanVerdict>,
}

struct PlanStep {
    capability: String,
    version: String,
    params: BTreeMap<String, String>,
}

struct PlanVerdict {
    state: String,
    elevation: bool,
}

impl PlanText {
    fn parse(text: &str) -> Result<PlanText, ElevatedError> {
        let mut lines = text.lines();
        if lines.next() != Some(PLAN_FORMAT) {
            return Err(ElevatedError::PlanMismatch);
        }
        let mut plan = PlanText {
            steps: BTreeMap::new(),
            verdicts: BTreeMap::new(),
        };
        let mut current: Option<String> = None;
        for line in lines {
            let (key, fields) = parse_line(line).map_err(|_| ElevatedError::PlanMismatch)?;
            match (key, fields.as_slice()) {
                ("catalog", [_]) | ("session", [_]) => {}
                ("step", [step_id, capability, version]) => {
                    current = Some(step_id.clone());
                    plan.steps.insert(
                        step_id.clone(),
                        PlanStep {
                            capability: capability.clone(),
                            version: version.clone(),
                            params: BTreeMap::new(),
                        },
                    );
                }
                ("param", [step_id, name, value]) => {
                    let Some(id) = current.as_ref() else {
                        return Err(ElevatedError::PlanMismatch);
                    };
                    if id != step_id {
                        return Err(ElevatedError::PlanMismatch);
                    }
                    plan.steps
                        .get_mut(step_id)
                        .ok_or(ElevatedError::PlanMismatch)?
                        .params
                        .insert(name.clone(), value.clone());
                }
                ("verdict", [step_id, state, rest @ ..]) => {
                    // `accepted|<risk>|<approval>|<elevation 0/1>|<verification>|<rollback>`;
                    // `rest` starts at the risk field, so the elevation flag is its third.
                    let elevation = rest.get(2).is_some_and(|e| e == "1");
                    plan.verdicts.insert(
                        step_id.clone(),
                        PlanVerdict {
                            state: state.clone(),
                            elevation,
                        },
                    );
                }
                _ => return Err(ElevatedError::PlanMismatch),
            }
        }
        if plan.steps.is_empty() {
            return Err(ElevatedError::PlanMismatch);
        }
        Ok(plan)
    }

    fn step(&self, step_id: &str) -> Option<&PlanStep> {
        self.steps.get(step_id)
    }

    fn verdict(&self, step_id: &str) -> Option<&PlanVerdict> {
        self.verdicts.get(step_id)
    }
}

/// FR-003/SEC-022: the request travels as one base64url token in the command line. base64url has no
/// space, quote, backslash or non-ASCII byte, so the Windows command-line parser has nothing to
/// reinterpret and no quoting rule can be got wrong.
pub fn encode_token(request: &ElevatedRequest) -> Result<String, ElevatedError> {
    let json = serde_json::to_string(request).map_err(|_| ElevatedError::BadRequest)?;
    if json.len() > MAX_REQUEST_BYTES {
        return Err(ElevatedError::RequestTooLarge);
    }
    Ok(base64url_encode(json.as_bytes()))
}

pub fn decode_token(token: &str) -> Result<ElevatedRequest, ElevatedError> {
    let json = base64url_decode(token)?;
    if json.len() > MAX_REQUEST_BYTES {
        return Err(ElevatedError::RequestTooLarge);
    }
    serde_json::from_str(&json).map_err(|_| ElevatedError::BadRequest)
}

/// FR-014: the result goes to the pipe and to stdout; both are small and sanitized.
pub fn result_json(result: &ElevatedResult) -> String {
    serde_json::to_string(result).unwrap_or_else(|_| {
        // Serializing a struct of strings and a three-variant enum cannot fail; if it ever did, the
        // fallback is still valid JSON with the stable code.
        format!(
            "{{\"protocol\":\"{PROTOCOL}\",\"outcome\":\"failed\",\"reason_code\":\"RESULT_NOT_SERIALIZABLE\"}}"
        )
    })
}

pub fn base64url_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let bits = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(bits >> 18) as usize & 63] as char);
        out.push(ALPHABET[(bits >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(bits >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[bits as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn base64_value(c: u8) -> Result<u32, ElevatedError> {
    Ok(match c {
        b'A'..=b'Z' => (c - b'A') as u32,
        b'a'..=b'z' => (c - b'a') as u32 + 26,
        b'0'..=b'9' => (c - b'0') as u32 + 52,
        b'-' => 62,
        b'_' => 63,
        _ => return Err(ElevatedError::BadRequest),
    })
}

fn base64url_decode(token: &str) -> Result<String, ElevatedError> {
    let bytes = token.as_bytes();
    if bytes.is_empty() || !bytes.len().is_multiple_of(4) {
        return Err(ElevatedError::BadRequest);
    }
    let chunks = bytes.len() / 4;
    let mut out = Vec::with_capacity(chunks * 3);
    let mut padding = 0usize;
    for (index, chunk) in bytes.chunks(4).enumerate() {
        let last = index + 1 == chunks;
        let mut bits: u32 = 0;
        for (position, &c) in chunk.iter().enumerate() {
            if c == b'=' {
                // Padding only in the last two positions of the last chunk, and nothing after it.
                if !last || position < 2 {
                    return Err(ElevatedError::BadRequest);
                }
                padding += 1;
                // Padding is a full group of zero bits, not the absence of one.
                bits <<= 6;
                continue;
            }
            if padding > 0 {
                return Err(ElevatedError::BadRequest);
            }
            bits = (bits << 6) | base64_value(c)?;
        }
        out.push((bits >> 16) as u8);
        out.push((bits >> 8) as u8);
        out.push(bits as u8);
    }
    out.truncate(out.len() - padding);
    String::from_utf8(out).map_err(|_| ElevatedError::BadRequest)
}

/// SEC-020: the helper refuses to run from anywhere but the per-machine install folder. A helper in
/// a folder the person can write to would be replaceable before the UAC click.
pub fn trusted_path(exe: &std::path::Path, install_dir: &std::path::Path) -> bool {
    match exe.parent() {
        Some(parent) => parent.starts_with(install_dir),
        None => false,
    }
}

/// Everything the helper needs from the outside, so the security decisions can be tested without
/// privileges and without the operating system.
pub struct HelperContext<'a> {
    /// Path of the running executable.
    pub exe: &'a std::path::Path,
    /// Folder the installer registered (per machine).
    pub install_dir: &'a std::path::Path,
    pub catalog: &'a LoadedCatalog,
    pub allowlist: &'a Allowlist,
}

/// The whole helper, as one pure function: token in, exit code and result out.
pub fn run_request(token: &str, context: &HelperContext<'_>) -> (i32, ElevatedResult) {
    let refuse = |error: ElevatedError| (error.exit_code(), error.result());
    if !trusted_path(context.exe, context.install_dir) {
        return refuse(ElevatedError::HelperPathNotTrusted);
    }
    let request = match decode_token(token) {
        Ok(request) => request,
        Err(error) => return refuse(error),
    };
    let step = match validate(&request, context.catalog, context.allowlist) {
        Ok(step) => step,
        Err(error) => return refuse(error),
    };
    let Some(action) = context.allowlist.get(&step.capability, &step.version) else {
        return refuse(ElevatedError::NotInAllowlist);
    };
    let result = action.run(&step);
    let code = match result.outcome {
        Outcome::Succeeded => 0,
        Outcome::Failed => ElevatedError::Failed.exit_code(),
        Outcome::Unknown => ElevatedError::Unknown.exit_code(),
    };
    (
        code,
        ElevatedResult {
            protocol: PROTOCOL.to_string(),
            outcome: result.outcome,
            reason_code: result.reason_code.map(str::to_string),
            detail: None,
        },
    )
}

/// Builds the request of an approved step, from the executor side.
pub fn build_request(
    plan: &EvaluatedPlan<'_>,
    step: &ProposedStep,
    platform: Platform,
    result_pipe: Option<String>,
) -> Result<ElevatedRequest, ExecError> {
    let params = step.params.iter().cloned().collect::<BTreeMap<_, _>>();
    Ok(ElevatedRequest {
        protocol: PROTOCOL.to_string(),
        plan: plan.canonical(),
        plan_hash: plan.hash(),
        step_id: step.step_id.clone(),
        capability: step.capability.clone(),
        capability_version: step.version.clone(),
        platform: platform_code(platform).to_string(),
        params,
        result_pipe,
    })
}

/// Why a launcher could not deliver a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchError {
    /// This build has no elevation path for the platform (spec 020 FR-015: no `pkexec` yet).
    NotAvailable,
    /// The person closed or refused the system dialog.
    Denied,
    /// The hard deadline expired (FR-010).
    Timeout,
    /// The helper answered with a refusal (SEC-025 exit code).
    Refused(&'static str),
    /// The request could not even be built or written.
    Malformed,
}

impl LaunchError {
    pub fn reason_code(&self) -> &'static str {
        match self {
            LaunchError::NotAvailable => "ELEVATION_NOT_AVAILABLE",
            LaunchError::Denied => "ELEVATION_DENIED",
            LaunchError::Timeout => "ELEVATION_TIMEOUT",
            LaunchError::Refused(code) => code,
            LaunchError::Malformed => "ELEVATION_MALFORMED",
        }
    }
}

/// The user-level side of the elevation path: asks the operating system for permission for one
/// action and brings back what the helper reported.
pub trait Launcher: Send + Sync {
    fn available(&self) -> bool;
    fn launch(
        &self,
        token: &str,
        timeout: std::time::Duration,
    ) -> Result<ElevatedResult, LaunchError>;
    /// The request goes through a launcher, so the catalog variant for the platform is what decides.
    fn platform(&self) -> Platform;
}

/// A launcher that does nothing: used by tests and as the default of this build until S-02.
pub struct FakeLauncher {
    pub platform: Platform,
    pub available: bool,
    /// Runs instead of the operating system. `None` means "the dialog was refused".
    pub answer: Option<ElevatedResult>,
    /// What the answer should take; `None` means timeout.
    pub elapsed: Option<std::time::Duration>,
}

impl FakeLauncher {
    pub fn refusing() -> FakeLauncher {
        FakeLauncher {
            platform: Platform::Windows,
            available: true,
            answer: None,
            elapsed: None,
        }
    }
}

impl Launcher for FakeLauncher {
    fn available(&self) -> bool {
        self.available
    }
    fn platform(&self) -> Platform {
        self.platform
    }
    fn launch(
        &self,
        _token: &str,
        _timeout: std::time::Duration,
    ) -> Result<ElevatedResult, LaunchError> {
        match self.elapsed {
            None => Err(LaunchError::Timeout),
            Some(_) => self.answer.clone().ok_or(LaunchError::Denied),
        }
    }
}

/// Adapts a [`Launcher`] to the executor: the executor never knows about tokens, pipes or dialogs.
pub struct ElevatedFix {
    launcher: Box<dyn Launcher>,
    platform: Platform,
}

impl ElevatedFix {
    pub fn new(launcher: Box<dyn Launcher>) -> ElevatedFix {
        let platform = launcher.platform();
        ElevatedFix { launcher, platform }
    }

    fn step_of<'a>(step: &ProposedStep, plan: &'a EvaluatedPlan<'_>) -> Option<&'a ProposedStep> {
        plan.proposal
            .steps
            .iter()
            .find(|s| s.step_id == step.step_id)
    }
}

impl Elevation for ElevatedFix {
    fn available(&self, _step: &ProposedStep, platform: Platform) -> bool {
        self.launcher.platform() == platform && self.launcher.available()
    }

    fn run(
        &self,
        step: &ProposedStep,
        plan: &EvaluatedPlan<'_>,
        _platform: Platform,
    ) -> Result<FixOutcome, ExecError> {
        let Some(step) = Self::step_of(step, plan) else {
            return Err(ExecError::StepNotApproved);
        };
        let token = encode_token(&build_request(plan, step, self.platform, None)?)
            .map_err(|_| ExecError::ElevationNotAvailable)?;
        let result = self
            .launcher
            .launch(&token, TIMEOUT)
            .map_err(map_launch_error)?;
        Ok(match result.outcome {
            Outcome::Succeeded => FixOutcome::Succeeded,
            Outcome::Failed => FixOutcome::Failed(
                result
                    .reason_code
                    .as_deref()
                    .and_then(static_reason)
                    .unwrap_or("FIX_FAILED"),
            ),
            Outcome::Unknown => FixOutcome::Unknown,
        })
    }
}

/// The reason codes the journal is allowed to keep, as `&'static str` (spec 008 stable codes).
fn static_reason(code: &str) -> Option<&'static str> {
    Some(match code {
        "FIX_FAILED" => "FIX_FAILED",
        "FIX_UNKNOWN" => "FIX_UNKNOWN",
        "INVALID_PARAMS" => "INVALID_PARAMS",
        "NOT_IN_ALLOWLIST" => "NOT_IN_ALLOWLIST",
        "PLATFORM_NOT_SUPPORTED" => "PLATFORM_NOT_SUPPORTED",
        "PLAN_HASH_MISMATCH" => "PLAN_HASH_MISMATCH",
        "BAD_REQUEST" => "BAD_REQUEST",
        "PROTOCOL_MISMATCH" => "PROTOCOL_MISMATCH",
        "REQUEST_TOO_LARGE" => "REQUEST_TOO_LARGE",
        "HELPER_PATH_NOT_TRUSTED" => "HELPER_PATH_NOT_TRUSTED",
        _ => return None,
    })
}

fn map_launch_error(error: LaunchError) -> ExecError {
    match error {
        LaunchError::Denied => ExecError::ElevationDenied,
        LaunchError::Timeout => ExecError::ElevationTimeout,
        LaunchError::NotAvailable => ExecError::ElevationNotAvailable,
        LaunchError::Refused(_) | LaunchError::Malformed => ExecError::ElevationNotAvailable,
    }
}

/// The launcher of this build.
///
/// There is none yet, on purpose: the UAC launcher is Windows-only FFI and the spike S-02 has to
/// measure on real machines which fixes really need privileges before anything runs elevated
/// (spec 020 FR-005, FR-015). Until then every elevated step is refused with
/// `ELEVATION_NOT_AVAILABLE`, which is the safe answer, not a missing feature.
pub fn production_launcher() -> Box<dyn Launcher> {
    Box::new(UnavailableLauncher {
        platform: host_platform(),
    })
}

/// The registry of the real app: no system-changing fix (spike S-02) and the elevation path of this
/// platform.
pub fn production_registry() -> Registry {
    Registry::production().with_elevation(Box::new(ElevatedFix::new(production_launcher())))
}

fn host_platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else {
        Platform::Linux
    }
}

/// Refuses every elevation: the default off Windows.
pub struct UnavailableLauncher {
    pub platform: Platform,
}

impl Launcher for UnavailableLauncher {
    fn available(&self) -> bool {
        false
    }
    fn platform(&self) -> Platform {
        self.platform
    }
    fn launch(
        &self,
        _token: &str,
        _timeout: std::time::Duration,
    ) -> Result<ElevatedResult, LaunchError> {
        Err(LaunchError::NotAvailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_round_trip_for_every_length() {
        // The token always carries UTF-8 text (the JSON request), so the round trip is checked with
        // text; the alphabet is checked below with the vectors of the standard.
        let text: String = (0..200u32)
            .filter_map(char::from_u32)
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        for len in 1..text.len().min(64) {
            let bytes = &text.as_bytes()[..len];
            let encoded = base64url_encode(bytes);
            assert!(
                encoded
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'=')
            );
            assert_eq!(base64url_decode(&encoded).unwrap().as_bytes(), bytes);
        }
    }

    #[test]
    fn base64url_matches_the_standard_alphabet() {
        assert_eq!(base64url_encode(b"abc"), "YWJj");
        assert_eq!(base64url_encode(b"ab"), "YWI=");
        assert_eq!(base64url_encode(b"a"), "YQ==");
        // `-` and `_` instead of `+` and `/`.
        assert_eq!(base64url_encode(&[0xfb, 0xff]), "-_8=");
    }

    #[test]
    fn base64url_rejects_a_token_it_did_not_write() {
        assert!(
            base64url_decode("YWJ").is_err(),
            "length not a multiple of four"
        );
        assert!(base64url_decode("YW+j").is_err(), "standard alphabet");
        assert!(base64url_decode("YW=j").is_err(), "padding in the middle");
        assert!(
            base64url_decode("YWJ*").is_err(),
            "character outside the alphabet"
        );
    }
}
