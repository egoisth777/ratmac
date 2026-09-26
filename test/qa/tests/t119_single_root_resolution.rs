//! t-119 / WEB-005: single root resolution.
//!
//! WEBV-017 counts resolutions through `RATMAC_TEST_ROOT_LOG`: every CLI
//! route and every public path-taking library entry resolves each distinct
//! addressed project exactly once, and a method on an already-opened
//! `Scheduler` adds nothing.
//! WEBV-018 arms `RATMAC_TEST_ROOT_REPEAT` with another runtime root and with
//! `fail`: no route may ever observe a second answer.
//! WEBV-019 exercises the scope rules: linked worktrees report and write the
//! primary runtime root while reading their own runbook, a spawn workspace is
//! judged against the resolved repository and never resolved itself, the
//! no-Git fallback stays local, and addressed doctor/scaffold targets are the
//! only other projects an invocation resolves.
//! WEBV-020 keeps the discovery constructor private to `src/root.rs`: a
//! context-bound handler in `src/cli.rs` must fail to compile with `E0624`,
//! the same call inside `src/root.rs` must compile, and no other file in
//! `src/` names `Roots::resolve` or `root::resolve(`.
//!
//! In-process library calls share this process's environment, so every test
//! in this file holds one static mutex while `RATMAC_TEST_*` variables are
//! set, and every holder restores what it replaced.

use ratmac::abandon::{apply_abandon, plan_abandon, resolve_target, AbandonRequest};
use ratmac::blocked::{apply_hold, plan_hold, HoldRequest};
use ratmac::cli;
use ratmac::contract;
use ratmac::doctor;
use ratmac::machine::MachineClass;
use ratmac::scaffold::write_scaffold;
use ratmac::skill::write_skill;
use ratmac::{RespawnRequest, Scheduler, StepRequest};
use ratmac_qa::json::Json;
use ratmac_qa::support::{self, CaptureOptions, TempTree};
use ratmac_qa::tempgit::TempRepo;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::{Mutex, MutexGuard};

/// Where a feature build appends one line per resolution.
const LOG_VAR: &str = "RATMAC_TEST_ROOT_LOG";
/// The divergent answer a second resolution of one project would return.
const REPEAT_VAR: &str = "RATMAC_TEST_ROOT_REPEAT";

/// Serializes every use of this process's `RATMAC_TEST_*` variables.
static ROOT_LANE: Mutex<()> = Mutex::new(());

fn root_lane() -> MutexGuard<'static, ()> {
    ROOT_LANE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

// --- shared helpers ----------------------------------------------------------

/// The canonical one-line spelling of a resolved project directory.
fn canonical_line(dir: &Path) -> String {
    fs::canonicalize(dir)
        .unwrap_or_else(|_| dir.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/")
}

/// Every line a resolution log file holds, read strictly. A read error
/// fails except NotFound, where no line is expected yet; the bytes must be
/// UTF-8; the file must be empty or end its last line with `\n`; records
/// split on `\n` alone; and any record that is empty or carries `\r` or a
/// backslash fails, always naming the log.
fn log_lines(log: &Path) -> Vec<String> {
    let bytes = match fs::read(log) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(error) => panic!("{} cannot be read: {error}", rendered(log)),
    };
    let text = String::from_utf8(bytes)
        .unwrap_or_else(|error| panic!("{} holds invalid UTF-8: {error}", rendered(log)));
    if text.is_empty() {
        return Vec::new();
    }
    assert!(
        text.ends_with('\n'),
        "{} must end its last line with a newline",
        rendered(log)
    );
    let lines: Vec<String> = text
        .strip_suffix('\n')
        .unwrap_or(&text)
        .split('\n')
        .map(str::to_owned)
        .collect();
    for line in &lines {
        assert!(
            !line.is_empty() && !line.contains('\r') && !line.contains('\\'),
            "{} must hold one canonical path per line; held {lines:?}",
            rendered(log)
        );
    }
    lines
}

/// The run id a successful command minted, from its own report.
fn minted(text: &str, marker: &str) -> String {
    let tail = text
        .lines()
        .find_map(|line| line.strip_prefix(marker))
        .unwrap_or_else(|| panic!("a minted id follows {marker:?}: {text}"));
    tail.split([' ', '/'])
        .find(|word| !word.is_empty())
        .unwrap_or_else(|| panic!("a minted id follows {marker:?}: {text}"))
        .to_owned()
}

/// The Engine's own path spelling: forward slashes.
fn rendered(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Every plausible text spelling of `path`, so "never named" checks both
/// separators and both the given and the canonical form.
fn spellings(path: &Path) -> Vec<String> {
    let mut found = vec![rendered(path), path.to_string_lossy().into_owned()];
    if let Ok(canonical) = fs::canonicalize(path) {
        found.push(canonical.to_string_lossy().into_owned());
        found.push(rendered(&canonical));
    }
    found
}

/// Run the harness Engine in `directory` with a scrubbed environment.
fn rtm(directory: &Path, args: &[&str]) -> Output {
    support::command(ratmac_qa::engine_bin!(), directory)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("invoke rtm {args:?}: {error}"))
}

/// Run the harness Engine with `RATMAC_TEST_ROOT_LOG` armed at `log`.
fn rtm_logged(directory: &Path, log: &Path, args: &[&str]) -> Output {
    support::command(ratmac_qa::engine_bin!(), directory)
        .args(args)
        .env(LOG_VAR, log)
        .output()
        .unwrap_or_else(|error| panic!("invoke rtm {args:?}: {error}"))
}

/// Assert a resolution log holds exactly `expected` lines.
fn assert_resolutions(label: &str, log: &Path, expected: &[String]) {
    let lines = log_lines(log);
    assert_eq!(
        lines,
        expected,
        "{label}: each distinct addressed project resolves exactly once; log {} held {lines:?}",
        rendered(log)
    );
}

/// The project tree as comparable bytes: every file and directory beneath
/// `root`, without Git storage or modification times.
fn project_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let options = CaptureOptions {
        excluded_directory_names: &[".git"],
        missing_root_is_empty: true,
        ..CaptureOptions::default()
    };
    support::capture(root, options)
        .unwrap_or_else(|error| panic!("capture {}: {error}", root.display()))
        .directories()
}

/// The complete tree, Git storage included, for refusals that must leave
/// every byte of the fixture alone.
fn complete_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    support::capture(root, CaptureOptions::default())
        .unwrap_or_else(|error| panic!("capture {}: {error}", root.display()))
        .directories()
}

/// Every byte of `haystack` with each exact occurrence of `needle` replaced
/// by `replacement`; nothing else moves, so line endings stay as written.
fn replace_bytes(haystack: &[u8], needle: &[u8], replacement: &[u8]) -> Vec<u8> {
    if needle.is_empty() {
        return haystack.to_vec();
    }
    let mut out = Vec::with_capacity(haystack.len());
    let mut at = 0;
    while at + needle.len() <= haystack.len() {
        if &haystack[at..at + needle.len()] == needle {
            out.extend_from_slice(replacement);
            at += needle.len();
        } else {
            out.push(haystack[at]);
            at += 1;
        }
    }
    out.extend_from_slice(&haystack[at..]);
    out
}

/// The fixture's own HEAD revision, as spawn ledgers record it.
fn fixture_revision(label: &str, home: &Path) -> Vec<u8> {
    let output = support::git(home)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap_or_else(|error| panic!("read {label}'s HEAD: {error}"));
    assert!(
        output.status.success(),
        "{label}: git rev-parse HEAD: {}",
        support::text(&output)
    );
    let mut revision = output.stdout;
    while revision.last() == Some(&b'\n') || revision.last() == Some(&b'\r') {
        revision.pop();
    }
    revision
}

/// One fixture's own path spellings - TOML-escaped, plain, and
/// forward-slashed - as exact byte sequences.
fn own_path_spellings(home: &Path) -> [Vec<u8>; 3] {
    let native = home.to_string_lossy().into_owned();
    [
        native.replace('\\', "\\\\").into_bytes(),
        native.into_bytes(),
        rendered(home).into_bytes(),
    ]
}

/// The bytes with every exact occurrence of the fixture's own path
/// spellings replaced by a placeholder; nothing else changes.
fn replace_own_paths(bytes: &[u8], home: &Path) -> Vec<u8> {
    let mut bytes = bytes.to_vec();
    for spelling in &own_path_spellings(home) {
        bytes = replace_bytes(&bytes, spelling, b"<PROJECT>");
    }
    bytes
}

/// One fixture's tree reduced for comparison against another fixture's:
/// only the fixture's own path spellings and, in spawn ledgers alone, the
/// Git revision provenance (first proven to be that fixture's own HEAD)
/// become placeholders.
fn comparable_tree(
    label: &str,
    tree: BTreeMap<String, Vec<u8>>,
    home: &Path,
    revision: &[u8],
) -> BTreeMap<String, Vec<u8>> {
    tree.into_iter()
        .map(|(key, bytes)| {
            let mut bytes = replace_own_paths(&bytes, home);
            if key.ends_with("spawn-ledger") && !bytes.is_empty() {
                let assigned = [b"spawned_at = \"", revision, b"\""].concat();
                assert!(
                    bytes
                        .windows(assigned.len())
                        .any(|window| window == assigned),
                    "{label}: {key} must record its own fixture's HEAD revision"
                );
                bytes = replace_bytes(&bytes, revision, b"<REVISION>");
            }
            (key, bytes)
        })
        .collect()
}

/// One of the two root-hook variables, hidden while a test holds the lane
/// and restored exactly when it releases it.
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

    /// Point the variable at `value` for as long as the returned guard lives.
    fn set(name: &'static str, value: &Path) -> Self {
        let previous = std::env::var_os(name);
        std::env::set_var(name, value);
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

/// Hide any inherited root hooks, and remember what to restore.
fn hide_root_hooks() -> Vec<HookVar> {
    vec![HookVar::hide(LOG_VAR), HookVar::hide(REPEAT_VAR)]
}

/// A fresh log file per measured in-process entry, so every assertion judges
/// exactly one call.
struct Probe {
    logs: TempTree,
    next: usize,
    /// The planted second answer every measured entry must never need.
    repeat: Option<PathBuf>,
}

impl Probe {
    fn new(label: &str, repeat: Option<&Path>) -> Self {
        Self {
            logs: TempTree::new(&format!("t119-{label}-logs")).expect("own a log tree"),
            next: 0,
            repeat: repeat.map(Path::to_path_buf),
        }
    }

    fn fresh_log(&mut self) -> PathBuf {
        self.next += 1;
        self.logs.join(format!("resolution-{:03}.log", self.next))
    }

    fn measure<T>(&mut self, label: &str, project: &Path, body: impl FnOnce() -> T) -> T {
        let log = self.fresh_log();
        let guard = HookVar::set(LOG_VAR, &log);
        let repeat = self
            .repeat
            .clone()
            .map(|answer| HookVar::set(REPEAT_VAR, &answer));
        let outcome = body();
        drop(repeat);
        drop(guard);
        assert_resolutions(
            &format!("WEBV-017: one call to {label}"),
            &log,
            &[canonical_line(project)],
        );
        outcome
    }
}

// --- fixtures ----------------------------------------------------------------

/// The machine every fixture shares: a parent that delegates `rev` children
/// of class `worker`, a join guard after delegating, and a blocked route
/// back out of `delegate` so a live parent can be held.
const MACHINE: &str = r#"
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
guards = [{ kind = "join", require = "all_passed", min = 1 }]

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

/// The command-line fixture's runbook: one declared workflow root.
fn cli_runbook() -> String {
    format!("[roots]\nwork = \"work\"\n{MACHINE}")
}

/// The library fixture's runbook: every fixed contract role declared, so the
/// `contract::*` entries find their roots.
fn library_runbook() -> String {
    format!(
        "[roots]\ngoal = \"goal\"\nissue = \"issue\"\nresidual = \"residual\"\n\
         ticket = \"ticket\"\nwork = \"work\"\n{MACHINE}"
    )
}

/// One Git repository holding the command-line fixture.
fn cli_repo(label: &str) -> TempRepo {
    let repo = TempRepo::new(&format!("t119-{label}"));
    repo.write(".ratmac/ratmac.toml", &cli_runbook());
    repo.write(
        "work/blocker.txt",
        "Opaque blocker; its content is not a gate.\n",
    );
    repo.write("work/item.md", "Contributor-owned original bytes.\n");
    repo.stage(".ratmac/ratmac.toml");
    repo.stage("work/blocker.txt");
    repo.stage("work/item.md");
    repo.commit_all("t-119 fixture base");
    repo
}

/// One Git repository holding the library fixture, with the mirrored roots a
/// workspace-bound child resolves and a spawn workspace inside the
/// repository.
fn library_repo(label: &str) -> TempRepo {
    let repo = TempRepo::new(&format!("t119-{label}"));
    repo.write(".ratmac/ratmac.toml", &library_runbook());
    repo.write(
        "goal/spec.md",
        "# Goal\n\nWEB-005: one resolution per project.\n",
    );
    repo.write(
        "issue/i-001-web-005/spec.md",
        "# One resolution per project.\n",
    );
    repo.write("residual/res-005.md", "---\nstatus: missing\n---\n");
    repo.write("ticket/t-001/item.md", "# Ticket item.\n");
    repo.write(
        "work/blocker.txt",
        "Opaque blocker; its content is not a gate.\n",
    );
    repo.write("work/item.md", "Contributor-owned original bytes.\n");
    for role in ["goal", "issue", "residual", "ticket", "work"] {
        repo.write(
            &format!("area/{role}/keeper.md"),
            &format!("# Workspace root {role} exists.\n"),
        );
    }
    repo.git(&["add", "--all"]);
    repo.commit_all("t-119 library fixture base");
    repo
}

/// WEBV-017: every route and public path-taking entry resolves its invoking
/// project exactly once; an opened `Scheduler`'s later methods add nothing;
/// bare `rtm`, `--help`, and an unsupported command resolve nothing.
#[test]
fn webv_017() {
    let _lane = root_lane();
    let _hooks = hide_root_hooks();
    if std::env::var_os("T119_ABANDON_CHILD").is_some() {
        abandon_spelling_child();
        std::process::exit(0);
    }
    abandon_spelling_names_the_callers_project();
    command_routes_log_one_resolution();
    let plain = library_entries_log_one_resolution("unset", None);
    let decoy = TempTree::new("t119-library-repeat").expect("own the planted library answer");
    let other_root = library_entries_log_one_resolution("another runtime root", Some(decoy.path()));
    assert_eq!(
        other_root, plain,
        "WEBV-017: with RATMAC_TEST_ROOT_REPEAT set to another runtime root, every library entry writes as without it"
    );
    assert_eq!(
        project_tree(decoy.path()),
        BTreeMap::new(),
        "WEBV-017: the planted library answer stays byte-identical"
    );
    let failing =
        library_entries_log_one_resolution("a failing second resolution", Some(Path::new("fail")));
    assert_eq!(
        failing, plain,
        "WEBV-017: with RATMAC_TEST_ROOT_REPEAT set to fail, every library entry writes as without it"
    );
}

/// WEBV-017, abandon naming: the Engine names the project from the caller's
/// own spelling. An empty spelling means the process working directory, so
/// that half runs in a child process over a throwaway Run-less, lock-less
/// fixture - never inside this repository.
fn abandon_spelling_names_the_callers_project() {
    let fixture = TempTree::new("t119-abandon-spelling").expect("own the abandon fixture");
    let marker = fixture.path().join("checked");
    let output = support::command(
        std::env::current_exe().expect("locate this test binary"),
        fixture.path(),
    )
    .args(["--exact", "webv_017", "--test-threads=1"])
    .env("T119_ABANDON_CHILD", "1")
    .env("T119_ABANDON_MARKER", &marker)
    .output()
    .expect("spawn the abandon spelling child");
    let text = support::text(&output);
    assert!(
        output.status.success(),
        "WEBV-017: the abandon spelling child passes: {text}"
    );
    assert_eq!(
        fs::read_to_string(&marker).unwrap_or_default(),
        "abandon this project\nnothing to retire in this project\n",
        "WEBV-017: the child checked both spellings"
    );
}

/// The child half of the abandon naming check, with the working directory
/// inside the throwaway fixture.
fn abandon_spelling_child() {
    let root = Path::new("");
    let phrase = ratmac::abandon::required_phrase(root, None);
    assert_eq!(
        phrase, "abandon this project",
        "WEBV-017: required_phrase names the caller's project"
    );
    let refusal = resolve_target(root, None)
        .expect_err("a Run-less, lock-less project refuses retirement addressing");
    let reason = refusal.to_string();
    assert!(
        reason.contains("nothing to retire in this project"),
        "WEBV-017: the refusal names the project: {reason}"
    );
    if let Some(marker) = std::env::var_os("T119_ABANDON_MARKER") {
        fs::write(
            marker,
            "abandon this project\nnothing to retire in this project\n",
        )
        .expect("mark the child's check");
    }
}

/// The CLI half of WEBV-017, in one fresh repository.
fn command_routes_log_one_resolution() {
    let repo = cli_repo("routes");
    let root = repo.root();
    let logs = TempTree::new("t119-routes-logs").expect("own a log tree");
    let checkout = canonical_line(root);
    let mut fresh = LogCounter {
        logs: &logs,
        next: 0,
    };
    let log = fresh.next_log();

    // start, success: the positive control every later route builds on.
    let output = rtm_logged(root, &log, &["start"]);
    let text = support::text(&output);
    assert!(
        output.status.success(),
        "WEBV-017: `rtm start` must succeed so its resolution is observable: {text}"
    );
    let parent = minted(&text, "rtm: started run ");
    assert!(
        root.join(".ratmac/runs")
            .join(&parent)
            .join("run.toml")
            .is_file(),
        "WEBV-017: `rtm start` must mint its Run under the invoking checkout"
    );
    assert_resolutions(
        "WEBV-017: `rtm start`",
        &log,
        std::slice::from_ref(&checkout),
    );

    // step, success.
    let log = fresh.next_log();
    let output = rtm_logged(root, &log, &["step", "--run", &parent]);
    assert!(
        output.status.success() && support::text(&output).contains("Delegate and wait."),
        "WEBV-017: `rtm step` must advance the started Run: {}",
        support::text(&output)
    );
    assert_resolutions(
        "WEBV-017: `rtm step`",
        &log,
        std::slice::from_ref(&checkout),
    );

    // spawn, success.
    let log = fresh.next_log();
    let output = rtm_logged(
        root,
        &log,
        &[
            "spawn",
            "rev",
            "--run",
            &parent,
            "--bind",
            "ticket=WEBV-017",
        ],
    );
    let text = support::text(&output);
    assert!(
        output.status.success(),
        "WEBV-017: `rtm spawn` must succeed: {text}"
    );
    let child = minted(&text, "rtm: spawned run ");
    assert_resolutions(
        "WEBV-017: `rtm spawn`",
        &log,
        std::slice::from_ref(&checkout),
    );

    // A passed child: step it to its terminal State before reading it.
    let output = rtm(root, &["step", "--run", &child]);
    assert!(
        output.status.success() && support::text(&output).contains("Child finished."),
        "WEBV-017: the child must pass before status reads it as history: {}",
        support::text(&output)
    );

    // status, a passed Run read as history.
    let log = fresh.next_log();
    let output = rtm_logged(root, &log, &["status", "--run", &child]);
    let text = support::text(&output);
    assert!(
        output.status.success()
            && text.contains("Engine root: ")
            && text.contains("passed")
            && text.contains("shown from its record"),
        "WEBV-017: `rtm status` must read the passed Run as history: {text}"
    );
    assert_resolutions(
        "WEBV-017: `rtm status` of a passed Run",
        &log,
        std::slice::from_ref(&checkout),
    );

    // status, live.
    let log = fresh.next_log();
    let output = rtm_logged(root, &log, &["status", "--run", &parent]);
    let text = support::text(&output);
    assert!(
        output.status.success() && text.contains("Delegate and wait."),
        "WEBV-017: `rtm status` must report the live Run: {text}"
    );
    assert_resolutions(
        "WEBV-017: `rtm status` of a live Run",
        &log,
        std::slice::from_ref(&checkout),
    );

    // hold, success twin.
    let output = rtm(root, &["start"]);
    let held = minted(&support::text(&output), "rtm: started run ");
    let output = rtm(root, &["step", "--run", &held]);
    assert!(
        output.status.success(),
        "WEBV-017: the held Run must reach its blocked-route State: {}",
        support::text(&output)
    );
    let log = fresh.next_log();
    let output = rtm_logged(
        root,
        &log,
        &[
            "hold",
            "--run",
            &held,
            "--blocker",
            "work/blocker.txt",
            "--confirm",
            &format!("hold {held}"),
        ],
    );
    let text = support::text(&output);
    assert!(
        output.status.success() && text.contains("paused against work/blocker.txt"),
        "WEBV-017: `rtm hold` must pause the Run: {text}"
    );
    assert_resolutions(
        "WEBV-017: `rtm hold`",
        &log,
        std::slice::from_ref(&checkout),
    );

    // hold, refusal twin.
    let output = rtm(root, &["start"]);
    let unheld = minted(&support::text(&output), "rtm: started run ");
    let log = fresh.next_log();
    let output = rtm_logged(
        root,
        &log,
        &[
            "hold",
            "--run",
            &unheld,
            "--confirm",
            &format!("hold {unheld}"),
        ],
    );
    let text = support::text(&output);
    assert!(
        !output.status.success() && text.contains("hold refused"),
        "WEBV-017: a refusing `rtm hold` must refuse by its known first line: {text}"
    );
    assert_resolutions(
        "WEBV-017: a refusing `rtm hold`",
        &log,
        std::slice::from_ref(&checkout),
    );

    // respawn, success.
    let output = rtm(root, &["start"]);
    let superseded = minted(&support::text(&output), "rtm: started run ");
    let log = fresh.next_log();
    let output = rtm_logged(
        root,
        &log,
        &[
            "respawn",
            "--run",
            &superseded,
            "--confirm",
            &format!("respawn {superseded}"),
        ],
    );
    let text = support::text(&output);
    assert!(
        output.status.success() && text.contains("successor run "),
        "WEBV-017: `rtm respawn` must mint a successor: {text}"
    );
    assert_resolutions(
        "WEBV-017: `rtm respawn`",
        &log,
        std::slice::from_ref(&checkout),
    );

    // abandon, success twin.
    let output = rtm(root, &["start"]);
    let retired = minted(&support::text(&output), "rtm: started run ");
    let log = fresh.next_log();
    let output = rtm_logged(
        root,
        &log,
        &[
            "abandon",
            "--run",
            &retired,
            "--confirm",
            &format!("abandon {retired}"),
        ],
    );
    let text = support::text(&output);
    assert!(
        output.status.success() && text.contains("Run abandoned"),
        "WEBV-017: `rtm abandon` must retire the Run: {text}"
    );
    assert_resolutions(
        "WEBV-017: `rtm abandon`",
        &log,
        std::slice::from_ref(&checkout),
    );

    // abandon, refusal twin.
    let output = rtm(root, &["start"]);
    let kept = minted(&support::text(&output), "rtm: started run ");
    let log = fresh.next_log();
    let output = rtm_logged(
        root,
        &log,
        &["abandon", "--run", &kept, "--confirm", "abandon nothing"],
    );
    let text = support::text(&output);
    assert!(
        !output.status.success() && text.contains("abandon refused"),
        "WEBV-017: a refusing `rtm abandon` must refuse by its known first line: {text}"
    );
    assert_resolutions(
        "WEBV-017: a refusing `rtm abandon`",
        &log,
        std::slice::from_ref(&checkout),
    );

    // start, refusal twin.
    let log = fresh.next_log();
    let output = rtm_logged(root, &log, &["start", "extra"]);
    let text = support::text(&output);
    assert!(
        !output.status.success() && text.contains("accepts no run-id"),
        "WEBV-017: a refusing `rtm start` must refuse by its known first line: {text}"
    );
    assert_resolutions(
        "WEBV-017: a refusing `rtm start`",
        &log,
        std::slice::from_ref(&checkout),
    );

    // doctor, human.
    let log = fresh.next_log();
    let output = rtm_logged(root, &log, &["doctor"]);
    let text = support::text(&output);
    assert!(
        text.contains(&format!("Engine root: {}", rendered(&root.join(".ratmac")))),
        "WEBV-017: `rtm doctor` must report the resolved Engine root: {text}"
    );
    assert_resolutions(
        "WEBV-017: `rtm doctor`",
        &log,
        std::slice::from_ref(&checkout),
    );

    // doctor, structured.
    let log = fresh.next_log();
    let output = rtm_logged(root, &log, &["doctor", "--json"]);
    let text = support::text(&output);
    let engine_root = Json::parse(&text)
        .ok()
        .and_then(|document| {
            document
                .as_object()
                .and_then(|object| object.get("engine_root"))
                .and_then(Json::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default();
    assert_eq!(
        engine_root,
        rendered(&root.join(".ratmac")),
        "WEBV-017: `rtm doctor --json` must report the resolved Engine root: {text}"
    );
    assert_resolutions(
        "WEBV-017: `rtm doctor --json`",
        &log,
        std::slice::from_ref(&checkout),
    );

    // skill, same directory: the invoking checkout alone.
    let log = fresh.next_log();
    let output = rtm_logged(root, &log, &["skill", "operator-skill"]);
    let text = support::text(&output);
    assert!(
        output.status.success() && text.contains("Wrote "),
        "WEBV-017: `rtm skill` in the same directory must succeed: {text}"
    );
    assert_resolutions(
        "WEBV-017: `rtm skill` in the same directory",
        &log,
        std::slice::from_ref(&checkout),
    );

    // skill into another project: the invoking checkout and the path's
    // project, each once.
    let other = TempTree::new("t119-skill-other").expect("own the other skill project");
    fs::create_dir_all(other.path().join("project")).expect("create the other project directory");
    let log = fresh.next_log();
    let output = support::command(ratmac_qa::engine_bin!(), root)
        .args(["skill"])
        .arg(other.path().join("project/operator-skill"))
        .env(LOG_VAR, &log)
        .output()
        .expect("invoke rtm skill in another project");
    let text = support::text(&output);
    assert!(
        output.status.success() && text.contains("Wrote "),
        "WEBV-017: `rtm skill` into another project must succeed: {text}"
    );
    let mut expected = vec![
        checkout.clone(),
        canonical_line(&other.path().join("project")),
    ];
    let mut lines = log_lines(&log);
    expected.sort();
    lines.sort();
    assert_eq!(
        lines, expected,
        "WEBV-017: `rtm skill <path>` resolves the invoking checkout and the path's project, each once"
    );

    // doctor --json over an addressed runbook: the runbook's project alone.
    let log = fresh.next_log();
    let output = rtm_logged(root, &log, &["doctor", "--json", ".ratmac/ratmac.toml"]);
    let text = support::text(&output);
    let engine_root = Json::parse(&text)
        .ok()
        .and_then(|document| {
            document
                .as_object()
                .and_then(|object| object.get("engine_root"))
                .and_then(Json::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default();
    assert_eq!(
        engine_root,
        rendered(&root.join(".ratmac")),
        "WEBV-017: `rtm doctor --json <runbook>` must report the resolved Engine root: {text}"
    );
    assert_resolutions(
        "WEBV-017: `rtm doctor --json <runbook>`",
        &log,
        std::slice::from_ref(&checkout),
    );

    // bare rtm, --help, an unsupported command: no resolution at all.
    for args in [&[] as &[&str], &["--help"], &["frobnicate"]] {
        let log = fresh.next_log();
        let output = rtm_logged(root, &log, args);
        let text = support::text(&output);
        let refused = args.contains(&"frobnicate");
        assert_eq!(
            output.status.success(),
            !refused,
            "WEBV-017: `rtm {args:?}` keeps its known exit: {text}"
        );
        let lines = log_lines(&log);
        assert!(
            lines.is_empty(),
            "WEBV-017: `rtm {args:?}` resolves nothing; log held {lines:?}"
        );
    }
}

/// Names fresh log files under one owned tree.
struct LogCounter<'a> {
    logs: &'a TempTree,
    next: usize,
}

impl LogCounter<'_> {
    fn next_log(&mut self) -> PathBuf {
        self.next += 1;
        self.logs.join(format!("route-{:03}.log", self.next))
    }
}

/// The in-process library half of WEBV-017, in one fresh repository per
/// variant. With a planted answer armed, every entry must still resolve its
/// project exactly once - so the answer never fires - and the run returns
/// the fixture's tree, reduced for comparison against the other variants.
fn library_entries_log_one_resolution(
    variant: &str,
    answer: Option<&Path>,
) -> BTreeMap<String, Vec<u8>> {
    let repo = library_repo(&format!("library-{}", variant.replace(' ', "-")));
    let root = repo.root().to_path_buf();
    let runbook = root.join(".ratmac/ratmac.toml");
    let engine_root = root.join(".ratmac");
    let checkout = canonical_line(&root);
    let bindings = |ticket: &str| BTreeMap::from([("ticket".to_owned(), ticket.to_owned())]);
    let mut probe = Probe::new("library", answer);

    // Fixture setup, unmeasured: one parent waiting in its spawning State.
    let parent = Scheduler::open(&root)
        .expect("the library fixture opens")
        .start()
        .expect("the library fixture starts a parent")
        .id()
        .expect("a started Run has an id")
        .to_owned();
    Scheduler::open_run(&root, &parent)
        .expect("the parent opens")
        .step(StepRequest::new("fixture prerequisite is complete"))
        .expect("the parent reaches its spawning State");

    // Scheduler::open, then the methods an opened Scheduler offers: the one
    // resolution open made is all any of them may use.
    let log = probe.fresh_log();
    let guard = HookVar::set(LOG_VAR, &log);
    let repeat = answer.map(|answer| HookVar::set(REPEAT_VAR, answer));
    let mut opened = Scheduler::open(&root).expect("the project opens");
    assert_resolutions(
        "WEBV-017: Scheduler::open",
        &log,
        std::slice::from_ref(&checkout),
    );
    let spawned_parent = opened
        .start()
        .expect("a method on an opened Scheduler starts a Run")
        .id()
        .expect("a started Run has an id")
        .to_owned();
    assert_resolutions(
        "WEBV-017: Scheduler::start after Scheduler::open",
        &log,
        std::slice::from_ref(&checkout),
    );
    opened
        .status()
        .expect("a method on an opened Scheduler reports status");
    assert_resolutions(
        "WEBV-017: Scheduler::status after Scheduler::open",
        &log,
        std::slice::from_ref(&checkout),
    );
    opened
        .step(StepRequest::new("a method on an opened Scheduler steps"))
        .expect("a method on an opened Scheduler steps its Run");
    assert_resolutions(
        "WEBV-017: Scheduler::step after Scheduler::open",
        &log,
        std::slice::from_ref(&checkout),
    );
    opened
        .spawn_with_bindings("rev", &bindings("WEBV-017-c"))
        .expect("an opened Scheduler's spawn method spawns its child");
    assert_resolutions(
        "WEBV-017: Scheduler::spawn_with_bindings after Scheduler::open",
        &log,
        std::slice::from_ref(&checkout),
    );
    opened
        .spawn("rev")
        .expect("an opened Scheduler's unbound spawn method still spawns");
    assert_resolutions(
        "WEBV-017: Scheduler::spawn after Scheduler::open",
        &log,
        std::slice::from_ref(&checkout),
    );
    drop(repeat);
    drop(guard);

    // Scheduler::open_run.
    probe.measure("Scheduler::open_run", &root, || {
        Scheduler::open_run(&root, &parent).expect("the addressed Run opens")
    });

    // Scheduler::spawn_to.
    let first_child = probe.measure("Scheduler::spawn_to", &root, || {
        Scheduler::spawn_to(&root, &parent, "rev", &bindings("WEBV-017-a"))
            .expect("the declared child spawns")
    });

    // Scheduler::spawn_to_with_workspace: the workspace never resolves.
    let workspace = root.join("area");
    probe.measure("Scheduler::spawn_to_with_workspace", &root, || {
        Scheduler::spawn_to_with_workspace(
            &root,
            &parent,
            "rev",
            &bindings("WEBV-017-b"),
            Some(&workspace),
        )
        .expect("the declared child spawns into its workspace")
    });

    // A passed Run for cli::run_from to read as history.
    Scheduler::open_run(&root, &first_child)
        .expect("the child opens")
        .step(StepRequest::new("the child finishes its work"))
        .expect("the child reaches its terminal State");

    // cli::run_from: the same command line the CLI route would log.
    let mut report = Vec::new();
    let code = probe.measure("cli::run_from", &root, || {
        cli::run_from(
            ["status", "--run", first_child.as_str()],
            &root,
            &mut report,
        )
        .expect("the in-process command renders")
    });
    assert_eq!(code, 0, "WEBV-017: run_from reports its exit code");
    assert!(
        String::from_utf8_lossy(&report).contains("shown from its record"),
        "WEBV-017: run_from reads the passed Run as history"
    );

    // Scheduler::runs_dir and Scheduler::run_roster.
    let runs = probe.measure("Scheduler::runs_dir", &root, || Scheduler::runs_dir(&root));
    assert_eq!(
        runs,
        root.join(".ratmac/runs"),
        "WEBV-017: runs_dir names the resolved Engine root's roster"
    );
    let roster = probe.measure("Scheduler::run_roster", &root, || {
        Scheduler::run_roster(&root).expect("the roster reads")
    });
    assert!(
        roster.contains(&parent) && roster.contains(&first_child),
        "WEBV-017: the roster holds the fixture's Runs: {roster:?}"
    );

    // Scheduler::respawn.
    let successor = probe.measure("Scheduler::respawn", &root, || {
        Scheduler::respawn(
            &root,
            &RespawnRequest {
                run: Some(spawned_parent.clone()),
                confirmation: Some(format!("respawn {spawned_parent}")),
            },
        )
        .expect("the live Run is superseded")
    });
    assert_ne!(successor, spawned_parent, "respawn mints a fresh id");

    // abandon::resolve_target, plan_abandon, apply_abandon.
    let retired = Scheduler::open(&root)
        .expect("the project opens")
        .start()
        .expect("a Run to abandon starts")
        .id()
        .expect("a started Run has an id")
        .to_owned();
    probe.measure("abandon::resolve_target", &root, || {
        resolve_target(&root, Some(&retired)).expect("the addressed Run resolves")
    });
    let plan = probe.measure("abandon::plan_abandon", &root, || {
        plan_abandon(
            &root,
            &AbandonRequest {
                confirmation: Some(format!("abandon {retired}")),
                run: Some(retired.clone()),
            },
        )
        .expect("the confirmed abandonment plans")
    });
    probe.measure("abandon::apply_abandon", &root, || {
        apply_abandon(&root, &plan).expect("the planned abandonment applies")
    });

    // blocked::plan_hold, apply_hold: a live Run in its blocked-route State.
    let held = Scheduler::open(&root)
        .expect("the project opens")
        .start()
        .expect("a Run to hold starts")
        .id()
        .expect("a started Run has an id")
        .to_owned();
    Scheduler::open_run(&root, &held)
        .expect("the Run opens")
        .step(StepRequest::new("the Run reaches its blocked-route State"))
        .expect("the Run waits in its spawning State");
    let hold_plan = probe.measure("blocked::plan_hold", &root, || {
        plan_hold(
            &root,
            &HoldRequest {
                run: Some(held.clone()),
                blocker: Some("work/blocker.txt".to_owned()),
                confirmation: Some(format!("hold {held}")),
            },
        )
        .expect("the confirmed hold plans")
    });
    probe.measure("blocked::apply_hold", &root, || {
        apply_hold(&root, &hold_plan).expect("the planned hold applies")
    });

    // doctor::diagnose.
    let _ = probe.measure("doctor::diagnose", &root, || doctor::diagnose(&runbook));

    // scaffold::write_scaffold and skill::write_skill.
    probe.measure("scaffold::write_scaffold", &root, || {
        write_scaffold(&root.join("probe-runbook.toml"))
            .expect("a fresh path accepts one scaffolded runbook")
    });
    probe.measure("skill::write_skill", &root, || {
        write_skill(&root.join("probe-skill")).expect("a fresh path accepts one skill folder")
    });

    // machine::MachineClass::load_from_project_root.
    probe.measure(
        "machine::MachineClass::load_from_project_root",
        &root,
        || MachineClass::load_from_project_root(&root).expect("the fixture's Machine Class loads"),
    );

    // The contract entries: each resolves the project it is handed.
    let _ = probe.measure("contract::gate_intake", &root, || {
        contract::gate_intake(&root)
    });
    let _ = probe.measure("contract::work_items", &root, || {
        contract::work_items(&root)
    });
    let _ = probe.measure("contract::gate_records", &root, || {
        contract::gate_records(&root, &engine_root, &first_child)
    });
    let _ = probe.measure("contract::unproven_mechanization", &root, || {
        contract::unproven_mechanization(&root)
    });

    let revision = fixture_revision("the library fixture", &root);
    comparable_tree(
        "WEBV-017: the library fixture",
        project_tree(&root),
        &root,
        &revision,
    )
}

/// WEBV-018: with `RATMAC_TEST_ROOT_REPEAT` set to another directory, and
/// separately to `fail`, every route prints, exits, and writes exactly what
/// it does without the variable; the other directory stays byte-identical
/// and is never named; the armed log proves exactly one resolution happened.
#[test]
fn webv_018() {
    let _lane = root_lane();
    let _hooks = hide_root_hooks();
    let other = TempTree::new("t119-repeat-other").expect("own the other runtime root");
    divergent_answers("another runtime root", other.path(), Some(&other));
    divergent_answers("a failing second resolution", Path::new("fail"), None);
    calibrate_the_hook();
}

/// WEBV-018, calibration of the hook itself: two read-only calls through one
/// log file append two identical lines; with `RATMAC_TEST_ROOT_REPEAT` set,
/// only the second resolution answers with it - another directory becomes
/// the reported Engine root, `fail` panics naming the project - while the
/// first call still answers the truth. Only the fixed Engine has the hook.
fn calibrate_the_hook() {
    let repo = cli_repo("calibrate");
    let root = repo.root();
    let logs = TempTree::new("t119-calibrate-logs").expect("own a log tree");
    let checkout = canonical_line(root);
    let real = rendered(&root.join(".ratmac"));
    let engine_root_line = |text: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix("Engine root: "))
            .unwrap_or_default()
            .to_owned()
    };

    // Two read-only calls, one log, no planted answer: two identical lines.
    let log = logs.join("twice.log");
    for _ in 0..2 {
        let output = rtm_logged(root, &log, &["doctor"]);
        let text = support::text(&output);
        assert!(
            output.status.success(),
            "WEBV-018: the calibration `rtm doctor` succeeds: {text}"
        );
        assert_eq!(
            engine_root_line(&text),
            real,
            "WEBV-018: an unplanted resolution answers the truth: {text}"
        );
    }
    assert_eq!(
        log_lines(&log),
        vec![checkout.clone(), checkout.clone()],
        "WEBV-018: two calls through one log file append two identical lines"
    );

    // A planted directory: only the second resolution answers with it.
    let decoy = TempTree::new("t119-calibrate-decoy").expect("own the planted answer");
    let planted = rendered(decoy.path());
    let log = logs.join("planted.log");
    let planted_doctor = |log: &Path| {
        support::command(ratmac_qa::engine_bin!(), root)
            .args(["doctor"])
            .env(LOG_VAR, log)
            .env(REPEAT_VAR, decoy.path())
            .output()
            .expect("invoke the planted rtm doctor")
    };
    let text = support::text(&planted_doctor(&log));
    assert_eq!(
        engine_root_line(&text),
        real,
        "WEBV-018: the first resolution ignores the planted answer: {text}"
    );
    let text = support::text(&planted_doctor(&log));
    assert_eq!(
        engine_root_line(&text),
        planted,
        "WEBV-018: the second resolution answers with the planted directory: {text}"
    );
    assert_eq!(
        log_lines(&log),
        vec![checkout.clone(), checkout.clone()],
        "WEBV-018: the planted pair still appends exactly two lines"
    );

    // `fail`: only the second resolution panics, naming the project.
    let failing_doctor = |log: &Path| {
        support::command(ratmac_qa::engine_bin!(), root)
            .args(["doctor"])
            .env(LOG_VAR, log)
            .env(REPEAT_VAR, "fail")
            .output()
            .expect("invoke the failing rtm doctor")
    };
    let log = logs.join("fail.log");
    let first = failing_doctor(&log);
    assert!(
        first.status.success(),
        "WEBV-018: the first resolution does not fail: {}",
        support::text(&first)
    );
    let second = failing_doctor(&log);
    let stderr = String::from_utf8_lossy(&second.stderr).into_owned();
    assert!(
        second.status.code() == Some(101) && stderr.contains(&checkout),
        "WEBV-018: the second resolution terminates in a panic that names the project: {stderr}"
    );
    assert_eq!(
        log_lines(&log),
        vec![checkout.clone(), checkout],
        "WEBV-018: the panicking resolution still appends its line"
    );
}

/// One variant of WEBV-018 over twin fixtures: the control runs without the
/// variables, the measured runs carry the log and the divergent answer.
fn divergent_answers(variant: &str, answer: &Path, other: Option<&TempTree>) {
    let control = cli_repo("twin-control");
    let measured = cli_repo("twin-measured");
    let control_revision = fixture_revision("the control twin", control.root());
    let measured_revision = fixture_revision("the measured twin", measured.root());
    let logs = TempTree::new("t119-twin-logs").expect("own a log tree");
    let routes: &[(&[&str], bool, &str)] = &[
        (&["start"], true, "rtm: started run run-001"),
        (&["status", "--run", "run-001"], true, "Engine root: "),
        (&["step", "--run", "run-001"], true, "Delegate and wait."),
        (
            &[
                "spawn",
                "rev",
                "--run",
                "run-001",
                "--bind",
                "ticket=WEBV-018",
            ],
            true,
            "rtm: spawned run run-002",
        ),
        (&["step", "--run", "run-002"], true, "Child finished."),
        (
            &["status", "--run", "run-002"],
            true,
            "shown from its record",
        ),
        (
            &[
                "hold",
                "--run",
                "run-001",
                "--blocker",
                "work/blocker.txt",
                "--confirm",
                "hold run-001",
            ],
            true,
            "paused against work/blocker.txt",
        ),
        (&["start"], true, "rtm: started run run-003"),
        (
            &[
                "respawn",
                "--run",
                "run-003",
                "--confirm",
                "respawn run-003",
            ],
            true,
            "successor run ",
        ),
        (
            &["hold", "--run", "run-004", "--confirm", "hold run-004"],
            false,
            "hold refused",
        ),
        (
            &[
                "abandon",
                "--run",
                "run-004",
                "--confirm",
                "abandon run-004",
            ],
            true,
            "Run abandoned",
        ),
        (&["scaffold", "probe.toml"], true, "Wrote probe.toml."),
        (&["skill", "operator-skill"], true, "Wrote "),
        (&["doctor"], true, "Engine root: "),
        (&["doctor", "--json"], true, "engine_root"),
        (&["doctor", ".ratmac/ratmac.toml"], true, "Engine root: "),
        (
            &["doctor", "--json", ".ratmac/ratmac.toml"],
            true,
            "engine_root",
        ),
    ];
    for (step, route) in routes.iter().enumerate() {
        let (args, must_succeed, needle) = *route;
        let log = logs.join(format!("route-{:03}.log", step + 1));
        let plain = rtm(control.root(), args);
        let plain_text = support::text(&plain);
        assert_eq!(
            plain.status.success(),
            must_succeed,
            "WEBV-018: the control `rtm {args:?}` keeps its known verdict: {plain_text}"
        );
        assert!(
            plain_text.contains(needle),
            "WEBV-018: the control `rtm {args:?}` keeps its known line: {plain_text}"
        );
        let output = support::command(ratmac_qa::engine_bin!(), measured.root())
            .args(args)
            .env(LOG_VAR, &log)
            .env(REPEAT_VAR, answer)
            .output()
            .unwrap_or_else(|error| panic!("invoke rtm {args:?}: {error}"));
        let text = support::text(&output);
        assert_eq!(
            output.status.code(),
            plain.status.code(),
            "WEBV-018: with RATMAC_TEST_ROOT_REPEAT set to {variant}, `rtm {args:?}` keeps its exit code; output was:\n{text}"
        );
        assert_eq!(
            replace_own_paths(&output.stdout, measured.root()),
            replace_own_paths(&plain.stdout, control.root()),
            "WEBV-018: with RATMAC_TEST_ROOT_REPEAT set to {variant}, `rtm {args:?}` prints the same stdout; output was:\n{text}"
        );
        assert_eq!(
            replace_own_paths(&output.stderr, measured.root()),
            replace_own_paths(&plain.stderr, control.root()),
            "WEBV-018: with RATMAC_TEST_ROOT_REPEAT set to {variant}, `rtm {args:?}` prints the same stderr; output was:\n{text}"
        );
        assert_eq!(
            comparable_tree(
                "WEBV-018: the measured twin",
                project_tree(measured.root()),
                measured.root(),
                &measured_revision,
            ),
            comparable_tree(
                "WEBV-018: the control twin",
                project_tree(control.root()),
                control.root(),
                &control_revision,
            ),
            "WEBV-018: with RATMAC_TEST_ROOT_REPEAT set to {variant}, `rtm {args:?}` writes byte-identically"
        );
        if let Some(other) = other {
            assert_eq!(
                project_tree(other.path()),
                BTreeMap::new(),
                "WEBV-018: the other runtime root stays byte-identical"
            );
            for spelling in spellings(other.path()) {
                assert!(
                    !text.contains(&spelling),
                    "WEBV-018: the other runtime root is never named by `rtm {args:?}`: {text}"
                );
            }
        }
        assert_resolutions(
            &format!("WEBV-018: with RATMAC_TEST_ROOT_REPEAT set to {variant}, `rtm {args:?}`"),
            &log,
            &[canonical_line(measured.root())],
        );
    }
}

/// WEBV-019: the scope rules. A linked worktree resolves itself, reports and
/// writes the primary runtime root, and reads its own runbook; a spawn
/// workspace is judged against the resolved repository and never resolved; a
/// workspace in another repository refuses with nothing written; without Git
/// the invoking checkout is the whole repository; an addressed runbook's
/// project is the only other project doctor resolves; scaffold resolves the
/// invoking checkout and the path's project, once each.
#[test]
fn webv_019() {
    let _lane = root_lane();
    let _hooks = hide_root_hooks();
    let tree = TempTree::new("t119-scope").expect("own the scope fixture tree");
    let primary = tree.join("primary");
    let linked = tree.join("linked");
    for directory in [
        primary.join(".ratmac"),
        primary.join("work"),
        primary.join("area/work"),
    ] {
        fs::create_dir_all(&directory)
            .unwrap_or_else(|error| panic!("create {}: {error}", directory.display()));
    }
    fs::write(primary.join(".ratmac/ratmac.toml"), cli_runbook())
        .expect("write the primary runbook");
    fs::write(
        primary.join("work/blocker.txt"),
        "Opaque blocker; its content is not a gate.\n",
    )
    .expect("write the blocker leaf");
    fs::write(primary.join("work/item.md"), "Contributor-owned bytes.\n")
        .expect("write the work leaf");
    fs::write(
        primary.join("area/work/blocker.txt"),
        "Workspace blocker.\n",
    )
    .expect("write the workspace work leaf");
    let identity = ["-c", "user.email=qa@example.invalid", "-c", "user.name=QA"];
    for args in [
        &["init", "-q"][..],
        &["add", "--all"][..],
        &["commit", "-q", "-m", "t-119 scope base"][..],
    ] {
        let output = support::git(&primary)
            .args(identity)
            .args(args)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            support::text(&output)
        );
    }
    let output = support::git(&primary)
        .args(["worktree", "add", "-q", "-b", "linked"])
        .arg(&linked)
        .output()
        .expect("create the linked worktree");
    assert!(
        output.status.success(),
        "git worktree add: {}",
        support::text(&output)
    );
    fs::write(
        linked.join(".ratmac/ratmac.toml"),
        cli_runbook().replace("Plan the cycle.", "LINKED: plan the cycle."),
    )
    .expect("give the linked checkout its own runbook bytes");

    let logs = TempTree::new("t119-scope-logs").expect("own a log tree");
    let mut fresh = LogCounter {
        logs: &logs,
        next: 0,
    };

    // The primary checkout: one resolution, its own runtime root.
    let log = fresh.next_log();
    let output = rtm_logged(&primary, &log, &["start"]);
    let text = support::text(&output);
    assert!(
        output.status.success(),
        "WEBV-019: `rtm start` in the primary checkout must succeed: {text}"
    );
    let parent = minted(&text, "rtm: started run ");
    assert!(
        primary
            .join(".ratmac/runs")
            .join(&parent)
            .join("run.toml")
            .is_file(),
        "WEBV-019: the primary checkout writes its own runtime root"
    );
    assert_resolutions(
        "WEBV-019: `rtm start` in the primary checkout",
        &log,
        &[canonical_line(&primary)],
    );

    // The linked worktree: resolves itself, writes the primary runtime root.
    let log = fresh.next_log();
    let output = rtm_logged(&linked, &log, &["start"]);
    let text = support::text(&output);
    assert!(
        output.status.success(),
        "WEBV-019: `rtm start` in the linked worktree must succeed: {text}"
    );
    let linked_run = minted(&text, "rtm: started run ");
    assert!(
        primary
            .join(".ratmac/runs")
            .join(&linked_run)
            .join("run.toml")
            .is_file(),
        "WEBV-019: the linked worktree writes the primary runtime root"
    );
    assert!(
        !linked.join(".ratmac/runs").exists(),
        "WEBV-019: the linked checkout keeps no roster of its own"
    );
    assert_resolutions(
        "WEBV-019: `rtm start` in the linked worktree",
        &log,
        &[canonical_line(&linked)],
    );

    // The linked worktree reports the primary runtime root and its own
    // runbook bytes.
    let output = rtm(&linked, &["status", "--run", &linked_run]);
    let text = support::text(&output);
    assert!(
        output.status.success()
            && text.contains(&format!(
                "Engine root: {}",
                rendered(&primary.join(".ratmac"))
            ))
            && text.contains("LINKED: plan the cycle."),
        "WEBV-019: the linked worktree reports the primary runtime root and its own runbook: {text}"
    );
    let log = fresh.next_log();
    let output = rtm_logged(&linked, &log, &["status", "--run", &linked_run]);
    assert!(
        support::text(&output).contains("LINKED: plan the cycle."),
        "WEBV-019: the logged status keeps the linked runbook: {}",
        support::text(&output)
    );
    assert_resolutions(
        "WEBV-019: `rtm status` in the linked worktree",
        &log,
        &[canonical_line(&linked)],
    );

    // A spawned child with a workspace: the workspace is never resolved.
    let output = rtm(&primary, &["step", "--run", &parent]);
    assert!(
        output.status.success(),
        "WEBV-019: the parent must reach its spawning State: {}",
        support::text(&output)
    );
    let workspace = primary.join("area");
    let log = fresh.next_log();
    let output = support::command(ratmac_qa::engine_bin!(), &primary)
        .args([
            "spawn",
            "rev",
            "--run",
            &parent,
            "--bind",
            "ticket=WEBV-019",
            "--workspace",
        ])
        .arg(&workspace)
        .env(LOG_VAR, &log)
        .output()
        .expect("invoke rtm spawn with a workspace");
    let text = support::text(&output);
    assert!(
        output.status.success(),
        "WEBV-019: `rtm spawn --workspace` inside the repository must succeed: {text}"
    );
    let child = minted(&text, "rtm: spawned run ");
    assert_resolutions(
        "WEBV-019: `rtm spawn --workspace`",
        &log,
        &[canonical_line(&primary)],
    );
    let logged = fs::read_to_string(&log).unwrap_or_default();
    assert!(
        !logged.contains(&rendered(&workspace)),
        "WEBV-019: the spawn workspace is never a resolved project: {logged}"
    );

    // Later commands addressing the child still resolve only the invocation.
    let log = fresh.next_log();
    let output = rtm_logged(&primary, &log, &["status", "--run", &child]);
    assert!(
        support::text(&output).contains("Child works the bound ticket."),
        "WEBV-019: the child's status reports its workspace State: {}",
        support::text(&output)
    );
    assert_resolutions(
        "WEBV-019: `rtm status` of the workspace child",
        &log,
        &[canonical_line(&primary)],
    );
    let log = fresh.next_log();
    let output = rtm_logged(&primary, &log, &["step", "--run", &child]);
    assert!(
        support::text(&output).contains("Child finished."),
        "WEBV-019: the workspace child steps: {}",
        support::text(&output)
    );
    assert_resolutions(
        "WEBV-019: `rtm step` of the workspace child",
        &log,
        &[canonical_line(&primary)],
    );

    // A workspace in another repository refuses and writes nothing.
    let outside = cli_repo("outside");
    let before = project_tree(&primary);
    let spawn_outside = |log: Option<&Path>| {
        let mut command = support::command(ratmac_qa::engine_bin!(), &primary);
        command.args([
            "spawn",
            "rev",
            "--run",
            &parent,
            "--bind",
            "ticket=WEBV-019",
            "--workspace",
        ]);
        command.arg(outside.root());
        if let Some(log) = log {
            command.env(LOG_VAR, log);
        }
        command
            .output()
            .expect("invoke rtm spawn outside the repository")
    };
    let output = spawn_outside(None);
    let text = support::text(&output);
    let refused = format!(
        "workspace {} is outside this repository",
        fs::canonicalize(outside.root())
            .map(|canonical| rendered(&canonical))
            .unwrap_or_else(|_| rendered(outside.root()))
    );
    assert!(
        !output.status.success() && text.contains(&refused),
        "WEBV-019: a workspace in another repository refuses by its known first line: {text}"
    );
    assert_eq!(
        project_tree(&primary),
        before,
        "WEBV-019: the refused spawn writes nothing"
    );
    let log = fresh.next_log();
    let output = spawn_outside(Some(&log));
    assert!(
        !output.status.success() && support::text(&output).contains(&refused),
        "WEBV-019: the logged refusal twin keeps its first line: {}",
        support::text(&output)
    );
    assert_resolutions(
        "WEBV-019: a refusing `rtm spawn --workspace`",
        &log,
        &[canonical_line(&primary)],
    );

    // A workspace inside Git storage belongs to no work tree: refused the
    // same way, with nothing written. The refusal must leave the complete
    // fixture byte-identical, Git storage included, so this case compares
    // the whole tree; the hook's log files live in their own tree.
    let before = complete_tree(&primary);
    let git_storage = primary.join(".git");
    let refused_git = format!(
        "workspace {} is outside this repository",
        fs::canonicalize(&git_storage)
            .map(|canonical| rendered(&canonical))
            .unwrap_or_else(|_| rendered(&git_storage))
    );
    let spawn_in_git_storage = |log: Option<&Path>| {
        let mut command = support::command(ratmac_qa::engine_bin!(), &primary);
        command.args([
            "spawn",
            "rev",
            "--run",
            &parent,
            "--bind",
            "ticket=WEBV-019",
            "--workspace",
        ]);
        command.arg(&git_storage);
        if let Some(log) = log {
            command.env(LOG_VAR, log);
        }
        command
            .output()
            .expect("invoke rtm spawn inside Git storage")
    };
    let output = spawn_in_git_storage(None);
    let text = support::text(&output);
    assert!(
        !output.status.success() && text.contains(&refused_git),
        "WEBV-019: a workspace inside Git storage refuses as outside this repository: {text}"
    );
    assert_eq!(
        complete_tree(&primary),
        before,
        "WEBV-019: the Git-storage refusal leaves the complete fixture byte-identical"
    );
    let log = fresh.next_log();
    let output = spawn_in_git_storage(Some(&log));
    assert!(
        !output.status.success() && support::text(&output).contains(&refused_git),
        "WEBV-019: the logged Git-storage refusal keeps its first line: {}",
        support::text(&output)
    );
    assert_eq!(
        complete_tree(&primary),
        before,
        "WEBV-019: the logged Git-storage refusal also leaves the complete fixture byte-identical"
    );
    assert_resolutions(
        "WEBV-019: a refusing `rtm spawn --workspace` inside Git storage",
        &log,
        &[canonical_line(&primary)],
    );

    // A linked worktree passes: it is judged inside the already-resolved
    // repository and is never resolved itself.
    let log = fresh.next_log();
    let output = support::command(ratmac_qa::engine_bin!(), &primary)
        .args([
            "spawn",
            "rev",
            "--run",
            &parent,
            "--bind",
            "ticket=WEBV-019",
            "--workspace",
        ])
        .arg(&linked)
        .env(LOG_VAR, &log)
        .output()
        .expect("invoke rtm spawn with the linked worktree as workspace");
    let text = support::text(&output);
    assert!(
        output.status.success(),
        "WEBV-019: a linked worktree is a workspace inside this repository: {text}"
    );
    assert_resolutions(
        "WEBV-019: `rtm spawn --workspace` at a linked worktree",
        &log,
        &[canonical_line(&primary)],
    );
    let logged = fs::read_to_string(&log).unwrap_or_default();
    assert!(
        !logged.contains(&canonical_line(&linked)),
        "WEBV-019: the linked workspace is never a resolved project: {logged}"
    );

    // The no-Git fallback: the invoking checkout is the whole repository.
    let nogit = tree.join("nogit");
    for directory in [nogit.join(".ratmac"), nogit.join("work")] {
        fs::create_dir_all(&directory)
            .unwrap_or_else(|error| panic!("create {}: {error}", directory.display()));
    }
    fs::write(nogit.join(".ratmac/ratmac.toml"), cli_runbook())
        .expect("write the fallback runbook");
    fs::write(
        nogit.join("work/blocker.txt"),
        "Opaque blocker; its content is not a gate.\n",
    )
    .expect("write the fallback blocker leaf");
    fs::write(nogit.join("work/item.md"), "Contributor-owned bytes.\n")
        .expect("write the fallback work leaf");
    assert!(
        !nogit
            .ancestors()
            .any(|ancestor| ancestor.join(".git").exists()),
        "WEBV-019: the no-Git fixture must not sit in a Git checkout"
    );
    let log = fresh.next_log();
    let output = rtm_logged(&nogit, &log, &["start"]);
    let text = support::text(&output);
    assert!(
        output.status.success(),
        "WEBV-019: `rtm start` without Git must succeed: {text}"
    );
    let fallback = minted(&text, "rtm: started run ");
    assert!(
        nogit
            .join(".ratmac/runs")
            .join(&fallback)
            .join("run.toml")
            .is_file(),
        "WEBV-019: without Git the invoking checkout is the repository"
    );
    let output = rtm(&nogit, &["status", "--run", &fallback]);
    assert!(
        support::text(&output).contains(&format!(
            "Engine root: {}",
            rendered(&nogit.join(".ratmac"))
        )),
        "WEBV-019: without Git the Engine root stays local: {}",
        support::text(&output)
    );
    assert_resolutions(
        "WEBV-019: `rtm start` without Git",
        &log,
        &[canonical_line(&nogit)],
    );

    // An addressed runbook: the invoking checkout and its project resolve,
    // each once; a `.ratmac/` path hoists to that project.
    let log = fresh.next_log();
    let addressed = primary.join(".ratmac/ratmac.toml");
    let output = support::command(ratmac_qa::engine_bin!(), tree.path())
        .arg("doctor")
        .arg(&addressed)
        .env(LOG_VAR, &log)
        .output()
        .expect("invoke rtm doctor with a runbook path");
    let text = support::text(&output);
    assert!(
        text.contains(&format!(
            "Engine root: {}",
            rendered(&primary.join(".ratmac"))
        )),
        "WEBV-019: `rtm doctor <runbook>` reports the runbook's project: {text}"
    );
    let mut expected = vec![canonical_line(tree.path()), canonical_line(&primary)];
    let mut lines = log_lines(&log);
    expected.sort();
    lines.sort();
    assert_eq!(
        lines, expected,
        "WEBV-019: `rtm doctor <primary runbook>` resolves the invoking checkout and the runbook's project, each once"
    );

    // A runbook in a different project: the invoking checkout and that
    // project resolve, each once.
    let other_project = tree.join("other-project");
    fs::create_dir_all(&other_project)
        .unwrap_or_else(|error| panic!("create {}: {error}", other_project.display()));
    fs::write(other_project.join("runbook.toml"), cli_runbook())
        .expect("write the other project's runbook");
    let log = fresh.next_log();
    let output = support::command(ratmac_qa::engine_bin!(), tree.path())
        .arg("doctor")
        .arg(other_project.join("runbook.toml"))
        .env(LOG_VAR, &log)
        .output()
        .expect("invoke rtm doctor with the other project's runbook");
    let text = support::text(&output);
    assert!(
        text.contains(&format!(
            "Engine root: {}",
            rendered(&other_project.join(".ratmac"))
        )),
        "WEBV-019: `rtm doctor <runbook>` reports the addressed project: {text}"
    );
    let mut expected = vec![canonical_line(tree.path()), canonical_line(&other_project)];
    let mut lines = log_lines(&log);
    expected.sort();
    lines.sort();
    assert_eq!(
        lines, expected,
        "WEBV-019: `rtm doctor <another project's runbook>` resolves the invoking checkout and that project, each once"
    );

    // Scaffold in the same directory: one resolution.
    let log = fresh.next_log();
    let output = rtm_logged(&primary, &log, &["scaffold", "probe.toml"]);
    let text = support::text(&output);
    assert!(
        output.status.success() && text.contains("Wrote probe.toml."),
        "WEBV-019: `rtm scaffold` in the invoking directory must succeed: {text}"
    );
    assert_resolutions(
        "WEBV-019: `rtm scaffold` in the same directory",
        &log,
        &[canonical_line(&primary)],
    );

    // Scaffold into another directory: the invoking checkout and the path's
    // project, each once.
    let log = fresh.next_log();
    let output = support::command(ratmac_qa::engine_bin!(), &primary)
        .args(["scaffold"])
        .arg(other_project.join("scaffolded.toml"))
        .env(LOG_VAR, &log)
        .output()
        .expect("invoke rtm scaffold in another directory");
    let text = support::text(&output);
    assert!(
        output.status.success() && text.contains("Wrote "),
        "WEBV-019: `rtm scaffold` into another directory must succeed: {text}"
    );
    let mut expected = vec![canonical_line(&primary), canonical_line(&other_project)];
    let mut lines = log_lines(&log);
    expected.sort();
    lines.sort();
    assert_eq!(
        lines, expected,
        "WEBV-019: `rtm scaffold <path>` resolves the invoking checkout and the path's project, each once"
    );
}

/// WEBV-020: the discovery constructor is private to `src/root.rs`. A
/// context-bound handler in `src/cli.rs` cannot call it (E0624), the same
/// call inside `src/root.rs` compiles, and no other file in `src/` names
/// `Roots::resolve` or `root::resolve(`.
#[test]
fn webv_020() {
    let _lane = root_lane();
    let _hooks = hide_root_hooks();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root resolves");

    // Positive control: inside src/root.rs the constructor stays callable.
    let inside = RepositoryCopy::new(&repository, "inside");
    append(
        &inside.path.join("src/root.rs"),
        "\n/// WEBV-020: the constructor stays callable inside its own module.\n\
         #[allow(dead_code)]\n\
         fn t119_visibility_probe() {\n    \
         let _ = crate::root::Roots::resolve(\".\");\n\
         }\n",
    );
    let compiled = cargo_check(&repository, &inside.path);
    assert!(
        compiled.status.success(),
        "WEBV-020: the same call inside src/root.rs must compile: {}",
        support::text(&compiled)
    );

    // New behavior: a context-bound handler in src/cli.rs cannot reach it.
    let outside = RepositoryCopy::new(&repository, "outside");
    append(
        &outside.path.join("src/cli.rs"),
        "\n/// WEBV-020: a context-bound handler must take the context, not\n\
         /// resolve a root of its own.\n\
         #[allow(dead_code)]\n\
         fn t119_context_bound_handler() {\n    \
         let _ = crate::root::Roots::resolve(\".\");\n\
         }\n",
    );
    let refused = cargo_check(&repository, &outside.path);
    let text = support::text(&refused);
    assert!(
        !refused.status.success() && text.contains("E0624"),
        "WEBV-020: inserting crate::root::Roots::resolve into a handler in \
         src/cli.rs must fail to compile with E0624 (private associated \
         function); cargo check said: {text}"
    );

    // Supplementary scan: only src/root.rs names the constructor, as a whole
    // path segment - a match preceded by a letter, digit, or `_` (such as
    // `ValidatedWorkflowRoots::resolve`) is a different name, not this one.
    let banned = [b"Roots::resolve".as_slice(), b"root::resolve(".as_slice()];
    for file in source_files(&repository.join("src")) {
        if file == repository.join("src/root.rs") {
            continue;
        }
        let bytes =
            fs::read(&file).unwrap_or_else(|error| panic!("read {}: {error}", file.display()));
        for pattern in banned {
            let named = bytes
                .windows(pattern.len())
                .enumerate()
                .any(|(at, window)| {
                    window == pattern
                        && !bytes[..at]
                            .last()
                            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                });
            assert!(
                !named,
                "WEBV-020: {} names the discovery constructor; only src/root.rs may",
                rendered(&file)
            );
        }
    }
}

/// One throwaway copy of the repository's manifest, lock, and source, with
/// an empty workspace so no test crate is needed to check the library.
struct RepositoryCopy {
    path: PathBuf,
    _tree: TempTree,
}

impl RepositoryCopy {
    fn new(repository: &Path, label: &str) -> Self {
        let tree =
            TempTree::new(&format!("t119-visibility-{label}")).expect("own a visibility tree");
        let copy = tree.join("copy");
        fs::create_dir_all(&copy)
            .unwrap_or_else(|error| panic!("create {}: {error}", copy.display()));
        let manifest =
            fs::read_to_string(repository.join("Cargo.toml")).expect("the manifest reads");
        let mut rewritten = String::new();
        let mut in_workspace = false;
        let mut members_done = false;
        for line in manifest.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_workspace = trimmed == "[workspace]";
            }
            if in_workspace && !members_done && trimmed.starts_with("members") {
                rewritten.push_str("members = []\n");
                members_done = true;
                continue;
            }
            rewritten.push_str(line);
            rewritten.push('\n');
        }
        assert!(
            members_done,
            "WEBV-020: the repository manifest declares its workspace members"
        );
        fs::write(copy.join("Cargo.toml"), rewritten).expect("write the copied manifest");
        fs::copy(repository.join("Cargo.lock"), copy.join("Cargo.lock"))
            .expect("copy the dependency lock");
        copy_tree(&repository.join("src"), &copy.join("src"));
        Self {
            path: copy,
            _tree: tree,
        }
    }
}

/// Copy one directory tree; the repository source carries no links.
fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination)
        .unwrap_or_else(|error| panic!("create {}: {error}", destination.display()));
    for entry in
        fs::read_dir(source).unwrap_or_else(|error| panic!("read {}: {error}", source.display()))
    {
        let entry = entry.unwrap_or_else(|error| panic!("read {}: {error}", source.display()));
        let path = entry.path();
        let target = destination.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            fs::copy(&path, &target)
                .unwrap_or_else(|error| panic!("copy {}: {error}", path.display()));
        }
    }
}

/// Append `text` to `path`.
fn append(path: &Path, text: &str) {
    let mut current =
        fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    if !current.ends_with('\n') {
        current.push('\n');
    }
    current.push_str(text);
    fs::write(path, current).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
}

/// `cargo check --offline --lib` in `directory`, with the repository's stable
/// visibility target dir caching dependencies.
fn cargo_check(repository: &Path, directory: &Path) -> Output {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    support::command(&cargo, directory)
        .args(["check", "--offline", "--lib", "--target-dir"])
        .arg(repository.join("target/t119-visibility"))
        .output()
        .unwrap_or_else(|error| panic!("run cargo check in {}: {error}", directory.display()))
}

/// Every `.rs` file under `root`, in walk order.
fn source_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()))
        {
            let entry =
                entry.unwrap_or_else(|error| panic!("read {}: {error}", directory.display()));
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }
    found
}
