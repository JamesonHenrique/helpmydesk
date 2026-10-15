// SPDX-License-Identifier: GPL-3.0-or-later
//! Text "modo básico" (spec 006 FR-006): menu + diagnosis in pt-BR, no AI, works offline.
//!
//! `cargo run -p hmd-core --example basic_mode`                 → menu
//! `cargo run -p hmd-core --example basic_mode -- 1`            → diagnose family 1
//! `... -- 2 --consent-startup-apps`                              → allow the N1-S probe
//! `... -- 1 --relatorio "a internet caiu"`                       → also print the report (spec 007)
//! `... -- --o-que-eu-olho`                                       → what each check reads (spec 009)
//!
//! Nothing is changed on the computer: fixes are only shown with the Policy Engine decision.

use std::collections::BTreeSet;

use hmd_core::report::{ReportInput, redact_free_text, render, system_summary_with};
use hmd_core::{DiagnoseOptions, LOCALE, ProbeOutcome, confidence_phrase, diagnose};
use hmd_domain::{ProblemFamily, ReportTrigger};
use hmd_policy::{ApprovalRequirement, StepVerdict};

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
    if args.iter().any(|a| a == "--o-que-eu-olho") {
        println!("O que eu olho no seu computador (nada é alterado):\n");
        for d in catalog.disclosures(hmd_core::current_platform()) {
            println!("• {}", d.title);
            println!("  Olho: {}", d.looks_at);
            println!("  Não guardo nem envio: {}", d.never);
            if let Some(category) = d.consent {
                let title = catalog
                    .consents
                    .get(category)
                    .and_then(|c| c.copy.get(LOCALE))
                    .map_or(category, |c| c.title.as_str());
                println!("  Só com a sua permissão: {title}");
            }
            println!();
        }
        return;
    }
    let report_at = args.iter().position(|a| a == "--relatorio");
    let symptom = report_at
        .and_then(|i| args.get(i + 1))
        .filter(|a| !a.starts_with("--"))
        .map(|a| redact_free_text(a));
    let choice = args
        .iter()
        .enumerate()
        .filter(|(i, _)| report_at.is_none_or(|r| *i != r + 1))
        .find_map(|(_, a)| a.parse::<usize>().ok());
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
    if report_at.is_some() {
        let system = system_summary_with(catalog, &opts, hmd_probes::run);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
        let text = render(&ReportInput {
            catalog,
            diagnosis: &report,
            // REP-06: the person asked, even when the diagnosis has no trigger of its own.
            trigger: report.report.unwrap_or(ReportTrigger::PersonRequested),
            symptom: symptom.as_deref(),
            system: system.as_ref(),
            created_at: now,
        });
        println!("\n----- relatório (você pode editar antes de entregar) -----\n{text}");
    } else if let Some(trigger) = report.report {
        println!(
            "\nSe não resolver, posso preparar um relatório para você levar a alguém ({}). \
             Use --relatorio \"o que aconteceu\".",
            trigger.code()
        );
    }
}
