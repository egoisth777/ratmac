//! t-074 / ENS-006: durable child-workspace binding and sibling receipts.
//!
//! ENSV-007 `spawn_records_canonical_or_inherited_workspace_and_uses_it`
//! ENSV-013 `parallel_sibling_receipts_are_run_scoped_and_merge_cleanly`
//!
//! Each test drives the compiled public `rtm` binary in a real temporary Git
//! repository. The first uses distinguishable guarded files to prove that a
//! child Run's durable workspace binding, rather than the later caller's
//! current directory, controls motion. The second derives both sibling IDs
//! from real spawn output and proves their evidence survives an ordinary Git
//! merge without a collision.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The parent can repeatedly spawn `worker` children while parked at
/// `delegate`. A worker's only transition is guarded by a file whose presence
/// is deliberately different between the caller's checkout and each child
/// workspace.
const WORKSPACE_RUNBOOK: &str = r#"
[classes.worker.states.work]
prompt = "Work in the child's bound workspace."
guards = [{ kind = "file_contains", path = "workspace-gate.txt", contains = "ready" }]

[classes.worker.states.done]
prompt = "The child completed its workspace-bound work."

[[classes.worker.transitions]]
from = "work"
to = "done"

[states.plan]
prompt = "Prepare the parent."

[states.delegate]
prompt = "Spawn workspace-bound children."

[[states.delegate.spawns]]
class = "worker"
name = "worker"

[states.done]
prompt = "The parent is done."

[[transitions]]
from = "plan"
to = "delegate"

[[transitions]]
from = "delegate"
to = "done"
"#;

struct GitFixture {
    sandbox: PathBuf,
    primary: PathBuf,
}

impl GitFixture {
    fn new(label: &str) -> Self {
        let sandbox = std::env::temp_dir().join(format!(
            "ratmac-t074-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock must be after the Unix epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&sandbox);

        let primary = sandbox.join("primary");
        fs::create_dir_all(primary.join(".ratmac")).expect("create primary Engine directory");
        fs::write(
            primary.join(".gitignore"),
            fs::read(repo_root().join(".gitignore")).expect("read repository .gitignore"),
        )
        .expect("copy repository .gitignore into fixture");
        fs::write(primary.join(".ratmac/ratmac.toml"), WORKSPACE_RUNBOOK)
            .expect("write workspace fixture Machine Class");

        git_success(&primary, &["init"]);
        git_success(&primary, &["config", "core.autocrlf", "false"]);
        git_success(&primary, &["config", "user.email", "qa@example.invalid"]);
        git_success(&primary, &["config", "user.name", "Ratmac QA"]);
        git_success(
            &primary,
            &["add", "--", ".gitignore", ".ratmac/ratmac.toml"],
        );
        git_success(&primary, &["commit", "-m", "fixture base"]);

        Self { sandbox, primary }
    }

    fn run_dir(&self, run_id: &str) -> PathBuf {
        self.primary.join(".ratmac/runs").join(run_id)
    }

    fn ledger_path(&self, parent: &str) -> PathBuf {
        self.run_dir(parent).join("spawn-ledger")
    }

    fn ledger_bytes(&self, parent: &str) -> Vec<u8> {
        fs::read(self.ledger_path(parent)).expect("parent spawn ledger is readable")
    }

    fn start_at_delegate(&self) -> String {
        let started = rtm_at(&self.primary, &["start"]);
        let started_text = combined(&started);
        assert!(
            started.status.success(),
            "fixture parent starts successfully: {started_text}"
        );
        let parent = minted_id(&started, "started run ");

        let step = rtm_at(&self.primary, &["step", "--run", &parent]);
        assert!(
            step.status.success(),
            "fixture parent enters its spawning State: {}",
            combined(&step)
        );
        parent
    }
}

impl Drop for GitFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.sandbox);
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rtm_at(root: &Path, args: &[&str]) -> Output {
    Command::new(ratmac_qa::engine_bin!())
        .args(args)
        .current_dir(root)
        .output()
        .expect("invoke built rtm binary")
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
    String::from_utf8(output.stdout).expect("Git output is UTF-8")
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn minted_id(output: &Output, wording: &str) -> String {
    let text = combined(output);
    text.split(wording)
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or_else(|| panic!("rtm output must contain {wording:?}: {text}"))
        .to_owned()
}

fn roster_at(root: &Path) -> Vec<String> {
    let runs = root.join(".ratmac/runs");
    assert!(
        runs.is_dir(),
        "fixture Engine roster is present at {}",
        runs.display()
    );
    let mut roster: Vec<String> = fs::read_dir(runs)
        .expect("fixture roster is listable")
        .map(|entry| entry.expect("fixture roster entry is readable"))
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    roster.sort();
    roster
}

/// Parse the parent-owned ledger without accepting a missing `workspace`
/// field. The `field` helper below deliberately panics for absent fields.
fn ledger_entries(bytes: &[u8]) -> Vec<toml::value::Table> {
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Vec::new();
    }
    let source = std::str::from_utf8(bytes).expect("spawn ledger is UTF-8");
    let value: toml::Value = source
        .parse()
        .unwrap_or_else(|error| panic!("spawn ledger is valid TOML: {error}\n{source}"));
    let table = value.as_table().expect("spawn ledger top level is a table");
    let Some(children) = table.get("children") else {
        return Vec::new();
    };
    children
        .as_array()
        .expect("spawn ledger children is an array")
        .iter()
        .map(|entry| {
            entry
                .as_table()
                .expect("each spawn ledger child is a table")
                .clone()
        })
        .collect()
}

fn field<'a>(entry: &'a toml::value::Table, name: &str) -> &'a str {
    entry
        .get(name)
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("spawn ledger field {name:?} is a string: {entry:?}"))
}

fn entry_for<'a>(entries: &'a [toml::value::Table], run_id: &str) -> &'a toml::value::Table {
    entries
        .iter()
        .find(|entry| field(entry, "id") == run_id)
        .unwrap_or_else(|| panic!("spawn ledger records child {run_id:?}: {entries:?}"))
}

/// Canonicalize first, then remove only the Windows verbatim marker and
/// separator spelling. Applying the same normalization to the expected
/// canonical path and the recorded string prevents a `\\\\?\\` prefix from
/// creating a false difference while still rejecting a noncanonical spelling.
fn canonical_workspace(path: &Path) -> String {
    let canonical = fs::canonicalize(path)
        .unwrap_or_else(|error| panic!("canonicalize workspace {}: {error}", path.display()));
    normalize_workspace_text(&canonical.to_string_lossy())
}

fn normalize_workspace_text(text: &str) -> String {
    let slash_normalized = text.replace('\\', "/");
    slash_normalized
        .strip_prefix("//?/")
        .unwrap_or(&slash_normalized)
        .to_owned()
}

fn assert_recorded_workspace(entry: &toml::value::Table, expected_workspace: &Path, label: &str) {
    let recorded = field(entry, "workspace");
    let expected = canonical_workspace(expected_workspace);
    let recorded_path = Path::new(recorded);
    assert!(
        recorded_path.is_absolute(),
        "ENSV-007 {label}: recorded workspace is an absolute path, not {recorded:?}"
    );
    assert_eq!(
        normalize_workspace_text(recorded),
        expected,
        "ENSV-007 {label}: the ledger stores exactly one normalized canonical workspace spelling"
    );
    assert_eq!(
        canonical_workspace(recorded_path),
        expected,
        "ENSV-007 {label}: canonicalizing the recorded workspace and expected workspace agrees"
    );
}

fn spawn_worker(root: &Path, parent: &str, workspace: Option<&str>, label: &str) -> String {
    let mut args = vec!["spawn", "worker", "--run", parent];
    if let Some(workspace) = workspace {
        args.extend(["--workspace", workspace]);
    }
    let output = rtm_at(root, &args);
    let text = combined(&output);
    assert!(output.status.success(), "{label}: spawn succeeds: {text}");
    minted_id(&output, "spawned run ")
}

/// Every workspace-input refusal is checked before a child exists or its
/// parent-owned ledger changes. `canonical_path` is reserved for the
/// out-of-repository case, whose diagnostic deliberately names the resolved
/// path rather than a caller-relative spelling.
fn assert_spawn_refusal(
    fixture: &GitFixture,
    parent: &str,
    args: &[&str],
    fragments: &[&str],
    canonical_path: Option<&Path>,
    label: &str,
) {
    let roster_before = roster_at(&fixture.primary);
    let ledger_before = fixture.ledger_bytes(parent);
    let output = rtm_at(&fixture.primary, args);
    let text = combined(&output);
    assert!(
        !output.status.success(),
        "ENSV-007 {label}: invalid workspace input refuses: {text}"
    );
    for fragment in fragments {
        assert!(
            text.contains(fragment),
            "ENSV-007 {label}: refusal contains {fragment:?}: {text}"
        );
    }
    if let Some(path) = canonical_path {
        let expected = canonical_workspace(path);
        assert!(
            normalize_workspace_text(&text).contains(&expected),
            "ENSV-007 {label}: outside-workspace refusal names canonical path {expected:?}: {text}"
        );
    }
    assert_eq!(
        roster_at(&fixture.primary),
        roster_before,
        "ENSV-007 {label}: a refused spawn adds no Run to the roster"
    );
    assert_eq!(
        fixture.ledger_bytes(parent),
        ledger_before,
        "ENSV-007 {label}: a refused spawn leaves the parent ledger byte-identical"
    );
}

fn assert_child_advanced(fixture: &GitFixture, child: &str, label: &str) {
    let step = rtm_at(&fixture.primary, &["step", "--run", child]);
    assert!(
        step.status.success(),
        "ENSV-007 {label}: the child advances from a caller outside its workspace: {}",
        combined(&step)
    );
    let state = fs::read_to_string(fixture.run_dir(child).join("run.toml"))
        .expect("advanced child State File is readable");
    assert!(
        state.contains("state = \"done\""),
        "ENSV-007 {label}: the child reaches its terminal State: {state}"
    );
}

fn write_receipt(root: &Path, run_id: &str, name: &str, bytes: &[u8]) -> PathBuf {
    let receipt = root
        .join(".ratmac/evidence")
        .join(run_id)
        .join(format!("{name}.toml"));
    fs::create_dir_all(
        receipt
            .parent()
            .expect("run-scoped receipt has an evidence directory"),
    )
    .expect("create run-scoped receipt directory");
    fs::write(&receipt, bytes).expect("write child receipt");
    receipt
}

fn receipt_relative(root: &Path, receipt: &Path) -> String {
    receipt
        .strip_prefix(root)
        .expect("receipt belongs to fixture primary checkout")
        .to_string_lossy()
        .replace('\\', "/")
}

fn assert_not_ignored_by_git(root: &Path, path: &str) {
    let output = git(root, &["check-ignore", "-v", "--", path]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "ENSV-013: {path} is eligible for tracking under the fixture's copied .gitignore; git check-ignore exited {:?}: {}",
        output.status.code(),
        combined(&output)
    );
}

fn current_branch(root: &Path) -> String {
    let branch = git_text(root, &["branch", "--show-current"])
        .trim()
        .to_owned();
    assert!(
        !branch.is_empty(),
        "fixture base commit has a checked-out branch"
    );
    branch
}

/// ENSV-007: an explicit workspace is canonical and durable, a relative
/// spelling is not a second binding, an omitted flag inherits the parent, and
/// child motion ignores a later caller's directory. Invalid workspace inputs
/// all refuse before mutating the roster or parent ledger.
#[test]
fn spawn_records_canonical_or_inherited_workspace_and_uses_it() {
    let fixture = GitFixture::new("workspace-binding");
    let child_workspace = fixture.primary.join("child-ws");
    let empty_workspace = fixture.primary.join("empty-ws");
    fs::create_dir_all(&child_workspace).expect("create guarded child workspace");
    fs::create_dir_all(&empty_workspace).expect("create child workspace without gate");
    fs::create_dir_all(fixture.primary.join("spelling")).expect("create relative spelling hop");
    fs::write(child_workspace.join("workspace-gate.txt"), "ready\n")
        .expect("write gate only in explicit child workspace");
    assert!(
        !fixture.primary.join("workspace-gate.txt").exists(),
        "the caller checkout lacks the positive child's guarded file"
    );

    let parent = fixture.start_at_delegate();
    let explicit_argument = child_workspace.to_string_lossy().into_owned();
    let explicit_spawn = rtm_at(
        &fixture.primary,
        &[
            "spawn",
            "worker",
            "--run",
            &parent,
            "--workspace",
            &explicit_argument,
        ],
    );
    let explicit_text = combined(&explicit_spawn);
    assert!(
        explicit_spawn.status.success(),
        "ENSV-007: explicit --workspace spawn succeeds: {explicit_text}"
    );
    let explicit_child = minted_id(&explicit_spawn, "spawned run ");

    let relative_argument = "./spelling/../child-ws";
    let relative_child = spawn_worker(
        &fixture.primary,
        &parent,
        Some(relative_argument),
        "ENSV-007: relative --workspace spawn",
    );
    let inherited_child = spawn_worker(
        &fixture.primary,
        &parent,
        None,
        "ENSV-007: omitted --workspace spawn",
    );

    let entries = ledger_entries(&fixture.ledger_bytes(&parent));
    let explicit_entry = entry_for(&entries, &explicit_child);
    let relative_entry = entry_for(&entries, &relative_child);
    let inherited_entry = entry_for(&entries, &inherited_child);
    assert_recorded_workspace(explicit_entry, &child_workspace, "explicit path");
    assert_recorded_workspace(relative_entry, &child_workspace, "relative path");
    assert_eq!(
        normalize_workspace_text(field(explicit_entry, "workspace")),
        normalize_workspace_text(field(relative_entry, "workspace")),
        "ENSV-007: absolute and relative spellings record one canonical workspace string"
    );
    assert_recorded_workspace(
        inherited_entry,
        &fixture.primary,
        "inherited parent workspace",
    );

    // The guarded file exists only beneath `child-ws`, while `rtm step` runs
    // from the primary checkout. A caller-root implementation therefore cannot
    // advance this child.
    assert_child_advanced(&fixture, &explicit_child, "explicit workspace guard");

    // Conversely, the caller now has the guarded file while this child's bound
    // workspace deliberately does not. This is the negative discriminating
    // half: resolving from the caller would incorrectly advance it.
    fs::write(fixture.primary.join("workspace-gate.txt"), "ready\n")
        .expect("write gate only in the caller checkout");
    assert_child_advanced(&fixture, &inherited_child, "inherited workspace guard");
    let empty_argument = empty_workspace.to_string_lossy().into_owned();
    let empty_child = spawn_worker(
        &fixture.primary,
        &parent,
        Some(&empty_argument),
        "ENSV-007: child with gate-free workspace",
    );
    assert!(
        fixture.primary.join("workspace-gate.txt").is_file()
            && !empty_workspace.join("workspace-gate.txt").exists(),
        "the negative child lacks the guarded file even though the caller has it"
    );
    let roster_before_refused_step = roster_at(&fixture.primary);
    let ledger_before_refused_step = fixture.ledger_bytes(&parent);
    let empty_state_before = fs::read(fixture.run_dir(&empty_child).join("run.toml"))
        .expect("negative child State File is readable before its refused step");
    let refused_step = rtm_at(&fixture.primary, &["step", "--run", &empty_child]);
    let refused_text = combined(&refused_step);
    assert!(
        refused_text.contains("step refused") && refused_text.contains("workspace-gate.txt"),
        "ENSV-007: a child guard resolves in its recorded workspace and refuses by naming the missing file: {refused_text}"
    );
    assert_eq!(
        fs::read(fixture.run_dir(&empty_child).join("run.toml"))
            .expect("negative child State File remains readable"),
        empty_state_before,
        "ENSV-007: a refused workspace guard leaves the child State File byte-identical"
    );
    assert_eq!(
        roster_at(&fixture.primary),
        roster_before_refused_step,
        "ENSV-007: a refused workspace guard adds no Run to the roster"
    );
    assert_eq!(
        fixture.ledger_bytes(&parent),
        ledger_before_refused_step,
        "ENSV-007: a refused workspace guard leaves the parent ledger byte-identical"
    );

    let missing_argument = "./does-not-exist";
    assert_spawn_refusal(
        &fixture,
        &parent,
        &[
            "spawn",
            "worker",
            "--run",
            &parent,
            "--workspace",
            missing_argument,
        ],
        &["workspace", "does not exist", missing_argument],
        None,
        "nonexistent path",
    );

    let file_argument = "./not-a-directory.txt";
    fs::write(
        fixture.primary.join("not-a-directory.txt"),
        "not a directory\n",
    )
    .expect("write file-shaped workspace input");
    assert_spawn_refusal(
        &fixture,
        &parent,
        &[
            "spawn",
            "worker",
            "--run",
            &parent,
            "--workspace",
            file_argument,
        ],
        &["workspace", "is not a directory", file_argument],
        None,
        "file path",
    );

    assert_spawn_refusal(
        &fixture,
        &parent,
        &["spawn", "worker", "--run", &parent, "--workspace"],
        &["--workspace needs a directory path"],
        None,
        "missing --workspace value",
    );

    assert_spawn_refusal(
        &fixture,
        &parent,
        &[
            "spawn",
            "worker",
            "--run",
            &parent,
            "--workspace",
            &explicit_argument,
            "--workspace",
            &explicit_argument,
        ],
        &["--workspace given twice"],
        None,
        "duplicate --workspace",
    );

    let outside_workspace = fixture.sandbox.join("outside-workspace");
    fs::create_dir_all(&outside_workspace).expect("create workspace outside the repository");
    let outside_argument = outside_workspace.to_string_lossy().into_owned();
    assert_spawn_refusal(
        &fixture,
        &parent,
        &[
            "spawn",
            "worker",
            "--run",
            &parent,
            "--workspace",
            &outside_argument,
        ],
        &["workspace", "is outside"],
        Some(&outside_workspace),
        "outside-repository path",
    );
}

/// ENSV-013: real sibling Run IDs scope their evidence paths. The fixture
/// copies this repository's `.gitignore`; each receipt is then committed on a
/// distinct branch and an ordinary merge must retain both exact byte streams.
#[test]
fn parallel_sibling_receipts_are_run_scoped_and_merge_cleanly() {
    let fixture = GitFixture::new("sibling-receipts");
    let first_workspace = fixture.primary.join("sibling-a-ws");
    let second_workspace = fixture.primary.join("sibling-b-ws");
    fs::create_dir_all(&first_workspace).expect("create first sibling workspace");
    fs::create_dir_all(&second_workspace).expect("create second sibling workspace");

    let parent = fixture.start_at_delegate();
    let first_argument = first_workspace.to_string_lossy().into_owned();
    let first_spawn = rtm_at(
        &fixture.primary,
        &[
            "spawn",
            "worker",
            "--run",
            &parent,
            "--workspace",
            &first_argument,
        ],
    );
    let first_text = combined(&first_spawn);
    assert!(
        first_spawn.status.success(),
        "ENSV-013: first sibling --workspace spawn succeeds: {first_text}"
    );
    let first_child = minted_id(&first_spawn, "spawned run ");

    let second_argument = second_workspace.to_string_lossy().into_owned();
    let second_child = spawn_worker(
        &fixture.primary,
        &parent,
        Some(&second_argument),
        "ENSV-013: second sibling --workspace spawn",
    );
    assert_ne!(
        first_child, second_child,
        "ENSV-013: real sibling spawns mint distinct Run identifiers"
    );

    let entries = ledger_entries(&fixture.ledger_bytes(&parent));
    assert_recorded_workspace(
        entry_for(&entries, &first_child),
        &first_workspace,
        "first sibling workspace",
    );
    assert_recorded_workspace(
        entry_for(&entries, &second_child),
        &second_workspace,
        "second sibling workspace",
    );

    let first_bytes = format!("run-id = {first_child:?}\nreceipt = \"sibling-a\"\n").into_bytes();
    let second_bytes = format!("run-id = {second_child:?}\nreceipt = \"sibling-b\"\n").into_bytes();
    let first_receipt = fixture
        .primary
        .join(".ratmac/evidence")
        .join(&first_child)
        .join("sibling-a.toml");
    let second_receipt = fixture
        .primary
        .join(".ratmac/evidence")
        .join(&second_child)
        .join("sibling-b.toml");
    assert_ne!(
        first_receipt, second_receipt,
        "ENSV-013: sibling receipt paths are disjoint"
    );
    assert_ne!(
        first_receipt.parent(),
        second_receipt.parent(),
        "ENSV-013: sibling receipts occupy distinct Run-scoped evidence directories"
    );

    let base_branch = current_branch(&fixture.primary);
    git_success(&fixture.primary, &["checkout", "-b", "receipt-sibling-a"]);
    let written_first = write_receipt(&fixture.primary, &first_child, "sibling-a", &first_bytes);
    assert_eq!(
        written_first, first_receipt,
        "ENSV-013: first receipt path is .ratmac/evidence/<first child id>/..."
    );
    let first_relative = receipt_relative(&fixture.primary, &first_receipt);
    assert_not_ignored_by_git(&fixture.primary, &first_relative);
    git_success(&fixture.primary, &["add", "--", &first_relative]);
    git_success(
        &fixture.primary,
        &["commit", "-m", "record first sibling receipt"],
    );

    git_success(&fixture.primary, &["checkout", &base_branch]);
    git_success(&fixture.primary, &["checkout", "-b", "receipt-sibling-b"]);
    let written_second = write_receipt(&fixture.primary, &second_child, "sibling-b", &second_bytes);
    assert_eq!(
        written_second, second_receipt,
        "ENSV-013: second receipt path is .ratmac/evidence/<second child id>/..."
    );
    let second_relative = receipt_relative(&fixture.primary, &second_receipt);
    assert_not_ignored_by_git(&fixture.primary, &second_relative);
    git_success(&fixture.primary, &["add", "--", &second_relative]);
    git_success(
        &fixture.primary,
        &["commit", "-m", "record second sibling receipt"],
    );

    git_success(&fixture.primary, &["checkout", "receipt-sibling-a"]);
    let merge = git(
        &fixture.primary,
        &["merge", "--no-edit", "receipt-sibling-b"],
    );
    let merge_text = combined(&merge);
    assert!(
        merge.status.success(),
        "ENSV-013: sibling receipt branches merge without conflict: {merge_text}"
    );
    assert!(
        !merge_text.to_ascii_lowercase().contains("conflict"),
        "ENSV-013: merge reports no conflict: {merge_text}"
    );
    let unmerged = git_text(&fixture.primary, &["ls-files", "-u"]);
    assert!(
        unmerged.trim().is_empty(),
        "ENSV-013: Git reports no unmerged receipt entry after the merge: {unmerged}"
    );
    assert_eq!(
        fs::read(&first_receipt).expect("merged first receipt is readable"),
        first_bytes,
        "ENSV-013: the merged first sibling receipt retains its exact bytes"
    );
    assert_eq!(
        fs::read(&second_receipt).expect("merged second receipt is readable"),
        second_bytes,
        "ENSV-013: the merged second sibling receipt retains its exact bytes"
    );
    assert_eq!(
        git_text(
            &fixture.primary,
            &["ls-files", "--error-unmatch", "--", &first_relative],
        )
        .trim(),
        first_relative,
        "ENSV-013: the merged first receipt remains tracked"
    );
    assert_eq!(
        git_text(
            &fixture.primary,
            &["ls-files", "--error-unmatch", "--", &second_relative],
        )
        .trim(),
        second_relative,
        "ENSV-013: the merged second receipt remains tracked"
    );
}
