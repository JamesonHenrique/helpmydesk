# Feature Specification: Application service (use cases for the UI)

**Feature Branch**: `012-app-service`
**Created**: 2026-10-15
**Status**: Approved (owner approval, 2026-10-14: "aprovo tudo")
**Input**: Plan v1 VII.2 (lifecycle, deduplication, REP), V.7 (approval), V.10 (interruptions), III.13 (screen map)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - One problem, one ticket (Priority: P1)

1. **Given** an open ticket of a family, **When** the person picks the same family, **Then** it is
   continued, not duplicated.
2. **Given** an unresolved or resolved ticket of the family, **Then** it is reopened (counts for
   REP-08).

### User Story 2 - Diagnosis prepares a plan only when policy allows (Priority: P1)

1. **Given** a diagnosis whose fixes the Policy Engine accepts, **Then** the plan (hash and steps
   with catalog title, risk, undo, elevation, per-action approval) is stored and the ticket waits
   for the person (`awaiting_user`).
2. **Given** "Só olhar" (observe only), **Then** no plan is offered.

### User Story 3 - Approval, execution, proof, confirmation (Priority: P1)

1. **Given** a plan not yet approved, **Then** execution is refused.
2. **Given** an approved step with no implementation in this build, **Then** `NotAvailable` and
   nothing changes (guidance and report remain).
3. **Given** a verified fix, **Then** the ticket waits for the person's confirmation; only "Sim,
   voltou" makes it `resolved`.
4. **Given** a fix not proven by its probe, **Then** REP-03; an `unknown` outcome, **Then** REP-04
   and the same fix is not offered again while it stays unknown.
5. **Given** "Não, obrigado", **Then** the plan is dropped and the ticket stays in progress.

### User Story 4 - Repeated problems and time rules (Priority: P2)

1. **Given** the same family reopened twice in 7 days, **Then** the diagnosis offers the report
   with REP-08.
2. **Given** startup, **Then** interrupted executions become `unknown`, time rules close or
   abandon tickets, and retention runs.

## Requirements *(mandatory)*

- **FR-001**: New crate `hmd-app`; the UI calls only it. Probe runners are injected.
- **FR-002**: Plans and approvals live in memory for the run of the app (re-proposed after a
  restart; approvals expire in 10 minutes anyway); everything else is in the store.
- **FR-003**: The autonomy mode is a stored setting (schema v4 `setting`, audited).
- **FR-004**: `hmd-core` exposes the evaluated fix proposal and honours pending `unknown`
  executions when evaluating fixes.

## Success Criteria *(mandatory)*

- **SC-001**: Each scenario has a test with fake probes and fake fixes.

## Assumptions

- "Não sei explicar" without a family is handled by the UI as the report of the last diagnosis;
  a family-less ticket is not in the domain yet.
