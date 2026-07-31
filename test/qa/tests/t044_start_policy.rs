//! t-043 / ORS-001: one caller policy on every active surface.
//!
//! PT-043-01 `surfaces_agree_and_audit_is_sensitive`
//! PT-043-02 `engine_gains_no_caller_state`
//! HT-043-01 `step_refusal_behavior_is_unchanged`
//! HT-043-02 `every_help_route_prints_one_usage`
//! HT-043-03 `no_active_surface_retains_retired_wording`

use ratmac::cli;
use ratmac_qa::policy::{audit_caller_policy, surface_from_file, PolicySurface};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

/// The active caller-facing surfaces in this checkout.
fn active_surfaces(root: &Path) -> Vec<PolicySurface> {
    let mut help = Vec::new();
    cli::run_from(["rtm", "start", "--help"], root, &mut help).expect("start help renders");
    vec![
        PolicySurface {
            name: "rtm start --help".to_owned(),
            text: String::from_utf8(help).expect("help is UTF-8"),
        },
        // AGENTS.md is a pointer; the audit resolves it to what it points at.
        surface_from_file(root, "AGENTS.md"),
        surface_from_file(root, ".arca/schema.md"),
    ]
}

#[test]
fn surfaces_agree_and_audit_is_sensitive() {
    let root = repo_root();

    audit_caller_policy(&active_surfaces(&root)).unwrap_or_else(|violations| {
        panic!(
            "active surfaces must state one caller policy:\n{}",
            violations
                .iter()
                .map(|violation| format!("{}: {}", violation.surface, violation.reason))
                .collect::<Vec<_>>()
                .join("\n")
        )
    });

    // Negative: a seeded copy carrying the retired wording must fail.
    let mut seeded = active_surfaces(&root);
    seeded.push(PolicySurface {
        name: "seeded-fixture/.arca/schema.md".to_owned(),
        text: format!(
            "{}\n\nStart is user-only; loop entry is never agent-initiated.\n",
            seeded[2].text
        ),
    });
    let violations =
        audit_caller_policy(&seeded).expect_err("retired user-only wording must fail the audit");
    assert!(
        violations.iter().any(
            |violation| violation.surface == "seeded-fixture/.arca/schema.md"
                && violation.reason.contains("user-only")
        ),
        "the audit names the offending surface and phrase: {violations:?}"
    );

    // Negative: a surface that drops the sign-off clause must fail.
    let incomplete = vec![PolicySurface {
        name: "seeded-fixture/help".to_owned(),
        text: "Usage: rtm start\n\nA human may invoke rtm start directly.\n".to_owned(),
    }];
    let violations =
        audit_caller_policy(&incomplete).expect_err("a partial policy statement must fail");
    assert!(
        violations
            .iter()
            .any(|violation| violation.reason.contains("sign-off")),
        "the audit names the missing clause: {violations:?}"
    );
}

#[test]
fn engine_gains_no_caller_state() {
    let root = repo_root();

    // 1. No caller identity, sign-off token, or authorization state in the Engine.
    let forbidden = [
        "caller_identity",
        "sign_off",
        "signoff",
        "authorize",
        "authorization",
        "authenticate",
        "authentication",
        "approval_file",
    ];
    let mut hits = Vec::new();
    for entry in walk_rust_sources(&root.join("src")) {
        let text = fs::read_to_string(&entry).expect("read engine source");
        for needle in forbidden {
            if text.contains(needle) {
                hits.push(format!("{}: {needle}", entry.display()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "the Engine must gain no caller identity or authorization state: {hits:?}"
    );

    // 2. The persisted State File schema is unchanged: exactly R-025's fields.
    let state_source = fs::read_to_string(root.join("src/state.rs")).expect("read state.rs");
    assert!(
        state_source.contains("const REQUIRED_FIELDS: [&str; 7]"),
        "no State File field may be added for the caller policy"
    );
    for field in [
        "phase",
        "status",
        "goal_revision",
        "input_revision",
        "output_revision",
        "active_refs",
        "blocker",
    ] {
        assert!(
            state_source.contains(&format!("\"{field}\"")),
            "state field {field} must remain"
        );
    }

    // 3. Behavior carries no caller identity: a second start on the same
    //    project behaves the same for any caller — it succeeds (FDC-006
    //    lifted the cap) and leaves the existing run byte-identical.
    let project = fixture_project("t044-policy");
    let first = run_rtm(&project, &["start"]);
    assert!(first.status.success(), "first start succeeds");
    // FDC-004: the State File resides in the minted run's directory.
    let state_path = run_state_path(&project);
    let state_before = fs::read(&state_path).expect("state after first start");
    let second = run_rtm(&project, &["start"]);
    assert!(
        second.status.success(),
        "a second start succeeds — FDC-006 enforces no active-Run cap"
    );
    assert_eq!(
        state_before,
        fs::read(&state_path).expect("state after the sibling start"),
        "the sibling start mutates nothing that already exists"
    );
    let _ = fs::remove_dir_all(&project);
}

fn walk_rust_sources(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(directory) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(walk_rust_sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
    found
}

fn fixture_project(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("ratmac-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".arca")).expect("create fixture project");
    let class =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/r007-start-policy/ratmac.toml");
    fs::copy(class, root.join(".arca/ratmac.toml")).expect("install machine class");
    root
}

fn run_rtm(project: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rtm"))
        .args(args)
        .current_dir(project)
        .output()
        .expect("invoke built rtm binary")
}

/// FDC-004: the started run's State File path, read off the plural roster.
fn run_state_path(project: &Path) -> PathBuf {
    let run_dir = fs::read_dir(project.join(".arca/runs"))
        .expect("list the runs roster")
        .map(|entry| entry.expect("roster entry is readable").path())
        .find(|path| path.is_dir())
        .expect("the started run appears on the roster");
    run_dir.join("state.toml")
}

/// FDC-004: the started run's id.
fn started_run_id(project: &Path) -> String {
    fs::read_dir(project.join(".arca/runs"))
        .expect("list the runs roster")
        .map(|entry| entry.expect("roster entry is readable"))
        .find(|entry| entry.path().is_dir())
        .expect("the started run appears on the roster")
        .file_name()
        .to_string_lossy()
        .into_owned()
}

/// HT-043-01 (Regression): wording changes must not touch behavior. A step
/// whose Exit Guard fails still refuses, reports, and leaves state unchanged.
#[test]
fn step_refusal_behavior_is_unchanged() {
    let project = fixture_project("t044-regression");
    assert!(
        run_rtm(&project, &["start"]).status.success(),
        "start succeeds"
    );
    // FDC-004: the State File resides in the minted run's directory.
    let state_path = run_state_path(&project);
    let before = fs::read(&state_path).expect("state after start");

    // A refused step reports and exits zero (R-017); only the report changes.
    let id = started_run_id(&project);
    let step = run_rtm(&project, &["step", "--run", &id]);
    let report = String::from_utf8_lossy(&step.stdout).to_ascii_lowercase()
        + &String::from_utf8_lossy(&step.stderr).to_ascii_lowercase();
    assert!(
        report.contains("step refused"),
        "a step with a failing Exit Guard must report a refusal: {report}"
    );
    assert!(
        report.contains("output.txt") || report.contains("required"),
        "the refusal must name the failing guard: {report}"
    );
    assert_eq!(
        before,
        fs::read(&state_path).expect("state after refusal"),
        "a refused step mutates no state"
    );
    let _ = fs::remove_dir_all(&project);
}

/// HT-043-02 (Input/Routing): every help route prints exactly one usage text
/// and none of them contradicts the policy.
#[test]
fn every_help_route_prints_one_usage() {
    let root = repo_root();
    for command in ["start", "status", "step", "doctor", "bogus"] {
        let mut output = Vec::new();
        cli::run_from(["rtm", command, "--help"], &root, &mut output)
            .unwrap_or_else(|error| panic!("help for {command} renders: {error}"));
        let text = String::from_utf8(output).expect("help is UTF-8");
        assert_eq!(
            text.matches("Usage:").count(),
            1,
            "help for {command} must print exactly one usage text: {text}"
        );
        let lowered = text.to_ascii_lowercase();
        for retired in ratmac_qa::policy::RETIRED_PHRASES {
            assert!(
                !lowered.contains(retired),
                "help for {command} retains retired wording {retired:?}: {text}"
            );
        }
    }
}

/// HT-043-03 (Output/Filesystem): no active surface - guidance files, engine
/// help sources, or a canonical skill present in the checkout - keeps the
/// retired wording.
#[test]
fn no_active_surface_retains_retired_wording() {
    let root = repo_root();
    let mut scanned = 0usize;
    let mut hits = Vec::new();
    let mut candidates = vec![
        root.join("AGENTS.md"),
        root.join("CLAUDE.md"),
        root.join("README.md"),
        root.join(".arca/index.md"),
        root.join(".arca/schema.md"),
        root.join("src/cli.rs"),
    ];
    candidates.extend(collect_skill_files(&root));

    for path in candidates.into_iter().filter(|path| path.exists()) {
        scanned += 1;
        let lowered = fs::read_to_string(&path)
            .expect("read active surface")
            .to_ascii_lowercase();
        for retired in ratmac_qa::policy::RETIRED_PHRASES {
            if lowered.contains(retired) {
                hits.push(format!("{}: {retired}", path.display()));
            }
        }
    }
    assert!(scanned >= 4, "the scan must cover the active surfaces");
    assert!(hits.is_empty(), "retired wording survives on: {hits:?}");
}

/// Canonical skill documents shipped inside this checkout, if any.
fn collect_skill_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for directory in [root.join("skills"), root.join(".claude/skills")] {
        collect_markdown(&directory, &mut found);
    }
    found
}

fn collect_markdown(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            found.push(path);
        }
    }
}
