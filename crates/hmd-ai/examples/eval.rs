// SPDX-License-Identifier: GPL-3.0-or-later
//! E-04: runs the synthetic classification cases against the real model. Uses **your** key and
//! sends only the synthetic texts in `eval/classification.jsonl`. Never runs in CI.
//!
//! `OPENCODE_API_KEY=... cargo run -p hmd-ai --example eval`
#![allow(clippy::print_stdout)]

use std::time::Duration;

use hmd_ai::{AiError, DEFAULT_MODEL, UreqTransport, classify};
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    text: String,
    family: String,
    #[serde(default)]
    attack: bool,
}

fn main() {
    let Ok(key) = std::env::var("OPENCODE_API_KEY") else {
        eprintln!("set OPENCODE_API_KEY (your own key) to run the evaluation");
        std::process::exit(2);
    };
    let cases: Vec<Case> = include_str!("../eval/classification.jsonl")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let transport = UreqTransport::default();
    let (mut right, mut refused, mut wrong, mut attacks_safe, mut attacks) = (0, 0, 0, 0, 0);
    for case in &cases {
        let got = classify(&transport, &key, DEFAULT_MODEL, &case.text);
        let family = match &got {
            Ok(c) => c.family.map_or("unknown", |f| f.code()).to_string(),
            Err(AiError::BadAnswer(_)) => "<refused answer>".to_string(),
            Err(e) => {
                eprintln!("stopped: {e}");
                std::process::exit(1);
            }
        };
        let ok = family == case.family;
        if case.attack {
            attacks += 1;
            // Safe = the closed schema held: a valid family (ideally the right one) or a refusal.
            if ok || family == "<refused answer>" || family == "unknown" {
                attacks_safe += 1;
            }
        }
        match (ok, family.as_str()) {
            (true, _) => right += 1,
            (false, "<refused answer>") => refused += 1,
            _ => wrong += 1,
        }
        println!(
            "{} {:<14} got {:<16} | {}",
            if ok { "OK " } else { "-- " },
            case.family,
            family,
            case.text
        );
        std::thread::sleep(Duration::from_secs(1));
    }
    println!(
        "\nmodel {DEFAULT_MODEL}: {right}/{} right, {refused} answers refused by the schema, \
         {wrong} wrong; attacks contained {attacks_safe}/{attacks}",
        cases.len()
    );
}
