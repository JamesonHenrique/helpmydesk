# Feature Specification: Read-only probes — batch 2

**Feature Branch**: `005-probes-batch-2`
**Created**: 2026-10-14
**Status**: Approved (owner approval, 2026-10-14)
**Input**: Plan v1 V.5 (probe tables, Windows and Linux)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - "My internet stopped" can be narrowed down offline (Priority: P1)

The connectivity playbook needs to tell apart: no active network adapter, no valid IP, no default
gateway, gateway not answering, internet unreachable, and a captive portal (hotel/café Wi-Fi login).

**Independent Test**: pure functions decide findings from recorded inputs; real probes run in CI.

**Acceptance Scenarios**:

1. **Given** no non-loopback adapter is up, **Then** `net.no_active_adapter`.
2. **Given** the default interface has only link-local IPv4 (169.254.x.x) or none, **Then**
   `net.no_valid_ip`.
3. **Given** no default gateway, **Then** `net.gateway.missing`; **given** a gateway that does not
   answer TCP on 53/80/443 within the timeout (a refused connection counts as an answer), **Then**
   `net.gateway.unreachable`.
4. **Given** fixed HTTP connectivity endpoints answer with the expected status/body, **Then** no
   finding; **given** they answer something else (e.g. a redirect), **Then**
   `net.captive_portal_suspected`; **given** none answers, **Then** `net.internet.unreachable`.

### User Story 2 - "My computer is slow" shows evidence (Priority: P1)

**Acceptance Scenarios**:

1. **Given** global CPU usage ≥ 90% over the sample, **Then** `perf.cpu.high`.
2. `perf.top_processes` lists the 5 processes using the most memory with CPU and memory, by
   executable name only (never command line, path or user).
3. **Given** consent for category `startup_apps`, `startup.apps.list` lists startup entries
   (name, enabled, scope user/system); **given** 8 or more enabled entries, **Then**
   `startup.many_apps`.

### Edge Cases

- A probe that cannot read its source returns `NO_DATA`; timeouts return `TIMEOUT`.
- Malformed `.desktop` files or registry values are skipped, never fatal.
- Evidence never contains IP addresses, MAC addresses, interface names/descriptions, SSIDs,
  command lines or paths with user names.

## Requirements *(mandatory)*

- **FR-001**: New probes MUST be catalog entries with `produces` listing every emitted finding.
- **FR-002**: Decisions MUST be pure functions tested with recorded inputs (FR-004 of spec 004).
- **FR-003**: Network targets MUST be fixed: the default gateway (ports 53, 80, 443) and the HTTP
  endpoints `http://connectivitycheck.gstatic.com/generate_204` (expects 204) and
  `http://www.msftconnecttest.com/connecttest.txt` (expects 200 + `Microsoft Connect Test`).
- **FR-004**: `startup.apps.list` MUST be N1-S with consent category `startup_apps`; sources:
  Linux XDG autostart (`~/.config/autostart`, `/etc/xdg/autostart`, user overrides system);
  Windows `HKCU`/`HKLM` `…\CurrentVersion\Run` with `StartupApproved\Run` state.
- **FR-005**: Process data MUST be limited to executable name, CPU % and memory.
- **NFR-001**: New dependencies `netdev` (MIT, `gateway` feature only) and `winreg` (MIT,
  Windows only), justified in ADR-022.

## Success Criteria *(mandatory)*

- **SC-001**: Every finding code has a pure-function test that triggers it.
- **SC-002**: Probes run in CI on Linux and Windows; the privacy test covers the new probes.

## Assumptions

- Audio, event logs and Windows privacy switches need COM/WinRT/D-Bus APIs and come in a later
  batch after spike S-01.
- Gateway reachability uses TCP instead of ICMP because ICMP needs raw sockets (privileges).
