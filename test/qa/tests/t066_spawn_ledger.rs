//! t-066 / FDC-011: the spawn ledger exists and is load-bearing.
//!
//! PT-066-01 `spawn_appends_one_ledger_entry_with_the_recorded_fields`
//! PT-066-02 `abandon_flips_only_the_mark_and_respawn_appends_the_successor`
//! PT-066-03 `join_reads_the_ledger_and_refuses_naming_the_missing_child`
//!
//! At the per-run path FDC-004 reserves under the parent Run's directory,
//! `rtm spawn` appends one entry carrying the child run id, class, binding
//! values, and the revision at spawn. The ledger is append/annotate-only:
//! confirmed abandonment flips only that entry's abandoned mark; confirmed
//! respawn appends the successor entry naming the superseded id and mints the
//! successor from the recorded class. The join guard reads the ledger as its
//! expected set: an entry whose child Run is missing on disk refuses loudly,
//! naming that child - the set never silently shrinks.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// The t-064 composed machine: one declared child class, one spawning Phase
/// (`delegate`), one join-guarded out-edge. `plan` is the initial Phase.
const COMPOSED_RUNBOOK: &str = r#"
[classes.reviewer.bindings.ticket]
required = true

[classes.reviewer.phases.review]
prompt = "Review the delegated ticket."

[phases.plan]
prompt = "Plan."

[phases.delegate]
prompt = "Delegate and wait."
guards = [{ kind = "join", require = "all_passed", min = 1 }]

[[phases.delegate.spawns]]
class = "reviewer"
name = "rev"
bind = ["ticket"]

[phases.done]
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
            "ratmac-t066-{label}-{}-{}",
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
        fs::write(root.join(".arca/ratmac.toml"), COMPOSED_RUNBOOK).expect("write fixture runbook");
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

    /// Start a Run and return its minted id.
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

    /// Start and step the parent into the spawning Phase `delegate`.
    fn start_at_delegate(&self) -> String {
        let parent = self.start();
        let step = self.rtm(&["step", "--run", &parent]);
        assert!(
            step.status.success(),
            "step into delegate succeeds: {}",
            combined(&step)
        );
        parent
    }

    /// Spawn `rev` binding `ticket` to the given value; return the child id.
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

    fn run_dir(&self, id: &str) -> PathBuf {
        self.root.join(".arca/runs").join(id)
    }

    fn ledger_path(&self, parent: &str) -> PathBuf {
        self.run_dir(parent).join("spawn-ledger")
    }

    fn ledger_text(&self, parent: &str) -> String {
        fs::read_to_string(self.ledger_path(parent)).expect("the parent spawn-ledger is readable")
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

/// Parse the ledger as TOML and return its `[[children]]` entries.
fn entries(text: &str) -> Vec<toml::value::Table> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    let value: toml::Value = text.parse().expect("the spawn ledger is valid TOML");
    let table = value.as_table().expect("ledger top level is a table");
    let Some(children) = table.get("children") else {
        return Vec::new();
    };
    children
        .as_array()
        .expect("children is an array of tables")
        .iter()
        .map(|entry| {
            entry
                .as_table()
                .expect("each ledger entry is a table")
                .clone()
        })
        .collect()
}

fn field<'a>(entry: &'a toml::value::Table, key: &str) -> &'a str {
    entry
        .get(key)
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("entry field {key} is a string: {entry:?}"))
}

fn abandoned(entry: &toml::value::Table) -> bool {
    entry
        .get("abandoned")
        .and_then(toml::Value::as_bool)
        .expect("entry field abandoned is a bool")
}

fn binding<'a>(entry: &'a toml::value::Table, name: &str) -> &'a str {
    entry
        .get("bind")
        .and_then(toml::Value::as_table)
        .unwrap_or_else(|| panic!("entry carries a bind table: {entry:?}"))
        .get(name)
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("bind value {name} is a string: {entry:?}"))
}

/// PT-066-01: `rtm spawn` appends exactly one entry at the reserved per-run
/// path, carrying the recorded fields; a second spawn appends after the
/// preserved prior bytes; an undeclared binding name refuses and writes
/// nothing.
#[test]
fn spawn_appends_one_ledger_entry_with_the_recorded_fields() {
    let fixture = Fixture::create("fields");
    let parent = fixture.start_at_delegate();
    assert_eq!(
        fixture.ledger_text(&parent),
        "",
        "the reserved ledger is empty before any spawn"
    );

    let child = fixture.spawn_rev(&parent, "t-042");
    let first = fixture.ledger_text(&parent);
    assert_eq!(
        first.matches("[[children]]").count(),
        1,
        "one spawn appends exactly one entry:\n{first}"
    );
    let recorded = entries(&first);
    assert_eq!(recorded.len(), 1, "one entry parsed");
    let entry = &recorded[0];
    assert_eq!(field(entry, "id"), child, "the entry names the child run id");
    assert_eq!(
        field(entry, "class"),
        "reviewer",
        "the entry names the declared class"
    );
    assert_eq!(
        binding(entry, "ticket"),
        "t-042",
        "the entry carries the binding value supplied at invocation"
    );
    assert!(
        !field(entry, "spawned_at").is_empty(),
        "the entry records the revision at spawn"
    );
    assert!(!abandoned(entry), "a fresh entry is not abandoned");

    let sibling = fixture.spawn_rev(&parent, "t-043");
    let second = fixture.ledger_text(&parent);
    assert!(
        second.starts_with(&first),
        "append preserves every prior byte as a prefix:\nfirst:\n{first}\nsecond:\n{second}"
    );
    let recorded = entries(&second);
    assert_eq!(recorded.len(), 2, "two entries after two spawns");
    assert_eq!(field(&recorded[1], "id"), sibling, "second entry names the sibling");

    let refused = fixture.rtm(&["spawn", "rev", "--run", &parent, "--bind", "price=9"]);
    let text = combined(&refused);
    assert!(
        !refused.status.success(),
        "an undeclared binding name refuses: {text}"
    );
    assert!(
        text.contains("price"),
        "the refusal names the undeclared binding: {text}"
    );
    assert_eq!(
        fixture.ledger_text(&parent),
        second,
        "a refused spawn writes no ledger byte"
    );
}

/// PT-066-02: confirmed abandonment flips only the addressed entry's
/// abandoned mark; confirmed respawn appends the successor entry naming the
/// superseded id and mints the successor from the recorded class. No prior
/// entry is ever rewritten.
#[test]
fn abandon_flips_only_the_mark_and_respawn_appends_the_successor() {
    let fixture = Fixture::create("annotate");
    let parent = fixture.start_at_delegate();
    let first_child = fixture.spawn_rev(&parent, "t-101");
    let second_child = fixture.spawn_rev(&parent, "t-102");
    let before = fixture.ledger_text(&parent);

    let phrase = format!("abandon {first_child}");
    let abandon = fixture.rtm(&["abandon", "--run", &first_child, "--confirm", &phrase]);
    assert!(
        abandon.status.success(),
        "confirmed abandon succeeds: {}",
        combined(&abandon)
    );
    let flipped = fixture.ledger_text(&parent);
    assert_eq!(
        flipped,
        before.replacen("abandoned = false", "abandoned = true", 1),
        "abandonment flips exactly the addressed entry's mark and no other byte"
    );

    let phrase = format!("respawn {second_child}");
    let respawn = fixture.rtm(&["respawn", "--run", &second_child, "--confirm", &phrase]);
    let text = combined(&respawn);
    assert!(respawn.status.success(), "confirmed respawn succeeds: {text}");
    let successor = text
        .split(" run ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .expect("respawn names the successor id")
        .trim_end_matches(&[';', ':', ',', '.'][..])
        .to_owned();

    let after = fixture.ledger_text(&parent);
    let recorded = entries(&after);
    assert_eq!(recorded.len(), 3, "respawn appends the successor entry:\n{after}");
    assert_eq!(field(&recorded[0], "id"), first_child);
    assert!(abandoned(&recorded[0]), "the first entry keeps its abandoned mark");
    assert_eq!(field(&recorded[1], "id"), second_child);
    assert!(
        abandoned(&recorded[1]),
        "the superseded entry's mark flips when respawn retires it"
    );
    let entry = &recorded[2];
    assert_eq!(field(entry, "id"), successor, "the successor entry names the successor");
    assert_eq!(
        field(entry, "supersedes"),
        second_child,
        "the successor entry records the superseded id"
    );
    assert_eq!(field(entry, "class"), "reviewer", "the successor keeps the recorded class");
    assert_eq!(
        binding(entry, "ticket"),
        "t-102",
        "the successor inherits the recorded binding values"
    );
    assert!(!abandoned(entry), "the successor entry is live");

    let state = fs::read_to_string(fixture.run_dir(&successor).join("state.toml"))
        .expect("the successor has its own State File");
    assert!(
        state.contains("phase = \"review\""),
        "the successor is minted from the recorded class, not the top-level machine: {state}"
    );
}

/// PT-066-03: the join guard reads the ledger as its expected set. A
/// recorded, passed child lets the join release the transition; deleting
/// that child's directory out-of-band makes the join refuse loudly, naming
/// the child, with the parent's State File byte-identical.
#[test]
fn join_reads_the_ledger_and_refuses_naming_the_missing_child() {
    let passing = Fixture::create("joinpass");
    let parent = passing.start_at_delegate();
    passing.spawn_rev(&parent, "t-201");
    let step = passing.rtm(&["step", "--run", &parent]);
    assert!(
        step.status.success(),
        "the join releases the transition off the ledger: {}",
        combined(&step)
    );
    let state = fs::read_to_string(passing.run_dir(&parent).join("state.toml"))
        .expect("parent state is readable");
    assert!(
        state.contains("phase = \"done\""),
        "the parent advanced through the join: {state}"
    );

    let refusing = Fixture::create("joinmiss");
    let parent = refusing.start_at_delegate();
    let hollow = refusing.rtm(&["step", "--run", &parent]);
    let text = combined(&hollow);
    assert!(
        !hollow.status.success(),
        "an empty ledger keeps the honest refusal: {text}"
    );
    assert!(
        text.contains("no spawn ledger records a child Run"),
        "the zero-children refusal is unchanged: {text}"
    );

    let child = refusing.spawn_rev(&parent, "t-202");
    let state_before = fs::read(refusing.run_dir(&parent).join("state.toml"))
        .expect("parent state is readable before the deletion");
    fs::remove_dir_all(refusing.run_dir(&child)).expect("delete the child run out-of-band");

    let step = refusing.rtm(&["step", "--run", &parent]);
    let text = combined(&step);
    assert!(
        !step.status.success(),
        "a ledger entry with no run on disk refuses: {text}"
    );
    assert!(
        text.contains(&child),
        "the refusal names the missing child: {text}"
    );
    assert!(
        text.contains("no run") || text.contains("missing"),
        "the refusal says the run is missing, never silently shrinking the set: {text}"
    );
    let state_after = fs::read(refusing.run_dir(&parent).join("state.toml"))
        .expect("parent state is readable after the refusal");
    assert_eq!(
        state_before, state_after,
        "a refused join leaves the parent State File byte-identical"
    );
}
