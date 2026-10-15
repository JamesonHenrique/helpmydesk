# Implementation Plan: Playbooks and basic mode

**Branch**: `006-playbooks-basic-mode` | **Date**: 2026-10-14 | **Spec**: [spec.md](spec.md)

## Technical Context

Playbooks as TOML in `catalog/playbooks/`, embedded by `hmd-catalog/build.rs` and validated in
`hmd-catalog::playbook`. New crate `crates/hmd-core` (no new external dependencies) with the pure
`select` and the orchestration `diagnose_with`/`diagnose`. Text runner `examples/basic_mode.rs`.

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I | Diagnosis is deterministic; fixes go through the Policy Engine | PASS |
| III | No execution in this spec; fallback is honest ("não encontrei") | PASS |
| IV | Probes authorized by policy (consent for N1-S) | PASS |
| VII | No new dependency | PASS |

## Design decisions

- Probes are capabilities: the playbook's probe list is turned into a proposal and evaluated by
  the Policy Engine (consent, kill switch, platform) before running.
- Confidence drops one level when any probe fails; N3 fixes then hit `LOW_CONFIDENCE`.
- Connectivity order: adapter → IP → gateway → captive portal → internet down → DNS. Internet down
  also breaks DNS, so `net.http.check` now probes fixed public IPs to tell them apart (spec 005
  FR-003 amended).
- Audio/video and app errors have no probes yet: guidance + REP-01, never fake checks.
