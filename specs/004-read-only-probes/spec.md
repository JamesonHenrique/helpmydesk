# Feature Specification: Read-only probes (Windows and Linux)

**Feature Branch**: `004-read-only-probes`
**Created**: 2026-10-14
**Status**: Approved (owner approval, 2026-10-14)
**Input**: Plan v1 V.5 (probe table), VIII.6 (parsing tests with recorded outputs)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The app sees basic facts without privileges (Priority: P1)

Without administrator rights, the app reads OS, uptime, memory and disk space and turns them into
structured evidence and findings (e.g. `storage.low_space`, `perf.memory.high`).

**Independent Test**: run each probe in CI on Linux and Windows as a normal user; rule functions are
tested with fixed numbers.

**Acceptance Scenarios**:

1. **Given** a normal user, **When** `sys.info.basic` runs, **Then** it returns OS name/version,
   kernel/build, architecture and uptime, and **never** the host name or user name.
2. **Given** memory available below 10% of total, **When** `perf.memory.snapshot` runs, **Then**
   finding `perf.memory.high`.
3. **Given** the system volume with less than 10% or less than 5 GiB free, **When**
   `storage.volumes` runs, **Then** finding `storage.low_space`.

### User Story 2 - DNS is tested against fixed names only (Priority: P1)

`net.dns.resolve` resolves a fixed list of names defined in code/catalog, never names from the
model or the person, with a timeout.

**Acceptance Scenarios**:

1. **Given** all names fail to resolve, **Then** finding `net.dns.resolution_failing`.
2. **Given** at least one resolves, **Then** no failing finding and evidence `resolved_ratio`.
3. **Given** resolution hangs, **Then** the probe returns within the timeout with outcome
   `probe_failed` (never an invented result).

### Edge Cases

- Missing data (e.g. no `/etc/os-release`) → field omitted, probe still succeeds.
- No disk information available → `probe_failed` with reason code `NO_DATA`.

## Requirements *(mandatory)*

- **FR-001**: Probes MUST be read-only, need no elevation and spawn no processes.
- **FR-002**: Each probe MUST have an ID present in the catalog as `kind = "probe"`, and every
  finding it can emit MUST be listed in that entry's `produces` (checked by a test).
- **FR-003**: Probe output MUST be structured: `probe_id`, `findings[]` (code, severity),
  `evidence` (typed key/values), and failures carry a stable reason code.
- **FR-004**: Thresholds MUST live in pure rule functions with unit tests.
- **FR-005**: No probe may collect host name, user name, serial numbers, MAC addresses, SSIDs or
  volume labels (privacy, constitution IV).
- **FR-006**: DNS names MUST come from a fixed list; resolution MUST be bounded by a timeout.
- **NFR-001**: Dependency `sysinfo` (MIT) only, features limited to what is used (ADR-021).

## Success Criteria *(mandatory)*

- **SC-001**: Probes run in CI on Linux and Windows without elevation.
- **SC-002**: A test proves FR-002 against the embedded catalog.
- **SC-003**: Evidence keys are checked against a deny-list of identifying fields (FR-005).

## Assumptions

- Network reachability (gateway ping, HTTP checks), audio, startup items and event logs come in a
  next probe batch (they need platform APIs evaluated in spike S-01/S-02).
