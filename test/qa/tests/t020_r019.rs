//! T-03 / R-019: failed guards produce actionable refusal evidence.

use ratmac::{Scheduler, StepOutcome, StepRequest};
use ratmac_qa::support::TempTree;
use std::fs;

const RATMAC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/t020-refusal-report/.ratmac/ratmac.toml"
));
const OBSERVED_ARTIFACT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/t020-refusal-report/artifacts/status.txt"
));

fn temporary_project() -> TempTree {
    let project = TempTree::new("t020").unwrap();
    fs::create_dir(project.join(".ratmac")).unwrap();
    fs::create_dir(project.join("artifacts")).unwrap();
    fs::write(project.join(".ratmac/ratmac.toml"), RATMAC).unwrap();
    fs::write(project.join("artifacts/status.txt"), OBSERVED_ARTIFACT).unwrap();
    project
}

#[test]
fn refusal_report_names_guard_and_facts() {
    let tree = temporary_project();
    let project = tree.path().to_path_buf();
    let mut scheduler = Scheduler::open(&project).expect("scheduler must open fixture project");
    scheduler.start().expect("fixture Run must start");
    let outcome = scheduler
        .step(StepRequest::new("I completed the state"))
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
}
