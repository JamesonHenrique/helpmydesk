# Feature Specification: Playbooks and basic mode

**Feature Branch**: `006-playbooks-basic-mode`
**Created**: 2026-10-14
**Status**: Approved (owner approval, 2026-10-14)
**Input**: Plan v1 III.6 (journeys), IV.5 diagram 3, VI.3 (AI is optional), VII.2 (REP triggers)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A person picks a problem from a menu and gets an honest diagnosis (Priority: P1)

Without AI and without internet, the person chooses one of the 5 families ("Internet",
"Lento", "Sem espaço", "Som/Microfone", "Erro/Programa"). The app runs that family's playbook:
the listed probes, then the first diagnosis whose conditions match the findings, explained with
human-written pt-BR text, possibly proposing catalog fixes that go through the Policy Engine.

**Independent Test**: playbook evaluation is a pure function of findings; the end-to-end
`diagnose` runs real probes in CI.

**Acceptance Scenarios**:

1. **Given** findings `{net.dns.resolution_failing}` with the gateway answering, **When** the
   connectivity playbook is evaluated, **Then** the diagnosis is the DNS one and it proposes
   `net.dns.flush_cache`, which the Policy Engine accepts with per-action approval (N3).
2. **Given** `{net.gateway.unreachable}`, **Then** the diagnosis tells the person to restart the
   router (guidance steps) and proposes no fix.
3. **Given** no matching condition, **Then** the playbook's fallback diagnosis is returned
   ("não encontrei problema nas verificações que faço") with report trigger REP-01.
4. **Given** a family without probes yet (audio, app error), **Then** the diagnosis is guidance
   plus report (REP-01), never a fake check.

### User Story 2 - Uncertainty lowers confidence instead of inventing (Priority: P1)

**Acceptance Scenarios**:

1. **Given** one of the playbook's probes failed (`NO_DATA`/`TIMEOUT`), **Then** the selected
   diagnosis confidence drops one level (almost certain → probable → unsure).
2. **Given** confidence "unsure", **Then** N3 fixes are rejected by the Policy Engine
   (`LOW_CONFIDENCE`) and the person gets guidance and the report option.

### User Story 3 - Sensitive probes need consent (Priority: P2)

**Acceptance Scenarios**:

1. **Given** the performance playbook lists `startup.apps.list` (N1-S) and the person has not
   consented to `startup_apps`, **Then** the probe is skipped (policy `CONSENT_REQUIRED`) and the
   rest of the playbook still runs.

### Edge Cases

- Conditions referencing findings no listed probe produces are rejected when the catalog loads.
- A diagnosis may list fixes; their own preconditions are still checked by the Policy Engine.
- Every family must have exactly one playbook (menu completeness).

## Requirements *(mandatory)*

- **FR-001**: Playbooks MUST be TOML files in `catalog/playbooks/`, embedded and validated with
  the catalog: probes exist and are probes; diagnosis conditions (`when_all`, `when_any`,
  `when_none`) only use findings produced by the playbook's probes; fixes are catalog fixes;
  report codes are REP-01…REP-08; pt-BR copy (`title`, `explanation`) present; one playbook per
  family and all 5 families covered.
- **FR-002**: Evaluation MUST be pure and ordered: first diagnosis whose conditions hold wins;
  otherwise the mandatory `fallback`.
- **FR-003**: Probes MUST be authorized through the Policy Engine like any capability (consent,
  kill switch, platform), and run only if accepted.
- **FR-004**: Confidence MUST drop one level when any playbook probe failed.
- **FR-005**: Proposed fixes MUST be evaluated by the Policy Engine with the diagnosis
  confidence; the result is returned, nothing is executed (execution is a later spec).
- **FR-006**: A text runner (`basic_mode` example) MUST show the menu and a diagnosis in pt-BR,
  as the first usable "modo básico" before the desktop UI.

## Success Criteria *(mandatory)*

- **SC-001**: Each acceptance scenario has a test (pure evaluation with fixed findings).
- **SC-002**: `diagnose` runs for every family in CI on Linux and Windows without elevation.
- **SC-003**: Every new catalog validation code has a failing-fixture test.

## Assumptions

- Questions to the person (clarifying dialog) come with the AI/dialog spec; the MVP basic mode
  uses the menu only.
- Guidance steps are human-written text in the catalog (N0); `settings.open` guides come later.
