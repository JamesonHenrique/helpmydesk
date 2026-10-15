# Implementation Plan: Desktop shell (Tauri) wired to the core

**Branch**: `013-desktop-shell` | **Date**: 2026-10-15 | **Spec**: [spec.md](spec.md)

## Technical Context

`hmd-ipc` (Rust, serde) over `hmd-app`; `apps/desktop/src-tauri` (Tauri 2) registers one command
per `Backend` method; the Angular UI gets a `CoreApi` token resolved to `TauriApi` inside the app
and `MockApi` in a browser. Approval: `open_approval` → window `approval` → `plan` / `approve` /
`decline` → event `hmd://approval-decided` → main window executes.

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I / INV-04 | Approval only in its own window with an exclusive capability | PASS |
| IV | Views carry no probe evidence text | PASS |
| VII | Tauri tree licenses reviewed (ADR-025) | PASS |
