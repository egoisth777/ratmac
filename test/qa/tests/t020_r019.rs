//! T-03 / R-019: failed guards produce actionable refusal evidence.

use ratmac::{Scheduler, StepOutcome, StepRequest};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const RATMAC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/t020-refusal-report/.arca/ratmac.toml"
));
const OBSERVED_ARTIFACT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/t020-refusal-report/artifacts/status.txt"
));

fn temporary_project() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let project = std::env::temp_dir().join(format!("ratmac-t020-{unique}"));
    fs::create_dir(&project).unwrap();
    fs::create_dir(project.join(".arca")).unwrap();
    fs::create_dir(project.join("artifacts")).unwrap();
    fs::write(project.join(".arca/ratmac.toml"), RATMAC).unwrap();
    fs::write(project.join("artifacts/status.txt"), OBSERVED_ARTIFACT).unwrap();
    project
}

#[test]
fn refusal_report_names_guard_and_facts() {
    let project = temporary_project();
    let mut scheduler = Scheduler::open(&project).expect("scheduler must open fixture project");
    scheduler.start().expect("fixture Run must start");
    let outcome = scheduler
        .step(StepRequest::new("I completed the phase"))
        .expect("guard failure is a reported StepOutcome");
    let failure = match outcome {
        StepOutcome::Refused { failures } => {
            assert_eq!(failures.len(), 1);
            failures.into_iter().next().unwrap()
        }
        StepOutcome::Advanced { .. } => panic!("NOT_READY must fail file_contains"),
    };
    let rendered = failure.to_string();
    assert!(failure.name().contains("file_contains"));
    assert!(failure.name().contains("artifacts/status.txt"));
    assert!(failure.observed().contains("NOT_READY"));
    assert!(failure.expected().contains("READY"));
    assert!(rendered.contains(failure.name()));
    assert!(rendered.contains(failure.observed()));
    assert!(rendered.contains(failure.expected()));
    let _ = fs::remove_dir_all(project);
}
