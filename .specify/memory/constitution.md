# HelpMyDesk Constitution

## Core Principles

### I. The AI Proposes, Code Decides (NON-NEGOTIABLE)

- No code path may turn model output into a system command, file path, registry key,
  D-Bus call target, URL to fetch, or policy rule.
- The model MUST NOT be given executable tools. Its only "tool" is answering in a closed
  JSON Schema whose enums are generated from the signed catalog.
- Autonomy level, risk, preconditions, verification and rollback of a capability come from
  the signed catalog, never from a plan or a model.
- All plans are validated by the deterministic Policy Engine before any execution.

Rationale: a free or weak model must never be able to cause harm on a non-technical
person's computer; prompt injection must have no consequence beyond a rejected proposal.

### II. Least Privilege and Human Control

- The application runs as the logged-in user. Privileged work happens only through the
  per-platform elevation path: an ephemeral, signed Elevated Helper under UAC (Windows) or
  system D-Bus APIs guarded by polkit, with a `pkexec` helper only when no API exists
  (Linux). No resident privileged service or daemon.
- The elevated helper contains its own compiled allowlist of actions and re-validates every
  request; it never runs a shell or a string received from outside.
- Level N3 actions require an approval bound to the exact plan hash, collected in the
  dedicated approval window. N4 actions never execute. Refusing is always possible.

Rationale: the agent acts on other people's machines; every elevation must be explicit,
minimal and auditable.

### III. Proof of Resolution

- Every `fix` capability MUST declare a verification probe per platform; a capability
  without verification cannot be N2 or N3.
- "Resolved" requires a passing verification AND the person's confirmation. A command
  exiting with code 0 is never proof.
- Interrupted executions become `unknown` and are never re-executed automatically.

Rationale: honesty about outcomes is the product.

### IV. Privacy by Default

- Every collected datum declares category, purpose, retention and whether it may leave
  the device. Probes collect the minimum; file contents, passwords, cookies, tokens,
  keystrokes and screen captures are never collected.
- Everything sent to an AI provider passes the Redactor first and is previewable by the
  person ("what goes to the AI").
- The project operates no server and has no telemetry. Data leaves the device only (a) to
  the AI provider the person connected, inside a session the person started, or (b) as a
  report the person exports by their own action.

Rationale: free software for non-technical people must not trade their privacy.

### V. No External AI Outside a Person's Session; No Project Keys

- No call to an external AI service may happen outside a session started by the person
  (no background, scheduled or automatic calls).
- No API key or credential owned by the project may exist in the code, the repository,
  or the releases. The person's own key lives in the operating system's credential vault.
- Only models on the signed allowlist (provider declares zero retention and no training)
  may be called.

Rationale: OpenCode's terms ("own internal use", no processes while not logged in) and
the people's privacy.

### VI. Tests and Traceability

- Every MUST requirement in a spec has at least one test that cites its ID
  (FR-/NFR-/SEC-). Bug fixes come with a test that reproduces the bug.
- Pure domain and policy crates have no I/O and are exhaustively unit/property tested.
- Done means the acceptance criteria were demonstrated by a test or a recorded run.

Rationale: a tool that changes computers needs evidence, not plausibility.

### VII. Simplicity

- A new language, service, database, queue or network dependency requires an ADR.
- Prefer the solution with fewer moving parts; Rust (core) and TypeScript (UI) only.

Rationale: a small team must be able to maintain and audit the whole system.

## Security and Safety Constraints

- Explicit errors: no `unwrap()`/`expect()`/`panic!` on production paths in Rust (enforced
  by lint); errors carry stable `reason_code` values; uncertain state degrades to read-only.
- No secrets in the repository: secret scanning runs in pre-commit and CI.
- Dependencies are justified in the PR (license, maintenance, alternative); high-severity
  advisories block merge.
- Process spawning is allowed only inside `hmd-executor` and `hmd-elevated` (enforced by
  static analysis).
- Logs are structured and redacted; no personal content.
- Contracts (catalog, plan, approval, kill switch, AI task schemas) are versioned; unknown
  protocol versions are rejected.
- Forbidden capabilities regardless of request: disabling antivirus, firewall, UAC,
  SELinux or AppArmor; deleting personal files; installing/removing drivers or packages;
  running arbitrary commands; remote control.

## Development Workflow and Contribution

- Spec-driven: `specify → clarify → plan → checklist → tasks → analyze → implement →
  converge`. Living Spec model: when implementation shows a spec is wrong, fix `spec.md`
  first (PR labeled `spec-change`), then re-run analyze and converge.
- A PR that changes behavior updates the affected spec, ADR and docs in the same PR.
- Protected paths (`catalog/`, `crates/hmd-policy`, `crates/hmd-executor`,
  `crates/hmd-elevated`, Tauri capabilities, `.github/workflows/`) require CODEOWNER review;
  a new capability requires a complete capability card and a lab scenario that breaks,
  fixes and verifies.
- While there is a single maintainer, changes to protected paths stay open at least 48 hours
  before merge.
- Commits follow Conventional Commits and carry a DCO `Signed-off-by`.
- Releases are built only by CI in a protected environment, signed, and attested.

## Governance

- This constitution supersedes other practices. In conflict, the order is:
  constitution > accepted ADR > approved spec > plan > code.
- Amendments are made by PR that updates this file, states the semantic version bump and
  lists affected specs and templates. MAJOR: principle removed or redefined; MINOR:
  principle or section added; PATCH: clarification.
- `/speckit-plan` Constitution Check, `/speckit-analyze` and `/speckit-converge` enforce
  these principles; a MUST violation is CRITICAL and blocks implementation.
- Runtime guidance for contributors lives in `CONTRIBUTING.md` and `docs/development.md`.

**Version**: 1.0.0 | **Ratified**: 2026-10-14 | **Last Amended**: 2026-10-14
