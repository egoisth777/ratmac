use ratmac::cli;
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
    // A process-wide counter keeps parallel tests apart even when the clock
    // hands two threads the same nanosecond stamp.
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ratmac-t029-{}-{unique}-{stamp}",
        std::process::id()
    ));
    let engine = root.join(".ratmac");
    fs::create_dir_all(&engine).expect("create isolated help project");
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/r007-start-policy/ratmac.toml");
    fs::copy(fixture, engine.join("ratmac.toml")).expect("copy start-policy fixture");
    Project { root }
}

fn run_rtm(project: &Project, args: &[&str]) -> Output {
    Command::new(ratmac_qa::engine_bin!())
        .args(args)
        .current_dir(&project.root)
        .output()
        .expect("invoke built rtm binary")
}

/// FDC-004: the run ids read off the plural roster.
fn roster(project: &Project) -> Vec<String> {
    let runs = project.root.join(".ratmac/runs");
    let mut ids: Vec<String> = fs::read_dir(&runs)
        .expect("list the runs roster")
        .map(|entry| entry.expect("roster entry is readable"))
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    ids.sort();
    ids
}

fn run_state_path(project: &Project, id: &str) -> PathBuf {
    project.root.join(".ratmac/runs").join(id).join("run.toml")
}

#[test]
fn start_help_states_the_one_caller_policy() {
    // ORS-001 supersedes R-007's user-only wording: help states who may
    // invoke start, under what sign-off, and who never may - without
    // claiming the Engine authenticates anyone.
    let project = setup_project();
    let class_path = project.root.join(".ratmac/ratmac.toml");
    let before = fs::read(&class_path).expect("read class before help");
    let mut output = Vec::new();
    cli::run_from(["rtm", "start", "--help"], &project.root, &mut output)
        .expect("start help is a successful CLI workflow");
    let help = String::from_utf8(output)
        .expect("help output is UTF-8")
        .to_ascii_lowercase();
    assert!(
        help.contains("human may invoke") && help.contains("rtm start"),
        "start help must say a human may invoke rtm start: {help}"
    );
    assert!(
        help.contains("main-agent") && help.contains("sign-off"),
        "start help must gate Main-Agent entry on explicit human sign-off: {help}"
    );
    assert!(
        help.contains("subagent never invokes"),
        "start help must say a Subagent never invokes rtm: {help}"
    );
    for retired in ["user-only", "user only", "never agent-initiated"] {
        assert!(
            !help.contains(retired),
            "start help must not retain retired wording {retired:?}: {help}"
        );
    }
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
        !project.root.join(".ratmac/state.toml").exists()
            && !project.root.join(".ratmac/runs").exists(),
        "help must not instantiate a Run"
    );
}

#[test]
fn binary_start_creates_owned_artifacts_and_releases_lock() {
    let project = setup_project();
    let class_path = project.root.join(".ratmac/ratmac.toml");
    let before = fs::read(&class_path).expect("read class before start");
    let output = run_rtm(&project, &["start"]);
    assert!(
        output.status.success(),
        "start failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // FDC-004: start mints a run whose State File resides in its directory;
    // the flat path is never written.
    let ids = roster(&project);
    assert_eq!(ids.len(), 1, "start must mint exactly one run");
    assert!(run_state_path(&project, &ids[0]).is_file());
    assert!(!project.root.join(".ratmac/state.toml").exists());
    assert!(project.root.join(".ratmac/log.md").is_file());
    assert!(!project.root.join(".ratmac/locks/root.lock").exists());
    assert_eq!(
        before,
        fs::read(class_path).expect("read class after start")
    );
}

/// FDC-006 (supersedes R-022/T-08): no active-Run cap — a second start
/// succeeds, mints exactly one distinct sibling, and mutates nothing that
/// already exists.
#[test]
fn binary_second_start_mints_sibling_without_mutating_run() {
    let project = setup_project();
    let first = run_rtm(&project, &["start"]);
    assert!(first.status.success());
    let ids = roster(&project);
    let state_path = run_state_path(&project, &ids[0]);
    let state = fs::read(&state_path).expect("read initial state");
    let log = fs::read(project.root.join(".ratmac/log.md")).expect("read initial log");
    let class = fs::read(project.root.join(".ratmac/ratmac.toml")).expect("read initial class");

    let second = run_rtm(&project, &["start"]);
    assert!(
        second.status.success(),
        "a second start succeeds — FDC-006 enforces no active-Run cap"
    );
    let after = roster(&project);
    assert_eq!(
        after.len(),
        ids.len() + 1,
        "the second start mints exactly one new run"
    );
    assert!(
        after.iter().filter(|id| !ids.contains(id)).count() == 1,
        "the minted sibling carries a fresh id: {after:?}"
    );
    assert_eq!(
        state,
        fs::read(&state_path).unwrap(),
        "the sibling start leaves the first run byte-identical"
    );
    assert_eq!(log, fs::read(project.root.join(".ratmac/log.md")).unwrap());
    assert_eq!(
        class,
        fs::read(project.root.join(".ratmac/ratmac.toml")).unwrap()
    );
}

#[test]
fn binary_status_prints_report_and_state_prompt() {
    let project = setup_project();
    assert!(run_rtm(&project, &["start"]).status.success());
    // FDC-004: run addressing is always required.
    let ids = roster(&project);
    let output = run_rtm(&project, &["status", "--run", &ids[0]]);
    assert!(
        output.status.success(),
        "status failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for field in [
        "State: prepare",
        "Status: planned",
        "Blocker:",
        "pending guard: files_exact",
    ] {
        assert!(stdout.contains(field), "status missing {field}: {stdout}");
    }
    assert!(
        stdout.contains("Prepare the run."),
        "status missing State Prompt: {stdout}"
    );
}
