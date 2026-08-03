//! t-063 / FDC-002: the Engine, not the agent, writes the end of a Run.
//!
//! PT-063-01 `terminal_initial_phase_is_passed_on_start`
//! PT-063-02 `step_into_terminal_phase_writes_passed`
//! PT-063-03 `abandonment_records_event_before_state_retirement`
//! PT-063-04 `guard_refusal_is_non_terminal_and_failed_is_never_written`

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// One Phase, no transitions: structurally terminal from the first byte.
const TERMINAL_ONLY_RUNBOOK: &str = r#"
[phases.done]
prompt = "Everything is already done."
"#;

/// Two Phases on a straight line; `done` has no ordinary outgoing edge.
const STRAIGHT_RUNBOOK: &str = r#"
[phases.start]
prompt = "Start."

[phases.done]
prompt = "Done."

[[transitions]]
from = "start"
to = "done"
"#;

/// A branching Phase whose two labelled destinations are both terminal.
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

/// A straight line behind a readiness guard that fails until `gate.txt` says so.
const GUARDED_STRAIGHT_RUNBOOK: &str = r#"
[phases.start]
prompt = "Start."
guards = [{ kind = "file_contains", path = "gate.txt", contains = "ready" }]

[phases.done]
prompt = "Done."

[[transitions]]
from = "start"
to = "done"
"#;

struct Fixture {
    root: PathBuf,
    run_id: String,
}

impl Fixture {
    /// Create the project tree without starting: PT-063-01 observes start itself.
    fn create(label: &str, runbook: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t063-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".arca/goal")).expect("create fixture goal tree");
        fs::create_dir_all(root.join("src")).expect("create fixture source tree");
        fs::write(root.join(".arca/goal/spec.md"), "# Fixture goal\n").expect("write fixture goal");
        fs::write(root.join(".arca/ratmac.toml"), runbook).expect("write fixture runbook");
        fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").expect("write fixture source");
        Self {
            root,
            run_id: String::new(),
        }
    }

    fn start(&mut self) -> Output {
        let output = self.rtm(&["start"]);
        if output.status.success() {
            let mut roster = fs::read_dir(self.root.join(".arca/runs"))
                .expect("started fixture has a runs roster")
                .map(|entry| entry.expect("read roster entry"))
                .filter(|entry| entry.path().is_dir())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            roster.sort();
            self.run_id = roster.pop().expect("start mints a Run");
        }
        output
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    fn step(&self) -> Output {
        self.rtm(&["step", "--run", self.run_id.as_str()])
    }

    fn run_dir(&self) -> PathBuf {
        self.root.join(".arca/runs").join(&self.run_id)
    }

    fn state_path(&self) -> PathBuf {
        self.run_dir().join("state.toml")
    }

    fn state_bytes(&self) -> Vec<u8> {
        fs::read(self.state_path()).expect("read Run State File")
    }

    fn state_field(&self, field: &str) -> String {
        let state = String::from_utf8(self.state_bytes()).expect("State File is UTF-8");
        let parsed: toml::Value = state.parse().expect("State File is valid TOML");
        parsed[field]
            .as_str()
            .expect("State File carries a string field")
            .to_owned()
    }

    fn publish_verdict(&self, phase: &str, input: &str, rationale: &str) {
        fs::write(
            self.run_dir().join("verdict.toml"),
            format!("phase = {phase:?}\ninput = {input:?}\nrationale = {rationale:?}\n"),
        )
        .expect("publish live verdict fixture");
    }

    /// The exact phrase a human must type to retire this project's Run.
    fn abandon_phrase(&self) -> String {
        format!(
            "abandon {}",
            self.root
                .file_name()
                .expect("fixture has a directory name")
                .to_string_lossy()
        )
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

/// A step against a terminal Run must refuse by name and change nothing.
fn assert_terminal_refusal(fixture: &Fixture, label: &str) {
    let before = fixture.run_snapshot();
    let refused = fixture.step();
    let diagnostic = combined(&refused).to_ascii_lowercase();
    assert!(
        diagnostic.contains("terminal"),
        "FDC-002 {label}: a passed Run refuses by naming the terminal fact: {diagnostic}"
    );
    assert_eq!(
        fixture.run_snapshot(),
        before,
        "FDC-002 {label}: refusing a terminal Run changes no Run-owned byte"
    );
}

/// PT-063-01 / FDCV-011: `rtm start` beginning in a Phase with no ordinary
/// outgoing edge writes `passed` in the very first State File.
#[test]
fn terminal_initial_phase_is_passed_on_start() {
    let mut fixture = Fixture::create("terminal-start", TERMINAL_ONLY_RUNBOOK);
    let start = fixture.start();
    assert!(
        start.status.success(),
        "a terminal-initial start succeeds: {}",
        combined(&start)
    );
    assert_eq!(fixture.state_field("phase"), "done");
    assert_eq!(
        fixture.state_field("status"),
        "passed",
        "FDC-002: starting in a terminal Phase writes the Engine-owned passed fact"
    );
    assert_terminal_refusal(&fixture, "post-start step");
}

/// PT-063-02 / FDCV-012: arrival at a Phase with no ordinary outgoing edge
/// writes Phase and `passed` in one replacement — on a straight line and on a
/// verdict-routed branch, where the archive still precedes the passed state.
#[test]
fn step_into_terminal_phase_writes_passed() {
    let mut straight = Fixture::create("straight-arrival", STRAIGHT_RUNBOOK);
    assert!(straight.start().status.success(), "fixture start succeeds");
    assert_eq!(
        straight.state_field("status"),
        "planned",
        "a non-terminal initial Phase stays planned"
    );
    let advance = straight.step();
    assert!(
        advance.status.success(),
        "the straight advance succeeds: {}",
        combined(&advance)
    );
    assert_eq!(straight.state_field("phase"), "done");
    assert_eq!(
        straight.state_field("status"),
        "passed",
        "FDC-002: arrival at the terminal Phase writes passed in the same replacement"
    );
    assert_terminal_refusal(&straight, "post-arrival step");

    let mut branch = Fixture::create("branch-arrival", TERMINAL_BRANCH_RUNBOOK);
    assert!(branch.start().status.success(), "branch start succeeds");
    assert_eq!(
        branch.state_field("status"),
        "planned",
        "a branching initial Phase has ordinary outgoing edges and is not terminal"
    );
    branch.publish_verdict("review", "approve", "The packet is complete.");
    let advance = branch.step();
    assert!(
        advance.status.success(),
        "the approved branch advance succeeds: {}",
        combined(&advance)
    );
    assert_eq!(branch.state_field("phase"), "approved");
    assert_eq!(
        branch.state_field("status"),
        "passed",
        "FDC-002: a verdict-routed arrival at a terminal Phase writes passed"
    );
    assert!(
        branch.run_dir().join("verdicts/000001.toml").is_file(),
        "FDC-003 ordering holds: the verdict archived before the passed state"
    );
    assert!(
        !branch.run_dir().join("verdict.toml").exists(),
        "the live slot is cleared by consumption"
    );
}

/// PT-063-03 / FDCV-013: one durable `- Abandoned:` event naming the addressed
/// Run lands in append-only history before the admission state retires, and
/// `abandoned` never survives as a State File value.
#[test]
fn abandonment_records_event_before_state_retirement() {
    let mut fixture = Fixture::create("abandon-event", STRAIGHT_RUNBOOK);
    assert!(fixture.start().status.success(), "fixture start succeeds");
    let log_before =
        fs::read_to_string(fixture.root.join(".arca/log.md")).expect("read fixture log");
    let phrase = fixture.abandon_phrase();

    let output = fixture.rtm(&[
        "abandon",
        "--confirm",
        &phrase,
        "--run",
        fixture.run_id.as_str(),
    ]);
    assert!(
        output.status.success(),
        "a confirmed abandonment succeeds: {}",
        combined(&output)
    );

    let log = fs::read_to_string(fixture.root.join(".arca/log.md")).expect("read fixture log");
    assert!(
        log.starts_with(&log_before),
        "history is append-only across abandonment"
    );
    let event = log
        .strip_prefix(&log_before)
        .expect("the terminal event is appended");
    assert_eq!(
        event.matches("- Abandoned:").count(),
        1,
        "exactly one terminal event is appended: {event:?}"
    );
    assert!(
        event.contains(&fixture.run_id),
        "FDC-002: the durable terminal event names the addressed Run: {event:?}"
    );
    assert!(
        event.contains("start"),
        "the event names the retired Run's last Phase: {event:?}"
    );
    assert!(
        event.contains("status planned"),
        "the event records the last real lifecycle value, never abandoned: {event:?}"
    );

    assert!(
        !fixture.state_path().exists(),
        "the admission state is retired after the event"
    );
    assert!(
        fixture.run_dir().is_dir(),
        "the run directory remains to reserve the id"
    );
    assert!(
        !log.contains("status = \"abandoned\""),
        "abandoned never appears as a State File value anywhere"
    );
}

/// PT-063-04 / FDCV-014: a guard refusal is non-terminal and byte-identical,
/// and no operational path ever writes the deferred `failed` outcome.
#[test]
fn guard_refusal_is_non_terminal_and_failed_is_never_written() {
    let mut fixture = Fixture::create("guard-refusal", GUARDED_STRAIGHT_RUNBOOK);
    assert!(fixture.start().status.success(), "fixture start succeeds");
    assert_eq!(fixture.state_field("status"), "planned");

    let before = fixture.run_snapshot();
    let refused = fixture.step();
    let diagnostic = combined(&refused).to_ascii_lowercase();
    assert!(
        diagnostic.contains("refused"),
        "the failing readiness guard refuses the step: {diagnostic}"
    );
    assert_eq!(
        fixture.run_snapshot(),
        before,
        "FDC-002: guard refusal leaves the whole Run byte-identical"
    );
    assert_eq!(
        fixture.state_field("status"),
        "planned",
        "refusal is non-terminal: the lifecycle value is untouched"
    );

    fs::write(fixture.root.join("gate.txt"), "ready\n").expect("satisfy the readiness guard");
    let advance = fixture.step();
    assert!(
        advance.status.success(),
        "the satisfied guard lets the Run advance: {}",
        combined(&advance)
    );
    assert_ne!(
        fixture.state_field("status"),
        "failed",
        "FDC-002: no Engine path writes the deferred failed outcome"
    );
    assert_eq!(
        fixture.state_field("status"),
        "passed",
        "the advance ended in the terminal Phase with the Engine-owned passed fact"
    );
}
