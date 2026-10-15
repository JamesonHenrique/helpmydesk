# Tasks: Read-only probes — batch 2

- [x] T001 ADR-022 and dependencies (`netdev`, `winreg`)
- [x] T002 Pure rules + tests: adapters, gateway, HTTP classification, CPU, startup (SC-001)
- [x] T003 `net.adapters.status`, `net.gateway.reach` (TCP 53/80/443), `net.http.check` (US1)
- [x] T004 `perf.cpu.snapshot`, `perf.top_processes` without threads (US2, FR-005)
- [x] T005 `startup.apps.list` Linux (XDG, OnlyShowIn/NotShowIn/TryExec) and Windows (Run + StartupApproved) (FR-004)
- [x] T006 Catalog entries; `net.dns.flush_cache` now requires a reachable gateway (`none_of`)
- [x] T007 Integration tests on Linux/Windows; privacy test covers new probes (SC-002)
