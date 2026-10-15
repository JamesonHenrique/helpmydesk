// SPDX-License-Identifier: GPL-3.0-or-later
//! Text "modo básico" (spec 006 FR-006): menu + diagnosis in pt-BR, no AI, works offline.
//!
//! `cargo run -p hmd-core --example basic_mode`                 → menu
//! `cargo run -p hmd-core --example basic_mode -- 1`            → diagnose family 1
//! `... -- 2 --consent-startup-apps`                              → allow the N1-S probe
//!
//! Nothing is changed on the computer: fixes are only shown with the Policy Engine decision.

use std::collections::BTreeSet;

use hmd_core::{DiagnoseOptions, LOCALE, ProbeOutcome, diagnose};
use hmd_domain::ProblemFamily;
use hmd_policy::{ApprovalRequirement, Confidence, StepVerdict};

fn confidence_phrase(c: Confidence) -> &'static str {
    match c {
        Confidence::AlmostCertain => "Tenho quase certeza",
        Confidence::Probable => "Acho provável",
        Confidence::Unsure => "Não tenho certeza",
    }
}

fn main() {
    let catalog = match hmd_catalog::embedded() {
        Ok(c) => c,
        Err(errors) => {
            eprintln!(
                "Catálogo inválido ({} erro(s)); modo somente leitura.",
                errors.len()
            );
            std::process::exit(2);
        }
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let choice = args.iter().find_map(|a| a.parse::<usize>().ok());
    let Some(family) = choice.and_then(|n| ProblemFamily::ALL.get(n.wrapping_sub(1)).copied())
    else {
        println!("O que está acontecendo? (HelpMyDesk — modo básico)\n");
        for (i, family) in ProblemFamily::ALL.iter().enumerate() {
            let title = catalog
                .playbook_for(*family)
                .and_then(|p| p.title.get(LOCALE))
                .map_or("?", String::as_str);
            println!("  {}. {title}", i + 1);
        }
        println!("\nUse: cargo run -p hmd-core --example basic_mode -- <número>");
        return;
    };

    let mut consents = BTreeSet::new();
    if args.iter().any(|a| a == "--consent-startup-apps") {
        consents.insert("startup_apps".to_string());
    }
    let opts = DiagnoseOptions {
        consents,
        ..DiagnoseOptions::default()
    };
    let report = match diagnose(family, &opts) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Não consegui fazer o diagnóstico: {e}");
            std::process::exit(1);
        }
    };

    println!("Verificando… (nada está sendo alterado)\n");
    for probe in &report.probes {
        let line = match &probe.outcome {
            ProbeOutcome::Ran(out) if out.findings.is_empty() => "✓ ok".to_string(),
            ProbeOutcome::Ran(out) => format!(
                "✗ {}",
                out.findings
                    .iter()
                    .map(|f| f.code)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            ProbeOutcome::Failed(code) => format!("? não consegui verificar ({code})"),
            ProbeOutcome::NotAuthorized(code) => format!("– pulado ({code})"),
        };
        println!("  {:<22} {line}", probe.probe_id);
    }
    println!(
        "\n{}: {}\n{}",
        confidence_phrase(report.confidence),
        report.copy.title,
        report.copy.explanation
    );
    if !report.copy.steps.is_empty() {
        println!("\nO que você pode fazer:");
        for (i, step) in report.copy.steps.iter().enumerate() {
            println!("  {}. {step}", i + 1);
        }
    }
    if let Some(decision) = &report.fix_decision {
        println!("\nO que eu posso fazer (precisa da sua aprovação; nada foi feito ainda):");
        for verdict in &decision.verdicts {
            match verdict {
                StepVerdict::Accepted {
                    capability,
                    approval,
                    requires_elevation,
                    ..
                } => {
                    let title = catalog
                        .meta
                        .get(capability)
                        .and_then(|m| m.copy.get(LOCALE))
                        .map_or(capability.as_str(), |c| c.title.as_str());
                    let ask = match approval {
                        ApprovalRequirement::None => "automático",
                        ApprovalRequirement::Plan => "pede aprovação",
                        ApprovalRequirement::PerAction => "pede aprovação para esta ação",
                    };
                    let elev = if *requires_elevation {
                        " · o sistema vai pedir sua senha"
                    } else {
                        ""
                    };
                    println!("  • {title} ({ask}{elev})");
                }
                StepVerdict::Rejected { reason, .. } | StepVerdict::Downgraded { reason, .. } => {
                    println!("  • (bloqueado pela política: {})", reason.reason_code());
                }
            }
        }
    }
    if let Some(trigger) = report.report {
        println!(
            "\nSe não resolver, posso preparar um relatório para você levar a alguém ({}).",
            trigger.code()
        );
    }
}
