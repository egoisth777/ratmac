//! t-087 / NRR-001: the Engine has no work-item concept.
//!
//! NRRV-001 `hold_records_the_pause_in_engine_state_alone`
//! NRRV-002 `the_engine_names_no_work_item_document`
//!
//! `rtm` is a generic state-machine runner. Some runbooks have no notion of a
//! ticket at all, so pausing a Run is Engine work end to end: the Run Record
//! and the transition log carry the whole fact, the blocker is an opaque
//! reference the Engine only locates, and no file under a workflow root is
//! touched.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A project whose runbook declares a blocked route and one workflow root
/// full of documents the Engine must never write.
struct Fixture {
    root: PathBuf,
    run_id: String,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    fn new(label: &str, runbook: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t087-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in ["work", "work/notes", ".ratmac"] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        // Documents inside the declared workflow root. One of them wears the
        // shape the Engine used to write, so a leftover write would be caught.
        fs::write(
            root.join("work/t-900.md"),
            "---\nticket-id: t-900\nstatus: \"executing\"\nblocker-ref: \"\"\n---\n\n# not the Engine's business\n",
        )
        .expect("write the convention-shaped document");
        fs::write(root.join("work/notes/plain.txt"), "an ordinary file\n")
            .expect("write the plain document");
        fs::write(root.join("work/blocker.txt"), "why the work stopped\n")
            .expect("write the blocker reference");
        fs::write(root.join(".ratmac/ratmac.toml"), runbook).expect("write machine class");

        let mut fixture = Fixture {
            root,
            run_id: String::new(),
        };
        assert!(
            fixture.rtm(&["start"]).status.success(),
            "the fixture Run starts"
        );
        fixture.run_id = fs::read_dir(fixture.root.join(".ratmac/runs"))
            .expect("list the runs roster")
            .map(|entry| entry.expect("roster entry is readable"))
            .find(|entry| entry.path().is_dir())
            .expect("the started Run appears on the roster")
            .file_name()
            .to_string_lossy()
            .into_owned();
        let id = fixture.run_id.clone();
        assert!(
            fixture.rtm(&["step", "--run", &id]).status.success(),
            "the Run reaches the state that declares a blocked route"
        );
        fixture
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    fn text(&self, args: &[&str]) -> String {
        let output = self.rtm(args);
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }

    fn hold(&self, args: &[&str]) -> Output {
        let mut all = vec!["hold"];
        all.extend_from_slice(args);
        all.push("--run");
        all.push(&self.run_id);
        self.rtm(&all)
    }

    fn hold_text(&self, args: &[&str]) -> String {
        let output = self.hold(args);
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }

    fn record_path(&self) -> PathBuf {
        self.root
            .join(".ratmac/runs")
            .join(&self.run_id)
            .join("run.toml")
    }

    fn record(&self) -> String {
        fs::read_to_string(self.record_path()).expect("read the Run Record")
    }

    fn record_field(&self, key: &str) -> String {
        let record = self.record();
        record
            .lines()
            .find_map(|line| line.trim().strip_prefix(&format!("{key} = ")))
            .map(|value| value.trim().trim_matches('"').to_owned())
            .unwrap_or_else(|| panic!("the Run Record carries a {key} field: {record}"))
    }

    fn transition_log(&self) -> String {
        fs::read_to_string(self.root.join(".ratmac/log.md")).unwrap_or_default()
    }

    /// Every file under the declared workflow root, by relative path.
    fn workflow_bytes(&self) -> BTreeMap<String, Vec<u8>> {
        let mut tree = BTreeMap::new();
        walk(&self.root.join("work"), &self.root, &mut tree);
        tree
    }
}

fn walk(directory: &Path, base: &Path, tree: &mut BTreeMap<String, Vec<u8>>) {
    for entry in fs::read_dir(directory).expect("read workflow root") {
        let path = entry.expect("read workflow entry").path();
        if path.is_dir() {
            walk(&path, base, tree);
        } else {
            let key = path
                .strip_prefix(base)
                .expect("workflow file stays under the project")
                .to_string_lossy()
                .replace('\\', "/");
            tree.insert(key, fs::read(&path).expect("read workflow file"));
        }
    }
}

/// A runbook whose vocabulary is entirely the shop's: it names a `work` root
/// and three states, and says nothing about tickets.
/// The same machine, with the completion gate armed on the State the Run
/// is paused in, so the gate's own verdict is observable.
fn gated_runbook() -> String {
    "[roots]\n\
     work = \"work\"\n\n\
     [states.intake]\nprompt = \"Take the next thing in.\"\n\n\
     [states.build]\nprompt = \"Do the work.\"\n\
     guards = [{ kind = \"completion_gate\", ticket = \"work/t-900.md\" }]\n\n\
     [states.review]\nprompt = \"Review the work.\"\n\n\
     [[transitions]]\nfrom = \"intake\"\nto = \"build\"\n\n\
     [[transitions]]\nfrom = \"build\"\nto = \"intake\"\nblocked-route = true\n\n\
     [[transitions]]\nfrom = \"build\"\nto = \"review\"\n"
        .to_owned()
}

fn work_item_free_runbook() -> String {
    "[roots]\n\
     work = \"work\"\n\n\
     [states.intake]\nprompt = \"Take the next thing in.\"\n\n\
     [states.build]\nprompt = \"Do the work.\"\n\n\
     [states.review]\nprompt = \"Review the work.\"\n\n\
     [[transitions]]\nfrom = \"intake\"\nto = \"build\"\n\n\
     [[transitions]]\nfrom = \"build\"\nto = \"intake\"\nblocked-route = true\n\n\
     [[transitions]]\nfrom = \"build\"\nto = \"review\"\n"
        .to_owned()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn engine_source_files() -> Vec<PathBuf> {
    fn collect(directory: &Path, files: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(directory).expect("read Engine source directory") {
            let path = entry.expect("read Engine source entry").path();
            if path.is_dir() {
                collect(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    let mut files = Vec::new();
    collect(&repo_root().join("src"), &mut files);
    files.sort();
    files
}

/// NRRV-001: a full hold leaves every file under a workflow root
/// byte-identical, and the Run Record and transition log carry the whole fact.
#[test]
fn hold_records_the_pause_in_engine_state_alone() {
    let fixture = Fixture::new("pause-in-state", &work_item_free_runbook());
    let before = fixture.workflow_bytes();
    assert!(
        before.contains_key("work/t-900.md"),
        "the fixture plants a document shaped like the old ticket convention"
    );

    let confirmation = format!("hold {}", fixture.run_id);
    let output = fixture.hold(&["--blocker", "work/blocker.txt", "--confirm", &confirmation]);
    assert!(
        output.status.success(),
        "a confirmed hold against a locatable blocker succeeds: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        fixture.workflow_bytes(),
        before,
        "NRR-001: a hold writes no file under a workflow root"
    );
    assert_eq!(
        fixture.record_field("status"),
        "blocked",
        "the Run Record carries the paused mark: {}",
        fixture.record()
    );
    assert_eq!(
        fixture.record_field("blocker"),
        "work/blocker.txt",
        "the Run Record carries the blocker reference verbatim: {}",
        fixture.record()
    );
    assert_eq!(
        fixture.record_field("state"),
        "intake",
        "the hold routes the Run along its declared blocked edge"
    );

    let log = fixture.transition_log();
    let entries = log
        .lines()
        .filter(|line| line.to_lowercase().contains("hold"))
        .count();
    assert_eq!(entries, 1, "the Engine appends exactly one entry: {log}");
    assert!(
        log.contains(&fixture.run_id) && log.contains("work/blocker.txt"),
        "the entry names the Run and its blocker: {log}"
    );

    // The completion gate learns the pause from Engine-owned state. The gate
    // is addressed at the paused Run, and the workflow document says nothing.
    let refusal = fixture.text(&["status", "--run", &fixture.run_id]);
    assert!(
        refusal.contains("work/blocker.txt"),
        "status reports the blocker the Engine recorded: {refusal}"
    );

    // The completion gate itself refuses the paused Run, and it learns the
    // pause from the Run Record: the gate is armed on the State the Run is
    // in, and the workflow document it names says nothing about a pause.
    let gated = Fixture::new("gated", &gated_runbook());
    let gate_phrase = format!("hold {}", gated.run_id);
    let document_before = fs::read(gated.root.join("work/t-900.md")).expect("read the document");
    assert!(
        gated
            .hold(&["--blocker", "work/blocker.txt", "--confirm", &gate_phrase])
            .status
            .success(),
        "the gated fixture holds"
    );
    // The blocked route moved the Run back to intake; step it to the guarded
    // State, then ask to leave it - that is when the gate speaks.
    let id = gated.run_id.clone();
    gated.rtm(&["step", "--run", &id]);
    let verdict = gated.text(&["step", "--run", &id]);
    assert!(
        verdict.contains("completion_gate")
            && verdict.contains("paused")
            && verdict.contains("work/blocker.txt"),
        "the completion gate refuses the paused Run and names the pause: {verdict}"
    );
    assert_eq!(
        fs::read(gated.root.join("work/t-900.md")).expect("re-read the document"),
        document_before,
        "the gate's verdict came from Engine state, not from the document"
    );

    // A blocker that escapes its declared root refuses, and refuses first.
    let escaped = Fixture::new("escaping-blocker", &work_item_free_runbook());
    let phrase = format!("hold {}", escaped.run_id);
    let untouched = escaped.workflow_bytes();
    let before_status = escaped.record_field("status");
    let text = escaped.hold_text(&["--blocker", "../outside.txt", "--confirm", &phrase]);
    assert!(
        text.to_lowercase().contains("blocker"),
        "an escaping blocker refuses by naming the blocker: {text}"
    );
    assert_ne!(before_status, "blocked", "the fixture Run starts unpaused");
    assert_eq!(
        escaped.record_field("status"),
        before_status,
        "a refused hold leaves the Run unpaused"
    );
    assert_eq!(
        escaped.workflow_bytes(),
        untouched,
        "a refused hold writes nothing"
    );
}

/// NRRV-002: no Engine argument, message, refusal, field, or path names a
/// work-item document or its shape.
#[test]
fn the_engine_names_no_work_item_document() {
    // Nowhere in the Engine is a work item held, marked, or read: the three
    // helpers that owned that knowledge are gone by name.
    let mut offenders: Vec<String> = Vec::new();
    for path in engine_source_files() {
        let text = fs::read_to_string(&path).expect("read Engine source file");
        let relative = path
            .strip_prefix(repo_root())
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        for (index, line) in text.lines().enumerate() {
            let lowered = line.to_lowercase();
            let holds_a_document = lowered.contains("ticket_file_name")
                || lowered.contains("hold_ticket")
                || lowered.contains("held_against");
            if holds_a_document {
                offenders.push(format!("{relative}:{}: {}", index + 1, line.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "NRR-001: no Engine path marks or reads a work-item document:\n{}",
        offenders.join("\n")
    );

    // The hold surface itself - the module and the command that route a
    // paused Run - names no work item at all. Other Engine features (the
    // completion gate's declared checks) carry their own contracts and are
    // not this row's subject.
    let hold_module =
        fs::read_to_string(repo_root().join("src/blocked.rs")).expect("read the hold module");
    let cli = fs::read_to_string(repo_root().join("src/cli.rs")).expect("read the command surface");
    let hold_command = {
        let start = cli
            .find("fn hold<W: Write>")
            .expect("the hold command exists");
        let usage = cli
            .find("\"Usage: rtm hold")
            .expect("the hold usage exists");
        let end = cli[start..]
            .find("\n/// PGE-007")
            .map(|offset| start + offset)
            .unwrap_or(cli.len());
        format!(
            "{}\n{}",
            &cli[usage..cli[usage..].find('\n').unwrap_or(0) + usage + 1],
            &cli[start..end]
        )
    };
    let mut mentions: Vec<String> = Vec::new();
    for (surface, body) in [
        ("src/blocked.rs", &hold_module),
        ("rtm hold", &hold_command),
    ] {
        for (index, line) in body.lines().enumerate() {
            if line.to_lowercase().contains("ticket") {
                mentions.push(format!("{surface}:{}: {}", index + 1, line.trim()));
            }
        }
    }
    assert!(
        mentions.is_empty(),
        "NRR-001: the hold surface names no work item:\n{}",
        mentions.join("\n")
    );

    // A runbook that never mentions a work item still holds, routes, and has
    // its completion refused.
    let fixture = Fixture::new("work-item-free", &work_item_free_runbook());
    let confirmation = format!("hold {}", fixture.run_id);
    let held = fixture.hold_text(&[
        "--blocker",
        "work/notes/plain.txt",
        "--confirm",
        &confirmation,
    ]);
    assert!(
        !held.to_lowercase().contains("ticket"),
        "no Engine message names a work item: {held}"
    );
    assert!(
        held.contains(&fixture.run_id),
        "the hold reports the Run it paused: {held}"
    );
    assert_eq!(
        fixture.record_field("state"),
        "intake",
        "the work-item-free runbook routes on its blocked edge"
    );

    let status = fixture.text(&["status", "--run", &fixture.run_id]);
    assert!(
        !status.to_lowercase().contains("ticket"),
        "status names no work item: {status}"
    );

    // The usage text a caller reads must not teach a work-item vocabulary
    // either: it addresses a Run.
    let usage = fixture.text(&["hold"]);
    assert!(
        !usage.to_lowercase().contains("ticket"),
        "the hold usage names no work item: {usage}"
    );
    assert!(
        usage.contains("--run"),
        "the hold usage addresses a Run: {usage}"
    );
}
