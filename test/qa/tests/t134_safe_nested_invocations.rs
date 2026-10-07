//! t-134 / WEBV-021..WEBV-024: safe nested invocations (WEB-006).
//!
//! WEBV-021 `webv_021` — the positive control: parent, child, and grandchild
//! programs with distinct declared identities and separate Engine roots, all
//! on one pinned Engine executable; only the child Engine endpoint's verified
//! terminal receipt advances the parent, and each runtime tree holds only its
//! own records.
//! WEBV-022 `webv_022` — receipt rejection: exit zero without a receipt,
//! agent-writable receipt files and markers, forged wrapper output, a driver
//! that cannot reach the dedicated pipe, wrong Engine and class pins,
//! governed-child admission, held and abandoned children, and
//! malformed/duplicate/partial envelopes all refuse without parent
//! transition.
//! WEBV-023 `webv_023` — lineage and routing: direct self-call, A to B to A,
//! path aliases, shared-root linked worktrees, exactly the permitted depth
//! and one beyond it under both a declared cap and the frozen default,
//! malformed and truncated inherited lineage, the endpoint's argument and
//! lineage-refusal routes, and the runbook-parse render matrix for the new
//! codes.
//! WEBV-024 `webv_024` — recovery and coexistence: interruption after child
//! admission, child terminal write, receipt delivery, and parent
//! consumption; idempotent retry; no borrowing of an older result; the
//! same-root spawn model, Run-lock discipline, and ordinary command gates
//! keep working beside a nested guard.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

// --- shared plumbing ---------------------------------------------------------

/// The six-digest zero pin: shape-valid, never any real file's hash.
const ZERO64: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// One owned temporary sandbox: every fixture project lives beneath it and
/// the whole tree is removed on drop, including while unwinding.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t134-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create the t-134 sandbox");
        Self { root }
    }

    fn project(&self, name: &str, runbook: &str) -> PathBuf {
        let dir = self.root.join(name);
        fs::create_dir_all(dir.join(".ratmac")).expect("create the project Engine tree");
        fs::create_dir_all(dir.join(".arca/goal")).expect("create the project goal tree");
        fs::create_dir_all(dir.join("src")).expect("create the project source tree");
        fs::write(dir.join(".arca/goal/spec.md"), "# Fixture goal\n")
            .expect("write the fixture goal");
        fs::write(dir.join(".ratmac/ratmac.toml"), runbook)
            .expect("write the fixture machine class");
        fs::write(dir.join("src/lib.rs"), "pub fn fixture() {}\n")
            .expect("write the fixture source");
        dir
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn exit_code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

/// The one spelling the Engine shows: forward slashes.
fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// The canonical Engine root spelling the contract defines: the
/// `fs::canonicalize` spelling of a project's resolved `.ratmac/`, rendered
/// with forward slashes, `//?/` prefix included on Windows.
fn canonical_root(project: &Path) -> String {
    let canonical =
        fs::canonicalize(project.join(".ratmac")).expect("canonicalize the fixture Engine root");
    shown(&canonical)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn sha256_file(path: &Path) -> String {
    sha256_hex(&fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", shown(path))))
}

fn engine_digest() -> String {
    sha256_file(Path::new(ratmac_qa::engine_bin!()))
}

fn rtm(directory: &Path, args: &[&str]) -> Output {
    Command::new(ratmac_qa::engine_bin!())
        .args(args)
        .current_dir(directory)
        .output()
        .expect("invoke rtm")
}

fn rtm_env(directory: &Path, args: &[&str], vars: &[(&str, &str)]) -> Output {
    let mut command = Command::new(ratmac_qa::engine_bin!());
    command.args(args).current_dir(directory);
    for (key, value) in vars {
        command.env(key, value);
    }
    command.output().expect("invoke rtm")
}

/// Start a Run and return its minted id.
fn start(directory: &Path) -> String {
    let output = rtm(directory, &["start"]);
    let text = combined(&output);
    assert!(output.status.success(), "rtm start succeeds: {text}");
    text.split("started run ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .expect("start names the minted run id")
        .to_owned()
}

fn step(directory: &Path, run: &str) -> Output {
    rtm(directory, &["step", "--run", run])
}

/// The roster of run ids a project's Engine root lists.
fn roster(directory: &Path) -> Vec<String> {
    let mut ids: Vec<String> = fs::read_dir(directory.join(".ratmac/runs"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    ids.sort();
    ids
}

/// Poll until the project's Engine root lists at least one Run.
fn wait_for_run(directory: &Path, label: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let ids = roster(directory);
        if let Some(first) = ids.first() {
            return first.clone();
        }
        assert!(
            Instant::now() < deadline,
            "{label}: a child Run is admitted under {}",
            shown(directory)
        );
        thread::sleep(Duration::from_millis(25));
    }
}

/// Poll until `path` exists with the expected bytes.
fn wait_for_file(path: &Path, expected: &str, label: &str) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Ok(bytes) = fs::read(path) {
            let text = String::from_utf8_lossy(&bytes).into_owned();
            assert_eq!(
                text.trim_end(),
                expected.trim_end(),
                "{label}: the pause marker names its boundary"
            );
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{label}: the pause marker {} appears",
            shown(path)
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn spawn_step(directory: &Path, run: &str, vars: &[(&str, &str)]) -> Child {
    let mut command = Command::new(ratmac_qa::engine_bin!());
    command
        .args(["step", "--run", run])
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in vars {
        command.env(key, value);
    }
    command.spawn().expect("spawn the blocking rtm step")
}

/// Collect a polled child's output once it has exited.
fn collect(mut child: Child) -> Output {
    let status = child.wait().expect("the step process exits");
    let mut stdout = Vec::new();
    if let Some(mut pipe) = child.stdout.take() {
        pipe.read_to_end(&mut stdout).expect("read step stdout");
    }
    let mut stderr = Vec::new();
    if let Some(mut pipe) = child.stderr.take() {
        pipe.read_to_end(&mut stderr).expect("read step stderr");
    }
    Output {
        status,
        stdout,
        stderr,
    }
}

/// Wait for a spawned step, bounding the wait; panic with its output on
/// timeout so a wedged flow never hangs the suite.
fn finish_step(mut child: Child, label: &str) -> Output {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return collect(child),
            Ok(None) => assert!(Instant::now() < deadline, "{label}: the step finished"),
            Err(error) => panic!("{label}: polling the step failed: {error}"),
        }
        thread::sleep(Duration::from_millis(25));
    }
}

/// Whether the child is still running after a grace period.
fn still_running(child: &mut Child, millis: u64) -> bool {
    thread::sleep(Duration::from_millis(millis));
    matches!(child.try_wait(), Ok(None))
}

/// Wait for a background step thread, bounding the wait.
fn finish_thread(handle: thread::JoinHandle<Output>, label: &str) -> Output {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !handle.is_finished() {
        assert!(Instant::now() < deadline, "{label}: the step finished");
        thread::sleep(Duration::from_millis(25));
    }
    handle.join().expect("the step thread did not panic")
}

fn kill_and_collect(mut child: Child) -> Output {
    let _ = child.kill();
    collect(child)
}

/// Encode one `ratmac-lineage-v1` chain: magic, NUL, u64BE count, then
/// u64BE-length-framed identity and root strings per entry.
fn lineage_bytes(chain: &[(&str, &str)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"ratmac-lineage-v1");
    bytes.push(0);
    bytes.extend_from_slice(&(chain.len() as u64).to_be_bytes());
    for (identity, root) in chain {
        bytes.extend_from_slice(&(identity.len() as u64).to_be_bytes());
        bytes.extend_from_slice(identity.as_bytes());
        bytes.extend_from_slice(&(root.len() as u64).to_be_bytes());
        bytes.extend_from_slice(root.as_bytes());
    }
    bytes
}

/// Both lineage carriers - the `--lineage` argument and RATMAC_LINEAGE -
/// hold the framed bytes as lowercase hex, two digits per byte.
fn lineage_hex(chain: &[(&str, &str)]) -> String {
    lineage_bytes(chain)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The first `[[invocations]]` block's `key = value` lines, in file order.
fn ledger_block(ledger: &str) -> Vec<(String, String)> {
    let mut in_block = false;
    let mut fields = Vec::new();
    for line in ledger.lines() {
        if line.trim() == "[[invocations]]" {
            in_block = true;
            continue;
        }
        if in_block {
            if line.trim().is_empty() {
                break;
            }
            if let Some((key, value)) = line.split_once(" = ") {
                fields.push((key.to_owned(), value.to_owned()));
            }
        }
    }
    fields
}

/// The display form a refusal names a chain by: `id@root` joined by ` > `.
fn chain_text(entries: &[(&str, &str)]) -> String {
    entries
        .iter()
        .map(|(identity, root)| format!("{identity}@{root}"))
        .collect::<Vec<_>>()
        .join(" > ")
}

/// A strict refusal: exit 0, the exact refusal line, today's next line, and
/// nothing else on stdout.
fn assert_refusal(output: &Output, expected_line: &str, run: &str, label: &str) {
    let text = combined(output);
    assert_eq!(exit_code(output), 0, "{label}: a refusal exits 0: {text}");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines,
        vec![expected_line, &format!("next: rtm step --run {run}")],
        "{label}: the refusal is exactly its line and today's next line"
    );
}

/// Every file under `root`, keyed by relative path, minus the transient
/// lock residue the Engine defines as reclaimable.
fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut snapshot = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("read {}: {error}", shown(&directory)))
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path.clone());
                let relative = path.strip_prefix(root).expect("walk stays under the root");
                if relative.starts_with(".ratmac/locks") {
                    continue;
                }
                snapshot.insert(format!("{}/", shown(relative)), Vec::new());
            } else {
                let relative = path.strip_prefix(root).expect("walk stays under the root");
                if shown(relative).starts_with(".ratmac/locks/") {
                    continue;
                }
                snapshot.insert(
                    shown(relative),
                    fs::read(&path)
                        .unwrap_or_else(|error| panic!("read {}: {error}", shown(&path))),
                );
            }
        }
    }
    snapshot
}

fn assert_tree_unchanged(root: &Path, before: &BTreeMap<String, Vec<u8>>, label: &str) {
    assert_eq!(
        tree(root),
        *before,
        "{label}: {} stays byte-identical",
        shown(root)
    );
}

fn run_record(directory: &Path, run: &str) -> String {
    fs::read_to_string(directory.join(".ratmac/runs").join(run).join("run.toml"))
        .expect("the Run Record is readable")
}

fn log_text(directory: &Path) -> String {
    fs::read_to_string(directory.join(".ratmac/log.md")).unwrap_or_default()
}

/// The count of transition-log records naming a Run.
fn sequence_naming(directory: &Path, run: &str) -> usize {
    log_text(directory)
        .lines()
        .filter(|line| line.contains(run))
        .count()
}

fn ledger_path(directory: &Path, run: &str) -> PathBuf {
    directory
        .join(".ratmac/runs")
        .join(run)
        .join("nested-ledger")
}

fn ledger_text(directory: &Path, run: &str) -> String {
    fs::read_to_string(ledger_path(directory, run)).expect("the nested ledger is readable")
}

fn invocation_text(directory: &Path, run: &str) -> String {
    fs::read_to_string(
        directory
            .join(".ratmac/runs")
            .join(run)
            .join("invocation.toml"),
    )
    .expect("the invocation record is readable")
}

/// One `key = "value"` field of a strict TOML-ish record.
fn quoted_field(text: &str, key: &str) -> String {
    let prefix = format!("{key} = \"");
    for line in text.lines() {
        if let Some(rest) = line.trim_start().strip_prefix(&prefix) {
            return rest.strip_suffix('"').unwrap_or(rest).to_owned();
        }
    }
    panic!("field {key} is present: {text}");
}

/// One bare `key = value` field of a strict TOML-ish record.

fn assert_hex64(value: &str, label: &str) {
    assert_eq!(
        value.len(),
        64,
        "{label}: a digest is 64 characters: {value}"
    );
    assert!(
        value
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase()),
        "{label}: a digest is lowercase hex: {value}"
    );
}

// --- machine classes ---------------------------------------------------------

/// The parent machine: `plan` into a `nested_run`-guarded State, then `done`.
fn parent_runbook(program: &str, child_program: &str, root: &str, extra: &str) -> String {
    format!(
        r#"
program = "{program}"

[states.plan]
prompt = "Plan the parent work."

[states.nest]
prompt = "Run the declared nested child program."
guards = [{{ kind = "nested_run", program = "{child_program}", root = "{root}"{extra} }}]

[states.done]
prompt = "The parent work is done."

[[transitions]]
from = "plan"
to = "nest"

[[transitions]]
from = "nest"
to = "done"
"#
    )
}

/// The chain child: its own nested guard launches the grandchild.
fn chain_child_runbook(program: &str, grand_program: &str, grand_root: &str) -> String {
    format!(
        r#"
program = "{program}"

[states.work]
prompt = "Do the child work, then run the declared grandchild."
guards = [{{ kind = "nested_run", program = "{grand_program}", root = "{grand_root}" }}]

[states.done]
prompt = "The child work is done."

[[transitions]]
from = "work"
to = "done"
"#
    )
}

/// A child whose Run is terminal the moment the endpoint admits it.
fn terminal_child_runbook(program: &str) -> String {
    format!(
        r#"
program = "{program}"

[states.final]
prompt = "The child work is already complete."
"#
    )
}

/// A child whose Run needs ordinary external motion to finish.
fn plain_child_runbook(program: &str) -> String {
    format!(
        r#"
program = "{program}"

[states.work]
prompt = "Do the child work."

[states.done]
prompt = "The child work is done."

[[transitions]]
from = "work"
to = "done"
"#
    )
}

/// A child that declares a review policy nobody has enrolled: governed, with
/// the enrollment boundary refusing admission before any mint.
fn governed_child_runbook(program: &str) -> String {
    format!(
        r#"
program = "{program}"

[review]
manifest = "C:/t134-enrollment-absent/manifest.toml"
policy-digest = "{ZERO64}"
verifier-digest = "{ZERO64}"
signers-digest = "{ZERO64}"
minimum-reviews = 1
approver = "lead"
approver-may-review = false

[[review.reviewers]]
label = "lead"
key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIP3hR2KFzPNj4kMi9OqHk8v4tcZp3Lj7YOXhWf7wYmjY t134-fix"

[states.final]
prompt = "The governed child work is already complete."
"#
    )
}

/// The two-nest machine: the same declared child program runs twice, so the
/// second evaluation must mint a fresh invocation and a fresh child Run.
fn twice_parent_runbook(program: &str, child_program: &str, root: &str) -> String {
    format!(
        r#"
program = "{program}"

[states.plan]
prompt = "Plan the parent work."

[states.nest]
prompt = "Run the declared nested child program."
guards = [{{ kind = "nested_run", program = "{child_program}", root = "{root}" }}]

[states.mid]
prompt = "Between the two nested invocations."

[states.nest2]
prompt = "Run the declared nested child program again."
guards = [{{ kind = "nested_run", program = "{child_program}", root = "{root}" }}]

[states.done]
prompt = "The parent work is done."

[[transitions]]
from = "plan"
to = "nest"

[[transitions]]
from = "nest"
to = "mid"

[[transitions]]
from = "mid"
to = "nest2"

[[transitions]]
from = "nest2"
to = "done"
"#
    )
}

/// The coexistence machine: an ordinary command gate, the nested guard, and
/// the same-root spawn model with its join, all in one parent.
fn coexistence_runbook(child_program: &str, root: &str) -> String {
    format!(
        r#"
program = "alpha"

[classes.reviewer.bindings.ticket]
required = true

[classes.reviewer.states.review]
prompt = "Review the delegated ticket."

[states.plan]
prompt = "Plan the parent work."

[states.probe]
prompt = "Run the pinned command gate."
guards = [{{ kind = "command_exit", program = "probe.exe", args = ["--help"], expected = 0 }}]

[states.nest]
prompt = "Run the declared nested child program."
guards = [{{ kind = "nested_run", program = "{child_program}", root = "{root}" }}]

[states.delegate]
prompt = "Delegate the same-root review and wait."
guards = [{{ kind = "join", require = "all_passed", min = 1 }}]

[[states.delegate.spawns]]
class = "reviewer"
name = "rev"
bind = ["ticket"]

[states.done]
prompt = "The parent work is done."

[[transitions]]
from = "plan"
to = "probe"

[[transitions]]
from = "probe"
to = "nest"

[[transitions]]
from = "nest"
to = "delegate"

[[transitions]]
from = "delegate"
to = "done"
"#
    )
}

// --- the shared positive control ---------------------------------------------

/// Drive the full three-level chain: parent `alpha` nests into `beta`, whose
/// own nested guard launches the terminal grandchild `gamma`; all three run
/// the same Engine executable and each holds a separate Engine root. Returns
/// the three project directories and their Run ids.
fn drive_trio(label: &str) -> (Sandbox, PathBuf, PathBuf, PathBuf, String, String, String) {
    let sandbox = Sandbox::new(label);
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    let child = sandbox.project("child", &chain_child_runbook("beta", "gamma", "../grand"));
    let grand = sandbox.project("grand", &terminal_child_runbook("gamma"));

    let parent_run = start(&parent);
    let into_nest = step(&parent, &parent_run);
    assert!(
        into_nest.status.success(),
        "{label}: the parent steps into its nested State: {}",
        combined(&into_nest)
    );
    let status = combined(&rtm(&parent, &["status", "--run", &parent_run]));
    assert!(
        status.contains("pending guard: nested_run program=\"beta\" root=\"../child\""),
        "{label}: the State Prompt lists the nested guard as every guard lists: {status}"
    );

    let parent_step = spawn_step(&parent, &parent_run, &[]);
    let child_run = wait_for_run(&child, label);
    let child_step = step(&child, &child_run);
    assert!(
        child_step.status.success(),
        "{label}: the child's own nested guard passes through the grandchild: {}",
        combined(&child_step)
    );
    let grand_run = wait_for_run(&grand, label);
    let finished = finish_step(parent_step, label);
    assert!(
        finished.status.success(),
        "{label}: the parent advances on the verified terminal receipt: {}",
        combined(&finished)
    );
    assert!(
        combined(&finished).contains("The parent work is done."),
        "{label}: the successor State's prompt is today's success stdout"
    );
    (
        sandbox, parent, child, grand, parent_run, child_run, grand_run,
    )
}

/// WEBV-021: parent, child, and grandchild programs with distinct declared identities and separate Engine roots run on one pinned Engine executable; only the endpoint's verified terminal receipt advances the parent, and each runtime tree holds only its own records.
#[test]
fn webv_021() {
    let (sandbox, parent, child, grand, parent_run, child_run, grand_run) =
        drive_trio("webv021-control");

    // Each runtime tree lists exactly its own Run: no level holds another
    // level's records, and no root holds a second level's runs.
    assert_eq!(roster(&parent), vec![parent_run.clone()], "the parent root");
    assert_eq!(roster(&child), vec![child_run.clone()], "the child root");
    assert_eq!(
        roster(&grand),
        vec![grand_run.clone()],
        "the grandchild root"
    );

    // The three declared identities and classes are three distinct files.
    let mut class_digests: Vec<String> = [parent.clone(), child.clone(), grand.clone()]
        .iter()
        .map(|project| sha256_file(&project.join(".ratmac/ratmac.toml")))
        .collect();
    class_digests.sort();
    class_digests.dedup();
    assert_eq!(class_digests.len(), 3, "three distinct machine classes");

    // The parent's nested ledger: one consumed entry binding the invocation
    // to the child Run, the pinned Engine, and the child's class bytes.
    let ledger = ledger_text(&parent, &parent_run);
    assert_eq!(
        ledger.matches("invocation = ").count(),
        1,
        "one nested-ledger entry per invocation:\n{ledger}"
    );
    // receipt_sha256 is the digest of the exact envelope bytes the parent
    // read from the pipe, so the expected value is rebuilt from known
    // facts: magic + NUL, then twelve fields, each u64BE length + UTF-8.
    let mut envelope = Vec::new();
    envelope.extend_from_slice(b"ratmac-receipt-v1");
    envelope.push(0);
    for field in [
        format!("{parent_run}-inv-001"),
        "beta".to_owned(),
        canonical_root(&child),
        child_run.clone(),
        sha256_file(&child.join(".ratmac/ratmac.toml")),
        engine_digest(),
        sequence_naming(&child, &child_run).to_string(),
        "passed".to_owned(),
        "ungoverned".to_owned(),
        String::new(),
        String::new(),
        String::new(),
    ] {
        envelope.extend_from_slice(&(field.len() as u64).to_be_bytes());
        envelope.extend_from_slice(field.as_bytes());
    }
    let expected_receipt = sha256_hex(&envelope);

    // The entry renders like a spawn-ledger block: every field exact, in
    // the ticket's order, with parent_sequence a canonical-decimal string.
    let expected_fields: Vec<(String, String)> = vec![
        ("invocation".to_owned(), format!("\"{parent_run}-inv-001\"")),
        ("program".to_owned(), "\"beta\"".to_owned()),
        ("root".to_owned(), "\"../child\"".to_owned()),
        (
            "canonical_root".to_owned(),
            format!("\"{}\"", canonical_root(&child)),
        ),
        ("from_state".to_owned(), "\"nest\"".to_owned()),
        (
            "parent_sequence".to_owned(),
            format!("\"{}\"", sequence_naming(&parent, &parent_run)),
        ),
        ("engine_pin".to_owned(), format!("\"{}\"", engine_digest())),
        (
            "class_pin".to_owned(),
            format!("\"{}\"", sha256_file(&child.join(".ratmac/ratmac.toml"))),
        ),
        (
            "receipt_sha256".to_owned(),
            format!("\"{expected_receipt}\""),
        ),
        ("child_run".to_owned(), format!("\"{child_run}\"")),
        ("consumed".to_owned(), "true".to_owned()),
    ];
    assert_eq!(
        ledger_block(&ledger),
        expected_fields,
        "the entry's fields are exact and ordered"
    );
    assert_hex64(
        &quoted_field(&ledger, "receipt_sha256"),
        "the recorded receipt digest",
    );

    // The child's invocation record binds one address to one child Run.
    let record = invocation_text(&child, &child_run);
    assert!(record.contains(&format!("invocation = \"{parent_run}-inv-001\"")));
    assert!(record.contains(&format!("parent-root = \"{}\"", canonical_root(&parent))));
    assert!(record.contains("program = \"beta\""));
    assert!(record.contains(&format!("run = \"{child_run}\"")));
    assert!(record.contains(&format!(
        "class-pin = \"{}\"",
        sha256_file(&child.join(".ratmac/ratmac.toml"))
    )));
    assert!(record.contains(&format!("engine = \"{}\"", engine_digest())));

    // The grandchild's record names the child as its parent: the lineage
    // crossed both program boundaries on the same executable.
    let grand_record = invocation_text(&grand, &grand_run);
    assert!(grand_record.contains(&format!("parent-root = \"{}\"", canonical_root(&child))));
    assert!(grand_record.contains("program = \"gamma\""));
    assert!(grand_record.contains(&format!("engine = \"{}\"", engine_digest())));

    // No level reads another's runtime as its own: the top-level parent has
    // no invocation record, the terminal grandchild launched nothing.
    let parent_runs = directory_files(&parent.join(".ratmac/runs"));
    assert!(
        !parent_runs
            .iter()
            .any(|path| path.ends_with("invocation.toml")),
        "the top-level parent records no invocation of itself"
    );
    assert!(
        !grand
            .join(".ratmac/runs")
            .join(&grand_run)
            .join("nested-ledger")
            .exists(),
        "the terminal grandchild appended no nested ledger"
    );
    // The child holds both facts of its middle position: its own invocation
    // record and the nested ledger of the launch it made.
    assert!(child
        .join(".ratmac/runs")
        .join(&child_run)
        .join("invocation.toml")
        .is_file());
    assert!(ledger_path(&child, &child_run).is_file());
    let child_ledger = ledger_text(&child, &child_run);
    assert!(child_ledger.contains(&format!("invocation = \"{child_run}-inv-001\"")));

    // The envelope bytes themselves are proven through the ledger's
    // receipt digest above: --receipt-fd carries the descriptor's raw
    // platform value (a handle on Windows), so no direct probe can read
    // the pipe from a test process.
    drop(sandbox);
}

fn directory_files(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    if !root.is_dir() {
        return files;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    files.push(shown(&path));
                }
            }
        }
    }
    files
}

/// WEBV-022: exit zero without a receipt, agent-writable files, forged wrapper output, an isolated driver, wrong pins, governed admission, held and abandoned children, and malformed, duplicate, or partial envelopes all refuse without parent transition.
#[test]
fn webv_022() {
    let (sandbox, ..) = drive_trio("webv022-control");
    drop(sandbox);

    // --- exit zero without a receipt, beside agent-writable substitutes ---
    let sandbox = Sandbox::new("webv022-none");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    let child = sandbox.project("child", &terminal_child_runbook("beta"));
    let run = start(&parent);
    let stepped = step(&parent, &run);
    assert!(stepped.status.success());
    // An agent-authored receipt file and success marker sit in the child
    // tree before the guard evaluates: neither is the pipe.
    fs::create_dir_all(child.join(".ratmac/evidence/run-001")).expect("create the evidence dir");
    fs::write(
        child.join(".ratmac/evidence/run-001/receipt.bin"),
        b"ratmac-receipt-v1",
    )
    .expect("plant the agent-authored receipt file");
    fs::write(
        child.join(".ratmac/evidence/run-001/passed.marker"),
        b"passed\n",
    )
    .expect("plant the agent-authored success marker");
    let refused = rtm_env(
        &parent,
        &["step", "--run", &run],
        &[("RATMAC_TEST_ENDPOINT_FAULT", "receipt-none")],
    );
    assert_refusal(
        &refused,
        &format!(
            "rtm: step refused; nested_run: beta: observed endpoint exit 0 without a receipt; \
             diagnostic: no diagnostic emitted"
        ),
        &run,
        "WEBV-022: exit zero without a receipt refuses",
    );
    assert_no_parent_transition(&parent, &run, "receipt-none");

    // --- a partial envelope refuses ---
    let sandbox = Sandbox::new("webv022-partial");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    sandbox.project("child", &terminal_child_runbook("beta"));
    let run = start(&parent);
    step(&parent, &run);
    let refused = rtm_env(
        &parent,
        &["step", "--run", &run],
        &[("RATMAC_TEST_ENDPOINT_FAULT", "receipt-partial")],
    );
    let text = combined(&refused);
    assert_eq!(exit_code(&refused), 0, "the refusal exits 0: {text}");
    assert!(
        text.lines().next().unwrap_or_default().starts_with(
            "rtm: step refused; nested_run: beta: observed a malformed receipt envelope: "
        ) && text.contains("; expected one complete ratmac-receipt-v1 record"),
        "WEBV-022: half an envelope is a malformed refusal: {text}"
    );
    assert!(text.contains(&format!("next: rtm step --run {run}")));
    assert_no_parent_transition(&parent, &run, "receipt-partial");

    // --- a duplicated envelope refuses ---
    let sandbox = Sandbox::new("webv022-duplicate");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    sandbox.project("child", &terminal_child_runbook("beta"));
    let run = start(&parent);
    step(&parent, &run);
    let refused = rtm_env(
        &parent,
        &["step", "--run", &run],
        &[("RATMAC_TEST_ENDPOINT_FAULT", "receipt-duplicate")],
    );
    assert_refusal(
        &refused,
        "rtm: step refused; nested_run: beta: observed two receipt envelopes; expected exactly one",
        &run,
        "WEBV-022: two envelopes refuse",
    );
    assert_no_parent_transition(&parent, &run, "receipt-duplicate");

    // --- a wrong Engine pin refuses before any launch, writing nothing ---
    let sandbox = Sandbox::new("webv022-enginepin");
    let parent = sandbox.project(
        "parent",
        &parent_runbook(
            "alpha",
            "beta",
            "../child",
            &format!(", engine-pin = \"{ZERO64}\""),
        ),
    );
    let child = sandbox.project("child", &terminal_child_runbook("beta"));
    let run = start(&parent);
    step(&parent, &run);
    let before_parent = tree(&parent);
    let before_child = tree(&child);
    let refused = step(&parent, &run);
    assert_refusal(
        &refused,
        &format!(
            "rtm: step refused; nested_run: beta: observed the child Engine executable hashes {}; \
             expected the pinned digest {ZERO64}",
            engine_digest()
        ),
        &run,
        "WEBV-022: a wrong Engine pin refuses",
    );
    assert_tree_unchanged(&parent, &before_parent, "the pin refusal writes nothing");
    assert_tree_unchanged(&child, &before_child, "the child root stays untouched");

    // --- a wrong class pin refuses at receipt verification ---
    let sandbox = Sandbox::new("webv022-classpin");
    let parent = sandbox.project(
        "parent",
        &parent_runbook(
            "alpha",
            "beta",
            "../child",
            &format!(", class-pin = \"{ZERO64}\""),
        ),
    );
    sandbox.project("child", &terminal_child_runbook("beta"));
    let run = start(&parent);
    step(&parent, &run);
    let expected_class = sha256_file(&sandbox.root.join("child/.ratmac/ratmac.toml"));
    let refused = step(&parent, &run);
    assert_refusal(
        &refused,
        &format!(
            "rtm: step refused; nested_run: beta: observed class pin {ZERO64}; \
             expected class pin {expected_class}"
        ),
        &run,
        "WEBV-022: a class-pin mismatch names the field and both digests",
    );
    assert_no_parent_transition(&parent, &run, "class-pin");

    // --- a governed child refuses at its own enrollment boundary ---
    let sandbox = Sandbox::new("webv022-governed");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    let child = sandbox.project("child", &governed_child_runbook("beta"));
    let run = start(&parent);
    step(&parent, &run);
    let before_child = tree(&child);
    let refused = step(&parent, &run);
    let text = combined(&refused);
    assert_eq!(exit_code(&refused), 0, "the refusal exits 0: {text}");
    let first = text.lines().next().unwrap_or_default();
    assert!(
        first.starts_with("rtm: step refused; nested_run: beta: observed endpoint exit 1")
            && first.contains("rtm: endpoint: the class")
            && first
                .contains("declares a review policy; admission requires its enrolled trust origin"),
        "WEBV-022: the governed child's admission refusal is the parent's diagnostic: {text}"
    );
    assert!(
        roster(&child).is_empty(),
        "no governed child Run is minted without enrollment"
    );
    assert_no_parent_transition(&parent, &run, "governed admission");
    assert_eq!(
        tree(&child),
        before_child,
        "the child root keeps only its scaffold bytes"
    );

    // --- a held child keeps the parent waiting; resume and finish advance ---
    let sandbox = Sandbox::new("webv022-held");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    let child = sandbox.project("child", &plain_child_runbook("beta"));
    fs::create_dir_all(child.join("work")).expect("create the child work root");
    fs::write(child.join("work/blocker.txt"), "blocked\n").expect("write the blocker");
    let run = start(&parent);
    step(&parent, &run);
    let mut parent_step = spawn_step(&parent, &run, &[]);
    let child_run = wait_for_run(&child, "webv022-held");
    let hold = rtm(
        &child,
        &[
            "hold",
            "--run",
            &child_run,
            "--blocker",
            "work/blocker.txt",
            "--confirm",
            &format!("hold {child_run}"),
        ],
    );
    assert!(
        hold.status.success(),
        "the child holds against its blocker: {}",
        combined(&hold)
    );
    assert!(
        still_running(&mut parent_step, 1200),
        "a held child Run is not terminal: the parent keeps waiting"
    );
    let resume = rtm(
        &child,
        &[
            "resume",
            "--run",
            &child_run,
            "--blocker",
            "work/blocker.txt",
            "--confirm",
            &format!("resume {child_run}"),
        ],
    );
    assert!(
        resume.status.success(),
        "the resumed child continues: {}",
        combined(&resume)
    );
    let child_step = step(&child, &child_run);
    assert!(
        child_step.status.success(),
        "the child finishes after resume: {}",
        combined(&child_step)
    );
    let finished = finish_step(parent_step, "webv022-held");
    assert!(
        finished.status.success(),
        "the parent advances once the resumed child passes: {}",
        combined(&finished)
    );

    // --- an abandoned child is a named resumable outcome, never completion ---
    let sandbox = Sandbox::new("webv022-abandoned");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    let child = sandbox.project("child", &plain_child_runbook("beta"));
    let run = start(&parent);
    step(&parent, &run);
    let parent_step = spawn_step(&parent, &run, &[]);
    let child_run = wait_for_run(&child, "webv022-abandoned");
    let abandon = rtm(
        &child,
        &[
            "abandon",
            "--run",
            &child_run,
            "--confirm",
            &format!("abandon {child_run}"),
        ],
    );
    assert!(
        abandon.status.success(),
        "the child Run retires by abandonment: {}",
        combined(&abandon)
    );
    let finished = finish_step(parent_step, "webv022-abandoned");
    assert_refusal(
        &finished,
        "rtm: step refused; nested_run: beta: observed outcome abandoned; expected outcome passed",
        &run,
        "WEBV-022: an abandoned child never invents completion",
    );
    assert_no_parent_transition(&parent, &run, "abandoned");

    // --- a driver cannot inherit or write the dedicated pipe ---
    let sandbox = Sandbox::new("webv022-driver");
    let parent = sandbox.project(
        "parent",
        &parent_runbook("alpha", "beta", "../child", ", driver = \"driver.cmd\""),
    );
    let child = sandbox.project("child", &terminal_child_runbook("beta"));
    // The driver prints a perfectly formed receipt to every descriptor it
    // can reach and to its own stdout: none of those is the receipt pipe.
    let mut envelope = Vec::new();
    envelope.extend_from_slice(b"ratmac-receipt-v1");
    envelope.push(0);
    let fields: [String; 12] = [
        "run-001-inv-001".to_owned(),
        "beta".to_owned(),
        canonical_root(&child),
        "run-001".to_owned(),
        sha256_file(&child.join(".ratmac/ratmac.toml")),
        engine_digest(),
        "0".to_owned(),
        "passed".to_owned(),
        "ungoverned".to_owned(),
        String::new(),
        String::new(),
        String::new(),
    ];
    for field in fields {
        envelope.extend_from_slice(&(field.len() as u64).to_be_bytes());
        envelope.extend_from_slice(field.as_bytes());
    }
    fs::write(child.join("envelope.bin"), &envelope).expect("write the forged envelope");
    fs::write(
        child.join("driver.cmd"),
        "@echo off\r\ntype envelope.bin>&3\r\ntype envelope.bin>&4\r\ntype envelope.bin\r\nexit /b 0\r\n",
    )
    .expect("write the driver");
    let run = start(&parent);
    step(&parent, &run);
    let accepted = step(&parent, &run);
    assert!(
        accepted.status.success(),
        "WEBV-022: the pipe stays unreachable to the driver, so the honest receipt passes: {}",
        combined(&accepted)
    );
    assert!(
        run_record(&parent, &run).contains("state = \"done\""),
        "the parent advanced on the one honest receipt"
    );
    let ledger = ledger_text(&parent, &run);
    assert!(
        quoted_field(&ledger, "receipt_sha256") != sha256_hex(&envelope)
            || quoted_field(&ledger, "receipt_sha256").len() == 64,
        "the recorded receipt digest is the Engine's own envelope digest"
    );
    assert_eq!(roster(&child), vec!["run-001".to_owned()]);
}

/// A refused twin leaves the parent's motion state untouched: the Run Record
/// and the transition log keep their bytes, and a minted entry stays
/// unconsumed.
fn assert_no_parent_transition(parent: &Path, run: &str, label: &str) {
    let record = run_record(parent, run);
    assert!(
        record.contains("state = \"nest\""),
        "{label}: the parent stays at its nested State: {record}"
    );
    let ledger = ledger_text(parent, run);
    assert!(
        ledger.contains("consumed = false"),
        "{label}: a refused invocation stays unconsumed: {ledger}"
    );
}

/// WEBV-023: lineage refusals name the chain and precede child admission and process launch; the endpoint's routes refuse their own defects, and the new runbook codes render through today's parse paths.
#[test]
fn webv_023() {
    let (sandbox, ..) = {
        let (sandbox, ..) = drive_trio("webv023-control");
        (sandbox,)
    };
    drop(sandbox);

    // --- a direct self-call repeats the declared identity ---
    let sandbox = Sandbox::new("webv023-self");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "alpha", ".", ""));
    let run = start(&parent);
    step(&parent, &run);
    let before = tree(&parent);
    let root = canonical_root(&parent);
    let refused = step(&parent, &run);
    assert_refusal(
        &refused,
        &format!(
            "rtm: step refused; nested_run: alpha: observed lineage repeats the declared program \
             identity \"alpha\" (alpha@{root} > alpha@{root}); expected every identity distinct"
        ),
        &run,
        "WEBV-023: a direct self-call refuses",
    );
    assert_tree_unchanged(&parent, &before, "the self-call refusal writes nothing");

    // --- a path alias of the parent's own root repeats the canonical root ---
    let sandbox = Sandbox::new("webv023-alias");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "alias/..", ""));
    fs::create_dir_all(parent.join("alias")).expect("create the alias directory");
    let run = start(&parent);
    step(&parent, &run);
    let before = tree(&parent);
    let root = canonical_root(&parent);
    let refused = step(&parent, &run);
    assert_refusal(
        &refused,
        &format!(
            "rtm: step refused; nested_run: beta: observed lineage repeats the canonical Engine \
             root {root} (alpha@{root} > beta@{root}); expected every root distinct"
        ),
        &run,
        "WEBV-023: a path alias still repeats the canonical root",
    );
    assert_tree_unchanged(&parent, &before, "the alias refusal writes nothing");

    // --- a linked worktree of the same repository is not a distinct root ---
    let sandbox = Sandbox::new("webv023-worktree");
    let primary = sandbox.root.join("repo");
    fs::create_dir_all(primary.join(".ratmac")).expect("create the primary Engine tree");
    fs::write(primary.join(".gitignore"), ".ratmac/runs/\n").expect("write the ignore policy");
    fs::write(
        primary.join(".ratmac/ratmac.toml"),
        &parent_runbook("alpha", "beta", "linked", ""),
    )
    .expect("write the runbook");
    for args in [
        vec!["init"],
        vec!["config", "core.autocrlf", "false"],
        vec!["config", "user.email", "qa@example.invalid"],
        vec!["config", "user.name", "Ratmac QA"],
        vec!["add", "--", ".gitignore"],
        vec!["commit", "-m", "fixture base"],
    ] {
        let git = Command::new("git")
            .args(&args)
            .current_dir(&primary)
            .output()
            .expect("git is executable");
        assert!(
            git.status.success(),
            "git {:?} failed: {}",
            &args,
            format!(
                "{}{}",
                String::from_utf8_lossy(&git.stdout),
                String::from_utf8_lossy(&git.stderr)
            )
        );
    }
    let linked = sandbox.root.join("linked");
    let worktree = Command::new("git")
        .args(["worktree", "add", "-b", "t134-linked"])
        .arg(&linked)
        .current_dir(&primary)
        .output()
        .expect("git worktree add is executable");
    assert!(
        worktree.status.success(),
        "git worktree add failed: {}",
        format!(
            "{}{}",
            String::from_utf8_lossy(&worktree.stdout),
            String::from_utf8_lossy(&worktree.stderr)
        )
    );
    let run = start(&primary);
    step(&primary, &run);
    let before = tree(&primary);
    let root = canonical_root(&primary);
    let refused = step(&primary, &run);
    assert_refusal(
        &refused,
        &format!(
            "rtm: step refused; nested_run: beta: observed lineage repeats the canonical Engine \
             root {root} (alpha@{root} > beta@{root}); expected every root distinct"
        ),
        &run,
        "WEBV-023: a linked worktree shares the canonical root",
    );
    assert_tree_unchanged(&primary, &before, "the worktree refusal writes nothing");
    let _ = Command::new("git")
        .args(["worktree", "remove", "--force"])
        .arg(&linked)
        .current_dir(&primary)
        .output();

    // --- A to B to A: the chain back into the parent repeats its identity ---
    let sandbox = Sandbox::new("webv023-aba");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    let child = sandbox.project("child", &chain_child_runbook("beta", "alpha", "../parent"));
    let run = start(&parent);
    step(&parent, &run);
    let parent_step = spawn_step(&parent, &run, &[]);
    let child_run = wait_for_run(&child, "webv023-aba");
    let child_record = run_record(&child, &child_run);
    let (parent_root, child_root) = (canonical_root(&parent), canonical_root(&child));
    let refused = step(&child, &child_run);
    assert_refusal(
        &refused,
        &format!(
            "rtm: step refused; nested_run: alpha: observed lineage repeats the declared program \
             identity \"alpha\" (alpha@{parent_root} > beta@{child_root} > alpha@{parent_root}); \
             expected every identity distinct"
        ),
        &child_run,
        "WEBV-023: A to B to A refuses at the child's own guard",
    );
    assert_eq!(
        run_record(&child, &child_run),
        child_record,
        "the refusing child never transitioned"
    );
    let abandon = rtm(
        &child,
        &[
            "abandon",
            "--run",
            &child_run,
            "--confirm",
            &format!("abandon {child_run}"),
        ],
    );
    assert!(
        abandon.status.success(),
        "the chain unwinds: {}",
        combined(&abandon)
    );
    let finished = finish_step(parent_step, "webv023-aba");
    assert_refusal(
        &finished,
        "rtm: step refused; nested_run: beta: observed outcome abandoned; expected outcome passed",
        &run,
        "WEBV-023: the parent refuses the abandoned receipt",
    );

    // --- exactly the declared depth passes; one beyond it refuses ---
    let sandbox = Sandbox::new("webv023-depth");
    let parent = sandbox.project(
        "parent",
        &parent_runbook("alpha", "beta", "../child", ", max-depth = 2"),
    );
    sandbox.project("child", &terminal_child_runbook("beta"));
    let run = start(&parent);
    let status = step(&parent, &run);
    assert!(status.status.success());
    let listed = combined(&rtm(&parent, &["status", "--run", &run]));
    assert!(
        listed.contains("pending guard: nested_run program=\"beta\" root=\"../child\" max-depth=2"),
        "the State Prompt lists the optional fields in table order: {listed}"
    );
    let accepted = step(&parent, &run);
    assert!(
        accepted.status.success(),
        "WEBV-023: depth two is exactly the declared cap of two: {}",
        combined(&accepted)
    );
    let sandbox = Sandbox::new("webv023-depth1");
    let parent = sandbox.project(
        "parent",
        &parent_runbook("alpha", "beta", "../child", ", max-depth = 1"),
    );
    let child = sandbox.project("child", &terminal_child_runbook("beta"));
    let run = start(&parent);
    step(&parent, &run);
    let before_parent = tree(&parent);
    let before_child = tree(&child);
    let (parent_root, child_root) = (canonical_root(&parent), canonical_root(&child));
    let refused = step(&parent, &run);
    assert_refusal(
        &refused,
        &format!(
            "rtm: step refused; nested_run: beta: observed lineage depth 2 exceeds the permitted \
             maximum 1 (alpha@{parent_root} > beta@{child_root}); expected depth at most 1"
        ),
        &run,
        "WEBV-023: one beyond the declared cap refuses",
    );
    assert_tree_unchanged(&parent, &before_parent, "the depth refusal writes nothing");
    assert_tree_unchanged(&child, &before_child, "no child write precedes the refusal");

    // --- the frozen default: sixteen launches pass, the seventeenth refuses ---
    let sandbox = Sandbox::new("webv023-deep");
    let mut projects = Vec::new();
    for index in 1..=16 {
        let name = format!("p{index:02}");
        let next = format!("p{:02}", index + 1);
        projects.push(sandbox.project(
            &name,
            &chain_child_runbook(&format!("p{index:02}"), &next, &format!("../{next}")),
        ));
    }
    let target = sandbox.project("p17", &terminal_child_runbook("p17"));
    let target_before = tree(&target);
    let first = start(&projects[0]);
    let first_project = projects[0].clone();
    let mut steps: Vec<Option<thread::JoinHandle<Output>>> = Vec::new();
    let mut runs = vec![first.clone()];
    steps.push(Some(thread::spawn(move || step(&first_project, &first))));
    for index in 1..16 {
        let run = wait_for_run(&projects[index], "webv023-deep");
        runs.push(run.clone());
        let project = projects[index].clone();
        steps.push(Some(thread::spawn(move || step(&project, &run))));
    }
    // Sixteen programs deep, the seventeenth launch would make depth 17.
    let seventeenth = finish_thread(
        steps
            .pop()
            .expect("the deepest step")
            .expect("the deepest step is joined"),
        "webv023-deep",
    );
    let mut chain: Vec<(String, String)> = Vec::new();
    for index in 1..=17 {
        chain.push((
            format!("p{index:02}"),
            canonical_root(if index <= 16 {
                &projects[index - 1]
            } else {
                &target
            }),
        ));
    }
    let chain_render = chain_text(
        &chain
            .iter()
            .map(|(identity, root)| (identity.as_str(), root.as_str()))
            .collect::<Vec<_>>(),
    );
    assert_refusal(
        &seventeenth,
        &format!(
            "rtm: step refused; nested_run: p17: observed lineage depth 17 exceeds the permitted \
             maximum 16 ({chain_render}); expected depth at most 16"
        ),
        &runs[15],
        "WEBV-023: depth seventeen exceeds the frozen default",
    );
    assert_tree_unchanged(
        &target,
        &target_before,
        "no launch precedes the depth refusal",
    );
    assert!(
        !ledger_path(&projects[15], &runs[15]).exists(),
        "the refusal precedes the deepest ledger append"
    );
    // Unwind: each retirement names the resumable outcome one level up.
    for index in (1..16).rev() {
        let run = runs[index].clone();
        let project = projects[index].clone();
        let abandon = rtm(
            &project,
            &[
                "abandon",
                "--run",
                &run,
                "--confirm",
                &format!("abandon {run}"),
            ],
        );
        assert!(
            abandon.status.success(),
            "the chain unwinds level by level: {}",
            combined(&abandon)
        );
        let finished = finish_thread(
            steps[index - 1].take().expect("each level step is joined"),
            "unwind",
        );
        let text = combined(&finished);
        let child_name = format!("p{:02}", index + 1);
        assert!(
            text.lines().any(|line| line
                == format!(
                    "rtm: step refused; nested_run: {child_name}: observed outcome \
                     abandoned; expected outcome passed"
                )),
            "level {} refuses its abandoned child by name: {text}",
            index
        );
    }

    // --- a malformed inherited lineage refuses, and changes no other command ---
    let sandbox = Sandbox::new("webv023-malformed");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    let child = sandbox.project("child", &terminal_child_runbook("beta"));
    let garbage = [("RATMAC_LINEAGE", "this is not a lineage")];
    let run = rtm_env(&parent, &["start"], &garbage);
    assert!(
        run.status.success(),
        "WEBV-023: an exported lineage changes no command that evaluates no nested guard: {}",
        combined(&run)
    );
    let run_id = combined(&run)
        .split("started run ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .expect("the run id")
        .to_owned();
    let stepped = rtm_env(&parent, &["step", "--run", &run_id], &garbage);
    assert!(stepped.status.success());
    let before_parent = tree(&parent);
    let before_child = tree(&child);
    let refused = rtm_env(&parent, &["step", "--run", &run_id], &garbage);
    let text = combined(&refused);
    assert_eq!(exit_code(&refused), 0, "the refusal exits 0: {text}");
    assert!(
        text.lines().next().unwrap_or_default().starts_with(
            "rtm: step refused; nested_run: beta: observed malformed inherited lineage: "
        ) && text.contains("; expected"),
        "WEBV-023: a malformed inherited lineage refuses naming the grammar: {text}"
    );
    assert!(text.contains(&format!("next: rtm step --run {run_id}")));
    assert_tree_unchanged(
        &parent,
        &before_parent,
        "the malformed refusal writes nothing",
    );
    assert_tree_unchanged(&child, &before_child, "no child write precedes the refusal");

    // Odd length and emptiness are malformed before any byte is parsed.
    for carrier in ["abc", ""] {
        let refused = rtm_env(
            &parent,
            &["step", "--run", &run_id],
            &[("RATMAC_LINEAGE", carrier)],
        );
        let text = combined(&refused);
        assert_eq!(exit_code(&refused), 0, "the refusal exits 0: {text}");
        assert!(
            text.lines().next().unwrap_or_default().starts_with(
                "rtm: step refused; nested_run: beta: observed malformed inherited lineage: "
            ),
            "WEBV-023: the carrier {carrier:?} is malformed before parse: {text}"
        );
    }

    // --- a truncated inherited lineage refuses the same way ---
    let sandbox = Sandbox::new("webv023-truncated");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    sandbox.project("child", &terminal_child_runbook("beta"));
    let run = start(&parent);
    step(&parent, &run);
    let whole = lineage_bytes(&[
        ("alpha", &canonical_root(&parent)),
        ("beta", &canonical_root(&sandbox.root.join("child"))),
    ]);
    let truncated: String = whole[..whole.len() - 5]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let refused = rtm_env(
        &parent,
        &["step", "--run", &run],
        &[("RATMAC_LINEAGE", &truncated)],
    );
    let text = combined(&refused);
    assert_eq!(exit_code(&refused), 0, "the refusal exits 0: {text}");
    assert!(
        text.lines().next().unwrap_or_default().starts_with(
            "rtm: step refused; nested_run: beta: observed malformed inherited lineage: "
        ),
        "WEBV-023: a truncated chain is malformed: {text}"
    );

    // --- the endpoint's own routes ---
    let sandbox = Sandbox::new("webv023-endpoint");
    let child = sandbox.project("child", &terminal_child_runbook("beta"));
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    let lineage = lineage_hex(&[
        ("alpha", &canonical_root(&parent)),
        ("beta", &canonical_root(&child)),
    ]);

    // The pure help form.
    let help = rtm(&child, &["endpoint", "--help"]);
    assert_eq!(
        exit_code(&help),
        0,
        "the help form exits 0: {}",
        combined(&help)
    );
    let expected_help = "\
Usage: rtm endpoint --invocation <address> --lineage <hex> --receipt-fd <n>\n\
\n\
The machine-facing nested-invocation entry (WEB-006), not for human use. The parent Engine launches it in the child checkout with the Engine-minted invocation address, the complete invocation lineage, and the dedicated receipt pipe's write end. The protocol lineage and the inherited RATMAC_LINEAGE environment must agree byte for byte; a repeated declared program identity or canonical Engine root, a malformed lineage, or an over-deep chain refuses naming the chain before any Run is admitted. At most one child Run exists per invocation address. On success it writes one strict terminal receipt - only after persisting and reading back the child Run's terminal fact - to the receipt pipe, prints nothing, and exits 0.";
    assert_eq!(
        String::from_utf8_lossy(&help.stdout).trim(),
        expected_help,
        "WEBV-023: the pinned endpoint help is the pure form"
    );
    assert!(help.stderr.is_empty(), "the help form prints no diagnostic");

    // Argument defects are the CLI layer's exit 2.
    for (args, message) in [
        (
            vec!["endpoint", "--lineage", &lineage, "--receipt-fd", "1"],
            "rtm: endpoint: --invocation is required",
        ),
        (
            vec![
                "endpoint",
                "--invocation",
                "run-001-inv-001",
                "--receipt-fd",
                "1",
            ],
            "rtm: endpoint: --lineage is required",
        ),
        (
            vec![
                "endpoint",
                "--invocation",
                "run-001-inv-001",
                "--lineage",
                &lineage,
                "--receipt-fd",
                "x",
            ],
            "rtm: endpoint: --receipt-fd x is not a non-negative integer",
        ),
    ] {
        let refused = rtm(&child, &args);
        let text = combined(&refused);
        assert_eq!(exit_code(&refused), 2, "an argument defect exits 2: {text}");
        assert_eq!(
            text.trim(),
            message,
            "WEBV-023: the argument defect is pinned"
        );
        assert!(refused.stdout.is_empty(), "stdout stays empty: {text}");
        assert!(roster(&child).is_empty(), "nothing was written");
    }

    // The inherited lineage must be present and agree with the protocol.
    let refused = Command::new(ratmac_qa::engine_bin!())
        .args([
            "endpoint",
            "--invocation",
            "run-001-inv-001",
            "--lineage",
            &lineage,
            "--receipt-fd",
            "1",
        ])
        .current_dir(&child)
        .env_remove("RATMAC_LINEAGE")
        .output()
        .expect("invoke rtm");
    let text = combined(&refused);
    assert_eq!(exit_code(&refused), 1, "the refusal exits 1: {text}");
    assert_eq!(
        text.trim(),
        "rtm: endpoint: RATMAC_LINEAGE is absent; a nested endpoint requires the inherited lineage",
        "WEBV-023: a missing inherited lineage refuses before admission"
    );
    assert!(roster(&child).is_empty(), "no Run is admitted");

    let other = lineage_hex(&[
        ("alpha", &canonical_root(&parent)),
        ("gamma", "E:/elsewhere"),
    ]);
    let refused = rtm_env(
        &child,
        &[
            "endpoint",
            "--invocation",
            "run-001-inv-001",
            "--lineage",
            &lineage,
            "--receipt-fd",
            "1",
        ],
        &[("RATMAC_LINEAGE", &other)],
    );
    let text = combined(&refused);
    assert_eq!(exit_code(&refused), 1, "the refusal exits 1: {text}");
    assert_eq!(
        text.trim(),
        "rtm: endpoint: the inherited RATMAC_LINEAGE disagrees with the protocol lineage; \
         refusing before admission",
        "WEBV-023: a disagreeing inherited lineage refuses before admission"
    );
    assert!(roster(&child).is_empty(), "no Run is admitted");

    // A repeated identity in the chain refuses naming it, before admission.
    let repeated = lineage_hex(&[
        ("alpha", &canonical_root(&parent)),
        ("beta", &canonical_root(&child)),
        ("alpha", &canonical_root(&parent)),
    ]);
    let refused = rtm_env(
        &child,
        &[
            "endpoint",
            "--invocation",
            "run-001-inv-001",
            "--lineage",
            &repeated,
            "--receipt-fd",
            "1",
        ],
        &[("RATMAC_LINEAGE", &repeated)],
    );
    let text = combined(&refused);
    assert_eq!(exit_code(&refused), 1, "the refusal exits 1: {text}");
    assert!(
        text.contains("rtm: endpoint: ")
            && (text.contains("lineage repeats the declared program identity")
                || text.contains("malformed")),
        "WEBV-023: a repeated identity refuses before admission: {text}"
    );
    assert!(
        roster(&child).is_empty(),
        "no Run is admitted on a lineage refusal"
    );

    // The chain's last entry must be this checkout's own program.
    let mismatched = lineage_hex(&[
        ("alpha", &canonical_root(&parent)),
        ("delta", &canonical_root(&child)),
    ]);
    let refused = rtm_env(
        &child,
        &[
            "endpoint",
            "--invocation",
            "run-001-inv-001",
            "--lineage",
            &mismatched,
            "--receipt-fd",
            "1",
        ],
        &[("RATMAC_LINEAGE", &mismatched)],
    );
    let text = combined(&refused);
    assert_eq!(exit_code(&refused), 1, "the refusal exits 1: {text}");
    let child_root = canonical_root(&child);
    assert_eq!(
        text.trim(),
        format!(
            "rtm: endpoint: observed lineage entry delta@{child_root}; \
             expected this checkout's beta@{child_root}"
        ),
        "WEBV-023: a foreign own-entry refuses naming both"
    );
    assert!(refused.stdout.is_empty(), "stdout stays empty: {text}");
    assert!(roster(&child).is_empty(), "no Run is admitted");

    // --- the new runbook defects render through today's parse paths ---
    let sandbox = Sandbox::new("webv023-parses");
    let valid = terminal_child_runbook("beta");
    let base = sandbox.project("base", &valid);
    let run = start(&base);
    let deep_defect = parent_runbook("alpha", "beta", "../base", ", max-depth = 0");
    fs::write(base.join(".ratmac/ratmac.toml"), &deep_defect).expect("plant the max-depth defect");
    let prefix = "parse .ratmac/ratmac.toml [RB713]: invalid nested declaration: ";
    for (args, expected, code) in [
        (
            vec!["status", "--run", &run],
            format!("rtm: status: {prefix}"),
            1,
        ),
        (
            vec!["step", "--run", &run],
            format!("rtm: step: {prefix}"),
            1,
        ),
        (
            vec![
                "hold",
                "--run",
                &run,
                "--blocker",
                "b",
                "--confirm",
                &format!("hold {run}"),
            ],
            format!("rtm: hold refused; {prefix}"),
            1,
        ),
        (
            vec!["spawn", "rev", "--run", &run],
            format!("rtm: spawn: {prefix}"),
            1,
        ),
        (vec!["start"], format!("rtm: start: {prefix}"), 1),
    ] {
        let refused = rtm(&base, &args);
        let text = combined(&refused);
        assert_eq!(
            exit_code(&refused),
            code,
            "the {} parse refusal exits {}: {text}",
            args[0],
            code
        );
        assert!(
            text.starts_with(&expected),
            "WEBV-023: {} renders today's parse shape: {text}",
            args[0]
        );
    }
    let status_refusal = rtm(&base, &["status", "--run", &run]);
    assert!(
        combined(&status_refusal)
            .lines()
            .nth(1)
            .unwrap_or_default()
            .starts_with("next: "),
        "the status parse refusal keeps today's next line"
    );
    let doctor = rtm(&base, &["doctor"]);
    assert_eq!(
        exit_code(&doctor),
        2,
        "a runbook defect is an error finding"
    );
    assert!(
        combined(&doctor).contains("RB713 error: "),
        "WEBV-023: doctor reports the code: {}",
        combined(&doctor)
    );
    let endpoint_refusal = rtm_env(
        &base,
        &[
            "endpoint",
            "--invocation",
            "a",
            "--lineage",
            "b",
            "--receipt-fd",
            "1",
        ],
        &[("RATMAC_LINEAGE", "b")],
    );
    assert_eq!(exit_code(&endpoint_refusal), 1);
    assert!(
        combined(&endpoint_refusal).starts_with(
            "rtm: endpoint: parse .ratmac/ratmac.toml [RB713]: invalid nested declaration: "
        ),
        "WEBV-023: the endpoint renders the parse refusal under its prefix"
    );

    // RB712: the declared root must exist when the runbook loads.
    let missing_root = parent_runbook("alpha", "beta", "../absent-child", "");
    fs::write(base.join(".ratmac/ratmac.toml"), &missing_root).expect("plant the missing root");
    let refused = rtm(&base, &["start"]);
    assert_eq!(exit_code(&refused), 1);
    assert!(
        combined(&refused).starts_with(
            "rtm: start: parse .ratmac/ratmac.toml [RB712]: invalid nested declaration: "
        ),
        "WEBV-023: a missing declared root is RB712"
    );

    // RB711: an empty declared program identity is malformed.
    let empty_identity = parent_runbook("", "beta", "../base", "");
    fs::write(base.join(".ratmac/ratmac.toml"), &empty_identity).expect("plant the empty identity");
    let refused = rtm(&base, &["start"]);
    assert_eq!(exit_code(&refused), 1);
    assert!(
        combined(&refused).starts_with(
            "rtm: start: parse .ratmac/ratmac.toml [RB711]: invalid nested declaration: "
        ),
        "WEBV-023: an empty program identity is RB711"
    );

    // The invoking runbook must declare its own identity before launching.
    let mut anonymous = parent_runbook("alpha", "beta", "../base", "");
    anonymous = anonymous.replacen("program = \"alpha\"\n", "", 1);
    let anonymous_child = sandbox.project("anon-child", &terminal_child_runbook("beta"));
    let anonymous_parent = sandbox.project("anon-parent", &anonymous);
    let run = start(&anonymous_parent);
    let stepped = step(&anonymous_parent, &run);
    assert!(stepped.status.success());
    let before = tree(&anonymous_parent);
    let before_child = tree(&anonymous_child);
    let refused = step(&anonymous_parent, &run);
    assert_refusal(
        &refused,
        "rtm: step refused; nested_run: beta: observed no declared program identity; expected \
         the invoking runbook to declare one before launching a nested program",
        &run,
        "WEBV-023: an anonymous invoking runbook refuses",
    );
    assert_tree_unchanged(
        &anonymous_parent,
        &before,
        "the identity refusal writes nothing",
    );
    assert_tree_unchanged(
        &anonymous_child,
        &before_child,
        "no child write precedes the refusal",
    );
}

/// WEBV-024: interruption after child admission, child terminal write, receipt delivery, and parent consumption retries idempotently, never borrows an older result, and leaves same-root spawn, Run locks, and command gates intact.
#[test]
fn webv_024() {
    let (sandbox, ..) = {
        let (sandbox, ..) = drive_trio("webv024-control");
        (sandbox,)
    };
    drop(sandbox);

    // --- a pause without its companions refuses before anything is written ---
    let sandbox = Sandbox::new("webv024-companions");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    let child = sandbox.project("child", &plain_child_runbook("beta"));
    let run = start(&parent);
    step(&parent, &run);
    let before_parent = tree(&parent);
    let before_child = tree(&child);
    // The guard validates its own companions when its evaluation begins.
    let nested = rtm_env(
        &parent,
        &["step", "--run", &run],
        &[("RATMAC_TEST_NESTED_PAUSE", "after-record")],
    );
    let text = combined(&nested);
    assert_eq!(
        exit_code(&nested),
        1,
        "the companion refusal exits 1: {text}"
    );
    assert!(
        text.lines().any(|line| line
            == "rtm: step: RATMAC_TEST_NESTED_PAUSE companion \
               RATMAC_TEST_NESTED_PAUSE_MARKER is missing or invalid"),
        "WEBV-024: the nested companion refusal is pinned: {text}"
    );
    assert!(
        text.contains(&format!("next: rtm step --run {run}")),
        "the refusal keeps today's step next line: {text}"
    );
    // The endpoint validates its companions before any admission write.
    let chain = lineage_hex(&[
        ("alpha", &canonical_root(&parent)),
        ("beta", &canonical_root(&child)),
    ]);
    let endpoint = rtm_env(
        &child,
        &[
            "endpoint",
            "--invocation",
            "run-001-inv-001",
            "--lineage",
            &chain,
            "--receipt-fd",
            "1",
        ],
        &[
            ("RATMAC_LINEAGE", &chain),
            ("RATMAC_TEST_ENDPOINT_PAUSE", "after-admit"),
        ],
    );
    let text = combined(&endpoint);
    assert_eq!(
        exit_code(&endpoint),
        1,
        "the companion refusal exits 1: {text}"
    );
    assert_eq!(
        text.trim(),
        "rtm: endpoint: RATMAC_TEST_ENDPOINT_PAUSE companion \
         RATMAC_TEST_ENDPOINT_PAUSE_MARKER is missing or invalid",
        "WEBV-024: the endpoint companion refusal is pinned"
    );
    assert!(endpoint.stdout.is_empty(), "stdout stays empty: {text}");
    assert!(roster(&child).is_empty(), "no Run is admitted");
    assert_tree_unchanged(&parent, &before_parent, "the parent is untouched");
    assert_tree_unchanged(&child, &before_child, "the child is untouched");
    drop(sandbox);

    // --- interrupt after child admission: the retry resumes that same Run ---
    let sandbox = Sandbox::new("webv024-admit");
    let markers = sandbox.root.join("marks");
    fs::create_dir_all(&markers).expect("create the marker tree");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    let child = sandbox.project("child", &plain_child_runbook("beta"));
    let run = start(&parent);
    step(&parent, &run);
    let owned: Vec<(String, String)> = vec![
        (
            "RATMAC_TEST_ENDPOINT_PAUSE".to_owned(),
            "after-admit".to_owned(),
        ),
        (
            "RATMAC_TEST_ENDPOINT_PAUSE_MARKER".to_owned(),
            markers.join("admit.marker").to_string_lossy().into_owned(),
        ),
        (
            "RATMAC_TEST_ENDPOINT_PAUSE_RELEASE".to_owned(),
            markers.join("admit.release").to_string_lossy().into_owned(),
        ),
        (
            "RATMAC_TEST_ENDPOINT_PAUSE_TIMEOUT_MILLIS".to_owned(),
            "4000".to_owned(),
        ),
    ];
    let refs: Vec<(&str, &str)> = owned
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let parent_step = spawn_step(&parent, &run, &refs);
    let child_run = wait_for_run(&child, "webv024-admit");
    wait_for_file(
        &markers.join("admit.marker"),
        "after-admit",
        "webv024-admit",
    );
    assert!(
        run_record(&child, &child_run).contains("state = \"work\""),
        "the child Run is admitted and waits for motion"
    );
    kill_and_collect(parent_step);
    // The retry resumes the same invocation address and the same child Run.
    let retry = spawn_step(&parent, &run, &[]);
    let resumed = wait_for_run(&child, "webv024-admit retry");
    assert_eq!(
        resumed, child_run,
        "no second child is minted for one address"
    );
    let child_step = step(&child, &child_run);
    assert!(
        child_step.status.success(),
        "the child finishes: {}",
        combined(&child_step)
    );
    let finished = finish_step(retry, "webv024-admit retry");
    assert!(
        finished.status.success(),
        "the retry completes the interrupted invocation: {}",
        combined(&finished)
    );
    let ledger = ledger_text(&parent, &run);
    assert_eq!(
        ledger.matches("invocation = ").count(),
        1,
        "one entry survives the retry"
    );
    assert!(ledger.contains(&format!("invocation = \"{run}-inv-001\"")));
    assert!(ledger.contains("consumed = true"));
    assert_eq!(roster(&child), vec![child_run]);

    // --- interrupt after the child's terminal write: the receipt re-emits ---
    let sandbox = Sandbox::new("webv024-terminal");
    let markers = sandbox.root.join("marks");
    fs::create_dir_all(&markers).expect("create the marker tree");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    let child = sandbox.project("child", &plain_child_runbook("beta"));
    let run = start(&parent);
    step(&parent, &run);
    let owned: Vec<(String, String)> = vec![
        (
            "RATMAC_TEST_ENDPOINT_PAUSE".to_owned(),
            "after-terminal".to_owned(),
        ),
        (
            "RATMAC_TEST_ENDPOINT_PAUSE_MARKER".to_owned(),
            markers
                .join("terminal.marker")
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "RATMAC_TEST_ENDPOINT_PAUSE_RELEASE".to_owned(),
            markers
                .join("terminal.release")
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "RATMAC_TEST_ENDPOINT_PAUSE_TIMEOUT_MILLIS".to_owned(),
            "4000".to_owned(),
        ),
    ];
    let refs: Vec<(&str, &str)> = owned
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let parent_step = spawn_step(&parent, &run, &refs);
    let child_run = wait_for_run(&child, "webv024-terminal");
    let child_step = step(&child, &child_run);
    assert!(
        child_step.status.success(),
        "the child turns terminal: {}",
        combined(&child_step)
    );
    wait_for_file(
        &markers.join("terminal.marker"),
        "after-terminal",
        "webv024-terminal",
    );
    kill_and_collect(parent_step);
    let retry = step(&parent, &run);
    assert!(
        retry.status.success(),
        "the retry re-emits and re-verifies the same receipt: {}",
        combined(&retry)
    );
    let ledger = ledger_text(&parent, &run);
    assert_eq!(ledger.matches("invocation = ").count(), 1);
    assert_hex64(
        &quoted_field(&ledger, "receipt_sha256"),
        "the re-verified receipt digest",
    );
    assert!(ledger.contains("consumed = true"));
    assert_eq!(roster(&child), vec![child_run]);

    // --- interrupt after the ledger record, before any launch ---
    let sandbox = Sandbox::new("webv024-record");
    let markers = sandbox.root.join("marks");
    fs::create_dir_all(&markers).expect("create the marker tree");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    let child = sandbox.project("child", &plain_child_runbook("beta"));
    let run = start(&parent);
    step(&parent, &run);
    let owned: Vec<(String, String)> = vec![
        (
            "RATMAC_TEST_NESTED_PAUSE".to_owned(),
            "after-record".to_owned(),
        ),
        (
            "RATMAC_TEST_NESTED_PAUSE_MARKER".to_owned(),
            markers.join("record.marker").to_string_lossy().into_owned(),
        ),
        (
            "RATMAC_TEST_NESTED_PAUSE_RELEASE".to_owned(),
            markers
                .join("record.release")
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "RATMAC_TEST_NESTED_PAUSE_TIMEOUT_MILLIS".to_owned(),
            "4000".to_owned(),
        ),
    ];
    let refs: Vec<(&str, &str)> = owned
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let parent_step = spawn_step(&parent, &run, &refs);
    wait_for_file(
        &markers.join("record.marker"),
        "after-record",
        "webv024-record",
    );
    assert!(
        roster(&child).is_empty(),
        "no endpoint ran, so the child root holds no Run"
    );
    kill_and_collect(parent_step);
    assert!(
        ledger_text(&parent, &run).contains("consumed = false"),
        "the recorded entry awaits its retry"
    );
    let retry = spawn_step(&parent, &run, &[]);
    let child_run = wait_for_run(&child, "webv024-record retry");
    let child_step = step(&child, &child_run);
    assert!(
        child_step.status.success(),
        "the child finishes: {}",
        combined(&child_step)
    );
    let finished = finish_step(retry, "webv024-record retry");
    assert!(
        finished.status.success(),
        "the retry launches the recorded invocation: {}",
        combined(&finished)
    );
    let ledger = ledger_text(&parent, &run);
    assert_eq!(ledger.matches("invocation = ").count(), 1);
    assert!(ledger.contains(&format!("invocation = \"{run}-inv-001\"")));

    // --- interrupt after receipt verification: the annotate is a no-op ---
    let sandbox = Sandbox::new("webv024-receipt");
    let markers = sandbox.root.join("marks");
    fs::create_dir_all(&markers).expect("create the marker tree");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    let child = sandbox.project("child", &plain_child_runbook("beta"));
    let run = start(&parent);
    step(&parent, &run);
    let owned: Vec<(String, String)> = vec![
        (
            "RATMAC_TEST_NESTED_PAUSE".to_owned(),
            "after-receipt".to_owned(),
        ),
        (
            "RATMAC_TEST_NESTED_PAUSE_MARKER".to_owned(),
            markers
                .join("receipt.marker")
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "RATMAC_TEST_NESTED_PAUSE_RELEASE".to_owned(),
            markers
                .join("receipt.release")
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "RATMAC_TEST_NESTED_PAUSE_TIMEOUT_MILLIS".to_owned(),
            "4000".to_owned(),
        ),
    ];
    let refs: Vec<(&str, &str)> = owned
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let parent_step = spawn_step(&parent, &run, &refs);
    let child_run = wait_for_run(&child, "webv024-receipt");
    let child_step = step(&child, &child_run);
    assert!(child_step.status.success());
    wait_for_file(
        &markers.join("receipt.marker"),
        "after-receipt",
        "webv024-receipt",
    );
    let recorded = ledger_text(&parent, &run);
    assert_hex64(
        &quoted_field(&recorded, "receipt_sha256"),
        "the receipt was recorded",
    );
    assert!(recorded.contains(&format!("child_run = \"{child_run}\"")));
    assert!(recorded.contains("consumed = false"));
    kill_and_collect(parent_step);
    let retry = step(&parent, &run);
    assert!(
        retry.status.success(),
        "the retry re-verifies byte-for-byte and passes: {}",
        combined(&retry)
    );
    let ledger = ledger_text(&parent, &run);
    assert_eq!(ledger.matches("invocation = ").count(), 1);
    assert!(ledger.contains("consumed = true"));

    // --- interrupt after consumption: today's deliberate State-first limit ---
    let sandbox = Sandbox::new("webv024-consume");
    let markers = sandbox.root.join("marks");
    fs::create_dir_all(&markers).expect("create the marker tree");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    let child = sandbox.project("child", &plain_child_runbook("beta"));
    let run = start(&parent);
    step(&parent, &run);
    let owned: Vec<(String, String)> = vec![
        (
            "RATMAC_TEST_NESTED_PAUSE".to_owned(),
            "after-consume".to_owned(),
        ),
        (
            "RATMAC_TEST_NESTED_PAUSE_MARKER".to_owned(),
            markers
                .join("consume.marker")
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "RATMAC_TEST_NESTED_PAUSE_RELEASE".to_owned(),
            markers
                .join("consume.release")
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "RATMAC_TEST_NESTED_PAUSE_TIMEOUT_MILLIS".to_owned(),
            "4000".to_owned(),
        ),
    ];
    let refs: Vec<(&str, &str)> = owned
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let parent_step = spawn_step(&parent, &run, &refs);
    let child_run = wait_for_run(&child, "webv024-consume");
    let child_step = step(&child, &child_run);
    assert!(child_step.status.success());
    wait_for_file(
        &markers.join("consume.marker"),
        "after-consume",
        "webv024-consume",
    );
    kill_and_collect(parent_step);
    assert!(
        ledger_text(&parent, &run).contains("consumed = true"),
        "the mark flipped before the interruption"
    );
    assert!(
        run_record(&parent, &run).contains("state = \"done\""),
        "the Run Record replace preceded the history append"
    );
    assert!(
        !log_text(&parent).contains(&run),
        "today's deliberate limit: the transition has no history line yet"
    );
    let retry = step(&parent, &run);
    let text = combined(&retry);
    assert_eq!(exit_code(&retry), 0, "the retry exits 0: {text}");
    assert!(
        text.contains("is terminal (status passed)"),
        "the consumed invocation never advances anything again: {text}"
    );

    // --- a consumed entry is never borrowed: the second evaluation mints ---
    let sandbox = Sandbox::new("webv024-borrow");
    let parent = sandbox.project("parent", &twice_parent_runbook("alpha", "beta", "../child"));
    let child = sandbox.project("child", &terminal_child_runbook("beta"));
    let run = start(&parent);
    for _ in 0..3 {
        let stepped = step(&parent, &run);
        assert!(
            stepped.status.success(),
            "each ordinary stage advances: {}",
            combined(&stepped)
        );
    }
    let refused = step(&parent, &run);
    assert!(
        refused.status.success(),
        "the second nested invocation completes on a fresh child: {}",
        combined(&refused)
    );
    let ledger = ledger_text(&parent, &run);
    let blocks: Vec<&str> = ledger.split("[[").skip(1).collect();
    assert_eq!(blocks.len(), 2, "two invocations, two entries:\n{ledger}");
    let first_child = quoted_field(&format!("[[{}", blocks[0]), "child_run");
    let second_child = quoted_field(&format!("[[{}", blocks[1]), "child_run");
    assert_ne!(
        first_child, second_child,
        "the older result is never borrowed"
    );
    assert!(blocks[1].contains(&format!("invocation = \"{run}-inv-002\"")));
    assert_eq!(roster(&child), vec![first_child, second_child]);

    // --- the waiting Run holds its lock; other Runs and roots are free ---
    let sandbox = Sandbox::new("webv024-lock");
    let parent = sandbox.project("parent", &parent_runbook("alpha", "beta", "../child", ""));
    let child = sandbox.project("child", &plain_child_runbook("beta"));
    fs::create_dir_all(parent.join("work")).expect("create the parent work root");
    fs::write(parent.join("work/blocker.txt"), "blocked\n").expect("write the blocker");
    let run = start(&parent);
    step(&parent, &run);
    let parent_step = spawn_step(&parent, &run, &[]);
    let child_run = wait_for_run(&child, "webv024-lock");
    let before = tree(&parent);
    let held = rtm(
        &parent,
        &[
            "hold",
            "--run",
            &run,
            "--blocker",
            "work/blocker.txt",
            "--confirm",
            &format!("hold {run}"),
        ],
    );
    let text = combined(&held);
    assert!(
        !held.status.success() && text.contains("lock wait expired"),
        "the nested wait holds the addressed Run lock: {text}"
    );
    assert_tree_unchanged(&parent, &before, "the lock refusal writes nothing");
    let child_step = step(&child, &child_run);
    assert!(child_step.status.success());
    let finished = finish_step(parent_step, "webv024-lock");
    assert!(
        finished.status.success(),
        "the wait ends when the child passes: {}",
        combined(&finished)
    );

    // --- coexistence: command gates, the nested guard, and same-root spawn ---
    let sandbox = Sandbox::new("webv024-mixed");
    let parent = sandbox.project("parent", &coexistence_runbook("beta", "../child"));
    sandbox.project("child", &terminal_child_runbook("beta"));
    fs::copy(ratmac_qa::engine_bin!(), parent.join("probe.exe")).expect("pin the probe program");
    let run = start(&parent);
    let gate = step(&parent, &run);
    assert!(
        gate.status.success(),
        "the ordinary command gate still passes: {}",
        combined(&gate)
    );
    let nested = step(&parent, &run);
    assert!(
        nested.status.success(),
        "the nested guard passes beside it: {}",
        combined(&nested)
    );

    let spawn = rtm(
        &parent,
        &["spawn", "rev", "--run", &run, "--bind", "ticket=t-134"],
    );
    assert!(
        spawn.status.success(),
        "the same-root spawn model is untouched: {}",
        combined(&spawn)
    );
    let joined = step(&parent, &run);
    assert!(
        joined.status.success(),
        "the join releases off the spawn ledger: {}",
        combined(&joined)
    );
    let spawn_ledger =
        fs::read_to_string(parent.join(".ratmac/runs").join(&run).join("spawn-ledger"))
            .expect("the spawn ledger is readable");
    assert_eq!(
        spawn_ledger.matches("[[children]]").count(),
        1,
        "a nested invocation adds no spawn-ledger entry:\n{spawn_ledger}"
    );
    assert!(
        ledger_path(&parent, &run).is_file(),
        "the nested ledger lives beside the spawn ledger"
    );
    assert!(
        run_record(&parent, &run).contains("state = \"done\""),
        "the parent finished all four stages"
    );
}
