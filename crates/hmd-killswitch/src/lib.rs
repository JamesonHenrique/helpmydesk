// SPDX-License-Identifier: GPL-3.0-or-later
//! Signed kill switch (spec 014, Plan v1 VII.3 `disabled-capabilities.json`): lets the maintainers
//! turn off a catalog capability or an AI model without a new release. Pure: the caller fetches
//! the file and its detached signature and supplies `now`.
//!
//! File: UTF-8 JSON, exactly
//! `{"format": 1, "issued_at": <unix s>, "expires_at": <unix s>, "disabled_capabilities": [...],
//!   "disabled_models": [...], "min_app_version": "<x.y.z>" (optional)}`.
//! Signature: Ed25519 over the exact file bytes, as 128 hex characters (`*.sig`).
//!
//! Rules: only keys compiled into the app are trusted ([`TRUSTED_KEYS`]); a file older than the
//! last accepted one (`issued_at`) is refused, so an attacker cannot replay an old, more
//! permissive file; an already expired file is refused. Once accepted, the disables keep applying
//! offline until a newer file replaces them (turning things off is the safe direction).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::BTreeSet;
use std::fmt;

use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;

/// The only file format this build understands (Plan v1 XV: unknown versions are refused).
pub const FORMAT: u32 = 1;

/// Maximum accepted file size.
pub const MAX_BYTES: usize = 64 * 1024;

/// How far in the future `issued_at` may be (clock differences).
pub const MAX_CLOCK_SKEW: i64 = 24 * 60 * 60;

/// Ed25519 public keys of the release key, compiled into the app. **Empty until the maintainer
/// generates the release key** (see `docs/ops/kill-switch.md`); with no key every file is refused.
pub const TRUSTED_KEYS: &[[u8; 32]] = &[];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: u32,
    issued_at: i64,
    expires_at: i64,
    disabled_capabilities: Vec<String>,
    disabled_models: Vec<String>,
    #[serde(default)]
    min_app_version: Option<String>,
}

/// A verified and accepted kill switch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillSwitch {
    pub issued_at: i64,
    pub expires_at: i64,
    pub disabled_capabilities: BTreeSet<String>,
    pub disabled_models: BTreeSet<String>,
    pub min_app_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KillSwitchError {
    NoTrustedKey,
    TooLarge,
    BadSignature,
    Malformed(String),
    UnknownFormat(u32),
    /// `issued_at` is not newer than the last accepted file (replay or rollback).
    NotNewer {
        issued_at: i64,
        last: i64,
    },
    Expired,
    FromTheFuture,
}

impl KillSwitchError {
    pub fn reason_code(&self) -> &'static str {
        match self {
            KillSwitchError::NoTrustedKey => "KILLSWITCH_NO_TRUSTED_KEY",
            KillSwitchError::TooLarge => "KILLSWITCH_TOO_LARGE",
            KillSwitchError::BadSignature => "KILLSWITCH_BAD_SIGNATURE",
            KillSwitchError::Malformed(_) => "KILLSWITCH_MALFORMED",
            KillSwitchError::UnknownFormat(_) => "KILLSWITCH_UNKNOWN_FORMAT",
            KillSwitchError::NotNewer { .. } => "KILLSWITCH_NOT_NEWER",
            KillSwitchError::Expired => "KILLSWITCH_EXPIRED",
            KillSwitchError::FromTheFuture => "KILLSWITCH_FROM_THE_FUTURE",
        }
    }
}

impl fmt::Display for KillSwitchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KillSwitchError::Malformed(m) => write!(f, "{}: {m}", self.reason_code()),
            KillSwitchError::NotNewer { issued_at, last } => {
                write!(f, "{}: {issued_at} <= {last}", self.reason_code())
            }
            other => write!(f, "{}", other.reason_code()),
        }
    }
}

impl std::error::Error for KillSwitchError {}

fn signature_from_hex(hex: &str) -> Option<Signature> {
    let hex = hex.trim();
    if hex.len() != 128 {
        return None;
    }
    let mut bytes = [0u8; 64];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(Signature::from_bytes(&bytes))
}

/// Checks the signature against the trusted keys, then parses. Nothing is parsed before the
/// signature is valid.
pub fn verify(
    file: &[u8],
    signature_hex: &str,
    trusted: &[[u8; 32]],
) -> Result<KillSwitch, KillSwitchError> {
    if trusted.is_empty() {
        return Err(KillSwitchError::NoTrustedKey);
    }
    if file.len() > MAX_BYTES {
        return Err(KillSwitchError::TooLarge);
    }
    let signature = signature_from_hex(signature_hex).ok_or(KillSwitchError::BadSignature)?;
    let valid = trusted.iter().any(|key| {
        VerifyingKey::from_bytes(key).is_ok_and(|k| k.verify_strict(file, &signature).is_ok())
    });
    if !valid {
        return Err(KillSwitchError::BadSignature);
    }
    let raw: Raw =
        serde_json::from_slice(file).map_err(|e| KillSwitchError::Malformed(e.to_string()))?;
    if raw.format != FORMAT {
        return Err(KillSwitchError::UnknownFormat(raw.format));
    }
    if raw.expires_at <= raw.issued_at {
        return Err(KillSwitchError::Malformed(
            "expires_at must be after issued_at".into(),
        ));
    }
    Ok(KillSwitch {
        issued_at: raw.issued_at,
        expires_at: raw.expires_at,
        disabled_capabilities: raw.disabled_capabilities.into_iter().collect(),
        disabled_models: raw.disabled_models.into_iter().collect(),
        min_app_version: raw.min_app_version,
    })
}

/// Acceptance rules for a verified file: newer than the last accepted one, not expired, not
/// issued (much) in the future.
pub fn accept(
    candidate: KillSwitch,
    last_issued_at: Option<i64>,
    now: i64,
) -> Result<KillSwitch, KillSwitchError> {
    if let Some(last) = last_issued_at
        && candidate.issued_at <= last
    {
        return Err(KillSwitchError::NotNewer {
            issued_at: candidate.issued_at,
            last,
        });
    }
    if candidate.issued_at > now.saturating_add(MAX_CLOCK_SKEW) {
        return Err(KillSwitchError::FromTheFuture);
    }
    if candidate.expires_at <= now {
        return Err(KillSwitchError::Expired);
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    const NOW: i64 = 1_760_400_000;

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn file(issued_at: i64, expires_at: i64) -> Vec<u8> {
        format!(
            r#"{{"format": 1, "issued_at": {issued_at}, "expires_at": {expires_at},
               "disabled_capabilities": ["net.dns.flush_cache"], "disabled_models": ["m1"]}}"#
        )
        .into_bytes()
    }

    fn sign(k: &SigningKey, bytes: &[u8]) -> String {
        k.sign(bytes)
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    fn trusted() -> [[u8; 32]; 1] {
        [key(7).verifying_key().to_bytes()]
    }

    #[test]
    fn valid_file_is_verified_and_accepted() {
        let bytes = file(NOW - 10, NOW + 3600);
        let ks = verify(&bytes, &sign(&key(7), &bytes), &trusted()).unwrap();
        assert!(ks.disabled_capabilities.contains("net.dns.flush_cache"));
        assert!(ks.disabled_models.contains("m1"));
        assert_eq!(accept(ks, None, NOW).unwrap().issued_at, NOW - 10);
    }

    #[test]
    fn no_key_means_nothing_is_trusted() {
        let bytes = file(NOW, NOW + 1);
        assert_eq!(
            verify(&bytes, &sign(&key(7), &bytes), TRUSTED_KEYS),
            Err(KillSwitchError::NoTrustedKey)
        );
    }

    #[test]
    fn wrong_key_or_tampered_bytes_are_refused() {
        let bytes = file(NOW, NOW + 3600);
        assert_eq!(
            verify(&bytes, &sign(&key(8), &bytes), &trusted()),
            Err(KillSwitchError::BadSignature)
        );
        let sig = sign(&key(7), &bytes);
        let mut tampered = bytes.clone();
        tampered.extend_from_slice(b" ");
        assert_eq!(
            verify(&tampered, &sig, &trusted()),
            Err(KillSwitchError::BadSignature)
        );
        assert_eq!(
            verify(&bytes, "zz", &trusted()),
            Err(KillSwitchError::BadSignature)
        );
    }

    #[test]
    fn replay_of_an_older_file_is_refused() {
        let old = file(NOW - 100, NOW + 3600);
        let ks = verify(&old, &sign(&key(7), &old), &trusted()).unwrap();
        assert_eq!(
            accept(ks.clone(), Some(NOW - 50), NOW),
            Err(KillSwitchError::NotNewer {
                issued_at: NOW - 100,
                last: NOW - 50
            })
        );
        assert!(matches!(
            accept(ks, Some(NOW - 100), NOW),
            Err(KillSwitchError::NotNewer { .. })
        ));
    }

    #[test]
    fn expired_future_unknown_format_and_extra_fields_are_refused() {
        let expired = file(NOW - 100, NOW - 1);
        let ks = verify(&expired, &sign(&key(7), &expired), &trusted()).unwrap();
        assert_eq!(accept(ks, None, NOW), Err(KillSwitchError::Expired));

        let future = file(NOW + 2 * MAX_CLOCK_SKEW, NOW + 3 * MAX_CLOCK_SKEW);
        let ks = verify(&future, &sign(&key(7), &future), &trusted()).unwrap();
        assert_eq!(accept(ks, None, NOW), Err(KillSwitchError::FromTheFuture));

        let v2 = br#"{"format": 2, "issued_at": 1, "expires_at": 2, "disabled_capabilities": [], "disabled_models": []}"#;
        assert_eq!(
            verify(v2, &sign(&key(7), v2), &trusted()),
            Err(KillSwitchError::UnknownFormat(2))
        );

        let extra = br#"{"format": 1, "issued_at": 1, "expires_at": 2, "disabled_capabilities": [], "disabled_models": [], "enable": ["shell.exec"]}"#;
        assert!(matches!(
            verify(extra, &sign(&key(7), extra), &trusted()),
            Err(KillSwitchError::Malformed(_))
        ));
    }

    #[test]
    fn oversized_file_is_refused_before_anything_else() {
        let big = vec![b' '; MAX_BYTES + 1];
        assert_eq!(
            verify(&big, &"0".repeat(128), &trusted()),
            Err(KillSwitchError::TooLarge)
        );
    }
}
