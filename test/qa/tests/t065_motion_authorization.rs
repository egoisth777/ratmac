//! t-065 / FDC-007: authorization splits by motion kind.
//!
//! PT-065-01 `spawn_is_ordinary_checked_motion_without_phrase`
//! PT-065-02 `respawn_requires_phrase_naming_the_run_id`
//! PT-065-03 `abandon_phrase_names_the_run_id`
//!
//! `rtm spawn` is ordinary checked motion: no confirmation phrase, legal only
//! while the parent occupies the spawning State and only for a declared spawn.
//! `rtm respawn --run <id>` and abandon-with-run-id refuse without a
//! confirmation phrase naming that run id; the phrase is typed at invocation,
//! never read from a file. Every refusal is behavioral evidence - exit code
//! and message - with state, history, and lock byte-identical.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The t-064 composed machine: one declared child class, one spawning State
/// (`delegate`), one join-guarded out-edge. `plan` is the initial State.
const COMPOSED_RUNBOOK: &str = r#"
[classes.reviewer.bindings.ticket]
required = true

[classes.reviewer.states.review]
prompt = "Review the delegated ticket."

[states.plan]
prompt = "Plan."

[states.delegate]
prompt = "Delegate and wait."
guards = [{ kind = "join", require = "all_passed", min = 1 }]

[[states.delegate.spawns]]
class = "reviewer"
name = "rev"
bind = ["ticket"]

[states.done]
prompt = "Done."

[[transitions]]
from = "plan"
to = "delegate"

[[transitions]]
from = "delegate"
to = "done"
"#;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn create(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t065-{label}-{}-{}",
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
        fs::write(root.join(".ratmac/ratmac.toml"), COMPOSED_RUNBOOK)
            .expect("write fixture machine class");
        fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").expect("write fixture source");
        Self { root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    /// Start a Run and return its minted id.
    fn start(&self) -> String {
        let output = self.rtm(&["start"]);
        let text = combined(&output);
        assert!(output.status.success(), "start succeeds: {text}");
        let id = text
            .split("started run ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .expect("start names the minted run id")
            .to_owned();
        assert!(!id.is_empty(), "minted id is non-empty");
        id
    }

    /// The project-directory-name phrase the pre-FDC-007 abandon required.
    fn project_phrase(&self) -> String {
        format!(
            "abandon {}",
            self.root
                .file_name()
                .expect("fixture has a directory name")
                .to_string_lossy()
        )
    }

    fn runs_dir(&self) -> PathBuf {
        self.root.join(".ratmac/runs")
    }

    fn roster(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(self.runs_dir()) else {
            return Vec::new();
        };
        let mut ids: Vec<String> = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        ids.sort();
        ids
    }

    fn record_path(&self, id: &str) -> PathBuf {
        self.runs_dir().join(id).join("run.toml")
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

/// Every file under `.ratmac/runs/`, keyed by relative path, with exact bytes.
fn runs_snapshot(runs: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    fn walk(base: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, files);
            } else {
                let relative = path
                    .strip_prefix(base)
                    .expect("snapshot path is below the runs root")
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(
                    relative,
                    fs::read(&path).expect("snapshot entry is readable"),
                );
            }
        }
    }
    walk(runs, runs, &mut files);
    files
}

/// PT-065-01: `rtm spawn` proceeds with no `--confirm` while the parent
/// occupies the spawning State; outside it, or for an undeclared spawn, the
/// same verb refuses by name and writes nothing.
#[test]
fn spawn_is_ordinary_checked_motion_without_phrase() {
    let fixture = Fixture::create("spawn");
    let parent = fixture.start();

    // Outside the spawning State: `plan` declares no spawns.
    let early = fixture.rtm(&["spawn", "rev", "--run", &parent]);
    let early_text = combined(&early);
    assert!(
        !early.status.success(),
        "spawn outside the spawning State refuses: {early_text}"
    );
    assert!(
        early_text.contains("plan"),
        "the refusal names the parent's State: {early_text}"
    );
    assert_eq!(
        fixture.roster(),
        vec![parent.clone()],
        "a refused spawn mints nothing"
    );

    // Enter the spawning State.
    let step = fixture.rtm(&["step", "--run", &parent]);
    assert!(
        combined(&step).contains("delegate") || step.status.success(),
        "the parent steps into the spawning State: {}",
        combined(&step)
    );

    // An undeclared spawn name refuses by name, writing nothing.
    let ghost = fixture.rtm(&["spawn", "ghost", "--run", &parent]);
    let ghost_text = combined(&ghost);
    assert!(
        !ghost.status.success(),
        "an undeclared spawn refuses: {ghost_text}"
    );
    assert!(
        ghost_text.contains("ghost"),
        "the refusal names the undeclared spawn: {ghost_text}"
    );
    assert_eq!(
        fixture.roster(),
        vec![parent.clone()],
        "a refused spawn mints nothing"
    );

    // The declared spawn proceeds with no confirmation phrase.
    let spawn = fixture.rtm(&["spawn", "rev", "--run", &parent]);
    let spawn_text = combined(&spawn);
    assert!(
        spawn.status.success(),
        "a declared spawn in the spawning State is ordinary motion: {spawn_text}"
    );
    let roster = fixture.roster();
    assert_eq!(
        roster.len(),
        2,
        "the child is a flat top-level Run: {roster:?}"
    );
    let child = roster
        .iter()
        .find(|id| **id != parent)
        .expect("the roster gained a child id")
        .clone();
    let state = fs::read_to_string(fixture.record_path(&child)).expect("child State File exists");
    assert!(
        state.contains("state = \"review\""),
        "the child begins at its class's initial State: {state}"
    );
    assert!(
        state.contains("status = \"passed\""),
        "a child born in a terminal State carries the Engine-written terminal fact (FDC-002): {state}"
    );
}

/// PT-065-02: respawn refuses without a phrase naming the superseded run id -
/// tree byte-identical - and with the exact phrase mints a successor id while
/// the superseded record keeps its address.
#[test]
fn respawn_requires_phrase_naming_the_run_id() {
    let fixture = Fixture::create("respawn");
    let parent = fixture.start();
    fixture.rtm(&["step", "--run", &parent]);
    let spawn = fixture.rtm(&["spawn", "rev", "--run", &parent]);
    assert!(
        spawn.status.success(),
        "spawn precondition: {}",
        combined(&spawn)
    );
    let child = fixture
        .roster()
        .into_iter()
        .find(|id| *id != parent)
        .expect("a child Run exists");

    let before = runs_snapshot(&fixture.runs_dir());
    let refusals: Vec<Vec<&str>> = vec![
        vec!["respawn", "--run", &child],
        vec!["respawn", "--run", &child, "--confirm", "respawn"],
        vec!["respawn", "--run", &child, "--confirm", "respawn run-999"],
    ];
    for args in refusals {
        let output = fixture.rtm(&args);
        let text = combined(&output);
        assert!(
            !output.status.success(),
            "respawn without the id-naming phrase refuses: {args:?} -> {text}"
        );
        assert!(
            text.contains(&format!("respawn {child}")),
            "the refusal names the required phrase: {text}"
        );
        assert_eq!(
            runs_snapshot(&fixture.runs_dir()),
            before,
            "a refused respawn leaves the tree byte-identical: {args:?}"
        );
    }

    let phrase = format!("respawn {child}");
    let respawn = fixture.rtm(&["respawn", "--run", &child, "--confirm", &phrase]);
    let text = combined(&respawn);
    assert!(
        respawn.status.success(),
        "the exact phrase proceeds: {text}"
    );
    let roster = fixture.roster();
    assert_eq!(
        roster.len(),
        3,
        "respawn mints a successor id and never overwrites: {roster:?}"
    );
    let successor = roster
        .iter()
        .find(|id| **id != parent && **id != child)
        .expect("a fresh successor id is minted");
    assert!(
        fixture.record_path(successor).is_file(),
        "the successor is live"
    );
    assert!(
        fixture.runs_dir().join(&child).is_dir(),
        "the superseded record keeps its address"
    );
    assert!(
        !fixture.record_path(&child).exists(),
        "the superseded child is retired by the abandon path, never left live beside its successor"
    );
}

/// PT-065-03: the abandon confirmation phrase names the addressed run id; the
/// old project-name phrase refuses as recorded behavioral evidence with no
/// file changes.
#[test]
fn abandon_phrase_names_the_run_id() {
    let fixture = Fixture::create("abandon");
    let run = fixture.start();

    let before = runs_snapshot(&fixture.runs_dir());
    let old = fixture.rtm(&[
        "abandon",
        "--run",
        &run,
        "--confirm",
        &fixture.project_phrase(),
    ]);
    let old_text = combined(&old);
    assert!(
        !old.status.success(),
        "the old project-name phrase now refuses: {old_text}"
    );
    assert!(
        old_text.contains(&format!("abandon {run}")),
        "the refusal names the run-id phrase: {old_text}"
    );
    assert_eq!(
        runs_snapshot(&fixture.runs_dir()),
        before,
        "a refused abandonment changes no file"
    );

    let phrase = format!("abandon {run}");
    let retired = fixture.rtm(&["abandon", "--run", &run, "--confirm", &phrase]);
    let text = combined(&retired);
    assert!(
        retired.status.success(),
        "the run-id phrase retires the Run: {text}"
    );
    assert!(
        !fixture.record_path(&run).exists(),
        "the admission state is retired"
    );
}
