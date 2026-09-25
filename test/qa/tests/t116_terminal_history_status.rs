//! t-116 / WRS-006: terminal history status.
//!
//! Addressed status reads a strictly parsed, persisted passed Run as history:
//! its identity, State, status, and recorded evidence identities, even when
//! the current runbook changed, is gone, or no longer parses. The view says
//! the current instructions are unavailable, never shows a changed prompt or
//! guard as the old Run's instructions, never suggests retirement, and writes
//! nothing. A Run that has not passed keeps every existing pin check, a
//! passed Run still refuses step and hold, and corrupt records, bad
//! addresses, unknown Runs, and unreadable evidence refuse by name.

use ratmac_qa::support::{self, CaptureOptions, TempTree};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Output;

/// A parent that spawns one child; both reach a State with no way out. The
/// parent's `plan` carries the guard with an effect on disk (`GUARD`), so the
/// fixture drive itself proves that guard runs when a live Run leaves it.
const RUNBOOK: &str = r#"
[roots]
issue = ".arca/issue"

[classes.reviewer.states.review]
prompt = "Child reviews the work."

[classes.reviewer.states.finished]
prompt = "Child finished its review."

[[classes.reviewer.transitions]]
from = "review"
to = "finished"

[states.plan]
prompt = "Parent plans the work."
guards = [{ kind = "command_exit", program = "git", args = ["init", "-q", "guard-ran"], expected = 0 }]

[states.delegate]
prompt = "Parent delegates and waits."
guards = [{ kind = "join", require = "all_passed", min = 1 }]

[[states.delegate.spawns]]
class = "reviewer"
name = "rev"

[states.done]
prompt = "Parent finished the work."

[[transitions]]
from = "plan"
to = "delegate"

[[transitions]]
from = "plan"
to = "done"
blocked-route = true

[[transitions]]
from = "delegate"
to = "done"
"#;

/// Text only the changed runbook carries; a history view never shows it.
const CHANGED_MARK: &str = "CHANGED-INSTRUCTION";

/// The folder the changed runbook's guard creates when it runs.
const GUARD_MARK: &str = "guard-ran";

/// A guard with an effect on disk, as on the parent's `plan`; the changed
/// runbook adds it to both finished States, so a view that renders or
/// evaluates the current guards of a finished Run shows it or leaves the
/// folder behind.
const GUARD: &str = r#"guards = [{ kind = "command_exit", program = "git", args = ["init", "-q", "guard-ran"], expected = 0 }]"#;

/// Complete, distinct identities for the child's evidence: every field the
/// Engine records, so a view that drops any one of them is caught.
const SEEDED_EVIDENCE: &str = r#"[engine]
resolved = "C:\\seeded\\engine\\rtm.exe"
sha256 = "1111111111111111111111111111111111111111111111111111111111111111"
source-commit = "5eeded0c0ffee5eeded0c0ffee5eeded0c0ffee0"
channel = "seeded-channel"

[goal]
baseline = "2222222222222222222222222222222222222222222222222222222222222222"
frozen = "3333333333333333333333333333333333333333333333333333333333333333"

[runbook]
sha256 = "4444444444444444444444444444444444444444444444444444444444444444"

[[gate]]
program = "seeded-gate-one"
resolved = "C:/seeded/gates/one.exe"
sha256 = "5555555555555555555555555555555555555555555555555555555555555555"

[[gate]]
program = "seeded-gate-two"
resolved = "C:/seeded/gates/two.exe"
sha256 = "6666666666666666666666666666666666666666666666666666666666666666"
"#;

/// A blocker reference inside the declared `issue` root, so a hold is
/// refused only for the Run it addresses.
const BLOCKER: &str = ".arca/issue/i-001-blocker";

/// The same machine with new prompts, the effectful guard on both finished
/// States, and a way back out of them: the current graph no longer calls
/// them terminal.
fn changed_runbook() -> String {
    RUNBOOK
        .replace(
            "prompt = \"Parent finished the work.\"",
            &format!("prompt = \"CHANGED-INSTRUCTION: parent must redo the plan.\"\n{GUARD}"),
        )
        .replace(
            "prompt = \"Child finished its review.\"",
            &format!("prompt = \"CHANGED-INSTRUCTION: child must review again.\"\n{GUARD}"),
        )
        + "\n[[transitions]]\nfrom = \"done\"\nto = \"plan\"\n\n\
           [[classes.reviewer.transitions]]\nfrom = \"finished\"\nto = \"review\"\n"
}

/// The three ways the runbook moves on after a Run passed.
#[derive(Clone, Copy, Debug)]
enum Drift {
    Changed,
    Removed,
    Corrupt,
}

const DRIFTS: [Drift; 3] = [Drift::Changed, Drift::Removed, Drift::Corrupt];

/// One project with a passed parent (`run-001`) and its passed child
/// (`run-002`), owned by one temporary tree.
struct Project {
    _tree: TempTree,
    root: PathBuf,
    /// The child's bound workspace, a linked worktree, when it has one.
    linked: Option<PathBuf>,
}

impl Project {
    fn finished(label: &str) -> Self {
        Self::build(label, false)
    }

    /// The same finished pair, with the child spawned into a linked worktree
    /// of the project as its bound workspace.
    fn finished_linked(label: &str) -> Self {
        Self::build(label, true)
    }

    fn build(label: &str, bound: bool) -> Self {
        let tree = TempTree::new(&format!("t116-{label}")).expect("own a fixture tree");
        let root = tree.join("histproj");
        tree.write("histproj/src/lib.rs", "pub fn work() {}\n");
        tree.write("histproj/.arca/issue/i-001-blocker/spec.md", "# Blocker\n");
        tree.write("histproj/.ratmac/ratmac.toml", RUNBOOK);
        let linked = bound.then(|| {
            let linked = tree.join("histproj-linked");
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
        let workspace = linked
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned());
        let mut spawn = vec!["spawn", "rev", "--run", "run-001"];
        if let Some(workspace) = &workspace {
            spawn.extend(["--workspace", workspace.as_str()]);
        }
        let project = Project {
            _tree: tree,
            root,
            linked,
        };
        for args in [
            &["start"][..],
            &["step", "--run", "run-001"],
            spawn.as_slice(),
            &["step", "--run", "run-002"],
            &["step", "--run", "run-001"],
        ] {
            let (ok, text) = project.say(args);
            assert!(ok, "fixture step {args:?} succeeds: {text}");
        }
        // Live control: leaving `plan` ran its guard, so the mark exists;
        // it is cleared so a history read that evaluates a guard shows.
        let mark = project.root.join(GUARD_MARK);
        assert!(
            mark.join(".git").is_dir(),
            "the plan guard ran for the live Run: {}",
            mark.display()
        );
        fs::remove_dir_all(&mark).expect("clear the guard mark");
        for (id, state) in [("run-001", "done"), ("run-002", "finished")] {
            let record = project.read(&format!(".ratmac/runs/{id}/run.toml"));
            assert!(
                record.contains(&format!("state = \"{state}\""))
                    && record.contains("status = \"passed\""),
                "{id} is persisted as passed at {state}: {record}"
            );
        }
        project.write(".ratmac/runs/run-002/evidence.toml", SEEDED_EVIDENCE);
        project
    }

    fn rtm(&self, args: &[&str]) -> Output {
        support::command(ratmac_qa::engine_bin!(), &self.root)
            .args(args)
            .output()
            .expect("invoke rtm")
    }

    fn say(&self, args: &[&str]) -> (bool, String) {
        let output = self.rtm(args);
        (output.status.success(), support::text(&output))
    }

    fn status(&self, id: &str) -> (bool, String) {
        self.say(&["status", "--run", id])
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.root.join(relative))
            .unwrap_or_else(|error| panic!("read {relative}: {error}"))
    }

    fn write(&self, relative: &str, body: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("a parent")).expect("create parent");
        fs::write(&path, body).unwrap_or_else(|error| panic!("write {relative}: {error}"));
    }

    fn drift(&self, drift: Drift) {
        let runbook = self.root.join(".ratmac/ratmac.toml");
        match drift {
            Drift::Changed => fs::write(&runbook, changed_runbook()),
            Drift::Removed => fs::remove_file(&runbook),
            Drift::Corrupt => fs::write(&runbook, "]]] not toml"),
        }
        .unwrap_or_else(|error| panic!("apply {drift:?}: {error}"));
    }

    fn heal(&self) {
        self.write(".ratmac/ratmac.toml", RUNBOOK);
    }

    /// Every byte of the fixture, links refused.
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

    /// Every non-empty value the Run's evidence file records, paths in the
    /// forward-slash spelling the Engine shows.
    fn recorded_values(&self, id: &str) -> Vec<String> {
        let source = self.read(&format!(".ratmac/runs/{id}/evidence.toml"));
        let value: toml::Value = source.parse().expect("the recorded evidence parses");
        let mut values = Vec::new();
        collect_values(&value, &mut values);
        assert!(!values.is_empty(), "{id} records at least one identity");
        values
    }
}

fn collect_values(value: &toml::Value, values: &mut Vec<String>) {
    match value {
        toml::Value::String(text) if !text.is_empty() => values.push(text.replace('\\', "/")),
        toml::Value::Table(table) => table
            .values()
            .for_each(|inner| collect_values(inner, values)),
        toml::Value::Array(items) => items.iter().for_each(|item| collect_values(item, values)),
        _ => {}
    }
}

/// Whether the text carries anything shaped like a SHA-256 digest.
fn has_digest(text: &str) -> bool {
    let mut run = 0;
    for byte in text.bytes() {
        run = if byte.is_ascii_hexdigit() { run + 1 } else { 0 };
        if run == 64 {
            return true;
        }
    }
    false
}

fn next_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.starts_with("next:"))
        .collect()
}

/// The history view's own statement that no current instructions are shown.
fn says_instructions_unavailable(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("current instructions") && lower.contains("unavailable")
}

/// A passed Run read as history: exit 0, its recorded facts, no current
/// prompt or guard, no taught motion, and no retirement suggestion.
fn assert_history(project: &Project, id: &str, state: &str, ok: bool, text: &str, context: &str) {
    assert!(ok, "{context}: the passed Run {id} stays readable: {text}");
    assert!(
        text.contains(id),
        "{context}: the view names the Run {id}: {text}"
    );
    assert!(
        text.contains(&format!("State: {state}\n")) && text.contains("Status: passed\n"),
        "{context}: the recorded State and status are shown: {text}"
    );
    for value in project.recorded_values(id) {
        assert!(
            text.contains(&value),
            "{context}: the recorded evidence value {value} is shown: {text}"
        );
    }
    assert!(
        says_instructions_unavailable(text),
        "{context}: the view says current instructions are unavailable: {text}"
    );
    assert!(
        !text.contains(CHANGED_MARK),
        "{context}: a changed prompt is never shown as the old Run's instructions: {text}"
    );
    assert!(
        !text.contains("Exit Guards")
            && !text.contains("pending guard")
            && !text.contains(GUARD_MARK),
        "{context}: no current guard is shown for the finished Run: {text}"
    );
    assert!(
        next_lines(text).is_empty(),
        "{context}: a finished Run is taught no further act: {text}"
    );
    let lower = text.to_ascii_lowercase();
    assert!(
        !lower.contains("abandon") && !lower.contains("retire"),
        "{context}: reading a finished Run never suggests retirement: {text}"
    );
}

/// WRSV-006-01: finish a parent and child, then change, remove, and corrupt
/// the runbook; status still reports each persisted passed Run with no
/// changed prompt and no file mutation.
#[test]
fn wrsv_006_01() {
    let project = Project::finished("drift");

    // With the runbook the Runs passed under, the same history view.
    for (id, state) in [("run-001", "done"), ("run-002", "finished")] {
        let before = project.bytes();
        let (ok, text) = project.status(id);
        assert_history(&project, id, state, ok, &text, "unchanged runbook");
        assert_eq!(project.bytes(), before, "reading {id} changes nothing");
    }
    let (_, child) = project.status("run-002");
    assert!(
        child.contains("reviewer"),
        "the child's identity names its recorded class: {child}"
    );

    for drift in DRIFTS {
        project.drift(drift);
        for (id, state) in [("run-001", "done"), ("run-002", "finished")] {
            let before = project.bytes();
            let (ok, text) = project.status(id);
            assert_history(&project, id, state, ok, &text, &format!("{drift:?}"));
            assert_eq!(
                project.bytes(),
                before,
                "{drift:?}: reading {id} changes nothing"
            );
        }
        if let Drift::Changed = drift {
            assert!(
                !project.root.join(GUARD_MARK).exists(),
                "history never evaluates the changed runbook's guards"
            );
        }
        project.heal();
    }
}

/// WRSV-006-02: record checksums of records, evidence, receipts, ledger
/// abandoned flags, and logs before repeated status; every byte remains
/// identical.
#[test]
fn wrsv_006_02() {
    let project = Project::finished("bytes");
    project.write(
        ".ratmac/evidence/run-002/completion/review.toml",
        "ticket-id = \"t-1\"\nexit-status = 0\n",
    );
    project.write(".ratmac/log.md", "- a human note that must survive\n");
    let ledger = ".ratmac/runs/run-001/spawn-ledger";
    assert!(
        project.read(ledger).contains("abandoned = false"),
        "the child's ledger entry starts live: {}",
        project.read(ledger)
    );

    for drift in DRIFTS {
        project.drift(drift);
        let before = project.bytes();
        for id in ["run-001", "run-002"] {
            let (ok, first) = project.status(id);
            assert!(ok, "{drift:?}: {id} reads: {first}");
            for _ in 0..3 {
                let (again_ok, again) = project.status(id);
                assert!(
                    again_ok && again == first,
                    "{drift:?}: repeated status of {id} reads the same facts: {again}"
                );
            }
        }
        let after = project.bytes();
        assert_eq!(
            after.keys().collect::<Vec<_>>(),
            before.keys().collect::<Vec<_>>(),
            "{drift:?}: no file, lock, or folder appears or disappears"
        );
        assert_eq!(after, before, "{drift:?}: every byte stays identical");
        assert!(
            project.read(ledger).contains("abandoned = false"),
            "{drift:?}: reading never marks the child abandoned"
        );
        project.heal();
    }
}

/// WRSV-006-03: perform the same drift on a nonterminal Run; current
/// status/motion still refuses, while terminal step and hold remain
/// prohibited.
#[test]
fn wrsv_006_03() {
    let project = Project::finished("motion");
    let (ok, text) = project.say(&["start"]);
    assert!(ok, "a third Run starts: {text}");
    let live = "run-003";

    for drift in DRIFTS {
        project.drift(drift);
        let before = project.bytes();

        // The live Run keeps its existing refusals.
        for route in [&["status", "--run", live][..], &["step", "--run", live]] {
            let (ok, text) = project.say(route);
            assert!(!ok, "{drift:?}/{route:?}: the live Run refuses: {text}");
            assert!(
                !says_instructions_unavailable(&text),
                "{drift:?}/{route:?}: a live Run is never read as history: {text}"
            );
            assert!(
                text.contains("ratmac.toml"),
                "{drift:?}/{route:?}: the refusal names the runbook: {text}"
            );
        }
        if let Drift::Changed = drift {
            let (_, text) = project.status(live);
            assert!(
                text.contains("runbook pin mismatch"),
                "the live Run keeps its pin check: {text}"
            );
        }

        // Positive control: the passed Run reads under the same drift.
        let (ok, text) = project.status("run-001");
        assert_history(
            &project,
            "run-001",
            "done",
            ok,
            &text,
            &format!("{drift:?}"),
        );

        // Reading authorizes no motion on a passed Run.
        for id in ["run-001", "run-002"] {
            let (ok, stepped) = project.say(&["step", "--run", id]);
            assert!(
                !ok || stepped.contains("step refused"),
                "{drift:?}: a passed Run never moves: {stepped}"
            );
        }
        let (held, text) = project.say(&[
            "hold",
            "--run",
            "run-001",
            "--blocker",
            BLOCKER,
            "--confirm",
            "hold run-001",
        ]);
        assert!(!held, "{drift:?}: a passed Run is never held: {text}");
        assert_eq!(
            project.bytes(),
            before,
            "{drift:?}: every refusal changes nothing"
        );
        project.heal();
    }

    // Healthy runbook: terminal step and hold stay refused, the live Run
    // keeps its ordinary current view.
    let before = project.bytes();
    let (_, stepped) = project.say(&["step", "--run", "run-001"]);
    assert!(
        stepped.contains("step refused"),
        "a passed Run's step is refused: {stepped}"
    );
    let (held, text) = project.say(&[
        "hold",
        "--run",
        "run-001",
        "--blocker",
        BLOCKER,
        "--confirm",
        "hold run-001",
    ]);
    assert!(
        !held && text.contains("terminal"),
        "a passed Run's hold is refused as terminal: {text}"
    );
    let (ok, current) = project.status(live);
    assert!(
        ok && current.contains("Status: planned")
            && current.contains("Parent plans the work.")
            && current.contains("next: rtm step --run run-003")
            && !says_instructions_unavailable(&current),
        "the live Run shows its current instructions: {current}"
    );
    assert_eq!(
        project.bytes(),
        before,
        "the healthy refusals change nothing"
    );

    // Positive control: the same hold is accepted for the live Run, so the
    // passed Run's refusal is about its terminal status alone.
    let (held, text) = project.say(&[
        "hold",
        "--run",
        live,
        "--blocker",
        BLOCKER,
        "--confirm",
        "hold run-003",
    ]);
    assert!(held, "the live Run accepts the same hold: {text}");
}

/// WRSV-006-04: malformed Run Record, invalid address, unknown Run, and
/// unreadable evidence produce honest named diagnostics without bypassing
/// residue checks or implying verification.
#[test]
fn wrsv_006_04() {
    let project = Project::finished("refusals");
    project.drift(Drift::Changed);

    // Positive control: the healthy passed Run reads under drift.
    let (ok, text) = project.status("run-001");
    assert_history(&project, "run-001", "done", ok, &text, "control");

    let refuses = |args: &[&str], names: &str, context: &str| {
        let before = project.bytes();
        let (ok, text) = project.say(args);
        assert!(!ok, "{context}: refuses: {text}");
        assert!(
            text.contains(names),
            "{context}: the refusal names {names}: {text}"
        );
        assert!(
            !says_instructions_unavailable(&text) && !text.contains("Status: passed"),
            "{context}: nothing is reported as a readable passed Run: {text}"
        );
        assert!(
            !text.to_ascii_lowercase().contains("verified"),
            "{context}: nothing is claimed verified: {text}"
        );
        assert_eq!(
            project.bytes(),
            before,
            "{context}: the refusal changes nothing"
        );
        text
    };

    // Bad addresses and an unknown Run list the roster.
    for bad in ["run-1", "../run-001", "run-999", "RUN-001"] {
        refuses(&["status", "--run", bad], "runs: run-001, run-002", bad);
    }

    // Malformed Run Records of a passed Run refuse by name, even under drift.
    let record = ".ratmac/runs/run-002/run.toml";
    let healthy = project.read(record);
    for (label, body) in [
        ("garbage", "]]] not toml".to_owned()),
        (
            "truncated",
            "state = \"finished\"\nstatus = \"passed\"\n".to_owned(),
        ),
        ("extra field", format!("{healthy}extra = \"x\"\n")),
        (
            "bad status",
            healthy.replace("\"passed\"", "\"finished-ish\""),
        ),
    ] {
        project.write(record, &body);
        refuses(&["status", "--run", "run-002"], "run.toml", label);
    }
    project.write(record, &healthy);

    // Unreadable evidence refuses by name; nothing is claimed.
    let evidence = ".ratmac/runs/run-001/evidence.toml";
    let recorded = project.read(evidence);
    project.write(evidence, "[engine\nsha256 = ");
    refuses(
        &["status", "--run", "run-001"],
        "evidence.toml",
        "garbage evidence",
    );
    fs::remove_file(project.root.join(evidence)).expect("remove the evidence file");
    fs::create_dir(project.root.join(evidence)).expect("plant a folder in its place");
    refuses(
        &["status", "--run", "run-001"],
        "evidence.toml",
        "evidence folder",
    );
    fs::remove_dir(project.root.join(evidence)).expect("remove the folder");

    // Evidence that is TOML but not the recorded shape refuses naming the
    // file and the field; no identity is dropped in silence.
    for (label, body, field) in [
        (
            "engine without its digest",
            "[engine]\nresolved = \"C:/e/rtm.exe\"\n".to_owned(),
            "engine.sha256",
        ),
        (
            "gate digest that is a number",
            "[[gate]]\nprogram = \"git\"\nresolved = \"C:/g/git.exe\"\nsha256 = 7\n".to_owned(),
            "gate[0].sha256",
        ),
        (
            "goal revision that is a number",
            "[goal]\nbaseline = 1\nfrozen = \"\"\n".to_owned(),
            "goal.baseline",
        ),
        (
            "unknown field",
            format!("extra = \"x\"\n{recorded}"),
            "extra",
        ),
        (
            "unknown field inside a table",
            "[runbook]\nsha256 = \"4444\"\nnote = \"x\"\n".to_owned(),
            "runbook.note",
        ),
        (
            "empty runbook digest",
            "[runbook]\nsha256 = \"\"\n".to_owned(),
            "runbook.sha256",
        ),
    ] {
        project.write(evidence, &body);
        let text = refuses(&["status", "--run", "run-001"], "evidence.toml", label);
        assert!(
            text.contains(field),
            "{label}: the refusal names the field {field}: {text}"
        );
    }
    project.write(evidence, &recorded);

    // Absent evidence is named as absent, never shown as recorded, and no
    // identity is guessed in its place.
    let child = ".ratmac/runs/run-002/evidence.toml";
    let seeded = project.recorded_values("run-002");
    fs::remove_file(project.root.join(child)).expect("remove the child's evidence");
    let before = project.bytes();
    let (ok, text) = project.status("run-002");
    assert!(
        ok && text.contains("evidence.toml")
            && text.contains("Status: passed")
            && text.to_ascii_lowercase().contains("absent"),
        "a passed Run without evidence reads and names the absent file: {text}"
    );
    assert!(
        !has_digest(&text) && seeded.iter().all(|value| !text.contains(value)),
        "no identity is invented for absent evidence: {text}"
    );
    assert_eq!(
        project.bytes(),
        before,
        "reading without evidence changes nothing"
    );
    project.write(child, SEEDED_EVIDENCE);

    // Residue is still refused first.
    project.write(".ratmac/state.toml", "state = \"intake\"\n");
    refuses(
        &["status", "--run", "run-001"],
        "state.toml",
        "pre-split residue",
    );

    // A passed child bound to a linked worktree: residue in that bound
    // workspace refuses its history read first, by name, as it refuses every
    // other addressed operation on that child.
    let bound = Project::finished_linked("bound");
    let workspace = bound.linked.clone().expect("a bound workspace");
    let (ok, text) = bound.status("run-002");
    assert!(
        ok && text.contains("Status: passed"),
        "the bound child reads as history: {text}"
    );
    fs::write(workspace.join(".arca/state.toml"), "state = \"intake\"\n")
        .expect("plant bound-workspace residue");
    let before = bound.bytes();
    let (ok, text) = bound.status("run-002");
    assert!(
        !ok && text.contains("histproj-linked/.arca/state.toml")
            && !text.contains("Status: passed"),
        "residue in the bound workspace refuses the history read by name: {text}"
    );
    assert_eq!(
        bound.bytes(),
        before,
        "the bound-workspace refusal changes nothing"
    );
}
