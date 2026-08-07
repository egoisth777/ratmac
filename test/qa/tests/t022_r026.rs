use ratmac::{Scheduler, StepOutcome, StepRequest};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct TempProject(PathBuf);

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A live cross-process Run-lock holder. Its marker is published by
/// `lock-barrier` only after the kernel claim is acquired.
struct LiveRunLock {
    child: Child,
    release: PathBuf,
}

impl LiveRunLock {
    fn acquire(project_root: &Path, engine_root: &Path, run_id: &str) -> Self {
        let barrier_dir = project_root.join("qa-run-lock-barrier");
        fs::create_dir_all(&barrier_dir).expect("create live Run-lock barrier directory");
        let marker = barrier_dir.join("acquired.marker");
        let release = barrier_dir.join("release");
        let timeout_marker = barrier_dir.join("timed-out");
        for path in [&marker, &release, &timeout_marker] {
            let _ = fs::remove_file(path);
        }
        let mut child = Command::new(env!("CARGO_BIN_EXE_lock-barrier"))
            .current_dir(project_root)
            .env("RATMAC_QA_RUN_LOCK_ENGINE", engine_root)
            .env("RATMAC_QA_RUN_LOCK_ID", run_id)
            .env("RATMAC_QA_BARRIER_MARKER", &marker)
            .env("RATMAC_QA_BARRIER_RELEASE", &release)
            .env("RATMAC_QA_BARRIER_TIMEOUT_MARKER", &timeout_marker)
            .env("RATMAC_QA_BARRIER_TIMEOUT_MILLIS", "15000")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn live Run-lock holder");

        let deadline = Instant::now() + Duration::from_secs(5);
        while !marker.is_file() && Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(status)) => panic!(
                    "live Run-lock holder exited before its post-acquisition marker: {status}"
                ),
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(error) => panic!("inspect live Run-lock holder: {error}"),
            }
        }
        assert!(
            marker.is_file(),
            "live Run-lock holder must publish its marker after acquiring the claim"
        );
        Self { child, release }
    }

    fn release(&mut self) -> bool {
        let wrote_release = fs::write(&self.release, "release\n").is_ok();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match self.child.try_wait() {
                Ok(Some(status)) => return wrote_release && status.success(),
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(_) => return false,
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        false
    }
}

impl Drop for LiveRunLock {
    fn drop(&mut self) {
        let _ = fs::write(&self.release, "release during fixture cleanup\n");
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn fixture_root(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("fixtures")
        .join(name)
}

fn isolated_project(fixture: &str) -> (TempProject, PathBuf, PathBuf) {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos();
    let root = env::temp_dir().join(format!("ratmac-pt-022-{}-{}", std::process::id(), nonce));
    let engine = root.join(".ratmac");
    fs::create_dir_all(&engine).expect("create isolated .ratmac directory");
    let source = fixture_root(fixture);
    for name in ["ratmac.toml", "log.md"] {
        fs::copy(source.join(".ratmac").join(name), engine.join(name))
            .expect("copy transition-log fixture file");
    }
    // FDC-004: the State File resides in the addressed run's directory.
    let run_dir = engine.join("runs/run-001");
    fs::create_dir_all(&run_dir).expect("create run directory");
    fs::copy(
        source.join(".ratmac/state.toml"),
        run_dir.join("state.toml"),
    )
    .expect("copy transition-log fixture file");
    fs::create_dir_all(root.join("required")).expect("create passing guard directory");
    fs::copy(
        source.join("required").join("output.txt"),
        root.join("required").join("output.txt"),
    )
    .expect("copy passing guard artifact");
    (
        TempProject(root),
        run_dir.join("state.toml"),
        engine.join("log.md"),
    )
}

fn start_project() -> TempProject {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos();
    let root = env::temp_dir().join(format!(
        "ratmac-pt-022-start-{}-{}",
        std::process::id(),
        nonce
    ));
    let engine = root.join(".ratmac");
    fs::create_dir_all(&engine).expect("create start fixture directory");
    fs::copy(
        fixture_root("r026-transition-log").join(".ratmac/ratmac.toml"),
        engine.join("ratmac.toml"),
    )
    .expect("copy start class fixture");
    TempProject(root)
}

fn successful_step(project_root: &Path) {
    let mut scheduler =
        Scheduler::open_run(project_root, "run-001").expect("open transition-log fixture");
    let request = StepRequest::new("agent claims the required output is complete");
    let outcome = scheduler
        .step(request)
        .expect("passing guards should return a transition outcome");
    assert!(
        matches!(outcome, StepOutcome::Advanced { .. }),
        "all artifact guards passing must advance the Phase"
    );
}

fn refused_step(project_root: &Path) {
    let mut scheduler =
        Scheduler::open_run(project_root, "run-001").expect("open transition-log fixture");
    let outcome = scheduler
        .step(StepRequest::new("agent claims completion"))
        .expect("guard refusal should be reported as an outcome");
    assert!(matches!(outcome, StepOutcome::Refused { .. }));
}

fn state_phase(state_path: &Path) -> String {
    let bytes = fs::read(state_path).expect("read State File");
    let text = String::from_utf8(bytes).expect("State File is UTF-8");
    let value: toml::Value = text.parse().expect("State File is valid TOML");
    value["phase"]
        .as_str()
        .expect("State File has phase")
        .to_owned()
}

fn appended_record(before: &[u8], after: &[u8]) -> String {
    assert!(
        after.starts_with(before),
        "existing Transition Log bytes must remain an exact prefix"
    );
    let appended = &after[before.len()..];
    assert!(
        !appended.is_empty(),
        "successful step must append one record"
    );
    String::from_utf8(appended.to_vec()).expect("appended transition record is UTF-8")
}

#[test]
fn successful_step_appends_one_transition_record() {
    let (project, state_path, log_path) = isolated_project("r026-transition-log");
    let log_before = fs::read(&log_path).expect("read log before successful step");

    successful_step(&project.0);

    let log_after = fs::read(&log_path).expect("read log after successful step");
    let record = appended_record(&log_before, &log_after);
    assert_eq!(
        record
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
        1,
        "one successful step must append exactly one human-readable record"
    );
    assert!(
        record.contains("prepare"),
        "record identifies the source Phase"
    );
    assert!(
        record.contains("done"),
        "record identifies the target Phase"
    );
    assert_eq!(state_phase(&state_path), "done");
}

#[test]
fn transition_log_is_append_only() {
    let (project, state_path, log_path) = isolated_project("r026-transition-log-existing");
    let log_before = fs::read(&log_path).expect("read pre-populated log before step");

    successful_step(&project.0);

    let log_after = fs::read(&log_path).expect("read pre-populated log after step");
    let record = appended_record(&log_before, &log_after);
    assert_eq!(
        record
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
        1,
        "one successful step must append only one new record"
    );
    assert!(record.contains("prepare"));
    assert!(record.contains("done"));
    assert_eq!(state_phase(&state_path), "done");
}

#[test]
fn refused_step_does_not_append_transition_record() {
    let (project, state_path, log_path) = isolated_project("r026-transition-log");
    fs::remove_file(project.0.join("required/output.txt"))
        .expect("remove required artifact to force guard refusal");
    let state_before = fs::read(&state_path).expect("read state before refusal");
    let log_before = fs::read(&log_path).expect("read log before refusal");

    refused_step(&project.0);

    assert_eq!(
        state_before,
        fs::read(&state_path).expect("read state after refusal")
    );
    assert_eq!(
        log_before,
        fs::read(&log_path).expect("read log after refusal")
    );
}

#[test]
fn log_open_failure_leaves_state_unchanged() {
    let (project, state_path, _log_path) = isolated_project("r026-transition-log");
    let log_path = project.0.join(".ratmac/log.md");
    fs::remove_file(&log_path).expect("remove log file to induce open failure");
    fs::create_dir(&log_path).expect("replace log file with directory");
    let state_before = fs::read(&state_path).expect("read state before log failure");
    let mut scheduler =
        Scheduler::open_run(&project.0, "run-001").expect("open transition-log fixture");

    let result = scheduler.step(StepRequest::new("agent claims completion"));

    assert!(result.is_err(), "log directory must make append fail");
    assert_eq!(
        state_before,
        fs::read(&state_path).expect("read state after log failure")
    );
}

#[test]
fn start_log_open_failure_leaves_no_state_and_lock() {
    let project = start_project();
    let engine = project.0.join(".ratmac");
    fs::create_dir(engine.join("log.md")).expect("make log path an open failure");
    let mut scheduler = Scheduler::open(&project.0).expect("open start fixture");

    assert!(scheduler.start().is_err());
    // FDC-004: a failed start leaves neither a flat State File nor a run-owned one.
    assert!(!engine.join("state.toml").exists());
    let no_run_state = match fs::read_dir(engine.join("runs")) {
        Ok(entries) => entries
            .map(|entry| entry.expect("roster entry is readable").path())
            .all(|path| !path.join("state.toml").exists()),
        Err(_) => true,
    };
    assert!(no_run_state, "failed start must leave no run-owned state");
    assert!(!engine.join("locks/root.lock").exists());
}

#[test]
fn missing_transition_log_is_hard_error_without_state_change() {
    let (project, state_path, log_path) = isolated_project("r026-transition-log");
    fs::remove_file(&log_path).expect("remove transition log");
    let state_before = fs::read(&state_path).expect("read state before missing-log step");
    let mut scheduler =
        Scheduler::open_run(&project.0, "run-001").expect("open transition-log fixture");

    assert!(scheduler
        .step(StepRequest::new("agent claims completion"))
        .is_err());
    assert_eq!(
        state_before,
        fs::read(&state_path).expect("read state after missing-log step")
    );
    assert!(!log_path.exists(), "step must not silently create log.md");
}

#[test]
fn non_newline_log_gets_separator_before_record() {
    let (project, _state_path, log_path) = isolated_project("r026-transition-log");
    fs::write(&log_path, b"existing record without newline").expect("write non-newline log prefix");
    let log_before = fs::read(&log_path).expect("read log prefix");

    successful_step(&project.0);

    let log_after = fs::read(&log_path).expect("read log after transition");
    assert!(log_after.starts_with(&log_before));
    assert_eq!(&log_after[log_before.len()..2 + log_before.len()], b"\n-");
}

#[test]
fn state_mutators_honor_run_lock_while_status_is_read_only() {
    let (project, state_path, _log_path) = isolated_project("r026-transition-log");
    let engine_root = project.0.join(".ratmac");
    let scheduler =
        Scheduler::open_run(&project.0, "run-001").expect("open transition-log fixture");
    let lock = ratmac::lock::run_path(&engine_root, "run-001");
    let mut held = LiveRunLock::acquire(&project.0, &engine_root, "run-001");
    assert!(
        lock.is_file(),
        "the post-acquisition marker must correspond to the canonical Run-lock path"
    );
    let state_before = fs::read(&state_path).expect("read state before lock checks");
    // ENS-005 keeps read-only operations outside both lock domains.
    assert!(
        scheduler.status().is_ok(),
        "status remains available while the addressed Run has a live lock holder"
    );
    let state = scheduler
        .load_state()
        .expect("a read-only State File load does not claim a motion lock");

    let initialize = scheduler
        .clone()
        .initialize_state(state.clone())
        .expect_err("a state mutation must wait on the addressed Run lock");
    let expected_refusal = format!("lock wait expired: {}", lock.display());
    assert!(
        initialize.to_string().starts_with(&expected_refusal),
        "the expiry refusal must name exactly the addressed Run lock before any optional diagnostic: {initialize}"
    );
    let missing = scheduler
        .clone()
        .record_missing_prerequisite(state, "input_revision")
        .expect_err("a state mutation must wait on the addressed Run lock");
    assert!(
        missing.to_string().starts_with(&expected_refusal),
        "the expiry refusal must name exactly the addressed Run lock before any optional diagnostic: {missing}"
    );
    assert_eq!(
        state_before,
        fs::read(&state_path).expect("read state after lock checks")
    );
    assert!(
        held.release(),
        "the live Run-lock holder must exit after its deterministic release"
    );
    assert!(
        !lock.exists(),
        "a released live Run-lock holder removes its canonical pathname"
    );
}

#[test]
fn status_on_missing_engine_does_not_create_metadata() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos();
    let project = env::temp_dir().join(format!(
        "ratmac-pt-022-status-missing-{}-{}",
        std::process::id(),
        nonce
    ));
    fs::create_dir_all(&project).expect("create project without .ratmac");
    // TRP-005: a project with no runbook refuses where the runbook is read;
    // R-026 still holds - the refusal creates no metadata.
    let error = Scheduler::open(&project).expect_err("a project without metadata has no machine");
    assert!(error.to_string().contains("ratmac.toml"));
    assert!(!project.join(".ratmac").exists());
    fs::remove_dir_all(project).expect("clean missing-metadata project");
}
