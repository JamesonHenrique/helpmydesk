# Feature Specification: Sound, microphone and camera probes (batch 3)

**Feature Branch**: `010-av-probes`
**Created**: 2026-10-15
**Status**: Approved (owner approval, 2026-10-14: "aprovo tudo")
**Input**: Plan v1 V.5 (probe tables), spec 005 Assumptions (audio deferred), family "Som/Microfone"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - "Ninguém me ouve na chamada" gets a real check (Priority: P1)

1. **Given** the Windows privacy switch denies the microphone (machine or user), **Then**
   `av.mic.blocked_by_privacy` and the diagnosis says how to turn it on (almost certain).
2. Same for the camera (`av.camera.blocked_by_privacy`).
3. **Given** no active output device, **Then** `av.no_output_device`; no active input device,
   **Then** `av.no_input_device`.
4. **Given** Linux and no PipeWire/PulseAudio socket in `$XDG_RUNTIME_DIR`, **Then**
   `av.no_sound_server`.
5. **Given** Linux and no `/dev/video*`, **Then** `av.no_camera`, shown last and as "unsure"
   because the person may be asking about sound.

### Edge Cases

- No `/proc/asound` (ALSA not loaded, e.g. CI) → `NO_DATA`, never "zero devices".
- A probe without a variant for the platform is skipped by the Policy Engine
  (`PLATFORM_UNSUPPORTED`) and returns `NO_DATA` if called directly.
- Evidence holds counts and booleans only: no device names, no app usage history.

## Requirements *(mandatory)*

- **FR-001**: Probes `av.audio.devices` (Windows, Linux), `av.camera.present` (Linux),
  `av.privacy.mic_camera` (Windows), all N1-A, with disclosure texts (spec 009).
- **FR-002**: Sources: Linux `/proc/asound/pcm`, `$XDG_RUNTIME_DIR/pipewire-0` or
  `pulse/native`, `/dev/video*`; Windows `HKLM\…\MMDevices\Audio\{Render,Capture}\*\DeviceState`
  (active = 1) and `HKLM`/`HKCU` `…\CapabilityAccessManager\ConsentStore\{microphone,webcam}`
  value `Value` (`Deny` blocks).
- **FR-003**: Decisions are pure functions tested with recorded inputs.
- **FR-004**: Playbook PB-AV-001 1.1.0 uses the three probes; diagnoses are guidance only.

## Success Criteria *(mandatory)*

- **SC-001**: Every new finding has a pure-function test; playbook scenarios tested with fakes.
- **SC-002**: The probes run in CI on both platforms without privileges; the privacy test covers them.

## Assumptions

- The Windows registry semantics (FR-002) come from documentation and common knowledge, not yet
  from a measured Windows 10/11 machine: **to confirm in spike S-01**. Until then a wrong reading
  can only produce a wrong guidance text, never an action.
- Windows camera presence and the "desktop apps" microphone switch need device/WinRT APIs (later).
- Settings menu paths for Windows 10/11 are still to be checked on real machines (open P2).
