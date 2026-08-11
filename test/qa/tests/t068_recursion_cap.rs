//! t-068 / FDC-012: the one-level recursion cap.
//!
//! PT-068-01 `child_spawn_refuses_naming_the_cap`
//! PT-068-02 `top_level_parents_spawn_freely`

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A composed machine whose child class reuses the state name `delegate`.
/// The top-level `delegate` is the initial spawning State; a child Run born
/// at its class-initial `delegate` therefore resolves to a State name that
/// the top level declares spawns for - the exact hole the cap must plug.
const NESTED_NAME_RUNBOOK: &str = r#"
[classes.reviewer.bindings.ticket]
required = true

[classes.reviewer.states.delegate]
prompt = "Review the delegated ticket."

[classes.reviewer.states.wrap]
prompt = "Wrap up."

[[classes.reviewer.transitions]]
from = "delegate"
to = "wrap"

[states.delegate]
prompt = "Delegate and wait."

[[states.delegate.spawns]]
class = "reviewer"
name = "rev"
bind = ["ticket"]

[states.done]
prompt = "Done."

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
            "ratmac-t068-{label}-{}-{}",
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
        fs::write(root.join(".ratmac/ratmac.toml"), NESTED_NAME_RUNBOOK)
            .expect("write fixture machine class");
        fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").expect("write fixture source");
        Self { root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    /// Start a Run; the initial State `delegate` is already the spawning one.
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

    /// Spawn `rev` binding `ticket`; return the child id.
    fn spawn_rev(&self, parent: &str, ticket: &str) -> String {
        let bind = format!("ticket={ticket}");
        let output = self.rtm(&["spawn", "rev", "--run", parent, "--bind", &bind]);
        let text = combined(&output);
        assert!(output.status.success(), "spawn succeeds: {text}");
        text.split("spawned run ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .expect("spawn names the child run id")
            .to_owned()
    }

    fn runs_dir(&self) -> PathBuf {
        self.root.join(".ratmac/runs")
    }

    fn ledger_text(&self, parent: &str) -> String {
        fs::read_to_string(self.runs_dir().join(parent).join("spawn-ledger"))
            .expect("the parent spawn-ledger is readable")
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

/// Every file under `base`, keyed by relative path, with exact bytes.
fn tree_snapshot(base: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    walk(base, base, &mut files);
    files
}

fn walk(base: &Path, directory: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let relative = path
            .strip_prefix(base)
            .expect("walked path is below base")
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            walk(base, &path, files);
        } else if let Ok(bytes) = fs::read(&path) {
            files.insert(relative, bytes);
        }
    }
}

/// PT-068-01: a spawn addressed to a ledger-recorded child refuses naming the
/// one-level cap, before any write - child dir, every ledger, and the roster
/// stay byte-identical.
#[test]
fn child_spawn_refuses_naming_the_cap() {
    let fixture = Fixture::create("cap");
    let parent = fixture.start();
    let child = fixture.spawn_rev(&parent, "t-001");

    let before = tree_snapshot(&fixture.runs_dir());
    let refused = fixture.rtm(&["spawn", "rev", "--run", &child, "--bind", "ticket=t-002"]);
    let text = combined(&refused);

    assert!(
        !refused.status.success(),
        "a child-addressed spawn must refuse, got: {text}"
    );
    assert!(
        text.contains("capped at one level"),
        "the refusal names the cap, got: {text}"
    );
    assert!(
        text.contains(&child),
        "the refusal names the addressed child run, got: {text}"
    );

    let after = tree_snapshot(&fixture.runs_dir());
    assert_eq!(
        before, after,
        "a refused child-spawn leaves the runs tree byte-identical"
    );
}

/// PT-068-02: the cap is depth, not count - two independent top-level parents
/// keep spawning freely, each ledger holding exactly its own child.
#[test]
fn top_level_parents_spawn_freely() {
    let fixture = Fixture::create("breadth");
    let first_parent = fixture.start();
    let second_parent = fixture.start();

    let first_child = fixture.spawn_rev(&first_parent, "t-101");
    let second_child = fixture.spawn_rev(&second_parent, "t-102");

    let first_ledger = fixture.ledger_text(&first_parent);
    assert!(
        first_ledger.contains(&first_child),
        "the first parent's ledger records its child"
    );
    assert!(
        !first_ledger.contains(&second_child),
        "the first parent's ledger holds no foreign child"
    );

    let second_ledger = fixture.ledger_text(&second_parent);
    assert!(
        second_ledger.contains(&second_child),
        "the second parent's ledger records its child"
    );
    assert!(
        !second_ledger.contains(&first_child),
        "the second parent's ledger holds no foreign child"
    );
}
