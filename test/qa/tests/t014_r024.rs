//! PT-014-01 / R-024: Scheduler-owned Run files stay flat under `.arca`.

use arca_scheduler::Scheduler;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const RATMAC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/t014-flat-arca/.arca/ratmac.toml"
));

const CURRENT_SENTINEL: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/t014-flat-arca/.arca/current/sentinel.txt"
));

fn temporary_project() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos();
    let project = std::env::temp_dir().join(format!("arca-scheduler-t014-{unique}"));
    fs::create_dir(&project).expect("temporary project must be creatable");
    fs::create_dir_all(project.join(".arca/current"))
        .expect("legacy current directory must be creatable");
    fs::write(project.join(".arca/current/sentinel.txt"), CURRENT_SENTINEL)
        .expect("legacy current sentinel must be writable");
    fs::write(project.join(".arca/ratmac.toml"), RATMAC)
        .expect("fixture ratmac.toml must be writable");
    project
}

#[test]
fn scheduler_files_are_flat_under_arca() {
    let project = temporary_project();
    let arca = project.join(".arca");
    let ratmac = arca.join("ratmac.toml");
    let sentinel = arca.join("current/sentinel.txt");
    let ratmac_before = fs::read(&ratmac).expect("class bytes must be readable before start");
    let sentinel_before = fs::read(&sentinel).expect("legacy current sentinel must be readable");

    let run = Scheduler::open(&project)
        .expect("scheduler must open the fixture project")
        .start()
        .expect("Run setup must create scheduler-owned artifacts");
    let artifacts = run
        .artifacts()
        .expect("successful start must return RunArtifacts");

    assert_eq!(artifacts.state_path(), arca.join("state.toml").as_path());
    assert_eq!(artifacts.log_path(), arca.join("log.md").as_path());
    assert_eq!(artifacts.lock_path(), arca.join("schd.lock").as_path());
    assert_eq!(artifacts.state_path().parent(), Some(arca.as_path()));
    assert_eq!(artifacts.log_path().parent(), Some(arca.as_path()));
    assert_eq!(artifacts.lock_path().parent(), Some(arca.as_path()));
    assert!(
        artifacts.state_path().is_file(),
        "state.toml must be flat under .arca"
    );
    assert!(
        artifacts.log_path().is_file(),
        "log.md must be flat under .arca"
    );
    assert!(
        !artifacts.lock_path().exists(),
        "schd.lock is transient and must be absent after start returns"
    );
    assert_eq!(
        fs::read(&ratmac).expect("class bytes must be readable after start"),
        ratmac_before,
        "start must not rewrite ratmac.toml"
    );
    assert_eq!(
        fs::read(&sentinel).expect("legacy current sentinel must remain"),
        sentinel_before,
        "legacy .arca/current content must remain untouched"
    );
    assert!(
        !artifacts.state_path().starts_with(arca.join("current"))
            && !artifacts.log_path().starts_with(arca.join("current"))
            && !artifacts.lock_path().starts_with(arca.join("current")),
        "scheduler-owned artifacts must not be nested below .arca/current"
    );

    let _ = fs::remove_dir_all(project);
}
