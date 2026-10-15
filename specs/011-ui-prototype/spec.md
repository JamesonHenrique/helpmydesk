# Feature Specification: UI prototype (Angular)

**Feature Branch**: `011-ui-prototype`
**Created**: 2026-10-15
**Status**: Approved (owner approval, 2026-10-14: "aprovo tudo")
**Input**: Plan v1 F1 ("protótipo Angular de todas as telas"), III.13 (stages A–H, screen map, accessibility)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Every MVP screen can be walked through (Priority: P1)

Welcome (stage A), new ticket in basic mode (B), checks running with "Parar" (C), explanation with
confidence words (D), plan and approval with catalog risk/undo texts (E), execution and the four
verification outcomes (F–G), closing or the editable "não consegui" report (H), history,
settings (privacy, autonomy, AI, about) and help.

**Acceptance Scenarios**:

1. **Given** basic mode, **Then** the start screen offers the five families plus "Não sei explicar"
   and the free-text box is disabled.
2. **Given** a family, **Then** the checks run one by one saying nothing is being changed, and
   checks that do not exist on the platform are shown as skipped.
3. **Given** a diagnosis with fixes, **Then** the plan shows each fix's catalog title, risk and
   undo text, warns when the system will ask for permission, and offers "Não, obrigado".
4. **Given** the report screen, **Then** it shows a sample rendered by the real `report@1`
   renderer in an editable box with copy and save.
5. **Given** settings, **Then** "o que eu olho" lists every probe of the chosen platform with
   `looks_at` and `never`, and "Apagar tudo" asks for confirmation in the page.

### Edge Cases

- Every screen states it is a prototype with example data; nothing touches the computer.
- Texts that exist in the catalog are never retyped: they are generated from it.

## Requirements *(mandatory)*

- **FR-001**: Catalog texts reach the UI only through `catalog.generated.ts`, produced by
  `cargo run -p hmd-core --example ui_fixtures`; CI fails when it is stale.
- **FR-002**: Accessibility basics: `lang="pt-BR"`, skip link, visible focus, keyboard-only use,
  adjustable text size, light/dark themes with AA contrast, nothing conveyed by color alone.
- **FR-003**: Strict CSP in `index.html` (no remote scripts, styles from self).
- **FR-004**: Exact dependency versions, each at least two weeks old when added; `npm ci
  --ignore-scripts` and `npm audit` in CI.

## Success Criteria *(mandatory)*

- **SC-001**: Each acceptance scenario has a component test (Vitest).
- **SC-002**: CI builds and tests the prototype.

## Assumptions

- The approval screen becomes a separate Tauri window with its own IPC capability later (INV-04).
- Usability with lay people (E-03) and a screen-reader pass are still to be done.
