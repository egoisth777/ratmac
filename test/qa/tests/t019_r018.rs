use arca_scheduler::{Scheduler, StepOutcome, StepRequest};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

type StatusAndBlocker = (String, Option<String>);

struct Project {
    root: PathBuf,
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn setup_project() -> Project {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "arca-scheduler-t019-{}-{stamp}",
        std::process::id()
    ));
    let arca = root.join(".arca");
    fs::create_dir_all(&arca).expect("create isolated guard-failure project");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/t019-guard-failure-status/ratmac.toml");
    fs::copy(fixture, arca.join("ratmac.toml")).expect("copy guard-failure fixture");
    Project { root }
}

fn read_status_and_blocker(project: &Project) -> StatusAndBlocker {
    let state: toml::Value = fs::read_to_string(project.root.join(".arca/state.toml"))
        .expect("read scheduler state")
        .parse()
        .expect("state file is valid TOML");
    let table = state.as_table().expect("state file is a TOML table");
    let status = table
        .get("status")
        .and_then(toml::Value::as_str)
        .expect("state has a status")
        .to_owned();
    let blocker = table
        .get("blocker")
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .filter(|value| !value.is_empty());
    (status, blocker)
}

#[test]
fn guard_failure_never_sets_blocked() {
    let project = setup_project();
    let mut scheduler = Scheduler::open(&project.root).expect("open project");
    scheduler.start().expect("start Run");
    let before = read_status_and_blocker(&project);
    assert_ne!(
        before.0, "blocked",
        "guard test starts from a non-blocked status"
    );

    let outcome = scheduler
        .step(StepRequest::new("claim-complete"))
        .expect("failed guard yields a refusal outcome");
    assert!(
        matches!(outcome, StepOutcome::Refused { .. }),
        "missing required-output guard must refuse step"
    );
    let _reloaded = scheduler.load_state().expect("reload state after refusal");

    let after = read_status_and_blocker(&project);
    assert_eq!(
        after.0, before.0,
        "guard refusal preserves the existing status"
    );
    assert_ne!(after.0, "blocked", "guard failure must never enter blocked");
    assert!(
        after.1.is_none(),
        "guard failure must not create an entry-prerequisite blocker"
    );
}
