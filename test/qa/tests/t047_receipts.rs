//! t-045 / PGE-003, PGE-004: sensitivity receipts and evidence ownership.
//!
//! PT-045-01 `sensitivity_receipt_required`
//! PT-045-02 `digest_binds_receipt_to_output`
//! PT-045-03 `ownership_audit_is_sensitive`
//! HT-045-01 `truncated_receipt_is_rejected`
//! HT-045-02 `no_scheduler_owned_path_in_any_instruction`
//! HT-045-03 `unknown_planned_test_id_refuses`
//!
//! `.ratmac/evidence/<run-id>/` is agent-writable and holds one structured
//! receipt per executed check. The P4 gate reads receipts, never prose; prompts
//! and gate contracts never hand an agent a Scheduler-owned file.

use ratmac::machine::MachineClass;
use ratmac::ownership::{
    audit_ownership, runbook_instructions, template_instructions, Instruction, SCHEDULER_OWNED,
};
use ratmac::receipt::{sha256_text, EVIDENCE_DIR};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Fixture {
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    /// A project whose single transition is gated on t-900's receipts.
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t047-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".arca/ticket")).expect("create fixture project");
        fs::create_dir_all(root.join(".ratmac")).expect("create Engine directory");
        fs::create_dir_all(root.join("test/qa/tests")).expect("create test tree");
        fs::write(
            root.join(".ratmac/ratmac.toml"),
            "[phases.build]\n\
             prompt = \"Write the test, then the code.\"\n\
             guards = [{ kind = \"sensitivity_receipts\", ticket = \".arca/ticket/t-900.md\" }]\n\
             \n\
             [phases.review]\n\
             prompt = \"Review the work.\"\n\
             \n\
             [[transitions]]\n\
             from = \"build\"\n\
             to = \"review\"\n",
        )
        .expect("write machine class");
        fs::write(
            root.join(".arca/ticket/t-900.md"),
            "---\n\
             ticket-id: t-900\n\
             planned-test-refs:\n\
             \x20 - \"PT-900-01\"\n\
             status: \"approved\"\n\
             ---\n\
             \n\
             # Ticket: t-900\n",
        )
        .expect("write ticket");
        fs::write(
            root.join("test/qa/tests/t900_example.rs"),
            "#[test]\nfn planned_behavior_is_checked() {\n    assert!(true);\n}\n",
        )
        .expect("write the planned test");
        Fixture { root }
    }

    fn start(&self) {
        assert!(self.rtm(&["start"]).status.success(), "start succeeds");
    }

    fn run_id(&self) -> String {
        fs::read_dir(self.root.join(".ratmac/runs"))
            .expect("list the runs roster")
            .map(|entry| entry.expect("roster entry is readable"))
            .find(|entry| entry.path().is_dir())
            .expect("the started run appears on the roster")
            .file_name()
            .to_string_lossy()
            .into_owned()
    }

    fn evidence_dir(&self) -> PathBuf {
        self.root.join(EVIDENCE_DIR).join(self.run_id())
    }

    /// Write a receipt whose digest is computed from its own output.
    fn write_receipt(&self, planned_test: &str, output: &str, exit_status: i64) {
        self.write_receipt_with_digest(planned_test, output, exit_status, &sha256_text(output));
    }

    fn write_receipt_with_digest(
        &self,
        planned_test: &str,
        output: &str,
        exit_status: i64,
        digest: &str,
    ) {
        let body = format!(
            "planned-test-id = \"{planned_test}\"\n\
             ticket-id = \"t-900\"\n\
             kind = \"baseline-failure\"\n\
             command = \"cargo test -p ratmac-qa --test t900_example\"\n\
             working-dir = \".\"\n\
             test-file = \"test/qa/tests/t900_example.rs\"\n\
             test-name = \"planned_behavior_is_checked\"\n\
             exit-status = {exit_status}\n\
             output-sha256 = \"{digest}\"\n\
             output = \"\"\"\n{output}\"\"\"\n"
        );
        let path = self.evidence_dir().join(format!("{planned_test}.toml"));
        fs::create_dir_all(path.parent().expect("receipt has an evidence directory"))
            .expect("create run-scoped evidence directory");
        fs::write(path, body).expect("write receipt");
    }

    fn receipt_path(&self, planned_test: &str) -> PathBuf {
        self.evidence_dir().join(format!("{planned_test}.toml"))
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    fn step_text(&self) -> String {
        // FDC-004: run addressing is always required.
        let id = self.run_id();
        let step = self.rtm(&["step", "--run", &id]);
        format!(
            "{}{}",
            String::from_utf8_lossy(&step.stdout),
            String::from_utf8_lossy(&step.stderr)
        )
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root resolves")
}

fn typed_runbook_instructions(path: &Path) -> Vec<Instruction> {
    let source = fs::read_to_string(path).expect("read Runbook");
    let class = MachineClass::from_toml(&source).expect("parse Runbook");
    let shown = path.to_string_lossy().replace('\\', "/");
    runbook_instructions(&class, &shown)
}

const BASELINE_OUTPUT: &str = "test planned_behavior_is_checked ... FAILED\n\
                               test result: FAILED. 0 passed; 1 failed\n";

/// PT-045-01: a receipt passes the gate; prose in its place does not.
#[test]
fn sensitivity_receipt_required() {
    let fixture = Fixture::new("required");
    fixture.start();
    fixture.write_receipt("PT-900-01", BASELINE_OUTPUT, 101);
    let accepted = fixture.step_text();
    assert!(
        !accepted.contains("step refused"),
        "a planned test with a baseline-failure receipt passes: {accepted}"
    );

    // Same fixture, receipt replaced by the prose line the old loop wrote.
    let fixture = Fixture::new("prose");
    fixture.start();
    let evidence_dir = fixture.evidence_dir();
    fs::create_dir_all(&evidence_dir).expect("create run-scoped evidence directory");
    fs::write(
        evidence_dir.join("notes.md"),
        "- PT-900-01 failed before implementation, honest.\n",
    )
    .expect("write prose");
    let refusal = fixture.step_text();
    assert!(
        refusal.contains("step refused") && refusal.contains("PT-900-01"),
        "the refusal identifies the receiptless planned test: {refusal}"
    );
    assert!(
        refusal.contains("prose and file names are not evidence"),
        "the refusal says what would count: {refusal}"
    );

    // A file named like the planned test is not a receipt either.
    fs::write(
        fixture.evidence_dir().join("PT-900-01.baseline-failure.md"),
        "PT-900-01 baseline failure\n",
    )
    .expect("write filename-convention evidence");
    let still_refused = fixture.step_text();
    assert!(
        still_refused.contains("step refused") && still_refused.contains("PT-900-01"),
        "a filename convention is not a receipt: {still_refused}"
    );
}

/// PT-045-02: the digest binds the receipt to the output it claims.
#[test]
fn digest_binds_receipt_to_output() {
    let fixture = Fixture::new("digest");
    fixture.start();
    fixture.write_receipt_with_digest("PT-900-01", BASELINE_OUTPUT, 101, &"0".repeat(64));
    let refusal = fixture.step_text();
    assert!(
        refusal.contains("step refused") && refusal.contains("digest does not re-derive"),
        "a mismatched digest is refused: {refusal}"
    );
    assert!(
        refusal.contains(&sha256_text(BASELINE_OUTPUT)[..16]),
        "the refusal names the digest it observed: {refusal}"
    );

    // Editing the output after the fact breaks the same binding.
    let fixture = Fixture::new("edited");
    fixture.start();
    fixture.write_receipt("PT-900-01", BASELINE_OUTPUT, 101);
    let path = fixture.receipt_path("PT-900-01");
    let source = fs::read_to_string(&path).expect("read receipt");
    fs::write(
        &path,
        source.replace("0 passed; 1 failed", "1 passed; 0 failed"),
    )
    .expect("edit the recorded output");
    let refusal = fixture.step_text();
    assert!(
        refusal.contains("digest does not re-derive"),
        "an edited output no longer matches its digest: {refusal}"
    );

    // A receipt that records a passing run proves no sensitivity at all.
    let fixture = Fixture::new("passing");
    fixture.start();
    fixture.write_receipt("PT-900-01", "test result: ok. 1 passed\n", 0);
    let refusal = fixture.step_text();
    assert!(
        refusal.contains("proves no sensitivity"),
        "a passing run is not a sensitivity receipt: {refusal}"
    );
}

/// PT-045-03: no active prompt or gate contract hands an agent a
/// Scheduler-owned file, and the audit notices when one does.
#[test]
fn ownership_audit_is_sensitive() {
    let root = repo_root();
    let mut instructions = typed_runbook_instructions(&root.join(".ratmac/ratmac.toml"));
    instructions.extend(template_instructions(&root.join(".arca/tpl")));
    assert!(
        !instructions.is_empty(),
        "the audit must actually read the active instruction set"
    );
    if let Err(violations) = audit_ownership(&instructions) {
        panic!("active instructions must not assign Scheduler-owned writes: {violations:?}");
    }

    for owned in SCHEDULER_OWNED {
        let violating = Instruction {
            source: format!("fixture prompt for {owned}"),
            text: format!("Complete the phase and append your result to {owned}."),
        };
        let mut seeded = instructions.clone();
        seeded.push(violating);
        let violations = audit_ownership(&seeded)
            .expect_err("a prompt that assigns a Scheduler-owned write must fail the audit");
        assert!(
            violations.iter().any(|violation| violation.path == owned),
            "the audit names the Scheduler-owned path: {violations:?}"
        );
    }

    // A gate contract pointing at a Scheduler-owned path is the same defect.
    let fixture = Fixture::new("ownership");
    fs::write(
        fixture.root.join(".ratmac/ratmac.toml"),
        "[phases.build]\n\
         prompt = \"Do the work.\"\n\
         guards = [{ kind = \"file_contains\", path = \".ratmac/runs/run-1/state.toml\", contains = \"passed\" }]\n\
         \n\
         [phases.review]\n\
         prompt = \"Review.\"\n\
         \n\
         [[transitions]]\n\
         from = \"build\"\n\
         to = \"review\"\n",
    )
    .expect("write violating class");
    let violations = audit_ownership(&typed_runbook_instructions(
        &fixture.root.join(".ratmac/ratmac.toml"),
    ))
    .expect_err("a guard contract on an Engine-owned path must fail the audit");
    assert!(
        violations
            .iter()
            .any(|violation| violation.path == ".ratmac/runs/<id>/state.toml"),
        "the audit names the canonical Engine-owned path: {violations:?}"
    );

    // Prose that merely mentions the file is not an instruction to write it.
    let mention = Instruction {
        source: "fixture note".to_owned(),
        text: "The Scheduler owns .ratmac/state.toml; read it, never touch it. Write your notes to .ratmac/evidence/.".to_owned(),
    };
    assert!(
        audit_ownership(std::slice::from_ref(&mention)).is_ok(),
        "a read-only mention is not a violation"
    );
}

/// HT-045-01 (Durability/Recovery): a receipt truncated mid-write is not proof.
#[test]
fn truncated_receipt_is_rejected() {
    let fixture = Fixture::new("truncated");
    fixture.start();
    fixture.write_receipt("PT-900-01", BASELINE_OUTPUT, 101);
    let path = fixture.receipt_path("PT-900-01");
    let source = fs::read_to_string(&path).expect("read receipt");
    let cut = source.len() * 2 / 3;
    fs::write(&path, &source[..cut]).expect("truncate the receipt mid-write");

    let refusal = fixture.step_text();
    assert!(
        refusal.contains("step refused"),
        "a partial receipt must not pass the gate: {refusal}"
    );
    assert!(
        refusal.contains("not valid TOML") || refusal.contains("missing"),
        "the refusal says why the partial receipt is not proof: {refusal}"
    );
    assert!(
        refusal.contains("PT-900-01"),
        "the refusal still names the unproven planned test: {refusal}"
    );
}

/// HT-045-02 (Output/Filesystem): scan every Runbook and template in the
/// repository, not just the active project's.
#[test]
fn no_scheduler_owned_path_in_any_instruction() {
    let root = repo_root();
    let mut instructions = Vec::new();
    let mut runbooks = 0;
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if matches!(name.as_str(), "target" | ".git") {
                    continue;
                }
                stack.push(path);
            } else if name == "ratmac.toml" {
                // Fixtures under test/fixtures/ are inputs to negative tests;
                // the audit covers every Runbook that is not one of those.
                if path.to_string_lossy().replace('\\', "/").contains("/test/") {
                    continue;
                }
                runbooks += 1;
                let from_runbook = typed_runbook_instructions(&path);
                assert!(
                    !from_runbook.is_empty(),
                    "Runbook {} contributes no auditable prompt: it is unparseable or empty \
                     under the current Machine Class schema",
                    path.display()
                );
                instructions.extend(from_runbook);
            }
        }
    }
    instructions.extend(template_instructions(&root.join(".arca/tpl")));
    assert!(runbooks >= 1, "at least the project Runbook is audited");
    assert!(
        instructions.len() >= 8,
        "the scan must actually collect prompts and gate contracts: {} collected",
        instructions.len()
    );
    if let Err(violations) = audit_ownership(&instructions) {
        panic!(
            "no Runbook prompt or gate contract may assign a Scheduler-owned write: {violations:?}"
        );
    }
}

/// HT-045-03 (Input/Routing): a receipt for a planned test the ticket does not
/// declare is refused by name.
#[test]
fn unknown_planned_test_id_refuses() {
    let fixture = Fixture::new("unknown");
    fixture.start();
    fixture.write_receipt("PT-900-01", BASELINE_OUTPUT, 101);
    fixture.write_receipt("PT-999-99", BASELINE_OUTPUT, 101);
    let refusal = fixture.step_text();
    assert!(
        refusal.contains("step refused") && refusal.contains("PT-999-99"),
        "the refusal names the unresolvable planned-test ID: {refusal}"
    );
    assert!(
        refusal.contains("does not declare"),
        "the refusal says why the ID is unresolvable: {refusal}"
    );
}
