# Tasks: Executor, plan-hash approval and execution journal

- [x] T001 `hmd-domain::execution`: states and pure transitions with full table test (FR-004)
- [x] T002 ADR-024 for `sha2`
- [x] T003 `hmd-executor`: canonical encoding + `plan_hash` + `idempotency_key` (FR-001, US1.1)
- [x] T004 Approval type and checks in order (FR-002, FR-003, US1.2–1.5)
- [x] T005 `FixImpl` registry; `NOT_IMPLEMENTED` before `started` (edge cases)
- [x] T006 `execute_step` with write-ahead journal through a `Journal` trait (US2.1)
- [x] T007 `verify_step`: policy-authorized probe, precondition findings, rollback, REP-03 (US3)
- [x] T008 `hmd-store` schema v2 + journal API + `recover_interrupted` (FR-005, US2.2)
- [x] T009 v1 → v2 migration test with backup file (SC-002)
- [x] T010 `unknown` blocks the capability in the Policy Engine counters (US2.3)
- [x] T011 End-to-end test with fake fix and fake probes: approve → run → verify → ticket state
- [x] T012 Update indexes, docs and the progress roadmap
