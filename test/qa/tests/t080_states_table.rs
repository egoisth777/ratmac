//! t-080 / SVC-003: the runbook declares States.
//!
//! SVCV-002 `a_states_runbook_parses_passes_the_doctor_and_runs`
//!
//! A runbook written the settled way - `[states.<name>]` under a top-level
//! `states` table, a State's children under `[[states.<name>.spawns]]`, and a
//! child class's machine under `[classes.<name>.states]` - parses, passes the
//! doctor clean, and drives a Run from start to a terminal State. Transition
//! `from` and `to` name States.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// A runbook that exercises every renamed table path at once: a top-level
/// `states` table, a spawn declaration under a State, and a child class whose
/// own machine lives under `[classes.<name>.states]`.
const RUNBOOK: &str = r#"[roots]
ticket = ".arca/ticket"

[states.intake]
prompt = "Integrate the issues."

[[states.intake.spawns]]
class = "reviewer"
name = "first-reviewer"
bind = ["scope"]

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

[classes.reviewer.states.review]
prompt = "Review the delivery."

[classes.reviewer.bindings.scope]
required = true
"#;

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
            "ratmac-t080-{label}-{}-{}",
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

    fn run_id(&self) -> String {
        fs::read_dir(self.root.join(".ratmac/runs"))
            .expect("list the runs roster")
            .map(|entry| entry.expect("roster entry is readable"))
            .find(|entry| entry.path().is_dir())
            .expect("the started run appears on the roster")
            .file_name()
            .to_string_lossy()
            .into_owned()
    }

    /// The Run Record's bytes, read without naming the file: this ticket
    /// renames the runbook's tables, and a sibling ticket renames the record's
    /// filename and field, so the check must not pin either spelling.
    fn record(&self, run_id: &str) -> String {
        let dir = self.root.join(".ratmac/runs").join(run_id);
        fs::read_dir(&dir)
            .expect("list the run directory")
            .map(|entry| entry.expect("run entry is readable").path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "toml")
            })
            .filter_map(|path| fs::read_to_string(path).ok())
            // The Run Record is the one carrying the Run's lifecycle fields;
            // the sibling evidence file carries hashes and no status.
            .find(|body| {
                body.lines().any(|line| line.starts_with("status = "))
                    && body.lines().any(|line| line.starts_with("blocker = "))
            })
            .unwrap_or_else(|| panic!("a Run Record exists under {}", dir.display()))
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The record names the position by value, whatever the field is called.
fn sits_at(record: &str, position: &str) -> bool {
    record
        .lines()
        .any(|line| line.trim().ends_with(&format!("= \"{position}\"")))
}

#[test]
fn a_states_runbook_parses_passes_the_doctor_and_runs() {
    let fixture = Fixture::new("settled");

    let doctor = fixture.rtm(&["doctor"]);
    let doctor_text = text(&doctor);
    assert!(
        doctor.status.success(),
        "the settled runbook passes the doctor: {doctor_text}"
    );
    for code in ["RB1", "RB2", "RB3", "RB5", "RB6"] {
        assert!(
            !doctor_text.contains(code),
            "the doctor reports no diagnostic over a settled runbook, found {code}: {doctor_text}"
        );
    }

    let start = fixture.rtm(&["start"]);
    assert!(start.status.success(), "the Run starts: {}", text(&start));
    let run_id = fixture.run_id();
    assert!(
        sits_at(&fixture.record(&run_id), "intake"),
        "a started Run sits at the first declared State: {}",
        fixture.record(&run_id)
    );

    let first = fixture.rtm(&["step", "--run", &run_id]);
    assert!(
        first.status.success(),
        "the Run leaves the first State: {}",
        text(&first)
    );
    assert!(
        sits_at(&fixture.record(&run_id), "build"),
        "the transition's `to` named a declared State: {}",
        fixture.record(&run_id)
    );

    let second = fixture.rtm(&["step", "--run", &run_id]);
    assert!(
        second.status.success(),
        "the Run reaches its terminal State: {}",
        text(&second)
    );
    assert!(
        sits_at(&fixture.record(&run_id), "done"),
        "the Run sits at the terminal State: {}",
        fixture.record(&run_id)
    );

    let past_the_end = text(&fixture.rtm(&["step", "--run", &run_id]));
    assert!(
        past_the_end.contains("refused") && past_the_end.contains("terminal"),
        "a terminal State has no outgoing edge to take: {past_the_end}"
    );
    assert!(
        sits_at(&fixture.record(&run_id), "done"),
        "a refused step leaves the Run where it was: {}",
        fixture.record(&run_id)
    );
}

/// The spawn declaration and the child class body are read from the renamed
/// table paths, not merely tolerated as unknown keys: a spawn naming a class
/// that is not declared under `[classes.<name>.states]` still refuses by its
/// own code, and a class body that declares no States refuses by its own code.
#[test]
fn the_child_class_and_its_spawn_are_read_from_the_states_tables() {
    let undeclared = Fixture::new("undeclared-class");
    fs::write(
        undeclared.root.join(".ratmac/ratmac.toml"),
        RUNBOOK.replace("class = \"reviewer\"", "class = \"absent\""),
    )
    .expect("write the runbook with an undeclared spawn class");
    let refusal = text(&undeclared.rtm(&["doctor"]));
    assert!(
        refusal.contains("RB504"),
        "an undeclared spawn class refuses with its own code: {refusal}"
    );

    let empty_class = Fixture::new("empty-class");
    fs::write(
        empty_class.root.join(".ratmac/ratmac.toml"),
        RUNBOOK.replace(
            "[classes.reviewer.states.review]\nprompt = \"Review the delivery.\"\n",
            "",
        ),
    )
    .expect("write the runbook with a State-less class");
    let empty = text(&empty_class.rtm(&["doctor"]));
    assert!(
        !empty.is_empty() && !empty_class.rtm(&["doctor"]).status.success(),
        "a class declaring no States under its own table refuses: {empty}"
    );

    let bad_binding = Fixture::new("unsupplied-binding");
    fs::write(
        bad_binding.root.join(".ratmac/ratmac.toml"),
        RUNBOOK.replace("bind = [\"scope\"]", "bind = []"),
    )
    .expect("write the runbook with an unsupplied required binding");
    let binding = text(&bad_binding.rtm(&["doctor"]));
    assert!(
        binding.contains("RB505"),
        "a spawn that does not cover the class's required bindings refuses by its own code: {binding}"
    );
}
