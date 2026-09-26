//! t-117 / WRS-005: descriptive run roster.
//!
//! Every refusal that prints the roster keeps its first line and then lists
//! one row per Run: its address, its recorded top-level or child role, the
//! parent, class, and binding values a spawn ledger records, and its State
//! and status when its Run Record reads. Retired, missing, and unreadable
//! records and damaged ledgers are labeled rather than omitted or guessed.
//! Rows come only from Engine-owned records, in byte order, never from the
//! current runbook; a missing or bad address still refuses without choosing
//! a Run, and nothing is written. Stored values are quoted, so a binding
//! value cannot start a row of its own.

use ratmac_qa::support::{self, CaptureOptions, TempTree};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

/// A parent machine that spawns `rev` children of class `reviewer`, each
/// bound to a required `ticket`. `plan` carries a guard with an effect on
/// disk, so the fixture drive proves that guard runs when a live Run leaves
/// it, and a listing that evaluated guards would leave the same mark.
const RUNBOOK: &str = r#"
[classes.reviewer.bindings.ticket]
required = true

[classes.reviewer.states.review]
prompt = "Review the delegated ticket."

[classes.reviewer.states.finished]
prompt = "Review finished."

[[classes.reviewer.transitions]]
from = "review"
to = "finished"

[states.plan]
prompt = "Plan."
guards = [{ kind = "command_exit", program = "git", args = ["init", "-q", "guard-ran"], expected = 0 }]

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

/// The guard's mark: a directory the guard program creates.
const GUARD_MARK: &str = "guard-ran";

/// A changed runbook: new prompts, and the marking guard on every State, so
/// a listing that read it or evaluated its guards would show.
fn changed_runbook() -> String {
    let guard = r#"guards = [{ kind = "command_exit", program = "git", args = ["init", "-q", "guard-ran"], expected = 0 }]"#;
    RUNBOOK
        .replace("prompt = \"Plan.\"", "prompt = \"CHANGED plan.\"")
        .replace(
            "prompt = \"Delegate and wait.\"\nguards = [{ kind = \"join\", require = \"all_passed\", min = 1 }]",
            &format!("prompt = \"CHANGED delegate.\"\n{guard}"),
        )
        .replace(
            "prompt = \"Review the delegated ticket.\"",
            &format!("prompt = \"CHANGED review.\"\n{guard}"),
        )
}

struct Project {
    _tree: TempTree,
    root: PathBuf,
    linked: Option<PathBuf>,
}

impl Project {
    fn new(label: &str) -> Self {
        Self::build(label, false)
    }

    /// The same project as a Git repository with one linked worktree.
    fn with_linked(label: &str) -> Self {
        Self::build(label, true)
    }

    fn build(label: &str, linked: bool) -> Self {
        let tree = TempTree::new(&format!("t117-{label}")).expect("own a fixture tree");
        let root = tree.join("rosterproj");
        tree.write("rosterproj/src/lib.rs", "pub fn work() {}\n");
        tree.write(
            "rosterproj/.arca/issue/i-001-blocker/spec.md",
            "# Blocker\n",
        );
        tree.write("rosterproj/.ratmac/ratmac.toml", RUNBOOK);
        let linked = linked.then(|| {
            let linked = tree.join("rosterproj-linked");
            let identity = ["-c", "user.email=qa@example.invalid", "-c", "user.name=QA"];
            for args in [
                &["init", "-q"][..],
                &["add", "--all"],
                &["commit", "-q", "-m", "fixture base"],
            ] {
                let output = support::git(&root)
                    .args(identity)
                    .args(args)
                    .output()
                    .expect("run git");
                assert!(
                    output.status.success(),
                    "git {args:?}: {}",
                    support::text(&output)
                );
            }
            let output = support::git(&root)
                .args(["worktree", "add", "-q", "-b", "linked"])
                .arg(&linked)
                .output()
                .expect("run git worktree add");
            assert!(
                output.status.success(),
                "git worktree add: {}",
                support::text(&output)
            );
            linked
        });
        Project {
            _tree: tree,
            root,
            linked,
        }
    }

    fn rtm_in(&self, dir: &Path, args: &[&str]) -> Output {
        support::command(ratmac_qa::engine_bin!(), dir)
            .args(args)
            .output()
            .expect("invoke rtm")
    }

    /// Run a command that must succeed; return its combined output.
    fn ok(&self, args: &[&str]) -> String {
        let output = self.rtm_in(&self.root, args);
        let text = support::text(&output);
        assert!(
            output.status.success(),
            "fixture command {args:?} succeeds: {text}"
        );
        text
    }

    /// Start a top-level Run and return its minted id.
    fn start(&self) -> String {
        minted(&self.ok(&["start"]), "started run ")
    }

    fn step(&self, id: &str) {
        self.ok(&["step", "--run", id]);
    }

    /// Spawn `rev` under `parent` with `ticket` bound; return the child id.
    fn spawn(&self, parent: &str, ticket: &str) -> String {
        let bind = format!("ticket={ticket}");
        minted(
            &self.ok(&["spawn", "rev", "--run", parent, "--bind", &bind]),
            "spawned run ",
        )
    }

    fn abandon(&self, id: &str) {
        self.ok(&[
            "abandon",
            "--run",
            id,
            "--confirm",
            &format!("abandon {id}"),
        ]);
    }

    /// Respawn `id` and return the successor id.
    fn respawn(&self, id: &str) -> String {
        minted(
            &self.ok(&[
                "respawn",
                "--run",
                id,
                "--confirm",
                &format!("respawn {id}"),
            ]),
            "successor run ",
        )
    }

    /// Start a parent, step it into `delegate`, and clear the mark its
    /// `plan` guard left, after checking that the guard really ran.
    fn parent(&self) -> String {
        let parent = self.start();
        self.step(&parent);
        let mark = self.root.join(GUARD_MARK);
        assert!(
            mark.join(".git").is_dir(),
            "the plan guard ran for the live Run: {}",
            mark.display()
        );
        fs::remove_dir_all(&mark).expect("clear the guard mark");
        parent
    }

    fn write(&self, relative: &str, body: &str) {
        fs::write(self.root.join(relative), body).expect("write a fixture file");
    }

    /// The Engine root exactly as the Engine spells it, read from a healthy
    /// addressed status of `id`.
    fn engine_root(&self, id: &str) -> String {
        let text = self.ok(&["status", "--run", id]);
        text.lines()
            .find_map(|line| line.strip_prefix("Engine root: "))
            .unwrap_or_else(|| panic!("status names the Engine root: {text}"))
            .trim_end()
            .to_owned()
    }

    /// The recorded `state` and `status` of a readable Run Record.
    fn recorded(&self, id: &str) -> String {
        let path = self.root.join(format!(".ratmac/runs/{id}/run.toml"));
        let source = fs::read_to_string(&path).expect("the Run Record reads");
        let value: toml::Value = source.parse().expect("the Run Record parses");
        let field = |name: &str| {
            value
                .get(name)
                .and_then(toml::Value::as_str)
                .unwrap_or_else(|| panic!("{id} records {name}: {source}"))
                .to_owned()
        };
        format!("state {}; status {}", field("state"), field("status"))
    }

    /// Every byte of the fixture (and of the linked checkout), links refused.
    fn bytes(&self) -> BTreeMap<String, Vec<u8>> {
        let capture = |root: &PathBuf| {
            support::capture(root, CaptureOptions::default())
                .and_then(|snapshot| snapshot.refuse_links())
                .expect("capture the fixture")
                .tagged()
        };
        let mut bytes = capture(&self.root);
        if let Some(linked) = &self.linked {
            for (path, body) in capture(linked) {
                bytes.insert(format!("linked:{path}"), body);
            }
        }
        bytes
    }

    /// Run a refusal from `dir`; it must fail and change no byte.
    fn refusal_in(&self, dir: &Path, args: &[&str]) -> Refusal {
        let before = self.bytes();
        let output = self.rtm_in(dir, args);
        let stderr = String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8");
        assert!(
            !output.status.success(),
            "{args:?} refuses: {}",
            support::text(&output)
        );
        assert!(
            self.bytes() == before,
            "{args:?}: the refusal changes nothing: {stderr}"
        );
        assert!(
            !self.root.join(GUARD_MARK).exists(),
            "{args:?}: listing the roster evaluates no guard"
        );
        Refusal::parse(args, &stderr)
    }

    fn refusal(&self, args: &[&str]) -> Refusal {
        self.refusal_in(&self.root, args)
    }
}

/// A refusal split into its first line, its roster rows, and what follows.
#[derive(Debug, PartialEq)]
struct Refusal {
    first: String,
    rows: Vec<String>,
    rest: Vec<String>,
}

impl Refusal {
    fn parse(args: &[&str], stderr: &str) -> Self {
        let mut lines = stderr.lines().map(str::to_owned);
        let first = lines
            .next()
            .unwrap_or_else(|| panic!("{args:?} prints a refusal"));
        let mut rows = Vec::new();
        let mut rest = Vec::new();
        for line in lines {
            if rest.is_empty() && line.starts_with("  ") {
                rows.push(line);
            } else {
                rest.push(line);
            }
        }
        Refusal { first, rows, rest }
    }
}

fn minted(text: &str, marker: &str) -> String {
    text.split(marker)
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .map(|id| id.trim_end_matches(&[';', ':', ',', '.'][..]).to_owned())
        .unwrap_or_else(|| panic!("the output names the minted id after {marker:?}: {text}"))
}

fn row(id: &str, fields: &[&str]) -> String {
    format!("  {id}: {}", fields.join("; "))
}

fn child(parent: &str, ticket: &str) -> String {
    format!("child of {parent}; class reviewer; bind ticket={ticket:?}")
}

/// Assert `row` is `prefix`, one quoted reason, then `suffix`.
fn assert_reason_row(rows: &[String], prefix: &str, suffix: &str) {
    let found = rows.iter().find(|row| row.starts_with(prefix));
    let row = found.unwrap_or_else(|| panic!("a row starts {prefix:?}: {rows:#?}"));
    let reason = row[prefix.len()..]
        .strip_suffix(suffix)
        .unwrap_or_else(|| panic!("{row:?} ends {suffix:?}"));
    assert!(
        reason.len() > 2 && reason.starts_with('"') && reason.ends_with('"'),
        "the reason in {row:?} is one quoted value"
    );
}

/// WRSV-005-01: one top-level Run and two children with distinct bindings;
/// each row names the address, role, parent and class, binding, State, and
/// status, in byte order, repeatably.
#[test]
fn wrsv_005_01() {
    let project = Project::new("describe");
    let parent = project.parent();
    let first = project.spawn(&parent, "t-7");
    let second = project.spawn(&parent, "t-9");
    project.step(&second);
    assert_eq!(
        [&parent[..], &first, &second],
        ["run-001", "run-002", "run-003"]
    );

    // Positive control: an addressed status of each Run still succeeds.
    for id in [&parent, &first, &second] {
        project.ok(&["status", "--run", id]);
    }

    let expected = vec![
        row(&parent, &["top-level", &project.recorded(&parent)]),
        row(&first, &[&child(&parent, "t-7"), &project.recorded(&first)]),
        row(
            &second,
            &[&child(&parent, "t-9"), &project.recorded(&second)],
        ),
    ];
    assert_ne!(
        project.recorded(&first),
        project.recorded(&second),
        "the two children rest in different States, so rows cannot be swapped"
    );

    let listed = project.refusal(&["status"]);
    assert_eq!(
        listed.first,
        "rtm: status: run addressing is always required — pass --run <id>; runs: run-001, run-002, run-003",
        "the first line is unchanged"
    );
    assert_eq!(listed.rows, expected, "one descriptive row per Run");
    assert!(
        listed.rest.len() == 1 && listed.rest[0].starts_with("next: "),
        "one next line follows the rows: {listed:?}"
    );
    assert_eq!(
        project.refusal(&["status"]),
        listed,
        "the listing is stable across reads"
    );

    // The same rows follow every addressing refusal.
    for (args, first) in [
        (
            &["step", "--run", "run-999"][..],
            "rtm: step: run id \"run-999\" is not an exact roster member; runs: run-001, run-002, run-003",
        ),
        (
            &["status", "--run"],
            "rtm: status: --run needs a run id; runs: run-001, run-002, run-003",
        ),
    ] {
        let refused = project.refusal(args);
        assert_eq!(refused.first, first, "{args:?} keeps its first line");
        assert_eq!(refused.rows, expected, "{args:?} lists the same rows");
    }
}

/// WRSV-005-02: retired, malformed, missing-record, non-canonical, and
/// ledger-only entries are labeled; a damaged ledger makes roles unknown
/// instead of inventing top-level ownership.
#[test]
fn wrsv_005_02() {
    let project = Project::new("damage");
    let parent = project.parent();
    let corrupt = project.spawn(&parent, "t-7");
    let retired = project.spawn(&parent, "t-9");
    let superseded = project.spawn(&parent, "t-11");
    let missing = project.spawn(&parent, "t-13");
    let gone = project.spawn(&parent, "t-15");
    let engine_root = project.engine_root(&parent);
    project.abandon(&retired);
    let successor = project.respawn(&superseded);
    let marked = project.spawn(&parent, "t-17");
    assert_eq!(
        [
            &corrupt[..],
            &retired,
            &superseded,
            &missing,
            &gone,
            &successor,
            &marked
        ],
        ["run-002", "run-003", "run-004", "run-005", "run-006", "run-007", "run-008"]
    );

    // A retirement that stopped after flipping the ledger mark: the mark
    // is set while the Run Record remains.
    let ledger_file = project
        .root
        .join(format!(".ratmac/runs/{parent}/spawn-ledger"));
    let mut ledger: toml::Value = fs::read_to_string(&ledger_file)
        .expect("the parent's ledger reads")
        .parse()
        .expect("the parent's ledger parses");
    let entries = ledger
        .get_mut("children")
        .and_then(toml::Value::as_array_mut)
        .expect("the ledger records children");
    let mut found = entries
        .iter_mut()
        .filter_map(toml::Value::as_table_mut)
        .filter(|entry| entry.get("id").and_then(toml::Value::as_str) == Some(marked.as_str()));
    let entry = found.next().expect("the ledger records the marked child");
    assert_eq!(
        entry.get("abandoned").and_then(toml::Value::as_bool),
        Some(false),
        "the marked child starts unmarked"
    );
    entry.insert("abandoned".to_owned(), toml::Value::Boolean(true));
    assert!(found.next().is_none(), "the marked child is recorded once");
    project.write(
        &format!(".ratmac/runs/{parent}/spawn-ledger"),
        &toml::to_string(&ledger).expect("render the ledger"),
    );

    let corrupt_record = format!(".ratmac/runs/{corrupt}/run.toml");
    project.write(&corrupt_record, "state = [\n");
    fs::remove_file(
        project
            .root
            .join(format!(".ratmac/runs/{missing}/run.toml")),
    )
    .expect("remove a Run Record without retirement");
    fs::remove_dir_all(project.root.join(format!(".ratmac/runs/{gone}")))
        .expect("remove a child's whole directory");
    fs::create_dir_all(project.root.join(".ratmac/runs/scratch")).expect("add a stray directory");

    // Positive control: the healthy parent and successor still read.
    project.ok(&["status", "--run", &parent]);
    project.ok(&["status", "--run", &successor]);

    let listed = project.refusal(&["status"]);
    assert_eq!(
        listed.first,
        "rtm: status: run addressing is always required — pass --run <id>; runs: run-001, run-002, run-003, run-004, run-005, run-007, run-008, scratch",
        "the first line is unchanged"
    );
    let runs = format!("{engine_root}/runs");
    let exact = [
        row(&parent, &["top-level", &project.recorded(&parent)]),
        row(&retired, &[&child(&parent, "t-9"), "retired"]),
        row(&superseded, &[&child(&parent, "t-11"), "retired"]),
        row(
            &missing,
            &[
                &child(&parent, "t-13"),
                &format!("Run Record missing ({runs}/{missing}/run.toml is absent and no retirement is recorded)"),
            ],
        ),
        row(
            &gone,
            &[
                &child(&parent, "t-15"),
                &format!("not on the roster ({runs}/{gone} is absent)"),
            ],
        ),
        row(
            &successor,
            &[
                &child(&parent, "t-11"),
                &format!("supersedes {superseded}"),
                &project.recorded(&successor),
            ],
        ),
        row(
            &marked,
            &[
                &child(&parent, "t-17"),
                &project.recorded(&marked),
                "marked abandoned in its parent's spawn ledger",
            ],
        ),
        row("scratch", &["not a canonical run address", "not addressable"]),
    ];
    for expected in &exact {
        assert!(
            listed.rows.contains(expected),
            "the roster lists {expected:?}: {:#?}",
            listed.rows
        );
    }
    assert_eq!(
        listed.rows.len(),
        9,
        "one row per entry: {:#?}",
        listed.rows
    );
    assert_reason_row(
        &listed.rows,
        &format!(
            "  {corrupt}: {}; Run Record unreadable ({runs}/{corrupt}/run.toml: ",
            child(&parent, "t-7")
        ),
        ")",
    );
    let mut sorted = listed.rows.clone();
    sorted.sort();
    assert_eq!(listed.rows, sorted, "rows are in byte order");

    // Doctor tells a retired Run from a record that is simply gone.
    let doctor = support::text(&project.rtm_in(&project.root, &["doctor"]));
    for line in [
        format!("Run Record: .ratmac/runs/{retired}/run.toml (absent — run {retired} is retired)"),
        format!("Run Record: .ratmac/runs/{missing}/run.toml (absent — no retirement is recorded for run {missing})"),
    ] {
        assert!(doctor.contains(&line), "doctor says {line:?}: {doctor}");
    }

    // An unreadable transition log: a retirement the ledger marks still
    // reads, and an absent record only the log could explain is labeled
    // unknown instead of retired or missing.
    let log_path = project.root.join(".ratmac/log.md");
    let log_bytes = fs::read(&log_path).expect("the transition log reads");
    fs::remove_file(&log_path).expect("remove the transition log");
    fs::create_dir(&log_path).expect("put a directory where the log was");
    let blind = project.refusal(&["status"]);
    assert_eq!(blind.first, listed.first, "the first line is unchanged");
    let unknown_log = format!(
        "  {missing}: {}; Run Record absent (retirement unknown: transition log {engine_root}/log.md is unreadable: ",
        child(&parent, "t-13")
    );
    assert_reason_row(&blind.rows, &unknown_log, ")");
    let missing_row = format!("  {missing}: ");
    let others = |rows: &[String]| -> Vec<String> {
        rows.iter()
            .filter(|row| !row.starts_with(&missing_row))
            .cloned()
            .collect()
    };
    assert_eq!(
        others(&blind.rows),
        others(&listed.rows),
        "every other row, retired ones included, reads as before"
    );
    assert_eq!(blind.rows.len(), listed.rows.len(), "one row per entry");
    let doctor = support::text(&project.rtm_in(&project.root, &["doctor"]));
    let doctor_unknown = format!(
        "Run Record: .ratmac/runs/{missing}/run.toml (absent — retirement unknown: transition log {engine_root}/log.md is unreadable: \""
    );
    for line in [
        doctor_unknown,
        format!("Run Record: .ratmac/runs/{retired}/run.toml (absent — run {retired} is retired)"),
    ] {
        assert!(doctor.contains(&line), "doctor says {line:?}: {doctor}");
    }
    fs::remove_dir(&log_path).expect("remove the directory");
    fs::write(&log_path, &log_bytes).expect("restore the transition log");

    // A damaged ledger: no Run is claimed top-level, the ledger-only child
    // cannot be known, and retirement still reads from the transition log.
    let ledger = format!("{runs}/{parent}/spawn-ledger");
    project.write(
        &format!(".ratmac/runs/{parent}/spawn-ledger"),
        "[[children]\n",
    );
    let damaged = project.refusal(&["status"]);
    let unknown = format!("role unknown (unreadable spawn ledger {ledger}: ");
    assert_eq!(
        damaged.rows.len(),
        8,
        "one row per directory: {:#?}",
        damaged.rows
    );
    for (id, suffix) in [
        (&parent[..], format!("); {}", project.recorded(&parent))),
        (&retired, "); retired".to_owned()),
        (&superseded, "); retired".to_owned()),
        (&successor, format!("); {}", project.recorded(&successor))),
        (&marked, format!("); {}", project.recorded(&marked))),
    ] {
        assert_reason_row(&damaged.rows, &format!("  {id}: {unknown}"), &suffix);
    }
    assert!(
        damaged.rows.iter().all(|row| !row.contains("top-level")),
        "no Run is claimed top-level while a ledger is unreadable: {:#?}",
        damaged.rows
    );
    assert!(
        damaged.rows.contains(&row(
            "scratch",
            &["not a canonical run address", "not addressable"]
        )),
        "the stray directory keeps its label: {:#?}",
        damaged.rows
    );
}

/// WRSV-005-03: the rows are the same under runbook drift and from a linked
/// checkout, and reading them evaluates no guard and writes nothing.
#[test]
fn wrsv_005_03() {
    let project = Project::with_linked("drift");
    let linked = project.linked.clone().expect("a linked checkout");
    let parent = project.parent();
    let first = project.spawn(&parent, "t-7");
    let second = project.spawn(&parent, "t-9");
    project.step(&first);

    let expected = vec![
        row(&parent, &["top-level", &project.recorded(&parent)]),
        row(&first, &[&child(&parent, "t-7"), &project.recorded(&first)]),
        row(
            &second,
            &[&child(&parent, "t-9"), &project.recorded(&second)],
        ),
    ];
    let healthy = project.refusal(&["status"]);
    assert_eq!(healthy.rows, expected, "the healthy roster");
    let from_linked = project.refusal_in(&linked, &["status"]);
    assert_eq!(
        from_linked.rows, expected,
        "a linked checkout lists the shared Runs identically"
    );

    let changed = changed_runbook();
    assert_ne!(changed, RUNBOOK, "the drifted runbook differs");
    for (label, body) in [
        ("changed", Some(changed.as_str())),
        ("corrupt", Some("[states.plan\n")),
        ("removed", None),
    ] {
        let path = project.root.join(".ratmac/ratmac.toml");
        match body {
            Some(body) => fs::write(&path, body).expect("drift the runbook"),
            None => fs::remove_file(&path).expect("remove the runbook"),
        }
        for dir in [&project.root, &linked] {
            let refused = project.refusal_in(dir, &["status"]);
            assert_eq!(
                refused.rows,
                expected,
                "{label} runbook from {}: the rows come only from Engine records",
                dir.display()
            );
            assert!(
                !refused.rows.iter().any(|row| row.contains("CHANGED")),
                "{label}: no runbook text reaches the roster"
            );
        }
        fs::write(&path, RUNBOOK).expect("heal the runbook");
    }
}

/// WRSV-005-04: missing and invalid addresses still refuse and never pick
/// the sole Run; a binding value holding control characters stays inside
/// its own quoted row.
#[test]
fn wrsv_005_04() {
    let project = Project::new("address");
    let sole = project.start();
    assert_eq!(sole, "run-001");
    project.ok(&["status", "--run", &sole]);
    let only = vec![row(&sole, &["top-level", &project.recorded(&sole)])];

    // Every line but the rows is exactly what the Engine printed before
    // the rows existed: the first line with its summary, and any `next:`.
    let usage = "a Run is retired only by its own phrase: rtm abandon --run <id> --confirm \"abandon <run id>\"";
    let next = "next: rtm status --run run-001";
    let required = |command: &str| {
        format!(
            "rtm: {command}: run addressing is always required — pass --run <id>; runs: run-001"
        )
    };
    let cases: Vec<(&[&str], String, Vec<&str>)> = vec![
        (&["status"], required("status"), vec![next]),
        (&["step"], required("step"), vec![next]),
        (
            &["hold"],
            "rtm: hold refused; hold requires --run <id>; runs: run-001".to_owned(),
            vec![],
        ),
        (
            &["abandon"],
            format!("rtm: abandon refused; abandon requires --run <id>; runs: run-001; {usage}"),
            vec![],
        ),
        (
            &["respawn"],
            "rtm: respawn refused; respawn requires --run <id>; runs: run-001".to_owned(),
            vec![],
        ),
        (
            &["spawn", "rev"],
            "rtm: spawn requires --run <parent id>; runs: run-001".to_owned(),
            vec![],
        ),
        (&["status", "--run", ""], required("status"), vec![next]),
        (
            &["step", "--run", "run-1"],
            "rtm: step: run id \"run-1\" is not one canonical minted path segment; runs: run-001"
                .to_owned(),
            vec![next],
        ),
        (
            &["step", "--run", "RUN-001"],
            "rtm: step: run id \"RUN-001\" is not one canonical minted path segment; runs: run-001"
                .to_owned(),
            vec![next],
        ),
        (
            &["step", "--run", "run-009"],
            "rtm: step: run id \"run-009\" is not an exact roster member; runs: run-001".to_owned(),
            vec![next],
        ),
        (
            &["status", "--run", "run-001", "--run", "run-001"],
            "rtm: status: --run given twice; address exactly one run; runs: run-001".to_owned(),
            vec![next],
        ),
        (
            &["hold", "--run"],
            "rtm: hold: --run needs a run id; runs: run-001".to_owned(),
            vec![],
        ),
        (
            &["abandon", "--run"],
            format!("rtm: abandon: --run needs a run id; runs: run-001; {usage}"),
            vec![],
        ),
        (
            &["abandon", "--run", "run-009", "--confirm", "abandon run-009"],
            "rtm: abandon refused; abandonment refused: run id \"run-009\" is not an exact roster member; runs: run-001"
                .to_owned(),
            vec![],
        ),
        (
            &["respawn", "--run", "run-009", "--confirm", "respawn run-009"],
            "rtm: respawn refused; respawn names no run: \"run-009\" is not on the roster; runs: run-001"
                .to_owned(),
            vec![],
        ),
    ];
    let observed: Vec<(&[&str], Refusal)> = cases
        .iter()
        .map(|(args, _, _)| (*args, project.refusal(args)))
        .collect();
    let kept: Vec<(&[&str], &str, Vec<&str>)> = observed
        .iter()
        .map(|(args, refused)| {
            (
                *args,
                refused.first.as_str(),
                refused.rest.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    let frozen: Vec<(&[&str], &str, Vec<&str>)> = cases
        .iter()
        .map(|(args, first, rest)| (*args, first.as_str(), rest.clone()))
        .collect();
    assert_eq!(
        kept, frozen,
        "each refusal keeps its first line and every line after the rows"
    );
    for (args, refused) in &observed {
        assert_eq!(
            refused.rows, only,
            "{args:?} describes the sole Run and picks none"
        );
    }

    // A binding value built to look like another row.
    project.step(&sole);
    fs::remove_dir_all(project.root.join(GUARD_MARK)).expect("clear the guard mark");
    let forged = "t-1\n  run-009: top-level; state done; status passed\u{1b}[2K\r";
    let spawned = project.spawn(&sole, forged);
    let refused = project.refusal(&["status"]);
    assert_eq!(
        refused.rows,
        vec![
            row(&sole, &["top-level", &project.recorded(&sole)]),
            row(
                &spawned,
                &[&child(&sole, forged), &project.recorded(&spawned)]
            ),
        ],
        "the forged value stays quoted inside its own row"
    );
    assert!(
        refused
            .rows
            .iter()
            .all(|row| !row.contains(&['\u{1b}', '\r'][..])),
        "no raw control character reaches the roster: {:?}",
        refused.rows
    );

    // Every other stored token turned hostile: the child's class, State,
    // and a second binding name; a ledger-only child id; and a stray
    // directory whose name is not plain and whose own ledger also claims
    // that child. Each stays inside its own row, quoted where not plain.
    let runs = format!("{}/runs", project.engine_root(&sole));
    let class = "rev\u{1b}iewer";
    let state = "review\n  run-009: top-level; state done; status passed";
    let ghost = "run-0\n  x";
    let stray = "z z";
    let ledger_path = project
        .root
        .join(format!(".ratmac/runs/{sole}/spawn-ledger"));
    let mut ledger: toml::Value = fs::read_to_string(&ledger_path)
        .expect("the parent's spawn ledger reads")
        .parse()
        .expect("the spawn ledger parses");
    let children = ledger
        .get_mut("children")
        .and_then(toml::Value::as_array_mut)
        .expect("the ledger records children");
    let entry = children[0].as_table_mut().expect("an entry is a table");
    assert_eq!(
        entry.get("id").and_then(toml::Value::as_str),
        Some(&spawned[..])
    );
    let mut ghost_entry = entry.clone();
    entry.insert("class".to_owned(), class.into());
    let mut bind = toml::map::Map::new();
    bind.insert("tick et".to_owned(), forged.into());
    bind.insert("alpha".to_owned(), "w".into());
    entry.insert("bind".to_owned(), toml::Value::Table(bind));
    ghost_entry.insert("id".to_owned(), ghost.into());
    children.push(toml::Value::Table(ghost_entry.clone()));
    fs::write(
        &ledger_path,
        toml::to_string(&ledger).expect("render the ledger"),
    )
    .expect("rewrite the parent's ledger");
    let stray_dir = project.root.join(format!(".ratmac/runs/{stray}"));
    fs::create_dir_all(&stray_dir).expect("add a stray directory");
    let mut stray_ledger = toml::map::Map::new();
    stray_ledger.insert(
        "children".to_owned(),
        toml::Value::Array(vec![toml::Value::Table(ghost_entry)]),
    );
    fs::write(
        stray_dir.join("spawn-ledger"),
        toml::to_string(&stray_ledger).expect("render the stray ledger"),
    )
    .expect("write the stray ledger");
    let record_path = project
        .root
        .join(format!(".ratmac/runs/{spawned}/run.toml"));
    let mut record: toml::Value = fs::read_to_string(&record_path)
        .expect("the child's Run Record reads")
        .parse()
        .expect("the Run Record parses");
    let status = record
        .get("status")
        .and_then(toml::Value::as_str)
        .expect("the Run Record has a status")
        .to_owned();
    record
        .as_table_mut()
        .expect("the Run Record is a table")
        .insert("state".to_owned(), state.into());
    fs::write(
        &record_path,
        toml::to_string(&record).expect("render the record"),
    )
    .expect("rewrite the child's Run Record");

    let hostile = project.refusal(&["status"]);
    assert_eq!(
        hostile.first,
        format!(
            "{}, \"z z\"",
            required("status").replace("run-001", "run-001, run-002")
        ),
        "a name that is not plain is quoted in the summary"
    );
    assert_eq!(
        hostile.rows,
        vec![
            row(
                &format!("{ghost:?}"),
                &[
                    "role unknown (recorded as a child by run-001, \"z z\")",
                    &format!("not on the roster ({runs}/run-0\\n  x is absent)"),
                ],
            ),
            row(&sole, &["top-level", &project.recorded(&sole)]),
            row(
                &spawned,
                &[
                    &format!("child of {sole}; class {class:?}"),
                    &format!("bind alpha=\"w\", \"tick et\"={forged:?}"),
                    &format!("state {state:?}"),
                    &format!("status {status}"),
                ],
            ),
            row(
                "\"z z\"",
                &["not a canonical run address", "not addressable"]
            ),
        ],
        "every stored token stays inside its own row, parents in name order"
    );
    assert!(
        hostile
            .rows
            .iter()
            .all(|row| !row.contains(&['\u{1b}', '\r', '\n'][..])),
        "no raw control character reaches the roster: {:?}",
        hostile.rows
    );
}
