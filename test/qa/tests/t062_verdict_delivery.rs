//! t-062 / FDC-003: durable, consume-before-advance verdict delivery.
//!
//! PT-062-01 `empty_slot_and_straight_line_contract`
//! PT-062-02 `valid_verdict_is_archived_before_state_advance`
//! PT-062-03 `invalid_or_missing_verdict_refuses_unchanged`
//! PT-062-04 `interruption_windows_preserve_consume_then_advance`

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const STRAIGHT_RUNBOOK: &str = r#"
[phases.start]
prompt = "Start."

[phases.middle]
prompt = "Continue."

[phases.done]
prompt = "Done."

[[transitions]]
from = "start"
to = "middle"

[[transitions]]
from = "middle"
to = "done"
"#;

const TERMINAL_BRANCH_RUNBOOK: &str = r#"
[phases.review]
prompt = "Review."
inputs = ["approve", "rework"]

[phases.approved]
prompt = "Approved."

[phases.rework]
prompt = "Rework."

[[transitions]]
from = "review"
to = "rework"
input = "rework"

[[transitions]]
from = "review"
to = "approved"
input = "approve"
"#;

const REPEATED_BRANCH_RUNBOOK: &str = r#"
[phases.start]
prompt = "Start."

[phases.review]
prompt = "Review."
inputs = ["approve", "rework"]

[phases.approved]
prompt = "Approved."

[phases.rework]
prompt = "Rework."

[[transitions]]
from = "start"
to = "review"

[[transitions]]
from = "review"
to = "rework"
input = "rework"

[[transitions]]
from = "review"
to = "approved"
input = "approve"

[[transitions]]
from = "approved"
to = "review"

[[transitions]]
from = "rework"
to = "review"
"#;

const GUARDED_BRANCH_RUNBOOK: &str = r#"
[phases.review]
prompt = "Review."
inputs = ["approve", "rework"]
guards = [{ kind = "file_contains", path = "gate.txt", contains = "ready" }]

[phases.approved]
prompt = "Approved."

[phases.rework]
prompt = "Rework."

[[transitions]]
from = "review"
to = "approved"
input = "approve"

[[transitions]]
from = "review"
to = "rework"
input = "rework"
"#;

struct Fixture {
    root: PathBuf,
    run_id: String,
}

impl Fixture {
    fn new(label: &str, runbook: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t062-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".arca/goal")).expect("create fixture goal tree");
        fs::create_dir_all(root.join(".ratmac")).expect("create fixture Engine tree");
        fs::create_dir_all(root.join("src")).expect("create fixture source tree");
        fs::write(root.join(".arca/goal/spec.md"), "# Fixture goal\n").expect("write fixture goal");
        fs::write(root.join(".ratmac/ratmac.toml"), runbook).expect("write fixture machine class");
        fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").expect("write fixture source");

        let start = Command::new(env!("CARGO_BIN_EXE_rtm"))
            .arg("start")
            .current_dir(&root)
            .output()
            .expect("invoke rtm start");
        assert!(
            start.status.success(),
            "FDC-003 fixture start must succeed: {}",
            combined(&start)
        );

        let runs = root.join(".ratmac/runs");
        let mut roster = fs::read_dir(&runs)
            .expect("started fixture has a runs roster")
            .map(|entry| entry.expect("read roster entry"))
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        roster.sort();
        assert_eq!(roster.len(), 1, "one fixture start must mint one Run");

        Self {
            root,
            run_id: roster.remove(0),
        }
    }

    fn run_dir(&self) -> PathBuf {
        self.root.join(".ratmac/runs").join(&self.run_id)
    }

    fn state_path(&self) -> PathBuf {
        self.run_dir().join("state.toml")
    }

    fn verdict_path(&self) -> PathBuf {
        self.run_dir().join("verdict.toml")
    }

    fn state(&self) -> Vec<u8> {
        fs::read(self.state_path()).expect("read Run State File")
    }

    fn phase(&self) -> String {
        let state = String::from_utf8(self.state()).expect("State File is UTF-8");
        let parsed: toml::Value = state.parse().expect("State File is valid TOML");
        parsed["phase"]
            .as_str()
            .expect("State File carries a string phase")
            .to_owned()
    }

    fn publish(&self, bytes: &[u8]) {
        fs::write(self.verdict_path(), bytes).expect("publish live verdict fixture");
    }

    fn step(&self) -> Output {
        self.step_with_fault(None)
    }

    fn step_with_fault(&self, fault: Option<&str>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rtm"));
        command
            .args(["step", "--run", self.run_id.as_str()])
            .current_dir(&self.root)
            .env_remove("RATMAC_TEST_STEP_FAULT");
        if let Some(fault) = fault {
            command.env("RATMAC_TEST_STEP_FAULT", fault);
        }
        command.output().expect("invoke addressed rtm step")
    }

    fn archives(&self) -> BTreeMap<String, Vec<u8>> {
        let directory = self.run_dir().join("verdicts");
        if !directory.exists() {
            return BTreeMap::new();
        }
        fs::read_dir(directory)
            .expect("list verdict evidence")
            .map(|entry| {
                let entry = entry.expect("read verdict evidence entry");
                assert!(
                    entry.path().is_file(),
                    "verdict evidence contains only record files: {:?}",
                    entry.path()
                );
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    fs::read(entry.path()).expect("read archived verdict"),
                )
            })
            .collect()
    }

    fn run_snapshot(&self) -> BTreeMap<String, Option<Vec<u8>>> {
        tree_snapshot(&self.run_dir())
    }
}

impl Drop for Fixture {
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

/// Every directory and file under `root`, with exact file bytes.
fn tree_snapshot(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    fn walk(root: &Path, directory: &Path, into: &mut BTreeMap<String, Option<Vec<u8>>>) {
        for entry in fs::read_dir(directory).expect("snapshot directory is listable") {
            let path = entry.expect("snapshot entry is readable").path();
            let relative = path
                .strip_prefix(root)
                .expect("snapshot path is below root")
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                into.insert(format!("{relative}/"), None);
                walk(root, &path, into);
            } else {
                into.insert(relative, Some(fs::read(path).expect("snapshot file bytes")));
            }
        }
    }

    let mut snapshot = BTreeMap::new();
    walk(root, root, &mut snapshot);
    snapshot
}

fn verdict(phase: &str, input: &str, rationale: &str) -> Vec<u8> {
    format!("phase = {phase:?}\ninput = {input:?}\nrationale = {rationale:?}\n").into_bytes()
}

fn assert_step_succeeds(output: &Output, context: &str) {
    let text = combined(output);
    assert!(
        output.status.success() && !text.to_ascii_lowercase().contains("step refused"),
        "{context}: {text}"
    );
}

fn assert_refuses_unchanged(fixture: &Fixture, label: &str, expected: &[&str]) {
    let before = fixture.run_snapshot();
    let refused = fixture.step();
    let diagnostic = combined(&refused).to_ascii_lowercase();
    assert!(
        diagnostic.contains("step refused"),
        "FDC-003 {label} must refuse instead of advancing: {diagnostic}"
    );
    assert!(
        diagnostic.contains("verdict"),
        "FDC-003 {label} must identify the verdict boundary: {diagnostic}"
    );
    for token in expected {
        assert!(
            diagnostic.contains(&token.to_ascii_lowercase()),
            "FDC-003 {label} diagnostic must identify {token:?}: {diagnostic}"
        );
    }
    assert_eq!(
        fixture.run_snapshot(),
        before,
        "FDC-003 {label} must preserve the live record, State File, evidence files, and directories byte-for-byte"
    );
}

fn assert_archive_record(bytes: &[u8], phase: &str, input: &str, rationale: &str) {
    let text = std::str::from_utf8(bytes).expect("archived verdict is UTF-8");
    let value: toml::Value = text.parse().expect("archived verdict is valid TOML");
    let table = value
        .as_table()
        .expect("archived verdict is one TOML table");
    let mut keys = table.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["input", "phase", "rationale"],
        "the archived strict record must retain exactly the authoritative fields"
    );
    assert_eq!(table["phase"].as_str(), Some(phase));
    assert_eq!(table["input"].as_str(), Some(input));
    assert_eq!(table["rationale"].as_str(), Some(rationale));
}

/// PT-062-01 / FDCV-020, FDCV-022: absence is the empty slot. Straight-line
/// movement neither needs nor consumes a verdict, while a stray live record
/// refuses untouched.
#[test]
fn empty_slot_and_straight_line_contract() {
    let fixture = Fixture::new("straight", STRAIGHT_RUNBOOK);
    assert_eq!(fixture.phase(), "start");
    assert!(
        !fixture.verdict_path().exists(),
        "FDC-003: rtm start must not create an empty verdict placeholder"
    );
    assert!(
        fixture.archives().is_empty(),
        "FDC-003: a new Run has no consumed verdict evidence"
    );

    let stray = verdict("start", "invented", "a straight Phase has no input");
    fixture.publish(&stray);
    let before = fixture.run_snapshot();
    let refused = fixture.step();
    let diagnostic = combined(&refused).to_ascii_lowercase();
    assert!(
        diagnostic.contains("step refused"),
        "FDC-003: a live verdict at a straight-line Phase must refuse: {diagnostic}"
    );
    assert!(
        diagnostic.contains("verdict") && diagnostic.contains("straight"),
        "the refusal must explain that a straight-line Phase rejects a verdict: {diagnostic}"
    );
    assert_eq!(
        fixture.run_snapshot(),
        before,
        "a stray straight-line verdict must remain live and the Run must remain byte-identical"
    );

    fs::remove_file(fixture.verdict_path()).expect("external reviewer clears stray verdict");
    let first = fixture.step();
    assert_step_succeeds(&first, "straight start -> middle needs no verdict");
    assert_eq!(fixture.phase(), "middle");
    assert!(!fixture.verdict_path().exists());
    assert!(fixture.archives().is_empty());

    let second = fixture.step();
    assert_step_succeeds(&second, "straight middle -> done needs no verdict");
    assert_eq!(fixture.phase(), "done");
    assert!(!fixture.verdict_path().exists());
    assert!(
        fixture.archives().is_empty(),
        "straight-line movement must never synthesize verdict evidence"
    );
}

/// PT-062-02 / FDCV-015, FDCV-018, FDCV-019: a strict external record selects
/// its labelled edge, is renamed out of the live slot before advance, and every
/// repeated visit appends a distinct monotonic archive without rewriting prior
/// evidence.
#[test]
fn valid_verdict_is_archived_before_state_advance() {
    let fixture = Fixture::new("repeated", REPEATED_BRANCH_RUNBOOK);
    assert_step_succeeds(&fixture.step(), "enter review on the straight edge");
    assert_eq!(fixture.phase(), "review");

    let first = verdict("review", "approve", "first external decision");
    fixture.publish(&first);
    assert_step_succeeds(&fixture.step(), "valid approve verdict selects approve");
    assert_eq!(
        fixture.phase(),
        "approved",
        "input, not transition declaration order, selects the successor"
    );
    assert!(
        !fixture.verdict_path().exists(),
        "a consumed verdict clears the live slot"
    );
    let archives = fixture.archives();
    assert_eq!(archives.keys().collect::<Vec<_>>(), ["000001.toml"]);
    assert_eq!(archives["000001.toml"], first);
    assert_archive_record(
        &archives["000001.toml"],
        "review",
        "approve",
        "first external decision",
    );

    assert_step_succeeds(
        &fixture.step(),
        "approved returns to review without a verdict",
    );
    assert_eq!(fixture.phase(), "review");
    assert_eq!(
        fixture.archives()["000001.toml"],
        first,
        "later movement cannot rewrite immutable evidence"
    );

    let second = verdict("review", "rework", "second external decision");
    fixture.publish(&second);
    assert_step_succeeds(&fixture.step(), "valid rework verdict selects rework");
    assert_eq!(fixture.phase(), "rework");
    let archives = fixture.archives();
    assert_eq!(
        archives.keys().collect::<Vec<_>>(),
        ["000001.toml", "000002.toml"]
    );
    assert_eq!(archives["000001.toml"], first);
    assert_eq!(archives["000002.toml"], second);
    assert_archive_record(
        &archives["000002.toml"],
        "review",
        "rework",
        "second external decision",
    );

    assert_step_succeeds(
        &fixture.step(),
        "rework returns to review without a verdict",
    );
    assert_eq!(fixture.phase(), "review");
    let third = verdict("review", "approve", "third external decision");
    fixture.publish(&third);
    assert_step_succeeds(&fixture.step(), "a repeated visit consumes a fresh verdict");
    let archives = fixture.archives();
    assert_eq!(
        archives.keys().collect::<Vec<_>>(),
        ["000001.toml", "000002.toml", "000003.toml"]
    );
    assert_eq!(archives["000001.toml"], first);
    assert_eq!(archives["000002.toml"], second);
    assert_eq!(archives["000003.toml"], third);
    assert!(!fixture.verdict_path().exists());
}

/// PT-062-03 / FDCV-006, FDCV-007, FDCV-009, FDCV-010, FDCV-018,
/// FDCV-020, FDCV-021: all guards run before the verdict is inspected. Once
/// guards pass, absent, malformed, non-strict, stale, and illegal records all
/// refuse without consuming bytes or changing any Run-owned artifact.
#[test]
fn invalid_or_missing_verdict_refuses_unchanged() {
    let fixture = Fixture::new("strict", GUARDED_BRANCH_RUNBOOK);
    assert_eq!(fixture.phase(), "review");

    let poison = b"phase = \"review\"\ninput = \"approve\"\nrationale = \"why\"\npoison = true\n";
    fixture.publish(poison);
    let before_guard = fixture.run_snapshot();
    let guarded = fixture.step();
    let guard_diagnostic = combined(&guarded).to_ascii_lowercase();
    assert!(
        guard_diagnostic.contains("step refused"),
        "the missing gate must refuse the step: {guard_diagnostic}"
    );
    assert!(
        guard_diagnostic.contains("gate.txt"),
        "the readiness-guard refusal must be reported first: {guard_diagnostic}"
    );
    assert!(
        !guard_diagnostic.contains("poison"),
        "an unknown verdict field must not be observed before guards pass: {guard_diagnostic}"
    );
    assert_eq!(fixture.run_snapshot(), before_guard);

    fs::write(fixture.root.join("gate.txt"), "ready\n").expect("satisfy readiness guard");
    let after_guard = fixture.step();
    let verdict_diagnostic = combined(&after_guard).to_ascii_lowercase();
    assert!(
        verdict_diagnostic.contains("step refused"),
        "the same non-strict verdict must refuse after the guard passes: {verdict_diagnostic}"
    );
    assert!(
        verdict_diagnostic.contains("verdict")
            && (verdict_diagnostic.contains("poison")
                || verdict_diagnostic.contains("unknown")
                || verdict_diagnostic.contains("extra")),
        "after guards pass the strict-record defect must be diagnosed: {verdict_diagnostic}"
    );
    assert_eq!(fixture.run_snapshot(), before_guard);

    let cases = vec![
        ("empty document", Vec::new(), vec![]),
        (
            "malformed TOML",
            b"phase = \"review\"\ninput = [\n".to_vec(),
            vec![],
        ),
        (
            "missing phase",
            b"input = \"approve\"\nrationale = \"why\"\n".to_vec(),
            vec!["phase"],
        ),
        (
            "missing input",
            b"phase = \"review\"\nrationale = \"why\"\n".to_vec(),
            vec!["input"],
        ),
        (
            "missing rationale",
            b"phase = \"review\"\ninput = \"approve\"\n".to_vec(),
            vec!["rationale"],
        ),
        ("unknown extra field", poison.to_vec(), vec![]),
        (
            "non-string phase",
            b"phase = 7\ninput = \"approve\"\nrationale = \"why\"\n".to_vec(),
            vec!["phase"],
        ),
        (
            "non-string input",
            b"phase = \"review\"\ninput = [\"approve\"]\nrationale = \"why\"\n".to_vec(),
            vec!["input"],
        ),
        (
            "non-string rationale",
            b"phase = \"review\"\ninput = \"approve\"\nrationale = false\n".to_vec(),
            vec!["rationale"],
        ),
        ("empty phase", verdict("", "approve", "why"), vec!["phase"]),
        ("empty input", verdict("review", "", "why"), vec!["input"]),
        (
            "empty rationale",
            verdict("review", "approve", ""),
            vec!["rationale"],
        ),
        (
            "blank rationale",
            verdict("review", "approve", " \t "),
            vec!["rationale"],
        ),
        (
            "stale phase",
            verdict("previous", "approve", "stale review"),
            vec!["phase"],
        ),
        (
            "input outside the closed list",
            verdict("review", "escalate", "not a legal route"),
            vec!["input"],
        ),
    ];

    for (label, bytes, expected) in cases {
        fixture.publish(&bytes);
        assert_refuses_unchanged(&fixture, label, &expected);
    }

    fs::remove_file(fixture.verdict_path()).expect("clear final defective verdict");
    assert_refuses_unchanged(&fixture, "missing live record", &[]);
    assert_eq!(fixture.phase(), "review");
    assert!(fixture.archives().is_empty());
}

/// PT-062-04 / FDCV-015--FDCV-017: deterministic faults expose all three
/// boundaries. Before consumption, retry may use the intact live record. After
/// consumption, retry cannot replay the archive and needs a fresh record. A
/// post-state fault observes an already advanced Run and cannot duplicate its
/// archive on retry.
#[test]
fn interruption_windows_preserve_consume_then_advance() {
    let before_archive = Fixture::new("fault-before-archive", TERMINAL_BRANCH_RUNBOOK);
    let first = verdict("review", "approve", "survives before archive");
    before_archive.publish(&first);
    let pristine = before_archive.run_snapshot();
    let interrupted = before_archive.step_with_fault(Some("before-verdict-archive"));
    assert!(
        !interrupted.status.success(),
        "the before-verdict-archive hook must interrupt the step"
    );
    assert_eq!(
        before_archive.run_snapshot(),
        pristine,
        "a pre-consumption fault leaves old state and the complete live record byte-identical"
    );
    assert_eq!(fs::read(before_archive.verdict_path()).unwrap(), first);
    assert!(before_archive.archives().is_empty());
    assert_step_succeeds(
        &before_archive.step(),
        "retry before consumption may consume the still-live record",
    );
    assert_eq!(before_archive.phase(), "approved");
    assert!(!before_archive.verdict_path().exists());
    assert_eq!(
        before_archive.archives(),
        BTreeMap::from([("000001.toml".to_owned(), first)])
    );

    let before_state = Fixture::new("fault-before-state", TERMINAL_BRANCH_RUNBOOK);
    let consumed = verdict("review", "approve", "consumed before state replacement");
    before_state.publish(&consumed);
    let old_state = before_state.state();
    let interrupted = before_state.step_with_fault(Some("before-state-replace"));
    assert!(
        !interrupted.status.success(),
        "the before-state-replace hook must interrupt the step"
    );
    assert_eq!(
        before_state.state(),
        old_state,
        "the State File remains in the old Phase after consumption"
    );
    assert!(!before_state.verdict_path().exists());
    assert_eq!(
        before_state.archives(),
        BTreeMap::from([("000001.toml".to_owned(), consumed.clone())])
    );

    let consumed_snapshot = before_state.run_snapshot();
    let replay = before_state.step();
    let replay_diagnostic = combined(&replay).to_ascii_lowercase();
    assert!(
        replay_diagnostic.contains("step refused") && replay_diagnostic.contains("verdict"),
        "retry must refuse and explain that a fresh live verdict is required: {replay_diagnostic}"
    );
    assert_eq!(
        before_state.run_snapshot(),
        consumed_snapshot,
        "a replay attempt cannot duplicate, move, or rewrite consumed evidence"
    );

    let fresh = verdict("review", "approve", "fresh retry decision");
    before_state.publish(&fresh);
    assert_step_succeeds(
        &before_state.step(),
        "a fresh verdict completes a consumed-before-state retry",
    );
    assert_eq!(before_state.phase(), "approved");
    let archives = before_state.archives();
    assert_eq!(
        archives.keys().collect::<Vec<_>>(),
        ["000001.toml", "000002.toml"]
    );
    assert_eq!(archives["000001.toml"], consumed);
    assert_eq!(archives["000002.toml"], fresh);

    let after_state = Fixture::new("fault-after-state", TERMINAL_BRANCH_RUNBOOK);
    let advanced = verdict("review", "rework", "state replacement completed");
    after_state.publish(&advanced);
    let interrupted = after_state.step_with_fault(Some("after-state-replace"));
    assert!(
        !interrupted.status.success(),
        "the after-state-replace hook must report the injected interruption"
    );
    assert_eq!(
        after_state.phase(),
        "rework",
        "a post-replacement fault observes the successor State File"
    );
    assert!(!after_state.verdict_path().exists());
    assert_eq!(
        after_state.archives(),
        BTreeMap::from([("000001.toml".to_owned(), advanced)])
    );

    let advanced_snapshot = after_state.run_snapshot();
    let _retry = after_state.step();
    assert_eq!(
        after_state.run_snapshot(),
        advanced_snapshot,
        "retry after completed state replacement cannot replay or duplicate the archive"
    );
    assert_eq!(after_state.phase(), "rework");
    assert_eq!(
        after_state.archives().keys().collect::<Vec<_>>(),
        ["000001.toml"]
    );
}
