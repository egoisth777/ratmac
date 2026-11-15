//! t-136 / ENS-005 + WEB-002: lock waits survive a peer's release and a
//! busy policy step.
//!
//! ENSV-006: a waiting root- or Run-lock acquirer treats a peer's Windows
//! delete-pending release window as contention, retries inside its one
//! deadline, and finishes with today's outcome; a hold that outlasts the
//! deadline refuses as today's open refusal naming the last open error.
//! WEBV-008: the ignore-policy claim covers only the policy bytes, so a
//! second `rtm init` proceeds beside a peer paused at the pre-claim
//! inspection or at a non-writing verification, blocks only behind a
//! writing peer, and sixteen concurrent inits all succeed.
//!
//! WTCV-010..012 (t-149 / WTC-004): `rtm abandon` reads whether the root
//! lock and the addressed Run's lock exist while it holds no lock. A read
//! that meets a peer's Windows release window - `Access is denied. (os
//! error 5)` - is read again inside the lock module's five-second bound,
//! measured from the first denial, and the settled answer, present or
//! absent, decides exactly as an unfaulted read; a denial that outlasts the
//! bound, or any other error, refuses with today's `abandon cannot inspect
//! lock <path>: <error>`. The denials come from the test-only presence
//! fault (`RATMAC_TEST_LOCK_PRESENCE_FAULT=<domains>:<reads>:<code>`), set
//! only on its own case's `rtm` child, which records each injected failure
//! as one whole line in `RATMAC_TEST_LOCK_PRESENCE_RECORD`, a file outside
//! the project read only after that child has exited.
//!
//! Every pause is forced through the test seams (`RATMAC_TEST_LOCK_HOLD=`
//! `unlink`, `RATMAC_TEST_POLICY_PAUSE`); no window case relies on a race.
//! Every wait is bounded, every child is released or killed before an
//! assertion, and a barrier guard writes the release file on unwinding so
//! no paused child is ever stranded.

use ratmac_qa::support::{
    self, await_marker_line, CaptureOptions, MarkerLine, MarkerWait, TempTree,
};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

// --- ceilings -------------------------------------------------------------------

/// How long a seam marker may take to appear. Arrival well inside this
/// bound is the seam pausing; expiry is the seam being missing.
const MARKER_WAIT: Duration = Duration::from_secs(15);
/// Every child is reaped inside this ceiling and killed on expiry, so no
/// child outlives the test whatever it does.
const CHILD_CEILING: Duration = Duration::from_secs(30);
/// How long a concurrent caller is watched for early exit while its peer
/// holds the window or the claim. Comfortably above one step's Git work,
/// comfortably below the one five-second wait deadline.
const WINDOW_PROBE: Duration = Duration::from_millis(2500);
/// The lock hold outlasts every waiter deadline in these checks and is
/// always released explicitly well before it expires.
const LOCK_HOLD_MILLIS: &str = "30000";

// --- the contract's exact bytes (t-121's pinned texts) ---------------------------

/// Success stdout line 2 inside a verified repository.
const VERIFIED_LINE: &str = "rtm: verified: runs/, mint.toml, locks/, and log.md ignored; \
                            .gitignore, ratmac.toml, and evidence/ trackable";

/// Success stdout line 3 with no runbook at the invoking checkout.
const NEXT_SCAFFOLD_LINE: &str = "next: rtm scaffold .ratmac/ratmac.toml";

/// The merged policy bytes for the operator fixture: the operator's lines,
/// a supplied final newline, then the four runtime lines in contract order.
const MERGED_POLICY: &[u8] = b"operator-a/\noperator-b\n/runs/\n/mint.toml\n/locks/\n/log.md\n";

/// The runbook fixture: a work root plus a three-state machine with one
/// outgoing edge per state, so every `rtm step` is deterministic.
const RUNBOOK: &str = r#"
[roots]
work = "work"

[states.plan]
prompt = "Plan the cycle."

[states.review]
prompt = "Review the cycle."

[states.done]
prompt = "Done."

[[transitions]]
from = "plan"
to = "review"

[[transitions]]
from = "review"
to = "done"
"#;

// --- isolated Git configuration --------------------------------------------------

/// One owned empty file every Git child (direct or beneath `rtm`) sees as
/// `GIT_CONFIG_GLOBAL`, beside `GIT_CONFIG_NOSYSTEM=1`, so the operator's
/// own configuration can never leak into a fixture or be written by one.
struct GitIsolate {
    tree: TempTree,
}

impl GitIsolate {
    fn new(label: &str) -> Self {
        let tree = TempTree::new(label).expect("own the Git isolation tree");
        tree.write("global-git-config", b"");
        Self { tree }
    }

    fn apply(&self, command: &mut Command) {
        command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.tree.join("global-git-config"));
    }
}

// --- small helpers ---------------------------------------------------------------

/// This path in the one spelling the Engine shows: forward slashes.
fn rendered(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn combined_text(output: &Output) -> String {
    format!("{}{}", stdout_text(output), stderr_text(output))
}

fn exit_code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

/// The Run id a successful `rtm start` reports.
fn started_run_id(label: &str, output: &Output) -> String {
    let text = combined_text(output);
    text.split("rtm: started run ")
        .nth(1)
        .and_then(|suffix| suffix.split_whitespace().next())
        .unwrap_or_else(|| panic!("{label}: start output lacks a Run id: {text}"))
        .to_owned()
}

/// `text` with every `from` Run id replaced by `to`, so a control's stdout
/// on one Run is the expected stdout of the same transition on another.
fn retarget(text: &str, from: &str, to: &str) -> String {
    text.replace(from, to)
}

/// Fixture Git in `directory` under the isolated configuration.
fn git(isolate: &GitIsolate, directory: &Path, args: &[&str]) -> Output {
    let mut command = support::git(directory);
    command.args(args);
    isolate.apply(&mut command);
    command
        .output()
        .unwrap_or_else(|error| panic!("git {args:?} in {}: {error}", rendered(directory)))
}

/// Fixture Git that must succeed.
fn git_ok(isolate: &GitIsolate, label: &str, directory: &Path, args: &[&str]) {
    let output = git(isolate, directory, args);
    assert!(
        output.status.success(),
        "{label}: git {args:?}: {}",
        support::text(&output)
    );
}

/// A fresh isolated Git repository: local identity only, byte-exact line
/// endings, no global or system configuration reachable.
fn git_repo(isolate: &GitIsolate, label: &str) -> TempTree {
    let tree = TempTree::new(&format!("t136-{label}")).expect("own a fixture tree");
    for args in [
        &["init", "-q"][..],
        &["symbolic-ref", "HEAD", "refs/heads/main"][..],
        &["config", "user.email", "qa@ratmac.test"][..],
        &["config", "user.name", "ratmac qa"][..],
        &["config", "core.autocrlf", "false"][..],
        &["config", "commit.gpgsign", "false"][..],
    ] {
        git_ok(isolate, label, tree.path(), args);
    }
    tree
}

/// A repository whose runbook and work leaf are committed, the shape the
/// existing start suites require before a Run can be admitted.
fn committed_runbook_repo(isolate: &GitIsolate, label: &str) -> TempTree {
    let tree = git_repo(isolate, label);
    tree.write(".ratmac/ratmac.toml", RUNBOOK);
    tree.write("work/item.md", "Contributor-owned bytes.\n");
    git_ok(isolate, label, tree.path(), &["add", "--all"]);
    git_ok(
        isolate,
        label,
        tree.path(),
        &["commit", "-q", "-m", "t-136 fixture base"],
    );
    tree
}

/// Every node below `root` as comparable tagged bytes, `excluded` left out.
fn tagged_tree_excluding(root: &Path, excluded: &[PathBuf]) -> BTreeMap<String, Vec<u8>> {
    let options = CaptureOptions {
        excluded_paths: excluded,
        ..CaptureOptions::default()
    };
    support::capture(root, options)
        .unwrap_or_else(|error| panic!("capture {}: {error}", rendered(root)))
        .tagged()
}

/// The Engine root's nodes as tagged bytes.
fn ratmac_tagged(checkout: &Path) -> BTreeMap<String, Vec<u8>> {
    tagged_tree_excluding(&checkout.join(".ratmac"), &[])
}

// --- bounded children -------------------------------------------------------------

/// A scrubbed, isolated `rtm` child in `directory`, paused-output pipes
/// attached, carrying exactly the given explicit environment entries.
fn spawn_rtm(
    label: &str,
    isolate: &GitIsolate,
    directory: &Path,
    args: &[&str],
    vars: &[(&str, &OsStr)],
) -> Child {
    let mut command = support::command(ratmac_qa::engine_bin!(), directory);
    command.args(args);
    for (key, value) in vars {
        command.env(key, value);
    }
    isolate.apply(&mut command);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
        .spawn()
        .unwrap_or_else(|error| panic!("{label}: spawn rtm {args:?}: {error}"))
}

/// Collect a finished child's remaining output.
fn collect(label: &str, child: &mut Child) -> Output {
    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    if let Some(mut pipe) = child.stdout.take() {
        pipe.read_to_end(&mut stdout_bytes)
            .unwrap_or_else(|error| panic!("{label}: read rtm stdout: {error}"));
    }
    if let Some(mut pipe) = child.stderr.take() {
        pipe.read_to_end(&mut stderr_bytes)
            .unwrap_or_else(|error| panic!("{label}: read rtm stderr: {error}"));
    }
    let status = child
        .wait()
        .unwrap_or_else(|error| panic!("{label}: wait for rtm: {error}"));
    Output {
        status,
        stdout: stdout_bytes,
        stderr: stderr_bytes,
    }
}

/// Wait for a child inside `ceiling`, killing it on expiry: no child
/// outlives the test even when it hangs.
fn reap(label: &str, child: &mut Child, ceiling: Duration) -> Output {
    let deadline = Instant::now() + ceiling;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(error) => panic!("{label}: inspect the child: {error}"),
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "{label}: the child did not finish within {} s and was killed",
                ceiling.as_secs()
            );
        }
        thread::sleep(Duration::from_millis(25));
    }
    collect(label, child)
}

/// Run one `rtm` invocation to completion inside the child ceiling.
fn run_rtm(
    label: &str,
    isolate: &GitIsolate,
    directory: &Path,
    args: &[&str],
    vars: &[(&str, &OsStr)],
) -> Output {
    let mut child = spawn_rtm(label, isolate, directory, args, vars);
    reap(label, &mut child, CHILD_CEILING)
}

/// Whether a child stayed live for the whole `probe` window (no exit was
/// observed before the window ended).
fn stayed_live(label: &str, child: &mut Child, probe: Duration) -> bool {
    let deadline = Instant::now() + probe;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return false,
            Ok(None) => {}
            Err(error) => panic!("{label}: inspect the child: {error}"),
        }
        if Instant::now() >= deadline {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// Whether a seam's marker wait (the shared whole-line wait, SVE-005) found a
/// whole line at all: a missing, empty, or unfinished marker by the limit is
/// the seam never pausing.
fn published(marker: &Result<String, MarkerWait>) -> bool {
    !matches!(marker, Err(MarkerWait::NotYetPublished(_)))
}

/// The whole line a seam's marker wait found: the line it accepted, or the
/// other text it reported; empty when no whole line was published.
fn whole_line(marker: &Result<String, MarkerWait>) -> String {
    match marker {
        Ok(line) => line.clone(),
        Err(MarkerWait::OtherText(bytes)) => String::from_utf8_lossy(bytes).into_owned(),
        Err(MarkerWait::NotYetPublished(_)) => String::new(),
    }
}

/// Writes the release file on normal cleanup and unwinding alike, so a
/// fixture failure cannot strand a paused child behind its barrier.
struct Barrier {
    release: PathBuf,
    released: bool,
}

impl Barrier {
    fn new(release: &Path) -> Self {
        let _ = fs::remove_file(release);
        Self {
            release: release.to_path_buf(),
            released: false,
        }
    }

    fn release(&mut self) {
        if !self.released {
            let _ = fs::write(&self.release, "release\n");
            self.released = true;
        }
    }
}

impl Drop for Barrier {
    fn drop(&mut self) {
        self.release();
    }
}

// --- ENSV-006: the release window is contention -----------------------------------

/// One window round: a holder paused at the `unlink` hold, a concurrent
/// waiter started once the marker appears, both reaped after release.
struct WindowCase {
    label: &'static str,
    marker: Result<String, MarkerWait>,
    expected_marker: String,
    holder: Output,
    waiter: Option<Output>,
    /// On Windows the waiter is shown still running inside the peer's
    /// delete-pending window before the release file exists.
    #[cfg(windows)]
    waiter_stayed_live: bool,
}

fn observe_window(
    isolate: &GitIsolate,
    label: &'static str,
    root: &Path,
    holder_args: &[&str],
    waiter_args: &[&str],
    lock_path: &Path,
) -> WindowCase {
    let scratch = TempTree::new(&format!("t136-{label}")).expect("own the hold barrier tree");
    let marker_path = scratch.join("marker");
    let release_path = scratch.join("release");
    let mut barrier = Barrier::new(&release_path);

    let mut holder = spawn_rtm(
        label,
        isolate,
        root,
        holder_args,
        &[
            ("RATMAC_TEST_LOCK_HOLD", OsStr::new("unlink")),
            ("RATMAC_TEST_LOCK_MARKER", marker_path.as_os_str()),
            ("RATMAC_TEST_LOCK_RELEASE", release_path.as_os_str()),
            (
                "RATMAC_TEST_LOCK_HOLD_TIMEOUT_MILLIS",
                OsStr::new(LOCK_HOLD_MILLIS),
            ),
        ],
    );
    let marker = await_marker_line(
        &marker_path,
        MarkerLine::Prefix("holding unlink lock "),
        MARKER_WAIT,
    );
    let mut waiter = marker
        .is_ok()
        .then(|| spawn_rtm(label, isolate, root, waiter_args, &[]));
    #[cfg(windows)]
    let waiter_stayed_live = match waiter.as_mut() {
        Some(child) => stayed_live(label, child, WINDOW_PROBE),
        None => false,
    };
    barrier.release();
    let holder_output = reap(label, &mut holder, CHILD_CEILING);
    let waiter_output = waiter
        .as_mut()
        .map(|child| reap(label, child, CHILD_CEILING));
    WindowCase {
        label,
        marker,
        expected_marker: format!("holding unlink lock {}\n", rendered(lock_path)),
        holder: holder_output,
        waiter: waiter_output,
        #[cfg(windows)]
        waiter_stayed_live,
    }
}

/// Assert one window round: the seam paused, the waiter (on Windows) stayed
/// inside the window, and both sides finished with today's outcomes and no
/// `cannot open lock` anywhere.
fn assert_waited_through(
    case: &WindowCase,
    expected_holder_stdout: &str,
    expected_waiter_stdout: &str,
) {
    let label = case.label;
    assert!(
        published(&case.marker),
        "{label}: the holder must pause after requesting its pathname's deletion - the \
         `holding unlink lock` marker never appeared within {} s; RATMAC_TEST_LOCK_HOLD=unlink \
         does not pause today (contract: 'A root or Run lock's release pauses after its pathname \
         removal has been requested and before the kernel unlock and handle close')",
        MARKER_WAIT.as_secs()
    );
    assert_eq!(
        whole_line(&case.marker),
        case.expected_marker,
        "{label}: the unlink hold marker reads `holding unlink lock <path>` plus LF"
    );
    #[cfg(windows)]
    assert!(
        case.waiter_stayed_live,
        "{label}: while the peer held the release window, the waiter was shown still running \
         inside it - a waiting acquirer treats the window as contention and retries inside its \
         one deadline instead of refusing at once"
    );
    assert!(
        case.holder.status.success(),
        "{label}: the released holder finishes with today's outcome: {}",
        combined_text(&case.holder)
    );
    assert_eq!(
        stdout_text(&case.holder),
        expected_holder_stdout,
        "{label}: the holder's stdout is today's outcome for an ordinary completed run"
    );
    let waiter = case
        .waiter
        .as_ref()
        .unwrap_or_else(|| panic!("{label}: the waiter must start once the marker appears"));
    assert!(
        waiter.status.success(),
        "{label}: the waiter that waited out the peer's release window finishes with today's \
         outcome for an ordinary contended wait: {}",
        combined_text(waiter)
    );
    assert_eq!(
        stdout_text(waiter),
        expected_waiter_stdout,
        "{label}: the waiter's stdout is today's outcome for an ordinary contended wait"
    );
    assert!(
        !combined_text(waiter).contains("cannot open lock"),
        "{label}: the waiter never prints `cannot open lock` for a peer's release window: {}",
        combined_text(waiter)
    );
    assert!(
        !combined_text(&case.holder).contains("cannot open lock"),
        "{label}: the holder never prints `cannot open lock`: {}",
        combined_text(&case.holder)
    );
}

/// ENSV-006: waiting root- and Run-lock acquirers wait through a peer's
/// release window; a hold past the deadline refuses as today's open refusal.
#[test]
fn ensv_006_waiters_wait_through_a_peer_release() {
    let isolate = GitIsolate::new("t136-ensv-isolate");

    // Positive controls on one repository: plain starts mint two Runs and
    // two plain steps record today's step stdout for each transition.
    let repo = committed_runbook_repo(&isolate, "ensv-control");
    let root = repo.path();
    let first_start = run_rtm("ENSV-006 control start 1", &isolate, root, &["start"], &[]);
    assert!(
        first_start.status.success(),
        "ENSV-006 control: the first start mints a Run: {}",
        combined_text(&first_start)
    );
    let second_start = run_rtm("ENSV-006 control start 2", &isolate, root, &["start"], &[]);
    assert!(
        second_start.status.success(),
        "ENSV-006 control: the second start mints a Run: {}",
        combined_text(&second_start)
    );
    let holder_run = started_run_id("ENSV-006 control start 1", &first_start);
    let control_run = started_run_id("ENSV-006 control start 2", &second_start);
    let step_one = run_rtm(
        "ENSV-006 control step one",
        &isolate,
        root,
        &["step", "--run", &control_run],
        &[],
    );
    assert!(
        step_one.status.success(),
        "ENSV-006 control: a plain first step succeeds: {}",
        combined_text(&step_one)
    );
    let step_two = run_rtm(
        "ENSV-006 control step two",
        &isolate,
        root,
        &["step", "--run", &control_run],
        &[],
    );
    assert!(
        step_two.status.success(),
        "ENSV-006 control: a plain second step succeeds: {}",
        combined_text(&step_two)
    );
    let expected_step_one_stdout = retarget(&stdout_text(&step_one), &control_run, &holder_run);
    let expected_step_two_stdout = retarget(&stdout_text(&step_two), &control_run, &holder_run);

    // The Run-lock window: a holder paused at `unlink` after its step, a
    // second `rtm step` on the same Run started once the marker appears.
    let run_lock = root
        .join(".ratmac")
        .join("locks")
        .join("runs")
        .join(format!("{holder_run}.lock"));
    let run_case = observe_window(
        &isolate,
        "ensv-run-window",
        root,
        &["step", "--run", &holder_run],
        &["step", "--run", &holder_run],
        &run_lock,
    );
    assert_waited_through(
        &run_case,
        &expected_step_one_stdout,
        &expected_step_two_stdout,
    );

    // The root-lock window: a holder `rtm start` paused at `unlink` after
    // minting, a second `rtm start` minting behind it.
    let root_lock = root.join(".ratmac").join("locks").join("root.lock");
    let root_case = observe_window(
        &isolate,
        "ensv-root-window",
        root,
        &["start"],
        &["start"],
        &root_lock,
    );
    assert!(
        published(&root_case.marker),
        "ensv-root-window: the root holder must pause after requesting its pathname's deletion - \
         the `holding unlink lock` marker never appeared within {} s; \
         RATMAC_TEST_LOCK_HOLD=unlink does not pause today",
        MARKER_WAIT.as_secs()
    );
    assert_eq!(
        whole_line(&root_case.marker),
        root_case.expected_marker,
        "ensv-root-window: the unlink hold marker reads `holding unlink lock <path>` plus LF"
    );
    assert!(
        root_case.holder.status.success(),
        "ensv-root-window: the released root holder finishes with today's outcome: {}",
        combined_text(&root_case.holder)
    );
    let holder_id = started_run_id("ensv-root-window holder", &root_case.holder);
    assert_eq!(
        stdout_text(&root_case.holder),
        format!("rtm: started run {holder_id} at .ratmac/runs/{holder_id}/\n"),
        "ensv-root-window: the holder's stdout is today's start outcome"
    );
    let root_waiter = root_case.waiter.as_ref().unwrap_or_else(|| {
        panic!("ensv-root-window: the waiter must start once the marker appears")
    });
    assert!(
        root_waiter.status.success(),
        "ensv-root-window: the root waiter that waited out the peer's release window finishes \
         with today's outcome for an ordinary contended wait: {}",
        combined_text(root_waiter)
    );
    #[cfg(windows)]
    assert!(
        root_case.waiter_stayed_live,
        "ensv-root-window: while the peer held the release window, the waiter was shown still running \
         inside it - a waiting acquirer treats the window as contention and retries inside its \
         one deadline instead of refusing at once"
    );
    let waiter_id = started_run_id("ensv-root-window waiter", root_waiter);
    assert_eq!(
        stdout_text(root_waiter),
        format!("rtm: started run {waiter_id} at .ratmac/runs/{waiter_id}/\n"),
        "ensv-root-window: the waiter's stdout is today's start outcome"
    );
    assert_ne!(
        holder_id, waiter_id,
        "ensv-root-window: the waiter mints its own Run once the window closes"
    );
    assert!(
        !combined_text(root_waiter).contains("cannot open lock"),
        "ensv-root-window: the waiter never prints `cannot open lock` for a peer's release \
         window: {}",
        combined_text(root_waiter)
    );

    // Windows only: a hold past the waiter's one deadline refuses as today's
    // open refusal, naming the last open error, never `lock wait expired`,
    // and leaves every byte as found.
    #[cfg(windows)]
    {
        let fresh_start = run_rtm(
            "ENSV-006 deadline setup start",
            &isolate,
            root,
            &["start"],
            &[],
        );
        assert!(
            fresh_start.status.success(),
            "ENSV-006 deadline setup: the start mints a Run: {}",
            combined_text(&fresh_start)
        );
        let deadline_run = started_run_id("ENSV-006 deadline setup start", &fresh_start);
        let deadline_lock = root
            .join(".ratmac")
            .join("locks")
            .join("runs")
            .join(format!("{deadline_run}.lock"));
        let excluded = [PathBuf::from(format!(
            ".ratmac/locks/runs/{deadline_run}.lock"
        ))];

        let scratch = TempTree::new("t136-ensv-deadline").expect("own the deadline barrier tree");
        let marker_path = scratch.join("marker");
        let release_path = scratch.join("release");
        let mut barrier = Barrier::new(&release_path);
        let mut holder = spawn_rtm(
            "ensv-deadline",
            &isolate,
            root,
            &["step", "--run", &deadline_run],
            &[
                ("RATMAC_TEST_LOCK_HOLD", OsStr::new("unlink")),
                ("RATMAC_TEST_LOCK_MARKER", marker_path.as_os_str()),
                ("RATMAC_TEST_LOCK_RELEASE", release_path.as_os_str()),
                (
                    "RATMAC_TEST_LOCK_HOLD_TIMEOUT_MILLIS",
                    OsStr::new(LOCK_HOLD_MILLIS),
                ),
            ],
        );
        let marker = await_marker_line(
            &marker_path,
            MarkerLine::Prefix("holding unlink lock "),
            MARKER_WAIT,
        );
        let mut waiter_output = None;
        let mut tree_before = None;
        let mut tree_after = None;
        if marker.is_ok() {
            tree_before = Some(tagged_tree_excluding(root, &excluded));
            let mut waiter = spawn_rtm(
                "ensv-deadline waiter",
                &isolate,
                root,
                &["step", "--run", &deadline_run],
                &[],
            );
            waiter_output = Some(reap("ensv-deadline waiter", &mut waiter, CHILD_CEILING));
            tree_after = Some(tagged_tree_excluding(root, &excluded));
        }
        barrier.release();
        let _ = reap("ensv-deadline holder", &mut holder, CHILD_CEILING);

        assert!(
            published(&marker),
            "ensv-deadline: the Run holder must pause after requesting its pathname's deletion - \
             the `holding unlink lock` marker never appeared within {} s; \
             RATMAC_TEST_LOCK_HOLD=unlink does not pause today",
            MARKER_WAIT.as_secs()
        );
        let waiter = waiter_output
            .as_ref()
            .expect("ensv-deadline: the waiter must start once the marker appears");
        assert_eq!(
            exit_code(waiter),
            1,
            "ensv-deadline: a hold past the deadline refuses with exit 1: {}",
            combined_text(waiter)
        );
        assert!(
            stdout_text(waiter).is_empty(),
            "ensv-deadline: the refusal prints nothing on stdout: {}",
            stdout_text(waiter)
        );
        let expected_refusal = format!(
            "rtm: step: cannot open lock {}: Access is denied. (os error 5)",
            rendered(&deadline_lock)
        );
        assert_eq!(
            stderr_text(waiter).lines().next(),
            Some(expected_refusal.as_str()),
            "ensv-deadline: the refusal names the last open error as `cannot open lock <path>: \
             <os error>`, never `lock wait expired`: {}",
            stderr_text(waiter)
        );
        assert!(
            !stderr_text(waiter).contains("lock wait expired"),
            "ensv-deadline: the refusal is never `lock wait expired`: {}",
            stderr_text(waiter)
        );
        assert_eq!(
            tree_before, tree_after,
            "ensv-deadline: the refused waiter leaves every byte as found"
        );
    }
}

// --- WEBV-008: the policy claim covers only the policy bytes ----------------------

/// Success stdout for a run that wrote the policy.
fn installed_stdout(checkout: &Path) -> String {
    format!(
        "rtm: installed the Engine-root tracking policy at {}\n{}\n{}\n",
        rendered(&checkout.join(".ratmac").join(".gitignore")),
        VERIFIED_LINE,
        NEXT_SCAFFOLD_LINE
    )
}

/// Success stdout for a run that found every line already present.
fn already_present_stdout(checkout: &Path) -> String {
    format!(
        "rtm: Engine-root tracking policy already present at {}\n{}\n{}\n",
        rendered(&checkout.join(".ratmac").join(".gitignore")),
        VERIFIED_LINE,
        NEXT_SCAFFOLD_LINE
    )
}

/// One paused-peer round whose second `rtm init` is expected to complete
/// while the peer is still paused.
struct ParallelCase {
    label: &'static str,
    boundary: &'static str,
    marker: Result<String, MarkerWait>,
    second: Option<Output>,
    peer: Output,
    peer_live_when_second_finished: Option<bool>,
}

fn observe_second_completes_while_paused(
    isolate: &GitIsolate,
    label: &'static str,
    root: &Path,
    boundary: &'static str,
) -> ParallelCase {
    let scratch = TempTree::new(&format!("t136-{label}")).expect("own the pause barrier tree");
    let marker_path = scratch.join("marker");
    let release_path = scratch.join("release");
    let mut barrier = Barrier::new(&release_path);

    let mut peer = spawn_rtm(
        label,
        isolate,
        root,
        &["init"],
        &[
            ("RATMAC_TEST_POLICY_PAUSE", OsStr::new(boundary)),
            ("RATMAC_TEST_POLICY_PAUSE_MARKER", marker_path.as_os_str()),
            ("RATMAC_TEST_POLICY_PAUSE_RELEASE", release_path.as_os_str()),
        ],
    );
    let boundary_line = format!("{boundary}\n");
    let marker = await_marker_line(&marker_path, MarkerLine::Exact(&boundary_line), MARKER_WAIT);
    let mut second = None;
    let mut peer_live = None;
    if marker.is_ok() {
        // The release file does not exist yet, so a completion collected
        // here happened while the peer was still paused.
        second = Some(run_rtm(label, isolate, root, &["init"], &[]));
        peer_live = Some(matches!(peer.try_wait(), Ok(None)));
    }
    barrier.release();
    let peer_output = reap(label, &mut peer, CHILD_CEILING);
    ParallelCase {
        label,
        boundary,
        marker,
        second,
        peer: peer_output,
        peer_live_when_second_finished: peer_live,
    }
}

/// Assert the seam paused with the boundary's marker and stayed paused.
fn assert_peer_paused(case: &ParallelCase) {
    assert!(
        published(&case.marker),
        "{}: the peer must pause at the `{}` boundary - its marker never appeared within {} s; \
         RATMAC_TEST_POLICY_PAUSE={} does not pause today (contract: 'Boundary `inspect` pauses \
         inside the pre-claim Git inspection' / 'Boundary `verify` pauses at the start of \
         effective-rule verification')",
        case.label,
        case.boundary,
        MARKER_WAIT.as_secs(),
        case.boundary
    );
    assert_eq!(
        whole_line(&case.marker),
        format!("{}\n", case.boundary),
        "{}: the pause marker content is the boundary name plus LF",
        case.label
    );
    assert_eq!(
        case.peer_live_when_second_finished,
        Some(true),
        "{}: the peer is still paused when the second init has finished",
        case.label
    );
}

/// One paused-writer round: the second caller is watched for early exit
/// while the peer holds the claim, then both complete after release.
struct BlockedCase {
    marker: Result<String, MarkerWait>,
    second: Option<Output>,
    peer: Output,
    second_stayed_live: Option<bool>,
}

fn observe_blocked_second(isolate: &GitIsolate, root: &Path) -> BlockedCase {
    let scratch = TempTree::new("t136-webv-verify-writing").expect("own the writing barrier tree");
    let marker_path = scratch.join("marker");
    let release_path = scratch.join("release");
    let mut barrier = Barrier::new(&release_path);

    let mut peer = spawn_rtm(
        "webv-verify-writing",
        isolate,
        root,
        &["init"],
        &[
            ("RATMAC_TEST_POLICY_PAUSE", OsStr::new("verify")),
            ("RATMAC_TEST_POLICY_PAUSE_MARKER", marker_path.as_os_str()),
            ("RATMAC_TEST_POLICY_PAUSE_RELEASE", release_path.as_os_str()),
        ],
    );
    let marker = await_marker_line(&marker_path, MarkerLine::Exact("verify\n"), MARKER_WAIT);
    let mut second = None;
    let mut second_stayed_live = None;
    if marker.is_ok() {
        let mut child = spawn_rtm("webv-verify-writing second", isolate, root, &["init"], &[]);
        second_stayed_live = Some(stayed_live(
            "webv-verify-writing second",
            &mut child,
            WINDOW_PROBE,
        ));
        second = Some(child);
    }
    barrier.release();
    let peer_output = reap("webv-verify-writing", &mut peer, CHILD_CEILING);
    let second_output = second
        .as_mut()
        .map(|child| reap("webv-verify-writing second", child, CHILD_CEILING));
    BlockedCase {
        marker,
        second: second_output,
        peer: peer_output,
        second_stayed_live,
    }
}

/// WEBV-008: the ignore-policy claim covers only the policy bytes, so
/// concurrent inits never queue behind Git inspection or a non-writing
/// verification, and sixteen of them all succeed.
#[test]
fn webv_008_policy_claim_covers_only_the_policy_bytes() {
    let isolate = GitIsolate::new("t136-webv-isolate");

    // Positive control: an ordinary init installs, and its byte-identical
    // repeat reports already present.
    let control = git_repo(&isolate, "webv-control");
    let control_first = run_rtm(
        "WEBV-008 control init",
        &isolate,
        control.path(),
        &["init"],
        &[],
    );
    assert!(
        control_first.status.success(),
        "WEBV-008 control: the first init exits 0: {}",
        combined_text(&control_first)
    );
    assert_eq!(
        stdout_text(&control_first),
        installed_stdout(control.path()),
        "WEBV-008 control: the first init prints today's installed stdout"
    );
    let control_repeat = run_rtm(
        "WEBV-008 control repeat",
        &isolate,
        control.path(),
        &["init"],
        &[],
    );
    assert!(
        control_repeat.status.success(),
        "WEBV-008 control: the repeat init exits 0: {}",
        combined_text(&control_repeat)
    );
    assert_eq!(
        stdout_text(&control_repeat),
        already_present_stdout(control.path()),
        "WEBV-008 control: the repeat init prints today's already-present stdout"
    );

    // A peer paused inside the pre-claim Git inspection does not stop a
    // second init: the claim covers only the policy bytes.
    let fresh = git_repo(&isolate, "webv-inspect");
    let inspect_case =
        observe_second_completes_while_paused(&isolate, "webv-inspect", fresh.path(), "inspect");
    assert_peer_paused(&inspect_case);
    let inspect_second = inspect_case
        .second
        .as_ref()
        .expect("webv-inspect: the second init must run once the marker appears");
    assert!(
        inspect_second.status.success(),
        "webv-inspect: a second rtm init exits 0 while the peer is still paused at `inspect`: {}",
        combined_text(inspect_second)
    );
    assert_eq!(
        stdout_text(inspect_second),
        installed_stdout(fresh.path()),
        "webv-inspect: the second init prints today's stdout for what it did"
    );
    assert!(
        inspect_case.peer.status.success(),
        "webv-inspect: the released peer finishes with today's outcome: {}",
        combined_text(&inspect_case.peer)
    );
    assert_eq!(
        stdout_text(&inspect_case.peer),
        already_present_stdout(fresh.path()),
        "webv-inspect: the peer, released after the second init wrote the policy, writes nothing \
         and reports already present"
    );

    // A non-writing peer paused at `verify` holds no claim: its verification
    // runs unserialized and a second init exits 0 meanwhile.
    let initialized = git_repo(&isolate, "webv-verify-nonwriting");
    let setup = run_rtm(
        "WEBV-008 verify setup init",
        &isolate,
        initialized.path(),
        &["init"],
        &[],
    );
    assert!(
        setup.status.success(),
        "WEBV-008 verify setup: the pre-initializing init exits 0: {}",
        combined_text(&setup)
    );
    let verify_case = observe_second_completes_while_paused(
        &isolate,
        "webv-verify-nonwriting",
        initialized.path(),
        "verify",
    );
    assert_peer_paused(&verify_case);
    let verify_second = verify_case
        .second
        .as_ref()
        .expect("webv-verify-nonwriting: the second init must run once the marker appears");
    assert!(
        verify_second.status.success(),
        "webv-verify-nonwriting: a second rtm init exits 0 while a non-writing peer is still \
         paused at `verify`: {}",
        combined_text(verify_second)
    );
    assert_eq!(
        stdout_text(verify_second),
        already_present_stdout(initialized.path()),
        "webv-verify-nonwriting: the second init finds all four lines present, writes nothing, \
         and prints today's stdout"
    );
    assert!(
        verify_case.peer.status.success(),
        "webv-verify-nonwriting: the released peer finishes with today's outcome: {}",
        combined_text(&verify_case.peer)
    );
    assert_eq!(
        stdout_text(&verify_case.peer),
        already_present_stdout(initialized.path()),
        "webv-verify-nonwriting: the non-writing peer also reports already present"
    );

    // A writing peer paused at `verify` keeps the claim through its own
    // verification: a second caller completes only after release.
    let writing = git_repo(&isolate, "webv-verify-writing");
    writing.write(".ratmac/.gitignore", "operator-a/\noperator-b");
    let blocked = observe_blocked_second(&isolate, writing.path());
    assert!(
        published(&blocked.marker),
        "webv-verify-writing: the peer must pause at the `verify` boundary - its marker never \
         appeared within {} s; RATMAC_TEST_POLICY_PAUSE=verify does not pause today",
        MARKER_WAIT.as_secs()
    );
    assert_eq!(
        whole_line(&blocked.marker),
        "verify\n",
        "webv-verify-writing: the pause marker content is the boundary name plus LF"
    );
    assert_eq!(
        blocked.second_stayed_live,
        Some(true),
        "webv-verify-writing: with a writing peer paused at `verify`, a second caller stays \
         running for the whole probe window - it completes only after release, never beside the \
         writer's claim"
    );
    assert!(
        blocked.peer.status.success(),
        "webv-verify-writing: the released writing peer finishes with today's outcome: {}",
        combined_text(&blocked.peer)
    );
    assert_eq!(
        stdout_text(&blocked.peer),
        installed_stdout(writing.path()),
        "webv-verify-writing: the writing peer prints today's stdout for what it did"
    );
    let blocked_second = blocked
        .second
        .as_ref()
        .expect("webv-verify-writing: the second init must run once the marker appears");
    assert!(
        blocked_second.status.success(),
        "webv-verify-writing: the second caller exits 0 once the writer released the claim: {}",
        combined_text(blocked_second)
    );
    assert_eq!(
        stdout_text(blocked_second),
        already_present_stdout(writing.path()),
        "webv-verify-writing: the second caller reports already present"
    );
    assert_eq!(
        fs::read(writing.path().join(".ratmac").join(".gitignore"))
            .expect("read the writing fixture's policy"),
        MERGED_POLICY,
        "webv-verify-writing: the policy file holds the operator's bytes followed by each of the \
         four lines exactly once, in contract order"
    );

    // Sixteen concurrent inits on one fresh repository: a waiter waits for
    // at most one writer's step, never for the number of concurrent callers.
    let concurrent = git_repo(&isolate, "webv-sixteen");
    concurrent.write(".ratmac/.gitignore", "operator-a/\noperator-b");
    let mut children: Vec<Child> = (0..16)
        .map(|_| spawn_rtm("webv-sixteen", &isolate, concurrent.path(), &["init"], &[]))
        .collect();
    let outputs: Vec<Output> = children
        .iter_mut()
        .map(|child| reap("webv-sixteen", child, CHILD_CEILING))
        .collect();
    let policy_line = rendered(&concurrent.path().join(".ratmac").join(".gitignore"));
    let installed_line = format!("rtm: installed the Engine-root tracking policy at {policy_line}");
    let present_line = format!("rtm: Engine-root tracking policy already present at {policy_line}");
    let mut any_installed = false;
    for output in &outputs {
        assert!(
            output.status.success(),
            "WEBV-008 sixteen: every concurrent init exits 0 (a waiter waits for at most one \
             writer's step, never for the number of concurrent callers): {}",
            combined_text(output)
        );
        let text = stdout_text(output);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines.len(),
            3,
            "WEBV-008 sixteen: every concurrent init prints its three lines: {text:?}"
        );
        assert!(
            lines[0] == installed_line || lines[0] == present_line,
            "WEBV-008 sixteen: line 1 is the installed or already-present line: {text:?}"
        );
        any_installed |= lines[0] != present_line;
        assert_eq!(
            lines[1], VERIFIED_LINE,
            "WEBV-008 sixteen: line 2 is the verified line"
        );
        assert_eq!(
            lines[2], NEXT_SCAFFOLD_LINE,
            "WEBV-008 sixteen: line 3 is the scaffold next line"
        );
        assert!(
            stderr_text(output).is_empty(),
            "WEBV-008 sixteen: a concurrent init prints no error: {}",
            stderr_text(output)
        );
    }
    assert!(
        any_installed,
        "WEBV-008 sixteen: at least one concurrent run did the installing"
    );
    assert_eq!(
        fs::read(concurrent.path().join(".ratmac").join(".gitignore"))
            .expect("read the sixteen-run policy"),
        MERGED_POLICY,
        "WEBV-008 sixteen: the policy file holds the operator's bytes followed by each of the \
         four lines exactly once, in contract order"
    );
    assert_eq!(
        ratmac_tagged(concurrent.path()),
        BTreeMap::from([(
            ".gitignore".to_owned(),
            [b"F\0".to_vec(), MERGED_POLICY.to_vec()].concat()
        )]),
        "WEBV-008 sixteen: the Engine root holds the policy file alone - no staging copy, claim, \
         or any other leftover"
    );
}

// --- WTCV-010..012 (t-149 / WTC-004): abandon's lock-free read waits out a release window

/// The test-only presence fault, `<domains>:<reads>:<code>`: which lock's
/// presence reads fail (`root`, `run`, or `root,run`), how many fail (the
/// first `N` reads of each named lock, or `all`), and the raw OS error code
/// they fail with. It reaches only the child it is set on (SVE-006).
const PRESENCE_FAULT: &str = "RATMAC_TEST_LOCK_PRESENCE_FAULT";

/// The fault's companion: the file outside the project where each injected
/// failure is recorded as one whole line, `<domain> <code> <lock path>`.
const PRESENCE_RECORD: &str = "RATMAC_TEST_LOCK_PRESENCE_RECORD";

/// The raw code a read answers inside a Windows peer's release window.
const ACCESS_DENIED: i32 = 5;

/// The other-code control: a sharing violation is never a release window.
const SHARING_VIOLATION: i32 = 32;

/// The lock module's one wait bound, measured from the first denial.
const LOCK_WAIT_BOUND: Duration = Duration::from_secs(5);

/// How many leading presence reads of each named lock the settling cases
/// deny.
const SETTLING_DENIALS: usize = 3;

/// Today's success line for a retirement whose only remnant is a lock.
const LEFTOVER_RETIRED: &str =
    "rtm: retirement completed; the leftover lock is retired and no Run remains.\n";

/// A leftover lock body whose owner cannot be alive (t-115's shape): the
/// pid is far outside any supported host's process-id range.
fn stale_lock_token(guard: &str) -> String {
    format!("ratmac-lock-v1\npid=2000000000\nguard={guard}\nnonce=0\n")
}

/// The fault selector and its record as one child's explicit environment.
fn presence_fault<'a>(selector: &'a str, record: &'a Path) -> [(&'static str, &'a OsStr); 2] {
    [
        (PRESENCE_FAULT, OsStr::new(selector)),
        (PRESENCE_RECORD, record.as_os_str()),
    ]
}

/// A fault record in a tree of its own, outside every project. The tree
/// must outlive the record's reads.
fn record_tree(label: &str) -> (TempTree, PathBuf) {
    let tree = TempTree::new(&format!("t136-wtcv-{label}-record")).expect("own a record tree");
    let record = tree.join("record");
    (tree, record)
}

/// The record's whole lines, read after its writer exited: a missing
/// record holds no line, and only newline-terminated text counts.
fn record_lines(record: &Path) -> Vec<String> {
    let text = match fs::read_to_string(record) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(error) => panic!("read the fault record {}: {error}", rendered(record)),
    };
    text.split_inclusive('\n')
        .filter_map(|line| line.strip_suffix('\n'))
        .map(str::to_owned)
        .collect()
}

/// One expected record line: an injected `code` at a presence read of
/// `lock`.
fn record_line(domain: &str, code: i32, lock: &Path) -> String {
    format!("{domain} {code} {}", rendered(lock))
}

fn root_lock_path(root: &Path) -> PathBuf {
    root.join(".ratmac/locks/root.lock")
}

fn run_lock_path(root: &Path, id: &str) -> PathBuf {
    root.join(".ratmac/locks/runs").join(format!("{id}.lock"))
}

fn run_record_path(root: &Path, id: &str) -> PathBuf {
    root.join(".ratmac/runs").join(id).join("run.toml")
}

/// The text the standard library gives a raw OS error code here.
fn os_error_text(code: i32) -> String {
    std::io::Error::from_raw_os_error(code).to_string()
}

/// Today's planning-route refusal for a lock read that did not settle.
fn inspect_refusal(lock: &Path, code: i32) -> String {
    format!(
        "rtm: abandon refused; abandon cannot inspect lock {}: {}\n",
        rendered(lock),
        os_error_text(code)
    )
}

/// The folder name a project phrase and a project refusal carry.
fn project_name(root: &Path) -> String {
    root.file_name()
        .expect("a fixture root has a folder name")
        .to_string_lossy()
        .into_owned()
}

/// A committed-runbook repository holding one Run a plain start admitted,
/// with that Run's id.
fn started_repo(isolate: &GitIsolate, label: &str) -> (TempTree, String) {
    let repo = committed_runbook_repo(isolate, label);
    let start_label = format!("{label} start");
    let started = run_rtm(&start_label, isolate, repo.path(), &["start"], &[]);
    assert!(
        started.status.success(),
        "{label}: a plain start admits a Run: {}",
        combined_text(&started)
    );
    let id = started_run_id(&start_label, &started);
    (repo, id)
}

/// A committed-runbook repository whose only remnant is a leftover root
/// lock: no Run was ever admitted.
fn leftover_repo(isolate: &GitIsolate, label: &str) -> TempTree {
    let repo = committed_runbook_repo(isolate, label);
    repo.write(".ratmac/locks/root.lock", stale_lock_token("root"));
    repo
}

/// `rtm abandon --run <id> --confirm "abandon <id>"` with `vars` on the
/// child alone.
fn addressed_abandon(
    label: &str,
    isolate: &GitIsolate,
    root: &Path,
    id: &str,
    vars: &[(&str, &OsStr)],
) -> Output {
    let phrase = format!("abandon {id}");
    run_rtm(
        label,
        isolate,
        root,
        &["abandon", "--run", id, "--confirm", &phrase],
        vars,
    )
}

/// `rtm abandon --confirm "abandon <project folder name>"` with `vars` on
/// the child alone.
fn leftover_abandon(
    label: &str,
    isolate: &GitIsolate,
    root: &Path,
    vars: &[(&str, &OsStr)],
) -> Output {
    let phrase = format!("abandon {}", project_name(root));
    run_rtm(
        label,
        isolate,
        root,
        &["abandon", "--confirm", &phrase],
        vars,
    )
}

/// Today's success line for retiring a Run admitted at `state`.
fn abandoned_line(state: &str) -> String {
    format!(
        "rtm: Run abandoned at state {state}; admission state, Run evidence, and lock retired. \
         A fresh rtm start may begin.\n"
    )
}

/// The State a plain start admits a Run at: the first State of `RUNBOOK`.
const INITIAL_STATE: &str = "plan";

/// WTCV-010: denials of the first presence reads that then settle decide
/// exactly as an unfaulted read - an addressed Run and a leftover root lock
/// each retire with today's success line.
#[test]
fn wtcv_010() {
    let isolate = GitIsolate::new("t136-wtcv-010-isolate");
    #[cfg(windows)]
    assert_eq!(
        os_error_text(ACCESS_DENIED),
        "Access is denied. (os error 5)",
        "WTCV-010: the injected code is the release window's own answer"
    );

    // Valid controls: unfaulted twins retire with today's success lines.
    let (run_twin, twin_id) = started_repo(&isolate, "wtcv-010-run-twin");
    let run_reference = addressed_abandon(
        "WTCV-010 run twin",
        &isolate,
        run_twin.path(),
        &twin_id,
        &[],
    );
    assert_eq!(
        (exit_code(&run_reference), stdout_text(&run_reference)),
        (0, abandoned_line(INITIAL_STATE)),
        "WTCV-010 run twin: an unfaulted addressed abandon retires its Run as today: {}",
        combined_text(&run_reference)
    );
    let leftover_twin = leftover_repo(&isolate, "wtcv-010-leftover-twin");
    let leftover_reference = leftover_abandon(
        "WTCV-010 leftover twin",
        &isolate,
        leftover_twin.path(),
        &[],
    );
    assert_eq!(
        (
            exit_code(&leftover_reference),
            stdout_text(&leftover_reference)
        ),
        (0, LEFTOVER_RETIRED.to_owned()),
        "WTCV-010 leftover twin: an unfaulted project-phrase abandon retires the leftover root \
         lock as today: {}",
        combined_text(&leftover_reference)
    );

    // The faulted cases: the first reads of each named lock are denied.
    let (run_repo, id) = started_repo(&isolate, "wtcv-010-run");
    let (_run_record_tree, run_record) = record_tree("010-run");
    let run_selector = format!("root,run:{SETTLING_DENIALS}:{ACCESS_DENIED}");
    let run_output = addressed_abandon(
        "WTCV-010 run",
        &isolate,
        run_repo.path(),
        &id,
        &presence_fault(&run_selector, &run_record),
    );
    let leftover = leftover_repo(&isolate, "wtcv-010-leftover");
    let (_leftover_record_tree, leftover_record) = record_tree("010-leftover");
    let leftover_selector = format!("root:{SETTLING_DENIALS}:{ACCESS_DENIED}");
    let leftover_output = leftover_abandon(
        "WTCV-010 leftover",
        &isolate,
        leftover.path(),
        &presence_fault(&leftover_selector, &leftover_record),
    );

    let root_lock = root_lock_path(run_repo.path());
    let run_lock = run_lock_path(run_repo.path(), &id);
    let leftover_lock = root_lock_path(leftover.path());
    // Windows reads again until a read settles; elsewhere the first denial
    // is the single read's answer.
    let denials = if cfg!(windows) { SETTLING_DENIALS } else { 1 };
    let mut expected_run_record = vec![record_line("root", ACCESS_DENIED, &root_lock); denials];
    if cfg!(windows) {
        expected_run_record.extend(vec![record_line("run", ACCESS_DENIED, &run_lock); denials]);
    }
    assert_eq!(
        record_lines(&run_record),
        expected_run_record,
        "WTCV-010 run: the fault's record shows a denial at each lock path read - the first \
         reads of the root lock, then of the Run lock (the record is missing when no fault \
         exists); abandon said: {}",
        combined_text(&run_output)
    );
    assert_eq!(
        record_lines(&leftover_record),
        vec![record_line("root", ACCESS_DENIED, &leftover_lock); denials],
        "WTCV-010 leftover: the fault's record shows a denial at the root lock path read (the \
         record is missing when no fault exists); abandon said: {}",
        combined_text(&leftover_output)
    );

    if cfg!(windows) {
        assert_eq!(
            exit_code(&run_output),
            0,
            "WTCV-010 run: the waited-out denials retire the Run as today, never `abandon cannot \
             inspect lock`: {}",
            combined_text(&run_output)
        );
        assert_eq!(
            stdout_text(&run_output),
            retarget(&stdout_text(&run_reference), &twin_id, &id),
            "WTCV-010 run: stdout is byte-identical to the unfaulted twin's"
        );
        assert_eq!(
            stderr_text(&run_output),
            stderr_text(&run_reference),
            "WTCV-010 run: stderr is byte-identical to the unfaulted twin's"
        );
        assert!(
            !run_record_path(run_repo.path(), &id).exists(),
            "WTCV-010 run: the Run Record is retired"
        );
        assert!(
            !root_lock.exists() && !run_lock.exists(),
            "WTCV-010 run: no lock file remains"
        );
        assert_eq!(
            exit_code(&leftover_output),
            0,
            "WTCV-010 leftover: the waited-out denials retire the leftover root lock as today, \
             never `abandon cannot inspect lock`: {}",
            combined_text(&leftover_output)
        );
        assert_eq!(
            stdout_text(&leftover_output),
            stdout_text(&leftover_reference),
            "WTCV-010 leftover: stdout is byte-identical to the unfaulted twin's"
        );
        assert_eq!(
            stderr_text(&leftover_output),
            stderr_text(&leftover_reference),
            "WTCV-010 leftover: stderr is byte-identical to the unfaulted twin's"
        );
        assert!(
            !leftover_lock.exists(),
            "WTCV-010 leftover: the leftover root lock is retired"
        );
    } else {
        for (label, output, lock) in [
            ("WTCV-010 run", &run_output, &root_lock),
            ("WTCV-010 leftover", &leftover_output, &leftover_lock),
        ] {
            assert_eq!(
                (exit_code(output), stdout_text(output), stderr_text(output)),
                (1, String::new(), inspect_refusal(lock, ACCESS_DENIED)),
                "{label}: other platforms keep the single read, so the injected denial refuses \
                 at once with today's `abandon cannot inspect lock`"
            );
        }
    }
}

/// WTCV-011: denials that settle to not-found act exactly as the read
/// without the fault - the unaddressed abandon refuses with today's
/// nothing-to-retire text and the addressed one retires its Run.
#[test]
fn wtcv_011() {
    let isolate = GitIsolate::new("t136-wtcv-011-isolate");

    // The byte references, without the fault. The empty project's refusal
    // writes nothing, so its reference runs on the same fixture.
    let empty = committed_runbook_repo(&isolate, "wtcv-011-empty");
    let empty_before = ratmac_tagged(empty.path());
    let empty_reference = run_rtm(
        "WTCV-011 empty reference",
        &isolate,
        empty.path(),
        &["abandon"],
        &[],
    );
    assert_eq!(
        (
            exit_code(&empty_reference),
            stdout_text(&empty_reference),
            stderr_text(&empty_reference)
        ),
        (
            1,
            String::new(),
            format!(
                "rtm: abandon refused; nothing to retire in {}: no live run and no leftover \
                 lock\n",
                project_name(empty.path())
            )
        ),
        "WTCV-011 empty reference: with no live Run and no lock, the unaddressed abandon \
         refuses with today's nothing-to-retire text"
    );
    assert_eq!(
        ratmac_tagged(empty.path()),
        empty_before,
        "WTCV-011 empty reference: the refusal writes nothing"
    );
    let (run_twin, twin_id) = started_repo(&isolate, "wtcv-011-run-twin");
    let run_reference = addressed_abandon(
        "WTCV-011 run twin",
        &isolate,
        run_twin.path(),
        &twin_id,
        &[],
    );
    assert_eq!(
        (exit_code(&run_reference), stdout_text(&run_reference)),
        (0, abandoned_line(INITIAL_STATE)),
        "WTCV-011 run twin: an unfaulted addressed abandon retires its Run as today: {}",
        combined_text(&run_reference)
    );

    // The same fault on lock paths that do not exist.
    let (_empty_record_tree, empty_record) = record_tree("011-empty");
    let empty_selector = format!("root:{SETTLING_DENIALS}:{ACCESS_DENIED}");
    let empty_output = run_rtm(
        "WTCV-011 empty",
        &isolate,
        empty.path(),
        &["abandon"],
        &presence_fault(&empty_selector, &empty_record),
    );
    let (run_repo, id) = started_repo(&isolate, "wtcv-011-run");
    let (_run_record_tree, run_record) = record_tree("011-run");
    let run_selector = format!("root,run:{SETTLING_DENIALS}:{ACCESS_DENIED}");
    let run_output = addressed_abandon(
        "WTCV-011 run",
        &isolate,
        run_repo.path(),
        &id,
        &presence_fault(&run_selector, &run_record),
    );

    let empty_lock = root_lock_path(empty.path());
    let root_lock = root_lock_path(run_repo.path());
    let run_lock = run_lock_path(run_repo.path(), &id);
    let denials = if cfg!(windows) { SETTLING_DENIALS } else { 1 };
    assert_eq!(
        record_lines(&empty_record),
        vec![record_line("root", ACCESS_DENIED, &empty_lock); denials],
        "WTCV-011 empty: the fault's record shows it fired at the root lock path (the record \
         is missing when no fault exists); abandon said: {}",
        combined_text(&empty_output)
    );
    let mut expected_run_record = vec![record_line("root", ACCESS_DENIED, &root_lock); denials];
    if cfg!(windows) {
        expected_run_record.extend(vec![record_line("run", ACCESS_DENIED, &run_lock); denials]);
    }
    assert_eq!(
        record_lines(&run_record),
        expected_run_record,
        "WTCV-011 run: the fault's record shows it fired at each lock path read (the record is \
         missing when no fault exists); abandon said: {}",
        combined_text(&run_output)
    );

    if cfg!(windows) {
        assert_eq!(
            (
                exit_code(&empty_output),
                stdout_text(&empty_output),
                stderr_text(&empty_output)
            ),
            (
                exit_code(&empty_reference),
                stdout_text(&empty_reference),
                stderr_text(&empty_reference)
            ),
            "WTCV-011 empty: exit code, stdout, and stderr are byte-identical to the run without \
             the fault, never `abandon cannot inspect lock`"
        );
        assert_eq!(
            (
                exit_code(&run_output),
                stdout_text(&run_output),
                stderr_text(&run_output)
            ),
            (
                exit_code(&run_reference),
                retarget(&stdout_text(&run_reference), &twin_id, &id),
                stderr_text(&run_reference)
            ),
            "WTCV-011 run: exit code, stdout, and stderr are byte-identical to the run without \
             the fault: the Run retires as today"
        );
        assert!(
            !run_record_path(run_repo.path(), &id).exists(),
            "WTCV-011 run: the Run Record is retired"
        );
    } else {
        for (label, output, lock) in [
            ("WTCV-011 empty", &empty_output, &empty_lock),
            ("WTCV-011 run", &run_output, &root_lock),
        ] {
            assert_eq!(
                (exit_code(output), stdout_text(output), stderr_text(output)),
                (1, String::new(), inspect_refusal(lock, ACCESS_DENIED)),
                "{label}: other platforms keep the single read, so the injected denial refuses \
                 at once with today's `abandon cannot inspect lock`"
            );
        }
    }
    assert_eq!(
        ratmac_tagged(empty.path()),
        empty_before,
        "WTCV-011 empty: the faulted refusal writes nothing either"
    );
}

/// WTCV-012: a denial that outlasts the bound, or any other error, refuses
/// with today's `abandon cannot inspect lock <path>: <error>` and retires
/// nothing.
#[test]
fn wtcv_012() {
    let isolate = GitIsolate::new("t136-wtcv-012-isolate");

    // Valid control: the fixture shape can be retired.
    let (twin, twin_id) = started_repo(&isolate, "wtcv-012-twin");
    let reference = addressed_abandon("WTCV-012 twin", &isolate, twin.path(), &twin_id, &[]);
    assert_eq!(
        (exit_code(&reference), stdout_text(&reference)),
        (0, abandoned_line(INITIAL_STATE)),
        "WTCV-012 twin: an unfaulted addressed abandon retires its Run as today: {}",
        combined_text(&reference)
    );

    // (a) Every presence read denied: the lasting denial.
    let (lasting_repo, lasting_id) = started_repo(&isolate, "wtcv-012-lasting");
    let lasting_before = ratmac_tagged(lasting_repo.path());
    let (_lasting_record_tree, lasting_record) = record_tree("012-lasting");
    let lasting_selector = format!("root,run:all:{ACCESS_DENIED}");
    let started = Instant::now();
    let lasting = addressed_abandon(
        "WTCV-012 lasting denial",
        &isolate,
        lasting_repo.path(),
        &lasting_id,
        &presence_fault(&lasting_selector, &lasting_record),
    );
    let lasting_elapsed = started.elapsed();
    let lasting_lock = root_lock_path(lasting_repo.path());
    assert_eq!(
        exit_code(&lasting),
        1,
        "WTCV-012 lasting denial: abandon refuses with exit 1 (with no fault it retires the \
         Run): {}",
        combined_text(&lasting)
    );
    assert_eq!(
        stderr_text(&lasting),
        inspect_refusal(&lasting_lock, ACCESS_DENIED),
        "WTCV-012 lasting denial: the refusal is today's planning-route text naming the root \
         lock, read first, with the last error"
    );
    #[cfg(windows)]
    assert_eq!(
        stderr_text(&lasting),
        format!(
            "rtm: abandon refused; abandon cannot inspect lock {}: Access is denied. (os error \
             5)\n",
            rendered(&lasting_lock)
        ),
        "WTCV-012 lasting denial: the refusal carries the release window's own answer"
    );
    assert!(
        stdout_text(&lasting).is_empty(),
        "WTCV-012 lasting denial: the refusal prints nothing on stdout: {}",
        stdout_text(&lasting)
    );
    assert_eq!(
        ratmac_tagged(lasting_repo.path()),
        lasting_before,
        "WTCV-012 lasting denial: nothing is retired - the Engine root is byte-identical"
    );
    assert!(
        run_record_path(lasting_repo.path(), &lasting_id).is_file(),
        "WTCV-012 lasting denial: the Run Record stays"
    );
    let lasting_lines = record_lines(&lasting_record);
    let lasting_line = record_line("root", ACCESS_DENIED, &lasting_lock);
    if cfg!(windows) {
        assert!(
            lasting_elapsed >= LOCK_WAIT_BOUND,
            "WTCV-012 lasting denial: abandon refuses no sooner than five seconds after it \
             started; it took {lasting_elapsed:?}"
        );
        assert!(
            lasting_lines.len() >= 2 && lasting_lines.iter().all(|line| *line == lasting_line),
            "WTCV-012 lasting denial: the record shows the root lock read again and again, and \
             never a Run lock read: {lasting_lines:?}"
        );
    } else {
        assert_eq!(
            lasting_lines,
            vec![lasting_line],
            "WTCV-012 lasting denial: other platforms keep the single read"
        );
        assert!(
            lasting_elapsed < LOCK_WAIT_BOUND,
            "WTCV-012 lasting denial: other platforms refuse at once; it took \
             {lasting_elapsed:?}"
        );
    }

    // (b) Another code: a sharing violation is not a release window.
    let (other_repo, other_id) = started_repo(&isolate, "wtcv-012-other");
    let other_before = ratmac_tagged(other_repo.path());
    let (_other_record_tree, other_record) = record_tree("012-other");
    let other_selector = format!("root,run:all:{SHARING_VIOLATION}");
    let started = Instant::now();
    let other = addressed_abandon(
        "WTCV-012 other code",
        &isolate,
        other_repo.path(),
        &other_id,
        &presence_fault(&other_selector, &other_record),
    );
    let other_elapsed = started.elapsed();
    let other_lock = root_lock_path(other_repo.path());
    assert_eq!(
        (exit_code(&other), stdout_text(&other), stderr_text(&other)),
        (
            1,
            String::new(),
            inspect_refusal(&other_lock, SHARING_VIOLATION)
        ),
        "WTCV-012 other code: abandon refuses with the same message shape and that error"
    );
    assert_eq!(
        record_lines(&other_record),
        vec![record_line("root", SHARING_VIOLATION, &other_lock)],
        "WTCV-012 other code: the record shows exactly one injected read"
    );
    assert!(
        other_elapsed < LOCK_WAIT_BOUND,
        "WTCV-012 other code: the refusal comes at once, never after the wait bound (a sanity \
         bound, not a timing oracle); it took {other_elapsed:?}"
    );
    assert_eq!(
        ratmac_tagged(other_repo.path()),
        other_before,
        "WTCV-012 other code: nothing is retired - the Engine root is byte-identical"
    );
    assert!(
        run_record_path(other_repo.path(), &other_id).is_file(),
        "WTCV-012 other code: the Run Record stays"
    );
}
