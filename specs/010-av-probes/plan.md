# Implementation Plan: Sound, microphone and camera probes (batch 3)

**Branch**: `010-av-probes` | **Date**: 2026-10-15 | **Spec**: [spec.md](spec.md)

## Technical Context

`hmd-probes::av` (pure parsers + per-platform readers), rules in `hmd-probes::rules`, three
catalog entries, PB-AV-001 1.1.0, catalog version 2026.10.1. No new dependency (`winreg` already
present, ADR-022).

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| IV | Counts and allow/deny only; disclosure texts | PASS |
| VI | No processes, no elevation | PASS |
| VII | No new dependency | PASS |
