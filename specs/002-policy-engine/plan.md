# Implementation Plan: Policy Engine

**Branch**: `002-policy-engine` | **Date**: 2026-10-14 | **Spec**: [spec.md](spec.md)

## Summary

Pure crate `hmd-policy` with typed catalog entries, `min_level` and catalog validation, and
`evaluate(proposal, catalog, context) -> PolicyDecision` applying the checks of Plan v1 V.6 in a
fixed order.

## Technical Context

**Language**: Rust 1.99, edition 2024 · **Dependencies**: `hmd-domain` (path) only ·
**Testing**: `cargo test`, exhaustive combination tests · **Platforms**: Windows, Linux (CI)

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I | Proposal type carries only IDs, versions and params; risk/approval/verification come from the catalog | PASS |
| II | N3 → per-action approval; N4 never accepted | PASS |
| III | Fix variants without verification rejected at catalog validation | PASS |
| IV | N1-S requires consent | PASS |
| VI | Every reason code tested (SC-001) | PASS |
| VII | No new dependency | PASS |
| Protected path | `crates/hmd-policy` is CODEOWNER-protected | Noted |

## Check order per step (first failure wins)

1. duplicate `step_id` → `DUPLICATE_STEP`
2. capability exists → `UNKNOWN_CAPABILITY`; version → `VERSION_MISMATCH`
3. platform variant → `PLATFORM_UNSUPPORTED`
4. kill switch → `CAPABILITY_DISABLED`
5. N4 → `BLOCKED_CAPABILITY`
6. params → `INVALID_PARAMS`
7. preconditions → `PRECONDITION_UNMET`
8. N1-S consent → `CONSENT_REQUIRED`
9. pending unknown → `PENDING_UNKNOWN_EXECUTION`; per-capability and total limits → `RATE_LIMIT`
10. exclusivity with an already accepted step → `EXCLUSIVE_CONFLICT`
11. N3 confidence → `LOW_CONFIDENCE`
12. autonomy mode → accept with approval requirement, or downgrade (`LEVEL_EXCEEDS_MODE`)

Plan-level checks before steps: `EMPTY_PLAN`, `TOO_MANY_STEPS` (max 10).

## Minimum level (FR-011)

| Effect | Implied minimum |
|---|---|
| kind = probe, no effects | N1-A |
| requires_elevation | N3 |
| data_loss = some | N3 |
| disruption = noticeable | N3; brief → N2 |
| persistent_change on system scope | N3; user scope → N2 |
| kind = fix (any) | N2 |
| kind = guide | N0 |

## Approval mapping (FR-006)

| Mode \ Level | N0 / N1-A / N1-B | N1-S | N2 fix | N3 fix |
|---|---|---|---|---|
| ObserveOnly | none | consent | downgrade | downgrade |
| AskAlways | none | consent | plan approval | per-action |
| AutoSimple | none | consent | none if `auto_eligible`, else plan | per-action |

## Structure

```text
crates/hmd-policy/src/
├── lib.rs        # re-exports
├── catalog.rs    # RiskLevel, Capability, PlatformVariant, effects, min_level, validate_catalog
├── proposal.rs   # ActionPlanProposal, ProposedStep
├── context.rs    # PolicyContext, AutonomyMode, Confidence, SessionCounters
└── engine.rs     # evaluate, PolicyDecision, StepVerdict, RejectReason
```
