// SPDX-License-Identifier: GPL-3.0-or-later
//! `net.dns.resolve`: resolves a fixed list of names through the system resolver (FR-006).

use std::net::ToSocketAddrs;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::{Evidence, ProbeError, ProbeOutput, rules};

/// Fixed names; never taken from the model or the person.
pub(crate) const NAMES: &[&str] = &["dns.google", "one.one.one.one", "www.microsoft.com"];
/// Overall time limit for the probe.
pub(crate) const TIMEOUT: Duration = Duration::from_secs(5);

/// Resolves each name on its own thread so a hanging resolver cannot block the probe.
/// Threads still running after the deadline are detached; they hold no shared state.
pub(crate) fn resolve(
    names: &[&'static str],
    timeout: Duration,
) -> Result<ProbeOutput, ProbeError> {
    let (tx, rx) = mpsc::channel();
    for &name in names {
        let tx = tx.clone();
        thread::spawn(move || {
            let ok = (name, 443)
                .to_socket_addrs()
                .is_ok_and(|mut addrs| addrs.next().is_some());
            let _ = tx.send(ok);
        });
    }
    drop(tx);

    let deadline = Instant::now() + timeout;
    let (mut answered, mut resolved) = (0usize, 0usize);
    while answered < names.len() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(remaining) {
            Ok(ok) => {
                answered += 1;
                resolved += usize::from(ok);
            }
            Err(_) => break,
        }
    }
    // US2-3: if nothing answered in time we do not know; never invent a result.
    if answered == 0 && !names.is_empty() {
        return Err(ProbeError::Timeout);
    }

    let mut out = ProbeOutput::new("net.dns.resolve");
    out.evidence
        .insert("attempted", Evidence::Uint(answered as u64));
    out.evidence
        .insert("resolved", Evidence::Uint(resolved as u64));
    out.evidence.insert(
        "resolved_ratio",
        Evidence::Float(if answered == 0 {
            0.0
        } else {
            resolved as f64 / answered as f64
        }),
    );
    out.findings = rules::dns_findings(resolved, answered);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn us2_unresolvable_names_produce_failing_finding() {
        // `.invalid` is reserved (RFC 2606) and never resolves; no network needed.
        let out = resolve(&["a.invalid", "b.invalid"], Duration::from_secs(10)).unwrap();
        assert_eq!(out.findings[0].code, rules::DNS_FAILING);
        assert_eq!(out.evidence["resolved"], Evidence::Uint(0));
    }

    #[test]
    fn us2_localhost_resolves() {
        let out = resolve(&["localhost"], Duration::from_secs(10)).unwrap();
        assert!(out.findings.is_empty());
        assert_eq!(out.evidence["resolved_ratio"], Evidence::Float(1.0));
    }

    #[test]
    fn us2_zero_timeout_never_invents_result() {
        match resolve(&["a.invalid"], Duration::ZERO) {
            Err(e) => assert_eq!(e, ProbeError::Timeout),
            // Only answers that actually arrived are counted.
            Ok(out) => assert_eq!(out.evidence["attempted"], Evidence::Uint(1)),
        }
    }
}
