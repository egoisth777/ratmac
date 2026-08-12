//! t-090 / PCR-009: deliberate damage refuses on an uncommitted tree.
//!
//! PCRV-007 `the_damage_step_refuses_until_the_tree_is_committed`
//!
//! The safety-commit rule stops being prose. A command-class Exit Guard on
//! the step into the damage stage observes whether the working tree holds an
//! uncommitted change to a tracked file. While one exists the step refuses,
//! naming the observed fact against the expected one and leaving State and
//! Status untouched; committing the change makes the identical step succeed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A two-State machine whose exit from `build` is the checkpoint guard. The
/// guard is the version-control command itself: exit `0` is a tree with no
/// uncommitted change to a tracked file, and its output names the files that
/// make the answer otherwise.
const RUNBOOK: &str = r#"
[states.build]
prompt = "Build, then checkpoint before damaging anything."
guards = [{ kind = "command_exit", program = "git", args = ["diff", "--stat", "--exit-code", "HEAD"], expected = 0, exempt = true }]

[states.damage]
prompt = "Damage one line, run the test, restore."

[[transitions]]
from = "build"
to = "damage"
"#;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn create(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t090-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in [".arca/goal", ".ratmac", "src"] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        fs::write(root.join(".arca/goal/spec.md"), "# Fixture goal\n").expect("write fixture goal");
        fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").expect("write fixture source");
        fs::write(root.join(".ratmac/ratmac.toml"), RUNBOOK).expect("write fixture runbook");
        let fixture = Self { root };
        fixture.git(&["init", "--quiet"]);
        fixture.git(&["config", "user.email", "fixture@example.invalid"]);
        fixture.git(&["config", "user.name", "Fixture"]);
        // Without this, git warns about line endings on stderr and the file
        // would be named by platform accident rather than by the guard.
        fixture.git(&["config", "core.autocrlf", "false"]);
        // The Engine's runtime is never a tracked file.
        fs::write(fixture.root.join(".gitignore"), ".ratmac/\n").expect("write fixture ignores");
        fixture.commit("fixture checkpoint");
        fixture
    }

    fn git(&self, args: &[&str]) -> Output {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke git");
        assert!(
            output.status.success(),
            "git {args:?} succeeds: {}",
            combined(&output)
        );
        output
    }

    fn commit(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "--quiet", "-m", message]);
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    fn start(&self) -> String {
        let output = self.rtm(&["start"]);
        let text = combined(&output);
        assert!(output.status.success(), "start succeeds: {text}");
        text.split("started run ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .expect("start names the minted run id")
            .to_owned()
    }

    fn record(&self, run: &str) -> String {
        fs::read_to_string(self.root.join(format!(".ratmac/runs/{run}/run.toml")))
            .expect("read the Run Record")
    }

    fn transition_log(&self) -> String {
        fs::read_to_string(self.root.join(".ratmac/log.md")).unwrap_or_default()
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

/// Every tracked path with its bytes, plus the index, so "the guard touched
/// nothing" is provable rather than asserted.
fn tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
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
                files.push((path, bytes));
            }
        }
    }
    files.sort();
    files
}

/// PCRV-007: the step into the damage stage refuses while a tracked file
/// carries an uncommitted change, names the observed fact against the
/// expected one, leaves State and Status untouched, and succeeds unchanged
/// once the change is committed.
#[test]
fn the_damage_step_refuses_until_the_tree_is_committed() {
    let fixture = Fixture::create("checkpoint");
    let run = fixture.start();
    let before_record = fixture.record(&run);
    let before_log = fixture.transition_log();
    assert!(
        before_record.contains("state = \"build\""),
        "the Run is parked on the step before damage: {before_record}"
    );

    // A tracked file changed and not committed - exactly the condition that
    // once destroyed a file in this repository.
    fs::write(
        fixture.root.join("src/lib.rs"),
        "pub fn fixture() { /* uncommitted */ }\n",
    )
    .expect("modify a tracked file");
    let before_tree = tree(&fixture.root);

    let refusal = combined(&fixture.rtm(&["step", "--run", &run]));
    assert!(
        refusal.contains("step refused"),
        "the damage step refuses on an uncommitted tree: {refusal}"
    );
    assert!(
        refusal.contains("src/lib.rs") || refusal.contains("src\\lib.rs"),
        "the refusal names the observed fact - the file that is not checkpointed: {refusal}"
    );
    assert!(
        refusal.contains("exit 0"),
        "the refusal names the expected fact: {refusal}"
    );
    assert!(
        refusal.contains("diagnostic (stdout)"),
        "ETB-002: the refusal labels the channel the observed fact came from, \
         so a reader is never guessing which one spoke: {refusal}"
    );
    assert_eq!(
        fixture.record(&run),
        before_record,
        "R-017: a refused step leaves the Run Record byte-identical"
    );
    assert_eq!(
        fixture.transition_log(),
        before_log,
        "a refused step appends no transition entry"
    );
    assert_eq!(
        tree(&fixture.root),
        before_tree,
        "the guard observes the tree and writes nothing"
    );

    // The identical step, after the checkpoint the rule demands.
    fixture.commit("checkpoint before damage");
    let accepted = combined(&fixture.rtm(&["step", "--run", &run]));
    assert!(
        !accepted.contains("step refused"),
        "the same step succeeds once the tree is committed: {accepted}"
    );
    assert!(
        fixture.record(&run).contains("state = \"damage\""),
        "the Run moves into the damage stage: {}",
        fixture.record(&run)
    );
}
