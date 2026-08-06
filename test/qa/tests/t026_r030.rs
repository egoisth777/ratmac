//! PT-026-01 / PT-026-02: print-first status and successful-step output.

use ratmac::cli::run_from;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const STATUS_RATMAC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/r030-status-print/.ratmac/ratmac.toml"
));
const STATUS_STATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/r030-status-print/.ratmac/state.toml"
));
const STATUS_LOG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/r030-status-print/.ratmac/log.md"
));
const STEP_RATMAC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/r030-step-print/.ratmac/ratmac.toml"
));
const STEP_STATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/r030-step-print/.ratmac/state.toml"
));
const STEP_LOG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/r030-step-print/.ratmac/log.md"
));

fn temporary_project(ratmac: &str, state: &str, log: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos();
    let project = std::env::temp_dir().join(format!("ratmac-t026-{unique}"));
    fs::create_dir(&project).expect("temporary project must be creatable");
    fs::create_dir(project.join(".ratmac")).expect("project .ratmac directory must be creatable");
    fs::create_dir(project.join("artifacts")).expect("artifact directory must be creatable");
    fs::write(project.join(".ratmac/ratmac.toml"), ratmac)
        .expect("fixture ratmac.toml must be writable");
    // FDC-004: the State File resides in the addressed run's directory.
    fs::create_dir_all(project.join(".ratmac/runs/run-001"))
        .expect("run directory must be creatable");
    fs::write(project.join(".ratmac/runs/run-001/state.toml"), state)
        .expect("fixture state.toml must be writable");
    fs::write(project.join(".ratmac/log.md"), log).expect("fixture log.md must be writable");
    fs::write(project.join("artifacts/proof.txt"), "proof\n")
        .expect("passing guard artifact must be writable");
    project
}

#[test]
fn status_prints_current_phase_prompt() {
    let project = temporary_project(STATUS_RATMAC, STATUS_STATE, STATUS_LOG);
    let engine = project.join(".ratmac");
    let before = [
        fs::read(engine.join("ratmac.toml")).unwrap(),
        fs::read(engine.join("runs/run-001/state.toml")).unwrap(),
        fs::read(engine.join("log.md")).unwrap(),
    ];
    let mut stdout = Vec::new();
    run_from(
        vec![
            "rtm".to_owned(),
            "status".to_owned(),
            "--run".to_owned(),
            "run-001".to_owned(),
        ],
        &project,
        &mut stdout,
    )
    .expect("rtm status must succeed for the valid fixture");
    let output = String::from_utf8(stdout).expect("status output must be UTF-8");

    assert!(
        output.contains("Prepare the run."),
        "status must print current prompt: {output}"
    );
    assert!(
        output.contains("files_exact"),
        "status must include generated guard list: {output}"
    );
    assert!(
        output.contains("artifacts"),
        "status must identify generated guard target: {output}"
    );
    assert!(
        !output.contains("Complete the run."),
        "status must not print another Phase: {output}"
    );
    assert_eq!(fs::read(engine.join("ratmac.toml")).unwrap(), before[0]);
    assert_eq!(
        fs::read(engine.join("runs/run-001/state.toml")).unwrap(),
        before[1]
    );
    assert_eq!(fs::read(engine.join("log.md")).unwrap(), before[2]);

    let _ = fs::remove_dir_all(project);
}

#[test]
fn successful_step_prints_prompt_without_spawning() {
    let project = temporary_project(STEP_RATMAC, STEP_STATE, STEP_LOG);
    let engine = project.join(".ratmac");
    let mut stdout = Vec::new();
    let args = vec![
        "rtm".to_owned(),
        "step".to_owned(),
        "--run".to_owned(),
        "run-001".to_owned(),
    ];
    run_from(args, &project, &mut stdout).expect("successful step must return normally");
    let output = String::from_utf8(stdout).expect("step output must be UTF-8");

    assert!(
        output.contains("Complete the run."),
        "successful step must print the current post-transition prompt: {output}"
    );
    assert!(
        output.contains("files_exact"),
        "step output must include generated guards: {output}"
    );
    assert!(
        !output.contains("--spawn"),
        "print-first output must not advertise a spawn path: {output}"
    );
    assert!(
        !engine.join("spawn-sentinel").exists(),
        "successful step must not create the spawn sentinel"
    );
    assert!(
        !project.join("spawn-sentinel").exists(),
        "successful step must not create a process marker"
    );

    let _ = fs::remove_dir_all(project);
}
