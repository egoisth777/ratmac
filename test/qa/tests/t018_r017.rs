use ratmac::{Scheduler, StepOutcome, StepRequest};

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempProject(PathBuf);

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("fixtures")
        .join("t018-failed-guard")
}

fn isolated_project() -> (TempProject, PathBuf, PathBuf) {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos();
    let root = env::temp_dir().join(format!("ratmac-pt-018-{}-{}", std::process::id(), nonce));
    let arca = root.join(".arca");
    fs::create_dir_all(&arca).expect("create isolated .arca directory");
    for name in ["ratmac.toml", "state.toml", "log.md"] {
        fs::copy(fixture_root().join(".arca").join(name), arca.join(name))
            .expect("copy failed-guard fixture file");
    }
    (
        TempProject(root),
        arca.join("state.toml"),
        arca.join("log.md"),
    )
}

fn phase_and_status(state_bytes: &[u8]) -> (String, String) {
    let state_text = String::from_utf8(state_bytes.to_vec()).expect("state is UTF-8");
    let state: toml::Value = state_text.parse().expect("state fixture is valid TOML");
    let table = state.as_table().expect("state is a TOML table");
    let phase = table
        .get("phase")
        .and_then(toml::Value::as_str)
        .expect("state has phase")
        .to_owned();
    let status = table
        .get("status")
        .and_then(toml::Value::as_str)
        .expect("state has status")
        .to_owned();
    (phase, status)
}

fn assert_no_retry_counter(arca: &Path) {
    for entry in fs::read_dir(arca).expect("read scheduler-owned directory") {
        let name = entry
            .expect("read scheduler-owned directory entry")
            .file_name()
            .to_string_lossy()
            .to_ascii_lowercase();
        assert!(
            !name.contains("retry"),
            "refusal must not create a retry counter"
        );
    }
}

fn run_failed_step(project_root: &Path) {
    let mut scheduler = Scheduler::open(project_root).expect("open failed-guard fixture");
    let request = StepRequest::new("agent claims completion");
    let outcome = scheduler
        .step(request)
        .expect("failed guard should return a refusal outcome");
    assert!(
        matches!(outcome, StepOutcome::Refused { .. }),
        "step with an unsatisfied artifact guard must refuse; report wording is owned by t-020"
    );
}

#[test]
fn failed_guard_refuses_and_stays() {
    let (project, state_path, log_path) = isolated_project();
    let state_before = fs::read(&state_path).expect("read state before refusal");
    let log_before = fs::read(&log_path).expect("read log before refusal");
    let position_before = phase_and_status(&state_before);

    run_failed_step(&project.0);

    let state_after = fs::read(&state_path).expect("read state after refusal");
    let log_after = fs::read(&log_path).expect("read log after refusal");
    assert_eq!(position_before, phase_and_status(&state_after));
    assert_eq!(
        state_before, state_after,
        "refusal must not rewrite State File"
    );
    assert_eq!(
        log_before, log_after,
        "refusal must not append a transition log entry"
    );
}

#[test]
fn failed_guard_writes_no_counter_or_extra_log() {
    let (project, state_path, log_path) = isolated_project();
    let state_before = fs::read(&state_path).expect("read state before refusal");
    let log_before = fs::read(&log_path).expect("read log before refusal");

    run_failed_step(&project.0);
    assert_no_retry_counter(state_path.parent().expect("state has parent directory"));

    let state_after = fs::read(&state_path).expect("read state after refusal");
    let log_after = fs::read(&log_path).expect("read log after refusal");
    assert_eq!(
        state_before, state_after,
        "failed step must not add a retry counter"
    );
    assert_eq!(
        log_before, log_after,
        "failed step must not add a transition entry"
    );
    assert!(
        !String::from_utf8_lossy(&state_after).contains("retry"),
        "State File must not gain a retry counter"
    );
}
