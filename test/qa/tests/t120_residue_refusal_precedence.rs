//! t-120 / WEB-004: residue refusal precedence.
//!
//! WEBV-013: every command on the general usage `Commands:` line, and every
//! public path-taking library entry of the t-119 table, refuses retired-layout
//! residue before its own argument or request validation, naming the observed
//! artifact with forward slashes and its repair.
//! WEBV-014: the operation observer - clean positive controls log their
//! documented kinds, while residue refusals and pure usage responses leave the
//! operation log absent and every tree byte-identical.
//! WEBV-015: residue only at the invoking checkout, at the shared primary root
//! seen from a linked worktree, or at a valid explicit target still refuses
//! first even with malformed extra options; a missing or ambiguous target
//! inspects only the invoking checkout and then answers the ordinary usage
//! error.
//! WEBV-016: without residue, behavior and argument errors are unchanged; the
//! exact standalone help forms and unknown commands stay pure usage responses
//! even with residue, and a help token mixed with any other argument cannot
//! bypass preflight.

use ratmac::abandon::{self, AbandonRequest};
use ratmac::blocked::{self, HoldRequest};
use ratmac::cli;
use ratmac::contract::{self, ContractDefect};
use ratmac::machine::MachineClass;
use ratmac::{RespawnRequest, Scheduler, StepRequest};
use ratmac_qa::support::{self, TempTree};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::{Mutex, MutexGuard};

/// Where a feature build records one operation kind per line.
const OP_LOG_VAR: &str = "RATMAC_TEST_OPERATION_LOG";
/// In-process Engine test hooks this file must keep hidden while it works.
const ROOT_LOG_VAR: &str = "RATMAC_TEST_ROOT_LOG";
const REPEAT_VAR: &str = "RATMAC_TEST_ROOT_REPEAT";

/// The kinds the operation observer documents, one per line.
const KNOWN_KINDS: [&str; 7] = [
    "roster", "record", "ledger", "runbook", "blocker", "target", "lock",
];

/// Serializes every in-process use of this process's `RATMAC_TEST_*` hooks.
static LIBRARY_LANE: Mutex<()> = Mutex::new(());

fn library_lane() -> MutexGuard<'static, ()> {
    LIBRARY_LANE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

// --- the machine every fixture shares -----------------------------------------

const RUNBOOK: &str = r#"
[roots]
work = "work"

[classes.worker.bindings.ticket]
required = true

[classes.worker.states.work]
prompt = "Child works the bound ticket."

[classes.worker.states.finished]
prompt = "Child finished."

[[classes.worker.transitions]]
from = "work"
to = "finished"

[states.plan]
prompt = "Plan the cycle."

[states.delegate]
prompt = "Delegate and wait."

[[states.delegate.spawns]]
class = "worker"
name = "rev"
bind = ["ticket"]

[states.done]
prompt = "Done."

[[transitions]]
from = "plan"
to = "delegate"

[[transitions]]
from = "delegate"
to = "done"

[[transitions]]
from = "delegate"
to = "plan"
blocked-route = true
"#;

/// The library fixture's runbook: every fixed contract role declared, so the
/// `contract::*` entries find their roots and only residue can refuse them.
const LIBRARY_RUNBOOK: &str = r#"
[roots]
goal = "goal"
issue = "issue"
residual = "residual"
ticket = "ticket"
work = "work"

[classes.worker.bindings.ticket]
required = true

[classes.worker.states.work]
prompt = "Child works the bound ticket."

[classes.worker.states.finished]
prompt = "Child finished."

[[classes.worker.transitions]]
from = "work"
to = "finished"

[states.plan]
prompt = "Plan the cycle."

[states.delegate]
prompt = "Delegate and wait."

[[states.delegate.spawns]]
class = "worker"
name = "rev"
bind = ["ticket"]

[states.done]
prompt = "Done."

[[transitions]]
from = "plan"
to = "delegate"

[[transitions]]
from = "delegate"
to = "done"

[[transitions]]
from = "delegate"
to = "plan"
blocked-route = true
"#;

/// The same machine still written the pre-cutover way (top-level `phases`).
const PHASES_RUNBOOK: &str = r#"
[roots]
work = "work"

[phases.plan]
prompt = "Plan the cycle."

[phases.delegate]
prompt = "Delegate and wait."

[[transitions]]
from = "plan"
to = "delegate"
"#;

// --- residue shapes -----------------------------------------------------------

/// One retired-layout residue shape, exactly what the Engine's existing
/// residue check finds.
#[derive(Clone, Copy, Debug)]
enum Shape {
    /// A flat Run artifact at `<Engine root>/state.toml`.
    FlatState,
    /// One live pre-split artifact under `.arca/`.
    Presplit(&'static str),
    /// `.arca/rtm.lock` as a dangling link, where the platform allows one.
    DanglingLock,
    /// A pre-cutover runbook declaring a top-level `phases` table (RB111).
    PhasesRunbook,
    /// A pre-cutover Run Record at the old filename.
    RecordState,
    /// A pre-cutover Run Record still carrying the `phase` field.
    RecordPhase,
}

const RECORD_RUN: &str = "webv013-legacy";

/// One planted residue, restored exactly when dropped.
struct Planted {
    /// The refusal sentences this shape answers with, one per acceptable
    /// path spelling.
    sentences: Vec<String>,
    runbook_restore: Option<(PathBuf, Vec<u8>)>,
    clean: Vec<PathBuf>,
}

impl Drop for Planted {
    fn drop(&mut self) {
        if let Some((path, bytes)) = &self.runbook_restore {
            let _ = fs::write(path, bytes);
        }
        for path in &self.clean {
            if path.is_dir() {
                let _ = fs::remove_dir_all(path);
            } else {
                let _ = fs::remove_file(path);
            }
        }
    }
}

/// This path in the one spelling the Engine shows.
fn rendered(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// The canonical spelling, for engines that render resolved paths.
fn canonical_rendered(path: &Path) -> String {
    fs::canonicalize(path)
        .map(|canonical| rendered(&canonical))
        .unwrap_or_else(|_| rendered(path))
}

/// Every plausible rendering of `path`.
fn spellings(path: &Path) -> Vec<String> {
    vec![rendered(path), canonical_rendered(path)]
}

/// The refusal sentences for `shape` planted at `root`, in every acceptable
/// path spelling. The texts are the Engine's existing residue refusals.
fn residue_sentences(shape: Shape, root: &Path) -> Vec<String> {
    let shown = |parts: &[&str]| {
        let mut path = root.to_path_buf();
        for part in parts {
            path.push(part);
        }
        spellings(&path)
    };
    let pre_split = |paths: Vec<String>| {
        paths
            .into_iter()
            .map(|path| {
                format!(
                    "refusing to run: pre-split Engine residue {path} exists; migrate or remove \
                     it, then retry; nothing was modified"
                )
            })
            .collect()
    };
    match shape {
        Shape::FlatState => pre_split(shown(&[".ratmac", "state.toml"])),
        Shape::Presplit(artifact) => pre_split(shown(&artifact.split('/').collect::<Vec<_>>())),
        Shape::DanglingLock => pre_split(shown(&[".arca", "rtm.lock"])),
        Shape::PhasesRunbook => shown(&[".ratmac", "ratmac.toml"])
            .into_iter()
            .map(|path| {
                format!(
                    "refusing to run: RB111: the runbook {path} declares a pre-cutover \
                     \"phases\" table; rename it to \"states\" (and any \
                     [[phases.<name>.spawns]] to [[states.<name>.spawns]], \
                     [classes.<name>.phases] to [classes.<name>.states]), then retry; nothing \
                     was modified"
                )
            })
            .collect(),
        Shape::RecordState => shown(&[".ratmac", "runs", RECORD_RUN, "state.toml"])
            .into_iter()
            .map(|path| {
                format!(
                    "refusing to run: pre-cutover Run Record {path} exists; rename it to \
                     run.toml, then retry; nothing was modified"
                )
            })
            .collect(),
        Shape::RecordPhase => shown(&[".ratmac", "runs", RECORD_RUN, "run.toml"])
            .into_iter()
            .map(|path| {
                format!(
                    "refusing to run: pre-cutover Run Record {path} carries the field \
                     \"phase\"; rename that field to \"state\", then retry; nothing was \
                     modified"
                )
            })
            .collect(),
    }
}

/// Try to create a dangling link at `link`; `false` when the platform refuses.
fn try_dangling_link(link: &Path) -> bool {
    let _ = fs::create_dir_all(link.parent().expect("a link path has a parent"));
    #[cfg(unix)]
    let created = std::os::unix::fs::symlink("webv013-dangling-target", link);
    #[cfg(windows)]
    let created = std::os::windows::fs::symlink_file("webv013-dangling-target", link);
    match created {
        Ok(()) => true,
        Err(error) => {
            eprintln!(
                "t-120: this platform refused a dangling link ({}): {error}; noting the skip",
                rendered(link)
            );
            false
        }
    }
}

/// Plant `shape` at `root`; the returned value restores the tree when dropped.
fn plant(root: &Path, shape: Shape) -> Planted {
    let write = |parts: &[&str], contents: &str| {
        let mut path = root.to_path_buf();
        for part in parts {
            path.push(part);
        }
        fs::create_dir_all(path.parent().expect("planted path has a parent"))
            .unwrap_or_else(|error| panic!("create residue parent {}: {error}", rendered(&path)));
        fs::write(&path, contents)
            .unwrap_or_else(|error| panic!("plant residue {}: {error}", rendered(&path)));
        path
    };
    // When this plant creates `.arca` itself, restoring means removing the
    // whole tree: leftover empty directories are still residue.
    let arca = root.join(".arca");
    let fresh_arca = !arca.exists();
    let sentences = residue_sentences(shape, root);
    match shape {
        Shape::FlatState => Planted {
            sentences,
            runbook_restore: None,
            clean: vec![write(&[".ratmac", "state.toml"], "phase = \"plan\"\n")],
        },
        Shape::Presplit(".arca/runs") => Planted {
            sentences,
            runbook_restore: None,
            clean: if fresh_arca {
                write(
                    &[".arca", "runs", "r000001", "state.toml"],
                    "phase = \"plan\"\nstatus = \"planned\"\n",
                );
                vec![arca]
            } else {
                vec![write(
                    &[".arca", "runs", "r000001", "state.toml"],
                    "phase = \"plan\"\nstatus = \"planned\"\n",
                )]
            },
        },
        Shape::Presplit(artifact) => Planted {
            sentences,
            runbook_restore: None,
            clean: {
                let planted = write(
                    &artifact.split('/').collect::<Vec<_>>(),
                    "legacy engine residue\n",
                );
                if fresh_arca {
                    vec![arca]
                } else {
                    vec![planted]
                }
            },
        },
        Shape::DanglingLock => {
            let link = root.join(".arca").join("rtm.lock");
            assert!(
                try_dangling_link(&link),
                "the dangling-link row reached planting after the probe failed"
            );
            Planted {
                sentences,
                runbook_restore: None,
                clean: vec![root.join(".arca")],
            }
        }
        Shape::PhasesRunbook => {
            let runbook = root.join(".ratmac").join("ratmac.toml");
            let previous = fs::read(&runbook).unwrap_or_else(|error| {
                panic!("read {} to back it up: {error}", rendered(&runbook))
            });
            fs::write(&runbook, PHASES_RUNBOOK)
                .unwrap_or_else(|error| panic!("plant the phases runbook: {error}"));
            Planted {
                sentences,
                runbook_restore: Some((runbook, previous)),
                clean: Vec::new(),
            }
        }
        Shape::RecordState => Planted {
            sentences,
            runbook_restore: None,
            clean: vec![write(
                &[".ratmac", "runs", RECORD_RUN, "state.toml"],
                "phase = \"plan\"\n",
            )],
        },
        Shape::RecordPhase => Planted {
            sentences,
            runbook_restore: None,
            clean: vec![write(
                &[".ratmac", "runs", RECORD_RUN, "run.toml"],
                "   phase = \"plan\"\nstatus = \"executing\"\n",
            )],
        },
    }
}

// --- fixtures and shared runners ----------------------------------------------

/// A checkout-shaped project without Git: resolution falls back to the
/// checkout-local Engine root, which is enough for every invoking-checkout
/// and addressed-project row.
fn plain_project(label: &str, runbook: &str) -> TempTree {
    let tree = TempTree::new(&format!("t120-{label}")).expect("own a fixture tree");
    tree.write(".ratmac/ratmac.toml", runbook);
    tree.write(
        "work/blocker.txt",
        "Opaque blocker; its content is not a gate.\n",
    );
    tree.write("work/item.md", "Contributor-owned bytes.\n");
    tree
}

/// Run the harness Engine in `directory` with a scrubbed environment.
fn rtm(directory: &Path, args: &[&str]) -> Output {
    support::command(ratmac_qa::engine_bin!(), directory)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("invoke rtm {args:?}: {error}"))
}

/// Run the harness Engine with the operation observer armed at `log`.
fn rtm_observed(directory: &Path, log: &Path, args: &[&str]) -> Output {
    support::command(ratmac_qa::engine_bin!(), directory)
        .args(args)
        .env(OP_LOG_VAR, log)
        .output()
        .unwrap_or_else(|error| panic!("invoke rtm {args:?}: {error}"))
}

/// Every node below `root` as comparable bytes, links included.
fn tree_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    support::capture(root, support::CaptureOptions::default())
        .unwrap_or_else(|error| panic!("capture {}: {error}", rendered(root)))
        .tagged()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The run id a successful start minted, from its own report.
fn started_run(output: &Output) -> String {
    let text = stdout(output);
    let tail = text
        .lines()
        .find_map(|line| line.strip_prefix("rtm: started run "))
        .unwrap_or_else(|| panic!("a started run id follows the report: {text}"));
    tail.split([' ', '/'])
        .find(|word| !word.is_empty())
        .unwrap_or_else(|| panic!("a started run id follows the report: {text}"))
        .to_owned()
}

/// The refusal shape a binary row expects.
#[derive(Clone, Copy)]
enum Expect {
    /// Residue in the invoking checkout or its shared root: the error text is
    /// `<command>: <refusal>` behind `rtm: `, exit 1. `status` keeps its
    /// trailing `next:` line.
    Invoking {
        command: &'static str,
        trailing_next: bool,
    },
    /// Residue in doctor's, scaffold's, or skill's addressed project: the
    /// error text is the bare refusal behind `rtm: `, exit 2.
    Addressed,
    /// Residue in spawn's `--workspace`: `spawn: <refusal>` behind `rtm: `,
    /// exit 1.
    SpawnWorkspace,
}

impl Expect {
    fn exit_code(self) -> i32 {
        match self {
            Expect::Invoking { .. } | Expect::SpawnWorkspace => 1,
            Expect::Addressed => 2,
        }
    }

    fn first_line_prefix(self) -> Option<String> {
        match self {
            Expect::Invoking { command, .. } => Some(format!("{command}: ")),
            Expect::Addressed => None,
            Expect::SpawnWorkspace => Some("spawn: ".to_owned()),
        }
    }
}

/// Assert one binary residue refusal: exact exit code, empty stdout, the
/// refusal's first stderr line naming the artifact and its repair, and the
/// operation log absent.
fn assert_refusal(label: &str, output: &Output, expect: Expect, planted: &Planted, log: &Path) {
    assert!(
        output.stdout.is_empty(),
        "{label}: a residue refusal prints nothing on stdout: {}",
        stdout(output)
    );
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        expect.exit_code(),
        "{label}: the residue refusal exits {}; stderr: {}",
        expect.exit_code(),
        stderr(output)
    );
    let stderr = stderr(output);
    let lines: Vec<&str> = stderr.lines().collect();
    let expected_first: Vec<String> = planted
        .sentences
        .iter()
        .map(|sentence| match expect.first_line_prefix() {
            Some(prefix) => format!("rtm: {prefix}{sentence}"),
            None => format!("rtm: {sentence}"),
        })
        .collect();
    assert!(
        expected_first
            .iter()
            .any(|line| lines.first() == Some(&line.as_str())),
        "{label}: the refusal's first stderr line must name the artifact and its repair; \
         expected one of {expected_first:?}, held {lines:?}"
    );
    match expect {
        Expect::Invoking { trailing_next, .. } if trailing_next => {
            assert!(
                lines.iter().skip(1).any(|line| line.starts_with("next: ")),
                "{label}: this refusal keeps its trailing next: line; held {lines:?}"
            );
        }
        _ => {
            assert_eq!(
                lines.len(),
                1,
                "{label}: this refusal is exactly one stderr line; held {lines:?}"
            );
        }
    }
    assert!(
        !log.exists(),
        "{label}: the residue inspection is not an operation; {} must stay absent",
        rendered(log)
    );
}

/// One binary refusal row: plant, snapshot every involved checkout, invoke,
/// assert, compare snapshots, restore.
fn refuse_row(
    label: &str,
    invoke_from: &Path,
    args: &[&str],
    plant_at: &Path,
    shape: Shape,
    expect: Expect,
    logs: &TempTree,
    step: &mut usize,
) {
    *step += 1;
    let log = logs.join(format!("op-{:03}.log", step));
    let planted = plant(plant_at, shape);
    let mut trees: Vec<PathBuf> = Vec::new();
    for root in [invoke_from, plant_at] {
        if !trees.iter().any(|tree| tree == root) {
            trees.push(root.to_path_buf());
        }
    }
    let before: Vec<(PathBuf, BTreeMap<String, Vec<u8>>)> = trees
        .iter()
        .map(|root| (root.clone(), tree_snapshot(root)))
        .collect();
    let output = rtm_observed(invoke_from, &log, args);
    assert_refusal(label, &output, expect, &planted, &log);
    for (root, snapshot) in before {
        assert_eq!(
            tree_snapshot(&root),
            snapshot,
            "{label}: the refusal leaves {} byte-identical",
            rendered(&root)
        );
    }
    drop(planted);
}

// --- WEBV-013 -----------------------------------------------------------------

/// The commands the general usage's `Commands:` line lists - the one public
/// route table.
fn route_table_commands(label: &str, directory: &Path) -> Vec<String> {
    let output = rtm(directory, &["--help"]);
    assert!(
        output.status.success() && stderr(&output).is_empty(),
        "{label}: `rtm --help` is the pure general usage: {}{}",
        stdout(&output),
        stderr(&output)
    );
    let text = stdout(&output);
    let line = text
        .lines()
        .find(|line| line.starts_with("Commands:"))
        .unwrap_or_else(|| panic!("{label}: the general usage carries a `Commands:` line: {text}"));
    line["Commands:".len()..]
        .split(',')
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

/// One library row's bookkeeping: a fresh operation log per call.
struct LibraryProbe {
    logs: TempTree,
    next: usize,
}

impl LibraryProbe {
    fn fresh_log(&mut self) -> PathBuf {
        self.next += 1;
        self.logs.join(format!("library-op-{:03}.log", self.next))
    }

    /// One library row: residue planted at the inspected project, an outcome
    /// that must carry the refusal, no operation logged, tree identical.
    fn row(&mut self, root: &Path, shape: Shape, label: &str, body: impl FnOnce() -> String) {
        let log = self.fresh_log();
        let planted = plant(root, shape);
        let before = tree_snapshot(root);
        let text = body();
        assert!(
            planted
                .sentences
                .iter()
                .any(|sentence| text.contains(sentence)),
            "WEBV-013: {label} must answer the residue refusal before its own validation; \
             held: {text}"
        );
        assert!(
            !log.exists(),
            "WEBV-013: {label} inspects residue before any operation; the log must stay absent"
        );
        assert_eq!(
            tree_snapshot(root),
            before,
            "WEBV-013: {label} must not modify the project"
        );
        drop(planted);
    }
}

/// A contract defect names the project's checkout path and the refusal.
fn assert_defect_names_the_refusal(
    label: &str,
    defect: &ContractDefect,
    planted: &Planted,
    root: &Path,
) {
    let shown = rendered(root);
    let canonical = canonical_rendered(root);
    assert!(
        defect.artifact == shown || defect.artifact == canonical,
        "WEBV-013: {label}'s defect artifact is the project's checkout path with forward \
         slashes; held {:?}",
        defect.artifact
    );
    assert!(
        planted
            .sentences
            .iter()
            .any(|sentence| defect.reason.contains(sentence)),
        "WEBV-013: {label}'s defect reason carries the refusal; held {:?}",
        defect.reason
    );
}

/// WEBV-013's library half: one row per public path-taking entry, each with
/// residue planted at the project the entry inspects and an invalid request
/// value that must lose to the residue.
fn library_entries_refuse_residue_first() {
    let _lane = library_lane();
    let _hooks = hide_test_hooks();
    let mut probe = LibraryProbe {
        logs: TempTree::new("t120-013-library-op").expect("own the library log tree"),
        next: 0,
    };

    let project = plain_project("013-library", LIBRARY_RUNBOOK);
    project.write("goal/spec.md", "# Goal\n");
    project.write("issue/i-001/spec.md", "# Issue\n");
    project.write("residual/res-001.md", "---\nstatus: missing\n---\n");
    project.write("ticket/t-001/item.md", "# Ticket item.\n");
    let root = project.path().to_path_buf();
    let runbook = root.join(".ratmac").join("ratmac.toml");
    let engine_root = root.join(".ratmac");

    // Positive controls on the clean fixture: the entries work.
    let first = Scheduler::open(&root)
        .expect("WEBV-013: the library fixture opens")
        .start()
        .expect("WEBV-013: the library fixture starts a run")
        .id()
        .expect("a started run has an id")
        .to_owned();
    Scheduler::open_run(&root, &first)
        .expect("WEBV-013: the started run opens")
        .step(StepRequest::new("fixture control"))
        .expect("WEBV-013: the control run reaches its spawning state");
    let hold_request = HoldRequest {
        blocker: Some("work/blocker.txt".to_owned()),
        confirmation: Some(format!("hold {first}")),
        run: Some(first.clone()),
    };
    let hold_plan =
        blocked::plan_hold(&root, &hold_request).expect("WEBV-013: the control hold plans");
    blocked::apply_hold(&root, &hold_plan).expect("WEBV-013: the control hold applies");
    let second = Scheduler::open(&root)
        .expect("WEBV-013: the library fixture reopens")
        .start()
        .expect("WEBV-013: the library fixture starts a second run")
        .id()
        .expect("a started run has an id")
        .to_owned();
    let abandon_request = AbandonRequest {
        confirmation: Some(format!("abandon {second}")),
        run: Some(second.clone()),
    };
    let abandon_plan = abandon::plan_abandon(&root, &abandon_request)
        .expect("WEBV-013: the control abandon plans");
    abandon::apply_abandon(&root, &abandon_plan).expect("WEBV-013: the control abandon applies");
    let mut rendered_report = Vec::new();
    let reported = cli::run_from(
        ["status", "--run", first.as_str()],
        &root,
        &mut rendered_report,
    )
    .expect("WEBV-013: the control run_from status reports");
    assert_eq!(reported, 0, "WEBV-013: the control run_from status exits 0");

    let ghost = "ghost-run-900";

    // Scheduler entries: Err(StateError) whose text is the refusal.
    probe.row(
        &root,
        Shape::Presplit(".arca/rtm.lock"),
        "Scheduler::open",
        || {
            Scheduler::open(&root)
                .expect_err("open refuses residue")
                .to_string()
        },
    );
    probe.row(
        &root,
        Shape::Presplit(".arca/runs"),
        "Scheduler::open_run",
        || {
            Scheduler::open_run(&root, ghost)
                .expect_err("open_run refuses residue before its run address")
                .to_string()
        },
    );
    probe.row(&root, Shape::FlatState, "Scheduler::run_roster", || {
        Scheduler::run_roster(&root)
            .expect_err("run_roster refuses residue")
            .to_string()
    });
    probe.row(
        &root,
        Shape::Presplit(".arca/ratmac.toml"),
        "Scheduler::spawn_to",
        || {
            Scheduler::spawn_to(&root, ghost, "undeclared", &BTreeMap::new())
                .expect_err("spawn_to refuses residue before the unknown parent and name")
                .to_string()
        },
    );
    let area = root.join("area");
    fs::create_dir_all(area.join(".arca")).expect("create the workspace area");
    fs::write(area.join(".arca/rtm.lock"), "legacy engine residue\n")
        .expect("plant the workspace residue");
    probe.row(
        &area,
        Shape::Presplit(".arca/rtm.lock"),
        "Scheduler::spawn_to_with_workspace",
        || {
            let bindings = BTreeMap::from([("ticket".to_owned(), "WEBV-013".to_owned())]);
            Scheduler::spawn_to_with_workspace(&root, ghost, "rev", &bindings, Some(&area))
                .expect_err("spawn_to_with_workspace refuses its workspace residue first")
                .to_string()
        },
    );
    let _ = fs::remove_dir_all(&area);
    probe.row(&root, Shape::RecordState, "Scheduler::respawn", || {
        let request = RespawnRequest {
            run: Some(first.clone()),
            confirmation: Some("not the phrase".to_owned()),
        };
        Scheduler::respawn(&root, &request)
            .expect_err("respawn refuses residue before the confirmation")
            .to_string()
    });

    // Abandon and hold entries: the entry's Err refusal contains the refusal.
    probe.row(
        &root,
        Shape::Presplit(".arca/state.toml"),
        "abandon::resolve_target",
        || {
            abandon::resolve_target(&root, Some(ghost))
                .expect_err("resolve_target refuses residue before the unknown run")
                .to_string()
        },
    );
    probe.row(&root, Shape::FlatState, "abandon::plan_abandon", || {
        let request = AbandonRequest {
            confirmation: Some("wrong phrase".to_owned()),
            run: Some(first.clone()),
        };
        abandon::plan_abandon(&root, &request)
            .expect_err("plan_abandon refuses residue before the confirmation")
            .to_string()
    });
    probe.row(
        &root,
        Shape::Presplit(".arca/rtm.lock"),
        "abandon::apply_abandon",
        || {
            abandon::apply_abandon(&root, &abandon_plan)
                .expect_err("apply_abandon refuses residue before the stale plan")
                .to_string()
        },
    );
    probe.row(
        &root,
        Shape::Presplit(".arca/runs"),
        "blocked::plan_hold",
        || {
            let request = HoldRequest {
                blocker: None,
                confirmation: None,
                run: Some(first.clone()),
            };
            blocked::plan_hold(&root, &request)
                .expect_err("plan_hold refuses residue before the missing blocker")
                .to_string()
        },
    );
    probe.row(&root, Shape::RecordPhase, "blocked::apply_hold", || {
        blocked::apply_hold(&root, &hold_plan)
            .expect_err("apply_hold refuses residue before the stale plan")
            .to_string()
    });

    // Diagnose: exactly one error finding, RB111 for a pre-cutover runbook
    // and RB101 otherwise, its message carrying the refusal.
    for (shape, code) in [
        (Shape::Presplit(".arca/rtm.lock"), "RB101"),
        (Shape::PhasesRunbook, "RB111"),
    ] {
        let log = probe.fresh_log();
        let planted = plant(&root, shape);
        let before = tree_snapshot(&root);
        let findings = ratmac::doctor::diagnose(&runbook);
        assert_eq!(
            findings.len(),
            1,
            "WEBV-013: doctor::diagnose stops at one residue finding; held {findings:?}"
        );
        assert_eq!(
            findings[0].code(),
            code,
            "WEBV-013: the residue finding carries its documented code"
        );
        assert_eq!(
            findings[0].severity(),
            ratmac::doctor::Severity::Error,
            "WEBV-013: the residue finding is an error"
        );
        assert!(
            planted
                .sentences
                .iter()
                .any(|sentence| findings[0].message().contains(sentence)),
            "WEBV-013: the residue finding's message carries the refusal: {}",
            findings[0].message()
        );
        assert!(!log.exists(), "WEBV-013: diagnose logs no operation here");
        assert_eq!(
            tree_snapshot(&root),
            before,
            "WEBV-013: diagnose writes nothing"
        );
        drop(planted);
    }

    // Scaffold and skill: Err(Preflight(<refusal>)) even for an occupied path.
    let taken_scaffold = root.join("taken-scaffold.toml");
    fs::write(&taken_scaffold, "occupied\n").expect("occupy the scaffold path");
    probe.row(
        &root,
        Shape::Presplit(".arca/rtm.lock"),
        "scaffold::write_scaffold",
        || {
            let refusal = ratmac::scaffold::write_scaffold(&taken_scaffold)
                .expect_err("write_scaffold refuses residue before the occupied path");
            assert!(
                matches!(refusal, ratmac::scaffold::ScaffoldRefusal::Preflight(_)),
                "WEBV-013: the scaffold refusal is its preflight: {refusal}"
            );
            refusal.to_string()
        },
    );
    let taken_skill = root.join("taken-skill");
    fs::create_dir_all(&taken_skill).expect("occupy the skill path");
    probe.row(
        &root,
        Shape::Presplit(".arca/runs"),
        "skill::write_skill",
        || {
            let refusal = ratmac::skill::write_skill(&taken_skill)
                .expect_err("write_skill refuses residue before the occupied path");
            assert!(
                matches!(refusal, ratmac::skill::SkillRefusal::Preflight(_)),
                "WEBV-013: the skill refusal is its preflight: {refusal}"
            );
            refusal.to_string()
        },
    );

    // The contract gates: Err holding exactly one defect naming the project's
    // checkout path and the refusal.
    {
        let log = probe.fresh_log();
        let planted = plant(&root, Shape::Presplit(".arca/rtm.lock"));
        let before = tree_snapshot(&root);
        let defects = contract::gate_intake(&root)
            .expect_err("contract::gate_intake refuses residue before its contract reads");
        assert_eq!(
            defects.len(),
            1,
            "WEBV-013: contract::gate_intake returns exactly one defect; held {defects:?}"
        );
        assert_defect_names_the_refusal("contract::gate_intake", &defects[0], &planted, &root);
        assert!(
            !log.exists(),
            "WEBV-013: contract::gate_intake logs no operation"
        );
        assert_eq!(
            tree_snapshot(&root),
            before,
            "WEBV-013: contract::gate_intake writes nothing"
        );
        drop(planted);
    }
    {
        let log = probe.fresh_log();
        let planted = plant(&root, Shape::FlatState);
        let before = tree_snapshot(&root);
        let defects = contract::work_items(&root)
            .expect_err("contract::work_items refuses residue before its contract reads");
        assert_eq!(
            defects.len(),
            1,
            "WEBV-013: contract::work_items returns exactly one defect; held {defects:?}"
        );
        assert_defect_names_the_refusal("contract::work_items", &defects[0], &planted, &root);
        assert!(
            !log.exists(),
            "WEBV-013: contract::work_items logs no operation"
        );
        assert_eq!(
            tree_snapshot(&root),
            before,
            "WEBV-013: contract::work_items writes nothing"
        );
        drop(planted);
    }
    {
        let log = probe.fresh_log();
        let planted = plant(&root, Shape::Presplit(".arca/state.toml"));
        let before = tree_snapshot(&root);
        let defects = contract::gate_records(&root, &engine_root, ghost)
            .expect_err("contract::gate_records refuses residue before the run id");
        assert_eq!(
            defects.len(),
            1,
            "WEBV-013: contract::gate_records returns exactly one defect; held {defects:?}"
        );
        assert_defect_names_the_refusal("contract::gate_records", &defects[0], &planted, &root);
        assert!(
            !log.exists(),
            "WEBV-013: contract::gate_records logs no operation"
        );
        assert_eq!(
            tree_snapshot(&root),
            before,
            "WEBV-013: contract::gate_records writes nothing"
        );
        drop(planted);
    }
    {
        let log = probe.fresh_log();
        let planted = plant(&root, Shape::RecordState);
        let before = tree_snapshot(&root);
        let defects = contract::unproven_mechanization(&root);
        assert_eq!(
            defects.len(),
            1,
            "WEBV-013: contract::unproven_mechanization returns exactly the residue defect; \
             held {defects:?}"
        );
        assert_defect_names_the_refusal(
            "contract::unproven_mechanization",
            &defects[0],
            &planted,
            &root,
        );
        assert!(
            !log.exists(),
            "WEBV-013: contract::unproven_mechanization logs no operation"
        );
        assert_eq!(
            tree_snapshot(&root),
            before,
            "WEBV-013: contract::unproven_mechanization writes nothing"
        );
        drop(planted);
    }

    // The machine loader: Err whose code, location, and message are the
    // residue refusal's.
    for (shape, code) in [
        (Shape::Presplit(".arca/rtm.lock"), "RB101"),
        (Shape::PhasesRunbook, "RB111"),
    ] {
        let log = probe.fresh_log();
        let planted = plant(&root, shape);
        let before = tree_snapshot(&root);
        let error = MachineClass::load_from_project_root(&root)
            .expect_err("MachineClass::load_from_project_root refuses residue");
        assert_eq!(
            error.code(),
            code,
            "WEBV-013: the loader's residue refusal carries its documented code"
        );
        let shown_runbook = rendered(&runbook);
        let canonical_runbook = canonical_rendered(&runbook);
        assert!(
            error.location() == shown_runbook || error.location() == canonical_runbook,
            "WEBV-013: the loader's residue refusal locates the checkout's runbook with \
             forward slashes; held {:?}",
            error.location()
        );
        assert!(
            planted
                .sentences
                .iter()
                .any(|sentence| error.message().contains(sentence)),
            "WEBV-013: the loader's residue refusal message carries the refusal: {}",
            error.message()
        );
        assert!(!log.exists(), "WEBV-013: the loader logs no operation here");
        assert_eq!(
            tree_snapshot(&root),
            before,
            "WEBV-013: the loader writes nothing"
        );
        drop(planted);
    }

    // The CLI entry itself: the same command line the binary answers.
    probe.row(
        &root,
        Shape::Presplit(".arca/rtm.lock"),
        "cli::run_from",
        || {
            let mut report = Vec::new();
            let error = cli::run_from(["doctor", "--no-such-option"], &root, &mut report)
                .expect_err("run_from refuses residue before the unknown option");
            assert!(
                report.is_empty(),
                "WEBV-013: the refusal prints nothing on stdout"
            );
            assert_eq!(
                error.exit_code(),
                1,
                "WEBV-013: run_from's residue refusal exits 1"
            );
            error.to_string()
        },
    );
}

/// WEBV-013: every listed command and every library entry refuses residue
/// before its own argument validation, and the matrix fails until it covers
/// every command the general usage lists.
#[test]
fn webv_013() {
    let logs = TempTree::new("t120-013-op").expect("own the operation log tree");
    let mut step = 0usize;

    // The route table: the general usage's `Commands:` line.
    let project = plain_project("013-cli", RUNBOOK);
    let root = project.path().to_path_buf();
    let commands = route_table_commands("WEBV-013", &root);
    assert!(
        !commands.is_empty(),
        "WEBV-013: the `Commands:` line lists the route table"
    );

    // Positive controls on the clean fixture: every route works.
    let start = rtm(&root, &["start"]);
    assert!(
        start.status.success(),
        "WEBV-013: the control `rtm start` succeeds: {}{}",
        stdout(&start),
        stderr(&start)
    );
    let first = started_run(&start);
    let status = rtm(&root, &["status", "--run", &first]);
    assert!(
        status.status.success() && stdout(&status).contains("Engine root:"),
        "WEBV-013: the control `rtm status` reports: {}{}",
        stdout(&status),
        stderr(&status)
    );
    let step_control = rtm(&root, &["step", "--run", &first]);
    assert!(
        step_control.status.success(),
        "WEBV-013: the control `rtm step` advances: {}{}",
        stdout(&step_control),
        stderr(&step_control)
    );
    let spawn_control = rtm(
        &root,
        &["spawn", "rev", "--run", &first, "--bind", "ticket=WEBV-013"],
    );
    assert!(
        spawn_control.status.success(),
        "WEBV-013: the control `rtm spawn` is legal: {}{}",
        stdout(&spawn_control),
        stderr(&spawn_control)
    );
    let hold_confirmation = format!("hold {first}");
    let hold_control = rtm(
        &root,
        &[
            "hold",
            "--run",
            &first,
            "--blocker",
            "work/blocker.txt",
            "--confirm",
            &hold_confirmation,
        ],
    );
    assert!(
        hold_control.status.success(),
        "WEBV-013: the control `rtm hold` pauses: {}{}",
        stdout(&hold_control),
        stderr(&hold_control)
    );
    let second_output = rtm(&root, &["start"]);
    assert!(
        second_output.status.success(),
        "WEBV-013: the control second `rtm start` succeeds: {}{}",
        stdout(&second_output),
        stderr(&second_output)
    );
    let second = started_run(&second_output);
    let step_second = rtm(&root, &["step", "--run", &second]);
    assert!(
        step_second.status.success(),
        "WEBV-013: the control run reaches its spawning state: {}{}",
        stdout(&step_second),
        stderr(&step_second)
    );
    let respawn_confirmation = format!("respawn {second}");
    let respawn_control = rtm(
        &root,
        &[
            "respawn",
            "--run",
            &second,
            "--confirm",
            &respawn_confirmation,
        ],
    );
    assert!(
        respawn_control.status.success(),
        "WEBV-013: the control `rtm respawn` supersedes: {}{}",
        stdout(&respawn_control),
        stderr(&respawn_control)
    );
    let third_output = rtm(&root, &["start"]);
    assert!(
        third_output.status.success(),
        "WEBV-013: the control third `rtm start` succeeds: {}{}",
        stdout(&third_output),
        stderr(&third_output)
    );
    let third = started_run(&third_output);
    let abandon_confirmation = format!("abandon {third}");
    let abandon_control = rtm(
        &root,
        &[
            "abandon",
            "--run",
            &third,
            "--confirm",
            &abandon_confirmation,
        ],
    );
    assert!(
        abandon_control.status.success(),
        "WEBV-013: the control `rtm abandon` retires: {}{}",
        stdout(&abandon_control),
        stderr(&abandon_control)
    );
    let doctor_control = rtm(&root, &["doctor"]);
    assert!(
        doctor_control.status.success() && stdout(&doctor_control).contains("Engine root:"),
        "WEBV-013: the control `rtm doctor` diagnoses: {}{}",
        stdout(&doctor_control),
        stderr(&doctor_control)
    );
    let writes = TempTree::new("t120-013-writes").expect("own the control write tree");
    let scaffold_target = writes.join("control-scaffold.toml");
    let scaffold_control = rtm(
        &writes.path(),
        &[
            "scaffold",
            scaffold_target.to_str().expect("the control path is UTF-8"),
        ],
    );
    assert!(
        scaffold_control.status.success() && stdout(&scaffold_control).contains("Wrote"),
        "WEBV-013: the control `rtm scaffold` writes: {}{}",
        stdout(&scaffold_control),
        stderr(&scaffold_control)
    );
    let skill_target = writes.join("control-skill");
    let skill_control = rtm(
        &writes.path(),
        &[
            "skill",
            skill_target.to_str().expect("the control path is UTF-8"),
        ],
    );
    assert!(
        skill_control.status.success() && stdout(&skill_control).contains("Wrote"),
        "WEBV-013: the control `rtm skill` writes: {}{}",
        stdout(&skill_control),
        stderr(&skill_control)
    );

    // Every residue shape answers with its own named repair.
    let dangling = {
        let probe = TempTree::new("t120-013-linkprobe").expect("own the link probe tree");
        try_dangling_link(&probe.join(".arca").join("rtm.lock"))
    };
    let mut shapes = vec![
        Shape::FlatState,
        Shape::Presplit(".arca/ratmac.toml"),
        Shape::Presplit(".arca/runs"),
        Shape::Presplit(".arca/rtm.lock"),
        Shape::Presplit(".arca/state.toml"),
        Shape::PhasesRunbook,
        Shape::RecordState,
        Shape::RecordPhase,
    ];
    if dangling {
        shapes.push(Shape::DanglingLock);
    } else {
        eprintln!(
            "t-120: this platform refuses dangling links; the DanglingLock shape is covered by \
             the regular pre-split artifacts and noted here"
        );
    }
    for shape in shapes {
        let label =
            format!("WEBV-013: `rtm status --run <id>` refuses {shape:?} with its named repair");
        refuse_row(
            &label,
            &root,
            &["status", "--run", &first],
            &root,
            shape,
            Expect::Invoking {
                command: "status",
                trailing_next: true,
            },
            &logs,
            &mut step,
        );
    }

    // The addressed-project fixture and its clean twin.
    let target = plain_project("013-target", RUNBOOK);
    let target_root = target.path().to_path_buf();
    let target_runbook = target_root.join(".ratmac").join("ratmac.toml");
    let target_runbook = target_runbook.to_str().expect("target path is UTF-8");
    let clean = plain_project("013-clean", RUNBOOK);
    let clean_runbook = clean.path().join(".ratmac").join("ratmac.toml");
    let clean_runbook = clean_runbook.to_str().expect("clean path is UTF-8");

    // The matrix: one row per command, residue defeating malformed options.
    struct Row {
        command: &'static str,
        args: Vec<String>,
        plant_at: PathBuf,
        shape: Shape,
        expect: Expect,
    }
    let invoking = |command: &'static str, args: Vec<String>, shape: Shape| Row {
        command,
        args,
        plant_at: root.clone(),
        shape,
        expect: Expect::Invoking {
            command,
            trailing_next: false,
        },
    };
    let spawn_shape = if dangling {
        Shape::DanglingLock
    } else {
        Shape::Presplit(".arca/rtm.lock")
    };
    let mut rows = vec![
        invoking(
            "start",
            vec!["start".into(), "--no-such-option".into()],
            Shape::Presplit(".arca/rtm.lock"),
        ),
        invoking(
            "status",
            vec!["status".into(), "--run".into()],
            Shape::Presplit(".arca/runs"),
        ),
        invoking(
            "step",
            vec![
                "step".into(),
                "--run".into(),
                first.clone(),
                "stray-positional".into(),
            ],
            Shape::Presplit(".arca/ratmac.toml"),
        ),
        invoking(
            "hold",
            vec![
                "hold".into(),
                "--run".into(),
                first.clone(),
                "--blocker".into(),
                "work/blocker.txt".into(),
                "--confirm".into(),
                "not the phrase".into(),
            ],
            Shape::Presplit(".arca/state.toml"),
        ),
        invoking(
            "abandon",
            vec![
                "abandon".into(),
                "--run".into(),
                first.clone(),
                "--confirm".into(),
                "wrong phrase".into(),
            ],
            Shape::FlatState,
        ),
        invoking(
            "spawn",
            vec![
                "spawn".into(),
                "rev".into(),
                "--run".into(),
                first.clone(),
                "--bind".into(),
                "malformed".into(),
            ],
            spawn_shape,
        ),
        invoking(
            "respawn",
            vec![
                "respawn".into(),
                "--run".into(),
                first.clone(),
                "--confirm".into(),
                "wrong phrase".into(),
            ],
            Shape::RecordState,
        ),
        invoking(
            "scaffold",
            vec!["scaffold".into(), "--bogus".into()],
            Shape::Presplit(".arca/runs"),
        ),
        invoking(
            "skill",
            vec!["skill".into(), "--bogus".into()],
            Shape::Presplit(".arca/state.toml"),
        ),
        // doctor: the invoking checkout is inspected even when a target is
        // given, and residue defeats its own option validation.
        invoking(
            "doctor",
            vec!["doctor".into(), "--no-such-option".into()],
            Shape::Presplit(".arca/rtm.lock"),
        ),
        invoking(
            "doctor",
            vec![
                "doctor".into(),
                "--json".into(),
                "--json".into(),
                clean_runbook.to_owned(),
            ],
            Shape::Presplit(".arca/rtm.lock"),
        ),
        invoking(
            "doctor",
            vec!["doctor".into(), clean_runbook.to_owned()],
            Shape::Presplit(".arca/rtm.lock"),
        ),
        // The addressed project: residue there defeats later argument
        // defects and answers with the bare refusal.
        Row {
            command: "doctor",
            args: vec![
                "doctor".into(),
                target_runbook.to_owned(),
                "extra-positional".into(),
            ],
            plant_at: target_root.clone(),
            shape: Shape::Presplit(".arca/rtm.lock"),
            expect: Expect::Addressed,
        },
        Row {
            command: "scaffold",
            args: vec![
                "scaffold".into(),
                target_root.join("new.toml").to_string_lossy().into_owned(),
                "extra-positional".into(),
            ],
            plant_at: target_root.clone(),
            shape: Shape::Presplit(".arca/rtm.lock"),
            expect: Expect::Addressed,
        },
        Row {
            command: "skill",
            args: vec![
                "skill".into(),
                target_root.join("new-skill").to_string_lossy().into_owned(),
                "--bogus".into(),
            ],
            plant_at: target_root.clone(),
            shape: Shape::Presplit(".arca/rtm.lock"),
            expect: Expect::Addressed,
        },
    ];
    // `status`'s matrix row keeps its trailing next: line.
    for row in &mut rows {
        if row.command == "status" {
            row.expect = Expect::Invoking {
                command: "status",
                trailing_next: true,
            };
        }
    }

    // The coverage check: every listed command has a matrix row.
    for command in &commands {
        assert!(
            rows.iter().any(|row| row.command == *command),
            "WEBV-013: the general usage lists `{command}`, so the residue matrix must cover it"
        );
    }

    for row in &rows {
        let args: Vec<&str> = row.args.iter().map(String::as_str).collect();
        let label = format!(
            "WEBV-013: `rtm {}` reports residue before its own argument defects",
            row.command
        );
        refuse_row(
            &label,
            &root,
            &args,
            &row.plant_at,
            row.shape,
            row.expect,
            &logs,
            &mut step,
        );
    }

    // The library entries table.
    library_entries_refuse_residue_first();
}

// --- WEBV-014 -----------------------------------------------------------------

/// Assert the operation log holds at least `required` kinds, every line a
/// documented kind.
fn assert_logs_kinds(label: &str, log: &Path, required: &[&str]) {
    let bytes = fs::read(log).unwrap_or_else(|error| {
        panic!(
            "WEBV-014: {label} records its operations through RATMAC_TEST_OPERATION_LOG; \
             reading {}: {error}",
            rendered(log)
        )
    });
    let text = String::from_utf8(bytes)
        .unwrap_or_else(|error| panic!("WEBV-014: {} holds invalid UTF-8: {error}", rendered(log)));
    assert!(
        text.is_empty() || text.ends_with('\n'),
        "WEBV-014: {} ends every kind line with a newline",
        rendered(log)
    );
    let held: Vec<&str> = text.lines().collect();
    for kind in required {
        assert!(
            held.contains(kind),
            "WEBV-014: {label} logs the `{kind}` operation; the log held {held:?}"
        );
    }
    for kind in &held {
        assert!(
            KNOWN_KINDS.contains(kind),
            "WEBV-014: {label} logs only documented kinds; held {held:?}"
        );
    }
}

/// WEBV-014: clean positive controls log their documented operation kinds;
/// residue refusals leave the log absent and every tree byte-identical.
#[test]
fn webv_014() {
    let logs = TempTree::new("t120-014-op").expect("own the operation log tree");
    let project = plain_project("014", RUNBOOK);
    let root = project.path();

    // Positive controls: each route works and logs at least its kinds.
    let start_log = logs.join("start.log");
    let start = rtm_observed(root, &start_log, &["start"]);
    assert!(
        start.status.success(),
        "WEBV-014: the control `rtm start` succeeds: {}{}",
        stdout(&start),
        stderr(&start)
    );
    assert_logs_kinds("`rtm start`", &start_log, &["runbook", "lock"]);
    let run = started_run(&start);

    let status_log = logs.join("status.log");
    let status = rtm_observed(root, &status_log, &["status", "--run", &run]);
    assert!(
        status.status.success(),
        "WEBV-014: the control `rtm status --run <planned Run>` reports: {}{}",
        stdout(&status),
        stderr(&status)
    );
    assert_logs_kinds(
        "`rtm status --run <planned Run>`",
        &status_log,
        &["roster", "record", "runbook"],
    );

    let stepped = rtm(root, &["step", "--run", &run]);
    assert!(
        stepped.status.success(),
        "WEBV-014: the control `rtm step` reaches the spawning state: {}{}",
        stdout(&stepped),
        stderr(&stepped)
    );

    let spawn_log = logs.join("spawn.log");
    let spawned = rtm_observed(
        root,
        &spawn_log,
        &["spawn", "rev", "--run", &run, "--bind", "ticket=WEBV-014"],
    );
    assert!(
        spawned.status.success(),
        "WEBV-014: the control `rtm spawn` is legal: {}{}",
        stdout(&spawned),
        stderr(&spawned)
    );
    assert_logs_kinds("a legal `rtm spawn`", &spawn_log, &["ledger", "lock"]);

    let hold_confirmation = format!("hold {run}");
    let hold_log = logs.join("hold.log");
    let held = rtm_observed(
        root,
        &hold_log,
        &[
            "hold",
            "--run",
            &run,
            "--blocker",
            "work/blocker.txt",
            "--confirm",
            &hold_confirmation,
        ],
    );
    assert!(
        held.status.success(),
        "WEBV-014: the control `rtm hold` pauses: {}{}",
        stdout(&held),
        stderr(&held)
    );
    assert_logs_kinds("a valid `rtm hold`", &hold_log, &["blocker", "lock"]);

    let writes = TempTree::new("t120-014-writes").expect("own the write tree");
    let scaffold_target = writes.join("scaffold.toml");
    let scaffold_log = logs.join("scaffold.log");
    let scaffold = rtm_observed(
        &writes.path(),
        &scaffold_log,
        &[
            "scaffold",
            scaffold_target
                .to_str()
                .expect("the scaffold path is UTF-8"),
        ],
    );
    assert!(
        scaffold.status.success(),
        "WEBV-014: the control `rtm scaffold` writes: {}{}",
        stdout(&scaffold),
        stderr(&scaffold)
    );
    assert_logs_kinds("`rtm scaffold <new path>`", &scaffold_log, &["target"]);

    let skill_target = writes.join("skill");
    let skill_log = logs.join("skill.log");
    let skill = rtm_observed(
        &writes.path(),
        &skill_log,
        &[
            "skill",
            skill_target.to_str().expect("the skill path is UTF-8"),
        ],
    );
    assert!(
        skill.status.success(),
        "WEBV-014: the control `rtm skill` writes: {}{}",
        stdout(&skill),
        stderr(&skill)
    );
    assert_logs_kinds("`rtm skill <new path>`", &skill_log, &["target"]);

    let doctor_log = logs.join("doctor.log");
    let doctor = rtm_observed(root, &doctor_log, &["doctor"]);
    assert!(
        doctor.status.success() && stdout(&doctor).contains("Engine root:"),
        "WEBV-014: the control `rtm doctor` diagnoses: {}{}",
        stdout(&doctor),
        stderr(&doctor)
    );
    assert_logs_kinds("`rtm doctor`", &doctor_log, &["runbook"]);

    // Refusal twins: residue in the invoking checkout refuses every route
    // before any operation, leaving the log absent and the tree identical.
    let planted = plant(root, Shape::FlatState);
    let before = tree_snapshot(root);
    let hold_args = [
        "hold",
        "--run",
        run.as_str(),
        "--blocker",
        "work/blocker.txt",
        "--confirm",
        hold_confirmation.as_str(),
    ];
    let spawn_args = [
        "spawn",
        "rev",
        "--run",
        run.as_str(),
        "--bind",
        "ticket=WEBV-014",
    ];
    let refusal_rows: [(&str, Vec<&str>); 6] = [
        ("start", vec!["start"]),
        ("status", vec!["status", "--run", &run]),
        ("step", vec!["step", "--run", &run]),
        ("hold", hold_args.to_vec()),
        ("spawn", spawn_args.to_vec()),
        ("doctor", vec!["doctor"]),
    ];
    for (index, (command, args)) in refusal_rows.into_iter().enumerate() {
        let log = logs.join(format!("refusal-{index}.log"));
        let output = rtm_observed(root, &log, &args);
        assert_refusal(
            &format!("WEBV-014: `rtm {command}` refuses residue before any operation"),
            &output,
            Expect::Invoking {
                command,
                trailing_next: command == "status",
            },
            &planted,
            &log,
        );
        assert_eq!(
            tree_snapshot(root),
            before,
            "WEBV-014: the refused `rtm {command}` leaves the project byte-identical"
        );
    }
    drop(planted);
}

// --- WEBV-015 -----------------------------------------------------------------

/// One Git repository with a linked worktree, for shared-root rows.
struct LinkedFixture {
    // Owns the fixture tree; read implicitly by Drop.
    #[allow(dead_code)]
    tree: TempTree,
    primary: PathBuf,
    linked: PathBuf,
}

fn linked_fixture(label: &str) -> LinkedFixture {
    let tree = TempTree::new(&format!("t120-{label}")).expect("own the linked fixture tree");
    let primary = tree.join("primary");
    let linked = tree.join("linked");
    fs::create_dir_all(primary.join(".ratmac")).expect("create the primary runbook directory");
    fs::write(primary.join(".ratmac/ratmac.toml"), RUNBOOK).expect("write the primary runbook");
    fs::create_dir_all(primary.join("work")).expect("create the primary work directory");
    fs::write(
        primary.join("work/blocker.txt"),
        "Opaque blocker; its content is not a gate.\n",
    )
    .expect("write the blocker leaf");
    fs::write(primary.join("work/item.md"), "Contributor-owned bytes.\n")
        .expect("write the work leaf");
    for args in [
        &["init", "-q"][..],
        &["add", "--all"][..],
        &[
            "-c",
            "user.email=qa@example.invalid",
            "-c",
            "user.name=Ratmac QA",
            "commit",
            "-q",
            "-m",
            "t-120 fixture base",
        ][..],
    ] {
        let output = support::git(&primary).args(args).output().expect("run git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            support::text(&output)
        );
    }
    let output = support::git(&primary)
        .args(["worktree", "add", "-q", "-b", "t120-linked"])
        .arg(&linked)
        .output()
        .expect("create the linked worktree");
    assert!(
        output.status.success(),
        "git worktree add: {}",
        support::text(&output)
    );
    LinkedFixture {
        tree,
        primary,
        linked,
    }
}

/// WEBV-015: residue at the invoking root, the shared primary root seen from
/// a linked worktree, or a valid explicit target refuses first; missing or
/// ambiguous targets inspect only the invoking checkout and then answer the
/// ordinary usage error.
#[test]
fn webv_015() {
    let logs = TempTree::new("t120-015-op").expect("own the operation log tree");
    let mut step = 0usize;
    let fixture = linked_fixture("015");
    let primary = fixture.primary.clone();
    let linked = fixture.linked.clone();

    // Positive control: a linked invocation works and writes the primary root.
    let start = rtm(&linked, &["start"]);
    assert!(
        start.status.success(),
        "WEBV-015: the control `rtm start` from the linked worktree succeeds: {}{}",
        stdout(&start),
        stderr(&start)
    );
    let run = started_run(&start);
    assert!(
        primary
            .join(".ratmac/runs")
            .join(&run)
            .join("run.toml")
            .is_file(),
        "WEBV-015: the linked control run writes the shared primary root"
    );

    // Residue only at the shared primary root: a linked invocation refuses
    // before anything else, naming the primary artifact.
    refuse_row(
        "WEBV-015: a linked `rtm start` refuses residue only at the shared primary root",
        &linked,
        &["start"],
        &primary,
        Shape::Presplit(".arca/state.toml"),
        Expect::Invoking {
            command: "start",
            trailing_next: false,
        },
        &logs,
        &mut step,
    );
    refuse_row(
        "WEBV-015: a linked `rtm doctor` refuses residue only at the shared primary root",
        &linked,
        &["doctor"],
        &primary,
        Shape::Presplit(".arca/rtm.lock"),
        Expect::Invoking {
            command: "doctor",
            trailing_next: false,
        },
        &logs,
        &mut step,
    );

    // Residue only at the invoking root, defeated against malformed options.
    refuse_row(
        "WEBV-015: `rtm doctor --json --json` reports invoking residue before the option defect",
        &primary,
        &["doctor", "--json", "--json"],
        &primary,
        Shape::Presplit(".arca/rtm.lock"),
        Expect::Invoking {
            command: "doctor",
            trailing_next: false,
        },
        &logs,
        &mut step,
    );
    let wrong_hold = format!("hold-{run}-wrong");
    refuse_row(
        "WEBV-015: `rtm hold` with a wrong phrase reports invoking residue first",
        &primary,
        &[
            "hold",
            "--run",
            &run,
            "--blocker",
            "work/blocker.txt",
            "--confirm",
            &wrong_hold,
        ],
        &primary,
        Shape::FlatState,
        Expect::Invoking {
            command: "hold",
            trailing_next: false,
        },
        &logs,
        &mut step,
    );

    // A valid explicit target: residue at that project refuses with the bare
    // refusal and exit 2, clean arguments or malformed extras alike.
    let target = plain_project("015-target", RUNBOOK);
    let target_root = target.path().to_path_buf();
    let target_runbook = target_root.join(".ratmac").join("ratmac.toml");
    let target_runbook = target_runbook.to_str().expect("target path is UTF-8");
    refuse_row(
        "WEBV-015: `rtm doctor --json <runbook>` refuses the addressed project's residue",
        &primary,
        &["doctor", "--json", target_runbook],
        &target_root,
        Shape::Presplit(".arca/rtm.lock"),
        Expect::Addressed,
        &logs,
        &mut step,
    );
    refuse_row(
        "WEBV-015: `rtm doctor <runbook> <extra>` reports the addressed residue first",
        &primary,
        &["doctor", target_runbook, "extra-positional"],
        &target_root,
        Shape::Presplit(".arca/runs"),
        Expect::Addressed,
        &logs,
        &mut step,
    );
    let scaffold_path = target_root.join("new.toml").to_string_lossy().into_owned();
    refuse_row(
        "WEBV-015: `rtm scaffold <path> <extra>` reports the addressed residue first",
        &primary,
        &["scaffold", &scaffold_path, "extra-positional"],
        &target_root,
        Shape::Presplit(".arca/rtm.lock"),
        Expect::Addressed,
        &logs,
        &mut step,
    );
    let skill_path = target_root.join("new-skill").to_string_lossy().into_owned();
    refuse_row(
        "WEBV-015: `rtm skill <path> --bogus` reports the addressed residue first",
        &primary,
        &["skill", &skill_path, "--bogus"],
        &target_root,
        Shape::Presplit(".arca/state.toml"),
        Expect::Addressed,
        &logs,
        &mut step,
    );

    // A spawn workspace: residue there refuses under `spawn:` with exit 1,
    // even beside a malformed binding.
    let area = primary.join("area");
    fs::create_dir_all(area.join(".arca/runs/legacy")).expect("create the workspace area");
    fs::write(
        area.join(".arca/runs/legacy/state.toml"),
        "phase = \"plan\"\n",
    )
    .expect("plant the workspace residue");
    let area_sentences = residue_sentences(Shape::Presplit(".arca/runs"), &area);
    for (label, args) in [
        (
            "WEBV-015: `rtm spawn --workspace` refuses the workspace's residue",
            vec![
                "spawn".to_owned(),
                "rev".into(),
                "--run".into(),
                run.clone(),
                "--bind".into(),
                "ticket=WEBV-015".into(),
                "--workspace".into(),
                area.to_string_lossy().into_owned(),
            ],
        ),
        (
            "WEBV-015: `rtm spawn --workspace` with a malformed bind still reports the \
             workspace's residue first",
            vec![
                "spawn".to_owned(),
                "rev".into(),
                "--run".into(),
                run.clone(),
                "--bind".into(),
                "malformed".into(),
                "--workspace".into(),
                area.to_string_lossy().into_owned(),
            ],
        ),
    ] {
        step += 1;
        let log = logs.join(format!("op-{:03}.log", step));
        let before_primary = tree_snapshot(&primary);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = rtm_observed(&primary, &log, &arg_refs);
        assert_refusal(
            label,
            &output,
            Expect::SpawnWorkspace,
            &Planted {
                sentences: area_sentences.clone(),
                runbook_restore: None,
                clean: Vec::new(),
            },
            &log,
        );
        assert_eq!(
            tree_snapshot(&primary),
            before_primary,
            "WEBV-015: the refused spawn leaves the project byte-identical"
        );
    }

    // A missing or ambiguous target: only the invoking checkout is inspected,
    // then the ordinary usage error follows.
    let other_area = primary.join("other-area");
    fs::create_dir_all(&other_area).expect("create the second workspace area");
    for (label, args, first_line) in [
        (
            "WEBV-015: a repeated `--workspace` addresses no project",
            vec![
                "spawn".to_owned(),
                "rev".into(),
                "--run".into(),
                run.clone(),
                "--bind".into(),
                "ticket=WEBV-015".into(),
                "--workspace".into(),
                area.to_string_lossy().into_owned(),
                "--workspace".into(),
                other_area.to_string_lossy().into_owned(),
            ],
            "rtm: spawn: --workspace given twice",
        ),
        (
            "WEBV-015: a valueless `--workspace` addresses no project",
            vec![
                "spawn".to_owned(),
                "rev".into(),
                "--run".into(),
                run.clone(),
                "--bind".into(),
                "ticket=WEBV-015".into(),
                "--workspace".into(),
            ],
            "rtm: spawn: --workspace needs a directory path",
        ),
    ] {
        step += 1;
        let log = logs.join(format!("op-{:03}.log", step));
        let before_primary = tree_snapshot(&primary);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = rtm_observed(&primary, &log, &arg_refs);
        assert!(
            output.stdout.is_empty(),
            "{label}: the ordinary usage error prints nothing on stdout: {}",
            stdout(&output)
        );
        assert_eq!(
            output.status.code(),
            Some(1),
            "{label}: the ordinary usage error exits 1: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).starts_with(first_line),
            "{label}: the ordinary usage error is unchanged; held {}",
            stderr(&output)
        );
        assert!(
            !log.exists(),
            "{label}: no operation runs before the ordinary usage error"
        );
        assert_eq!(
            tree_snapshot(&primary),
            before_primary,
            "{label}: nothing is written"
        );
    }

    // An unrelated project's residue is not inspected when no target
    // addresses it: the ordinary routes still answer.
    {
        let residue_target = plain_project("015-unrelated", RUNBOOK);
        let _planted = plant(residue_target.path(), Shape::Presplit(".arca/rtm.lock"));
        let doctor = rtm(&primary, &["doctor"]);
        assert!(
            doctor.status.success() && stdout(&doctor).contains("Engine root:"),
            "WEBV-015: `rtm doctor` without a target inspects only the invoking checkout: {}{}",
            stdout(&doctor),
            stderr(&doctor)
        );
        let scaffold = rtm(&primary, &["scaffold"]);
        assert_eq!(
            scaffold.status.code(),
            Some(2),
            "WEBV-015: `rtm scaffold` with no path answers its ordinary usage error: {}",
            stderr(&scaffold)
        );
        assert!(
            stderr(&scaffold).contains("scaffold: no path given"),
            "WEBV-015: the ordinary scaffold usage error is unchanged: {}",
            stderr(&scaffold)
        );
    }
}

// --- WEBV-016 -----------------------------------------------------------------

/// One mixed-help row with residue planted: the preflight refuses.
fn mixed_help_refuses(
    root: &Path,
    planted: &Planted,
    log: &Path,
    args: &[&str],
    command: &'static str,
) {
    let before = tree_snapshot(root);
    let output = rtm_observed(root, log, args);
    assert_refusal(
        &format!(
            "WEBV-016: `rtm {}` carries a help token among other arguments, so its preflight \
             still refuses residue",
            args.join(" ")
        ),
        &output,
        Expect::Invoking {
            command,
            trailing_next: false,
        },
        planted,
        log,
    );
    assert_eq!(
        tree_snapshot(root),
        before,
        "WEBV-016: the refused mixed help form changes nothing"
    );
}

/// WEBV-016: without residue, behavior and argument errors are unchanged;
/// only the exact standalone help forms and unknown commands stay pure usage
/// responses, even with residue, and a help token mixed with any other
/// argument cannot bypass preflight.
#[test]
fn webv_016() {
    let logs = TempTree::new("t120-016-op").expect("own the operation log tree");
    let project = plain_project("016", RUNBOOK);
    let root = project.path().to_path_buf();

    // Positive control: the route works.
    let start = rtm(&root, &["start"]);
    assert!(
        start.status.success(),
        "WEBV-016: the control `rtm start` succeeds: {}{}",
        stdout(&start),
        stderr(&start)
    );
    let run = started_run(&start);

    // Without residue: ordinary valid behavior and argument errors stay.
    let errors: [(&[&str], i32, &str); 6] = [
        (
            &["start", "extra"],
            1,
            "rtm: start accepts no run-id or extra arguments\n",
        ),
        (
            &["doctor", "--bogus"],
            2,
            "rtm: doctor: unknown option \"--bogus\"; doctor accepts --json and one runbook \
             path\n",
        ),
        (
            &["spawn", "rev", "--bind", "malformed"],
            1,
            "rtm: spawn: --bind \"malformed\" is not shaped name=value\n",
        ),
        (&["step", "--run"], 2, "rtm: step: --run needs a run id"),
        (&["status", "--run", "ghost-run-900"], 2, "rtm: status:"),
        (
            &[
                "hold",
                "--run",
                "x",
                "--blocker",
                "work/blocker.txt",
                "--confirm",
                "wrong",
            ],
            1,
            "rtm: hold refused;",
        ),
    ];
    for (args, code, first) in errors {
        let output = rtm(&root, args);
        assert_eq!(
            output.status.code(),
            Some(code),
            "WEBV-016: `rtm {args:?}` keeps its ordinary exit; held {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).starts_with(first),
            "WEBV-016: `rtm {args:?}` keeps its ordinary error text; held {}",
            stderr(&output)
        );
        assert!(
            output.stdout.is_empty(),
            "WEBV-016: `rtm {args:?}` keeps printing nothing on stdout"
        );
    }

    // Pure help forms and unknown commands stay pure without residue.
    let general = cli::help("");
    for (args, expected_stdout, code) in [
        (vec!["--help"], general, 0),
        (vec!["-h"], general, 0),
        (vec!["bogus", "--help"], general, 0),
        (vec!["step", "--help"], cli::help("step"), 0),
        (vec!["step", "-h"], cli::help("step"), 0),
        (vec!["doctor", "-h"], cli::help("doctor"), 0),
        (vec!["spawn", "--help"], cli::help("spawn"), 0),
        (
            vec!["step", "--run", "run-001", "--help"],
            cli::help("step"),
            0,
        ),
        (vec!["init"], "", 1),
    ] {
        let output = rtm(&root, &args);
        assert_eq!(
            output.status.code(),
            Some(code),
            "WEBV-016: `rtm {args:?}` keeps its ordinary exit"
        );
        if code == 0 {
            assert_eq!(
                stdout(&output),
                expected_stdout,
                "WEBV-016: `rtm {args:?}` keeps its help bytes"
            );
            assert!(
                stderr(&output).is_empty(),
                "WEBV-016: `rtm {args:?}` prints no error"
            );
        } else {
            assert_eq!(
                stderr(&output),
                "rtm: unsupported command or option: init\n",
                "WEBV-016: an unknown command keeps its usage refusal"
            );
            assert!(stdout(&output).is_empty());
        }
    }
    let unknown = rtm(&root, &["run"]);
    assert_eq!(
        stderr(&unknown),
        "rtm: unsupported command or option: run\n",
        "WEBV-016: another unknown command keeps its usage refusal"
    );
    assert_eq!(unknown.status.code(), Some(1));

    // With residue: the pure forms stay pure, the unknown command stays a
    // pure usage refusal, and nothing is read or written for them.
    {
        let planted = plant(&root, Shape::Presplit(".arca/rtm.lock"));
        let before = tree_snapshot(&root);
        let log = logs.join("pure.log");
        for (args, expected_stdout, code) in [
            (vec![] as Vec<&str>, general, 0),
            (vec!["--help"], general, 0),
            (vec!["-h"], general, 0),
            (vec!["bogus", "--help"], general, 0),
            (vec!["step", "--help"], cli::help("step"), 0),
            (vec!["step", "-h"], cli::help("step"), 0),
            (vec!["doctor", "-h"], cli::help("doctor"), 0),
            (vec!["init"], "", 1),
        ] {
            let output = rtm_observed(&root, &log, &args);
            assert_eq!(
                output.status.code(),
                Some(code),
                "WEBV-016: `rtm {args:?}` stays pure even with residue"
            );
            if code == 0 {
                assert_eq!(
                    stdout(&output),
                    expected_stdout,
                    "WEBV-016: `rtm {args:?}` prints the same help bytes with residue"
                );
                assert!(stderr(&output).is_empty());
            } else {
                assert_eq!(
                    stderr(&output),
                    "rtm: unsupported command or option: init\n",
                    "WEBV-016: the unknown command stays a pure usage refusal with residue"
                );
                assert!(stdout(&output).is_empty());
            }
        }
        assert!(
            !log.exists(),
            "WEBV-016: the pure forms and unknown commands log no operation"
        );
        assert_eq!(
            tree_snapshot(&root),
            before,
            "WEBV-016: the pure forms and unknown commands change nothing"
        );

        // A help token mixed with other arguments cannot bypass preflight.
        mixed_help_refuses(
            &root,
            &planted,
            &logs.join("mixed-step.log"),
            &["step", "--run", run.as_str(), "--help"],
            "step",
        );
        mixed_help_refuses(
            &root,
            &planted,
            &logs.join("mixed-doctor.log"),
            &["doctor", "--json", "-h"],
            "doctor",
        );
        mixed_help_refuses(
            &root,
            &planted,
            &logs.join("mixed-spawn.log"),
            &["spawn", "-h", "--help"],
            "spawn",
        );
    }

    // Without residue, the mixed help forms print the command's help and
    // nothing else runs.
    let mixed_clean = rtm(&root, &["step", "--run", &run, "--help"]);
    assert_eq!(
        mixed_clean.status.code(),
        Some(0),
        "WEBV-016: a mixed help form without residue still prints help: {}",
        stderr(&mixed_clean)
    );
    assert_eq!(
        stdout(&mixed_clean),
        cli::help("step"),
        "WEBV-016: a mixed help form without residue prints the command's help"
    );
}

// --- in-process hook hygiene ---------------------------------------------------

/// One in-process `RATMAC_TEST_*` variable, hidden for a scope and restored
/// exactly when dropped.
struct HookVar {
    name: &'static str,
    previous: Option<OsString>,
}

impl HookVar {
    fn hide(name: &'static str) -> Self {
        let previous = std::env::var_os(name);
        std::env::remove_var(name);
        Self { name, previous }
    }
}

impl Drop for HookVar {
    fn drop(&mut self) {
        match &self.previous {
            Some(previous) => std::env::set_var(self.name, previous),
            None => std::env::remove_var(self.name),
        }
    }
}

/// Hide every inherited Engine test hook for the in-process library rows.
fn hide_test_hooks() -> Vec<HookVar> {
    vec![
        HookVar::hide(OP_LOG_VAR),
        HookVar::hide(ROOT_LOG_VAR),
        HookVar::hide(REPEAT_VAR),
    ]
}
