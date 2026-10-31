//! t-140 / SVE-008: run-private freeze and ports folders.
//!
//! SVEV-009 `svev_009` (freeze and sweep folders stay private), lane half.
//! The suite half - the pre-split ports suite's own crate runs leave
//! `target/lanes` as it was - is `svev_009` in `t109_pre_split_ports.rs`.
//!
//! The check runs two real lanes, each once, the way `tools/sweep_lanes.py`
//! runs a crate: `cargo test --offline --target-dir <crate>/target` in the
//! crate's folder, with every inherited `CARGO_` variable dropped. A lane's
//! standalone verdict is exit 0 with every test it reports `ok`, its six
//! tests (named below, from the lane's source) among them. A run still going
//! after 30 minutes is stopped and reads as timed out.
//!
//! In place, not in a scratch copy: the lanes run in this checkout's own
//! `test-hidden/<lane>` folders. The second part's oracle is the
//! repository's own `target/lanes`, which a copy could only stand in for.
//! Neither lane edits a tracked file: each writes its own `target` folder
//! and scratch trees under the system temp folder (and, today, the pre-split
//! ports lane writes `target/lanes`, which is what the check catches). The
//! private proof agent never runs lanes in place here, so nothing else
//! drives these lanes in this checkout meanwhile; a second run of the same
//! lane would wait on that lane's own lock (SVE-007) rather than share its
//! folder. A copy would also cost a full copy and cold builds of both lanes
//! in every process.
//!
//! The freeze folder. Fixture: a scratch tree this check owns, holding one
//! short folder made fresh for the run, `t`, which holds nothing but a plain
//! file named `ratmac-freeze-<FREEZE_COMMIT>` with bytes the check wrote.
//! One run of the rename lane (`t-085`) gets `TMP`, `TEMP`, and `TMPDIR`
//! naming `t`, so `t` is its system temp folder: Rust's `std::env::temp_dir()`
//! reads `TMP`, then `TEMP`, on Windows and `TMPDIR` elsewhere. Oracle: that
//! run gives the lane's standalone verdict (`ht_085_01`..`ht_085_06` all
//! `ok`), and the plain file is still a plain file holding exactly those
//! bytes. Red today: the shared freeze build tries to create
//! `<t>/ratmac-freeze-<commit>/tree`, so the first test that asks for the
//! freeze Engine fails with "the freeze Engine builds offline" and every
//! later one finds that once-per-process build poisoned. The folder is fresh
//! and holds nothing warm, so no earlier build can hide that.
//!
//! The ports sweep folder. Fixture: this checkout's `target/lanes` as it is,
//! absent included. Oracle: a listing of every path under it - relative
//! path, kind (folder, file, other), and each file's SHA-256, hashed
//! streaming, files spread over the host's cores (a test build hashes about
//! 50 MB/s per core, and `target/lanes` holds 1.6 GB here when warm) - taken
//! before one run of the pre-split ports lane (`t-109`)
//! equals the one taken after it, and that run gives the lane's standalone
//! verdict (`ht_109_01`..`ht_109_06` all `ok`). Absent counts as a state:
//! absent before and present after is a change. A lock file some run may
//! hold - Cargo's own (`.cargo-lock`, `.cargo-build-lock`), rustc's session
//! locks, a lane folder's `lane.lock`: any file named `*.lock` or `*-lock` -
//! counts by name and length only and is never opened. Red today: HT-109-06's
//! nested sweep builds two copied crates with `--target-dir
//! <repo>/target/lanes`. Each run copies the two crates afresh, to a new path
//! and with one source file rewritten, so Cargo finds their units stale and
//! rebuilds them there even when `target/lanes` is warm and every other unit
//! is fresh, rewriting their files and adding new incremental session
//! folders: a red run always changes the listing (seen on this host: 365
//! paths changed on a warm folder of 3548), and the comparison alone is
//! sensitive for this lane.
//!
//! Controls: before any lane runs, the recorder proves itself on a scratch
//! folder - absent, empty, and filled folders record apart; an unchanged
//! folder records the same; one changed byte, an added file, or a file
//! turned into a folder each change the listing; and a lock file held open
//! and locked is recorded without being read - and the verdict reader proves
//! itself on known transcripts. The freeze folder fixture proves itself
//! too: once the plain file is in place, an isolated child of this test
//! executable, given the same `TMP`, `TEMP`, and `TMPDIR` as the lane's run,
//! must find `t` as its system temp folder, and leaves nothing in it. A green
//! run therefore cannot come from a lane that never met the plain file.
//! And no lane runs while this process's system temp folder lies inside a
//! Git work tree (a `.git` entry at or above it; the search must first find
//! this checkout's): both lanes put their scratch projects under it, and an
//! Engine started in a project inside a work tree mints its Runs in that
//! repository's primary checkout.
//! No lane control run is needed: each part
//! demands the lane's standalone verdict itself, so a passing run is its own
//! positive control, and a red run reports the tests that failed and their
//! distinct panic lines, which tell a broken lane apart from the missing
//! behavior.
//!
//! Exclusion and cost: the check holds `lane_runs::exclusive` from before
//! the first run to after the last listing, so no suite step in the same
//! test process runs a real lane or rewrites `target/lanes` in between. Each
//! lane runs exactly once, nothing is reused from an earlier observation
//! (`lane_runs::Observations` answers repeat calls without running anything,
//! so the check never goes through it), and each lane keeps its own `target`
//! warm between processes, as the sweep's command does.
//!
//! Hole-poke notes:
//! - A freeze build that still names `ratmac-freeze-<commit>` under the
//!   system temp folder meets the plain file and fails the lane; one that
//!   removes or rewrites the plain file to get past it changes its bytes.
//! - A lane run that ignores `TMP`, `TEMP`, and `TMPDIR` never meets the plain
//!   file. Rust's standard library reads them, and today's red run shows the
//!   lane stopping at that very file; a lane that took its temp folder some
//!   other way is beyond this check.
//! - A lane that writes `target/lanes` and puts every byte back before it
//!   ends is beyond any before-and-after listing. A process outside this
//!   test process that writes `target/lanes` during the run reads as a
//!   change: a false red, never a false green.

use ratmac_qa::baseline::{repo_root, FREEZE_COMMIT};
use ratmac_qa::lane_runs;
use ratmac_qa::support::{self, TempTree};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, ExitStatus, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// The rename lane and its six tests, as its source declares them.
const RENAME_LANE: &str = "t-085";
const RENAME_TESTS: [&str; 6] = [
    "ht_085_01_the_comparison_catches_a_dropped_or_weakened_check",
    "ht_085_02_input_selects_the_same_edge_as_before",
    "ht_085_03_every_terminal_path_ends_the_run_as_before",
    "ht_085_04_interruption_and_contention_reproduce_the_baseline",
    "ht_085_05_exit_codes_and_receipts_match_the_baseline",
    "ht_085_06_a_spawned_child_joins_exactly_as_before",
];

/// The pre-split ports lane and its six tests, as its source declares them.
const PORTS_LANE: &str = "t-109";
const PORTS_TESTS: [&str; 6] = [
    "ht_109_01_regression_lane_names_are_frozen",
    "ht_109_02_input_routing_scaffolds_address_the_current_root",
    "ht_109_03_lifecycle_readers_speak_the_settled_vocabulary",
    "ht_109_04_durability_a_port_builds_cold",
    "ht_109_05_output_filesystem_the_engine_is_untouched",
    "ht_109_06_cross_feature_the_sweep_reads_the_ports_green",
];

/// What the check writes into the plain file at the freeze folder's old name.
const PLAIN_FILE: &[u8] =
    b"t-140 SVEV-009: a plain file at the shared freeze build folder's name\n";

const RUN_LIMIT: Duration = Duration::from_secs(1800);

/// How many changed paths a finding lists.
const SHOWN: usize = 20;

/// Names the fresh temp folder to the isolated child of the temp folder
/// control; set only on that child.
const FRESH_TEMP: &str = "RATMAC_T140_FRESH_TEMP";

// --- SVEV-009 -----------------------------------------------------------------

#[test]
fn svev_009() {
    if let Some(expected) = std::env::var_os(FRESH_TEMP) {
        // The isolated child of the temp folder control, and nothing else.
        temp_folder_probe(expected);
        return;
    }
    let scratch = TempTree::new("t140-svev-009")
        .unwrap_or_else(|error| panic!("own a scratch tree: {error}"));
    recorder_controls(&scratch.join("control"));
    reader_controls();
    let root = worktree();
    outside_work_trees(&root);
    let _lanes = lane_runs::exclusive();
    let mut failures = freeze_folder(&root, &scratch);
    failures.extend(ports_folder(&root, &scratch));
    assert!(
        failures.is_empty(),
        "SVEV-009: {} finding(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The rename lane, run once with a fresh system temp folder that holds a
/// plain file named `ratmac-freeze-<FREEZE_COMMIT>`: what it got wrong.
fn freeze_folder(root: &Path, scratch: &TempTree) -> Vec<String> {
    let temp = scratch.join("t");
    fs::create_dir(&temp)
        .unwrap_or_else(|error| panic!("create the fresh temp folder {}: {error}", temp.display()));
    let plain = temp.join(format!("ratmac-freeze-{FREEZE_COMMIT}"));
    fs::write(&plain, PLAIN_FILE)
        .unwrap_or_else(|error| panic!("write the plain file {}: {error}", plain.display()));
    temp_folder_control(&temp, &plain);
    let run = Run::lane(
        root,
        RENAME_LANE,
        &scratch.join(format!("{RENAME_LANE}.log")),
        Some(&temp),
    );
    let mut failures = Vec::new();
    if let Some(observed) = run.not_standalone(&RENAME_TESTS) {
        failures.push(format!(
            "with a plain file at {}, the rename lane {RENAME_LANE} does not give its \
             standalone verdict: {observed}",
            plain.display()
        ));
    }
    if let Some(observed) = plain_file_changed(&plain) {
        failures.push(format!(
            "after one run of the rename lane {RENAME_LANE}, the plain file {} {observed}",
            plain.display()
        ));
    }
    failures
}

/// `None` while `plain` is a plain file holding exactly [`PLAIN_FILE`];
/// otherwise what it is now.
fn plain_file_changed(plain: &Path) -> Option<String> {
    match fs::symlink_metadata(plain) {
        Err(error) => Some(format!("is gone ({error})")),
        Ok(metadata) if !metadata.is_file() => Some(format!(
            "is no longer a plain file ({:?})",
            metadata.file_type()
        )),
        Ok(_) => match fs::read(plain) {
            Ok(bytes) if bytes == PLAIN_FILE => None,
            Ok(bytes) => Some(format!(
                "holds other bytes ({} byte(s): {:?})",
                bytes.len(),
                clip(&String::from_utf8_lossy(&bytes), 120)
            )),
            Err(error) => Some(format!("cannot be read ({error})")),
        },
    }
}

/// This case's name in the running test executable: `svev_009` in its own
/// target, prefixed by its module inside the aggregate `lane_sweeps` binary.
fn case_name() -> String {
    match module_path!().split_once("::") {
        Some((_, module)) => format!("{module}::svev_009"),
        None => "svev_009".to_owned(),
    }
}

/// Refuse to run any lane while this process's system temp folder lies
/// inside a Git work tree: the lanes put their scratch projects under it
/// (the rename lane under `t`, inside it), and an Engine started in a
/// project inside a work tree mints its Runs in that repository's primary
/// checkout. The search proves itself on this checkout first.
fn outside_work_trees(root: &Path) {
    assert!(
        enclosing_work_tree(root).is_some(),
        "control: no folder at or above the checkout {} holds a `.git` entry",
        root.display()
    );
    let temp = std::env::temp_dir();
    if let Some(tree) = enclosing_work_tree(&temp) {
        panic!(
            "control: this process's system temp folder {} lies inside the Git work tree {}, \
             where an Engine a lane starts would mint Runs; the check runs no lane. Point \
             TMP, TEMP, and TMPDIR outside every Git work tree.",
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

/// Positive control for the freeze folder fixture: an isolated child of this
/// test executable, given `TMP`, `TEMP`, and `TMPDIR` naming `temp` as the
/// rename lane's run is, finds `temp` as its system temp folder; afterwards
/// `temp` still holds the plain file alone.
fn temp_folder_control(temp: &Path, plain: &Path) {
    let envs = [
        ("TMP", temp.as_os_str()),
        ("TEMP", temp.as_os_str()),
        ("TMPDIR", temp.as_os_str()),
        (FRESH_TEMP, temp.as_os_str()),
    ];
    match support::isolated_test(&case_name(), &envs) {
        Ok(false) => {}
        Ok(true) => panic!("control: this process is not the temp folder control's child"),
        Err(failure) => panic!(
            "control: a child given TMP, TEMP, and TMPDIR naming {} must find it as its \
             system temp folder: {failure}",
            temp.display()
        ),
    }
    let held: Vec<PathBuf> = fs::read_dir(temp)
        .unwrap_or_else(|error| panic!("control: list {}: {error}", temp.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("control: list {}: {error}", temp.display()))
                .path()
        })
        .collect();
    assert!(
        held == [plain.to_path_buf()],
        "control: the fresh temp folder {} holds the plain file alone, found {held:?}",
        temp.display()
    );
}

/// In the isolated child of [`temp_folder_control`]: enter the case, then
/// compare this process's system temp folder with the fresh one.
fn temp_folder_probe(expected: OsString) {
    match support::isolated_test(&case_name(), &[]) {
        Ok(true) => {}
        other => panic!("the temp folder control's child enters its own case, got {other:?}"),
    }
    let found = std::env::temp_dir();
    let same = match (fs::canonicalize(&found), fs::canonicalize(&expected)) {
        (Ok(found), Ok(expected)) => found == expected,
        _ => false,
    };
    assert!(
        same,
        "control: this process's system temp folder is {}, not the fresh folder {}",
        found.display(),
        Path::new(&expected).display()
    );
}

/// The pre-split ports lane, run once between two listings of this
/// checkout's `target/lanes`: what it got wrong.
fn ports_folder(root: &Path, scratch: &TempTree) -> Vec<String> {
    let lanes = root.join("target").join("lanes");
    let before = listing(&lanes);
    let run = Run::lane(
        root,
        PORTS_LANE,
        &scratch.join(format!("{PORTS_LANE}.log")),
        None,
    );
    let after = listing(&lanes);
    let mut failures = Vec::new();
    let changed = changes(&before, &after);
    if !changed.is_empty() {
        failures.push(format!(
            "one run of the pre-split ports lane {PORTS_LANE} changes {} path(s) under {} \
             ({} before, {} after):\n{}",
            changed.len(),
            lanes.display(),
            state(&before),
            state(&after),
            shown(&changed)
        ));
    }
    if let Some(observed) = run.not_standalone(&PORTS_TESTS) {
        failures.push(format!(
            "the pre-split ports lane {PORTS_LANE} does not give its standalone verdict: \
             {observed}"
        ));
    }
    failures
}

/// This checkout's root, without the verbatim prefix `canonicalize` adds on
/// Windows: the spelling the sweep would hand a lane.
fn worktree() -> PathBuf {
    let root = repo_root();
    match root.to_str().and_then(|text| text.strip_prefix(r"\\?\")) {
        Some(plain) if !plain.starts_with("UNC") => PathBuf::from(plain),
        _ => root,
    }
}

// --- one lane run -------------------------------------------------------------

/// How one lane run ended, and its merged output.
struct Run {
    /// `None` when the run was stopped at [`RUN_LIMIT`].
    status: Option<ExitStatus>,
    text: String,
}

impl Run {
    /// One run of `lane` the way `tools/sweep_lanes.py` runs a crate: `cargo
    /// test --offline --target-dir <crate>/target` in the lane's crate folder,
    /// every inherited `CARGO_` variable dropped. With `temp`, `TMP`, `TEMP`,
    /// and `TMPDIR` name it. The merged output goes to `log`; a run still
    /// going after [`RUN_LIMIT`] is stopped with the test executable it
    /// started.
    fn lane(root: &Path, lane: &str, log: &Path, temp: Option<&Path>) -> Self {
        let directory = root.join("test-hidden").join(lane);
        let file =
            File::create(log).unwrap_or_else(|error| panic!("create {}: {error}", log.display()));
        let also = file
            .try_clone()
            .unwrap_or_else(|error| panic!("share {}: {error}", log.display()));
        let mut command = support::command("cargo", &directory);
        for (key, _) in std::env::vars_os() {
            if key
                .to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("CARGO_")
            {
                command.env_remove(&key);
            }
        }
        if let Some(temp) = temp {
            for key in ["TMP", "TEMP", "TMPDIR"] {
                command.env(key, temp);
            }
        }
        command
            .args(["test", "--offline", "--target-dir"])
            .arg(directory.join("target"))
            .stdout(Stdio::from(also))
            .stderr(Stdio::from(file));
        let mut child = command
            .spawn()
            .unwrap_or_else(|error| panic!("start {lane}'s cargo test: {error}"));
        let deadline = Instant::now() + RUN_LIMIT;
        let mut stopped = false;
        while child
            .try_wait()
            .unwrap_or_else(|error| panic!("watch {lane}'s run: {error}"))
            .is_none()
        {
            if Instant::now() >= deadline {
                stop(&mut child);
                stopped = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        let status = child
            .wait()
            .unwrap_or_else(|error| panic!("end {lane}'s run: {error}"));
        let text = fs::read(log)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();
        Self {
            status: (!stopped).then_some(status),
            text,
        }
    }

    /// `None` when the run gives the lane's standalone verdict - exit 0,
    /// every test it reports `ok`, and each of `tests` among them; otherwise
    /// what it gave instead, with the distinct panic and error lines.
    fn not_standalone(&self, tests: &[&str]) -> Option<String> {
        let reported = test_lines(&self.text);
        let mut found = Vec::new();
        match self.status {
            None => found.push(format!("timed out after {}s", RUN_LIMIT.as_secs())),
            Some(status) if !status.success() => found.push(format!("exited {status}")),
            Some(_) => {}
        }
        let not_ok: Vec<String> = reported
            .iter()
            .filter(|(_, result)| result != "ok")
            .map(|(name, result)| format!("{name} {result}"))
            .collect();
        if !not_ok.is_empty() {
            found.push(format!("reported not ok: {}", not_ok.join(", ")));
        }
        let ok: BTreeSet<&str> = reported
            .iter()
            .filter(|(_, result)| result == "ok")
            .map(|(name, _)| name.as_str())
            .collect();
        let missing: Vec<&str> = tests
            .iter()
            .copied()
            .filter(|test| !ok.contains(test))
            .collect();
        if !missing.is_empty() {
            found.push(format!("never reported ok: {}", missing.join(", ")));
        }
        if found.is_empty() {
            return None;
        }
        Some(format!("{}{}", found.join("; "), excerpt(&self.text)))
    }
}

/// Stop a run and the test executable it started: on Windows, killing `cargo
/// test` alone leaves its test executable running.
fn stop(child: &mut Child) {
    if cfg!(windows) {
        let _ = support::command("taskkill", std::env::temp_dir())
            .args(["/T", "/F", "/PID"])
            .arg(child.id().to_string())
            .output();
    }
    let _ = child.kill();
}

/// Every `test <name> ... <result>` line of a `cargo test` transcript, in
/// order.
fn test_lines(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let (name, result) = line.trim().strip_prefix("test ")?.rsplit_once(" ... ")?;
            Some((name.to_owned(), result.trim().to_owned()))
        })
        .collect()
}

/// The distinct panic locations and messages, and error lines, of a
/// transcript, indented under a finding.
fn excerpt(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut kept: Vec<String> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let picked: Vec<&str> = if let Some(at) = line.find("panicked at") {
            let message = lines.get(index + 1).copied().unwrap_or_default();
            vec![&line[at..], message]
        } else if line.trim_start().starts_with("error") {
            vec![*line]
        } else {
            continue;
        };
        for line in picked {
            let line = clip(line.trim_end(), 300);
            if !line.is_empty() && !kept.contains(&line) {
                kept.push(line);
            }
        }
        if kept.len() >= 12 {
            break;
        }
    }
    kept.iter()
        .map(|line| format!("\n      | {line}"))
        .collect()
}

fn clip(text: &str, limit: usize) -> String {
    match text.char_indices().nth(limit) {
        Some((at, _)) => format!("{}...", &text[..at]),
        None => text.to_owned(),
    }
}

// --- the listing of a folder --------------------------------------------------

/// One path under a listed folder.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Node {
    Folder,
    /// A file: its length and SHA-256.
    File(u64, String),
    /// A lock file some run may hold: its length only, never opened.
    Lock(u64),
    /// A link or any other kind: where a link points.
    Other(Option<PathBuf>),
}

impl Node {
    fn describe(&self) -> String {
        match self {
            Node::Folder => "folder".to_owned(),
            Node::File(length, sha256) => {
                format!("file, {length} byte(s), sha256 {}", clip(sha256, 16))
            }
            Node::Lock(length) => format!("lock file, {length} byte(s)"),
            Node::Other(Some(target)) => format!("link to {}", target.display()),
            Node::Other(None) => "other".to_owned(),
        }
    }
}

/// Whether a file is one some run may hold locked - Cargo's own
/// `.cargo-lock` and `.cargo-build-lock`, rustc's session locks, a lane
/// folder's `lane.lock` - and so is listed by name and length only.
fn is_lock(name: &str) -> bool {
    name.ends_with(".lock") || name.ends_with("-lock")
}

/// Every path under `folder` - the folder itself as `.`, the rest relative
/// and `/`-separated - with its node; empty when `folder` is absent. Links
/// are listed, never followed.
fn listing(folder: &Path) -> BTreeMap<String, Node> {
    let mut found = BTreeMap::new();
    let mut files = Vec::new();
    let mut pending = vec![(folder.to_path_buf(), ".".to_owned())];
    while let Some((path, key)) = pending.pop() {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound && key == "." => break,
            Err(error) => panic!("list {}: {error}", path.display()),
        };
        let node = if metadata.is_dir() {
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
            Node::Folder
        } else if metadata.is_file() {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            if is_lock(&name) {
                Node::Lock(metadata.len())
            } else {
                files.push((key, path, metadata.len()));
                continue;
            }
        } else {
            Node::Other(fs::read_link(&path).ok())
        };
        found.insert(key, node);
    }
    found.extend(hashed(&files));
    found
}

/// Each of `files` - key, path, length - as a file node, hashed on all the
/// host's cores: in a test build hashing, not reading, is the cost.
fn hashed(files: &[(String, PathBuf, u64)]) -> Vec<(String, Node)> {
    let workers = std::thread::available_parallelism()
        .map_or(1, |count| count.get())
        .min(files.len());
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    while let Some((key, path, length)) =
                        files.get(next.fetch_add(1, Ordering::Relaxed))
                    {
                        done.push((key.clone(), Node::File(*length, sha256(path))));
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
fn sha256(path: &Path) -> String {
    let mut file =
        File::open(path).unwrap_or_else(|error| panic!("open {}: {error}", path.display()));
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => hasher.update(&buffer[..read]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => panic!("read {}: {error}", path.display()),
        }
    }
    format!("{:x}", hasher.finalize())
}

/// What differs between two listings, one line per path, in path order.
fn changes(before: &BTreeMap<String, Node>, after: &BTreeMap<String, Node>) -> Vec<String> {
    let paths: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    paths
        .into_iter()
        .filter_map(|path| match (before.get(path), after.get(path)) {
            (Some(was), None) => Some(format!("removed {path} ({})", was.describe())),
            (None, Some(now)) => Some(format!("added {path} ({})", now.describe())),
            (Some(was), Some(now)) if was != now => Some(format!(
                "changed {path}: {} -> {}",
                was.describe(),
                now.describe()
            )),
            _ => None,
        })
        .collect()
}

/// A listing in a few words.
fn state(listing: &BTreeMap<String, Node>) -> String {
    if listing.is_empty() {
        "absent".to_owned()
    } else {
        format!("{} path(s)", listing.len())
    }
}

/// The first [`SHOWN`] changes, indented, and how many more there are.
fn shown(changed: &[String]) -> String {
    let mut text: String = changed
        .iter()
        .take(SHOWN)
        .map(|line| format!("  {line}\n"))
        .collect();
    if changed.len() > SHOWN {
        text.push_str(&format!("  ... and {} more\n", changed.len() - SHOWN));
    }
    text
}

// --- controls -----------------------------------------------------------------

/// The recorder proves itself on a scratch `folder` (absent at first), so a
/// blind listing cannot pass.
fn recorder_controls(folder: &Path) {
    let absent = listing(folder);
    assert!(
        absent.is_empty(),
        "control: an absent folder lists as absent, read {absent:?}"
    );
    fs::create_dir(folder).unwrap_or_else(|error| panic!("control: create a folder: {error}"));
    let empty = listing(folder);
    assert_eq!(
        empty,
        BTreeMap::from([(".".to_owned(), Node::Folder)]),
        "control: an empty folder lists as one folder"
    );
    let inner = folder.join("inner");
    fs::create_dir(&inner).unwrap_or_else(|error| panic!("control: create a folder: {error}"));
    fs::write(inner.join("file"), b"one").unwrap_or_else(|error| panic!("control: {error}"));
    fs::write(inner.join("second"), b"two").unwrap_or_else(|error| panic!("control: {error}"));
    let filled = listing(folder);
    assert_eq!(
        listing(folder),
        filled,
        "control: an unchanged folder lists the same"
    );
    fs::write(inner.join("file"), b"onE").unwrap_or_else(|error| panic!("control: {error}"));
    let rewritten = listing(folder);
    let changed = changes(&filled, &rewritten);
    assert!(
        changed.len() == 1 && changed[0].starts_with("changed inner/file: "),
        "control: one changed byte in inner/file changes that path alone, read {changed:?}"
    );
    fs::write(folder.join("added"), b"").unwrap_or_else(|error| panic!("control: {error}"));
    let added = listing(folder);
    assert_eq!(
        changes(&rewritten, &added),
        ["added added (file, 0 byte(s), sha256 e3b0c44298fc1c14...)"],
        "control: an added file is one added path"
    );
    fs::remove_file(folder.join("added")).unwrap_or_else(|error| panic!("control: {error}"));
    fs::create_dir(folder.join("added")).unwrap_or_else(|error| panic!("control: {error}"));
    let turned = listing(folder);
    assert_eq!(
        changes(&added, &turned).len(),
        1,
        "control: a file turned into a folder changes one path, read {:?}",
        changes(&added, &turned)
    );
    let held_path = folder.join("held.lock");
    let held = File::create(&held_path).unwrap_or_else(|error| panic!("control: {error}"));
    held.lock()
        .unwrap_or_else(|error| panic!("control: lock {}: {error}", held_path.display()));
    let locked = listing(folder);
    assert_eq!(
        locked.get("held.lock"),
        Some(&Node::Lock(0)),
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
        .filter(|(name, lock)| is_lock(name) != *lock)
        .map(|(name, _)| *name)
        .collect();
    assert!(
        misread.is_empty(),
        "control: lock file names are told from build output, misread {misread:?}"
    );
}

/// The verdict reader proves itself on known transcripts.
fn reader_controls() {
    let text = "running 3 tests\n\
                test alpha ... ok\n\
                test beta has been running for over 60 seconds\n\
                test beta ... FAILED\n\
                test gamma ... ignored\n\
                \n\
                test result: FAILED. 1 passed; 1 failed; 1 ignored; 0 measured\n";
    let read = test_lines(text);
    let expected: Vec<(String, String)> =
        [("alpha", "ok"), ("beta", "FAILED"), ("gamma", "ignored")]
            .iter()
            .map(|(name, result)| ((*name).to_owned(), (*result).to_owned()))
            .collect();
    assert_eq!(read, expected, "control: the reader finds every test line");
    let passed = |text: &str| Run {
        status: Some(ExitStatus::default()),
        text: text.to_owned(),
    };
    let green = "test alpha ... ok\ntest beta ... ok\n";
    assert_eq!(
        passed(green).not_standalone(&["alpha", "beta"]),
        None,
        "control: exit 0 with every test ok is the standalone verdict"
    );
    for (text, why) in [
        ("test alpha ... ok\n", "a test never reported"),
        (
            "test alpha ... ok\ntest beta ... ignored\n",
            "an ignored test",
        ),
        ("test alpha ... ok\ntest beta ... FAILED\n", "a failed test"),
    ] {
        assert!(
            passed(text).not_standalone(&["alpha", "beta"]).is_some(),
            "control: {why} is not the standalone verdict"
        );
    }
    let stopped = Run {
        status: None,
        text: green.to_owned(),
    };
    assert!(
        stopped.not_standalone(&["alpha", "beta"]).is_some(),
        "control: a stopped run is not the standalone verdict"
    );
}
