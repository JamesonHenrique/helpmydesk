// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 002 acceptance tests. Test names cite requirement IDs.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use hmd_policy::*;

fn variant(level: RiskLevel) -> PlatformVariant {
    PlatformVariant {
        risk_level: level,
        requires_elevation: false,
        data_loss: DataLoss::None,
        disruption: Disruption::None,
        persistent_change: Scope::None,
        rollback: Rollback::NotNeeded,
        verification: None,
        preconditions: Preconditions::default(),
        params: vec![],
        max_per_session: 1,
        consent_category: None,
        auto_eligible: false,
    }
}

fn cap(id: &str, kind: CapabilityKind, variants: &[(Platform, PlatformVariant)]) -> Capability {
    Capability {
        id: id.into(),
        version: "1.0.0".into(),
        kind,
        family: None,
        variants: variants.iter().cloned().collect::<BTreeMap<_, _>>(),
        exclusive_with: vec![],
    }
}

fn both(v: PlatformVariant) -> Vec<(Platform, PlatformVariant)> {
    vec![(Platform::Windows, v.clone()), (Platform::Linux, v)]
}

fn catalog() -> Catalog {
    let mut dns = variant(RiskLevel::N2);
    dns.verification = Some("net.dns.resolve".into());
    dns.preconditions = Preconditions {
        all_of: vec!["net.dns.resolution_failing".into()],
        none_of: vec!["net.gateway.unreachable".into()],
    };

    let mut adapter = variant(RiskLevel::N3);
    adapter.requires_elevation = true;
    adapter.disruption = Disruption::Noticeable;
    adapter.verification = Some("net.gateway.reach".into());

    let mut dhcp = adapter.clone();
    dhcp.verification = Some("net.ip.config".into());

    let mut startup = variant(RiskLevel::N2);
    startup.persistent_change = Scope::User;
    startup.rollback = Rollback::Implemented;
    startup.verification = Some("startup.apps.list".into());
    startup.params = vec!["entry".into()];
    startup.max_per_session = 3;
    startup.auto_eligible = true;

    let mut installed = variant(RiskLevel::N1S);
    installed.consent_category = Some("installed_apps".into());
    installed.max_per_session = 5;

    let mut guide = variant(RiskLevel::N0);
    guide.max_per_session = 5;

    let mut adapter_cap = cap(
        "net.adapter.restart",
        CapabilityKind::Fix,
        &[(Platform::Windows, adapter)],
    );
    adapter_cap.exclusive_with = vec!["net.dhcp.renew".into()];

    Catalog::new([
        cap("net.dns.flush_cache", CapabilityKind::Fix, &both(dns)),
        adapter_cap,
        cap(
            "net.dhcp.renew",
            CapabilityKind::Fix,
            &[(Platform::Windows, dhcp)],
        ),
        cap("startup.app.disable", CapabilityKind::Fix, &both(startup)),
        cap(
            "apps.installed.list",
            CapabilityKind::Probe,
            &both(installed),
        ),
        cap("settings.open", CapabilityKind::Guide, &both(guide)),
        cap(
            "security.av.disable",
            CapabilityKind::Fix,
            &both(variant(RiskLevel::N4)),
        ),
    ])
}

fn step(id: &str, capability: &str) -> ProposedStep {
    ProposedStep {
        step_id: id.into(),
        capability: capability.into(),
        version: "1.0.0".into(),
        params: vec![],
    }
}

fn plan(steps: Vec<ProposedStep>) -> ActionPlanProposal {
    ActionPlanProposal {
        session_id: "s1".into(),
        steps,
    }
}

fn ctx(platform: Platform) -> PolicyContext {
    let mut c = PolicyContext::new(platform);
    c.findings.insert("net.dns.resolution_failing".into());
    c.confidence = Confidence::Probable;
    c
}

fn only(decision: PolicyDecision) -> StepVerdict {
    assert_eq!(decision.verdicts.len(), 1);
    decision.verdicts.into_iter().next().unwrap()
}

fn reason(v: &StepVerdict) -> &'static str {
    match v {
        StepVerdict::Rejected { reason, .. } | StepVerdict::Downgraded { reason, .. } => {
            reason.reason_code()
        }
        StepVerdict::Accepted { .. } => "ACCEPTED",
    }
}

fn eval_one(p: ActionPlanProposal, c: &PolicyContext) -> StepVerdict {
    only(evaluate(&p, &catalog(), c).unwrap())
}

#[test]
fn catalog_fixture_is_valid() {
    assert_eq!(validate_catalog(&catalog()), vec![]);
}

#[test]
fn fr001_unknown_capability() {
    let v = eval_one(plan(vec![step("s1", "shell.exec")]), &ctx(Platform::Linux));
    assert_eq!(reason(&v), "UNKNOWN_CAPABILITY");
}

#[test]
fn fr001_version_mismatch() {
    let mut s = step("s1", "net.dns.flush_cache");
    s.version = "9.9.9".into();
    assert_eq!(
        reason(&eval_one(plan(vec![s]), &ctx(Platform::Linux))),
        "VERSION_MISMATCH"
    );
}

#[test]
fn fr002_platform_unsupported() {
    let v = eval_one(
        plan(vec![step("s1", "net.adapter.restart")]),
        &ctx(Platform::Linux),
    );
    assert_eq!(reason(&v), "PLATFORM_UNSUPPORTED");
}

#[test]
fn fr002_kill_switch_disables() {
    let mut c = ctx(Platform::Windows);
    c.disabled.insert("net.dns.flush_cache".into());
    let v = eval_one(plan(vec![step("s1", "net.dns.flush_cache")]), &c);
    assert_eq!(reason(&v), "CAPABILITY_DISABLED");
}

#[test]
fn fr002_n4_is_blocked_in_every_mode() {
    for mode in [
        AutonomyMode::ObserveOnly,
        AutonomyMode::AskAlways,
        AutonomyMode::AutoSimple,
    ] {
        let mut c = ctx(Platform::Windows);
        c.mode = mode;
        let v = eval_one(plan(vec![step("s1", "security.av.disable")]), &c);
        assert_eq!(reason(&v), "BLOCKED_CAPABILITY");
    }
}

#[test]
fn fr003_accepted_fields_come_from_catalog() {
    let v = eval_one(
        plan(vec![step("s1", "net.adapter.restart")]),
        &ctx(Platform::Windows),
    );
    match v {
        StepVerdict::Accepted {
            risk_level,
            approval,
            requires_elevation,
            verification,
            ..
        } => {
            assert_eq!(risk_level, RiskLevel::N3);
            assert_eq!(approval, ApprovalRequirement::PerAction);
            assert!(requires_elevation);
            assert_eq!(verification.as_deref(), Some("net.gateway.reach"));
        }
        other => panic!("expected accepted, got {other:?}"),
    }
}

#[test]
fn fr004_preconditions_from_findings_only() {
    let p = plan(vec![step("s1", "net.dns.flush_cache")]);
    assert!(eval_one(p.clone(), &ctx(Platform::Linux)).is_accepted());

    let mut c = ctx(Platform::Linux);
    c.findings.insert("net.gateway.unreachable".into());
    assert_eq!(reason(&eval_one(p.clone(), &c)), "PRECONDITION_UNMET");

    let mut c = ctx(Platform::Linux);
    c.findings.clear();
    assert_eq!(reason(&eval_one(p, &c)), "PRECONDITION_UNMET");
}

#[test]
fn fr005_invalid_params() {
    let c = ctx(Platform::Linux);
    let mut s = step("s1", "startup.app.disable");
    s.params = vec![("entry".into(), "Spotify".into())];
    assert!(eval_one(plan(vec![s.clone()]), &c).is_accepted());

    s.params = vec![("command".into(), "rm -rf /".into())];
    assert_eq!(
        reason(&eval_one(plan(vec![s.clone()]), &c)),
        "INVALID_PARAMS"
    );

    s.params = vec![("entry".into(), "x".repeat(c.max_param_len + 1))];
    assert_eq!(
        reason(&eval_one(plan(vec![s.clone()]), &c)),
        "INVALID_PARAMS"
    );

    s.params = vec![("entry".into(), "a".into()), ("entry".into(), "b".into())];
    assert_eq!(reason(&eval_one(plan(vec![s]), &c)), "INVALID_PARAMS");
}

#[test]
fn fr006_observe_only_downgrades_fixes() {
    let mut c = ctx(Platform::Windows);
    c.mode = AutonomyMode::ObserveOnly;
    for id in ["net.dns.flush_cache", "net.adapter.restart"] {
        let v = eval_one(plan(vec![step("s1", id)]), &c);
        assert!(matches!(v, StepVerdict::Downgraded { .. }), "{id}");
        assert_eq!(reason(&v), "LEVEL_EXCEEDS_MODE");
    }
}

#[test]
fn fr006_auto_simple_respects_auto_eligible() {
    let mut c = ctx(Platform::Linux);
    c.mode = AutonomyMode::AutoSimple;
    let mut s = step("s1", "startup.app.disable");
    s.params = vec![("entry".into(), "App".into())];
    let auto = eval_one(plan(vec![s]), &c);
    let not_auto = eval_one(plan(vec![step("s1", "net.dns.flush_cache")]), &c);
    assert!(matches!(
        auto,
        StepVerdict::Accepted {
            approval: ApprovalRequirement::None,
            ..
        }
    ));
    assert!(matches!(
        not_auto,
        StepVerdict::Accepted {
            approval: ApprovalRequirement::Plan,
            ..
        }
    ));
}

#[test]
fn fr007_n1s_requires_consent() {
    let mut c = ctx(Platform::Linux);
    let p = plan(vec![step("s1", "apps.installed.list")]);
    assert_eq!(reason(&eval_one(p.clone(), &c)), "CONSENT_REQUIRED");
    c.consents.insert("installed_apps".into());
    assert!(eval_one(p, &c).is_accepted());
}

#[test]
fn fr008_rate_limit_per_capability_and_within_plan() {
    let mut c = ctx(Platform::Linux);
    c.counters
        .executions
        .insert("net.dns.flush_cache".into(), 1);
    let v = eval_one(plan(vec![step("s1", "net.dns.flush_cache")]), &c);
    assert_eq!(reason(&v), "RATE_LIMIT");

    let d = evaluate(
        &plan(vec![
            step("s1", "net.dns.flush_cache"),
            step("s2", "net.dns.flush_cache"),
        ]),
        &catalog(),
        &ctx(Platform::Linux),
    )
    .unwrap();
    assert!(d.verdicts[0].is_accepted());
    assert_eq!(reason(&d.verdicts[1]), "RATE_LIMIT");
}

#[test]
fn fr008_rate_limit_total_fixes() {
    let mut c = ctx(Platform::Linux);
    c.counters.total_fixes = c.max_fixes_per_session;
    let v = eval_one(plan(vec![step("s1", "net.dns.flush_cache")]), &c);
    assert_eq!(reason(&v), "RATE_LIMIT");
    // Guides are not fixes and are not limited by the fix total.
    assert!(eval_one(plan(vec![step("s1", "settings.open")]), &c).is_accepted());
}

#[test]
fn fr008_pending_unknown_execution() {
    let mut c = ctx(Platform::Linux);
    c.counters
        .unknown_executions
        .insert("net.dns.flush_cache".into());
    let v = eval_one(plan(vec![step("s1", "net.dns.flush_cache")]), &c);
    assert_eq!(reason(&v), "PENDING_UNKNOWN_EXECUTION");
}

#[test]
fn fr008_exclusive_conflict_both_directions() {
    let c = ctx(Platform::Windows);
    for (a, b) in [
        ("net.adapter.restart", "net.dhcp.renew"),
        ("net.dhcp.renew", "net.adapter.restart"),
    ] {
        let d = evaluate(&plan(vec![step("s1", a), step("s2", b)]), &catalog(), &c).unwrap();
        assert!(d.verdicts[0].is_accepted());
        assert_eq!(reason(&d.verdicts[1]), "EXCLUSIVE_CONFLICT", "{a} then {b}");
    }
}

#[test]
fn fr009_n3_requires_probable_confidence() {
    let mut c = ctx(Platform::Windows);
    c.confidence = Confidence::Unsure;
    let v = eval_one(plan(vec![step("s1", "net.adapter.restart")]), &c);
    assert_eq!(reason(&v), "LOW_CONFIDENCE");
    // N2 is not gated by confidence.
    assert!(eval_one(plan(vec![step("s1", "net.dns.flush_cache")]), &c).is_accepted());
}

#[test]
fn fr010_plan_size_and_duplicates() {
    let c = ctx(Platform::Linux);
    assert_eq!(
        evaluate(&plan(vec![]), &catalog(), &c)
            .unwrap_err()
            .reason_code(),
        "EMPTY_PLAN"
    );
    let many = (0..=MAX_STEPS)
        .map(|i| step(&format!("s{i}"), "settings.open"))
        .collect();
    assert_eq!(
        evaluate(&plan(many), &catalog(), &c)
            .unwrap_err()
            .reason_code(),
        "TOO_MANY_STEPS"
    );
    let d = evaluate(
        &plan(vec![
            step("s1", "settings.open"),
            step("s1", "settings.open"),
        ]),
        &catalog(),
        &c,
    )
    .unwrap();
    assert_eq!(reason(&d.verdicts[1]), "DUPLICATE_STEP");
}

#[test]
fn fr011_min_level_and_catalog_validation() {
    let mut v = variant(RiskLevel::N2);
    v.verification = Some("p".into());
    v.requires_elevation = true;
    assert_eq!(min_level(CapabilityKind::Fix, &v), RiskLevel::N3);
    let bad_level = Catalog::new([cap("x", CapabilityKind::Fix, &both(v))]);
    let errs = validate_catalog(&bad_level);
    assert!(
        errs.iter()
            .all(|e| e.reason_code() == "LEVEL_BELOW_MINIMUM")
    );
    assert_eq!(errs.len(), 2);

    let no_verification =
        Catalog::new([cap("y", CapabilityKind::Fix, &both(variant(RiskLevel::N2)))]);
    assert!(
        validate_catalog(&no_verification)
            .iter()
            .any(|e| e.reason_code() == "FIX_WITHOUT_VERIFICATION")
    );

    let mut data = variant(RiskLevel::N2);
    data.data_loss = DataLoss::Some;
    assert_eq!(min_level(CapabilityKind::Fix, &data), RiskLevel::N3);
    assert_eq!(
        min_level(CapabilityKind::Guide, &variant(RiskLevel::N0)),
        RiskLevel::N0
    );
    assert_eq!(
        min_level(CapabilityKind::Probe, &variant(RiskLevel::N1A)),
        RiskLevel::N1A
    );
}

#[test]
fn fr011_capability_without_variants_is_invalid() {
    let empty = Catalog::new([cap("z", CapabilityKind::Probe, &[])]);
    assert_eq!(validate_catalog(&empty)[0].reason_code(), "NO_VARIANTS");
}

/// SC-002: across every mode × level × kind, nothing above N1 runs in ObserveOnly, N3 always
/// needs per-action approval, and N2 runs without approval only when auto-eligible in AutoSimple.
#[test]
fn sc002_exhaustive_mode_level_kind() {
    let modes = [
        AutonomyMode::ObserveOnly,
        AutonomyMode::AskAlways,
        AutonomyMode::AutoSimple,
    ];
    let kinds = [
        CapabilityKind::Probe,
        CapabilityKind::Fix,
        CapabilityKind::Guide,
    ];
    for mode in modes {
        for level in RiskLevel::ALL {
            for kind in kinds {
                for auto_eligible in [false, true] {
                    let mut v = variant(level);
                    v.verification = Some("probe".into());
                    v.auto_eligible = auto_eligible;
                    v.consent_category = Some("cat".into());
                    let cat = Catalog::new([cap("c", kind, &both(v))]);
                    let mut c = ctx(Platform::Linux);
                    c.mode = mode;
                    c.consents.insert("cat".into());
                    c.confidence = Confidence::AlmostCertain;
                    let verdict = only(evaluate(&plan(vec![step("s1", "c")]), &cat, &c).unwrap());
                    let label = format!("{mode:?} {level:?} {kind:?} auto={auto_eligible}");
                    match (level, &verdict) {
                        (RiskLevel::N4, v) => {
                            assert_eq!(reason(v), "BLOCKED_CAPABILITY", "{label}")
                        }
                        (RiskLevel::N2 | RiskLevel::N3, StepVerdict::Accepted { approval, .. }) => {
                            assert_ne!(mode, AutonomyMode::ObserveOnly, "{label}");
                            let expected = match (level, mode, auto_eligible) {
                                (RiskLevel::N3, _, _) => ApprovalRequirement::PerAction,
                                (_, AutonomyMode::AutoSimple, true) => ApprovalRequirement::None,
                                _ => ApprovalRequirement::Plan,
                            };
                            assert_eq!(*approval, expected, "{label}");
                        }
                        (RiskLevel::N2 | RiskLevel::N3, v) => {
                            assert_eq!(mode, AutonomyMode::ObserveOnly, "{label}");
                            assert_eq!(reason(v), "LEVEL_EXCEEDS_MODE", "{label}");
                        }
                        (_, StepVerdict::Accepted { approval, .. }) => {
                            assert_eq!(*approval, ApprovalRequirement::None, "{label}");
                        }
                        (_, v) => panic!("{label}: unexpected {v:?}"),
                    }
                }
            }
        }
    }
}

#[test]
fn nfr001_deterministic() {
    let p = plan(vec![
        step("s1", "net.dns.flush_cache"),
        step("s2", "net.adapter.restart"),
        step("s3", "shell.exec"),
    ]);
    let c = ctx(Platform::Windows);
    assert_eq!(
        evaluate(&p, &catalog(), &c).unwrap(),
        evaluate(&p, &catalog(), &c).unwrap()
    );
}
