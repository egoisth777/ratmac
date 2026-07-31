use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ratmac::Scheduler;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/t015-corrupt-state")
}

fn copy_fixture_to_temp() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is after the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("ratmac-t015-{nonce}"));
    let arca = root.join(".arca");
    fs::create_dir_all(&arca).expect("create isolated .arca directory");
    for file in ["ratmac.toml", "log.md"] {
        fs::copy(fixture_root().join(".arca").join(file), arca.join(file))
            .expect("copy corrupt-state fixture");
    }
    // FDC-004: the State File resides in the addressed run's directory.
    let run_dir = arca.join("runs/run-001");
    fs::create_dir_all(&run_dir).expect("create run directory");
    fs::copy(
        fixture_root().join(".arca/state.toml"),
        run_dir.join("state.toml"),
    )
    .expect("copy corrupt-state fixture");
    root
}

fn bytes(path: &Path) -> Vec<u8> {
    fs::read(path).expect("read fixture artifact")
}

fn assert_actionable_parse_error(error: impl std::fmt::Display) {
    let report = error.to_string();
    assert!(
        !report.trim().is_empty(),
        "corrupt state must produce a report"
    );
    assert!(
        report.contains("state.toml"),
        "report should identify the corrupt state file: {report}"
    );
    assert!(
        report.contains("line")
            || report.contains("column")
            || report.contains("expected")
            || report.contains("invalid"),
        "report should explain the parse failure: {report}"
    );
}

#[test]
fn corrupt_state_halts_without_guessing() {
    let root = copy_fixture_to_temp();
    let state_path = root.join(".arca/runs/run-001/state.toml");
    let class_path = root.join(".arca/ratmac.toml");
    let log_path = root.join(".arca/log.md");
    let before_state = bytes(&state_path);
    let before_class = bytes(&class_path);
    let before_log = bytes(&log_path);

    // Opening a project reads the class only; each command must hard-fail when
    // it reaches the malformed State File rather than guessing a state.
    let status = Scheduler::open_run(&root, "run-001").expect("valid class should open");
    let status_error = status
        .status()
        .expect_err("status must reject malformed state.toml");
    assert_actionable_parse_error(status_error);

    let reader = Scheduler::open_run(&root, "run-001").expect("valid class should open");
    let load_error = reader
        .load_state()
        .expect_err("load_state must reject malformed state.toml");
    assert_actionable_parse_error(load_error);

    assert_eq!(
        bytes(&state_path),
        before_state,
        "state must not be repaired"
    );
    assert_eq!(
        bytes(&class_path),
        before_class,
        "class must remain read-only"
    );
    assert_eq!(
        bytes(&log_path),
        before_log,
        "failed read must not append a transition"
    );

    fs::remove_dir_all(root).expect("clean isolated fixture directory");
}
