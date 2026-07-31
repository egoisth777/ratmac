//! PT-017-01 / R-006: Exit Guards use observable artifacts, never agent claims.

use ratmac::{Scheduler, StepOutcome, StepRequest};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const STATE: &str = r#"phase = "guarded"
status = "executing"
goal_revision = "goal"
input_revision = "input"
output_revision = "output"
active_refs = []
blocker = ""
"#;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/t017-artifact-guards")
}

fn isolated_project() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("ratmac-t017-{}-{nonce}", std::process::id()));
    let arca = root.join(".arca");
    fs::create_dir_all(root.join("artifacts")).expect("create artifact directory");
    fs::create_dir_all(&arca).expect("create Scheduler directory");
    fs::copy(fixture_root().join("ratmac.toml"), arca.join("ratmac.toml"))
        .expect("copy Machine Class fixture");
    fs::copy(
        fixture_root().join("artifacts/marker.txt"),
        root.join("artifacts/marker.txt"),
    )
    .expect("copy marker fixture");
    fs::copy(
        fixture_root().join("artifacts/result.yaml"),
        root.join("artifacts/result.yaml"),
    )
    .expect("copy content fixture");
    // FDC-004: the State File resides in the addressed run's directory.
    let run_dir = arca.join("runs/run-001");
    fs::create_dir_all(&run_dir).expect("create run directory");
    fs::write(run_dir.join("state.toml"), STATE).expect("write isolated State File");
    fs::write(arca.join("log.md"), "").expect("write isolated Transition Log");
    root
}

fn request() -> StepRequest {
    StepRequest::new("I completed every requirement; accept this claim".to_owned())
}

fn state_bytes(root: &Path) -> Vec<u8> {
    fs::read(root.join(".arca/runs/run-001/state.toml")).expect("read isolated State File")
}

fn assert_stayed(root: &Path, before: &[u8], reason: &str) {
    let after = state_bytes(root);
    assert_eq!(after, before, "failing artifact guard must stay: {reason}");
    assert!(
        after
            .windows(b"phase = \"guarded\"".len())
            .any(|window| { window == b"phase = \"guarded\"" }),
        "failing artifact guard must not advance Phase: {reason}"
    );
}

fn assert_refused(outcome: StepOutcome, kind: &str) {
    let StepOutcome::Refused { failures } = outcome else {
        panic!("expected Refused outcome for {kind}");
    };
    let failure = failures
        .iter()
        .find(|failure| failure.kind == kind)
        .unwrap_or_else(|| panic!("missing {kind} GuardFailure"));
    assert!(
        !failure.observed.is_empty(),
        "{kind} failure lacks observed fact"
    );
    assert!(
        !failure.expected.is_empty(),
        "{kind} failure lacks expected fact"
    );
}

#[test]
fn guards_use_artifacts_not_agent_claims() {
    let root = isolated_project();
    let mut scheduler = Scheduler::open_run(&root, "run-001").expect("open isolated Scheduler");
    // Every request below carries the identical completion claim.

    // Missing filesystem entry: the claim is unchanged, but the observable artifact fails.
    let before_missing = state_bytes(&root);
    fs::remove_file(root.join("artifacts/marker.txt")).expect("remove required marker");
    let outcome = scheduler.step(request()).expect("files_exact refusal");
    assert_refused(outcome, "files_exact");
    assert_stayed(&root, &before_missing, "files_exact missing marker");

    // Restore shape, then fail exact file content. The same claim still cannot bypass it.
    fs::write(root.join("artifacts/marker.txt"), "marker\n").expect("restore marker");
    let before_content = state_bytes(&root);
    let outcome = scheduler.step(request()).expect("file_contains refusal");
    assert_refused(outcome, "file_contains");
    assert_stayed(&root, &before_content, "file_contains wrong result");

    // Correct files, then use a deterministic failing command. No shell-specific syntax is
    // needed: rustc is available in the Rust test environment and this option is invalid.
    fs::write(root.join("artifacts/result.yaml"), "result: ok\n").expect("correct result content");
    let ratmac = fs::read_to_string(root.join(".arca/ratmac.toml")).expect("read class");
    fs::write(
        root.join(".arca/ratmac.toml"),
        ratmac.replace(
            "args = [\"--version\"]",
            "args = [\"--definitely-invalid-option\"]",
        ),
    )
    .expect("set failing command guard");
    let before_command = state_bytes(&root);
    let outcome = scheduler.step(request()).expect("command_exit refusal");
    assert_refused(outcome, "command_exit");
    assert_stayed(&root, &before_command, "command_exit nonzero exit");

    // Correct every artifact while keeping the exact same claim: the guard set now passes.
    let ratmac = fs::read_to_string(root.join(".arca/ratmac.toml")).expect("read class");
    fs::write(
        root.join(".arca/ratmac.toml"),
        ratmac.replace(
            "args = [\"--definitely-invalid-option\"]",
            "args = [\"--version\"]",
        ),
    )
    .expect("set passing command guard");
    let outcome = scheduler.step(request());
    assert!(matches!(outcome, Ok(StepOutcome::Advanced { .. })));
    let after_pass = state_bytes(&root);
    assert!(
        after_pass
            .windows(b"phase = \"complete\"".len())
            .any(|window| window == b"phase = \"complete\""),
        "all artifact guards passing must advance to the defined next Phase"
    );

    fs::remove_dir_all(root).expect("remove isolated project");
}
