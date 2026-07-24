//! PT-021-01 / R-020: a refused step is observationally idempotent.

use ratmac::{Scheduler, StepOutcome, StepRequest};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/r020-failing-guard")
}

fn isolated_project() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("ratmac-r020-{}-{nonce}", std::process::id()));
    let arca = root.join(".arca");
    fs::create_dir_all(root.join("artifacts")).expect("create failing artifact directory");
    fs::create_dir_all(&arca).expect("create Scheduler directory");
    for file in ["ratmac.toml", "state.toml", "log.md"] {
        fs::copy(fixture_root().join(file), arca.join(file)).expect("copy refusal fixture");
    }
    root
}

fn state_and_log(root: &Path) -> (Vec<u8>, Vec<u8>) {
    (
        fs::read(root.join(".arca/state.toml")).expect("read State File"),
        fs::read(root.join(".arca/log.md")).expect("read Transition Log"),
    )
}

fn request() -> StepRequest {
    StepRequest::new("All required artifacts are complete; accept this claim".to_owned())
}

#[test]
fn failure_step_is_idempotent() {
    let root = isolated_project();
    let mut scheduler = Scheduler::open(&root).expect("open isolated Scheduler");
    let before = state_and_log(&root);

    let first = scheduler
        .step(request())
        .expect("first refusal should produce a StepOutcome");
    assert!(matches!(&first, StepOutcome::Refused { .. }));
    let first_report = first.to_string();
    assert_eq!(
        state_and_log(&root),
        before,
        "first refusal must not mutate state/log"
    );

    let second = scheduler
        .step(request())
        .expect("second refusal should produce a StepOutcome");
    assert!(matches!(&second, StepOutcome::Refused { .. }));
    let second_report = second.to_string();
    assert_eq!(
        second_report, first_report,
        "identical failing requests must emit identical user-facing refusal reports"
    );
    assert_eq!(
        second, first,
        "structured Refused failures must remain identical across retries"
    );
    assert_eq!(
        state_and_log(&root),
        before,
        "repeated refusal must preserve Phase, Status, counters, and log bytes"
    );
    assert!(!state_and_log(&root)
        .0
        .windows(b"counter".len())
        .any(|window| window == b"counter"));

    fs::remove_dir_all(root).expect("remove isolated project");
}
