//! t-085 / SVC-007: the cutover changed names only.
//!
//! SVCV-008 `the_rename_changed_names_only`
//!
//! Two halves, both mechanical. The inventory half reads every behavioral
//! suite as it stood at the freeze and requires each check to still exist and
//! still assert the same fact once the renamed vocabulary is set aside. The
//! behavior half builds the freeze Engine and runs the same scenarios through
//! both commands, requiring the same exit codes, the same reports, and the
//! same files left on disk.

use ratmac_qa::baseline::{self, scenario, Pair, Scenario};
use std::fs;
use std::path::PathBuf;

#[test]
fn the_rename_changed_names_only() {
    inventory_is_intact();
    behavior_is_unchanged();
}

/// No check was deleted, skipped, or weakened between the freeze and today.
fn inventory_is_intact() {
    let root = baseline::repo_root();
    let mut failures = Vec::new();
    for suite in baseline::BEHAVIORAL_SUITES {
        let relative = format!("test/qa/tests/{suite}.rs");
        let before = baseline::freeze_file(&root, &relative);
        let after = fs::read_to_string(root.join(&relative))
            .unwrap_or_else(|error| panic!("SVCV-008: {relative} must still exist: {error}"));
        failures.extend(baseline::inventory_differences(&relative, &before, &after));
    }

    assert!(
        failures.is_empty(),
        "SVCV-008: the cutover may only change names; {} check(s) changed meaning:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The same scenarios, through the freeze Engine and today's, produce the
/// same exit codes, the same reports, and the same tree.
fn behavior_is_unchanged() {
    // The harness's own copy of the Engine, rebuilt by every `cargo test`
    // run, so the "today" side can never be a stale binary.
    let today_engine = PathBuf::from(ratmac_qa::engine_bin!());
    let pair = Pair::new(
        "t085",
        &today_engine,
        baseline::DEFAULT_RUNBOOK,
        &[("done/done.txt", "done\n")],
    );

    let scenarios: Vec<Scenario> = vec![
        scenario("doctor before any Run", &["doctor"]),
        scenario("status before any Run", &["status"]),
        scenario("start", &["start"]),
        scenario("status after start", &["status"]),
        scenario("step out of intake", &["step", "--run", "run-001"]),
        scenario("step with the guard met", &["step", "--run", "run-001"]),
        scenario("step past the last phase", &["step", "--run", "run-001"]),
        scenario("step against an unknown Run", &["step", "--run", "run-404"]),
        scenario(
            "hold without the confirmation",
            &["hold", "t-900", "--blocker", ".arca/issue/i-777-blocker"],
        ),
        scenario(
            "abandon without the confirmation",
            &["abandon", "--run", "run-001"],
        ),
        scenario(
            "abandon with the confirmation",
            &[
                "abandon",
                "--run",
                "run-001",
                "--confirm",
                "abandon run-001",
            ],
        ),
        scenario("status after abandoning", &["status"]),
        scenario("doctor after abandoning", &["doctor"]),
    ];

    let mut differences = Vec::new();
    let mut succeeded = 0usize;
    for scenario in &scenarios {
        let comparison = pair.compare(scenario);
        if comparison.freeze_code == Some(0) {
            succeeded += 1;
        }
        differences.extend(comparison.differences);
    }

    // A comparison in which every command refused would match trivially, so
    // the scenario set has to move the machine as well as refuse.
    assert!(
        succeeded >= 4,
        "SVCV-008: only {succeeded} of {} scenarios succeeded at the freeze; the comparison must exercise motion, not only refusals",
        scenarios.len()
    );

    differences.extend(pair.tree_differences());

    assert!(
        differences.is_empty(),
        "SVCV-008: {} behavior difference(s) between the freeze Engine and today's:\n{}",
        differences.len(),
        differences.join("\n")
    );
}
