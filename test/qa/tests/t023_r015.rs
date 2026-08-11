use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;

use ratmac::{Scheduler, StepOutcome, StepRequest};

fn fixture_root() -> PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/r015-concurrent-run");
    let root = std::env::temp_dir().join(format!("ratmac-r015-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".ratmac")).expect("create isolated project root");
    for name in ["ratmac.toml", "log.md"] {
        fs::copy(
            fixture.join(".ratmac").join(name),
            root.join(".ratmac").join(name),
        )
        .expect("copy concurrent-run fixture");
    }
    // FDC-004: the State File resides in the addressed run's directory.
    let run_dir = root.join(".ratmac/runs/run-001");
    fs::create_dir_all(&run_dir).expect("create run directory");
    fs::copy(fixture.join(".ratmac/run.toml"), run_dir.join("run.toml"))
        .expect("copy concurrent-run fixture");
    root
}

#[test]
fn concurrent_steps_are_arbitrated_by_lockfile() {
    let root = fixture_root();
    // ENS-005: same-Run motion serializes on its addressed lock, not root.
    let engine_root = root.join(".ratmac");
    let lock_path = ratmac::lock::run_path(&engine_root, "run-001");
    let root_lock_path = ratmac::lock::root_path(&engine_root);
    assert!(!lock_path.exists(), "Run lock starts transient and absent");

    let barrier = Arc::new(Barrier::new(2));
    let root_a = root.clone();
    let barrier_a = Arc::clone(&barrier);
    let first = thread::spawn(move || {
        let mut scheduler = Scheduler::open_run(&root_a, "run-001").expect("open first scheduler");
        barrier_a.wait();
        scheduler
            .step(StepRequest::new("complete prepare"))
            .map_err(|error| error.to_string())
    });
    let root_b = root.clone();
    let barrier_b = Arc::clone(&barrier);
    let second = thread::spawn(move || {
        let mut scheduler = Scheduler::open_run(&root_b, "run-001").expect("open second scheduler");
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
        "ENS-005 Run lock is removed after both same-Run motions"
    );
    assert!(
        !root_lock_path.exists(),
        "ENS-005 same-Run motion leaves no root lock behind"
    );

    let state: toml::Value = fs::read_to_string(root.join(".ratmac/runs/run-001/run.toml"))
        .expect("state remains readable")
        .parse()
        .expect("state remains parseable");
    assert_eq!(
        state.get("state").and_then(toml::Value::as_str),
        Some("review")
    );
    let log = fs::read_to_string(root.join(".ratmac/log.md")).expect("log remains readable");
    assert_eq!(log.lines().filter(|line| !line.is_empty()).count(), 1);
    let _ = fs::remove_dir_all(root);
}
