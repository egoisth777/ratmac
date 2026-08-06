//! Public proof that the engine is project-agnostic for arbitrary Machine Classes.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ratmac::{Scheduler, StepOutcome, StepRequest};

fn isolated_project() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("ratmac-t028-{}-{nonce}", std::process::id()))
}

fn copy_fixture(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create isolated project");
    for entry in fs::read_dir(source).expect("read neutral class fixture") {
        let entry = entry.expect("read fixture entry");
        let from = entry.path();
        let to = destination.join(entry.file_name());
        if from.is_dir() {
            copy_fixture(&from, &to);
        } else {
            fs::copy(from, to).expect("copy fixture input");
        }
    }
}

#[test]
fn non_wishwillow_machine_class_runs_identically() {
    let project = isolated_project();
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/r016-non-wishwillow-class");
    copy_fixture(&fixture, &project);

    let mut scheduler = Scheduler::open(&project).expect("open isolated Scheduler");
    scheduler.start().expect("start neutral Machine Class");
    let initial_report = scheduler.status().expect("report neutral initial state");
    let initial_status = initial_report.to_string();
    let initial_prompt = initial_report.phase_prompt().to_string();
    assert!(initial_status.contains("Phase: inspect"));
    assert!(initial_prompt.contains("Inspect the source material"));
    assert!(!initial_prompt.contains("P1"));
    assert!(!initial_prompt.contains("P5"));
    assert!(!initial_prompt.to_ascii_lowercase().contains("wishwillow"));

    let first = scheduler
        .step(StepRequest::new("the neutral class is complete"))
        .expect("first neutral transition returns an outcome");
    assert!(matches!(first, StepOutcome::Advanced { .. }));
    assert_eq!(
        scheduler.load_state().expect("load first state").phase,
        "compose"
    );

    let second = scheduler
        .step(StepRequest::new("the neutral class is complete"))
        .expect("second neutral transition returns an outcome");
    assert!(matches!(second, StepOutcome::Advanced { .. }));
    assert_eq!(
        scheduler.load_state().expect("load second state").phase,
        "publish"
    );

    let final_report = scheduler.status().expect("report neutral final state");
    let status = final_report.to_string();
    let prompt = final_report.phase_prompt().as_str();
    assert!(status.contains("Phase: publish"));
    assert!(prompt.contains("Publish the completed result"));
    assert!(!prompt.contains("P1"));
    assert!(!prompt.contains("P5"));
    assert!(!prompt.to_ascii_lowercase().contains("wishwillow"));

    // FDC-004: the State File resides in the started run's directory.
    let run_dir = fs::read_dir(project.join(".ratmac/runs"))
        .expect("list the runs roster")
        .map(|entry| entry.expect("roster entry is readable").path())
        .find(|path| path.is_dir())
        .expect("the started run appears on the roster");
    let state = fs::read(run_dir.join("state.toml")).expect("read generic State File");
    let log =
        fs::read_to_string(project.join(".ratmac/log.md")).expect("read generic Transition Log");
    assert!(!state.is_empty());
    assert!(log.contains("inspect"));
    assert!(log.contains("compose"));
    assert!(log.contains("publish"));
    assert!(!log.to_ascii_lowercase().contains("wishwillow"));
    let _ = fs::remove_dir_all(project);
}
