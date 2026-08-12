//! t-089 / PCR-007: a per-item gate addressed by a binding.
//!
//! PCRV-005 `a_bound_gate_grades_each_child_against_its_own_receipts`
//!
//! A receipt-class or completion-class guard names what it judges without the
//! identifier being written in the runbook: the guard declares a binding name,
//! the caller supplies the value at `rtm spawn --bind`, and the Engine reads
//! that value back from the append-only spawn ledger. Two children spawned
//! from one class are graded against their own evidence, the runbook file
//! carries neither address, a guard declaring both address forms refuses at
//! parse under its own code, and a guard naming a binding nobody supplied
//! refuses at dispatch without writing anything.

use ratmac::receipt::sha256_text;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// One child class whose completion gate is addressed by the binding `item`.
/// No ticket identifier appears anywhere in this file - that absence is half
/// the claim under test.
const BOUND_RUNBOOK: &str = r#"
[roots]
ticket = ".arca/ticket"

[classes.worker.bindings.item]
required = true

[classes.worker.states.work]
prompt = "Work the bound item."
guards = [{ kind = "completion_gate", root = "ticket", ticket-binding = "item" }]

[classes.worker.states.done]
prompt = "Done."

[[classes.worker.transitions]]
from = "work"
to = "done"

[states.plan]
prompt = "Plan."

[states.delegate]
prompt = "Delegate and wait."
guards = [{ kind = "join", require = "all_passed", min = 1 }]

[[states.delegate.spawns]]
class = "worker"
name = "item"
bind = ["item"]

[states.done]
prompt = "Done."

[[transitions]]
from = "plan"
to = "delegate"

[[transitions]]
from = "delegate"
to = "done"
"#;

/// The same class with the binding name nobody supplies at spawn.
const UNSUPPLIED_RUNBOOK: &str = r#"
[roots]
ticket = ".arca/ticket"

[classes.worker.bindings.item]
required = false

[classes.worker.states.work]
prompt = "Work the bound item."
guards = [{ kind = "completion_gate", root = "ticket", ticket-binding = "absent" }]

[classes.worker.states.done]
prompt = "Done."

[[classes.worker.transitions]]
from = "work"
to = "done"

[states.plan]
prompt = "Plan."

[states.delegate]
prompt = "Delegate and wait."
guards = [{ kind = "join", require = "all_passed", min = 1 }]

[[states.delegate.spawns]]
class = "worker"
name = "item"
bind = ["item"]

[states.done]
prompt = "Done."

[[transitions]]
from = "plan"
to = "delegate"

[[transitions]]
from = "delegate"
to = "done"
"#;

/// A guard that declares both address forms at once.
const BOTH_FORMS_RUNBOOK: &str = r#"
[states.work]
prompt = "Work."
guards = [{ kind = "completion_gate", ticket = "t-100.md", ticket-binding = "item" }]

[states.done]
prompt = "Done."

[[transitions]]
from = "work"
to = "done"
"#;

/// A guard that declares neither.
const NO_FORM_RUNBOOK: &str = r#"
[states.work]
prompt = "Work."
guards = [{ kind = "completion_gate" }]

[states.done]
prompt = "Done."

[[transitions]]
from = "work"
to = "done"
"#;

const GREEN: &str = "test result: ok. 1 passed; 0 failed\n";

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn create(label: &str, runbook: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t089-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in [".arca/goal", ".arca/ticket", ".ratmac", "src"] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        fs::write(root.join(".arca/goal/spec.md"), "# Fixture goal\n").expect("write fixture goal");
        fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").expect("write fixture source");
        fs::write(root.join(".ratmac/ratmac.toml"), runbook).expect("write fixture runbook");
        Self { root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    /// Start the parent and step it into the spawning State.
    fn start_at_delegate(&self) -> String {
        let start = self.rtm(&["start"]);
        let text = combined(&start);
        assert!(start.status.success(), "start succeeds: {text}");
        let parent = text
            .split("started run ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .expect("start names the minted run id")
            .to_owned();
        let step = self.rtm(&["step", "--run", &parent]);
        assert!(
            step.status.success(),
            "step into delegate succeeds: {}",
            combined(&step)
        );
        parent
    }

    /// Spawn one child, optionally binding `item`.
    fn spawn(&self, parent: &str, bind: Option<&str>) -> String {
        let pair = bind.map(|value| format!("item={value}"));
        let mut args = vec!["spawn", "item", "--run", parent];
        if let Some(pair) = pair.as_deref() {
            args.push("--bind");
            args.push(pair);
        }
        let output = self.rtm(&args);
        let text = combined(&output);
        assert!(output.status.success(), "spawn succeeds: {text}");
        text.split("spawned run ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .expect("spawn names the child run id")
            .to_owned()
    }

    /// A ticket declaring exactly one focused test.
    fn write_ticket(&self, ticket: &str, planned: &str) {
        fs::write(
            self.root.join(format!(".arca/ticket/{ticket}.md")),
            format!(
                "---\nticket-id: {ticket}\nresidual-ids:\n  - \"res-900\"\n\
                 planned-test-refs:\n  - \"{planned}\"\nstatus: \"executing\"\n---\n\n\
                 # Ticket: {ticket}\n\n## Merge Gate\n\n- Focused test only.\n"
            ),
        )
        .expect("write fixture ticket");
    }

    /// One green, fresh completion receipt for `check`, recorded for `run`.
    fn write_receipt(&self, run: &str, ticket: &str, check: &str) {
        let directory = self.root.join(format!(".ratmac/evidence/{run}/completion"));
        fs::create_dir_all(&directory).expect("create evidence directory");
        let digest = ratmac::completion::tree_digest(&self.root, &["src".to_owned()])
            .expect("source roots are readable");
        let body = format!(
            "ticket-id = \"{ticket}\"\n\
             check-id = \"{check}\"\n\
             kind = \"focused\"\n\
             command = \"cargo test --test t089\"\n\
             working-dir = \".\"\n\
             exit-status = 0\n\
             output-sha256 = \"{}\"\n\
             tree-roots = [\"src\"]\n\
             tree-sha256 = \"{digest}\"\n\
             output = \"\"\"\n{GREEN}\"\"\"\n",
            sha256_text(GREEN)
        );
        fs::write(directory.join(format!("{}.toml", slug(check))), body)
            .expect("write completion receipt");
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

fn slug(check: &str) -> String {
    check
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// Every file under `root` with its bytes, so an absence claim is provable.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(bytes) = fs::read(&path) {
                files.insert(path, bytes);
            }
        }
    }
    files
}

/// PCRV-005: two children of one class are graded against their own receipts,
/// the runbook names neither address, and both malformed declarations refuse
/// with their own diagnostic while writing nothing.
#[test]
fn a_bound_gate_grades_each_child_against_its_own_receipts() {
    let fixture = Fixture::create("bound", BOUND_RUNBOOK);
    fixture.write_ticket("t-100", "PT-100-01");
    fixture.write_ticket("t-101", "PT-101-01");

    let parent = fixture.start_at_delegate();
    let first = fixture.spawn(&parent, Some("t-100.md"));
    let second = fixture.spawn(&parent, Some("t-101.md"));

    // Each child carries the evidence for its own item only.
    fixture.write_receipt(&first, "t-100", "PT-100-01");
    fixture.write_receipt(&second, "t-101", "PT-101-01");

    let step = fixture.rtm(&["step", "--run", &first]);
    let text = combined(&step);
    assert!(
        step.status.success() && !text.contains("step refused"),
        "the first child passes on its own receipts: {text}"
    );
    let state = combined(&fixture.rtm(&["status", "--run", &first]));
    assert!(
        state.contains("State: done"),
        "the first child reaches the State beyond its gate; got:\n{state}"
    );

    // The second child's evidence is deliberately the first child's: the gate
    // must grade it against `t-101` and refuse.
    fs::remove_dir_all(
        fixture
            .root
            .join(format!(".ratmac/evidence/{second}/completion")),
    )
    .expect("clear the second child's evidence");
    fixture.write_receipt(&second, "t-100", "PT-100-01");
    let step = fixture.rtm(&["step", "--run", &second]);
    let text = combined(&step);
    assert!(
        text.contains("step refused"),
        "a child graded against a neighbour's receipts refuses: {text}"
    );
    assert!(
        text.contains("PT-101-01"),
        "the refusal names the check the second child's own item declares; got:\n{text}"
    );

    // The runbook file carries neither address value.
    let runbook = fs::read_to_string(fixture.root.join(".ratmac/ratmac.toml"))
        .expect("read the fixture runbook");
    for value in ["t-100", "t-101"] {
        assert!(
            !runbook.contains(value),
            "the runbook file names no item; found {value:?} in:\n{runbook}"
        );
    }
}

/// PCRV-005: a guard declaring both address forms, or neither, refuses at
/// parse under its own diagnostic code.
#[test]
fn a_guard_declaring_both_forms_or_neither_refuses_at_parse() {
    for (label, runbook, expected) in [
        ("both", BOTH_FORMS_RUNBOOK, "both"),
        ("neither", NO_FORM_RUNBOOK, "neither"),
    ] {
        let fixture = Fixture::create(&format!("parse-{label}"), runbook);
        let output = fixture.rtm(&["doctor"]);
        let text = combined(&output);
        assert!(
            !output.status.success(),
            "the {label} form is not a runbook: {text}"
        );
        assert!(
            text.contains("RB112"),
            "the {label} form refuses under its own code; got:\n{text}"
        );
        assert!(
            text.contains(expected),
            "the {label} refusal says which side is wrong; got:\n{text}"
        );
    }
}

/// PCRV-005: a bound guard whose binding nobody supplied refuses at dispatch,
/// names the binding, and writes nothing.
#[test]
fn an_unsupplied_binding_refuses_at_dispatch_without_writing() {
    let fixture = Fixture::create("unsupplied", UNSUPPLIED_RUNBOOK);
    fixture.write_ticket("t-100", "PT-100-01");

    let parent = fixture.start_at_delegate();
    let child = fixture.spawn(&parent, Some("t-100.md"));
    fixture.write_receipt(&child, "t-100", "PT-100-01");

    let before = snapshot(&fixture.root);
    let step = fixture.rtm(&["step", "--run", &child]);
    let text = combined(&step);
    assert!(
        text.contains("step refused"),
        "a binding nobody supplied cannot grade anything: {text}"
    );
    assert!(
        text.contains("absent"),
        "the refusal names the unsupplied binding; got:\n{text}"
    );
    assert_eq!(
        snapshot(&fixture.root),
        before,
        "a refusing dispatch writes nothing"
    );
}
