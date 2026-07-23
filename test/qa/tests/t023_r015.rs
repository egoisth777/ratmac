use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;

use arca_scheduler::{Scheduler, StepOutcome, StepRequest};

fn fixture_root() -> PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/r015-concurrent-run");
    let root = std::env::temp_dir().join(format!("arca-scheduler-r015-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".arca")).expect("create isolated project root");
    for name in ["ratmac.toml", "state.toml", "log.md"] {
        fs::copy(
            fixture.join(".arca").join(name),
            root.join(".arca").join(name),
        )
        .expect("copy concurrent-run fixture");
    }
    root
}

#[test]
fn concurrent_steps_are_arbitrated_by_lockfile() {
    let root = fixture_root();
    let lock_path = root.join(".arca/schd.lock");
    assert!(!lock_path.exists(), "lock starts transient and absent");

    let barrier = Arc::new(Barrier::new(2));
    let root_a = root.clone();
    let barrier_a = Arc::clone(&barrier);
    let first = thread::spawn(move || {
        let mut scheduler = Scheduler::open(&root_a).expect("open first scheduler");
        barrier_a.wait();
        scheduler
            .step(StepRequest::new("complete prepare"))
            .map_err(|error| error.to_string())
    });
    let root_b = root.clone();
    let barrier_b = Arc::clone(&barrier);
    let second = thread::spawn(move || {
        let mut scheduler = Scheduler::open(&root_b).expect("open second scheduler");
        barrier_b.wait();
        scheduler
            .step(StepRequest::new("complete prepare"))
            .map_err(|error| error.to_string())
    });

    let outcomes = [
        first.join().expect("first invocation did not panic"),
        second.join().expect("second invocation did not panic"),
    ];
    let advanced = outcomes
        .iter()
        .filter(|outcome| matches!(outcome, Ok(StepOutcome::Advanced { .. })))
        .count();
    assert_eq!(
        advanced, 1,
        "exactly one invocation may advance the shared Run"
    );
    assert!(
        !lock_path.exists(),
        "transient lock is removed after both invocations"
    );

    let state: toml::Value = fs::read_to_string(root.join(".arca/state.toml"))
        .expect("state remains readable")
        .parse()
        .expect("state remains parseable");
    assert_eq!(
        state.get("phase").and_then(toml::Value::as_str),
        Some("review")
    );
    let log = fs::read_to_string(root.join(".arca/log.md")).expect("log remains readable");
    assert_eq!(log.lines().filter(|line| !line.is_empty()).count(), 1);
    let _ = fs::remove_dir_all(root);
}
