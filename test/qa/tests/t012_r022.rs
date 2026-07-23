use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use arca_scheduler::Scheduler;

const RATMAC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/t012-active-run/.arca/ratmac.toml"
));

fn fresh_project_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!("arca-scheduler-t012-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove stale t-012 fixture project");
    }
    fs::create_dir_all(root.join(".arca")).expect("create fixture project");
    fs::write(root.join(".arca/ratmac.toml"), RATMAC).expect("write fixture class");
    root
}

fn snapshot_owned_files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    for name in ["ratmac.toml", "state.toml", "log.md"] {
        let path = root.join(".arca").join(name);
        if path.exists() {
            files.insert(
                name.to_owned(),
                fs::read(path).expect("read scheduler-owned fixture file"),
            );
        }
    }
    files
}

#[test]
fn start_refuses_second_active_run() {
    let project = fresh_project_root();

    let mut first_scheduler = Scheduler::open(&project).expect("open fixture project");
    let first = first_scheduler.start().expect("first schd start succeeds");
    let before = snapshot_owned_files(&project);
    let mut second_scheduler = Scheduler::open(&project).expect("reopen fixture project");

    let refusal = match second_scheduler.start() {
        Ok(_) => panic!("second active Run must be refused"),
        Err(error) => error,
    };
    let message = refusal.to_string().to_ascii_lowercase();
    assert!(
        message.contains("active"),
        "second-start refusal must explain the active Run conflict: {message}"
    );

    let after = snapshot_owned_files(&project);
    assert_eq!(
        before, after,
        "refused second start must not mutate the original Run or class"
    );
    drop(first);
    fs::remove_dir_all(project).expect("clean up t-012 fixture project");
}
