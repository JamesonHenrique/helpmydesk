// SPDX-License-Identifier: GPL-3.0-or-later
//! "Não consegui" report (spec 007 FR-005, Plan v1 VII.3 `report@1`): pt-BR Markdown the person
//! can read, edit and take to someone. Pure: only finding codes, verdicts and the OS summary go in;
//! probe evidence text (process names, startup apps, addresses) never does.

use std::fmt::Write as _;

use hmd_catalog::LoadedCatalog;
use hmd_domain::{ReportTrigger, Timestamp};
use hmd_policy::{PolicyContext, StepVerdict, evaluate};
use hmd_probes::{Evidence, ProbeError, ProbeOutput, Severity};

use crate::{DiagnoseOptions, DiagnosisReport, LOCALE, ProbeOutcome, confidence_phrase, proposal};

/// Contract version written in the report header.
pub const REPORT_FORMAT: &str = "report@1";

/// Maximum characters kept from free text the person typed.
pub const MAX_FREE_TEXT: usize = 4000;

/// Probe that supplies the "Sistema" section.
pub const SYSTEM_PROBE: &str = "sys.info.basic";

/// OS facts shown in the report; never host or user names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemSummary {
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub arch: Option<String>,
}

impl SystemSummary {
    pub fn from_probe(out: &ProbeOutput) -> SystemSummary {
        let text = |key: &str| match out.evidence.get(key) {
            Some(Evidence::Text(t)) => Some(t.clone()),
            _ => None,
        };
        SystemSummary {
            os_name: text("os_name"),
            os_version: text("os_version"),
            kernel_version: text("kernel_version"),
            arch: text("arch"),
        }
    }
}

/// FR-006: runs `sys.info.basic` only if the Policy Engine accepts it; `None` otherwise.
pub fn system_summary_with(
    catalog: &LoadedCatalog,
    opts: &DiagnoseOptions,
    run_probe: impl Fn(&str) -> Result<ProbeOutput, ProbeError>,
) -> Option<SystemSummary> {
    let mut ctx = PolicyContext::new(opts.platform);
    ctx.mode = opts.mode;
    ctx.consents = opts.consents.clone();
    let decision = evaluate(
        &proposal(catalog, &[SYSTEM_PROBE.to_string()], "p"),
        &catalog.catalog,
        &ctx,
    )
    .ok()?;
    match decision.verdicts.first()? {
        StepVerdict::Accepted { .. } => run_probe(SYSTEM_PROBE)
            .ok()
            .map(|out| SystemSummary::from_probe(&out)),
        _ => None,
    }
}

/// Free text from the person: control/invisible characters stripped, length limited, user and
/// computer names replaced (Plan v1 V.9). Line breaks are kept.
pub fn redact_free_text(text: &str) -> String {
    let lines: Vec<String> = text
        .lines()
        .map(|l| hmd_probes::sanitize_untrusted(l, MAX_FREE_TEXT))
        .collect();
    let joined: String = lines
        .join("\n")
        .trim()
        .chars()
        .take(MAX_FREE_TEXT)
        .collect();
    // Placeholders can be longer than the names they replace: cut again after redacting.
    hmd_probes::redact_identity(&joined)
        .chars()
        .take(MAX_FREE_TEXT)
        .collect()
}

/// Why the report exists, in words a lay person understands.
pub fn trigger_text(trigger: ReportTrigger) -> &'static str {
    match trigger {
        ReportTrigger::NoApplicableFix => {
            "As verificações terminaram sem uma correção que eu possa aplicar com segurança."
        }
        ReportTrigger::LowConfidence => "Não tenho confiança suficiente no diagnóstico.",
        ReportTrigger::VerificationFailed => {
            "A correção foi feita, mas a verificação mostrou que o problema continua."
        }
        ReportTrigger::UnknownExecutionState => {
            "Não consegui confirmar o resultado de uma correção."
        }
        ReportTrigger::OutOfScopeFinding => {
            "Encontrei algo de hardware, segurança ou disco que precisa de outra pessoa."
        }
        ReportTrigger::PersonRequested => "Você pediu um relatório para levar a alguém.",
        ReportTrigger::BlockedRequest => "O que foi pedido é algo que eu não faço, por segurança.",
        ReportTrigger::RepeatedReopen => "Esse problema voltou várias vezes em poucos dias.",
    }
}

/// Everything the report is made of.
#[derive(Debug, Clone, Copy)]
pub struct ReportInput<'a> {
    pub catalog: &'a LoadedCatalog,
    pub diagnosis: &'a DiagnosisReport,
    pub trigger: ReportTrigger,
    /// Already passed through [`redact_free_text`].
    pub symptom: Option<&'a str>,
    pub system: Option<&'a SystemSummary>,
    pub created_at: Timestamp,
}

/// `YYYY-MM-DD` (UTC) from Unix seconds, without a date library (civil-from-days algorithm).
pub fn utc_date(ts: Timestamp) -> String {
    let z = ts.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

fn severity_word(s: Severity) -> &'static str {
    match s {
        Severity::Info => "informação",
        Severity::Warning => "atenção",
        Severity::Critical => "grave",
    }
}

fn probe_title<'a>(catalog: &'a LoadedCatalog, id: &'a str) -> &'a str {
    catalog
        .meta
        .get(id)
        .and_then(|m| m.copy.get(LOCALE))
        .map_or(id, |c| c.title.as_str())
}

/// FR-005: renders the `report@1` Markdown. Pure.
pub fn render(input: &ReportInput<'_>) -> String {
    let d = input.diagnosis;
    let catalog = input.catalog;
    let family_title = catalog
        .playbook_for(d.family)
        .and_then(|p| p.title.get(LOCALE))
        .map_or(d.family.code(), String::as_str);
    let playbook_version = catalog
        .playbooks
        .get(&d.playbook_id)
        .map_or("?", |p| p.version.as_str());

    let mut s = String::new();
    // `write!` into a String cannot fail; results are ignored on purpose.
    let _ = writeln!(s, "<!-- {REPORT_FORMAT} -->");
    let _ = writeln!(s, "# Relatório do HelpMyDesk: não consegui resolver\n");
    let _ = writeln!(s, "- Data: {}", utc_date(input.created_at));
    let _ = writeln!(
        s,
        "- Motivo: {} ({})\n",
        trigger_text(input.trigger),
        input.trigger.code()
    );

    let _ = writeln!(s, "## Sintoma\n");
    let _ = writeln!(s, "- Tipo de problema: {family_title}");
    match input.symptom.map(str::trim).filter(|t| !t.is_empty()) {
        Some(text) => {
            let _ = writeln!(s, "- O que a pessoa contou:\n");
            for line in text.lines() {
                let _ = writeln!(s, "> {line}");
            }
            s.push('\n');
        }
        None => {
            let _ = writeln!(s, "- O que a pessoa contou: (não informado)\n");
        }
    }

    let _ = writeln!(s, "## O que verifiquei\n");
    if d.probes.is_empty() {
        let _ = writeln!(
            s,
            "Ainda não tenho verificações automáticas para este tipo de problema.\n"
        );
    } else {
        for probe in &d.probes {
            let title = probe_title(catalog, &probe.probe_id);
            let line = match &probe.outcome {
                ProbeOutcome::Ran(out) if out.findings.is_empty() => "nada de errado".to_string(),
                ProbeOutcome::Ran(out) => out
                    .findings
                    .iter()
                    .map(|f| format!("`{}` ({})", f.code, severity_word(f.severity)))
                    .collect::<Vec<_>>()
                    .join(", "),
                ProbeOutcome::Failed(code) => format!("não consegui verificar (`{code}`)"),
                ProbeOutcome::NotAuthorized(code) => format!("não verifiquei (`{code}`)"),
            };
            let _ = writeln!(s, "- {title}: {line}");
        }
        s.push('\n');
    }

    let _ = writeln!(s, "## Diagnóstico\n");
    let _ = writeln!(
        s,
        "{}: **{}**\n",
        confidence_phrase(d.confidence),
        d.copy.title
    );
    let _ = writeln!(s, "{}\n", d.copy.explanation);

    let _ = writeln!(s, "## O que já foi tentado\n");
    let _ = writeln!(
        s,
        "- O HelpMyDesk não alterou nada neste computador: só fez verificações de leitura."
    );
    if !d.copy.steps.is_empty() {
        let _ = writeln!(s, "- Orientações que eu dei à pessoa:");
        for (i, step) in d.copy.steps.iter().enumerate() {
            let _ = writeln!(s, "  {}. {step}", i + 1);
        }
    }
    s.push('\n');

    let _ = writeln!(s, "## Sistema\n");
    match input.system {
        Some(sys) => {
            let os = [sys.os_name.as_deref(), sys.os_version.as_deref()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ");
            let _ = writeln!(
                s,
                "- Sistema operacional: {}",
                if os.is_empty() { "(desconhecido)" } else { &os }
            );
            if let Some(k) = &sys.kernel_version {
                let _ = writeln!(s, "- Versão do núcleo/compilação: {k}");
            }
            if let Some(a) = &sys.arch {
                let _ = writeln!(s, "- Arquitetura: {a}");
            }
        }
        None => {
            let _ = writeln!(s, "- (não consegui ler os dados do sistema)");
        }
    }
    s.push('\n');

    let _ = writeln!(s, "## Para quem for ajudar\n");
    let _ = writeln!(s, "- Roteiro: `{}@{playbook_version}`", d.playbook_id);
    let _ = writeln!(s, "- Diagnóstico: `{}`", d.diagnosis_id);
    let findings = if d.findings.is_empty() {
        "(nenhum)".to_string()
    } else {
        d.findings
            .iter()
            .map(|f| format!("`{f}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let _ = writeln!(s, "- Achados: {findings}");
    let _ = writeln!(s, "- Catálogo: `{}`\n", catalog.version);

    let _ = writeln!(s, "---\n");
    let _ = writeln!(
        s,
        "Este relatório foi gerado no seu computador e não foi enviado a ninguém. Revise e apague o \
         que não quiser compartilhar antes de entregar."
    );
    s
}
