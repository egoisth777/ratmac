//! t-071 / ENS: Engine root and resolution.
//!
//! ENSV-001 `engine_root_holds_runtime_and_never_writes_arca`
//! ENSV-002 `linked_worktree_shares_primary_runtime_but_reads_its_own_class`
//! ENSV-003 `non_git_checkout_uses_its_current_root`
//! ENSV-012 `git_tracks_class_and_receipts_but_ignores_runtime`
//!
//! The Engine owns `.ratmac/`; `.arca/` remains project-owned.  These tests
//! exercise the compiled `rtm` binary against real temporary projects rather
//! than calling Scheduler internals, including Git's actual linked-worktree
//! and ignore behaviour.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const RUNBOOK: &str = "[phases.intake]\nprompt = \"Integrate the issues.\"\n\n\
    [phases.build]\nprompt = \"Build the ticket.\"\n\n\
    [[transitions]]\nfrom = \"intake\"\nto = \"build\"\n";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fresh_sandbox(label: &str) -> PathBuf {
    let sandbox = std::env::temp_dir().join(format!(
        "ratmac-t071-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock must be after the Unix epoch")
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&sandbox);
    fs::create_dir_all(&sandbox).expect("create temporary fixture sandbox");
    sandbox
}

fn write_machine_class(root: &Path) {
    fs::create_dir_all(root.join(".ratmac")).expect("create Engine-root directory");
    fs::write(root.join(".ratmac/ratmac.toml"), RUNBOOK).expect("write Machine Class");
}

fn seed_human_arca(root: &Path) {
    fs::create_dir_all(root.join(".arca/goal")).expect("create human project tree");
    fs::write(root.join(".arca/goal/spec.md"), "# Fixture goal\n")
        .expect("write human project artifact");
}

struct Fixture {
    sandbox: PathBuf,
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.sandbox);
    }
}

impl Fixture {
    fn new(label: &str, seed_arca: bool) -> Self {
        let sandbox = fresh_sandbox(label);
        let root = sandbox.join("checkout");
        fs::create_dir_all(&root).expect("create fixture checkout");
        write_machine_class(&root);
        if seed_arca {
            seed_human_arca(&root);
        }
        Self { sandbox, root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        rtm_at(&self.root, args)
    }
}

fn rtm_at(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rtm"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("invoke built rtm binary")
}

fn rtm_at_with_env(root: &Path, args: &[&str], environment: &[(&str, &Path)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rtm"));
    command.args(args).current_dir(root);
    for &(name, value) in environment {
        command.env(name, value);
    }
    command.output().expect("invoke built rtm binary")
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Every file and directory under `root`, keyed by its forward-slashed path.
/// Directories carry a trailing slash, so creation of an otherwise empty
/// `.arca/` directory is observable too.
fn tree_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, directory: &Path, into: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(directory).expect("snapshot directory is listable") {
            let path = entry.expect("snapshot entry is readable").path();
            let relative = path
                .strip_prefix(root)
                .expect("snapshot path remains under its root")
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                into.insert(format!("{relative}/"), Vec::new());
                walk(root, &path, into);
            } else {
                into.insert(
                    relative,
                    fs::read(&path).expect("snapshot file is readable"),
                );
            }
        }
    }

    let mut snapshot = BTreeMap::new();
    walk(root, root, &mut snapshot);
    snapshot
}

/// Listing `.ratmac/runs/` is the Engine's observable Run roster.
fn roster_at(root: &Path) -> Vec<String> {
    let runs = root.join(".ratmac/runs");
    assert!(
        runs.is_dir(),
        "ENS-001: the Engine roster must reside at {}; it must not fall back to .arca/runs/",
        runs.display()
    );
    let mut ids: Vec<String> = fs::read_dir(&runs)
        .expect("Engine roster is listable")
        .map(|entry| entry.expect("roster entry is readable"))
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    ids.sort();
    ids
}

fn write_receipt(root: &Path, run_id: &str, name: &str) -> PathBuf {
    let receipt = root
        .join(".ratmac/evidence")
        .join(run_id)
        .join(format!("{name}.toml"));
    fs::create_dir_all(
        receipt
            .parent()
            .expect("receipt path has an evidence directory"),
    )
    .expect("create run-scoped receipt directory");
    fs::write(
        &receipt,
        format!("run-id = \"{run_id}\"\nreceipt = \"{name}\"\n"),
    )
    .expect("write run-scoped receipt");
    receipt
}

fn assert_runtime_layout(root: &Path, run_id: &str) {
    let engine = root.join(".ratmac");
    assert!(
        engine.join("ratmac.toml").is_file(),
        "ENS-001: the Machine Class must remain in the Engine root"
    );
    assert!(
        engine.join("runs").is_dir(),
        "ENS-001: the Run tree must be beneath .ratmac/"
    );
    assert!(
        engine
            .join("runs")
            .join(run_id)
            .join("state.toml")
            .is_file(),
        "ENS-001: the addressed Run State File must be beneath .ratmac/runs/{run_id}/"
    );
    assert!(
        engine.join("mint.toml").is_file(),
        "ENS-001: the Engine root must hold its durable mint record"
    );
    assert!(
        engine.join("locks").is_dir(),
        "ENS-001: the Engine root must hold its locks directory"
    );
    assert!(
        engine.join("log.md").is_file(),
        "ENS-001: the Engine transition log must live at .ratmac/log.md"
    );
}

fn git(root: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("Git must be executable for this fixture")
}

fn git_success(root: &Path, args: &[&str]) {
    let output = git(root, args);
    assert!(
        output.status.success(),
        "git {args:?} failed in {}: {}",
        root.display(),
        combined(&output)
    );
}

fn git_text(root: &Path, args: &[&str]) -> String {
    let output = git(root, args);
    assert!(
        output.status.success(),
        "git {args:?} failed in {}: {}",
        root.display(),
        combined(&output)
    );
    String::from_utf8(output.stdout).expect("Git output must be UTF-8")
}

fn assert_ignored_by_git(root: &Path, path: &str) {
    let output = git(root, &["check-ignore", "-v", "--", path]);
    assert!(
        output.status.success(),
        "ENS-012: {path} must be ignored by the fixture's repository .gitignore: {}",
        combined(&output)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains(path),
        "ENS-012: git check-ignore must identify ignored path {path}: {}",
        combined(&output)
    );
}

fn assert_not_ignored_by_git(root: &Path, path: &str) {
    let output = git(root, &["check-ignore", "-v", "--", path]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "ENS-012: {path} must be eligible for tracking, not ignored; git check-ignore must exit 1, got {:?}: {}",
        output.status.code(),
        combined(&output)
    );
}

struct GitFixture {
    sandbox: PathBuf,
    primary: PathBuf,
    linked: Option<PathBuf>,
}

impl Drop for GitFixture {
    fn drop(&mut self) {
        if let Some(linked) = &self.linked {
            let _ = Command::new("git")
                .args(["worktree", "remove", "--force"])
                .arg(linked)
                .current_dir(&self.primary)
                .output();
        }
        let _ = fs::remove_dir_all(&self.sandbox);
    }
}

impl GitFixture {
    /// Make a real repository whose ignore policy is byte-for-byte the policy
    /// of this repository.  The linked-worktree case needs the Machine Class
    /// committed so Git materializes an independent copy in that checkout.
    fn new(label: &str, commit_machine_class: bool) -> Self {
        let sandbox = fresh_sandbox(label);
        let primary = sandbox.join("primary");
        fs::create_dir_all(&primary).expect("create primary checkout");
        fs::write(
            primary.join(".gitignore"),
            fs::read(repo_root().join(".gitignore")).expect("read repository .gitignore"),
        )
        .expect("write fixture .gitignore");
        if commit_machine_class {
            write_machine_class(&primary);
        }

        git_success(&primary, &["init"]);
        // The pin is over class bytes, so a linked checkout must not receive
        // a global autocrlf rewrite that looks like an intentional drift.
        git_success(&primary, &["config", "core.autocrlf", "false"]);
        git_success(&primary, &["config", "user.email", "qa@example.invalid"]);
        git_success(&primary, &["config", "user.name", "Ratmac QA"]);
        if commit_machine_class {
            git_success(
                &primary,
                &["add", "--", ".gitignore", ".ratmac/ratmac.toml"],
            );
        } else {
            git_success(&primary, &["add", "--", ".gitignore"]);
        }
        git_success(&primary, &["commit", "-m", "fixture base"]);

        Self {
            sandbox,
            primary,
            linked: None,
        }
    }

    fn add_linked_worktree(&mut self) -> PathBuf {
        let linked = self.sandbox.join("linked");
        let output = Command::new("git")
            .args(["worktree", "add", "-b", "t071-linked"])
            .arg(&linked)
            .current_dir(&self.primary)
            .output()
            .expect("git worktree add must be executable");
        assert!(
            output.status.success(),
            "git worktree add failed: {}",
            combined(&output)
        );
        self.linked = Some(linked.clone());
        linked
    }
}

/// ENSV-001: a full ordinary Run uses only the Engine root.  `.arca/` is
/// deliberately pre-seeded with human project material so an Engine write is
/// distinguishable from merely creating an otherwise absent directory.
#[test]
fn engine_root_holds_runtime_and_never_writes_arca() {
    let fixture = Fixture::new("engine-root", true);
    let arca_before = tree_snapshot(&fixture.root.join(".arca"));

    let start = fixture.rtm(&["start"]);
    assert!(
        start.status.success(),
        "ENS-001: rtm start must load .ratmac/ratmac.toml and succeed: {}",
        combined(&start)
    );

    let roster = roster_at(&fixture.root);
    assert_eq!(
        roster.len(),
        1,
        "ENS-001: one start must create exactly one Run beneath .ratmac/runs/, found {roster:?}"
    );
    let run_id = &roster[0];
    assert_runtime_layout(&fixture.root, run_id);

    let status_before = fixture.rtm(&["status", "--run", run_id]);
    assert!(
        status_before.status.success(),
        "ENS-001: status must reopen the Run from .ratmac/runs/{run_id}/: {}",
        combined(&status_before)
    );
    assert!(
        combined(&status_before).contains("Integrate the issues."),
        "ENS-001: initial status must report the Run's initial Phase: {}",
        combined(&status_before)
    );

    let receipt = write_receipt(&fixture.root, run_id, "ensv-001");
    assert!(
        receipt.is_file(),
        "ENS-001: run-scoped receipts must live at {}",
        receipt.display()
    );

    let step = fixture.rtm(&["step", "--run", run_id]);
    assert!(
        step.status.success(),
        "ENS-001: rtm step must advance the Run stored in .ratmac/: {}",
        combined(&step)
    );
    let status_after = fixture.rtm(&["status", "--run", run_id]);
    assert!(
        status_after.status.success(),
        "ENS-001: status after step must still reopen the same .ratmac Run: {}",
        combined(&status_after)
    );
    assert!(
        combined(&status_after).contains("Build the ticket."),
        "ENS-001: the Run must advance to its second Phase: {}",
        combined(&status_after)
    );
    let state: toml::Value = fs::read_to_string(
        fixture
            .root
            .join(".ratmac/runs")
            .join(run_id)
            .join("state.toml"),
    )
    .expect("read relocated State File")
    .parse()
    .expect("relocated State File is valid TOML");
    assert_eq!(
        state["phase"].as_str(),
        Some("build"),
        "ENS-001: the relocated State File records the advanced Phase"
    );

    assert_eq!(
        tree_snapshot(&fixture.root.join(".arca")),
        arca_before,
        "ENS-001: Engine start, status, and step must leave every .arca/ path byte-identical"
    );
}

/// ENSV-002: Git's linked-worktree metadata selects one shared runtime root,
/// while the class file remains an input from the invoking checkout.
#[test]
fn linked_worktree_shares_primary_runtime_but_reads_its_own_class() {
    let mut fixture = GitFixture::new("linked-worktree", true);
    let linked = fixture.add_linked_worktree();
    let primary_class = fixture.primary.join(".ratmac/ratmac.toml");
    let linked_class = linked.join(".ratmac/ratmac.toml");
    let primary_bytes = fs::read(&primary_class).expect("read primary Machine Class");
    assert_eq!(
        fs::read(&linked_class).expect("linked checkout has its tracked Machine Class"),
        primary_bytes,
        "the linked checkout begins from the primary's committed Machine Class"
    );
    let expected_pin = sha256_hex(&primary_bytes);

    let start = rtm_at(&fixture.primary, &["start"]);
    assert!(
        start.status.success(),
        "ENS-002: the primary checkout must start a Run from .ratmac/ratmac.toml: {}",
        combined(&start)
    );
    let primary_roster = roster_at(&fixture.primary);
    assert_eq!(
        primary_roster.len(),
        1,
        "ENS-002: primary start must create one shared Run, found {primary_roster:?}"
    );
    let run_id = primary_roster[0].clone();
    assert_runtime_layout(&fixture.primary, &run_id);

    let linked_status = rtm_at(&linked, &["status", "--run", &run_id]);
    assert!(
        linked_status.status.success(),
        "ENS-002: linked status must open the primary checkout's Run {run_id}: {}",
        combined(&linked_status)
    );
    assert!(
        combined(&linked_status).contains("Integrate the issues."),
        "ENS-002: linked status must observe the shared Run's current Phase: {}",
        combined(&linked_status)
    );
    assert_eq!(
        roster_at(&fixture.primary),
        primary_roster,
        "ENS-002: a linked status invocation must not mint a checkout-local Run"
    );
    assert!(
        !linked.join(".ratmac/runs").exists(),
        "ENS-002: the linked checkout must not receive a private .ratmac/runs/ tree"
    );

    let mut drifted = primary_bytes.clone();
    drifted.extend_from_slice(b"\n# linked worktree pin drift\n");
    fs::write(&linked_class, &drifted).expect("edit only the linked Machine Class");
    assert_eq!(
        fs::read(&primary_class).expect("re-read primary Machine Class"),
        primary_bytes,
        "editing the linked checkout must not edit the primary checkout's class"
    );
    let observed_pin = sha256_hex(&drifted);
    let primary_before_refusal = tree_snapshot(&fixture.primary.join(".ratmac"));

    let drift = rtm_at(&linked, &["status", "--run", &run_id]);
    assert!(
        !drift.status.success(),
        "ENS-002/FDC-005: an invoking-worktree class edit must refuse on pin drift, not succeed: {}",
        combined(&drift)
    );
    let drift_text = combined(&drift);
    assert!(
        drift_text.contains("pin") || drift_text.contains("drift"),
        "ENS-002/FDC-005: the refusal must identify pin drift: {drift_text}"
    );
    for (role, pin) in [("expected", &expected_pin), ("observed", &observed_pin)] {
        assert!(
            drift_text.contains(pin),
            "ENS-002/FDC-005: the pin-drift refusal must name the {role} SHA-256 {pin}: {drift_text}"
        );
    }
    assert_eq!(
        tree_snapshot(&fixture.primary.join(".ratmac")),
        primary_before_refusal,
        "ENS-002/FDC-005: pin drift must leave the shared runtime byte-identical"
    );
}

/// ENSV-003: with no Git metadata, resolution is exactly the current
/// checkout.  An alternate `.ratmac/` in the parent is a sentinel that must
/// not be selected by a resolver that is allowed no Git-derived primary.
#[test]
fn non_git_checkout_uses_its_current_root() {
    let fixture = Fixture::new("no-git", true);
    assert!(
        !fixture.root.join(".git").exists() && !fixture.sandbox.join(".git").exists(),
        "ENS-002: the no-Git fixture must contain no .git directory"
    );
    let git_probe = git(&fixture.root, &["rev-parse", "--is-inside-work-tree"]);
    assert!(
        !git_probe.status.success(),
        "ENS-002: the no-Git fixture must not inherit a Git checkout: {}",
        combined(&git_probe)
    );

    let parent_engine = fixture.sandbox.join(".ratmac");
    fs::create_dir_all(&parent_engine).expect("create parent sentinel Engine root");
    fs::write(
        parent_engine.join("ratmac.toml"),
        "not = \"the checkout class\"\n",
    )
    .expect("write parent sentinel class");
    let parent_before = tree_snapshot(&parent_engine);
    let arca_before = tree_snapshot(&fixture.root.join(".arca"));

    let start = fixture.rtm(&["start"]);
    assert!(
        start.status.success(),
        "ENS-002: rtm start must work without Git by using the current .ratmac/: {}",
        combined(&start)
    );
    let roster = roster_at(&fixture.root);
    assert_eq!(
        roster.len(),
        1,
        "ENS-002: the current checkout must own exactly the Run it starts, found {roster:?}"
    );
    let run_id = &roster[0];
    assert_runtime_layout(&fixture.root, run_id);

    let status = fixture.rtm(&["status", "--run", run_id]);
    assert!(
        status.status.success(),
        "ENS-002: status must reopen the no-Git checkout's own Run: {}",
        combined(&status)
    );
    assert!(
        combined(&status).contains("Integrate the issues."),
        "ENS-002: no-Git status must report the current checkout's Machine Class: {}",
        combined(&status)
    );
    assert!(
        !parent_engine.join("runs").exists(),
        "ENS-002: no-Git resolution must not guess the parent .ratmac/ root"
    );
    assert_eq!(
        tree_snapshot(&parent_engine),
        parent_before,
        "ENS-002: no-Git resolution must not read or write the parent sentinel root"
    );
    assert_eq!(
        tree_snapshot(&fixture.root.join(".arca")),
        arca_before,
        "ENS-001: no-Git Engine operations must not write under .arca/"
    );
}

/// A worktree with separately stored Git metadata still owns its runtime.
#[test]
fn separate_git_dir_uses_worktree_engine_root() {
    let fixture = Fixture::new("separate-git-dir", false);
    let external_git_dir = fixture.sandbox.join("external-git");
    let misplaced_engine = external_git_dir
        .parent()
        .expect("external Git storage has a parent")
        .join(".ratmac");
    fs::create_dir_all(&misplaced_engine).expect("create external-storage sentinel Engine root");
    fs::write(misplaced_engine.join("sentinel"), "do not use this root\n")
        .expect("write external-storage sentinel");
    let misplaced_before = tree_snapshot(&misplaced_engine);

    let external_git_dir_arg = external_git_dir.to_string_lossy().into_owned();
    git_success(
        &fixture.root,
        &["init", "--separate-git-dir", &external_git_dir_arg],
    );
    assert!(
        fixture.root.join(".git").is_file(),
        "the worktree must use a .git indirection file"
    );
    assert!(
        external_git_dir.is_dir(),
        "git init must create the separately stored Git directory"
    );

    let start = fixture.rtm(&["start"]);
    assert!(
        start.status.success(),
        "a separate-git-dir worktree must start from its own .ratmac/: {}",
        combined(&start)
    );
    let roster = roster_at(&fixture.root);
    assert_eq!(
        roster.len(),
        1,
        "the worktree must own the Run it starts, found {roster:?}"
    );
    assert_runtime_layout(&fixture.root, &roster[0]);
    assert_eq!(
        tree_snapshot(&misplaced_engine),
        misplaced_before,
        "the Engine must not use .ratmac/ beside separately stored Git metadata"
    );
}

/// Ambient Git routing must not turn a non-Git invocation into an unrelated
/// checkout's shared runtime.
#[test]
fn ambient_git_dir_and_work_tree_do_not_divert_non_git_checkout() {
    let fixture = Fixture::new("ambient-git-environment", false);
    let unrelated = GitFixture::new("ambient-git-environment-unrelated", false);
    assert!(
        !fixture.root.join(".git").exists(),
        "the invoking fixture must have no Git metadata of its own"
    );

    let unrelated_git_dir = unrelated.primary.join(".git");
    assert!(
        unrelated_git_dir.is_dir(),
        "the unrelated fixture must provide a real Git directory"
    );
    let unrelated_engine = unrelated.primary.join(".ratmac");
    fs::create_dir_all(&unrelated_engine).expect("create unrelated Engine-root sentinel");
    fs::write(unrelated_engine.join("sentinel"), "do not use this root\n")
        .expect("write unrelated Engine-root sentinel");
    let unrelated_before = tree_snapshot(&unrelated_engine);

    let start = rtm_at_with_env(
        &fixture.root,
        &["start"],
        &[
            ("GIT_DIR", unrelated_git_dir.as_path()),
            ("GIT_WORK_TREE", unrelated.primary.as_path()),
        ],
    );
    assert!(
        start.status.success(),
        "ambient Git routing must not prevent the non-Git checkout from starting locally: {}",
        combined(&start)
    );
    let roster = roster_at(&fixture.root);
    assert_eq!(
        roster.len(),
        1,
        "the non-Git checkout must own the Run it starts, found {roster:?}"
    );
    assert_runtime_layout(&fixture.root, &roster[0]);
    assert_eq!(
        tree_snapshot(&unrelated_engine),
        unrelated_before,
        "ambient GIT_DIR and GIT_WORK_TREE must not divert runtime into the unrelated checkout"
    );
}

/// ENSV-012: a ticket branch stages the human-authored class and durable
/// receipts, never live Run state.  The fixture copies this repository's
/// `.gitignore` rather than embedding a second policy.
#[test]
fn git_tracks_class_and_receipts_but_ignores_runtime() {
    let fixture = GitFixture::new("git-tracking", false);
    git_success(
        &fixture.primary,
        &["checkout", "-b", "ticket/t071-engine-root"],
    );
    write_machine_class(&fixture.primary);
    assert_eq!(
        fs::read(fixture.primary.join(".gitignore")).expect("read fixture .gitignore"),
        fs::read(repo_root().join(".gitignore")).expect("read repository .gitignore"),
        "ENS-012: the Git fixture must carry this repository's exact ignore rules"
    );

    let first_start = rtm_at(&fixture.primary, &["start"]);
    assert!(
        first_start.status.success(),
        "ENS-012: rtm start must create runtime beneath .ratmac/: {}",
        combined(&first_start)
    );
    let second_start = rtm_at(&fixture.primary, &["start"]);
    assert!(
        second_start.status.success(),
        "ENS-012: a second Run must share the .ratmac runtime tree: {}",
        combined(&second_start)
    );
    let roster = roster_at(&fixture.primary);
    assert_eq!(
        roster.len(),
        2,
        "ENS-012: the fixture needs two Run identifiers for independent receipt paths, found {roster:?}"
    );
    assert_ne!(
        roster[0], roster[1],
        "ENS-012: the two receipt paths need distinct Run ids"
    );
    assert_runtime_layout(&fixture.primary, &roster[0]);
    assert_runtime_layout(&fixture.primary, &roster[1]);

    let receipt_one = write_receipt(&fixture.primary, &roster[0], "receipt-one");
    let receipt_two = write_receipt(&fixture.primary, &roster[1], "receipt-two");
    let receipt_one_rel = receipt_one
        .strip_prefix(&fixture.primary)
        .expect("receipt belongs to primary checkout")
        .to_string_lossy()
        .replace('\\', "/");
    let receipt_two_rel = receipt_two
        .strip_prefix(&fixture.primary)
        .expect("receipt belongs to primary checkout")
        .to_string_lossy()
        .replace('\\', "/");
    let lock_probe = fixture.primary.join(".ratmac/locks/settled.lock");
    fs::write(&lock_probe, "runtime lock fixture\n").expect("write ignored runtime lock");

    let run_state = format!(".ratmac/runs/{}/state.toml", roster[0]);
    let runtime_paths = [
        run_state.as_str(),
        ".ratmac/mint.toml",
        ".ratmac/locks/settled.lock",
        ".ratmac/log.md",
    ];
    for path in runtime_paths {
        assert_ignored_by_git(&fixture.primary, path);
    }
    for path in [
        ".ratmac/ratmac.toml",
        receipt_one_rel.as_str(),
        receipt_two_rel.as_str(),
    ] {
        assert_not_ignored_by_git(&fixture.primary, path);
    }

    git_success(
        &fixture.primary,
        &[
            "add",
            "--",
            ".ratmac/ratmac.toml",
            receipt_one_rel.as_str(),
            receipt_two_rel.as_str(),
        ],
    );
    let porcelain = git_text(&fixture.primary, &["status", "--porcelain"]);
    for path in [
        ".ratmac/ratmac.toml",
        receipt_one_rel.as_str(),
        receipt_two_rel.as_str(),
    ] {
        assert!(
            porcelain.contains(path),
            "ENS-012: tracked class and receipt paths must appear in git status --porcelain; {path} is absent from:\n{porcelain}"
        );
    }
    for path in runtime_paths {
        assert!(
            !porcelain.contains(path),
            "ENS-012: ignored runtime path {path} must not appear in git status --porcelain:\n{porcelain}"
        );
    }

    let staged = git_text(&fixture.primary, &["diff", "--cached", "--name-only"]);
    for path in [
        ".ratmac/ratmac.toml",
        receipt_one_rel.as_str(),
        receipt_two_rel.as_str(),
    ] {
        assert!(
            staged.lines().any(|line| line == path),
            "ENS-012: ticket-branch index must track {path}; staged paths were:\n{staged}"
        );
    }
    for path in runtime_paths {
        assert!(
            !staged.lines().any(|line| line == path),
            "ENS-012: ticket-branch index must contain no live Run state ({path}); staged paths were:\n{staged}"
        );
    }
}
