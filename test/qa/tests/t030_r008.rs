use arca_scheduler::cli;
use std::fs;
use std::path::{Path, PathBuf};
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
        "arca-scheduler-t030-{}-{stamp}",
        std::process::id()
    ));
    let arca = root.join(".arca");
    fs::create_dir_all(&arca).expect("create isolated step-help project");
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/r008-step-policy/ratmac.toml");
    fs::copy(fixture, arca.join("ratmac.toml")).expect("copy step-policy fixture");
    Project { root }
}

#[test]
fn step_help_documents_main_agent_only_policy() {
    let project = setup_project();
    let class_path = project.root.join(".arca/ratmac.toml");
    let before = fs::read(&class_path).expect("read class before help");
    let mut output = Vec::new();

    cli::run_from(["schd", "step", "--help"], &project.root, &mut output)
        .expect("step help is a successful CLI workflow");
    let help = String::from_utf8(output)
        .expect("help output is UTF-8")
        .to_ascii_lowercase();

    assert!(
        help.contains("main-agent") || help.contains("main agent"),
        "step help must identify the Main-Agent invoker: {help}"
    );
    assert!(
        help.contains("human"),
        "step help must identify the human invoker: {help}"
    );
    assert!(
        help.contains("subagent")
            && (help.contains("read state") || help.contains("read the state")),
        "step help must say Subagents only read state: {help}"
    );
    assert!(
        !help.contains("authentication")
            && !help.contains("authenticated")
            && !help.contains("caller identity"),
        "step help must document policy without claiming caller authentication: {help}"
    );
    assert_eq!(
        before,
        fs::read(&class_path).expect("read class after help"),
        "help must not rewrite the Machine Class"
    );
    assert!(
        !project.root.join(".arca/state.toml").exists(),
        "help must not instantiate or mutate Run state"
    );
}
