use std::fs;
use std::path::PathBuf;

use ratmac::{machine::MachineClass, Scheduler};

const RATMAC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/r028-phase-prompt/ratmac.toml"
));
const STATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/r028-phase-prompt/.arca/state.toml"
));

fn fixture_project() -> PathBuf {
    let root = std::env::temp_dir().join(format!("ratmac-r028-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove stale R-028 fixture project");
    }
    fs::create_dir_all(root.join(".arca")).expect("create R-028 fixture project");
    fs::write(root.join(".arca/ratmac.toml"), RATMAC).expect("write R-028 class");
    // FDC-004: the State File resides in the addressed run's directory.
    fs::create_dir_all(root.join(".arca/runs/run-001")).expect("create run directory");
    fs::write(root.join(".arca/runs/run-001/state.toml"), STATE).expect("write R-028 state");
    root
}

#[test]
fn phase_prompt_renders_inline_prose_then_generated_guards() {
    let project = fixture_project();
    let scheduler = Scheduler::open_run(&project, "run-001").expect("open R-028 fixture project");
    let status = scheduler.status().expect("read current status");
    let prompt = status.phase_prompt();
    let rendered = prompt.as_str();

    assert!(
        rendered.starts_with("Prepare the release artifact."),
        "prompt must begin with the selected phase's inline prose: {rendered}"
    );
    let prose_end = rendered
        .find("Prepare the release artifact.")
        .expect("inline prose is present")
        + "Prepare the release artifact.".len();
    let files = rendered
        .find("files_exact")
        .expect("files guard is rendered");
    let contains = rendered
        .find("file_contains")
        .expect("file-content guard is rendered");
    let command = rendered
        .find("command_exit")
        .expect("command guard is rendered");
    assert!(
        prose_end <= files && files < contains && contains < command,
        "inline prose must precede the complete ordered guard list: {rendered}"
    );
    assert!(rendered.contains("artifacts"));
    assert!(rendered.contains("artifacts/release.txt"));
    assert!(rendered.contains("ready: true"));
    assert!(rendered.contains("rustc"));
    assert_eq!(prompt.to_string(), rendered);

    fs::remove_dir_all(project).expect("clean up R-028 fixture project");
}

#[test]
fn phase_prompt_requires_a_string_prompt_field() {
    for source in ["[phases.prepare]\n", "[phases.prepare]\nprompt = 42\n"] {
        let error = MachineClass::from_toml(source)
            .expect_err("a Phase without a string prompt must be rejected");
        let message = error.to_string();
        assert!(
            message.contains("prompt"),
            "prompt parse errors must identify the required field: {message}"
        );
    }
}
