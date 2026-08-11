//! t-081 / SVC-004: the Run Record is `run.toml` with a `state` field.
//!
//! SVCV-004 `a_started_run_writes_run_toml_with_a_state_field`
//!
//! A started Run writes one file, `.ratmac/runs/<run-id>/run.toml`, whose
//! first field is `state` and which carries exactly seven fields. Nothing else
//! about the record moves: the strict parse still refuses a short record by
//! naming the missing field and leaves the bytes on disk untouched, and no
//! file is ever created at the pre-cutover name anywhere under the Engine
//! root.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const RUNBOOK: &str = r#"[roots]
ticket = ".arca/ticket"

[states.intake]
prompt = "Integrate the issues."

[states.build]
prompt = "Build the ticket."

[states.done]
prompt = "Nothing is left."

[[transitions]]
from = "intake"
to = "build"

[[transitions]]
from = "build"
to = "done"
"#;

/// The seven fields the record has always carried, with the position field
/// under its settled name.
const REQUIRED_FIELDS: [&str; 7] = [
    "state",
    "status",
    "goal_revision",
    "input_revision",
    "output_revision",
    "active_refs",
    "blocker",
];

struct Fixture {
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t081-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in [".arca/ticket", ".ratmac"] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        fs::write(root.join(".ratmac/ratmac.toml"), RUNBOOK).expect("write machine class");
        Fixture { root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("run rtm")
    }

    fn engine_root(&self) -> PathBuf {
        self.root.join(".ratmac")
    }

    fn run_id(&self) -> String {
        fs::read_dir(self.engine_root().join("runs"))
            .expect("list the runs roster")
            .map(|entry| entry.expect("roster entry is readable"))
            .find(|entry| entry.path().is_dir())
            .expect("the started run appears on the roster")
            .file_name()
            .to_string_lossy()
            .into_owned()
    }

    fn record_path(&self, run_id: &str) -> PathBuf {
        self.engine_root()
            .join("runs")
            .join(run_id)
            .join("run.toml")
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Every file path below `dir`, so a check can say "nowhere under the Engine
/// root" and mean it - leftover temporaries included.
fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, found);
        } else {
            found.push(path);
        }
    }
}

/// The record's top-level keys, in the order they were written.
fn field_order(record: &str) -> Vec<String> {
    record
        .lines()
        .filter_map(|line| line.split_once(" = "))
        .map(|(key, _)| key.trim().to_owned())
        .collect()
}

fn assert_no_precutover_name(fixture: &Fixture, after: &str) {
    let mut found = Vec::new();
    walk(&fixture.engine_root(), &mut found);
    let offenders = found
        .iter()
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().contains("state.toml"))
        })
        .collect::<Vec<_>>();
    assert!(
        offenders.is_empty(),
        "no file carries the pre-cutover Run Record name after {after}: {offenders:?}"
    );
}

#[test]
fn a_started_run_writes_run_toml_with_a_state_field() {
    let fixture = Fixture::new("record");

    let start = fixture.rtm(&["start"]);
    assert!(start.status.success(), "the Run starts: {}", text(&start));
    let run_id = fixture.run_id();
    let record_path = fixture.record_path(&run_id);
    assert!(
        record_path.is_file(),
        "a started Run writes {}",
        record_path.display()
    );
    assert_no_precutover_name(&fixture, "start");

    let record = fs::read_to_string(&record_path).expect("read the Run Record");
    let fields = field_order(&record);
    assert_eq!(
        fields.first().map(String::as_str),
        Some("state"),
        "the position field is `state` and it is written first: {record}"
    );
    assert_eq!(
        fields.len(),
        REQUIRED_FIELDS.len(),
        "the record still carries exactly seven fields: {record}"
    );
    for field in REQUIRED_FIELDS {
        assert!(
            fields.iter().any(|written| written == field),
            "the record carries `{field}`: {record}"
        );
    }
    assert!(
        record.contains("state = \"intake\""),
        "the position field carries the State the Run sits at: {record}"
    );

    let step = fixture.rtm(&["step", "--run", &run_id]);
    assert!(
        step.status.success(),
        "the Run takes its first edge: {}",
        text(&step)
    );
    let stepped = fs::read_to_string(&record_path).expect("read the Run Record after a step");
    assert!(
        stepped.contains("state = \"build\""),
        "the same single file records the new State: {stepped}"
    );
    assert_no_precutover_name(&fixture, "a step");
    let mut after_step = Vec::new();
    walk(
        &fixture.engine_root().join("runs").join(&run_id),
        &mut after_step,
    );
    let records = after_step
        .iter()
        .filter(|path| {
            fs::read_to_string(path).is_ok_and(|body| {
                body.lines().any(|line| line.starts_with("state = "))
                    && body.lines().any(|line| line.starts_with("blocker = "))
            })
        })
        .count();
    assert_eq!(
        records, 1,
        "one Run writes one Run Record, not two: {after_step:?}"
    );

    // The strict parse is unchanged: a short record refuses by naming the
    // missing field, and the refusal leaves the bytes exactly as they were.
    let short = stepped
        .lines()
        .filter(|line| !line.starts_with("blocker = "))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&record_path, format!("{short}\n")).expect("plant a short record");
    let planted = fs::read(&record_path).expect("read the planted bytes");

    let refused = fixture.rtm(&["status", "--run", &run_id]);
    let refusal = text(&refused);
    assert!(
        !refused.status.success(),
        "a short record refuses: {refusal}"
    );
    assert!(
        refusal.contains("missing required field blocker"),
        "the refusal names the missing field in the pre-existing words: {refusal}"
    );
    assert!(
        refusal.contains("run.toml") && !refusal.contains("state.toml"),
        "the refusal names the record by its settled filename: {refusal}"
    );
    assert_eq!(
        fs::read(&record_path).expect("read the record after the refusal"),
        planted,
        "a refused read writes nothing back"
    );
    assert_no_precutover_name(&fixture, "a refused read");
}
