# Implementation Plan: Read-only probes

**Branch**: `004-read-only-probes` | **Date**: 2026-10-14 | **Spec**: [spec.md](spec.md)

## Technical Context

Rust 1.99 · crate `crates/hmd-probes` · dep: `sysinfo` 0.39 with features `system` and `disk`
only (ADR-021) · DNS via `std::net::ToSocketAddrs` on worker threads with a deadline.

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| II | No elevation; tests run as normal user in CI | PASS |
| IV | No host/user names, serials, MACs, SSIDs, labels; deny-list test | PASS |
| Security | No process spawning (guard script in CI); fixed DNS names | PASS |

## Design

- `run(id)` dispatches by catalog ID; unknown IDs → `UNKNOWN_PROBE`.
- `rules.rs`: memory available < 10% → `perf.memory.high`; system volume free < 10% or < 5 GiB →
  `storage.low_space`; no DNS name resolved → `net.dns.resolution_failing`.
- System volume: `/` on Linux, `%SystemDrive%\` on Windows.
- Failures: `NO_DATA`, `TIMEOUT` (never an invented result).
