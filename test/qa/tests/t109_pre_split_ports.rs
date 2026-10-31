//! Ticket t-109 - the thirteen pre-split hidden crates are ported to the
//! current Engine (RLR-001, RLR-002; res-164).
//!
//! The subjects are this repository's own landed crates `t-058`..`t-070`
//! under `test-hidden/`, cut against the pre-split Engine (`.arca/` root,
//! `state.toml`, the retired position key) and refusing ever since. The port is renames and
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
//! - `SVEV-009` (t-140, SVE-008): the suite's own crate runs leave
//!   `target/lanes` as it was. Fixture: this checkout's `target/lanes` as it
//!   is (absent counts as a state), listed - every path, its kind, and each
//!   file's SHA-256 - before and after a fresh run of all thirteen crates
//!   through `run_crate` in an isolated child of this test executable.
//!   Oracles: the two listings are equal; and every `Running` line of each
//!   run, `tests\hidden.rs` among them, names a test executable inside the
//!   suite's own folder, `lane_folder("t109_pre_split_ports",
//!   env!("CARGO_TARGET_TMPDIR"))`, never one under `target/lanes`.
//!
//! The crates run with one shared build directory (`target/lanes`) so the
//! thirteen builds of the Engine collapse into one; the sweep shares build
//! output the same way through `--target-dir`.
//!
//! Both checks demand the same crate runs, so one test process executes each
//! crate once and both read that observation while its inputs are unchanged
//! (WCP-002, `ratmac_qa::lane_runs`); an input change re-runs the crate.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::sync::Arc;

use ratmac_qa::{lane_runs, support};

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
/// A run is reused within this process only while the checkout still
/// matches what it saw from start to finish (`lane_runs::Observations`).
fn run_crate(root: &Path, crate_id: &str) -> Arc<Output> {
    static RUNS: lane_runs::Observations<Output> = lane_runs::Observations::new();
    let _lanes = lane_runs::exclusive();
    RUNS.observe(root, crate_id, || {
        let mut command = support::command("cargo", crate_dir(root, crate_id));
        command.args(["test", "--offline"]).stdin(Stdio::null());
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().to_uppercase().starts_with("CARGO_") {
                command.env_remove(&key);
            }
        }
        command.env("CARGO_TARGET_DIR", root.join("target").join("lanes"));
        command.output().expect("invoke cargo test in the crate")
    })
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
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
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
    let ticket_path = if ticket_path.exists() {
        ticket_path
    } else {
        root.join(".arca/ticket/archive/t-109.md")
    };
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

// --- SVEV-009 -----------------------------------------------------------------

/// The suite's own folder for its crate runs (SVE-008): the name the
/// locked-folder helper gives it inside this suite's `CARGO_TARGET_TMPDIR`.
const SUITE_FOLDER: &str = "t109_pre_split_ports";

/// SVEV-009 (t-140, SVE-008): the suite's own crate runs leave
/// `target/lanes` as it was, and Cargo puts each run's test executables in
/// the suite's own folder, never under `target/lanes`.
///
/// Cached observations: `run_crate` answers a repeat call in one process
/// from `lane_runs::Observations` without running anything, so a listing
/// around calls the two checks above already made would observe nothing.
/// The crate runs this check judges therefore happen in an isolated child of
/// this test executable (`support::isolated_test`): a fresh process in which
/// nothing ran before, so each of its thirteen `run_crate` calls is a
/// genuine run, and the child's two listings bracket exactly those runs. The
/// parent holds `lane_runs::exclusive` for the child's whole life, so no
/// other step of the parent process runs a real lane or rewrites
/// `target/lanes` meanwhile. It takes the suite's folder first and hands the
/// child its grant (`LaneFolder::grant`), so a child whose `run_crate` asks
/// the helper for that folder does not wait on its own parent.
///
/// Warm folders: once `target/lanes` is warm and every unit fresh, a red
/// crate run may change no byte there, so the listings alone could pass on
/// red code. The second oracle is red on a warm tree too: every crate run,
/// fresh or not, prints `Running <source> (<executable>)` for each test
/// executable it runs, and each one named must lie inside the suite's own
/// folder. A run that prints no such line for `tests\hidden.rs` fails too.
///
/// Lock files some run may hold - Cargo's `.cargo-lock` and
/// `.cargo-build-lock`, rustc's session locks, a lane folder's `lane.lock`:
/// any file named `*.lock` or `*-lock` - are listed by name and length and
/// never opened. The listing, the transcript reader, and the place test
/// prove themselves on a scratch folder and known shapes first, and no crate
/// runs while this process's system temp folder lies inside a Git work tree
/// (an Engine started in a project there mints Runs in that checkout). The
/// listings hash files on all the host's cores. Cost: the
/// child repeats the thirteen crate runs once per test process that runs
/// this check; the two checks above keep sharing theirs.
#[test]
fn svev_009() {
    svev_009_controls();
    let root = repo_root();
    svev_009_outside_work_trees(&root);
    let folder = ratmac_qa::baseline::lane_folder(SUITE_FOLDER, env!("CARGO_TARGET_TMPDIR"));
    // The case's name in this binary: `svev_009` in its own target, prefixed
    // by the module path inside the aggregate `lane_sweeps` binary.
    let name = match module_path!().split_once("::") {
        Some((_, module)) => format!("{module}::svev_009"),
        None => "svev_009".to_owned(),
    };
    let serial = lane_runs::exclusive();
    match support::isolated_test(&name, &[folder.grant()]) {
        Ok(true) => {
            drop(serial);
            svev_009_fresh_runs(&root, folder.path());
        }
        Ok(false) => {}
        Err(failure) => panic!(
            "SVEV-009: the suite's thirteen crate runs, run afresh in an isolated child of \
             this test executable: {failure}"
        ),
    }
}

/// In the isolated child: list `target/lanes`, run every pre-split crate
/// through `run_crate` (each a genuine run: nothing ran before in this
/// process), list it again, and judge where each run put its test
/// executables.
fn svev_009_fresh_runs(root: &Path, folder: &Path) {
    let lanes = root.join("target").join("lanes");
    let before = {
        let _lanes = lane_runs::exclusive();
        lanes_listing(&lanes)
    };
    let mut unnamed = Vec::new();
    let mut misplaced = Vec::new();
    for (crate_id, _) in PRE_SPLIT {
        let output = run_crate(root, crate_id);
        let text = combined(&output);
        let executables = test_executables(&text);
        if !executables
            .iter()
            .any(|(source, _)| source == "tests/hidden.rs")
        {
            unnamed.push(format!(
                "{crate_id}: cargo exited {:?} without a `Running tests\\hidden.rs (..)` \
                 line:\n{}",
                output.status.code(),
                tail(&text)
            ));
        }
        for (source, executable) in executables {
            // Cargo prints a path inside the crate's folder relative to it.
            let executable = crate_dir(root, crate_id).join(executable);
            if !lies_under(&executable, folder) || lies_under(&executable, &lanes) {
                misplaced.push(format!("{crate_id}: {source} ran {}", executable.display()));
            }
        }
    }
    let after = {
        let _lanes = lane_runs::exclusive();
        lanes_listing(&lanes)
    };
    let changed = listing_changes(&before, &after);
    let mut failures = unnamed;
    if !misplaced.is_empty() {
        failures.push(format!(
            "{} test executable(s) of the suite's crate runs lie outside its own folder {} \
             (or under {}):\n  {}",
            misplaced.len(),
            folder.display(),
            lanes.display(),
            misplaced.join("\n  ")
        ));
    }
    if !changed.is_empty() {
        let shown: Vec<&str> = changed.iter().take(20).map(String::as_str).collect();
        failures.push(format!(
            "the suite's thirteen crate runs change {} path(s) under {} ({} before, {} \
             after):\n  {}{}",
            changed.len(),
            lanes.display(),
            listing_state(&before),
            listing_state(&after),
            shown.join("\n  "),
            if changed.len() > shown.len() {
                format!("\n  ... and {} more", changed.len() - shown.len())
            } else {
                String::new()
            }
        ));
    }
    assert!(failures.is_empty(), "SVEV-009: {}", failures.join("\n"));
}

/// One path under a listed folder.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Listed {
    Folder,
    /// A file: its length and SHA-256.
    File(u64, String),
    /// A lock file some run may hold: its length only, never opened.
    Lock(u64),
    /// A link or any other kind: where a link points.
    Other(Option<PathBuf>),
}

/// Refuse to run the crates while this process's system temp folder - which
/// the child and every crate run inherit - lies inside a Git work tree: an
/// Engine started in a scratch project under it would mint its Runs in that
/// repository's primary checkout. The search proves itself on this checkout.
fn svev_009_outside_work_trees(root: &Path) {
    assert!(
        enclosing_work_tree(root).is_some(),
        "control: no folder at or above the checkout {} holds a `.git` entry",
        root.display()
    );
    let temp = std::env::temp_dir();
    if let Some(tree) = enclosing_work_tree(&temp) {
        panic!(
            "control: this process's system temp folder {} lies inside the Git work tree {}, \
             where an Engine a crate run starts would mint Runs; the check runs no crate. \
             Point TMP, TEMP, and TMPDIR outside every Git work tree.",
            temp.display(),
            tree.display()
        );
    }
}

/// The nearest folder at or above `folder`, as spelled or resolved, that
/// holds a `.git` entry: the work tree Git finds from inside `folder`.
fn enclosing_work_tree(folder: &Path) -> Option<PathBuf> {
    let spellings = [
        std::path::absolute(folder).ok(),
        fs::canonicalize(folder).ok(),
    ];
    spellings.into_iter().flatten().find_map(|spelled| {
        spelled
            .ancestors()
            .find(|ancestor| fs::symlink_metadata(ancestor.join(".git")).is_ok())
            .map(Path::to_path_buf)
    })
}

/// Every path under a folder - the folder itself as `.`, the rest relative
/// and `/`-separated - with what it is; empty when the folder is absent.
type Listing = std::collections::BTreeMap<String, Listed>;

/// Whether a file is one some run may hold locked - Cargo's `.cargo-lock`
/// and `.cargo-build-lock`, rustc's session locks, a lane folder's
/// `lane.lock` - and so is listed by name and length only.
fn is_lock_file(name: &str) -> bool {
    name.ends_with(".lock") || name.ends_with("-lock")
}

/// The [`Listing`] of `folder`. Links are listed, never followed.
fn lanes_listing(folder: &Path) -> Listing {
    let mut found = Listing::new();
    let mut files = Vec::new();
    let mut pending = vec![(folder.to_path_buf(), ".".to_owned())];
    while let Some((path, key)) = pending.pop() {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && key == "." => break,
            Err(error) => panic!("list {}: {error}", path.display()),
        };
        let listed = if metadata.is_dir() {
            let entries = fs::read_dir(&path)
                .unwrap_or_else(|error| panic!("list {}: {error}", path.display()));
            for entry in entries {
                let entry =
                    entry.unwrap_or_else(|error| panic!("list {}: {error}", path.display()));
                let name = entry.file_name().to_string_lossy().into_owned();
                let relative = if key == "." {
                    name
                } else {
                    format!("{key}/{name}")
                };
                pending.push((entry.path(), relative));
            }
            Listed::Folder
        } else if metadata.is_file() {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            if is_lock_file(&name) {
                Listed::Lock(metadata.len())
            } else {
                files.push((key, path, metadata.len()));
                continue;
            }
        } else {
            Listed::Other(fs::read_link(&path).ok())
        };
        found.insert(key, listed);
    }
    found.extend(hashed_files(&files));
    found
}

/// Each of `files` - key, path, length - as a listed file, hashed on all the
/// host's cores: a test build hashes about 50 MB/s per core, and a warm
/// `target/lanes` holds gigabytes.
fn hashed_files(files: &[(String, PathBuf, u64)]) -> Vec<(String, Listed)> {
    let workers = std::thread::available_parallelism()
        .map_or(1, |count| count.get())
        .min(files.len());
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    while let Some((key, path, length)) =
                        files.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
                    {
                        done.push((key.clone(), Listed::File(*length, file_sha256(path))));
                    }
                    done
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect()
    })
}

/// A file's SHA-256, read in pieces: `target/lanes` may hold gigabytes.
fn file_sha256(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file =
        fs::File::open(path).unwrap_or_else(|error| panic!("open {}: {error}", path.display()));
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => hasher.update(&buffer[..read]),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => panic!("read {}: {error}", path.display()),
        }
    }
    format!("{:x}", hasher.finalize())
}

/// What differs between two listings, one line per path, in path order.
fn listing_changes(before: &Listing, after: &Listing) -> Vec<String> {
    let paths: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    paths
        .into_iter()
        .filter_map(|path| match (before.get(path), after.get(path)) {
            (Some(was), None) => Some(format!("removed {path} ({was:?})")),
            (None, Some(now)) => Some(format!("added {path} ({now:?})")),
            (Some(was), Some(now)) if was != now => {
                Some(format!("changed {path}: {was:?} -> {now:?}"))
            }
            _ => None,
        })
        .collect()
}

/// A listing in a few words.
fn listing_state(listing: &Listing) -> String {
    if listing.is_empty() {
        "absent".to_owned()
    } else {
        format!("{} path(s)", listing.len())
    }
}

/// Every `Running <source> (<executable>)` line of a `cargo test`
/// transcript: the source, `/`-separated and without `unittests`, and the
/// executable as printed.
fn test_executables(text: &str) -> Vec<(String, PathBuf)> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim_start().strip_prefix("Running ")?;
            let (source, executable) = rest.split_once(" (")?;
            let executable = executable.trim_end().strip_suffix(')')?;
            let source = source.strip_prefix("unittests ").unwrap_or(source);
            Some((source.replace('\\', "/"), PathBuf::from(executable)))
        })
        .collect()
}

/// `path` resolved through the file system when it exists, else made
/// absolute; without Windows's verbatim marker, and lower-cased on Windows,
/// so two spellings of one place compare equal.
fn comparable(path: &Path) -> PathBuf {
    let resolved = fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_else(|_| path.to_path_buf());
    let text = resolved.to_string_lossy();
    let text = text.strip_prefix(r"\\?\").unwrap_or(&text);
    if cfg!(windows) {
        PathBuf::from(text.to_lowercase())
    } else {
        PathBuf::from(text)
    }
}

/// Whether `path` lies strictly inside `folder`, however either is spelled.
fn lies_under(path: &Path, folder: &Path) -> bool {
    let path = comparable(path);
    let folder = comparable(folder);
    path != folder && path.starts_with(&folder)
}

/// The listing, the transcript reader, and the place test prove themselves
/// on a scratch folder and known shapes, so a blind one cannot pass.
fn svev_009_controls() {
    let scratch = support::TempTree::new("t109-svev-009-control")
        .unwrap_or_else(|error| panic!("control: own a scratch tree: {error}"));
    let folder = scratch.join("folder");
    let absent = lanes_listing(&folder);
    assert!(
        absent.is_empty(),
        "control: an absent folder lists as absent, read {absent:?}"
    );
    let inner = folder.join("inner");
    fs::create_dir_all(&inner).unwrap_or_else(|error| panic!("control: {error}"));
    fs::write(inner.join("file"), b"one").unwrap_or_else(|error| panic!("control: {error}"));
    fs::write(inner.join("second"), b"two").unwrap_or_else(|error| panic!("control: {error}"));
    let filled = lanes_listing(&folder);
    let expected: Vec<&str> = vec![".", "inner", "inner/file", "inner/second"];
    assert_eq!(
        filled.keys().map(String::as_str).collect::<Vec<_>>(),
        expected,
        "control: a folder lists itself and every path below it"
    );
    assert_eq!(
        lanes_listing(&folder),
        filled,
        "control: an unchanged folder lists the same"
    );
    fs::write(inner.join("file"), b"onE").unwrap_or_else(|error| panic!("control: {error}"));
    let rewritten = lanes_listing(&folder);
    let changed = listing_changes(&filled, &rewritten);
    assert!(
        changed.len() == 1 && changed[0].starts_with("changed inner/file: "),
        "control: one changed byte in inner/file changes that path alone, read {changed:?}"
    );
    let held_path = folder.join("held.lock");
    let held = fs::File::create(&held_path).unwrap_or_else(|error| panic!("control: {error}"));
    held.lock()
        .unwrap_or_else(|error| panic!("control: lock {}: {error}", held_path.display()));
    assert_eq!(
        lanes_listing(&folder).get("held.lock"),
        Some(&Listed::Lock(0)),
        "control: a held lock file is listed by name and length, never read"
    );
    drop(held);
    let names = [
        (".cargo-lock", true),
        (".cargo-build-lock", true),
        ("s-hnq9zar0sf-0bvlp11.lock", true),
        ("lane.lock", true),
        ("hidden-50db6b4b48e0b3a4.exe", false),
        ("lib-block_buffer", false),
    ];
    let misread: Vec<&str> = names
        .iter()
        .filter(|(name, lock)| is_lock_file(name) != *lock)
        .map(|(name, _)| *name)
        .collect();
    assert!(
        misread.is_empty(),
        "control: lock file names are told from build output, misread {misread:?}"
    );

    let transcript = [
        "   Compiling ratmac-t058-hidden v0.1.0 (C:\\x)",
        "     Running unittests src\\lib.rs (C:\\a b\\out (1)\\debug\\deps\\lib-1.exe)",
        "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out",
        "     Running tests\\hidden.rs (target\\debug\\deps\\hidden-2.exe)",
        "test ht_058_01_regression ... ok",
        "   Doc-tests ratmac_t058_hidden",
    ]
    .join("\n");
    let read = test_executables(&transcript);
    let expected = vec![
        (
            "src/lib.rs".to_owned(),
            PathBuf::from("C:\\a b\\out (1)\\debug\\deps\\lib-1.exe"),
        ),
        (
            "tests/hidden.rs".to_owned(),
            PathBuf::from("target\\debug\\deps\\hidden-2.exe"),
        ),
    ];
    assert_eq!(
        read, expected,
        "control: the reader finds each Running line's source and executable"
    );

    let inside = inner.join("file");
    assert!(
        lies_under(&inside, &folder),
        "control: {} lies under {}",
        inside.display(),
        folder.display()
    );
    assert!(
        !lies_under(&folder, &folder),
        "control: a folder does not lie under itself"
    );
    let sibling = scratch.join("folder-other").join("file");
    assert!(
        !lies_under(&sibling, &folder),
        "control: {} does not lie under {}",
        sibling.display(),
        folder.display()
    );
    if cfg!(windows) {
        let respelled = PathBuf::from(format!(r"\\?\{}", inside.display()).to_uppercase());
        assert!(
            lies_under(&respelled, &folder),
            "control: {} lies under {}, however spelled",
            respelled.display(),
            folder.display()
        );
    }
}
