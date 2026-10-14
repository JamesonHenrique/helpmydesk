# Feature Specification: Policy Engine

**Feature Branch**: `002-policy-engine`
**Created**: 2026-10-14
**Status**: Approved (owner approval, 2026-10-14)
**Input**: Plan v1, Parts V.2, V.3, V.6 and VII.3 — "Deterministic Policy Engine validating
action plan proposals against the signed catalog"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Nothing outside the catalog ever executes (Priority: P1)

Whatever the AI (or a playbook) proposes, a step is accepted only if it refers to a capability
that exists in the loaded catalog, for this platform, in the expected version, is not disabled by
the kill switch and is not blocked (N4).

**Why this priority**: this is constitution Principle I; without it no other safety holds.

**Independent Test**: feed proposals with unknown IDs, wrong versions, wrong platform, disabled
and blocked capabilities; every step must be rejected with the specific reason code.

**Acceptance Scenarios**:

1. **Given** a proposal with step `shell.exec`, **When** validated, **Then** the step is rejected
   with `UNKNOWN_CAPABILITY`.
2. **Given** a capability present only for Windows, **When** validated on Linux, **Then**
   `PLATFORM_UNSUPPORTED`.
3. **Given** a capability listed in the kill switch, **When** validated, **Then**
   `CAPABILITY_DISABLED`.
4. **Given** a capability whose variant is N4, **When** validated, **Then** `BLOCKED_CAPABILITY`.

---

### User Story 2 - Preconditions come from evidence, not claims (Priority: P1)

A fix is accepted only when the findings produced by probes in this session satisfy its
preconditions; the proposal cannot assert findings.

**Independent Test**: the same proposal is accepted or rejected depending only on the set of
findings supplied by the caller.

**Acceptance Scenarios**:

1. **Given** `net.dns.flush_cache` requires finding `net.dns.resolution_failing` and forbids
   `net.gateway.unreachable`, **When** the session has only the first, **Then** accepted;
   **When** it has both, **Then** `PRECONDITION_UNMET`.

---

### User Story 3 - The person's autonomy mode is a ceiling (Priority: P1)

The mode chosen by the person decides whether a step runs, needs approval, or becomes guidance.

**Acceptance Scenarios**:

1. **Given** mode "observe only", **When** an N2 or N3 fix is proposed, **Then** it is downgraded
   to guidance with `LEVEL_EXCEEDS_MODE` (never executed).
2. **Given** mode "ask always" (default), **Then** N2 requires plan approval and N3 requires
   per-action approval.
3. **Given** mode "auto simple", **Then** an N2 fix runs without approval only if the capability is
   marked auto-eligible (gate G-AUTO); otherwise plan approval. N3 always needs per-action approval.
4. **Given** an N1-S probe whose data category was not consented, **Then** `CONSENT_REQUIRED`.

---

### User Story 4 - Limits, conflicts and uncertainty stop repetition (Priority: P2)

**Acceptance Scenarios**:

1. **Given** a capability already executed `max_per_session` times, **Then** `RATE_LIMIT`.
2. **Given** an execution of the same capability in state `unknown`, **Then**
   `PENDING_UNKNOWN_EXECUTION`.
3. **Given** two steps of capabilities declared mutually exclusive, **Then** the second is rejected
   with `EXCLUSIVE_CONFLICT`.
4. **Given** an N3 step while the diagnosis confidence is below "probable", **Then**
   `LOW_CONFIDENCE`.
5. **Given** a proposal with no steps or more than the maximum, **Then** the whole plan is rejected
   (`EMPTY_PLAN` / `TOO_MANY_STEPS`); a repeated `step_id` yields `DUPLICATE_STEP`.

---

### User Story 5 - The catalog itself is checked (Priority: P1)

A catalog entry cannot declare a risk level lower than the minimum computed from its effects, and
a fix without a verification probe cannot be N2 or N3.

**Acceptance Scenarios**:

1. **Given** a variant that requires elevation declared N2, **When** the catalog is validated,
   **Then** error `LEVEL_BELOW_MINIMUM`.
2. **Given** a fix variant without verification declared N2, **Then** error
   `FIX_WITHOUT_VERIFICATION`.

### Edge Cases

- Parameters not declared by the capability, or longer than the limit → `INVALID_PARAMS`.
- Probes and guides need no approval (except N1-S consent).
- A plan where every step is rejected yields a decision with zero accepted steps (the
  orchestrator then goes to guidance or report; never "tries something else").

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The engine MUST reject any step whose capability ID is not in the catalog
  (`UNKNOWN_CAPABILITY`) or whose version differs (`VERSION_MISMATCH`).
- **FR-002**: The engine MUST reject steps without a variant for the current platform
  (`PLATFORM_UNSUPPORTED`), disabled by the kill switch (`CAPABILITY_DISABLED`) or at N4
  (`BLOCKED_CAPABILITY`).
- **FR-003**: Risk level, elevation, rollback and verification MUST be taken from the catalog
  variant, never from the proposal (the proposal type has no such fields).
- **FR-004**: Preconditions (`all_of` required findings, `none_of` forbidden findings) MUST be
  evaluated only against the findings supplied by the caller (`PRECONDITION_UNMET`).
- **FR-005**: Parameters MUST match the names declared by the variant and a maximum value length
  (`INVALID_PARAMS`).
- **FR-006**: The autonomy mode MUST be applied as a ceiling with the mapping of User Story 3.
- **FR-007**: N1-S probes MUST require the consent of their data category (`CONSENT_REQUIRED`).
- **FR-008**: The engine MUST enforce `max_per_session`, a per-session total of fixes, pending
  `unknown` executions and mutual exclusion.
- **FR-009**: N3 steps MUST require diagnosis confidence ≥ probable (`LOW_CONFIDENCE`).
- **FR-010**: Plans MUST have 1..=10 steps with unique `step_id` values.
- **FR-011**: The engine MUST expose `min_level(variant)` = max of the levels implied by
  elevation, data loss, disruption and persistent change, and a catalog validator reporting
  `LEVEL_BELOW_MINIMUM` and `FIX_WITHOUT_VERIFICATION`.
- **FR-012**: The decision MUST list, per step, either accepted (with risk level, approval
  requirement, elevation flag, verification probe) or rejected/downgraded with a stable reason code.

### Non-Functional Requirements

- **NFR-001**: Pure crate: no I/O, clock, randomness; deterministic output for equal inputs.
- **NFR-002**: No runtime dependencies besides `hmd-domain`.
- **NFR-003**: No `unwrap`/`expect`/`panic!` in production code.

### Key Entities

- **Capability / PlatformVariant**: catalog entry and its per-platform data.
- **ActionPlanProposal / ProposedStep**: what a playbook or the AI proposes (IDs and params only).
- **PolicyContext**: platform, autonomy mode, findings, consents, confidence, kill switch,
  session counters.
- **PolicyDecision / StepVerdict**: the result.

## Success Criteria *(mandatory)*

- **SC-001**: Every reason code in FR-001..FR-011 has at least one test that triggers it.
- **SC-002**: A property-style test over all (mode × level × kind) combinations shows that no
  N2/N3 fix is ever accepted without approval except auto-eligible N2 in "auto simple", and that
  nothing is accepted in "observe only" above N1.
- **SC-003**: CI green on Linux and Windows.

## Assumptions

- JSON parsing and schema validation of proposals and catalog files are a later feature
  (catalog crate); this crate works on typed values.
- Plan hashing and approval binding belong to spec 005 (approval flow).
