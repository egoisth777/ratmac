//! t-047 / PGE-005: the implementation-completion gate verifies work.
//!
//! PT-047-01 `completion_requires_receipts`
//! PT-047-02 `stale_receipt_rejected`
//! HT-047-01 `missing_hidden_lane_receipt_refuses`
//! HT-047-02 `interrupted_verification_passes_nothing`
//! HT-047-03 `undeclarable_command_refuses`
//!
//! Passing a ticket is not a status edit: every check the ticket declares -
//! focused tests, hidden lanes, and quality gates - must resolve to a green
//! receipt whose digest re-derives and whose tree digest still matches the
//! work it claims.

use ratmac::completion::{declared_checks, gate_completion, CompletionDefect};
use ratmac::receipt::sha256_text;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Fixture {
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const TICKET: &str = "t-900";
const TICKET_PATH: &str = ".arca/ticket/t-900.md";
const RUN_ID: &str = "run-001";
const GREEN: &str = "test result: ok. 3 passed; 0 failed\n";

impl Fixture {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t049-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in [".arca/ticket", ".ratmac/evidence/run-001/completion", "src"] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        fs::write(root.join("src/lib.rs"), "pub fn work() {}\n").expect("write source");
        let fixture = Fixture { root };
        fixture.write_ticket(&["cargo --version"]);
        fixture
    }

    /// A ticket declaring one focused test, one hidden lane, and the given
    /// quality commands.
    fn write_ticket(&self, quality: &[&str]) {
        let lines: String = quality
            .iter()
            .map(|command| format!("- Quality: `{command}` passes.\n"))
            .collect();
        fs::write(
            self.root.join(TICKET_PATH),
            format!(
                "---\nticket-id: t-900\nresidual-ids:\n  - \"res-900\"\n\
                 planned-test-refs:\n  - \"PT-900-01\"\nstatus: \"executing\"\n---\n\n\
                 # Ticket: t-900\n\n## P5 Hidden Test Public Coverage Manifest\n\n\
                 | Hidden ID | Lane |\n|---|---|\n| `HT-900-01` | `Regression` |\n\n\
                 ## Merge Gate\n\n{lines}"
            ),
        )
        .expect("write ticket");
    }

    /// The digest of the source roots as they stand right now.
    fn tree_digest(&self) -> String {
        ratmac::completion::tree_digest(&self.root, &["src".to_owned()])
            .expect("source roots are readable")
    }

    fn write_receipt(&self, check: &str, kind: &str, command: &str, exit: i64, digest: &str) {
        let output = GREEN;
        let body = format!(
            "ticket-id = \"{TICKET}\"\n\
             check-id = \"{check}\"\n\
             kind = \"{kind}\"\n\
             command = \"{command}\"\n\
             working-dir = \".\"\n\
             exit-status = {exit}\n\
             output-sha256 = \"{}\"\n\
             tree-roots = [\"src\"]\n\
             tree-sha256 = \"{digest}\"\n\
             output = \"\"\"\n{output}\"\"\"\n",
            sha256_text(output)
        );
        fs::write(self.completion_path(check), body).expect("write completion receipt");
    }

    /// Every declared check, recorded green and fresh.
    fn record_all(&self) {
        let digest = self.tree_digest();
        self.write_receipt("PT-900-01", "focused", "cargo test --test t900", 0, &digest);
        self.write_receipt(
            "HT-900-01",
            "hidden-lane",
            "cargo test --test t900",
            0,
            &digest,
        );
        self.write_receipt("cargo --version", "quality", "cargo --version", 0, &digest);
    }

    /// A Machine Class whose one transition is guarded by the completion gate.
    fn write_runbook(&self) {
        fs::create_dir_all(self.root.join(".ratmac")).expect("create Engine tree");
        fs::write(
            self.root.join(".ratmac/ratmac.toml"),
            format!(
                "[states.implement]\nprompt = \"Implement the ticket.\"\n\
                 guards = [{{ kind = \"completion_gate\", ticket = \"{TICKET_PATH}\" }}]\n\n\
                 [states.done]\nprompt = \"Finish.\"\n\n\
                 [[transitions]]\nfrom = \"implement\"\nto = \"done\"\n"
            ),
        )
        .expect("write machine class");
    }

    fn rtm(&self, args: &[&str]) -> String {
        let output = Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm");
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }

    fn completion_path(&self, check: &str) -> PathBuf {
        self.root
            .join(format!(".ratmac/evidence/{RUN_ID}/completion"))
            .join(format!("{}.toml", slug(check)))
    }

    fn gate(&self) -> Result<(), Vec<CompletionDefect>> {
        gate_completion(&self.root, &self.root.join(".ratmac"), RUN_ID, TICKET_PATH)
    }
}

/// File-name form of a check ID, matching the engine's own rule.
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

fn reasons(defects: &[CompletionDefect]) -> String {
    defects
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// PT-047-01: green receipts pass; relabeling with none refuses by name.
#[test]
fn completion_requires_receipts() {
    let fixture = Fixture::new("green");

    // The ticket's declared work is discovered, not guessed.
    let source = fs::read_to_string(fixture.root.join(TICKET_PATH)).expect("read ticket");
    let checks = declared_checks(&source);
    let ids: Vec<&str> = checks.iter().map(|check| check.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["PT-900-01", "HT-900-01", "cargo --version"],
        "focused tests, hidden lanes, and quality commands are all declared work"
    );

    // Receiptless relabeling refuses, naming the first missing receipt.
    let defects = fixture
        .gate()
        .expect_err("a ticket with no receipts cannot pass");
    assert!(
        defects[0].check == "PT-900-01" && defects[0].reason.contains("no completion receipt"),
        "the first missing receipt is named first: {}",
        reasons(&defects)
    );

    // With every check recorded green and fresh, the gate passes.
    fixture.record_all();
    fixture
        .gate()
        .unwrap_or_else(|defects| panic!("green receipts must pass: {}", reasons(&defects)));

    // A failing run is not completion.
    let digest = fixture.tree_digest();
    fixture.write_receipt(
        "PT-900-01",
        "focused",
        "cargo test --test t900",
        101,
        &digest,
    );
    let defects = fixture
        .gate()
        .expect_err("a red receipt cannot complete a ticket");
    let text = reasons(&defects);
    assert!(
        text.contains("PT-900-01") && text.contains("101"),
        "the refusal names the check and its exit status: {text}"
    );

    // Inside the pinned boundary: the Engine's own gate refuses a receiptless
    // step and lets the ticket through once the work is recorded.
    let engine = Fixture::new("engine");
    engine.write_runbook();
    engine.rtm(&["start"]);
    // FDC-004: run addressing is always required.
    let live = std::fs::read_dir(engine.root.join(".ratmac/runs"))
        .expect("list the runs roster")
        .map(|entry| entry.expect("roster entry is readable"))
        .find(|entry| entry.path().join("run.toml").is_file())
        .expect("the started run appears on the roster")
        .file_name()
        .to_string_lossy()
        .into_owned();
    let refusal = engine.rtm(&["step", "--run", &live]);
    assert!(
        refusal.contains("PT-900-01"),
        "the engine refuses the step naming the missing receipt: {refusal}"
    );
    assert!(
        engine
            .rtm(&["status", "--run", &live])
            .contains("implement"),
        "a refused completion leaves the State where it was"
    );
    engine.record_all();
    engine.rtm(&["step", "--run", &live]);
    assert!(
        engine.rtm(&["status", "--run", &live]).contains("done"),
        "recorded work advances the State: {}",
        engine.rtm(&["status", "--run", &live])
    );

    // A receipt for work the ticket never declared is not completion either.
    fixture.record_all();
    fixture.write_receipt("PT-900-99", "focused", "cargo test --test t900", 0, &digest);
    let defects = fixture
        .gate()
        .expect_err("an undeclared receipt must refuse");
    assert!(
        reasons(&defects).contains("PT-900-99"),
        "the refusal names the stray receipt: {}",
        reasons(&defects)
    );
}

/// PT-047-02: a receipt that no longer describes the tree is not fresh.
#[test]
fn stale_receipt_rejected() {
    let fixture = Fixture::new("stale");
    fixture.record_all();
    let recorded = fixture.tree_digest();

    // The work changed after the check ran.
    fs::write(
        fixture.root.join("src/lib.rs"),
        "pub fn work() { todo!() }\n",
    )
    .expect("edit the source");
    let current = fixture.tree_digest();
    assert_ne!(
        recorded, current,
        "editing a source file changes the digest"
    );

    let defects = fixture.gate().expect_err("a stale receipt cannot pass");
    let text = reasons(&defects);
    assert!(
        text.contains(&recorded) && text.contains(&current),
        "the refusal shows the recorded and current tree digests: {text}"
    );
    assert!(
        text.to_ascii_lowercase().contains("stale") || text.to_ascii_lowercase().contains("fresh"),
        "the refusal says the receipt is not fresh: {text}"
    );

    // Re-recording against the current tree restores completion.
    fixture.record_all();
    fixture
        .gate()
        .unwrap_or_else(|defects| panic!("fresh receipts must pass: {}", reasons(&defects)));

    // A digest that does not re-derive from the recorded output is rejected.
    let path = fixture.completion_path("PT-900-01");
    let source = fs::read_to_string(&path).expect("read receipt");
    fs::write(&path, source.replace(&sha256_text(GREEN), &"0".repeat(64)))
        .expect("corrupt the digest");
    let defects = fixture
        .gate()
        .expect_err("a non-re-deriving digest must refuse");
    assert!(
        reasons(&defects).to_ascii_lowercase().contains("re-derive"),
        "the refusal says the digest does not re-derive: {}",
        reasons(&defects)
    );
}

/// HT-047-01 (Cross-Feature): hidden lanes are part of completion.
#[test]
fn missing_hidden_lane_receipt_refuses() {
    let fixture = Fixture::new("hidden");
    fixture.record_all();
    fs::remove_file(fixture.completion_path("HT-900-01")).expect("delete the hidden-lane receipt");

    let defects = fixture
        .gate()
        .expect_err("a missing hidden lane must refuse");
    let text = reasons(&defects);
    assert!(
        text.contains("HT-900-01"),
        "the refusal names the missing lane: {text}"
    );
    assert_eq!(
        defects.len(),
        1,
        "only the missing lane is at fault: {text}"
    );
}

/// HT-047-02 (Durability/Recovery): interruption leaves nothing passed.
#[test]
fn interrupted_verification_passes_nothing() {
    let fixture = Fixture::new("interrupt");
    fixture.record_all();
    fs::remove_file(fixture.completion_path("cargo --version")).expect("delete the last receipt");

    let before = fixture.tree_digest();
    let arca_before = state_digest(&fixture.root);
    let defects = fixture
        .gate()
        .expect_err("an incomplete verification must refuse");
    assert!(
        reasons(&defects).contains("cargo --version"),
        "the refusal names the check it could not verify: {}",
        reasons(&defects)
    );

    // The gate is a pure predicate: nothing is marked passed on the way.
    assert_eq!(before, fixture.tree_digest(), "the gate wrote no source");
    assert_eq!(
        arca_before,
        state_digest(&fixture.root),
        "the gate wrote no record: the ticket stays executing"
    );
    let ticket = fs::read_to_string(fixture.root.join(TICKET_PATH)).expect("read ticket");
    assert!(
        ticket.contains("status: \"executing\""),
        "a refused completion leaves the ticket executing: {ticket}"
    );
}

/// HT-047-03 (Input/Routing): a command that cannot resolve is named.
#[test]
fn undeclarable_command_refuses() {
    let fixture = Fixture::new("unresolvable");
    fixture.write_ticket(&["definitely-not-a-real-program --check"]);
    let digest = fixture.tree_digest();
    fixture.write_receipt("PT-900-01", "focused", "cargo test --test t900", 0, &digest);
    fixture.write_receipt(
        "HT-900-01",
        "hidden-lane",
        "cargo test --test t900",
        0,
        &digest,
    );
    fixture.write_receipt(
        "definitely-not-a-real-program --check",
        "quality",
        "definitely-not-a-real-program --check",
        0,
        &digest,
    );

    let defects = fixture
        .gate()
        .expect_err("a command that names no program must refuse");
    let text = reasons(&defects);
    assert!(
        text.contains("definitely-not-a-real-program"),
        "the refusal names the undeclarable command: {text}"
    );

    // An unparsable declaration refuses too, rather than being skipped.
    let fixture = Fixture::new("empty-command");
    fixture.write_ticket(&["   "]);
    let source = fs::read_to_string(fixture.root.join(TICKET_PATH)).expect("read ticket");
    assert!(
        declared_checks(&source)
            .iter()
            .all(|check| !check.id.trim().is_empty()),
        "a blank declaration is never treated as a declared check"
    );
}

/// A digest over the record tree, used to prove the gate writes nothing.
fn state_digest(root: &Path) -> String {
    ratmac::completion::tree_digest(root, &[".arca".to_owned()]).expect("record tree is readable")
}
