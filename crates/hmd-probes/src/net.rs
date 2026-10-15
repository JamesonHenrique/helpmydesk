// SPDX-License-Identifier: GPL-3.0-or-later
//! Network probes (spec 005): adapters, default gateway, HTTP connectivity / captive portal.
//! Evidence never carries IP or MAC addresses, interface names or descriptions.

use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use netdev::interface::types::InterfaceType;

use crate::rules::{self, HttpOutcome};
use crate::{Evidence, ProbeError, ProbeOutput};

/// `net.adapters.status`: how many non-loopback adapters are up, and whether the default
/// interface has a usable IPv4 address.
pub(crate) fn adapters() -> ProbeOutput {
    let interfaces = netdev::get_interfaces();
    let active: Vec<_> = interfaces
        .iter()
        .filter(|i| !i.is_loopback() && i.is_up())
        .collect();
    let count = |t: InterfaceType| active.iter().filter(|i| i.if_type == t).count() as u64;
    let default_valid = netdev::get_default_interface()
        .is_ok_and(|i| i.ipv4_addrs().into_iter().any(rules::is_valid_ipv4));

    let mut out = ProbeOutput::new("net.adapters.status");
    out.evidence
        .insert("active_adapters", Evidence::Uint(active.len() as u64));
    out.evidence.insert(
        "wireless_active",
        Evidence::Uint(count(InterfaceType::Wireless80211)),
    );
    out.evidence.insert(
        "ethernet_active",
        Evidence::Uint(count(InterfaceType::Ethernet)),
    );
    out.evidence
        .insert("default_has_valid_ipv4", Evidence::Bool(default_valid));
    out.findings = rules::adapter_findings(active.len(), default_valid);
    out
}

const GATEWAY_PORTS: [u16; 3] = [53, 80, 443];
const GATEWAY_TIMEOUT: Duration = Duration::from_millis(1500);

/// A TCP answer (accepted or actively refused) proves the gateway is reachable.
fn tcp_answers(addr: SocketAddr, timeout: Duration) -> bool {
    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_) => true,
        Err(e) => e.kind() == ErrorKind::ConnectionRefused,
    }
}

fn default_gateway() -> Option<Ipv4Addr> {
    netdev::get_default_interface()
        .ok()?
        .gateway?
        .ipv4
        .into_iter()
        .find(|ip| !ip.is_unspecified())
}

/// `net.gateway.reach`: TCP to the default gateway on fixed ports (ICMP needs privileges).
pub(crate) fn gateway_reach() -> ProbeOutput {
    let mut out = ProbeOutput::new("net.gateway.reach");
    let gateway = default_gateway();
    let answered = gateway.is_some_and(|gw| {
        GATEWAY_PORTS
            .iter()
            .any(|&port| tcp_answers(SocketAddr::new(IpAddr::V4(gw), port), GATEWAY_TIMEOUT))
    });
    out.evidence
        .insert("gateway_present", Evidence::Bool(gateway.is_some()));
    out.evidence
        .insert("gateway_answered", Evidence::Bool(answered));
    if let Some(gw) = gateway {
        out.evidence
            .insert("gateway_private", Evidence::Bool(gw.is_private()));
    }
    out.findings = rules::gateway_findings(gateway.is_some(), answered);
    out
}

/// Fixed connectivity endpoints (spec 005 FR-003): host, path, expected status, body marker.
const ENDPOINTS: [(&str, &str, u16, Option<&str>); 2] = [
    ("connectivitycheck.gstatic.com", "/generate_204", 204, None),
    (
        "www.msftconnecttest.com",
        "/connecttest.txt",
        200,
        Some("Microsoft Connect Test"),
    ),
];
/// Fixed public IPs (Cloudflare and Google DNS) used to tell "no internet" from "no DNS".
const PUBLIC_IPS: [Ipv4Addr; 2] = [Ipv4Addr::new(1, 1, 1, 1), Ipv4Addr::new(8, 8, 8, 8)];
const HTTP_TIMEOUT: Duration = Duration::from_secs(4);
const HTTP_DEADLINE: Duration = Duration::from_secs(10);
const MAX_RESPONSE: usize = 4096;

fn fetch(host: &str, path: &str) -> Vec<u8> {
    let Some(addr) = (host, 80).to_socket_addrs().ok().and_then(|mut a| a.next()) else {
        return Vec::new();
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, HTTP_TIMEOUT) else {
        return Vec::new();
    };
    let _ = stream.set_read_timeout(Some(HTTP_TIMEOUT));
    let _ = stream.set_write_timeout(Some(HTTP_TIMEOUT));
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: HelpMyDesk\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(request.as_bytes()).is_err() {
        return Vec::new();
    }
    let mut buf = Vec::with_capacity(512);
    let _ = stream.take(MAX_RESPONSE as u64).read_to_end(&mut buf);
    buf
}

/// `net.http.check`: plain-HTTP connectivity checks; a wrong answer suggests a captive portal.
pub(crate) fn http_check() -> Result<ProbeOutput, ProbeError> {
    let (tx, rx) = mpsc::channel();
    for (host, path, status, body) in ENDPOINTS {
        let tx = tx.clone();
        thread::spawn(move || {
            let outcome = rules::classify_http(&fetch(host, path), status, body);
            let _ = tx.send(outcome);
        });
    }
    drop(tx);
    let mut outcomes = Vec::new();
    let deadline = std::time::Instant::now() + HTTP_DEADLINE;
    while outcomes.len() < ENDPOINTS.len() {
        match rx.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now())) {
            Ok(o) => outcomes.push(o),
            Err(_) => break,
        }
    }
    if outcomes.is_empty() {
        return Err(ProbeError::Timeout);
    }
    let ip_reachable = PUBLIC_IPS
        .iter()
        .any(|&ip| tcp_answers(SocketAddr::new(IpAddr::V4(ip), 443), HTTP_TIMEOUT));
    let count = |o: HttpOutcome| outcomes.iter().filter(|&&x| x == o).count() as u64;
    let mut out = ProbeOutput::new("net.http.check");
    out.evidence
        .insert("expected", Evidence::Uint(count(HttpOutcome::Expected)));
    out.evidence
        .insert("unexpected", Evidence::Uint(count(HttpOutcome::Unexpected)));
    out.evidence.insert(
        "no_response",
        Evidence::Uint(count(HttpOutcome::NoResponse)),
    );
    out.evidence
        .insert("public_ip_reachable", Evidence::Bool(ip_reachable));
    out.findings = rules::http_findings(&outcomes, ip_reachable);
    Ok(out)
}
