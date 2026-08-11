//! Public contract checks for deterministic guard-gated transition requests (t-016).
//!
//! The fixture's Machine Class and artifacts are copied into an isolated project; the
//! Scheduler's `start` operation owns Run/State File creation.  These tests do not
//! manufacture scheduler-owned state on disk.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ratmac::{Scheduler, StepOutcome, StepRequest};

fn isolated_project(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("ratmac-{label}-{}-{nonce}", std::process::id()))
}

fn copy_fixture(fixture: &Path, project: &Path) {
    fs::create_dir_all(project).expect("create isolated project root");
    for entry in fs::read_dir(fixture).expect("read guard fixture") {
        let entry = entry.expect("read fixture entry");
        let source = entry.path();
        let destination = project.join(entry.file_name());
        if source.is_dir() {
            copy_fixture(&source, &destination);
        } else {
            fs::copy(source, destination).expect("copy fixture input");
        }
    }
}

fn fixture_root(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures")
        .join(name)
}

#[test]
fn step_request_refuses_when_guard_fails() {
    let project = isolated_project("t016-failing");
    copy_fixture(&fixture_root("t016-failing-guard"), &project);
    let mut scheduler = Scheduler::open(&project).expect("open isolated Scheduler");
    scheduler
        .start()
        .expect("start Run from fixture Machine Class");

    let before = scheduler.load_state().expect("load current Run state");
    let outcome = scheduler
        .step(StepRequest::new("completed despite missing artifact"))
        .expect("guard evaluation returns a refusal outcome");
    let after = scheduler.load_state().expect("reload current Run state");

    assert!(
        matches!(outcome, StepOutcome::Refused { .. }),
        "a failed artifact guard must refuse step"
    );
    assert_eq!(
        after.state, before.state,
        "refused request cannot advance State"
    );
    let _ = fs::remove_dir_all(project);
}

#[test]
fn step_request_advances_when_guards_pass() {
    let project = isolated_project("t016-passing");
    copy_fixture(&fixture_root("t016-passing-guards"), &project);
    let mut scheduler = Scheduler::open(&project).expect("open isolated Scheduler");
    scheduler
        .start()
        .expect("start Run from fixture Machine Class");

    let before = scheduler.load_state().expect("load current Run state");
    let outcome = scheduler
        .step(StepRequest::new("the agent claims completion"))
        .expect("guard evaluation returns an accepted outcome");
    let after = scheduler.load_state().expect("reload current Run state");

    assert!(matches!(outcome, StepOutcome::Advanced { .. }));
    assert_eq!(before.state, "build");
    assert_eq!(
        after.state, "review",
        "passing guards advance to next State"
    );
    let _ = fs::remove_dir_all(project);
}
