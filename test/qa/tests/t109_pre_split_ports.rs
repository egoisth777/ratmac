//! Ticket t-109 - the thirteen pre-split hidden crates are ported to the
//! current Engine (RLR-001, RLR-002; res-164).
//!
//! The subjects are this repository's own landed crates `t-058`..`t-070`
//! under `test-hidden/`, cut against the pre-split Engine (`.arca/` root,
//! `state.toml`, `phase`) and refusing ever since. The port is renames and
//! scaffold rewrites only: every crate keeps exactly the lane names it was
//! landed with (frozen below, before the port), and every lane runs green
//! through the sweep's own per-crate command.
//!
//! - `RLRV-001`: each crate exits `0` with exactly its frozen lane names
//!   reported `ok`, and its source still declares exactly those names.
//! - `RLRV-005`: the working rules carry the triage rule as a requirement-ID
//!   heading; the ticket's class table names every one of the thirteen with a
//!   class that licenses a port and a named landing; none carries an expiry
//!   marker; and every crate the table licenses to port runs green.
//!
//! The crates run with one shared build directory (`target/lanes`) so the
//! thirteen builds of the Engine collapse into one; the sweep shares build
//! output the same way through `--target-dir`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// The thirteen pre-split crates and the lane names each was landed with -
/// frozen from the tree before the port. Seventy-six lanes: `t-061` and
/// `t-067` were cut with five.
const PRE_SPLIT: [(&str, &[&str]); 13] = [
    (
        "t-058",
        &[
            "ht_058_01_regression",
            "ht_058_02_input_routing",
            "ht_058_03_lifecycle",
            "ht_058_04_durability",
            "ht_058_05_output_filesystem",
            "ht_058_06_cross_feature",
        ],
    ),
    (
        "t-059",
        &[
            "ht_059_01_regression",
            "ht_059_02_input_routing",
            "ht_059_03_lifecycle",
            "ht_059_04_durability",
            "ht_059_05_output_filesystem",
            "ht_059_06_cross_feature",
        ],
    ),
    (
        "t-060",
        &[
            "ht_060_01_regression",
            "ht_060_02_input_routing",
            "ht_060_03_lifecycle",
            "ht_060_04_durability",
            "ht_060_05_output_filesystem",
            "ht_060_06_cross_feature",
        ],
    ),
    (
        "t-061",
        &[
            "ht_061_01_regression",
            "ht_061_02_input_routing",
            "ht_061_03_lifecycle_model",
            "ht_061_04_output_filesystem",
            "ht_061_05_cross_feature",
        ],
    ),
    (
        "t-062",
        &[
            "ht_062_01_regression",
            "ht_062_02_input_routing",
            "ht_062_03_lifecycle_model",
            "ht_062_04_durability_recovery",
            "ht_062_05_output_filesystem",
            "ht_062_06_cross_feature",
        ],
    ),
    (
        "t-063",
        &[
            "ht_063_01_regression",
            "ht_063_02_input_routing",
            "ht_063_03_lifecycle_model",
            "ht_063_04_durability_recovery",
            "ht_063_05_output_filesystem",
            "ht_063_06_cross_feature",
        ],
    ),
    (
        "t-064",
        &[
            "prior_shapes_parse_identically_and_stay_clean",
            "near_miss_keys_refuse_by_name",
            "declarations_are_inert_and_join_refuses_honestly",
            "refusal_and_diagnosis_write_nothing",
            "scaffold_unchanged_and_json_stable",
            "pinned_drift_still_refuses_after_table_append",
        ],
    ),
    (
        "t-065",
        &[
            "abandonment_shape_survives_the_phrase_change",
            "phrase_near_misses_refuse_before_any_write",
            "spawn_is_legal_only_in_the_spawning_phase_of_a_live_run",
            "interrupted_respawn_converges_on_retry",
            "spawn_writes_only_the_child_directory",
            "spawn_stays_orthogonal_to_siblings_goal_and_guards",
        ],
    ),
    (
        "t-066",
        &[
            "uncomposed_runs_never_gain_ledger_content",
            "mangled_ledger_refuses_strictly_at_read",
            "join_counts_only_live_passed_children",
            "failed_spawn_leaves_no_child_and_no_entry",
            "only_ledger_and_ordinary_run_files_exist",
            "disjoint_parent_ledgers_survive_retirement",
        ],
    ),
    (
        "t-067",
        &[
            "prior_fixtures_keep_their_exact_finding_sets",
            "overlapping_and_blocked_cycles_are_judged_exactly",
            "live_run_inside_guarded_cycle_is_unaffected",
            "failing_diagnosis_is_write_free_and_renders_stably",
            "composition_tables_do_not_move_termination_verdicts",
        ],
    ),
    (
        "t-068",
        &[
            "top_level_spawn_behavior_is_unchanged",
            "membership_addresses_the_right_parent",
            "successor_and_abandoned_child_refuse_naming_the_cap",
            "obstructed_tree_gets_the_identical_refusal_with_no_write",
            "refused_child_spawn_leaves_digests_equal",
            "cap_join_and_guard_refusals_keep_their_own_names",
        ],
    ),
    (
        "t-069",
        &[
            "human_and_child_verdicts_archive_identically",
            "child_authored_verdicts_get_no_leniency",
            "no_route_before_terminal_fact_and_never_from_abandoned",
            "archive_precedes_advance_for_child_bytes",
            "evidence_lands_only_in_the_parents_archive",
            "reviewer_flow_composes_with_cap_join_and_guards",
        ],
    ),
    (
        "t-070",
        &[
            "ht_070_01_regression_full_engine_identity",
            "ht_070_02_harness_selected_engine_path_has_full_identity",
            "ht_070_03_idle_and_active_reports_keep_lifecycle_facts",
            "ht_070_04_repeated_diagnosis_survives_fixture_recovery",
            "ht_070_05_human_format_is_full_and_fixture_is_byte_identical",
            "ht_070_06_pin_refusal_and_json_stay_independent_of_human_identity",
        ],
    ),
];

/// The two classes RLR-001 licenses to port; an `expired` class licenses a
/// marker instead and never appears in this ticket's table.
const PORT_CLASSES: [&str; 2] = ["retired contract", "fixture drift"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the harness runs from a checkout of this repository")
}

fn crate_dir(root: &Path, crate_id: &str) -> PathBuf {
    root.join("test-hidden").join(crate_id)
}

/// The lane names a crate's test source declares: every `fn` directly
/// following a `#[test]` attribute, in file order.
fn declared_lanes(root: &Path, crate_id: &str) -> Vec<String> {
    let path = crate_dir(root, crate_id).join("tests/hidden.rs");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let mut names = Vec::new();
    let mut armed = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed == "#[test]" {
            armed = true;
            continue;
        }
        if armed && trimmed.starts_with("fn ") {
            let name = trimmed[3..]
                .split(|c: char| c == '(' || c == '<' || c.is_whitespace())
                .next()
                .unwrap_or_default();
            names.push(name.to_owned());
            armed = false;
        }
    }
    names
}

/// The sweep's per-crate command, in the crate's own directory, with one
/// shared build directory under the checkout so the Engine builds once.
fn run_crate(root: &Path, crate_id: &str) -> Output {
    let mut command = Command::new("cargo");
    command
        .args(["test", "--offline"])
        .current_dir(crate_dir(root, crate_id))
        .stdin(Stdio::null());
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().to_uppercase().starts_with("CARGO_") {
            command.env_remove(&key);
        }
    }
    command.env("CARGO_TARGET_DIR", root.join("target").join("lanes"));
    command.output().expect("invoke cargo test in the crate")
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Lane names reported `ok` by a `cargo test` run.
fn green_lanes(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("test ")?;
            let (name, verdict) = rest.rsplit_once(" ... ")?;
            (verdict.trim() == "ok").then(|| name.to_owned())
        })
        .collect()
}

/// The last part of a long cargo transcript, for a readable failure.
fn tail(text: &str) -> &str {
    let start = text.len().saturating_sub(4000);
    let start = (start..text.len())
        .find(|&i| text.is_char_boundary(i))
        .unwrap_or(text.len());
    &text[start..]
}

fn frozen_set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

/// Run one crate and demand exactly its frozen lanes green; the caller
/// names the requirement the demand serves.
fn assert_crate_green(root: &Path, crate_id: &str, frozen: &[&str], why: &str) {
    let output = run_crate(root, crate_id);
    let text = combined(&output);
    assert!(
        output.status.success(),
        "{why}: {crate_id} must run green against today's Engine; cargo exited {:?}:\n{}",
        output.status.code(),
        tail(&text)
    );
    let green = green_lanes(&text);
    assert_eq!(
        green,
        frozen_set(frozen),
        "{why}: {crate_id} must report exactly its frozen lanes ok"
    );
}

// --- RLRV-001 -----------------------------------------------------------------

/// RLRV-001 (t-109, RLR-002): each of the thirteen pre-split crates still
/// declares exactly the lane names it was landed with and runs every one of
/// them green through the sweep's per-crate command.
#[test]
fn every_pre_split_crate_passes_keeping_its_lanes() {
    let root = repo_root();
    for (crate_id, frozen) in PRE_SPLIT {
        assert!(
            crate_dir(&root, crate_id).is_dir(),
            "{crate_id}: the landed crate is present under test-hidden/"
        );
        let declared = declared_lanes(&root, crate_id);
        assert_eq!(
            declared,
            frozen.to_vec(),
            "{crate_id}: the port keeps exactly the landed lane names, in order"
        );
    }
    for (crate_id, frozen) in PRE_SPLIT {
        assert_crate_green(&root, crate_id, frozen, "RLRV-001");
    }
}

// --- RLRV-005 -----------------------------------------------------------------

/// One row of the ticket's triage table: crate id, class, named landing.
fn class_rows(ticket: &str) -> Vec<(String, String, String)> {
    let mut rows = Vec::new();
    let mut in_table = false;
    for line in ticket.lines() {
        if line.starts_with("## Triage classes") {
            in_table = true;
            continue;
        }
        if in_table && line.starts_with("## ") {
            break;
        }
        if !in_table || !line.starts_with("| `t-") {
            continue;
        }
        let cells: Vec<&str> = line
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() < 4 {
            continue;
        }
        rows.push((
            cells[0].trim_matches('`').to_owned(),
            cells[1].to_owned(),
            cells[2].to_owned(),
        ));
    }
    rows
}

/// RLRV-005 (t-109, RLR-001): the triage rule is a requirement-ID heading in
/// the working rules; the ticket's class table names every one of the
/// thirteen with a port-licensing class and a named landing; no such crate
/// carries an expiry marker; and every crate the table licenses to port runs
/// green - the act the class fixes.
#[test]
fn the_class_table_licenses_a_port_and_never_an_expiry() {
    let root = repo_root();
    let schema = fs::read_to_string(root.join(".arca/schema.md")).expect("read the working rules");
    assert!(
        schema.lines().any(|line| line.starts_with("### RLR-001 ")),
        "the working rules carry the triage rule as a requirement-ID heading"
    );
    let ticket_path = root.join(".arca/ticket/t-109.md");
    let ticket = fs::read_to_string(&ticket_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", ticket_path.display()));
    let rows = class_rows(&ticket);
    for (crate_id, frozen) in PRE_SPLIT {
        let (_, class, landing) = rows
            .iter()
            .find(|(id, _, _)| id == crate_id)
            .unwrap_or_else(|| panic!("{crate_id}: the ticket's class table names this crate"));
        assert!(
            PORT_CLASSES.contains(&class.as_str()),
            "{crate_id}: the class licenses a port (retired contract or fixture drift), read {class:?}"
        );
        assert!(
            !landing.is_empty() && landing != "-",
            "{crate_id}: the row names the landing that changed the contract"
        );
        assert!(
            !crate_dir(&root, crate_id).join("EXPIRED.toml").exists(),
            "{crate_id}: a crate the table licenses to port carries no expiry marker"
        );
        assert_crate_green(&root, crate_id, frozen, "RLRV-005");
    }
}
