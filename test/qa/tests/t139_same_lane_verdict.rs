//! t-139 / SVE-007: lane builds stay private.
//!
//! SVEV-007 `svev_007` (shared output untouched)
//! SVEV-008 `svev_008` (forced concurrent runs agree)
//!
//! Both checks run six real lanes, one of each kind of Cargo run: the
//! single-Engine-binary lane (`t-086`), a `today_engine()` caller (`t-116`),
//! the edition-cut and snapshot lanes (`t-127`, `t-113`), an audit-consumer
//! lane (`t-084`), and the lane-sweep lane (`t-108`, which runs
//! `test-hidden/t-094`). They run in a scratch copy of the repository outside
//! the worktree: every file `git ls-files` lists, with its working bytes, plus
//! the untracked files `.gitignore` leaves, plus all of `test-hidden/` except
//! its `target` folders (`t-086` audits every rostered lane's declared
//! inputs). The copy is a Git repository whose index holds exactly the paths
//! the worktree indexes (`t-084` selects indexed files). A lane runs the way
//! `tools/sweep_lanes.py` runs a crate: `cargo test --offline --target-dir
//! <crate>/target` in the crate's folder, with every inherited `CARGO_`
//! variable dropped. The valid control is each lane's standalone verdict:
//! that command outside any sweep, with the copy's default `target` an
//! ordinary folder. Every lane must pass alone there. If one does not, the
//! check fails as a control failure, because then it proves nothing.
//!
//! Cost and reuse: there is one copy per test process. The first check that
//! needs it prepares it. Both checks hold `lane_runs::exclusive` and the
//! copy's own lock for their whole run, so their steps never overlap. Each
//! check sets up the default `target` it needs: SVEV-008 moves that folder
//! aside, out of the copy, and SVEV-007 moves it back. The controls are a
//! `lane_runs::Observations` over the copy. The second check reuses them only
//! while the copy's inputs still match what the control runs saw from start
//! to finish (build output is not an input). Every lane's own build output in
//! the copy carries over between steps, the same way a lane's `target` does
//! between two runs. The copy is removed when the test process ends normally.
//! The test harness ends a failed run without any cleanup, so each copy holds
//! a lock file for its process's life, and the next run that prepares a copy
//! first removes every copy whose lock no process holds.
//!
//! SVEV-007: first, Cargo builds `rtm`, `rtm-qa`, `edition-cut`, and
//! `snapshot-manifest` in the copy's default `target`, with the arguments the
//! lanes build them with, and `built_executable` reports each executable: the
//! file named after the binary, with the platform's suffix, in the profile
//! folder. Each one is replaced by a stand-in of its own bytes (never written
//! through Cargo's hard link), and the bytes are recorded. One
//! `tools/sweep_lanes.py sweep --config <declaration>` over a declaration
//! that rosters the six lanes must read each lane's standalone verdict and
//! leave every stand-in's bytes. Then the stand-ins are planted afresh, and
//! `t-086` runs alone with `CARGO_TARGET_DIR` naming a fresh folder that
//! holds the same stand-ins at the same places below it (the profile folder
//! and file names Cargo reported). This run is outside the sweep, which drops
//! `CARGO_` variables. It must give the same verdict and keep both sets of
//! stand-ins. The source inventory runs before the lanes and is reported with
//! them; its rule follows.
//!
//! The inventory rule. It covers every `.rs` file of every live lane in
//! `.ratmac/lanes.toml` except the pre-split ports lane `t-109`, whose folder
//! belongs to the freeze and ports folder ticket (`t-140`). It skips every
//! `target/` folder and `test-hidden/t-126/fixtures/before/`. Comments and
//! string literals never count as code. A site is:
//! - `today_engine` anywhere outside a `use` item, as a call or as a function
//!   value;
//! - a call of the free function `built_executable(root, ..)`;
//! - a Cargo command: `command(<cargo>, root)` (`support::command` by any
//!   path), or `Command::new(<cargo>)` with `root` taken from its
//!   `.current_dir(..)`, or the lane's own crate when it has none. It counts
//!   when its first literal `.arg`/`.args` is `build`, `test`, or `run`, or
//!   when it has none and the caller adds the arguments. `<cargo>` is the
//!   literal `"cargo"`, a read of `CARGO`, or a lane function that returns one.
//!
//! The last two count only when `root` is the repository or a crate in it.
//! That holds when the root expression reaches `repo_root()`,
//! `env!("CARGO_MANIFEST_DIR")`, or a lane function returning `Path` or
//! `PathBuf` whose tail or `return` expression reaches them. A function that
//! copies the repository into an owned tree and returns the copy is not one.
//! It can reach them directly, or through the latest `let` before the use.
//! If the root is a parameter of the enclosing function, it can reach
//! them through any caller in the lane that passes such an argument; the
//! finding then names the caller's line. Scratch copies and synthetic fixtures
//! in an owned temporary tree reach none of these, so they are never sites.
//! The shared helper's own methods on a `lane_folder(..)` value (`.engine()`,
//! `.build(..)`, `.test(..)`, `.cargo(..)`) are never sites either. Neither is
//! `baseline::engine()`, the freeze build, which belongs to SVE-008. One named
//! exception is set aside and listed: `ht_137_01_child` in
//! `test-hidden/t-137/tests/hidden.rs`. HT-137-01 proves `today_engine()`
//! itself, and that child's builds go through `CARGO_TARGET_DIR` into
//! `t-137`'s lane folder, which its parent holds under the lane lock. The
//! matcher proves itself on known shapes first, so a blind matcher cannot
//! pass.
//!
//! SVEV-008: the copy's default `target` is a plain file. For each lane, two
//! runs start at once, each in its own process, and both must read the
//! standalone verdict. A Cargo build into that output does not fail on this
//! toolchain: Cargo renames a fresh folder over the plain file and builds
//! there. So after the two runs, and again after the held run below, the
//! plain file must still hold the bytes the check wrote. If it does not, a
//! run built in the repository's shared output; the check reports it and
//! writes the plain file again before the next runs. Then the check itself
//! holds the lane's lock file with
//! `File::lock`. That file is never one of Cargo's; it is the path the
//! contract names, `<CARGO_TARGET_TMPDIR>/<lane>/lane.lock`, which for a crate
//! building in `<crate>/target` is `<crate>/target/tmp/<lane>/lane.lock`.
//! With that file held, one run starts. It must print `<lane>: waiting for the
//! lane lock <that file>`. For five seconds after that line, the folder's
//! listing (names, sizes, modified times) must not change; the lock file
//! counts by name and size only, since opening it is how a run waits. Once
//! the check releases the lock, the run must read the standalone verdict.
//!
//! Hole-poke notes:
//! - A lane that rebuilds into the shared `target` but keeps its verdict still
//!   fails SVEV-007, because every stand-in is compared byte for byte.
//! - A lane outside the six that still builds in the shared output is caught
//!   by the inventory. The inventory's limits: it does not trace a root held in
//!   a struct field or a static, or a Cargo command built by a method wrapper
//!   (`self.command("cargo", ..)`). No live lane uses either shape.
//! - A lane that waits on some other lock never prints the line naming this
//!   file. A lane that prints the line but builds anyway changes the listing.

use ratmac_qa::baseline::{built_executable, repo_root};
use ratmac_qa::lane_runs::{self, Observations};
use ratmac_qa::support::{self, TempTree};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{c_int, OsString};
use std::fs::{self, File, OpenOptions};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Once, PoisonError, TryLockError};
use std::time::{Duration, Instant, SystemTime};

/// One lane of each kind of Cargo run, in id order.
const LANES: [&str; 6] = ["t-084", "t-086", "t-108", "t-113", "t-116", "t-127"];

/// The single-Engine-binary lane, also run alone with `CARGO_TARGET_DIR`.
const SINGLE_ENGINE_LANE: &str = "t-086";

/// The repository binaries a lane builds and launches, each with the `cargo
/// build` arguments the lanes build it with.
const STAND_INS: [(&str, &[&str]); 4] = [
    ("rtm", &["--offline", "--bin", "rtm"]),
    (
        "rtm-qa",
        &["--offline", "-p", "ratmac-qa", "--bin", "rtm-qa"],
    ),
    (
        "edition-cut",
        &["--offline", "-p", "ratmac-qa", "--bin", "edition-cut"],
    ),
    (
        "snapshot-manifest",
        &["--offline", "-p", "ratmac-qa", "--bin", "snapshot-manifest"],
    ),
];

/// The pre-split ports lane: its folder moves in `t-140` (SVE-008).
const PORTS_LANE: &str = "t-109";

/// The frozen before-source corpus, never a source.
const FROZEN_CORPUS: &str = "test-hidden/t-126/fixtures/before/";

/// Cargo subcommands that build.
const BUILDING: [&str; 3] = ["build", "test", "run"];

/// The one named inventory exception: file, function, reason.
const EXCEPTION: (&str, &str, &str) = (
    "test-hidden/t-137/tests/hidden.rs",
    "ht_137_01_child",
    "HT-137-01 proves today_engine() itself (SVE-001: one build per test process); the child's \
     builds go by CARGO_TARGET_DIR into t-137's lane folder, which its parent holds under the \
     lane lock",
);

const RUN_LIMIT: Duration = Duration::from_secs(1800);
const WAIT_LIMIT: Duration = Duration::from_secs(900);
const SETTLE: Duration = Duration::from_secs(5);
const DEPTH: usize = 8;

// --- SVEV-007 -----------------------------------------------------------------

#[test]
fn svev_007() {
    inventory_controls();
    let inventory = inventory(&worktree());
    let mut failures = Vec::new();
    if !inventory.findings.is_empty() {
        failures.push(format!(
            "the inventory of {} sources in {} live lanes (t-109 aside) finds {} Cargo run(s) on \
             the repository or a lane crate outside the shared locked-folder helper:\n{}",
            inventory.files,
            inventory.lanes,
            inventory.findings.len(),
            inventory.findings.join("\n")
        ));
    }
    let _lanes = lane_runs::exclusive();
    with_copy(|shared| {
        let controls = standalone(shared);
        default_target_folder(shared);
        let built = default_binaries(shared);
        let planted = plant_stand_ins(&built);
        let rows = sweep(shared);
        for (lane, control) in controls.iter() {
            match rows.get(lane) {
                Some(verdict) if *verdict == control.verdict => {}
                Some(verdict) => failures.push(format!(
                    "{lane}, run once through tools/sweep_lanes.py with stand-ins in the shared \
                     target, reads {verdict} where it reads {} alone",
                    control.verdict
                )),
                None => failures.push(format!("the sweep report has no row for {lane}")),
            }
        }
        failures.extend(lost(&planted, "after the sweep"));
        let replanted = plant_stand_ins(&built);
        let redirect = fresh_folder(shared, "redirect");
        let redirected = plant_stand_ins(&below(&built, &redirect));
        let run = redirected_run(shared, &redirect);
        let control = &controls
            .iter()
            .find(|(lane, _)| lane == SINGLE_ENGINE_LANE)
            .expect("the single-Engine-binary lane has a control")
            .1;
        if run.verdict != control.verdict {
            failures.push(format!(
                "{SINGLE_ENGINE_LANE}, run alone with CARGO_TARGET_DIR={}, reads {} where it \
                 reads {} alone{}",
                redirect.display(),
                run.verdict,
                control.verdict,
                run.excerpt
            ));
        }
        failures.extend(lost(
            &redirected,
            "after t-086 ran alone with CARGO_TARGET_DIR naming their folder",
        ));
        failures.extend(lost(
            &replanted,
            "after t-086 ran alone with CARGO_TARGET_DIR set",
        ));
    });
    assert!(
        failures.is_empty(),
        "SVEV-007: with stand-ins in the shared build output, each lane must give its standalone \
         verdict and leave every stand-in's bytes, and no live lane may run Cargo on the \
         repository or a lane crate outside the shared locked-folder helper; observed:\n{}\n\
         set aside by named exceptions:\n{}",
        failures.join("\n"),
        inventory.set_aside.join("\n")
    );
}

/// Each repository binary built in the copy's default `target`, at the
/// executable `built_executable` reports. `--target-dir` names that folder,
/// so an inherited redirect cannot move the build. The executable must sit in
/// a profile folder directly inside that `target`, where a lane that builds
/// with no redirect finds it; anywhere else, no stand-in could stand for it,
/// and the check would prove nothing.
fn default_binaries(shared: &Shared) -> Vec<(&'static str, PathBuf)> {
    let target = shared.copy.join("target");
    STAND_INS
        .iter()
        .map(|(name, args)| {
            let mut full: Vec<OsString> = args.iter().map(OsString::from).collect();
            full.push("--target-dir".into());
            full.push(target.clone().into_os_string());
            let path = built_executable(&shared.copy, &full, name).unwrap_or_else(|refusal| {
                panic!("control: the copy's default build of {name} refused:\n{refusal}")
            });
            let folder = path.parent().and_then(Path::parent);
            assert!(
                folder.is_some_and(|folder| same_path(folder, &target)),
                "control: Cargo put the copy's default build of {name} at {}, not in a profile \
                 folder directly inside {}",
                path.display(),
                target.display()
            );
            (*name, path)
        })
        .collect()
}

/// The same places as `built`, below `folder` instead of the copy's default
/// `target`: the profile folder and file names Cargo reported.
fn below(built: &[(&'static str, PathBuf)], folder: &Path) -> Vec<(&'static str, PathBuf)> {
    built
        .iter()
        .map(|(name, path)| {
            let profile = path
                .parent()
                .and_then(Path::file_name)
                .expect("a built executable lies in a profile folder");
            let file = path
                .file_name()
                .expect("a built executable has a file name");
            (*name, folder.join(profile).join(file))
        })
        .collect()
}

/// Put a stand-in of its own bytes at each path. Any file there is removed
/// first, so a hard link Cargo made from its `deps` folder is never written
/// through.
fn plant_stand_ins(places: &[(&'static str, PathBuf)]) -> Vec<(PathBuf, Vec<u8>)> {
    places
        .iter()
        .map(|(name, path)| {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .unwrap_or_else(|error| panic!("create {}: {error}", parent.display()));
            }
            let bytes = format!("t-139 stand-in for {name}: no build wrote this; never run it\n");
            if fs::symlink_metadata(path).is_ok() {
                fs::remove_file(path)
                    .unwrap_or_else(|error| panic!("remove {}: {error}", path.display()));
            }
            fs::write(path, &bytes)
                .unwrap_or_else(|error| panic!("plant {}: {error}", path.display()));
            (path.clone(), bytes.into_bytes())
        })
        .collect()
}

/// Every stand-in that no longer holds its planted bytes.
fn lost(planted: &[(PathBuf, Vec<u8>)], when: &str) -> Vec<String> {
    planted
        .iter()
        .filter_map(|(path, bytes)| match fs::read(path) {
            Ok(now) if now == *bytes => None,
            Ok(now) => Some(format!(
                "the stand-in {} lost its bytes {when}: it now holds {} bytes{}",
                path.display(),
                now.len(),
                if now.starts_with(b"MZ") || now.starts_with(b"\x7fELF") {
                    ", an executable a build wrote"
                } else {
                    ""
                }
            )),
            Err(error) => Some(format!(
                "the stand-in {} is gone {when}: {error}",
                path.display()
            )),
        })
        .collect()
}

/// One sweep through the copy's shipped script over a declaration that
/// rosters the six lanes; each row's verdict by crate id.
fn sweep(shared: &Shared) -> BTreeMap<String, String> {
    let root = fresh_folder(shared, "sweep");
    let report = root.join("report.md");
    let declaration = root.join(".ratmac").join("lanes.toml");
    let roster: Vec<String> = LANES.iter().map(|lane| format!("\"{lane}\"")).collect();
    fs::create_dir_all(root.join(".ratmac")).expect("create the sweep declaration folder");
    fs::write(
        &declaration,
        format!(
            "[roots]\nlanes = {}\nreport = {}\nroster = [{}]\n",
            toml_path(&shared.copy.join("test-hidden")),
            toml_path(&report),
            roster.join(", ")
        ),
    )
    .expect("write the sweep declaration");
    let output = support::command("python", &shared.copy)
        .arg(shared.copy.join("tools").join("sweep_lanes.py"))
        .args(["sweep", "--config"])
        .arg(&declaration)
        .output()
        .expect("invoke the lane sweep");
    let text = fs::read_to_string(&report).unwrap_or_else(|error| {
        panic!(
            "SVEV-007: the sweep wrote no report at {}: {error}\n{}",
            report.display(),
            support::text(&output)
        )
    });
    text.lines()
        .filter(|line| line.starts_with("| t-"))
        .filter_map(|line| {
            let cells: Vec<&str> = line
                .trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();
            let [lane, verdict, detail] = cells.as_slice() else {
                return None;
            };
            let verdict = if *verdict == "pass" {
                (*verdict).to_owned()
            } else {
                format!("{verdict} ({detail})")
            };
            Some(((*lane).to_owned(), verdict))
        })
        .collect()
}

/// `path` as a TOML literal string.
fn toml_path(path: &Path) -> String {
    let text = path.display().to_string().replace('\\', "/");
    assert!(!text.contains('\''), "a scratch path holds a quote: {text}");
    format!("'{text}'")
}

/// `t-086` alone, outside the sweep, with `CARGO_TARGET_DIR` naming `folder`.
fn redirected_run(shared: &Shared, folder: &Path) -> Run {
    let directory = lane_dir(shared, SINGLE_ENGINE_LANE);
    let mut command = support::command("cargo", &directory);
    drop_cargo_variables(&mut command);
    let output = command
        .env("CARGO_TARGET_DIR", folder)
        .args(["test", "--offline"])
        .output()
        .expect("run t-086 with CARGO_TARGET_DIR");
    judged(Some(output.status), &support::text(&output))
}

// --- SVEV-008 -----------------------------------------------------------------

#[test]
fn svev_008() {
    let _lanes = lane_runs::exclusive();
    let failures = with_copy(|shared| {
        let controls = standalone(shared);
        default_target_file(shared);
        let mut failures = Vec::new();
        for (lane, control) in controls.iter() {
            let first = Started::new(shared, lane, "first");
            let second = Started::new(shared, lane, "second");
            for (which, run) in [("first", first.finish()), ("second", second.finish())] {
                if run.verdict != control.verdict {
                    failures.push(format!(
                        "{lane}: the {which} of two runs started at once reads {} where the lane \
                         alone reads {}{}",
                        run.verdict, control.verdict, run.excerpt
                    ));
                }
            }
            failures.extend(plain_target_kept(
                shared,
                lane,
                "after two runs started at once",
            ));
            failures.extend(held(shared, lane, control));
            failures.extend(plain_target_kept(
                shared,
                lane,
                "after the run started while the check held the lane lock",
            ));
        }
        failures
    });
    assert!(
        failures.is_empty(),
        "SVEV-008: with the default target a plain file, each lane must give its standalone \
         verdict in two runs started at once, and a run started while the check holds the \
         lane's lane.lock must say it waits for that file, write nothing in the folder until \
         release, then give its standalone verdict; observed:\n{}",
        failures.join("\n")
    );
}

/// One run started while the check holds the lane's own lock file.
fn held(shared: &Shared, lane: &str, control: &Run) -> Vec<String> {
    let folder = lane_dir(shared, lane).join("target").join("tmp").join(lane);
    fs::create_dir_all(&folder)
        .unwrap_or_else(|error| panic!("create {}: {error}", folder.display()));
    let lock_file = folder.join("lane.lock");
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_file)
        .unwrap_or_else(|error| panic!("open {}: {error}", lock_file.display()));
    lock.lock()
        .unwrap_or_else(|error| panic!("hold {}: {error}", lock_file.display()));
    let before = listing(&folder, &lock_file);
    let mut started = Started::new(shared, lane, "held");
    let line = format!("{lane}: waiting for the lane lock {}", lock_file.display());
    let mut failures = Vec::new();
    match started.wait_line(lane, WAIT_LIMIT) {
        Waiting::Named(named) => {
            if !same_path(Path::new(&named), &lock_file) {
                failures.push(format!(
                    "{lane}: the waiting run names {named}, not the lane's lock file {}",
                    lock_file.display()
                ));
            }
            std::thread::sleep(SETTLE);
            let during = listing(&folder, &lock_file);
            if during != before {
                failures.push(format!(
                    "{lane}: while it waited for the lock, the run wrote in {}: {}",
                    folder.display(),
                    changes(&before, &during)
                ));
            }
        }
        Waiting::Ended => failures.push(format!(
            "{lane}: with {} held by the check, a started run ended without the wait line `{line}`",
            lock_file.display()
        )),
        Waiting::Silent => failures.push(format!(
            "{lane}: with {} held by the check, a started run neither printed `{line}` nor ended \
             within {}s",
            lock_file.display(),
            WAIT_LIMIT.as_secs()
        )),
    }
    lock.unlock()
        .unwrap_or_else(|error| panic!("release {}: {error}", lock_file.display()));
    drop(lock);
    let run = started.finish();
    if run.verdict != control.verdict {
        failures.push(format!(
            "{lane}: the run started under the held lock reads {} where the lane alone reads {}{}",
            run.verdict, control.verdict, run.excerpt
        ));
    }
    failures
}

/// Every entry under `folder`: files with size and modified time, folders by
/// name, and the lock file by name and size only.
fn listing(folder: &Path, lock_file: &Path) -> BTreeMap<String, (u64, Option<SystemTime>)> {
    let mut found = BTreeMap::new();
    let mut pending = vec![folder.to_path_buf()];
    while let Some(next) = pending.pop() {
        let Ok(entries) = fs::read_dir(&next) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let shown = path
                .strip_prefix(folder)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if metadata.is_dir() {
                found.insert(format!("{shown}/"), (0, None));
                pending.push(path);
            } else if path == lock_file {
                found.insert(shown, (metadata.len(), None));
            } else {
                found.insert(shown, (metadata.len(), metadata.modified().ok()));
            }
        }
    }
    found
}

fn changes(
    before: &BTreeMap<String, (u64, Option<SystemTime>)>,
    after: &BTreeMap<String, (u64, Option<SystemTime>)>,
) -> String {
    let names: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    let changed: Vec<String> = names
        .into_iter()
        .filter_map(|name| match (before.get(name), after.get(name)) {
            (Some(_), None) => Some(format!("{name} removed")),
            (None, Some(_)) => Some(format!("{name} added")),
            (Some(old), Some(new)) if old != new => Some(format!("{name} rewritten")),
            _ => None,
        })
        .collect();
    let shown: Vec<&str> = changed.iter().take(10).map(String::as_str).collect();
    format!("{} change(s): {}", changed.len(), shown.join(", "))
}

/// Whether two spellings name the same file or folder.
fn same_path(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => comparable(left) == comparable(right),
    }
}

fn comparable(path: &Path) -> String {
    let text = path.display().to_string();
    let text = text
        .strip_prefix(r"\\?\")
        .unwrap_or(&text)
        .replace('\\', "/");
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    }
}

/// The path a `<lane>: waiting for the lane lock <file>` line names, without
/// any quotes around it.
fn wait_line(text: &str, lane: &str) -> Option<String> {
    let words = format!("{lane}: waiting for the lane lock ");
    text.lines().find_map(|line| {
        let at = line.find(&words)?;
        let named = line[at + words.len()..].trim();
        Some(named.trim_matches(['`', '"', '\'']).to_owned())
    })
}

// --- the scratch copy and its lane runs ---------------------------------------

/// The process's one scratch copy: `copy/` and the checks' own folders
/// beside it, all inside one owned tree whose owner lock this process holds.
/// The lock closes before the tree goes.
struct Shared {
    _owner: File,
    scratch: TempTree,
    copy: PathBuf,
}

/// The scratch tree's label: every tree this file makes is named
/// `ratmac-<label>-..` in the system temporary folder.
const TREE_LABEL: &str = "t139-svev-copy";

/// The lock file a scratch tree's owning process holds for its whole life.
const OWNER_LOCK: &str = "svev-copy-owner.lock";

static SHARED: Mutex<Option<Shared>> = Mutex::new(None);
static CONTROLS: Observations<Vec<(String, Run)>> = Observations::new();
static REMOVAL: Once = Once::new();
static NEXT: AtomicUsize = AtomicUsize::new(0);

extern "C" {
    fn atexit(callback: extern "C" fn()) -> c_int;
}

/// Remove the scratch copy when the test process ends normally: no static is
/// dropped, and the copy outlives every check that shares it. A copy still in
/// use stays; the next run removes it.
extern "C" fn remove_scratch() {
    let shared = match SHARED.try_lock() {
        Ok(mut shared) => shared.take(),
        Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner().take(),
        Err(TryLockError::WouldBlock) => None,
    };
    drop(shared);
}

/// Run `check` over the scratch copy, preparing it on first use; the copy's
/// lock serializes every step over it.
fn with_copy<R>(check: impl FnOnce(&Shared) -> R) -> R {
    let mut shared = SHARED.lock().unwrap_or_else(PoisonError::into_inner);
    if shared.is_none() {
        REMOVAL.call_once(|| {
            // SAFETY: `atexit` only records a plain function pointer; the
            // callback touches nothing but this file's static.
            let registered = unsafe { atexit(remove_scratch) };
            assert_eq!(registered, 0, "register the scratch copy's removal at exit");
        });
        *shared = Some(prepare());
    }
    check(shared.as_ref().expect("the scratch copy is prepared"))
}

/// The worktree root, without the verbatim prefix `canonicalize` adds on
/// Windows.
fn worktree() -> PathBuf {
    let root = repo_root();
    match root.to_str().and_then(|text| text.strip_prefix(r"\\?\")) {
        Some(plain) if !plain.starts_with("UNC") => PathBuf::from(plain),
        _ => root,
    }
}

fn prepare() -> Shared {
    remove_stale_trees();
    let scratch =
        TempTree::new(TREE_LABEL).unwrap_or_else(|error| panic!("own the scratch tree: {error}"));
    let owner = own(&scratch);
    let copy = scratch.join("copy");
    let source = worktree();
    let mut indexed = Vec::new();
    for relative in listed(&source, &["--cached"]) {
        if copy_file(&source.join(&relative), &copy.join(&relative)) {
            indexed.extend_from_slice(relative.as_bytes());
            indexed.push(0);
        }
    }
    for relative in listed(&source, &["--others", "--exclude-standard"]) {
        copy_file(&source.join(&relative), &copy.join(&relative));
    }
    copy_lanes(&source.join("test-hidden"), &copy.join("test-hidden"));
    let pathspec = scratch.join("indexed.nul");
    fs::write(&pathspec, &indexed).expect("write the indexed path list");
    git(&copy, &["init", "-q"]);
    let from_file = format!("--pathspec-from-file={}", pathspec.display());
    git(
        &copy,
        &[
            "--literal-pathspecs",
            "add",
            "--force",
            from_file.as_str(),
            "--pathspec-file-nul",
        ],
    );
    Shared {
        _owner: owner,
        scratch,
        copy,
    }
}

/// Take the new tree's owner lock for the life of this process. The lock
/// file gets its name only once it is held, so no other run can find it
/// free in between.
fn own(scratch: &TempTree) -> File {
    let pending = scratch.join(format!("{OWNER_LOCK}.new"));
    let owner = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&pending)
        .unwrap_or_else(|error| panic!("create {}: {error}", pending.display()));
    owner
        .lock()
        .unwrap_or_else(|error| panic!("hold {}: {error}", pending.display()));
    let named = scratch.join(OWNER_LOCK);
    fs::rename(&pending, &named)
        .unwrap_or_else(|error| panic!("name {}: {error}", named.display()));
    owner
}

/// Remove every scratch tree an earlier run of these checks left behind: one
/// whose owner lock no process holds. The test harness ends a failed run
/// without any cleanup, so this is where that run's copy goes.
fn remove_stale_trees() {
    let prefix = format!("ratmac-{TREE_LABEL}-");
    let Ok(entries) = fs::read_dir(std::env::temp_dir()) else {
        return;
    };
    for entry in entries.flatten() {
        if !entry.file_name().to_string_lossy().starts_with(&prefix) {
            continue;
        }
        let tree = entry.path();
        let Ok(owner) = OpenOptions::new()
            .read(true)
            .write(true)
            .open(tree.join(OWNER_LOCK))
        else {
            continue;
        };
        if owner.try_lock().is_ok() {
            drop(owner);
            let _ = fs::remove_dir_all(&tree);
        }
    }
}

/// `git ls-files -z` in `root` with `which`.
fn listed(root: &Path, which: &[&str]) -> Vec<String> {
    let output = support::git(root)
        .args(["ls-files", "-z"])
        .args(which)
        .output()
        .expect("list the worktree's files");
    assert!(
        output.status.success(),
        "git ls-files: {}",
        support::text(&output)
    );
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .map(|name| String::from_utf8_lossy(name).into_owned())
        .collect()
}

fn git(root: &Path, args: &[&str]) {
    let output = support::git(root).args(args).output().expect("invoke git");
    assert!(
        output.status.success(),
        "git {} in the scratch copy: {}",
        args.join(" "),
        support::text(&output)
    );
}

/// Copy one regular file; a listed path that is not one copies nothing.
fn copy_file(from: &Path, to: &Path) -> bool {
    if !from.is_file() {
        return false;
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("create {}: {error}", parent.display()));
    }
    fs::copy(from, to).unwrap_or_else(|error| panic!("copy {}: {error}", from.display()));
    true
}

/// Copy a lanes tree, leaving out every `target` folder.
fn copy_lanes(from: &Path, to: &Path) {
    let entries =
        fs::read_dir(from).unwrap_or_else(|error| panic!("list {}: {error}", from.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|error| panic!("list {}: {error}", from.display()));
        let kind = entry.file_type().expect("read an entry's kind");
        let name = entry.file_name();
        if kind.is_dir() {
            if name != "target" {
                copy_lanes(&entry.path(), &to.join(&name));
            }
        } else {
            copy_file(&entry.path(), &to.join(&name));
        }
    }
}

fn lane_dir(shared: &Shared, lane: &str) -> PathBuf {
    shared.copy.join("test-hidden").join(lane)
}

fn fresh_path(shared: &Shared, label: &str) -> PathBuf {
    shared
        .scratch
        .join(format!("{label}-{}", NEXT.fetch_add(1, Ordering::Relaxed)))
}

fn fresh_folder(shared: &Shared, label: &str) -> PathBuf {
    let folder = fresh_path(shared, label);
    fs::create_dir_all(&folder)
        .unwrap_or_else(|error| panic!("create {}: {error}", folder.display()));
    folder
}

/// The copy's default `target` as an ordinary folder: a plain file there
/// goes, and the folder an earlier step moved aside comes back.
fn default_target_folder(shared: &Shared) {
    let target = shared.copy.join("target");
    if fs::symlink_metadata(&target).is_ok_and(|metadata| !metadata.is_dir()) {
        fs::remove_file(&target).expect("remove the plain-file target");
    }
    let aside = shared.scratch.join("target-aside");
    if aside.is_dir() && fs::symlink_metadata(&target).is_err() {
        moved(&aside, &target);
    }
}

/// The copy's default `target` as a plain file. A folder there moves aside,
/// out of the copy, and keeps its build output for a later step.
fn default_target_file(shared: &Shared) {
    let target = shared.copy.join("target");
    let aside = shared.scratch.join("target-aside");
    match fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.is_dir() => {
            if aside.exists() {
                fs::remove_dir_all(&aside).expect("clear the old target-aside folder");
            }
            moved(&target, &aside);
        }
        Ok(_) => fs::remove_file(&target).expect("remove the old plain-file target"),
        Err(_) => {}
    }
    fs::write(&target, PLAIN_TARGET).expect("make the default target a plain file");
}

/// The bytes of the plain file that stands in for the default `target`.
const PLAIN_TARGET: &[u8] = b"t-139: the default build output is a plain file\n";

/// `None` while the copy's default `target` is the plain file the check
/// wrote. Otherwise a run built in the repository's shared output (Cargo
/// renames a fresh folder over the file): the finding, and the plain file is
/// written again so the next runs are judged on their own.
fn plain_target_kept(shared: &Shared, lane: &str, when: &str) -> Option<String> {
    let target = shared.copy.join("target");
    let found = match fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.is_dir() => {
            let mut names: Vec<String> = fs::read_dir(&target)
                .map(|entries| {
                    entries
                        .flatten()
                        .map(|entry| entry.file_name().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default();
            names.sort();
            let mut attempt = 0;
            while let Err(error) = fs::remove_dir_all(&target) {
                attempt += 1;
                assert!(
                    attempt < 40,
                    "clear the folder at {}: {error}",
                    target.display()
                );
                std::thread::sleep(Duration::from_millis(500));
            }
            format!("a folder holding {}", names.join(" "))
        }
        Ok(_) => match fs::read(&target) {
            Ok(bytes) if bytes == PLAIN_TARGET => return None,
            Ok(bytes) => format!("a file of {} other bytes", bytes.len()),
            Err(error) => format!("an unreadable file ({error})"),
        },
        Err(_) => "gone".to_owned(),
    };
    default_target_file(shared);
    Some(format!(
        "{lane}: {when}, the default target {} is no longer the plain file the check \
         wrote but {found}: a run built in the repository's shared build output",
        target.display()
    ))
}

/// Rename a folder, retrying while another program (an indexer or a virus
/// scanner) briefly holds a file inside it.
fn moved(from: &Path, to: &Path) {
    let mut attempt = 0;
    loop {
        match fs::rename(from, to) {
            Ok(()) => return,
            Err(_) if attempt < 20 => {
                attempt += 1;
                std::thread::sleep(Duration::from_millis(500));
            }
            Err(error) => panic!("move {} to {}: {error}", from.display(), to.display()),
        }
    }
}

/// Each lane's standalone verdict in the copy; every one must pass.
fn standalone(shared: &Shared) -> Arc<Vec<(String, Run)>> {
    default_target_folder(shared);
    let controls = CONTROLS.observe(&shared.copy, "standalone", || {
        LANES
            .iter()
            .map(|lane| {
                (
                    (*lane).to_owned(),
                    Started::new(shared, lane, "alone").finish(),
                )
            })
            .collect()
    });
    let failed: Vec<String> = controls
        .iter()
        .filter(|(_, run)| run.verdict != "pass")
        .map(|(lane, run)| format!("  {lane} reads {}{}", run.verdict, run.excerpt))
        .collect();
    assert!(
        failed.is_empty(),
        "control: every lane must pass alone in the scratch copy {}, or the check proves \
         nothing:\n{}",
        shared.copy.display(),
        failed.join("\n")
    );
    controls
}

fn drop_cargo_variables(command: &mut Command) {
    for (key, _) in std::env::vars_os() {
        if key
            .to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("CARGO_")
        {
            command.env_remove(&key);
        }
    }
}

/// A lane's verdict and the lines that explain a red one.
struct Run {
    verdict: String,
    excerpt: String,
}

fn judged(status: Option<ExitStatus>, text: &str) -> Run {
    let verdict = match status {
        None => format!("timed out after {}s", RUN_LIMIT.as_secs()),
        Some(status) if status.success() => "pass".to_owned(),
        Some(status) => {
            let failed: Vec<&str> = text
                .lines()
                .filter_map(|line| {
                    line.trim_end()
                        .strip_prefix("test ")?
                        .strip_suffix(" ... FAILED")
                })
                .collect();
            if failed.is_empty() {
                format!("red ({status}, no failing test named)")
            } else {
                format!("red ({})", failed.join(", "))
            }
        }
    };
    Run {
        verdict,
        excerpt: excerpt(text),
    }
}

/// Panic and error lines, indented under a failure.
fn excerpt(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut kept: Vec<String> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if kept.len() >= 12 {
            break;
        }
        if line.contains("panicked at") || line.trim_start().starts_with("error") {
            kept.extend(
                lines
                    .iter()
                    .skip(index)
                    .take(3)
                    .map(|line| clip(line.trim_end(), 220)),
            );
        }
    }
    kept.dedup();
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

/// One lane run in its own process, as the sweep runs a crate, its merged
/// output in a log file.
struct Started {
    child: Child,
    log: PathBuf,
    status: Option<ExitStatus>,
}

enum Waiting {
    Named(String),
    Ended,
    Silent,
}

impl Started {
    fn new(shared: &Shared, lane: &str, label: &str) -> Self {
        let log = fresh_path(shared, &format!("log-{lane}-{label}"));
        let file =
            File::create(&log).unwrap_or_else(|error| panic!("create {}: {error}", log.display()));
        let also = file
            .try_clone()
            .unwrap_or_else(|error| panic!("share {}: {error}", log.display()));
        let directory = lane_dir(shared, lane);
        let mut command = support::command("cargo", &directory);
        drop_cargo_variables(&mut command);
        command
            .args(["test", "--offline", "--target-dir"])
            .arg(directory.join("target"))
            .stdout(Stdio::from(also))
            .stderr(Stdio::from(file));
        let child = command
            .spawn()
            .unwrap_or_else(|error| panic!("start {lane}'s cargo test: {error}"));
        Self {
            child,
            log,
            status: None,
        }
    }

    fn text(&self) -> String {
        fs::read(&self.log)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default()
    }

    fn ended(&mut self) -> bool {
        if self.status.is_none() {
            self.status = self
                .child
                .try_wait()
                .unwrap_or_else(|error| panic!("watch a lane run: {error}"));
        }
        self.status.is_some()
    }

    fn wait_line(&mut self, lane: &str, limit: Duration) -> Waiting {
        let deadline = Instant::now() + limit;
        loop {
            let ended = self.ended();
            if let Some(named) = wait_line(&self.text(), lane) {
                return Waiting::Named(named);
            }
            if ended {
                return Waiting::Ended;
            }
            if Instant::now() >= deadline {
                return Waiting::Silent;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    fn finish(mut self) -> Run {
        let deadline = Instant::now() + RUN_LIMIT;
        while !self.ended() {
            if Instant::now() >= deadline {
                self.kill_tree();
                let _ = self.child.wait();
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        judged(self.status, &self.text())
    }

    /// Stop the run and the test executable it started: on Windows, killing
    /// `cargo test` leaves its test executable, which may hold a lane lock.
    fn kill_tree(&mut self) {
        if cfg!(windows) {
            let _ = support::command("taskkill", std::env::temp_dir())
                .args(["/T", "/F", "/PID"])
                .arg(self.child.id().to_string())
                .output();
        }
        let _ = self.child.kill();
    }
}

// --- the source inventory -----------------------------------------------------

struct Inventory {
    findings: Vec<String>,
    set_aside: Vec<String>,
    files: usize,
    lanes: usize,
}

fn inventory(root: &Path) -> Inventory {
    let text = fs::read_to_string(root.join(".ratmac").join("lanes.toml"))
        .expect("read the lane declaration");
    let value: toml::Value = text.parse().expect("the lane declaration is TOML");
    let roster: Vec<String> = value
        .get("roots")
        .and_then(|roots| roots.get("roster"))
        .and_then(toml::Value::as_array)
        .expect("the lane declaration has a roster")
        .iter()
        .map(|id| id.as_str().expect("a roster id is a string").to_owned())
        .collect();
    let mut inventory = Inventory {
        findings: Vec::new(),
        set_aside: Vec::new(),
        files: 0,
        lanes: 0,
    };
    for lane in roster.iter().filter(|lane| *lane != PORTS_LANE) {
        let files = lane_sources(root, lane);
        inventory.files += files.len();
        inventory.lanes += 1;
        for finding in Lane::new(&files).findings() {
            let shown = finding.shown(lane, &files);
            if (finding.file.as_str(), finding.function.as_str()) == (EXCEPTION.0, EXCEPTION.1) {
                inventory
                    .set_aside
                    .push(format!("{shown}\n      ({})", EXCEPTION.2));
            } else {
                inventory.findings.push(shown);
            }
        }
    }
    inventory
}

/// A lane's Rust sources by repository-relative path, never under a `target`
/// folder or the frozen corpus.
fn lane_sources(root: &Path, lane: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut pending = vec![root.join("test-hidden").join(lane)];
    while let Some(next) = pending.pop() {
        let entries =
            fs::read_dir(&next).unwrap_or_else(|error| panic!("list {}: {error}", next.display()));
        for entry in entries.flatten() {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                if entry.file_name() != "target"
                    && !format!("{relative}/").starts_with(FROZEN_CORPUS)
                {
                    pending.push(path);
                }
            } else if relative.ends_with(".rs") {
                let bytes = fs::read(&path)
                    .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
                found.push((relative, String::from_utf8_lossy(&bytes).into_owned()));
            }
        }
    }
    found.sort();
    found
}

/// One site, at the line where the repository root enters it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Finding {
    file: String,
    line: usize,
    function: String,
    kind: String,
    through: Option<(String, usize)>,
}

impl Finding {
    fn shown(&self, lane: &str, files: &[(String, String)]) -> String {
        let text = files
            .iter()
            .find(|(path, _)| *path == self.file)
            .and_then(|(_, text)| text.lines().nth(self.line - 1))
            .unwrap_or_default();
        let through = match &self.through {
            Some((file, line)) if *file == self.file => format!(" (the Cargo run at line {line})"),
            Some((file, line)) => format!(" (the Cargo run at {file}:{line})"),
            None => String::new(),
        };
        format!(
            "  {lane}: {}:{} in {}: {}{through}: {}",
            self.file,
            self.line,
            self.function,
            self.kind,
            clip(text.trim(), 140)
        )
    }
}

/// The matcher on known shapes first: each control is one lane source and
/// exactly the (line, kind) findings it must yield.
fn inventory_controls() {
    type Control = (&'static str, &'static [(usize, &'static str)]);
    let controls: &[Control] = &[
        // today's Engine: a call and a function value count; a use item, a
        // comment, and a string do not.
        (
            "use ratmac_qa::baseline::{self, today_engine};\n// today_engine() builds it\nfn a() {\n    let engine = baseline::today_engine();\n    std::thread::spawn(today_engine);\n    let text = \"today_engine()\";\n}\n",
            &[(4, "today_engine()"), (5, "today_engine()")],
        ),
        // built_executable on the repository through a binding ...
        (
            "fn a() {\n    let root = repo_root();\n    let rtm = built_executable(&root, &[\"--bin\", \"rtm\"], \"rtm\");\n}\n",
            &[(3, "built_executable")],
        ),
        // ... a lane function, or CARGO_MANIFEST_DIR ...
        (
            "fn repository() -> PathBuf {\n    Path::new(env!(\"CARGO_MANIFEST_DIR\")).join(\"../..\")\n}\nfn a() {\n    let path = built_executable(&repository(), &[], \"edition-cut\");\n    let up = Path::new(env!(\"CARGO_MANIFEST_DIR\")).ancestors().nth(2).unwrap();\n    let cut = built_executable(up, &[], \"edition-cut\");\n}\n",
            &[(5, "built_executable"), (7, "built_executable")],
        ),
        // ... but not on a scratch copy in an owned tree: the latest binding
        // decides.
        (
            "fn a() {\n    let tree = TempTree::new(\"copy\").unwrap();\n    let root = repo_root();\n    copy_repository(&root, &tree.join(\"copy\"));\n    let root = tree.join(\"copy\");\n    let built = built_executable(&root, &[], \"rtm-qa\");\n}\n",
            &[],
        ),
        // A lane function returning a repository path is a root; one that
        // copies the repository into an owned tree and returns the copy is
        // not, though its body reads the repository.
        (
            "fn worktree() -> PathBuf {\n    let root = repo_root();\n    root\n}\nfn scratch(tree: &TempTree) -> PathBuf {\n    let source = worktree();\n    let repo = tree.join(\"repo\");\n    for name in listed(&source) {\n        copy(&source.join(&name), &repo.join(&name));\n    }\n    repo\n}\nfn a() {\n    let tree = TempTree::new(\"t\").unwrap();\n    let repo = scratch(&tree);\n    let _ = built_executable(&repo, &[], \"rtm\");\n    let _ = built_executable(&worktree(), &[], \"rtm\");\n}\n",
            &[(17, "built_executable")],
        ),
        // cargo test on a lane crate counts; a subcommand that does not build,
        // another program, and a built binary do not.
        (
            "fn a() {\n    let root = repo_root();\n    let output = support::command(\"cargo\", root.join(\"test-hidden/t-094\"))\n        .args([\"test\", \"--offline\"])\n        .output();\n    let meta = support::command(\"cargo\", &root).args([\"metadata\", \"--no-deps\"]).output();\n    let git = support::command(\"git\", &root).args([\"status\"]).output();\n    let rtm = support::command(env!(\"CARGO_BIN_EXE_rtm\"), &root).arg(\"test\").output();\n}\n",
            &[(3, "cargo test")],
        ),
        // A root parameter is judged at each caller, through two hops, and
        // the finding names the caller's line.
        (
            "fn cargo() -> OsString {\n    std::env::var_os(\"CARGO\").unwrap_or_else(|| \"cargo\".into())\n}\nfn report_with(root: &Path, args: &[&str]) -> Output {\n    let mut command = Command::new(cargo());\n    command.arg(\"build\").args(args).current_dir(root);\n    command.output().unwrap()\n}\nfn report(root: &Path, args: &[&str]) -> Output {\n    report_with(root, args)\n}\nfn child() {\n    let _ = report(&repo_root(), &[\"--offline\"]);\n}\nfn fixture() {\n    let tree = TempTree::new(\"f\").unwrap();\n    let _ = report(&tree.join(\"copy\"), &[\"--offline\"]);\n}\n",
            &[(13, "cargo build")],
        ),
        // A caller adds the subcommand; Cargo with no folder named runs on the
        // lane's own crate.
        (
            "fn run_lane(directory: &Path) -> Output {\n    let mut command = support::command(\"cargo\", directory);\n    command.output().unwrap()\n}\nfn a() {\n    run_lane(&repo_root().join(\"test-hidden\").join(\"t-116\"));\n    let lane = TempTree::new(\"lane\").unwrap();\n    run_lane(&lane.join(\"t-116\"));\n    let own = Command::new(\"cargo\").args([\"test\", \"--offline\"]).output();\n}\n",
            &[(6, "cargo"), (9, "cargo test")],
        ),
        // The shared locked-folder helper's methods are not sites.
        (
            "fn a() {\n    let lane = lane_folder(\"t-086\", env!(\"CARGO_TARGET_TMPDIR\"));\n    let engine = lane.engine();\n    let rtm = lane.build(&repo_root(), &[\"--bin\", \"rtm\"], \"rtm\");\n    let output = lane.test(repo_root().join(\"test-hidden/t-094\")).output();\n    let comparison = lane.cargo(\"build\", &repo_root()).output();\n}\n",
            &[],
        ),
    ];
    for (number, (source, expected)) in controls.iter().enumerate() {
        let found: Vec<(usize, String)> =
            Lane::new(&[("src/lib.rs".to_owned(), (*source).to_owned())])
                .findings()
                .into_iter()
                .map(|finding| (finding.line, finding.kind))
                .collect();
        let expected: Vec<(usize, String)> = expected
            .iter()
            .map(|(line, kind)| (*line, (*kind).to_owned()))
            .collect();
        assert_eq!(
            found, expected,
            "SVEV-007 inventory control {number}: the matcher must yield exactly these findings \
             for:\n{source}"
        );
    }
}

// --- Rust source, enough to find sites ----------------------------------------

/// One token of Rust source: comments are dropped, string literals decoded
/// enough to compare, and character literals and lifetimes never open a
/// string.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Text(String),
    Mark(char),
}

type Tokens = [(usize, Token)];

fn rust_tokens(source: &str) -> Vec<(usize, Token)> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut line = 1;
    let mut at = 0;
    while let Some(&character) = chars.get(at) {
        let next = chars.get(at + 1).copied();
        if character == '\n' {
            line += 1;
            at += 1;
        } else if character.is_whitespace() {
            at += 1;
        } else if character == '/' && next == Some('/') {
            while chars.get(at).is_some_and(|c| *c != '\n') {
                at += 1;
            }
        } else if character == '/' && next == Some('*') {
            let mut depth = 0usize;
            while let Some(&c) = chars.get(at) {
                if c == '/' && chars.get(at + 1) == Some(&'*') {
                    depth += 1;
                    at += 2;
                } else if c == '*' && chars.get(at + 1) == Some(&'/') {
                    depth -= 1;
                    at += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    line += usize::from(c == '\n');
                    at += 1;
                }
            }
        } else if let Some((text, end)) = literal(&chars, at) {
            tokens.push((line, Token::Text(text)));
            line += chars[at..end].iter().filter(|c| **c == '\n').count();
            at = end;
        } else if character == '\'' {
            at = past_quote(&chars, at);
        } else if character.is_alphanumeric() || character == '_' {
            let raw = character == 'r'
                && next == Some('#')
                && chars
                    .get(at + 2)
                    .is_some_and(|c| c.is_alphabetic() || *c == '_');
            let start = if raw { at + 2 } else { at };
            let mut end = start;
            while chars
                .get(end)
                .is_some_and(|c| c.is_alphanumeric() || *c == '_')
            {
                end += 1;
            }
            tokens.push((line, Token::Word(chars[start..end].iter().collect())));
            at = end;
        } else {
            tokens.push((line, Token::Mark(character)));
            at += 1;
        }
    }
    tokens
}

/// A string literal starting at `at` (plain, byte, C, or raw): its text and
/// the index past it.
fn literal(chars: &[char], at: usize) -> Option<(String, usize)> {
    let mut cursor = at;
    if matches!(chars.get(cursor), Some('b' | 'c')) {
        cursor += 1;
    }
    let raw = chars.get(cursor) == Some(&'r');
    let mut hashes = 0;
    if raw {
        cursor += 1;
        while chars.get(cursor) == Some(&'#') {
            hashes += 1;
            cursor += 1;
        }
    }
    if chars.get(cursor) != Some(&'"') {
        return None;
    }
    cursor += 1;
    let mut text = String::new();
    while let Some(&c) = chars.get(cursor) {
        if raw {
            if c == '"' && (0..hashes).all(|offset| chars.get(cursor + 1 + offset) == Some(&'#')) {
                return Some((text, cursor + 1 + hashes));
            }
            text.push(c);
            cursor += 1;
        } else if c == '\\' {
            match chars.get(cursor + 1) {
                Some('n') => text.push('\n'),
                Some('t') => text.push('\t'),
                Some('\n') | None => {}
                Some(&other) => text.push(other),
            }
            cursor += 2;
        } else if c == '"' {
            return Some((text, cursor + 1));
        } else {
            text.push(c);
            cursor += 1;
        }
    }
    Some((text, cursor))
}

/// Past a character literal at `at`, or just past the quote of a lifetime or
/// label.
fn past_quote(chars: &[char], at: usize) -> usize {
    match (chars.get(at + 1), chars.get(at + 2)) {
        (Some('\\'), _) => {
            let mut cursor = at + 3;
            while chars.get(cursor).is_some_and(|c| *c != '\'' && *c != '\n') {
                cursor += 1;
            }
            cursor + 1
        }
        (Some(_), Some('\'')) => at + 3,
        _ => at + 1,
    }
}

fn word_is(tokens: &Tokens, at: usize, word: &str) -> bool {
    matches!(tokens.get(at), Some((_, Token::Word(found))) if found == word)
}

fn mark_is(tokens: &Tokens, at: usize, mark: char) -> bool {
    matches!(tokens.get(at), Some((_, Token::Mark(found))) if *found == mark)
}

fn mark_before(tokens: &Tokens, at: usize, mark: char) -> bool {
    at > 0 && mark_is(tokens, at - 1, mark)
}

/// The index of the bracket closing the one at `open`.
fn closing(tokens: &Tokens, open: usize) -> usize {
    let mut depth = 0usize;
    for (index, (_, token)) in tokens.iter().enumerate().skip(open) {
        match token {
            Token::Mark('(' | '[' | '{') => depth += 1,
            Token::Mark(')' | ']' | '}') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return index;
                }
            }
            _ => {}
        }
    }
    tokens.len()
}

/// The index where the statement starting at `from` ends: its `;`, or the
/// bracket closing the block it sits in.
fn statement_end(tokens: &Tokens, from: usize, limit: usize) -> usize {
    let mut depth = 0i32;
    for (index, (_, token)) in tokens.iter().enumerate().take(limit).skip(from) {
        match token {
            Token::Mark('(' | '[' | '{') => depth += 1,
            Token::Mark(')' | ']' | '}') => {
                depth -= 1;
                if depth < 0 {
                    return index;
                }
            }
            Token::Mark(';') if depth == 0 => return index,
            _ => {}
        }
    }
    limit
}

/// Words that start a statement ending at its block's closing brace.
const BLOCK_STATEMENTS: [&str; 6] = ["for", "while", "loop", "if", "match", "unsafe"];

/// The expressions a function's value comes from: each `return` expression
/// and the tail after the body's last statement. A block statement (`for`,
/// `while`, `loop`, `if` with no `else` next, `match`, `unsafe`, or a bare
/// block) ends at its closing brace when more of the body follows.
fn returned(tokens: &Tokens, body: &Range<usize>) -> Vec<Range<usize>> {
    let inner = body.start + 1..body.end.saturating_sub(1).max(body.start + 1);
    let mut found: Vec<Range<usize>> = inner
        .clone()
        .filter(|index| word_is(tokens, *index, "return"))
        .map(|index| index + 1..statement_end(tokens, index + 1, inner.end))
        .collect();
    let mut start = inner.start;
    let mut depth = 0i32;
    for index in inner.clone() {
        match &tokens[index].1 {
            Token::Mark('(' | '[' | '{') => depth += 1,
            Token::Mark(closer @ (')' | ']' | '}')) => {
                depth -= 1;
                let block = mark_is(tokens, start, '{')
                    || BLOCK_STATEMENTS
                        .iter()
                        .any(|word| word_is(tokens, start, word));
                let continued = word_is(tokens, index + 1, "else")
                    || mark_is(tokens, index + 1, '.')
                    || mark_is(tokens, index + 1, '?');
                if depth == 0 && *closer == '}' && block && !continued && index + 1 < inner.end {
                    start = index + 1;
                }
            }
            Token::Mark(';') if depth == 0 => start = index + 1,
            _ => {}
        }
    }
    if start < inner.end {
        found.push(start..inner.end);
    }
    found
}

/// Comma-separated parts at depth zero, as ranges into `tokens`; `angles`
/// also nests `<..>` (skipping `->`), for parameter types.
fn split_top(tokens: &Tokens, angles: bool) -> Vec<Range<usize>> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    let mut index = 0;
    while index < tokens.len() {
        match &tokens[index].1 {
            Token::Mark('-') if angles && mark_is(tokens, index + 1, '>') => index += 1,
            Token::Mark('(' | '[' | '{') => depth += 1,
            Token::Mark(')' | ']' | '}') => depth -= 1,
            Token::Mark('<') if angles => depth += 1,
            Token::Mark('>') if angles => depth -= 1,
            Token::Mark(',') if depth == 0 => {
                parts.push(start..index);
                start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }
    if start < tokens.len() {
        parts.push(start..tokens.len());
    }
    parts
}

/// The arguments of the call whose `(` is at `open`, and its `)`.
fn call_args(tokens: &Tokens, open: usize) -> (Vec<Range<usize>>, usize) {
    let close = closing(tokens, open).min(tokens.len());
    let inner = &tokens[(open + 1).min(close)..close];
    let arguments = split_top(inner, false)
        .into_iter()
        .map(|part| part.start + open + 1..part.end + open + 1)
        .collect();
    (arguments, close)
}

/// One function item: its name, parameter names in order, body tokens, and
/// the words of its return type.
struct Function {
    name: String,
    params: Vec<String>,
    body: Range<usize>,
    returns: Vec<String>,
}

fn functions(tokens: &Tokens) -> Vec<Function> {
    let mut found = Vec::new();
    for index in 0..tokens.len() {
        if !word_is(tokens, index, "fn") {
            continue;
        }
        let Some((_, Token::Word(name))) = tokens.get(index + 1) else {
            continue;
        };
        let mut at = index + 2;
        if mark_is(tokens, at, '<') {
            let mut depth = 0i32;
            while at < tokens.len() {
                if mark_is(tokens, at, '-') && mark_is(tokens, at + 1, '>') {
                    at += 2;
                    continue;
                }
                if mark_is(tokens, at, '<') {
                    depth += 1;
                } else if mark_is(tokens, at, '>') {
                    depth -= 1;
                    if depth == 0 {
                        at += 1;
                        break;
                    }
                }
                at += 1;
            }
        }
        if !mark_is(tokens, at, '(') {
            continue;
        }
        let close = closing(tokens, at).min(tokens.len());
        let inner = &tokens[at + 1..close.max(at + 1)];
        let params = split_top(inner, true)
            .into_iter()
            .map(|part| {
                let part = &inner[part];
                let colon = part
                    .iter()
                    .position(|(_, token)| *token == Token::Mark(':'))
                    .unwrap_or(part.len());
                part[..colon]
                    .iter()
                    .rev()
                    .find_map(|(_, token)| match token {
                        Token::Word(word) if word != "mut" => Some(word.clone()),
                        _ => None,
                    })
                    .unwrap_or_default()
            })
            .collect();
        let mut cursor = close + 1;
        let mut returns = Vec::new();
        while let Some((_, token)) = tokens.get(cursor) {
            match token {
                Token::Mark('{' | ';') => break,
                Token::Word(word) => returns.push(word.clone()),
                _ => {}
            }
            cursor += 1;
        }
        if !mark_is(tokens, cursor, '{') {
            continue;
        }
        let end = (closing(tokens, cursor) + 1).min(tokens.len());
        found.push(Function {
            name: name.clone(),
            params,
            body: cursor..end,
            returns,
        });
    }
    found
}

/// One lane source file, tokenized.
struct Source {
    path: String,
    tokens: Vec<(usize, Token)>,
    functions: Vec<Function>,
    in_use: Vec<bool>,
}

impl Source {
    fn new(path: &str, text: &str) -> Self {
        let tokens = rust_tokens(text);
        let mut in_use = vec![false; tokens.len()];
        let mut index = 0;
        while index < tokens.len() {
            if word_is(&tokens, index, "use") {
                while index < tokens.len() && !mark_is(&tokens, index, ';') {
                    in_use[index] = true;
                    index += 1;
                }
            }
            index += 1;
        }
        let functions = functions(&tokens);
        Self {
            path: path.to_owned(),
            tokens,
            functions,
            in_use,
        }
    }

    /// The innermost function whose body holds token `at`.
    fn enclosing(&self, at: usize) -> Option<usize> {
        self.functions
            .iter()
            .enumerate()
            .filter(|(_, function)| function.body.start < at && at < function.body.end)
            .max_by_key(|(_, function)| function.body.start)
            .map(|(index, _)| index)
    }

    /// The latest `let name = value;` in `function` before `before`: the
    /// `let`'s index and the value's tokens.
    fn binding(&self, function: usize, name: &str, before: usize) -> Option<(usize, Range<usize>)> {
        let tokens = &self.tokens;
        let body = &self.functions[function].body;
        let mut index = before.min(body.end);
        while index > body.start {
            index -= 1;
            if !word_is(tokens, index, "let") {
                continue;
            }
            let mut at = index + 1;
            if word_is(tokens, at, "mut") {
                at += 1;
            }
            if !word_is(tokens, at, name)
                || !(mark_is(tokens, at + 1, '=') || mark_is(tokens, at + 1, ':'))
            {
                continue;
            }
            let mut equals = at + 1;
            while equals < body.end
                && !mark_is(tokens, equals, ';')
                && !(mark_is(tokens, equals, '=')
                    && !mark_is(tokens, equals + 1, '=')
                    && !mark_is(tokens, equals + 1, '>'))
            {
                equals += 1;
            }
            if !mark_is(tokens, equals, '=') {
                continue;
            }
            return Some((
                index,
                equals + 1..statement_end(tokens, equals + 1, body.end),
            ));
        }
        None
    }

    /// The name a call at `at` is bound to by its statement's `let`.
    fn bound_name(&self, at: usize) -> Option<String> {
        let tokens = &self.tokens;
        let mut index = at;
        while index > 0 {
            index -= 1;
            match &tokens[index].1 {
                Token::Mark(';' | '{' | '}') => return None,
                Token::Word(word) if word == "let" => {
                    let at = index + 1 + usize::from(word_is(tokens, index + 1, "mut"));
                    return match tokens.get(at) {
                        Some((_, Token::Word(name))) => Some(name.clone()),
                        _ => None,
                    };
                }
                _ => {}
            }
        }
        None
    }

    /// The arguments of the first `.method(..)` in the chain starting at
    /// `at`.
    fn chained(&self, mut at: usize, methods: &[&str]) -> Option<Range<usize>> {
        let tokens = &self.tokens;
        while mark_is(tokens, at, '.') {
            let Some((_, Token::Word(method))) = tokens.get(at + 1) else {
                return None;
            };
            if !mark_is(tokens, at + 2, '(') {
                at += 2;
                continue;
            }
            let close = closing(tokens, at + 2);
            if methods.contains(&method.as_str()) {
                return Some(at + 3..close);
            }
            at = close + 1;
            while mark_is(tokens, at, '?') {
                at += 1;
            }
        }
        None
    }

    /// The arguments of the first `.method(..)` applied to the value the call
    /// at `at` (closing at `close`) makes: in its own chain, or in a later
    /// statement starting with the name it is bound to.
    fn method_arguments(&self, at: usize, close: usize, methods: &[&str]) -> Option<Range<usize>> {
        if let Some(found) = self.chained(close + 1, methods) {
            return Some(found);
        }
        let name = self.bound_name(at)?;
        let tokens = &self.tokens;
        let limit = self
            .enclosing(at)
            .map_or(tokens.len(), |function| self.functions[function].body.end);
        (close + 1..limit)
            .filter(|index| {
                word_is(tokens, *index, &name)
                    && mark_is(tokens, index + 1, '.')
                    && (mark_before(tokens, *index, ';')
                        || mark_before(tokens, *index, '{')
                        || mark_before(tokens, *index, '}'))
            })
            .find_map(|index| self.chained(index + 1, methods))
    }
}

/// Where a root expression leads.
enum Root {
    Repository,
    /// (file, function, parameter position)
    Parameter(usize, usize, usize),
    Other,
}

/// How a site's root is found.
enum SiteRoot {
    Direct,
    Expression(Range<usize>),
    OwnCrate,
}

/// One lane's sources and the lane functions that return a repository path
/// or the Cargo program.
struct Lane {
    sources: Vec<Source>,
    roots: BTreeSet<String>,
    cargos: BTreeSet<String>,
}

impl Lane {
    fn new(files: &[(String, String)]) -> Self {
        let sources: Vec<Source> = files
            .iter()
            .map(|(path, text)| Source::new(path, text))
            .collect();
        let cargos = sources
            .iter()
            .flat_map(|source| {
                source.functions.iter().filter(|function| {
                    let named = function.returns.iter().any(|word| {
                        matches!(word.as_str(), "OsString" | "OsStr" | "String" | "str" | "PathBuf" | "Path")
                    });
                    let command = function.returns.iter().any(|word| word == "Command" || word == "Output");
                    named
                        && !command
                        && source.tokens[function.body.clone()]
                            .iter()
                            .any(|(_, token)| matches!(token, Token::Text(text) if text == "cargo" || text == "CARGO"))
                })
            })
            .map(|function| function.name.clone())
            .collect();
        let mut lane = Self {
            sources,
            roots: BTreeSet::from(["repo_root".to_owned()]),
            cargos,
        };
        loop {
            let found = lane.root_functions();
            if found.is_subset(&lane.roots) {
                break;
            }
            lane.roots.extend(found);
        }
        lane
    }

    /// Every lane function that returns a repository path: its return type
    /// names `Path` or `PathBuf`, and its tail or a `return` expression
    /// reaches a root the way a site's root expression does.
    fn root_functions(&self) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        for (file, source) in self.sources.iter().enumerate() {
            for function in &source.functions {
                let path = function
                    .returns
                    .iter()
                    .any(|word| word == "Path" || word == "PathBuf");
                if !path {
                    continue;
                }
                let reaches = returned(&source.tokens, &function.body)
                    .into_iter()
                    .any(|value| {
                        let at = value.start;
                        matches!(self.root_of(file, value, at, 0), Root::Repository)
                    });
                if reaches {
                    found.insert(function.name.clone());
                }
            }
        }
        found
    }

    fn findings(&self) -> BTreeSet<Finding> {
        let mut found = BTreeSet::new();
        for (file, source) in self.sources.iter().enumerate() {
            for at in 0..source.tokens.len() {
                let Some((kind, root)) = self.site(file, at) else {
                    continue;
                };
                let origins = match root {
                    SiteRoot::Direct | SiteRoot::OwnCrate => vec![(file, at)],
                    SiteRoot::Expression(expression) => self.origins(file, expression, at, 0),
                };
                for (origin_file, origin_at) in origins {
                    let origin = &self.sources[origin_file];
                    let through = ((origin_file, origin_at) != (file, at))
                        .then(|| (source.path.clone(), source.tokens[at].0));
                    found.insert(Finding {
                        file: origin.path.clone(),
                        line: origin.tokens[origin_at].0,
                        function: origin
                            .enclosing(origin_at)
                            .map(|function| origin.functions[function].name.clone())
                            .unwrap_or_default(),
                        kind: kind.clone(),
                        through,
                    });
                }
            }
        }
        found
    }

    /// The site that token `at` starts, if any: its kind and how its root is
    /// found.
    fn site(&self, file: usize, at: usize) -> Option<(String, SiteRoot)> {
        let source = &self.sources[file];
        let tokens = &source.tokens;
        let Token::Word(word) = &tokens[at].1 else {
            return None;
        };
        if source.in_use[at] || (at > 0 && word_is(tokens, at - 1, "fn")) {
            return None;
        }
        let called = mark_is(tokens, at + 1, '(') && !mark_before(tokens, at, '.');
        match word.as_str() {
            "today_engine" => Some(("today_engine()".to_owned(), SiteRoot::Direct)),
            "built_executable" if called => {
                let (arguments, _) = call_args(tokens, at + 1);
                let root = arguments.first()?.clone();
                Some(("built_executable".to_owned(), SiteRoot::Expression(root)))
            }
            "command" if called => {
                let (arguments, close) = call_args(tokens, at + 1);
                if arguments.len() != 2 || !self.is_cargo(file, arguments[0].clone(), at, 0) {
                    return None;
                }
                let kind = self.cargo_kind(file, at, close)?;
                Some((kind, SiteRoot::Expression(arguments[1].clone())))
            }
            "Command"
                if mark_is(tokens, at + 1, ':')
                    && mark_is(tokens, at + 2, ':')
                    && word_is(tokens, at + 3, "new")
                    && mark_is(tokens, at + 4, '(') =>
            {
                let (arguments, close) = call_args(tokens, at + 4);
                if arguments.len() != 1 || !self.is_cargo(file, arguments[0].clone(), at, 0) {
                    return None;
                }
                let kind = self.cargo_kind(file, at, close)?;
                match source.method_arguments(at, close, &["current_dir"]) {
                    Some(root) => Some((kind, SiteRoot::Expression(root))),
                    None => Some((kind, SiteRoot::OwnCrate)),
                }
            }
            _ => None,
        }
    }

    /// `cargo <subcommand>` for a building subcommand, `cargo` when the
    /// caller adds it, and nothing for any other.
    fn cargo_kind(&self, file: usize, at: usize, close: usize) -> Option<String> {
        let source = &self.sources[file];
        let first = source
            .method_arguments(at, close, &["arg", "args"])
            .and_then(|arguments| {
                source.tokens[arguments]
                    .iter()
                    .find_map(|(_, token)| match token {
                        Token::Text(text) => Some(text.clone()),
                        _ => None,
                    })
            });
        match first {
            None => Some("cargo".to_owned()),
            Some(subcommand) if BUILDING.contains(&subcommand.as_str()) => {
                Some(format!("cargo {subcommand}"))
            }
            Some(_) => None,
        }
    }

    /// Whether `expression` (used at `at`) is the Cargo program.
    fn is_cargo(&self, file: usize, expression: Range<usize>, at: usize, depth: usize) -> bool {
        let source = &self.sources[file];
        let tokens = &source.tokens;
        expression.into_iter().any(|index| match &tokens[index].1 {
            Token::Text(text) => text == "cargo" || text == "CARGO",
            Token::Word(_) if mark_before(tokens, index, '.') => false,
            Token::Word(word) if mark_is(tokens, index + 1, '(') => self.cargos.contains(word),
            Token::Word(word) => {
                depth < DEPTH
                    && source
                        .enclosing(at)
                        .and_then(|function| source.binding(function, word, at))
                        .is_some_and(|(bound, value)| self.is_cargo(file, value, bound, depth + 1))
            }
            Token::Mark(_) => false,
        })
    }

    /// Where a root expression used at `at` leads.
    fn root_of(&self, file: usize, expression: Range<usize>, at: usize, depth: usize) -> Root {
        let source = &self.sources[file];
        let tokens = &source.tokens;
        let scope = source.enclosing(at);
        let mut parameter = Root::Other;
        for (offset, (_, token)) in tokens[expression.clone()].iter().enumerate() {
            let index = expression.start + offset;
            match token {
                Token::Text(text) if text == "CARGO_MANIFEST_DIR" => return Root::Repository,
                Token::Word(word) => {
                    if mark_before(tokens, index, '.') {
                        continue;
                    }
                    if mark_is(tokens, index + 1, '(') {
                        if self.roots.contains(word) {
                            return Root::Repository;
                        }
                        continue;
                    }
                    if mark_before(tokens, index, ':')
                        || mark_is(tokens, index + 1, ':')
                        || mark_is(tokens, index + 1, '!')
                    {
                        continue;
                    }
                    let Some(function) = scope else {
                        continue;
                    };
                    if let Some((bound, value)) = source.binding(function, word, at) {
                        if depth < DEPTH {
                            match self.root_of(file, value, bound, depth + 1) {
                                Root::Repository => return Root::Repository,
                                found @ Root::Parameter(..) => {
                                    if matches!(parameter, Root::Other) {
                                        parameter = found;
                                    }
                                }
                                Root::Other => {}
                            }
                        }
                    } else if let Some(position) = source.functions[function]
                        .params
                        .iter()
                        .position(|name| name == word)
                    {
                        if matches!(parameter, Root::Other) {
                            parameter = Root::Parameter(file, function, position);
                        }
                    }
                }
                _ => {}
            }
        }
        parameter
    }

    /// The (file, token) places where a repository root enters the root
    /// expression used at `at`.
    fn origins(
        &self,
        file: usize,
        expression: Range<usize>,
        at: usize,
        depth: usize,
    ) -> Vec<(usize, usize)> {
        match self.root_of(file, expression, at, depth) {
            Root::Repository => vec![(file, at)],
            Root::Parameter(defined, function, position) if depth < DEPTH => {
                self.callers(defined, function, position, depth + 1)
            }
            _ => Vec::new(),
        }
    }

    /// Origins through every free call in the lane of a function whose
    /// parameter `position` is a root.
    fn callers(
        &self,
        file: usize,
        function: usize,
        position: usize,
        depth: usize,
    ) -> Vec<(usize, usize)> {
        let callee = &self.sources[file].functions[function];
        let mut found = Vec::new();
        for (caller, source) in self.sources.iter().enumerate() {
            let tokens = &source.tokens;
            for at in 0..tokens.len() {
                if !word_is(tokens, at, &callee.name)
                    || !mark_is(tokens, at + 1, '(')
                    || mark_before(tokens, at, '.')
                    || (at > 0 && word_is(tokens, at - 1, "fn"))
                {
                    continue;
                }
                let (arguments, _) = call_args(tokens, at + 1);
                if arguments.len() != callee.params.len() {
                    continue;
                }
                found.extend(self.origins(caller, arguments[position].clone(), at, depth));
            }
        }
        found
    }
}
