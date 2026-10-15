// SPDX-License-Identifier: GPL-3.0-or-later
//! Sound, microphone and camera probes (spec 010). Only counts and allow/deny states: never
//! device names, recordings or which apps used them.
//!
//! Linux: `/proc/asound/pcm` (ALSA devices), the sound server socket in `$XDG_RUNTIME_DIR`
//! (PipeWire or PulseAudio) and `/dev/video*`. Windows: the `MMDevices` registry keys (audio
//! endpoints) and the `CapabilityAccessManager\ConsentStore` privacy switches.

use crate::{Evidence, ProbeError, ProbeOutput, rules};

/// Counts output (`playback`) and input (`capture`) devices in `/proc/asound/pcm`. Lines look
/// like `00-00: ALC887-VD Analog : ALC887-VD Analog : playback 1 : capture 1`. Pure.
pub fn parse_asound_pcm(text: &str) -> (u64, u64) {
    let mut outputs = 0;
    let mut inputs = 0;
    for line in text.lines() {
        // `any`, not a sum: absurd counts in the file must not overflow (spec 016).
        let has = |kind: &str| {
            line.split(':')
                .map(str::trim)
                .filter_map(|part| part.strip_prefix(kind))
                .filter_map(|n| n.trim().parse::<u64>().ok())
                .any(|n| n > 0)
        };
        if has("playback") {
            outputs += 1;
        }
        if has("capture") {
            inputs += 1;
        }
    }
    (outputs, inputs)
}

/// `MMDevices` `DeviceState` value meaning "active" (`DEVICE_STATE_ACTIVE`).
pub const DEVICE_STATE_ACTIVE: u32 = 0x1;

/// Number of active endpoints among `DeviceState` values. Pure.
pub fn active_endpoints(states: &[u32]) -> u64 {
    states.iter().filter(|s| **s == DEVICE_STATE_ACTIVE).count() as u64
}

/// A privacy switch blocks access when any of its values (machine or user) is `Deny`. Pure.
pub fn privacy_denied(values: &[Option<String>]) -> bool {
    values
        .iter()
        .flatten()
        .any(|v| v.trim().eq_ignore_ascii_case("deny"))
}

pub(crate) fn audio_devices() -> Result<ProbeOutput, ProbeError> {
    let devices = platform::audio()?;
    let mut out = ProbeOutput::new("av.audio.devices");
    out.evidence
        .insert("output_devices", Evidence::Uint(devices.outputs));
    out.evidence
        .insert("input_devices", Evidence::Uint(devices.inputs));
    if let Some(running) = devices.sound_server {
        out.evidence.insert("sound_server", Evidence::Bool(running));
    }
    out.findings = rules::audio_findings(devices.outputs, devices.inputs, devices.sound_server);
    Ok(out)
}

pub(crate) fn camera() -> Result<ProbeOutput, ProbeError> {
    let cameras = platform::cameras()?;
    let mut out = ProbeOutput::new("av.camera.present");
    out.evidence.insert("camera_nodes", Evidence::Uint(cameras));
    out.findings = rules::camera_findings(cameras);
    Ok(out)
}

pub(crate) fn privacy() -> Result<ProbeOutput, ProbeError> {
    let (mic_denied, camera_denied) = platform::privacy()?;
    let mut out = ProbeOutput::new("av.privacy.mic_camera");
    out.evidence
        .insert("microphone_denied", Evidence::Bool(mic_denied));
    out.evidence
        .insert("camera_denied", Evidence::Bool(camera_denied));
    out.findings = rules::privacy_findings(mic_denied, camera_denied);
    Ok(out)
}

pub(crate) struct AudioDevices {
    pub outputs: u64,
    pub inputs: u64,
    /// `None` when it cannot be told (Windows, or no `$XDG_RUNTIME_DIR`).
    pub sound_server: Option<bool>,
}

#[cfg(target_os = "linux")]
mod platform {
    use std::path::PathBuf;

    use super::{AudioDevices, parse_asound_pcm};
    use crate::ProbeError;

    pub(super) fn audio() -> Result<AudioDevices, ProbeError> {
        let pcm = match std::fs::read_to_string("/proc/asound/pcm") {
            Ok(text) => text,
            // ALSA loaded but no `pcm` file: no devices.
            Err(_) if std::path::Path::new("/proc/asound").is_dir() => String::new(),
            // No `/proc/asound` means ALSA is not loaded: no data, never "zero devices".
            Err(_) => return Err(ProbeError::NoData),
        };
        let (outputs, inputs) = parse_asound_pcm(&pcm);
        let sound_server = std::env::var_os("XDG_RUNTIME_DIR").map(|dir| {
            let dir = PathBuf::from(dir);
            dir.join("pipewire-0").exists() || dir.join("pulse").join("native").exists()
        });
        Ok(AudioDevices {
            outputs,
            inputs,
            sound_server,
        })
    }

    pub(super) fn cameras() -> Result<u64, ProbeError> {
        let dev = std::fs::read_dir("/dev").map_err(|_| ProbeError::NoData)?;
        Ok(dev
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.starts_with("video"))
            })
            .count() as u64)
    }

    pub(super) fn privacy() -> Result<(bool, bool), ProbeError> {
        // Linux desktops have no system-wide switch the app can read without D-Bus portals.
        Err(ProbeError::NoData)
    }
}

#[cfg(windows)]
mod platform {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};

    use super::{AudioDevices, active_endpoints, privacy_denied};
    use crate::ProbeError;

    const MMDEVICES: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio";
    const CONSENT: &str =
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore";

    fn endpoint_states(kind: &str) -> Option<Vec<u32>> {
        let root = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags(format!(r"{MMDEVICES}\{kind}"), KEY_READ)
            .ok()?;
        Some(
            root.enum_keys()
                .flatten()
                .filter_map(|id| root.open_subkey_with_flags(&id, KEY_READ).ok())
                .filter_map(|k| k.get_value::<u32, _>("DeviceState").ok())
                .collect(),
        )
    }

    pub(super) fn audio() -> Result<AudioDevices, ProbeError> {
        let render = endpoint_states("Render").ok_or(ProbeError::NoData)?;
        let capture = endpoint_states("Capture").unwrap_or_default();
        Ok(AudioDevices {
            outputs: active_endpoints(&render),
            inputs: active_endpoints(&capture),
            sound_server: None,
        })
    }

    pub(super) fn cameras() -> Result<u64, ProbeError> {
        Err(ProbeError::NoData)
    }

    fn switch_values(capability: &str) -> Vec<Option<String>> {
        [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER]
            .into_iter()
            .map(|hive| {
                RegKey::predef(hive)
                    .open_subkey_with_flags(format!(r"{CONSENT}\{capability}"), KEY_READ)
                    .ok()
                    .and_then(|k| k.get_value::<String, _>("Value").ok())
            })
            .collect()
    }

    pub(super) fn privacy() -> Result<(bool, bool), ProbeError> {
        Ok((
            privacy_denied(&switch_values("microphone")),
            privacy_denied(&switch_values("webcam")),
        ))
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod platform {
    use super::AudioDevices;
    use crate::ProbeError;

    pub(super) fn audio() -> Result<AudioDevices, ProbeError> {
        Err(ProbeError::NoData)
    }
    pub(super) fn cameras() -> Result<u64, ProbeError> {
        Err(ProbeError::NoData)
    }
    pub(super) fn privacy() -> Result<(bool, bool), ProbeError> {
        Err(ProbeError::NoData)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recorded from a Debian 13 desktop (Intel HDA + HDMI).
    const PCM: &str = "00-00: ALC887-VD Analog : ALC887-VD Analog : playback 1 : capture 1\n\
00-02: ALC887-VD Alt Analog : ALC887-VD Alt Analog : capture 1\n\
00-03: HDMI 0 : HDMI 0 : playback 1\n\
00-07: HDMI 1 : HDMI 1 : playback 1\n\
00-08: HDMI 2 : HDMI 2 : playback 1\n";

    #[test]
    fn asound_pcm_with_absurd_counts_does_not_overflow() {
        let line = "00-00: x : playback 18446744073709551615 : playback 1 : capture 0\n";
        assert_eq!(parse_asound_pcm(line), (1, 0));
        assert_eq!(parse_asound_pcm(":::\nplayback\n\u{0}"), (0, 0));
    }

    #[test]
    fn asound_pcm_counts_outputs_and_inputs() {
        assert_eq!(parse_asound_pcm(PCM), (4, 2));
        assert_eq!(parse_asound_pcm(""), (0, 0));
        assert_eq!(parse_asound_pcm("garbage\n: : playback x"), (0, 0));
    }

    #[test]
    fn mmdevices_active_states() {
        // 1 active, 2 disabled, 4 not present, 8 unplugged.
        assert_eq!(active_endpoints(&[1, 2, 4, 8, 1]), 2);
        assert_eq!(active_endpoints(&[]), 0);
    }

    #[test]
    fn privacy_switch_deny_in_any_hive_blocks() {
        assert!(privacy_denied(&[Some("Allow".into()), Some("Deny".into())]));
        assert!(privacy_denied(&[Some("Deny".into()), None]));
        assert!(!privacy_denied(&[Some("Allow".into()), None]));
        assert!(
            !privacy_denied(&[None, None]),
            "missing keys are not a block"
        );
    }
}
