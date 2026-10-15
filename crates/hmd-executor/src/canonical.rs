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

use std::fmt::Write as _;

use hmd_policy::{ActionPlanProposal, ApprovalRequirement, PolicyDecision, Rollback, StepVerdict};
use sha2::{Digest, Sha256};

pub const PLAN_FORMAT: &str = "hmd-plan@1";

fn esc(value: &str) -> String {
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
    let _ = writeln!(s, "catalog={}", esc(catalog_version));
    let _ = writeln!(s, "session={}", esc(&proposal.session_id));
    for step in &proposal.steps {
        let _ = writeln!(
            s,
            "step={}|{}|{}",
            esc(&step.step_id),
            esc(&step.capability),
            esc(&step.version)
        );
        for (name, value) in &step.params {
            let _ = writeln!(
                s,
                "param={}|{}|{}",
                esc(&step.step_id),
                esc(name),
                esc(value)
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
                    esc(step_id),
                    risk_level.code(),
                    approval_code(*approval),
                    u8::from(*requires_elevation),
                    verification.as_deref().map_or_else(|| "-".to_string(), esc),
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
                    esc(step_id),
                    esc(capability),
                    reason.reason_code()
                );
            }
            StepVerdict::Rejected { step_id, reason } => {
                let _ = writeln!(
                    s,
                    "verdict={}|rejected|{}",
                    esc(step_id),
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
    sha256_tagged(&format!("{}|{}|{nonce}", esc(session_id), esc(step_id)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_cannot_forge_separators() {
        assert_eq!(esc("a|b\nc\\d\re"), "a\\pb\\nc\\\\d\\re");
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
}
