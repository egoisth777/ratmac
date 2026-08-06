//! t-073 / ENS-005: split the Engine root lock from per-Run motion locks.
//!
//! ENSV-006 `per_run_motion_serializes_without_blocking_other_runs_or_root_work`
//!
//! File barriers make every concurrency observation a handshake: a command
//! guard writes its marker before waiting for the test's release file. The
//! bounded waits below are failure ceilings only; no elapsed duration orders a
//! transition.

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const MARKER_ARRIVAL_TIMEOUT: Duration = Duration::from_secs(5);
const BARRIER_RELEASE_TIMEOUT: Duration = Duration::from_secs(8);
const CHILD_COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);
const START_WORKER_COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);
const KILL_REAP_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug)]
struct ProcessResult {
    completed_before_timeout: bool,
    success: bool,
    output: String,
}

#[derive(Debug)]
struct StartReleaseResult {
    success: bool,
    output: String,
    release_written: bool,
    barrier_timed_out_before_release: bool,
}

struct StartReleaseWorker {
    receiver: Receiver<StartReleaseResult>,
    handle: Option<JoinHandle<()>>,
}

impl StartReleaseWorker {
    fn receive(mut self, timeout: Duration) -> Option<StartReleaseResult> {
        let result = self.receiver.recv_timeout(timeout).ok();
        if result.is_some() {
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
        result
    }
}

/// A release file is written on normal cleanup and unwinding alike, so a
/// fixture failure cannot strand a guard process behind its file barrier.
struct FileBarrier {
    release: PathBuf,
    timeout_marker: PathBuf,
    timeout: Duration,
    released: bool,
}

impl FileBarrier {
    fn new(root: &Path, label: &str) -> Self {
        let directory = root.join("qa-barriers");
        fs::create_dir_all(&directory).expect("create ENSV-006 barrier directory");
        let release = directory.join(format!("{label}.release"));
        let timeout_marker = directory.join(format!("{label}.timed-out"));
        let _ = fs::remove_file(&release);
        let _ = fs::remove_file(&timeout_marker);
        Self {
            release,
            timeout_marker,
            timeout: BARRIER_RELEASE_TIMEOUT,
            released: false,
        }
    }

    fn marker(&self, label: &str) -> PathBuf {
        let marker = self.release.with_file_name(format!("{label}.marker"));
        let _ = fs::remove_file(&marker);
        marker
    }

    fn release(&mut self) -> bool {
        match fs::write(&self.release, "release\n") {
            Ok(()) => {
                self.released = true;
                true
            }
            Err(_) => false,
        }
    }
}

impl Drop for FileBarrier {
    fn drop(&mut self) {
        if !self.released {
            let _ = fs::write(&self.release, "release during fixture cleanup\n");
        }
    }
}

/// A manually held root lock models another root-domain owner without relying
/// on scheduling to catch a short-lived lock acquisition.
struct ForeignRootLock {
    path: PathBuf,
    released: bool,
}

impl ForeignRootLock {
    fn create(path: PathBuf) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        Ok(Self {
            path,
            released: false,
        })
    }

    fn is_file(&self) -> bool {
        self.path.is_file()
    }

    fn release(&mut self) -> bool {
        match fs::remove_file(&self.path) {
            Ok(()) => {
                self.released = true;
                true
            }
            Err(_) => false,
        }
    }
}

impl Drop for ForeignRootLock {
    fn drop(&mut self) {
        if !self.released {
            let _ = fs::remove_file(&self.path);
        }
    }
}

struct Fixture {
    sandbox: PathBuf,
    root: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let sandbox = std::env::temp_dir().join(format!(
            "ratmac-t073-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock must be after the Unix epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&sandbox);
        let root = sandbox.join("checkout");
        fs::create_dir_all(root.join(".ratmac")).expect("create ENSV-006 fixture Engine root");
        fs::write(root.join(".ratmac/ratmac.toml"), barrier_runbook())
            .expect("write ENSV-006 fixture Machine Class");
        Self { sandbox, root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke built rtm binary for ENSV-006")
    }

    fn start_run(&self) -> String {
        let output = self.rtm(&["start"]);
        assert!(
            output.status.success(),
            "ENSV-006 fixture setup: rtm start must mint its initial Run: {}",
            combined(&output)
        );
        started_run_id(&output)
    }

    fn start_child(&self) -> Child {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .arg("start")
            .current_dir(&self.root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn concurrent rtm start for ENSV-006")
    }

    fn step_at_barrier(&self, run_id: &str, marker: &Path, barrier: &FileBarrier) -> Child {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .args(["step", "--run", run_id])
            .current_dir(&self.root)
            .env("RATMAC_QA_BARRIER_MARKER", marker)
            .env("RATMAC_QA_BARRIER_RELEASE", &barrier.release)
            .env("RATMAC_QA_BARRIER_TIMEOUT_MARKER", &barrier.timeout_marker)
            .env(
                "RATMAC_QA_BARRIER_TIMEOUT_MILLIS",
                barrier.timeout.as_millis().to_string(),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn concurrent rtm step for ENSV-006")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.sandbox);
    }
}

#[derive(Debug)]
struct OverlapObservation {
    markers_observed: bool,
    both_guards_waiting: bool,
    release_written: bool,
    first: ProcessResult,
    second: ProcessResult,
    first_advanced: bool,
    second_advanced: bool,
    lock_files: BTreeSet<String>,
    root_lock_absent: bool,
    legacy_lock_absent: bool,
}

impl OverlapObservation {
    fn proves_different_runs_overlap(&self) -> bool {
        self.markers_observed
            && self.both_guards_waiting
            && self.release_written
            && self.first.completed_before_timeout
            && self.second.completed_before_timeout
            && self.first.success
            && self.second.success
            && self.first_advanced
            && self.second_advanced
    }
}

#[derive(Debug)]
struct SameRunObservation {
    first_marker_observed: bool,
    state_unchanged_before_release: bool,
    second_marker_absent_before_release: bool,
    release_written: bool,
    first: ProcessResult,
    second: ProcessResult,
    state_is_single_advance: bool,
    log_entries_before: Option<usize>,
    log_entries_after: Option<usize>,
    per_run_lock_named_on_refusal: bool,
}

impl SameRunObservation {
    fn proves_same_run_serialization(&self) -> bool {
        self.first_marker_observed
            && self.state_unchanged_before_release
            && self.second_marker_absent_before_release
            && self.release_written
            && self.first.completed_before_timeout
            && self.second.completed_before_timeout
            && self.state_is_single_advance
            && self.log_entries_after
                == self
                    .log_entries_before
                    .map(|entries| entries.saturating_add(1))
    }
}

#[derive(Debug)]
struct LongGuardObservation {
    marker_observed: bool,
    root_lock_absent_while_parked: bool,
    guard: ProcessResult,
    guard_advanced: bool,
    guard_timed_out: bool,
    start: Option<StartReleaseResult>,
    roster_before: BTreeSet<String>,
    roster_after: BTreeSet<String>,
}

impl LongGuardObservation {
    fn proves_root_work_is_not_blocked(&self) -> bool {
        self.marker_observed
            && self.root_lock_absent_while_parked
            && self.guard.completed_before_timeout
            && self.guard.success
            && self.guard_advanced
            && !self.guard_timed_out
            && self.start.as_ref().is_some_and(|start| {
                start.success && start.release_written && !start.barrier_timed_out_before_release
            })
            && minted_one_run(&self.roster_before, &self.roster_after)
    }
}

#[derive(Debug)]
struct RootDomainObservation {
    root_lock_started_absent: bool,
    foreign_root_lock_present: bool,
    marker_observed: bool,
    start_not_successful_while_held: bool,
    no_mint_while_held: bool,
    release_written: bool,
    step_finished_while_root_held: bool,
    step: ProcessResult,
    blocked_start: ProcessResult,
    foreign_root_lock_removed: bool,
    fresh_start_success: bool,
    fresh_start_minted: bool,
    legacy_lock_absent: bool,
}

impl RootDomainObservation {
    fn proves_root_domain_and_unrelated_motion(&self) -> bool {
        self.root_lock_started_absent
            && self.foreign_root_lock_present
            && self.marker_observed
            && self.start_not_successful_while_held
            && self.no_mint_while_held
            && self.release_written
            && self.step_finished_while_root_held
            && self.step.completed_before_timeout
            && self.step.success
            && self.foreign_root_lock_removed
            && self.fresh_start_success
            && self.fresh_start_minted
            && self.legacy_lock_absent
    }
}

fn barrier_runbook() -> String {
    let program = env!("CARGO_BIN_EXE_lock-barrier")
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!(
        r#"[phases.prepare]
prompt = "Park at the deterministic guard barrier."
guards = [{{ kind = "command_exit", program = "{program}", expected = 0 }}]

[phases.done]
prompt = "The guarded transition completed."

[[transitions]]
from = "prepare"
to = "done"
"#
    )
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn started_run_id(output: &Output) -> String {
    let text = combined(output);
    text.split("rtm: started run ")
        .nth(1)
        .and_then(|suffix| suffix.split_whitespace().next())
        .unwrap_or_else(|| panic!("ENSV-006 fixture setup: start output lacks a Run id: {text}"))
        .to_owned()
}

fn wait_until(timeout: Duration, mut predicate: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if predicate() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::yield_now();
    }
}

fn wait_for_files(paths: &[&Path], timeout: Duration) -> bool {
    wait_until(timeout, || paths.iter().all(|path| path.is_file()))
}

fn child_is_live(child: &mut Child) -> bool {
    matches!(child.try_wait(), Ok(None))
}

fn child_exited_before(child: &mut Child, timeout: Duration) -> bool {
    let mut inspection_failed = false;
    let observed = wait_until(timeout, || match child.try_wait() {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(_) => {
            inspection_failed = true;
            true
        }
    });
    observed && !inspection_failed
}

fn reap_child(mut child: Child, timeout: Duration) -> ProcessResult {
    let mut inspection_failed = false;
    let observed = wait_until(timeout, || match child.try_wait() {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(_) => {
            inspection_failed = true;
            true
        }
    });
    let completed_before_timeout = observed && !inspection_failed;
    let exited = if completed_before_timeout {
        true
    } else {
        let _ = child.kill();
        wait_until(KILL_REAP_TIMEOUT, || {
            matches!(child.try_wait(), Ok(Some(_)))
        })
    };
    if !exited {
        return ProcessResult {
            completed_before_timeout: false,
            success: false,
            output: "child did not exit after the named cleanup timeout".to_owned(),
        };
    }
    match child.wait_with_output() {
        Ok(output) => ProcessResult {
            completed_before_timeout,
            success: output.status.success(),
            output: combined(&output),
        },
        Err(error) => ProcessResult {
            completed_before_timeout: false,
            success: false,
            output: format!("collect child output: {error}"),
        },
    }
}

fn stop_child(child: Child) -> ProcessResult {
    reap_child(child, Duration::ZERO)
}

fn run_roster(root: &Path) -> BTreeSet<String> {
    let Ok(entries) = fs::read_dir(root.join(".ratmac/runs")) else {
        return BTreeSet::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

fn minted_one_run(before: &BTreeSet<String>, after: &BTreeSet<String>) -> bool {
    after.len() == before.len().saturating_add(1) && before.is_subset(after)
}

fn state_bytes(root: &Path, run_id: &str) -> Option<Vec<u8>> {
    fs::read(root.join(".ratmac/runs").join(run_id).join("state.toml")).ok()
}

fn state_is_single_advance(root: &Path, run_id: &str) -> bool {
    let Some(bytes) = state_bytes(root, run_id) else {
        return false;
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return false;
    };
    let Ok(state) = text.parse::<toml::Value>() else {
        return false;
    };
    state.get("phase").and_then(toml::Value::as_str) == Some("done")
        && state.get("status").and_then(toml::Value::as_str) == Some("passed")
}

fn transition_log_entries(root: &Path) -> Option<usize> {
    fs::read_to_string(root.join(".ratmac/log.md"))
        .ok()
        .map(|log| log.lines().filter(|line| !line.is_empty()).count())
}

fn lock_file_snapshot(root: &Path) -> BTreeSet<String> {
    fn collect(root: &Path, directory: &Path, files: &mut BTreeSet<String>) {
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect(root, &path, files);
            } else if path.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .expect("lock snapshot path remains under locks directory")
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(relative);
            }
        }
    }

    let locks = root.join(".ratmac/locks");
    let mut files = BTreeSet::new();
    collect(&locks, &locks, &mut files);
    files
}

fn expected_run_locks(first: &str, second: &str) -> BTreeSet<String> {
    [format!("runs/{first}.lock"), format!("runs/{second}.lock")]
        .into_iter()
        .collect()
}

fn normalized(text: &str) -> String {
    text.replace('\\', "/")
}

fn spawn_start_then_release(
    root: &Path,
    release: &Path,
    timeout_marker: &Path,
) -> StartReleaseWorker {
    let (sender, receiver) = mpsc::channel();
    let root = root.to_path_buf();
    let release = release.to_path_buf();
    let timeout_marker = timeout_marker.to_path_buf();
    let handle = thread::spawn(move || {
        let output = Command::new(env!("CARGO_BIN_EXE_rtm"))
            .arg("start")
            .current_dir(root)
            .output();
        let (success, output) = match output {
            Ok(output) => {
                let success = output.status.success();
                (success, combined(&output))
            }
            Err(error) => (false, format!("invoke rtm start: {error}")),
        };
        let barrier_timed_out_before_release = timeout_marker.is_file();
        let release_written =
            success && fs::write(&release, "release after successful rtm start\n").is_ok();
        let _ = sender.send(StartReleaseResult {
            success,
            output,
            release_written,
            barrier_timed_out_before_release,
        });
    });
    StartReleaseWorker {
        receiver,
        handle: Some(handle),
    }
}

fn observe_different_run_overlap(
    fixture: &Fixture,
    first_run: &str,
    second_run: &str,
) -> OverlapObservation {
    let mut barrier = FileBarrier::new(&fixture.root, "different-runs");
    let first_marker = barrier.marker("different-runs-first");
    let second_marker = barrier.marker("different-runs-second");
    let mut first = fixture.step_at_barrier(first_run, &first_marker, &barrier);
    let mut second = fixture.step_at_barrier(second_run, &second_marker, &barrier);

    let markers_observed = wait_for_files(&[&first_marker, &second_marker], MARKER_ARRIVAL_TIMEOUT);
    let both_guards_waiting = markers_observed
        && child_is_live(&mut first)
        && child_is_live(&mut second)
        && !barrier.release.is_file();
    let lock_files = lock_file_snapshot(&fixture.root);
    let root_lock_absent = !fixture.root.join(".ratmac/locks/root.lock").exists();
    let legacy_lock_absent = !fixture.root.join(".arca/rtm.lock").exists();

    let release_written = barrier.release();
    let first = reap_child(first, CHILD_COMPLETION_TIMEOUT);
    let second = reap_child(second, CHILD_COMPLETION_TIMEOUT);

    OverlapObservation {
        markers_observed,
        both_guards_waiting,
        release_written,
        first,
        second,
        first_advanced: state_is_single_advance(&fixture.root, first_run),
        second_advanced: state_is_single_advance(&fixture.root, second_run),
        lock_files,
        root_lock_absent,
        legacy_lock_absent,
    }
}

fn observe_same_run_serialization(fixture: &Fixture, run_id: &str) -> SameRunObservation {
    let mut barrier = FileBarrier::new(&fixture.root, "same-run");
    let first_marker = barrier.marker("same-run-first");
    let second_marker = barrier.marker("same-run-second");
    let state_before = state_bytes(&fixture.root, run_id);
    let log_entries_before = transition_log_entries(&fixture.root);
    let mut first = fixture.step_at_barrier(run_id, &first_marker, &barrier);

    let first_marker_observed = wait_for_files(&[&first_marker], MARKER_ARRIVAL_TIMEOUT);
    let second = fixture.step_at_barrier(run_id, &second_marker, &barrier);
    let state_unchanged_before_release = state_before
        .as_deref()
        .is_some_and(|before| state_bytes(&fixture.root, run_id).as_deref() == Some(before));
    let second_marker_absent_before_release = !second_marker.is_file();

    // The first marker proves a guard is parked. The State File snapshot proves
    // the newly spawned second caller has not committed before this release.
    let release_written = barrier.release();
    let first = reap_child(first, CHILD_COMPLETION_TIMEOUT);
    let second = reap_child(second, CHILD_COMPLETION_TIMEOUT);
    let expected_lock = format!(".ratmac/locks/runs/{run_id}.lock");
    let per_run_lock_named_on_refusal =
        second.success || normalized(&second.output).contains(&expected_lock);

    SameRunObservation {
        first_marker_observed,
        state_unchanged_before_release,
        second_marker_absent_before_release,
        release_written,
        first,
        second,
        state_is_single_advance: state_is_single_advance(&fixture.root, run_id),
        log_entries_before,
        log_entries_after: transition_log_entries(&fixture.root),
        per_run_lock_named_on_refusal,
    }
}

fn observe_long_guard_does_not_block_start(
    fixture: &Fixture,
    run_id: &str,
) -> LongGuardObservation {
    let mut barrier = FileBarrier::new(&fixture.root, "long-guard");
    let marker = barrier.marker("long-guard");
    let roster_before = run_roster(&fixture.root);
    let guard = fixture.step_at_barrier(run_id, &marker, &barrier);
    let marker_observed = wait_for_files(&[&marker], MARKER_ARRIVAL_TIMEOUT);
    let root_lock_absent_while_parked = !fixture.root.join(".ratmac/locks/root.lock").exists();

    // This worker is the only writer of the release file. It performs that
    // write only after the root-domain start command has returned successfully.
    let worker = marker_observed.then(|| {
        spawn_start_then_release(&fixture.root, &barrier.release, &barrier.timeout_marker)
    });
    if !marker_observed {
        let _ = barrier.release();
    }

    let guard = reap_child(guard, BARRIER_RELEASE_TIMEOUT + CHILD_COMPLETION_TIMEOUT);
    let start = worker.and_then(|worker| worker.receive(START_WORKER_COMPLETION_TIMEOUT));
    let guard_timed_out = barrier.timeout_marker.is_file();
    let roster_after = run_roster(&fixture.root);

    LongGuardObservation {
        marker_observed,
        root_lock_absent_while_parked,
        guard_advanced: state_is_single_advance(&fixture.root, run_id),
        guard,
        guard_timed_out,
        start,
        roster_before,
        roster_after,
    }
}

fn observe_root_domain(fixture: &Fixture, unrelated_run: &str) -> RootDomainObservation {
    let root_lock_path = fixture.root.join(".ratmac/locks/root.lock");
    let root_lock_started_absent = !root_lock_path.exists();
    let mut foreign_root = ForeignRootLock::create(root_lock_path.clone())
        .expect("create the ENSV-006 foreign root-lock holder");
    let foreign_root_lock_present = foreign_root.is_file();
    let roster_before = run_roster(&fixture.root);
    let mut blocked_start = fixture.start_child();

    let mut barrier = FileBarrier::new(&fixture.root, "foreign-root");
    let marker = barrier.marker("foreign-root-step");
    let mut step = fixture.step_at_barrier(unrelated_run, &marker, &barrier);
    let marker_observed = wait_for_files(&[&marker], MARKER_ARRIVAL_TIMEOUT);
    let start_not_successful_while_held = match blocked_start.try_wait() {
        Ok(Some(status)) => !status.success(),
        Ok(None) => true,
        Err(_) => false,
    };
    let no_mint_while_held = run_roster(&fixture.root) == roster_before;

    let release_written = barrier.release();
    let step_finished_while_root_held =
        marker_observed && child_exited_before(&mut step, CHILD_COMPLETION_TIMEOUT);

    // Stop the blocked attempt before releasing the foreign holder. A new start
    // below then proves that the same root pathname admits minting once free.
    let blocked_start = stop_child(blocked_start);
    let foreign_root_lock_removed = foreign_root.release();
    let step = reap_child(step, CHILD_COMPLETION_TIMEOUT);

    let roster_before_fresh_start = run_roster(&fixture.root);
    let fresh_start = fixture.rtm(&["start"]);
    let fresh_start_success = fresh_start.status.success();
    let fresh_start_minted = minted_one_run(&roster_before_fresh_start, &run_roster(&fixture.root));

    RootDomainObservation {
        root_lock_started_absent,
        foreign_root_lock_present,
        marker_observed,
        start_not_successful_while_held,
        no_mint_while_held,
        release_written,
        step_finished_while_root_held,
        step,
        blocked_start,
        foreign_root_lock_removed,
        fresh_start_success,
        fresh_start_minted,
        legacy_lock_absent: !fixture.root.join(".arca/rtm.lock").exists(),
    }
}

#[test]
fn per_run_motion_serializes_without_blocking_other_runs_or_root_work() {
    let fixture = Fixture::new("lock-split");
    let different_first = fixture.start_run();
    let different_second = fixture.start_run();
    let same_run = fixture.start_run();
    let long_guard_run = fixture.start_run();
    let unrelated_run = fixture.start_run();

    let overlap = observe_different_run_overlap(&fixture, &different_first, &different_second);
    let same = observe_same_run_serialization(&fixture, &same_run);
    let long_guard = observe_long_guard_does_not_block_start(&fixture, &long_guard_run);
    let root_domain = observe_root_domain(&fixture, &unrelated_run);
    let expected_locks = expected_run_locks(&different_first, &different_second);

    // The single invocation lock already preserves same-Run State File and log
    // consistency. Keep that oracle separate from the required lock pathname,
    // so this test does not manufacture a same-Run failure from retry policy.
    assert!(
        same.proves_same_run_serialization(),
        "ENSV-006 claim 2: concurrent steps on one Run must leave exactly one committed transition, one transition-log entry, and a State File for one advance; observation: {same:#?}"
    );
    assert!(
        overlap.proves_different_runs_overlap(),
        "ENSV-006 claim 1: timed out after {MARKER_ARRIVAL_TIMEOUT:?} waiting for both different-Run guard markers while the shared release file was absent; a second marker is required to prove overlap rather than serialized motion. Observation: {overlap:#?}"
    );
    assert!(
        long_guard.proves_root_work_is_not_blocked(),
        "ENSV-006 claim 3: a successful concurrent rtm start must mint a new Run and write the long guard's release before its named barrier deadline; observation: {long_guard:#?}"
    );
    assert!(
        overlap.lock_files == expected_locks
            && overlap.root_lock_absent
            && overlap.legacy_lock_absent
            && same.per_run_lock_named_on_refusal
            && root_domain.proves_root_domain_and_unrelated_motion(),
        "ENSV-006 claim 4: while both guards are parked, lock files must be exactly {expected_locks:?} under .ratmac/locks/runs/ with root.lock absent; a foreign .ratmac/locks/root.lock must block minting but not unrelated motion, and .arca/rtm.lock must never appear. Overlap: {overlap:#?}; same-Run: {same:#?}; root-domain: {root_domain:#?}"
    );
}
