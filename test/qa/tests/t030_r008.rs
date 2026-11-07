use ratmac::cli;
use ratmac_qa::support::TempTree;
use std::fs;
use std::path::{Path, PathBuf};

struct Project {
    _tree: TempTree,
    root: PathBuf,
}

fn setup_project() -> Project {
    let tree = TempTree::new("t030").expect("create isolated step-help project");
    let root = tree.path().to_path_buf();
    let engine = root.join(".ratmac");
    fs::create_dir_all(&engine).expect("create isolated step-help project");
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/r008-step-policy/ratmac.toml");
    fs::copy(fixture, engine.join("ratmac.toml")).expect("copy step-policy fixture");
    Project { _tree: tree, root }
}

#[test]
fn step_help_documents_main_agent_only_policy() {
    let project = setup_project();
    let class_path = project.root.join(".ratmac/ratmac.toml");
    let before = fs::read(&class_path).expect("read class before help");
    let mut output = Vec::new();

    cli::run_from(["rtm", "step", "--help"], &project.root, &mut output)
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
        !project.root.join(".ratmac/state.toml").exists(),
        "help must not instantiate or mutate Run state"
    );
}
