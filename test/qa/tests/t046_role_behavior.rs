//! t-044 / ORS-003: behavioral role-scenario evidence.
//!
//! PT-044-01 `role_transcripts_are_behavioral`
//! PT-044-02 `violation_fails_and_wording_is_labelled`
//! HT-044-01 `malformed_transcripts_refuse`
//! HT-044-02 `every_check_carries_an_evidence_kind`
//!
//! Claims about who invoked `rtm` are proven by recorded scenarios, not by
//! documents that describe the policy.

use ratmac_qa::policy::{audit_caller_policy, surface_from_file};
use ratmac_qa::role::{
    behavioral_evidence_satisfies, check_invocation_policy, check_scenario_wording, load_dir,
    load_scenario, Caller, Check, EvidenceKind, Scenario,
};
use std::path::{Path, PathBuf};

fn scenarios_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/role-scenarios")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root resolves")
}

fn scenario(name: &str) -> Scenario {
    load_scenario(&scenarios_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|defect| panic!("{name} must load as evidence: {defect}"))
}

/// PT-044-01: the four policy-conforming transcripts satisfy their invocation
/// counts, and every check that proves it is behavioral.
#[test]
fn role_transcripts_are_behavioral() {
    let human = scenario("human-start");
    assert_eq!(human.caller, Caller::Human);
    assert_eq!(human.rtm_starts(), 1, "the human invoked start");

    let signed = scenario("main-agent-signed-off");
    assert!(signed.signoff, "the transcript records the sign-off");
    assert_eq!(
        signed.rtm_starts(),
        1,
        "a signed-off Main-Agent starts exactly once"
    );

    let unsigned = scenario("main-agent-unsigned");
    assert!(!unsigned.signoff);
    assert_eq!(
        unsigned.rtm_invocations(),
        0,
        "an unsigned Main-Agent invokes no rtm command"
    );

    let subagent = scenario("subagent");
    assert_eq!(
        subagent.rtm_invocations(),
        0,
        "a Subagent invokes no rtm command"
    );
    assert!(
        subagent.events.iter().any(|event| event
            .command
            .as_deref()
            .is_some_and(|command| command.contains("state.toml"))),
        "the Subagent scenario still reads state, which is allowed"
    );

    let checks: Vec<Check> = [&human, &signed, &unsigned, &subagent]
        .iter()
        .flat_map(|scenario| check_invocation_policy(scenario))
        .collect();
    assert_eq!(checks.len(), 4, "one behavioral check per scenario");
    for check in &checks {
        assert_eq!(
            check.kind,
            EvidenceKind::Behavioral,
            "invocation claims are behavioral: {}",
            check.render()
        );
        assert!(
            check.passed,
            "conforming scenario must pass: {}",
            check.render()
        );
    }
    assert!(
        behavioral_evidence_satisfies(&checks),
        "passing behavioral checks satisfy the invocation requirement"
    );
}

/// PT-044-02: the violating transcript fails the behavioral check, while a
/// wording check over the same scenario passes and is labelled as guidance -
/// and guidance alone can never satisfy the requirement.
#[test]
fn violation_fails_and_wording_is_labelled() {
    let violation = scenario("violation-unsigned-start");
    assert_eq!(violation.caller, Caller::MainAgent);
    assert!(!violation.signoff);

    let behavioral = check_invocation_policy(&violation);
    assert_eq!(behavioral.len(), 1);
    assert!(
        !behavioral[0].passed,
        "an unsigned Main-Agent start must fail: {}",
        behavioral[0].render()
    );
    assert!(
        behavioral[0].detail.contains("1 rtm invocation"),
        "the failure names what it observed: {}",
        behavioral[0].render()
    );
    assert!(
        !behavioral_evidence_satisfies(&behavioral),
        "a failing behavioral check cannot satisfy the requirement"
    );

    let wording = check_scenario_wording(&violation);
    assert_eq!(wording.kind, EvidenceKind::GuidanceConsistency);
    assert!(
        wording.passed,
        "the violating transcript still talks about the policy: {}",
        wording.render()
    );
    assert!(
        !behavioral_evidence_satisfies(std::slice::from_ref(&wording)),
        "guidance-consistency evidence alone never satisfies a behavioral requirement"
    );
    assert!(
        !behavioral_evidence_satisfies(&[wording, behavioral[0].clone()]),
        "a passing wording check cannot rescue a failing behavioral check"
    );
}

/// HT-044-01 (Input/Routing): a transcript a reviewer cannot trust is refused
/// by name, and produces no evidence at all.
#[test]
fn malformed_transcripts_refuse() {
    let truncated = load_scenario(&scenarios_dir().join("malformed-truncated.toml"))
        .expect_err("a truncated transcript must refuse");
    assert!(
        truncated.reason.contains("not valid TOML"),
        "the defect is named: {truncated}"
    );

    let schema = load_scenario(&scenarios_dir().join("malformed-schema.toml"))
        .expect_err("a schema-invalid transcript must refuse");
    assert!(
        schema.reason.contains("unknown caller role"),
        "the defect is named: {schema}"
    );

    let missing = load_scenario(&scenarios_dir().join("does-not-exist.toml"))
        .expect_err("an absent transcript must refuse");
    assert!(
        missing.reason.contains("unreadable"),
        "the defect is named: {missing}"
    );

    // A refused transcript is not evidence: it never reaches a check.
    let loaded = load_dir(&scenarios_dir());
    let refused: Vec<_> = loaded
        .iter()
        .filter(|(_, result)| result.is_err())
        .map(|(label, _)| label.as_str())
        .collect();
    assert_eq!(
        refused,
        vec!["malformed-schema", "malformed-truncated"],
        "exactly the malformed fixtures refuse"
    );
    let checks: Vec<Check> = loaded
        .iter()
        .filter_map(|(_, result)| result.as_ref().ok())
        .flat_map(check_invocation_policy)
        .collect();
    assert!(
        !checks
            .iter()
            .any(|check| check.id.contains("truncated") || check.id.contains("schema-invalid")),
        "a refused transcript yields no checks"
    );
}

/// HT-044-02 (Cross-Feature): behavioral and guidance-consistency evidence
/// appear in one run, and no check is ever emitted without its kind.
#[test]
fn every_check_carries_an_evidence_kind() {
    let mut report: Vec<Check> = Vec::new();
    for (_, result) in load_dir(&scenarios_dir()) {
        if let Ok(scenario) = result {
            report.extend(check_invocation_policy(&scenario));
            report.push(check_scenario_wording(&scenario));
        }
    }

    // The wording audit of the real caller-facing surfaces belongs in the same
    // run - and in the guidance-consistency column.
    let root = repo_root();
    let surfaces = [
        surface_from_file(&root, ".arca/schema.md"),
        surface_from_file(&root, "AGENTS.md"),
    ];
    let audit = audit_caller_policy(&surfaces);
    report.push(Check {
        id: "surfaces/caller-policy-wording".to_owned(),
        kind: EvidenceKind::GuidanceConsistency,
        passed: audit.is_ok(),
        detail: match &audit {
            Ok(()) => "every active surface states the one caller policy".to_owned(),
            Err(violations) => format!("{violations:?}"),
        },
    });

    assert!(
        report.len() >= 11,
        "the run reports every scenario's behavior and prose plus the surface audit: {}",
        report.len()
    );
    let kinds: Vec<EvidenceKind> = report.iter().map(|check| check.kind).collect();
    assert!(
        kinds.contains(&EvidenceKind::Behavioral)
            && kinds.contains(&EvidenceKind::GuidanceConsistency),
        "both evidence kinds are present in one run"
    );
    for check in &report {
        let line = check.render();
        let label = line.split(" | ").next().unwrap_or_default();
        assert!(
            label == "behavioral" || label == "guidance-consistency",
            "every reported check names its evidence kind first: {line}"
        );
        assert!(
            !check.id.is_empty() && line.contains(&check.id),
            "every reported check names what it checked: {line}"
        );
    }

    // The behavioral verdict ignores the guidance column entirely.
    let guidance_only: Vec<Check> = report
        .iter()
        .filter(|check| check.kind == EvidenceKind::GuidanceConsistency)
        .cloned()
        .collect();
    let passing_guidance: Vec<Check> = guidance_only
        .iter()
        .filter(|check| check.passed)
        .cloned()
        .collect();
    assert!(
        passing_guidance
            .iter()
            .any(|check| check.id == "surfaces/caller-policy-wording"),
        "the real surface audit is part of the guidance column and passes"
    );
    assert!(
        !behavioral_evidence_satisfies(&passing_guidance),
        "a passing guidance column proves nothing about invocation"
    );
    let behavioral_only: Vec<Check> = report
        .iter()
        .filter(|check| check.kind == EvidenceKind::Behavioral)
        .cloned()
        .collect();
    assert!(
        !behavioral_evidence_satisfies(&behavioral_only),
        "the violating scenario keeps the behavioral verdict honest"
    );
}
