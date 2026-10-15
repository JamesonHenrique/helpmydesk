# Feature Specification: Desktop shell (Tauri) wired to the core

**Feature Branch**: `013-desktop-shell`
**Created**: 2026-10-15
**Status**: Approved (owner approval, 2026-10-14: "aprovo tudo")
**Input**: Plan v1 ADR-004 (Tauri), V.7 (approval window, INV-04), F2 ("modo básico na interface")

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The screens talk to the real core in the desktop app (Priority: P1)

1. **Given** the desktop app, **Then** every screen calls the core through one command per use
   case (`hmd-ipc`), and the history lives in the per-user app data directory.
2. **Given** a browser (no Tauri), **Then** the same screens use example data and say so.
3. **Given** the core answers, **Then** the UI receives catalog texts and codes only, never probe
   evidence text.

### User Story 2 - Only the approval window can approve (Priority: P1)

1. **Given** the main window, **Then** its capability does not include `approve` or `decline`.
2. **Given** a plan, **When** the person asks to approve, **Then** a separate window shows the plan
   loaded from the core (not from the conversation) and is the only one allowed to approve; it
   tells the main window through an event and closes.

### User Story 3 - Honest results on screen (Priority: P1)

1. **Given** the core says a fix is not available in this build, **Then** the screen says nothing
   was changed and shows the guidance and the report option.
2. **Given** a verified fix, **Then** the screen asks "Para você também voltou a funcionar?" before
   calling it resolved.
3. **Given** startup recovered a damaged history or an interrupted fix, **Then** the shell tells
   the person.

## Requirements *(mandatory)*

- **FR-001**: `crates/hmd-ipc`: serializable views (camelCase), errors as codes, `Backend` with
  injected probe runner and clock; tested without a webview.
- **FR-002**: `apps/desktop/src-tauri`: separate workspace (WebKitGTK needed on Linux); commands
  are `async`; ACL from `build.rs` + `capabilities/{main,approval}.json`; CSP in
  `tauri.conf.json`; bundling off.
- **FR-003**: UI `CoreApi` with `TauriApi` (invoke + events) and `MockApi` (examples); hash
  routing so the approval window URL works.
- **NFR-001**: Tauri 2.12.1 / tauri-build 2.7.1 / @tauri-apps/api 2.12.1, ADR-025.

## Success Criteria *(mandatory)*

- **SC-001**: `hmd-ipc` tests cover the flow as JSON, errors and erase; UI tests cover each screen.
- **SC-002**: CI builds the shell on Ubuntu and Windows.

## Assumptions

- Nobody has launched the desktop app yet: it compiles in CI, but a manual run needs
  `libwebkit2gtk-4.1-dev` on the dev machine (or a Windows machine).
- Saving the report uses a browser download; in WebKitGTK this may need the dialog plugin later.
