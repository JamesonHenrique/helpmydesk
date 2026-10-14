// SPDX-License-Identifier: GPL-3.0-or-later
//! Runs every implemented probe and prints the structured output:
//! `cargo run -p hmd-probes --example probe_all`

fn main() {
    for (id, _) in hmd_probes::PROBES {
        match hmd_probes::run(id) {
            Ok(out) => println!(
                "{id}: findings={:?}\n  evidence={:?}",
                out.findings, out.evidence
            ),
            Err(e) => println!("{id}: probe_failed ({})", e.reason_code()),
        }
    }
}
