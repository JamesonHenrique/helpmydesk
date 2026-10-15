# Feature Specification: Executor, plan-hash approval and execution journal

**Feature Branch**: `008-executor-approval-journal`
**Created**: 2026-10-15
**Status**: Approved (owner approval, 2026-10-14)
**Input**: Plan v1 V.6 (plan validation), V.7 (approval), V.10 (interruptions), VII.3 (`ActionApproval`), VII.4 (`ActionPlan`, `ActionApproval`, `ActionExecution`)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Nothing runs without an approval of exactly this plan (Priority: P1)

**Acceptance Scenarios**:

1. **Given** a plan accepted by the Policy Engine, **Then** its `plan_hash` is SHA-256 of a
   canonical encoding of the proposal, the decision and the catalog version; reordering steps or
   changing any field changes the hash.
2. **Given** an approval for hash H, **When** the plan to execute has another hash, **Then**
   execution is refused (`PLAN_HASH_MISMATCH`) and nothing runs.
3. **Given** an approval older than 10 minutes, **Then** `APPROVAL_EXPIRED`.
4. **Given** a step not in `approved_step_ids`, or a step the Policy Engine did not accept,
   **Then** `STEP_NOT_APPROVED`.
5. **Given** a step already executed with the same approval, **Then** `DUPLICATE_EXECUTION`
   (`idempotency_key = sha256(session_id, step_id, nonce)` is unique).

### User Story 2 - The journal survives crashes and never guesses (Priority: P1)

1. **Given** a step is about to run, **Then** `started` is written before the implementation is
   called (write-ahead).
2. **Given** the app restarts with an execution in `started`, **Then** it becomes `unknown`.
3. **Given** an execution in `unknown`, **Then** it is never re-run automatically; the Policy
   Engine blocks the same capability (`UNKNOWN_PENDING`) and the ticket can go to REP-04.
4. Journal transitions are a pure function: `planned → approved → started →
   succeeded | failed | unknown`, then `succeeded → verified_ok | verified_failed`,
   `verified_failed → rolled_back` (if rollback is implemented); anything else is refused.

### User Story 3 - "Funcionou" means a probe proved it (Priority: P1)

1. **Given** a step succeeded, **Then** its catalog verification probe runs again (policy-authorized).
2. **Given** none of the step's precondition findings (`all_of`) appear in the verification
   output, **Then** `verified_ok` and the ticket may go to `resolved_pending_confirmation`.
3. **Given** a precondition finding is still there, or the verification probe failed, **Then**
   `verified_failed`, rollback when implemented, and report trigger REP-03.

### Edge Cases

- A capability without an implementation in this build returns `NOT_IMPLEMENTED` before
  `started` is written; the person gets the catalog guidance instead.
- One action per call; the executor never takes commands, paths or shell text from the plan.
- Fix implementations that change the system are **not** part of this spec: they depend on
  spike S-02 (privilege matrix). Tests use fake implementations.

## Requirements *(mandatory)*

- **FR-001**: Canonical encoding and hashing MUST be pure and documented (field order, escaping).
- **FR-002**: `ActionApproval { plan_hash, approved_step_ids, mode, decided_at, expires_at, nonce }`;
  the nonce is unique per approval (store sequence; uniqueness, not secrecy, is what matters
  locally).
- **FR-003**: The executor MUST check, in order: hash, expiry, step approved and accepted,
  idempotency key unused, implementation exists; only then write `started` and run.
- **FR-004**: Execution journal transitions MUST be a pure function in `hmd-domain`.
- **FR-005**: Store schema v2 adds `action_plan`, `action_approval` (`nonce` unique) and
  `action_execution` (`idempotency_key` unique); the v1 → v2 migration makes the backup copy first.
- **FR-006**: Verification MUST reuse the policy-authorized probe runner of spec 006.
- **NFR-001**: New dependency `sha2` (RustCrypto, MIT OR Apache-2.0), ADR-024.

## Success Criteria *(mandatory)*

- **SC-001**: Each acceptance scenario has a test (fake implementations and probes).
- **SC-002**: A v1 database created by spec 007 code migrates to v2 with its backup file (closes
  the untested migration path).

## Assumptions

- The approval window (isolated Tauri window, `approve_plan` IPC) is the desktop app's job; this
  spec provides the data and the checks it will call.
- `AuditEvent` with hash chain comes with the privacy/consent spec (SPEC-011).
