//! t-078 / ENS-010: resolved Engine-root reporting.
//!
//! ENSV-011 `status_and_doctor_report_the_actual_resolved_engine_root`
//!
//! The executable reports must expose the same root the resolver selected for
//! the invocation.  A real Git primary and linked worktree exercise shared
//! runtime selection; a separate non-Git checkout exercises the local fallback.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const RUNBOOK: &str = "[phases.build]\nprompt = \"Report the Engine root.\"\n";

type TreeSnapshot = BTreeMap<String, Option<Vec<u8>>>;

struct GitFixture {
    sandbox: PathBuf,
    primary: PathBuf,
    linked: PathBuf,
    run: String,
}

impl GitFixture {
    fn new() -> Self {
        let sandbox = fresh_sandbox("git");
        let primary = sandbox.join("primary");
        fs::create_dir_all(&primary).expect("create primary fixture checkout");
        write_machine_class(&primary);

        git_success(&primary, &["init"]);
        git_success(&primary, &["config", "core.autocrlf", "false"]);
        git_success(&primary, &["config", "user.email", "qa@example.invalid"]);
        git_success(&primary, &["config", "user.name", "Ratmac QA"]);
        git_success(&primary, &["add", "--", ".ratmac/ratmac.toml"]);
        git_success(&primary, &["commit", "-m", "fixture base"]);

        let linked = sandbox.join("linked");
        let linked_output = Command::new("git")
            .args(["worktree", "add", "-b", "t078-linked"])
            .arg(&linked)
            .current_dir(&primary)
            .output()
            .expect("create linked Git worktree");
        assert!(
            linked_output.status.success(),
            "create linked Git worktree: {}",
            combined(&linked_output)
        );

        let start = rtm_at(&primary, &["start"]);
        assert!(
            start.status.success(),
            "fixture setup must start a primary Run: {}",
            combined(&start)
        );
        let run = only_run(&primary);

        Self {
            sandbox,
            primary,
            linked,
            run,
        }
    }
}

impl Drop for GitFixture {
    fn drop(&mut self) {
        let _ = Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&self.linked)
            .current_dir(&self.primary)
            .output();
        let _ = fs::remove_dir_all(&self.sandbox);
    }
}

struct NonGitFixture {
    sandbox: PathBuf,
    root: PathBuf,
    run: String,
}

impl NonGitFixture {
    fn new() -> Self {
        let sandbox = fresh_sandbox("no-git");
        let root = sandbox.join("checkout");
        fs::create_dir_all(&root).expect("create non-Git fixture checkout");
        assert!(
            !root
                .ancestors()
                .any(|ancestor| ancestor.join(".git").exists()),
            "the no-Git fixture must not be nested in a Git checkout"
        );
        write_machine_class(&root);

        let start = rtm_at(&root, &["start"]);
        assert!(
            start.status.success(),
            "fixture setup must start a non-Git Run: {}",
            combined(&start)
        );
        let run = only_run(&root);

        Self { sandbox, root, run }
    }
}

impl Drop for NonGitFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.sandbox);
    }
}

fn fresh_sandbox(label: &str) -> PathBuf {
    let sandbox = std::env::temp_dir().join(format!(
        "ratmac-t078-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock is after the Unix epoch")
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&sandbox);
    fs::create_dir_all(&sandbox).expect("create fixture sandbox");
    sandbox
}

fn write_machine_class(root: &Path) {
    let engine_root = root.join(".ratmac");
    fs::create_dir_all(&engine_root).expect("create fixture Engine root");
    fs::write(engine_root.join("ratmac.toml"), RUNBOOK).expect("write fixture Machine Class");
}

fn rtm_at(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rtm"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("invoke built rtm binary")
}

fn git_success(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("Git is executable for the fixture");
    assert!(
        output.status.success(),
        "git {args:?} failed in {}: {}",
        root.display(),
        combined(&output)
    );
}

fn only_run(root: &Path) -> String {
    let runs = ratmac::root::resolve(root).engine_root().join("runs");
    let mut ids = fs::read_dir(&runs)
        .expect("fixture Engine roster is listable")
        .map(|entry| entry.expect("fixture roster entry is readable"))
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    ids.sort();
    assert_eq!(
        ids.len(),
        1,
        "fixture setup must mint exactly one Run; roster was {ids:?}"
    );
    ids.pop().expect("one minted Run has an id")
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Every file and directory below `root`, preserving exact file bytes.
/// Directory rows make creation or deletion of empty directories observable.
fn tree_snapshot(root: &Path) -> TreeSnapshot {
    fn walk(root: &Path, directory: &Path, snapshot: &mut TreeSnapshot) {
        for entry in fs::read_dir(directory).expect("snapshot directory is listable") {
            let path = entry.expect("snapshot entry is readable").path();
            let relative = path
                .strip_prefix(root)
                .expect("snapshot entry remains below the fixture")
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                snapshot.insert(format!("{relative}/"), None);
                walk(root, &path, snapshot);
            } else {
                snapshot.insert(
                    relative,
                    Some(fs::read(path).expect("snapshot file is readable")),
                );
            }
        }
    }

    let mut snapshot = TreeSnapshot::new();
    walk(root, root, &mut snapshot);
    snapshot
}

fn assert_reported_root(
    command: &str,
    invocation_root: &Path,
    expected_root: &Path,
    output: &Output,
) {
    let report = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "ENS-010: `rtm {command}` from {} must succeed so it can report its root: {}",
        invocation_root.display(),
        combined(output)
    );

    let expected = expected_root.to_string_lossy().replace('\\', "/");
    assert!(
        report.replace('\\', "/").contains(&expected),
        "ENS-010: `rtm {command}` from {} must print the resolver-selected Engine root {expected}; output was:\n{report}",
        invocation_root.display()
    );
}

fn assert_reported_root_without_writing(
    fixture: &str,
    snapshot_root: &Path,
    invocation_root: &Path,
    run: &str,
) {
    let expected_root = ratmac::root::resolve(invocation_root)
        .engine_root()
        .to_path_buf();

    let before_status = tree_snapshot(snapshot_root);
    let status = rtm_at(invocation_root, &["status", "--run", run]);
    assert_reported_root("status", invocation_root, &expected_root, &status);
    assert_eq!(
        tree_snapshot(snapshot_root),
        before_status,
        "ENS-010: `rtm status` must leave the {fixture} fixture byte-identical"
    );

    let before_doctor = tree_snapshot(snapshot_root);
    let doctor = rtm_at(invocation_root, &["doctor"]);
    assert_reported_root("doctor", invocation_root, &expected_root, &doctor);
    assert_eq!(
        tree_snapshot(snapshot_root),
        before_doctor,
        "ENS-010: argument-free `rtm doctor` must leave the {fixture} fixture byte-identical"
    );
}

/// ENSV-011: status and argument-free doctor expose the root actually selected
/// by the resolver, agree through that shared oracle, and remain read-only.
#[test]
fn status_and_doctor_report_the_actual_resolved_engine_root() {
    let git = GitFixture::new();
    let no_git = NonGitFixture::new();

    assert_eq!(
        ratmac::root::resolve(&git.linked).engine_root(),
        ratmac::root::resolve(&git.primary).engine_root(),
        "fixture setup must resolve the linked worktree to the primary Engine root"
    );

    assert_reported_root_without_writing(
        "primary Git checkout",
        &git.sandbox,
        &git.primary,
        &git.run,
    );
    assert_reported_root_without_writing(
        "linked Git worktree",
        &git.sandbox,
        &git.linked,
        &git.run,
    );
    assert_reported_root_without_writing(
        "non-Git checkout",
        &no_git.sandbox,
        &no_git.root,
        &no_git.run,
    );
}
