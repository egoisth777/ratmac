//! PT-014-01 / R-024: Scheduler-owned Run files stay under `.ratmac`.
//!
//! FDC-004 supersedes the flat State File: it resides in the minted run's
//! `.ratmac/runs/<id>/` directory. The preserved intent: every scheduler-owned
//! artifact lives under `.ratmac`, never below `.arca/goal`, and start touches
//! nothing human-authored.

use ratmac::Scheduler;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const RATMAC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/t014-flat-arca/.ratmac/ratmac.toml"
));

const GOAL_SENTINEL: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/t014-flat-arca/.arca/goal/sentinel.txt"
));

fn temporary_project() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos();
    let project = std::env::temp_dir().join(format!("ratmac-t014-{unique}"));
    fs::create_dir(&project).expect("temporary project must be creatable");
    fs::create_dir_all(project.join(".arca/goal")).expect("human goal directory must be creatable");
    fs::create_dir_all(project.join(".ratmac")).expect("Engine directory must be creatable");
    fs::write(project.join(".arca/goal/sentinel.txt"), GOAL_SENTINEL)
        .expect("human goal sentinel must be writable");
    fs::write(project.join(".ratmac/ratmac.toml"), RATMAC)
        .expect("fixture ratmac.toml must be writable");
    project
}

#[test]
fn scheduler_files_are_under_ratmac() {
    let project = temporary_project();
    let engine = project.join(".ratmac");
    let ratmac = engine.join("ratmac.toml");
    let sentinel = project.join(".arca/goal/sentinel.txt");
    let ratmac_before = fs::read(&ratmac).expect("class bytes must be readable before start");
    let sentinel_before = fs::read(&sentinel).expect("human goal sentinel must be readable");

    let run = Scheduler::open(&project)
        .expect("scheduler must open the fixture project")
        .start()
        .expect("Run setup must create scheduler-owned artifacts");
    let artifacts = run
        .artifacts()
        .expect("successful start must return RunArtifacts");

    let run_id = run.id().expect("start must mint a run id");
    let run_dir = engine.join("runs").join(run_id);
    let run_lock = engine.join("locks/runs").join(format!("{run_id}.lock"));
    assert_eq!(artifacts.state_path(), run_dir.join("state.toml").as_path());
    assert_eq!(artifacts.log_path(), engine.join("log.md").as_path());
    assert_eq!(artifacts.lock_path(), run_lock.as_path());
    assert_eq!(artifacts.state_path().parent(), Some(run_dir.as_path()));
    assert_eq!(artifacts.log_path().parent(), Some(engine.as_path()));
    assert_eq!(
        artifacts.lock_path().parent(),
        Some(engine.join("locks/runs").as_path())
    );
    assert!(
        artifacts.state_path().is_file(),
        "state.toml must reside in the run's directory under .ratmac/runs/"
    );
    assert!(
        !engine.join("state.toml").exists(),
        "no flat .ratmac/state.toml may be written"
    );
    assert!(
        artifacts.log_path().is_file(),
        "log.md must be flat under .ratmac"
    );
    assert!(
        !artifacts.lock_path().exists(),
        "the Run-scoped lock is transient and absent after start returns"
    );
    assert_eq!(
        fs::read(&ratmac).expect("class bytes must be readable after start"),
        ratmac_before,
        "start must not rewrite ratmac.toml"
    );
    assert_eq!(
        fs::read(&sentinel).expect("human goal sentinel must remain"),
        sentinel_before,
        "human .arca/goal content must remain untouched"
    );
    assert!(
        !artifacts
            .state_path()
            .starts_with(project.join(".arca/goal"))
            && !artifacts.log_path().starts_with(project.join(".arca/goal"))
            && !artifacts
                .lock_path()
                .starts_with(project.join(".arca/goal")),
        "scheduler-owned artifacts must not be nested below .arca/goal"
    );

    let _ = fs::remove_dir_all(project);
}
