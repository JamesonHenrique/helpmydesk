# Implementation Plan: UI prototype (Angular)

**Branch**: `011-ui-prototype` | **Date**: 2026-10-15 | **Spec**: [spec.md](spec.md)

## Technical Context

`apps/desktop/ui`: Angular 22.2.1 standalone components with signals, router with component input
binding, Vitest via `@angular/build:unit-test`, Prettier. In-memory `PrototypeState` stands in
for the Rust core. Generator `crates/hmd-core/examples/ui_fixtures.rs`.

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| III | Honest states: four verification outcomes, "Não tenho certeza", report | PASS |
| IV | "O que eu olho" from catalog; nothing collected by the prototype | PASS |
| VII | Frontend dependencies pinned, audited, scripts disabled in CI | PASS |
