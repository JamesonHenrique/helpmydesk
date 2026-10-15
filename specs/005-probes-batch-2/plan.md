# Implementation Plan: Read-only probes — batch 2

**Branch**: `005-probes-batch-2` | **Date**: 2026-10-14 | **Spec**: [spec.md](spec.md)

## Technical Context

Crate `crates/hmd-probes` · new deps `netdev` 0.46 (`gateway` feature) and `winreg` 0.56 (Windows
only), ADR-022 · HTTP via `std::net::TcpStream` with a fixed request · Windows code paths checked
locally with `cargo clippy --target x86_64-pc-windows-gnu` and run in CI on `windows-latest`.

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I / Security | Fixed targets only (gateway ports, two HTTP endpoints) | PASS |
| II | No elevation; TCP instead of ICMP | PASS |
| IV | No IP/MAC/interface names; processes only name/CPU/memory; startup names are N1-S with consent | PASS |
| V.9 (plan) | Untrusted names sanitized (`sanitize_untrusted`) | PASS |

## Lessons from the real run (2026-10-14, Debian 13)

- `sysinfo` lists Linux threads as processes by default → `without_tasks()` + `thread_kind()` filter.
- `/etc/xdg/autostart` has entries for other desktops and missing binaries → `OnlyShowIn`,
  `NotShowIn` (vs `XDG_CURRENT_DESKTOP`) and `TryExec` are honored before counting.
