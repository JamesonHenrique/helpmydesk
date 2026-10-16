// SPDX-License-Identifier: GPL-3.0-or-later
//! Canonical encoding of an evaluated plan and its hash (spec 008 FR-001).
//!
//! Format `hmd-plan@1`: UTF-8 text, one `key=value` per line, `\n` line ends, in this order:
//!
//! ```text
//! hmd-plan@1
//! catalog=<catalog version>
//! session=<session id>
//! step=<step_id>|<capability>|<version>        (one per step, plan order)
//! param=<step_id>|<name>|<value>               (after its step, proposal order)
//! verdict=<step_id>|accepted|<risk>|<approval>|<elevation 0/1>|<verification or ->|<rollback>
//! verdict=<step_id>|downgraded|<capability>|<reason code>
//! verdict=<step_id>|rejected|<reason code>
//! ```
//!
//! Every value is escaped: `\` → `\\`, `|` → `\p`, line feed → `\n`, carriage return → `\r`, so
//! no value can forge a separator. Reordering steps, params or verdicts changes the text and
//! therefore the hash.
//!
//! Spec 020 moved this module out of `hmd-executor` into its own crate so the elevated helper
//! recalculates the hash with the same code as the executor: two implementations of `hmd-plan@1`
//! would be a hole. The helper parses the text back with [`parse_line`], which applies the same
//! escaping rules in reverse.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::fmt::Write as _;

use hmd_policy::{ActionPlanProposal, ApprovalRequirement, PolicyDecision, Rollback, StepVerdict};
use sha2::{Digest, Sha256};

pub const PLAN_FORMAT: &str = "hmd-plan@1";

/// A canonical line that does not follow the escaping rules. The elevated helper turns this into
/// `BAD_REQUEST` (spec 020 SEC-023): text it cannot read exactly is text it does not trust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalError {
    /// No `=` in the line.
    NoSeparator,
    /// A `\` before the end of the line, or an escape other than `\\`, `\p`, `\n`, `\r`.
    BadEscape,
}

impl CanonicalError {
    pub fn reason_code(&self) -> &'static str {
        match self {
            CanonicalError::NoSeparator => "CANONICAL_NO_SEPARATOR",
            CanonicalError::BadEscape => "CANONICAL_BAD_ESCAPE",
        }
    }
}

impl std::fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.reason_code())
    }
}

impl std::error::Error for CanonicalError {}

/// Escapes a value so it cannot forge a separator of the canonical text.
pub fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\p"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

fn unescape_into(value: &str, out: &mut String) -> Result<(), CanonicalError> {
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('p') => out.push('|'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            _ => return Err(CanonicalError::BadEscape),
        }
    }
    Ok(())
}

/// [`escape`] in reverse.
pub fn unescape(value: &str) -> Result<String, CanonicalError> {
    let mut out = String::with_capacity(value.len());
    unescape_into(value, &mut out)?;
    Ok(out)
}

/// Splits one `key=value|field|field` canonical line into the key and the unescaped fields. The
/// inverse of what [`canonical`] writes, used by the elevated helper to revalidate the step it was
/// asked to run (spec 020 SEC-023).
pub fn parse_line(line: &str) -> Result<(&str, Vec<String>), CanonicalError> {
    let (key, values) = line.split_once('=').ok_or(CanonicalError::NoSeparator)?;
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = values.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('\\') => current.push('\\'),
                Some('p') => current.push('|'),
                Some('n') => current.push('\n'),
                Some('r') => current.push('\r'),
                _ => return Err(CanonicalError::BadEscape),
            },
            '|' => fields.push(std::mem::take(&mut current)),
            c => current.push(c),
        }
    }
    fields.push(current);
    Ok((key, fields))
}

fn approval_code(a: ApprovalRequirement) -> &'static str {
    match a {
        ApprovalRequirement::None => "none",
        ApprovalRequirement::Plan => "plan",
        ApprovalRequirement::PerAction => "per_action",
    }
}

fn rollback_code(r: Rollback) -> &'static str {
    match r {
        Rollback::NotNeeded => "not_needed",
        Rollback::Implemented => "implemented",
        Rollback::Impossible => "impossible",
    }
}

/// The canonical text of a proposal, its policy decision and the catalog version. Pure.
pub fn canonical(
    proposal: &ActionPlanProposal,
    decision: &PolicyDecision,
    catalog_version: &str,
) -> String {
    let mut s = String::new();
    // Writing into a String cannot fail.
    let _ = writeln!(s, "{PLAN_FORMAT}");
    let _ = writeln!(s, "catalog={}", escape(catalog_version));
    let _ = writeln!(s, "session={}", escape(&proposal.session_id));
    for step in &proposal.steps {
        let _ = writeln!(
            s,
            "step={}|{}|{}",
            escape(&step.step_id),
            escape(&step.capability),
            escape(&step.version)
        );
        for (name, value) in &step.params {
            let _ = writeln!(
                s,
                "param={}|{}|{}",
                escape(&step.step_id),
                escape(name),
                escape(value)
            );
        }
    }
    for verdict in &decision.verdicts {
        match verdict {
            StepVerdict::Accepted {
                step_id,
                capability: _,
                risk_level,
                approval,
                requires_elevation,
                verification,
                rollback,
            } => {
                let _ = writeln!(
                    s,
                    "verdict={}|accepted|{}|{}|{}|{}|{}",
                    escape(step_id),
                    risk_level.code(),
                    approval_code(*approval),
                    u8::from(*requires_elevation),
                    verification
                        .as_deref()
                        .map_or_else(|| "-".to_string(), escape),
                    rollback_code(*rollback)
                );
            }
            StepVerdict::Downgraded {
                step_id,
                capability,
                reason,
            } => {
                let _ = writeln!(
                    s,
                    "verdict={}|downgraded|{}|{}",
                    escape(step_id),
                    escape(capability),
                    reason.reason_code()
                );
            }
            StepVerdict::Rejected { step_id, reason } => {
                let _ = writeln!(
                    s,
                    "verdict={}|rejected|{}",
                    escape(step_id),
                    reason.reason_code()
                );
            }
        }
    }
    s
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// `sha256:<64 hex>` of a text.
pub fn sha256_tagged(text: &str) -> String {
    format!("sha256:{}", hex(&Sha256::digest(text.as_bytes())))
}

/// US1.1: hash of the canonical text.
pub fn plan_hash(
    proposal: &ActionPlanProposal,
    decision: &PolicyDecision,
    catalog_version: &str,
) -> String {
    sha256_tagged(&canonical(proposal, decision, catalog_version))
}

/// US1.5: `sha256(session_id | step_id | nonce)`, escaped like the canonical text.
pub fn idempotency_key(session_id: &str, step_id: &str, nonce: i64) -> String {
    sha256_tagged(&format!(
        "{}|{}|{nonce}",
        escape(session_id),
        escape(step_id)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_cannot_forge_separators() {
        assert_eq!(escape("a|b\nc\\d\re"), "a\\pb\\nc\\\\d\\re");
        assert_ne!(
            idempotency_key("a|b", "c", 1),
            idempotency_key("a", "b|c", 1)
        );
    }

    #[test]
    fn sha256_known_vector() {
        // SHA-256("abc"), FIPS 180-2 test vector.
        assert_eq!(
            sha256_tagged("abc"),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    /// SEC-023: the helper re-reads what the executor wrote, so parsing must be the exact
    /// inverse of writing, including for values that contain separators.
    #[test]
    fn parse_line_is_the_inverse_of_canonical() {
        for value in [
            "simple",
            "a|b",
            "linha\nquebra",
            "carro\r\n",
            "símbolo \\ e | juntos",
        ] {
            let escaped = escape(value);
            let line = format!("param=s1|{escaped}");
            let (key, fields) = parse_line(&line).unwrap();
            assert_eq!(key, "param");
            assert_eq!(fields, vec!["s1".to_string(), value.to_string()]);
        }
    }

    #[test]
    fn parse_line_rejects_text_it_cannot_read_exactly() {
        assert_eq!(
            parse_line("no-separator").unwrap_err(),
            CanonicalError::NoSeparator
        );
        // A dangling backslash and an escape the writer never produces.
        assert_eq!(
            parse_line("step=s1\\").unwrap_err(),
            CanonicalError::BadEscape
        );
        assert_eq!(
            parse_line("step=s1\\q").unwrap_err(),
            CanonicalError::BadEscape
        );
    }

    #[test]
    fn unescape_rejects_unknown_escapes() {
        assert_eq!(unescape("a\\pb").unwrap(), "a|b");
        assert!(unescape("a\\").is_err());
    }
}
