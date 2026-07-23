use arca_scheduler::cli;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

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
        "arca-scheduler-t029-{}-{stamp}",
        std::process::id()
    ));
    let arca = root.join(".arca");
    fs::create_dir_all(&arca).expect("create isolated help project");
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/r007-start-policy/ratmac.toml");
    fs::copy(fixture, arca.join("ratmac.toml")).expect("copy start-policy fixture");
    Project { root }
}

fn run_schd(project: &Project, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_schd"))
        .args(args)
        .current_dir(&project.root)
        .output()
        .expect("invoke built schd binary")
}

#[test]
fn start_help_documents_user_only_loop_entry() {
    let project = setup_project();
    let class_path = project.root.join(".arca/ratmac.toml");
    let before = fs::read(&class_path).expect("read class before help");
    let mut output = Vec::new();
    cli::run_from(["schd", "start", "--help"], &project.root, &mut output)
        .expect("start help is a successful CLI workflow");
    let help = String::from_utf8(output)
        .expect("help output is UTF-8")
        .to_ascii_lowercase();
    assert!(
        help.contains("user-only") || help.contains("user only"),
        "start help must explicitly say that start is user-only: {help}"
    );
    assert!(
        help.contains("not agent-initiated")
            || help.contains("never agent-initiated")
            || help.contains("not agent initiated")
            || help.contains("never agent initiated")
            || help.contains("agents must not start"),
        "start help must say loop entry is not agent-initiated: {help}"
    );
    assert!(
        !help.contains("authentication")
            && !help.contains("authenticated")
            && !help.contains("caller identity"),
        "start help must document policy without claiming caller authentication: {help}"
    );
    assert_eq!(
        before,
        fs::read(&class_path).expect("read class after help"),
        "help must not rewrite the Machine Class"
    );
    assert!(
        !project.root.join(".arca/state.toml").exists(),
        "help must not instantiate a Run"
    );
}

#[test]
fn binary_start_creates_owned_artifacts_and_releases_lock() {
    let project = setup_project();
    let class_path = project.root.join(".arca/ratmac.toml");
    let before = fs::read(&class_path).expect("read class before start");
    let output = run_schd(&project, &["start"]);
    assert!(
        output.status.success(),
        "start failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(project.root.join(".arca/state.toml").is_file());
    assert!(project.root.join(".arca/log.md").is_file());
    assert!(!project.root.join(".arca/schd.lock").exists());
    assert_eq!(
        before,
        fs::read(class_path).expect("read class after start")
    );
}

#[test]
fn binary_second_start_fails_without_mutating_run() {
    let project = setup_project();
    let first = run_schd(&project, &["start"]);
    assert!(first.status.success());
    let state = fs::read(project.root.join(".arca/state.toml")).expect("read initial state");
    let log = fs::read(project.root.join(".arca/log.md")).expect("read initial log");
    let class = fs::read(project.root.join(".arca/ratmac.toml")).expect("read initial class");

    let second = run_schd(&project, &["start"]);
    assert!(!second.status.success(), "second active start must fail");
    assert_eq!(
        state,
        fs::read(project.root.join(".arca/state.toml")).unwrap()
    );
    assert_eq!(log, fs::read(project.root.join(".arca/log.md")).unwrap());
    assert_eq!(
        class,
        fs::read(project.root.join(".arca/ratmac.toml")).unwrap()
    );
}

#[test]
fn binary_status_prints_report_and_phase_prompt() {
    let project = setup_project();
    assert!(run_schd(&project, &["start"]).status.success());
    let output = run_schd(&project, &["status"]);
    assert!(
        output.status.success(),
        "status failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for field in [
        "Phase: prepare",
        "Status: planned",
        "Blocker:",
        "pending guard: files_exact",
    ] {
        assert!(stdout.contains(field), "status missing {field}: {stdout}");
    }
    assert!(
        stdout.contains("Prepare the run."),
        "status missing Phase Prompt: {stdout}"
    );
}
