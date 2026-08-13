//! t-094 / EDN-002: the cycle cannot reach rest unmarked.
//!
//! `EDNV-001` `the_close_step_refuses_until_an_edition_is_cut`
//! `EDNV-002` `the_guard_reads_version_control_not_the_tree`
//! `EDNV-003` `a_refused_close_writes_nothing`
//!
//! Self-development makes the base a correctness input: work built on a commit
//! that was never green is measured against nothing. An edition is the mark
//! that says a commit met the bar, and the closing State's Exit Guard is what
//! stops a sprint from reaching rest without one. The guard is a read-only
//! version-control command, so the Engine still knows nothing about tags.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A two-State machine whose exit from `close` carries the edition guard. The
/// probe is the version-control command itself: exit `0` means a tag matching
/// `edition-*` names the commit being left, and its output names the tag.
const RUNBOOK: &str = r#"
[states.close]
prompt = "Close the turn, then cut the edition."
guards = [{ kind = "command_exit", program = "git", args = ["describe", "--exact-match", "--match", "edition-*"], expected = 0 }]

[states.rest]
prompt = "Nothing is open."

[[transitions]]
from = "close"
to = "rest"
"#;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn create(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t094-{label}-{}-{}",
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
        fixture.git(&["config", "core.autocrlf", "false"]);
        // The Engine's runtime is never a tracked file.
        fs::write(fixture.root.join(".gitignore"), ".ratmac/\n").expect("write fixture ignores");
        fixture.commit("fixture base");
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

    /// Cut an edition the way the working rules describe it: an annotated tag
    /// whose message records what was proven there.
    fn cut_edition(&self, name: &str) {
        self.git(&["tag", "-a", name, "-m", "fixture edition: every gate green"]);
    }

    fn head(&self) -> String {
        String::from_utf8_lossy(&self.git(&["rev-parse", "HEAD"]).stdout)
            .trim()
            .to_owned()
    }

    fn tags(&self) -> String {
        String::from_utf8_lossy(&self.git(&["tag", "--list"]).stdout).into_owned()
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

    fn step(&self, run: &str) -> String {
        combined(&self.rtm(&["step", "--run", run]))
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

/// Every tracked path with its bytes, so "the guard touched nothing" is
/// provable rather than asserted. The Engine's runtime directory is excluded
/// because a first guard evaluation legitimately records the command's pin in
/// Run evidence (`ETB-001`); the Run's own position and log are asserted
/// separately, and no tracked file may move either way.
fn tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .file_name()
                .is_some_and(|name| name == ".git" || name == ".ratmac")
            {
                continue;
            }
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

/// `EDNV-001`: the step out of the closing State refuses while no edition
/// names the commit, and the identical step succeeds once one does.
#[test]
fn the_close_step_refuses_until_an_edition_is_cut() {
    let fixture = Fixture::create("unmarked");
    let run = fixture.start();

    let refusal = fixture.step(&run);
    assert!(
        refusal.contains("step refused"),
        "the close step refuses while no edition names the commit: {refusal}"
    );
    assert!(
        refusal.contains("command_exit"),
        "the refusal names the guard class: {refusal}"
    );
    assert!(
        refusal.contains("git"),
        "the refusal names the command it ran: {refusal}"
    );
    assert!(
        refusal.contains("observed exit"),
        "the refusal names the observed exit code: {refusal}"
    );
    assert!(
        fixture.record(&run).contains("state = \"close\""),
        "a refused step leaves the Run in the closing State"
    );

    // Nothing changes but the mark itself.
    fixture.cut_edition("edition-001");
    let accepted = fixture.step(&run);
    assert!(
        !accepted.contains("step refused"),
        "the identical step succeeds once the edition is cut: {accepted}"
    );
    assert!(
        fixture.record(&run).contains("state = \"rest\""),
        "the Run reaches rest: {}",
        fixture.record(&run)
    );
}

/// `EDNV-002`: the verdict comes from the tag database, not from the tree, so a
/// tag on another commit and a name outside the pattern both refuse.
#[test]
fn the_guard_reads_version_control_not_the_tree() {
    let elsewhere = Fixture::create("elsewhere");
    elsewhere.cut_edition("edition-001");
    // A later commit is not the marked one, even though the mark exists.
    fs::write(elsewhere.root.join("src/lib.rs"), "pub fn moved() {}\n")
        .expect("modify the fixture source");
    elsewhere.commit("work after the edition");
    let run = elsewhere.start();
    let refusal = elsewhere.step(&run);
    assert!(
        refusal.contains("step refused"),
        "an edition on another commit does not mark this one: {refusal}"
    );

    let mistyped = Fixture::create("mistyped");
    // A plausible near-miss: the shop's own plural, and a capital.
    mistyped.git(&["tag", "-a", "editions-001", "-m", "near miss"]);
    mistyped.git(&["tag", "-a", "Edition-001", "-m", "near miss"]);
    let run = mistyped.start();
    let refusal = mistyped.step(&run);
    assert!(
        refusal.contains("step refused"),
        "a tag whose name is outside the pattern is not an edition: {refusal}"
    );
}

/// `EDNV-003`: a refused close writes nothing - not the tree, not the Run
/// Record's position, not the transition log, and no tag.
#[test]
fn a_refused_close_writes_nothing() {
    let fixture = Fixture::create("readonly");
    let run = fixture.start();

    let before_tree = tree(&fixture.root);
    let before_record = fixture.record(&run);
    let before_log = fixture.transition_log();
    let before_tags = fixture.tags();
    let before_head = fixture.head();

    let refusal = fixture.step(&run);
    assert!(
        refusal.contains("step refused"),
        "the step refuses: {refusal}"
    );

    assert_eq!(
        tree(&fixture.root),
        before_tree,
        "a refused step writes no tracked file"
    );
    assert_eq!(
        fixture.record(&run),
        before_record,
        "a refused step leaves the Run Record byte-identical"
    );
    assert_eq!(
        fixture.transition_log(),
        before_log,
        "a refused step appends no transition entry"
    );
    assert_eq!(
        fixture.tags(),
        before_tags,
        "the guard creates and moves no tag"
    );
    assert_eq!(fixture.head(), before_head, "the guard commits nothing");
}

/// `EDNV-001`, shipped side: the rule is only real if this repository's own
/// Machine Class carries the guard. A fixture proving the guard behaves would
/// otherwise sit beside a cycle that never runs it.
#[test]
fn the_shipped_cycle_declares_the_edition_guard() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let shipped = fs::read_to_string(repo_root.join(".ratmac/ratmac.toml"))
        .expect("read the shipped Machine Class");
    let declared = shipped
        .split("[states.close]")
        .nth(1)
        .expect("the cycle declares a closing State")
        .split("\n[")
        .next()
        .expect("the closing State has a body");
    // Comments carry authoring intent and reach no agent, so a comment naming
    // the pattern must never be able to satisfy a check about the guard.
    let close = declared
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        close.contains("command_exit"),
        "the closing State carries a command-class guard: {close}"
    );
    assert!(
        close.contains("edition-*"),
        "the closing State's guard asks for an edition at this commit: {close}"
    );
    assert!(
        close.contains("record_contract"),
        "the closing State keeps the record contract it already carried: {close}"
    );
    // The whole rule turns on the sense of the check: a guard that demanded a
    // non-zero exit would pass exactly when the commit is unmarked.
    assert!(
        close.contains("expected = 0"),
        "the guard passes on success, not on failure: {close}"
    );
    assert!(
        close.contains("\"describe\"") && close.contains("\"--exact-match\""),
        "the guard asks version control which tag names this exact commit: {close}"
    );
}
