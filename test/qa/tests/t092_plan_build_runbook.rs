//! t-092 / PCR-001, PCR-005: the shipped Machine Class is this repository's
//! own cycle.
//!
//! PCRV-001 `the_cycle_runs_from_intake_to_rest`
//! PCRV-004 `the_doctor_is_clean_on_the_shipped_machine_class`
//!
//! The engine stops demonstrating a build and starts running the P1-P5 cycle
//! this repository follows. The shipped file declares the stages, their
//! prompts, and the Exit Guards between them; a Run started on a seeded copy
//! of this repository reaches the terminal rest State by starting, spawning,
//! and stepping alone, with no rule supplied from outside the file.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use ratmac::doctor::{self, Severity};
use ratmac::machine::MachineClass;
use ratmac::receipt::sha256_text;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The Machine Class this repository ships, as it stands on disk.
fn shipped_runbook() -> String {
    fs::read_to_string(repo_root().join(".ratmac/ratmac.toml"))
        .expect("read the shipped machine class")
}

/// The green output a receipt records for a check that passed.
const GREEN: &str = "test result: ok. 1 passed; 0 failed\n";

/// The red output a sensitivity receipt records for a planned test that
/// failed before its implementation existed.
const RED: &str = "test result: FAILED. 0 passed; 1 failed\n";

/// A temporary repository seeded with the artifacts each stage's guards read,
/// carrying the shipped runbook itself.
struct Cycle {
    root: PathBuf,
}

impl Drop for Cycle {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Cycle {
    fn create(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t092-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in [
            ".arca/goal",
            ".arca/issue/i-100-demo",
            ".arca/residual",
            ".arca/ticket/archive",
            ".ratmac",
            "src",
            "test",
        ] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        let cycle = Self { root };
        cycle.write(".ratmac/ratmac.toml", &shipped_runbook());
        cycle.write(".gitignore", ".ratmac/\n");
        cycle.write("src/lib.rs", "pub fn work() {}\n");
        cycle.write("test/fixture_test.rs", "fn the_planned_test() {}\n");
        cycle.write(
            ".arca/schema.md",
            "# Working rules\n\n### AUTH-001 - the contributor rule\n\nProse.\n",
        );
        cycle.write(
            ".arca/goal/spec.md",
            "# Goal spec\n\n\
             | Req ID | Requirement | Source |\n|---|---|---|\n\
             | DEMO-001 | The demo behaves. | \
             [issue DEMO-001](../issue/i-100-demo/spec.md#requirement-records) |\n",
        );
        cycle.write_issue();
        cycle.git(&["init", "--quiet"]);
        cycle.git(&["config", "user.email", "fixture@example.invalid"]);
        cycle.git(&["config", "user.name", "Fixture"]);
        // Line-ending translation would make the checkpoint guard name files
        // by platform accident rather than by what the stage changed.
        cycle.git(&["config", "core.autocrlf", "false"]);
        cycle.commit("seed the cycle fixture");
        cycle
    }

    fn write(&self, relative: &str, body: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture directory");
        }
        fs::write(path, body).expect("write fixture file");
    }

    /// One integrated issue whose single ask resolves to the goal row.
    fn write_issue(&self) {
        let dir = self.root.join(".arca/issue/i-100-demo");
        fs::create_dir_all(&dir).expect("create issue folder");
        fs::write(
            dir.join("index.md"),
            "# Issue i-100-demo\n\n\
             ```yaml\nissue-id: \"i-100-demo\"\nstatus: \"integrated\"\n```\n\n\
             See [goal spec](../../goal/spec.md).\n",
        )
        .expect("write issue index");
        fs::write(
            dir.join("spec.md"),
            "# Requirement records\n\n\
             | Req ID | Requirement | Status |\n|---|---|---|\n\
             | `DEMO-001` | The demo behaves. | accepted |\n",
        )
        .expect("write issue spec");
        for name in ["design.md", "test-plan.md", "ubi-lang.md"] {
            fs::write(dir.join(name), format!("# {name}\n")).expect("write issue file");
        }
    }

    /// The gap record P2 writes, citing the revision the freeze recorded.
    fn write_gap(&self, status: &str, frozen: &str, evidence: &[&str]) {
        let refs: String = evidence
            .iter()
            .map(|entry| format!("  - \"{entry}\"\n"))
            .collect();
        self.write(
            ".arca/residual/res-100.md",
            &format!(
                "# Residual Record\n\n```yaml\n\
                 residual-id: \"res-100\"\n\
                 goal-requirement-ref: \"DEMO-001\"\n\
                 frozen-goal-bundle-revision: \"goal-sha256:{frozen}\"\n\
                 concrete-evidence-refs:\n{refs}\
                 status: \"{status}\"\n```\n"
            ),
        );
    }

    /// The work item P3 cuts for that gap.
    fn ticket_body(&self) -> String {
        let lanes = [
            "Regression",
            "Input/Routing",
            "Lifecycle/Model",
            "Durability/Recovery",
            "Output/Filesystem",
            "Cross-Feature",
        ]
        .iter()
        .map(|lane| format!("| `{lane}` | `covered` | Reason. | `HT-100-01` |\n"))
        .collect::<String>();
        format!(
            "---\nticket-id: \"t-100\"\nresidual-ids:\n  - \"res-100\"\n\
             planned-test-refs:\n  - \"PT-100-01\"\ndependencies: []\n\
             status: \"approved\"\n---\n\n\
             # Ticket: t-100\n\n## Vertical Outcome\n\nOutcome.\n\n\
             ## Worktree Scope\n\nScope.\n\n\
             ## P4 Apparent Test Plan\n\n| Apparent Test ID |\n|---|\n| `PT-100-01` |\n\n\
             ## P5 Hidden Test Public Coverage Manifest\n\n\
             | Lane | Assessment | Rationale | Hidden IDs |\n|---|---|---|---|\n{lanes}\n\
             ## Merge Gate\n\n- Quality: `cargo --version` passes.\n"
        )
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

    /// Advance one Run and require it to move.
    fn step(&self, run: &str) -> String {
        let output = self.rtm(&["step", "--run", run]);
        let text = combined(&output);
        assert!(
            output.status.success() && !text.contains("step refused"),
            "step of run {run} from state {:?} must succeed: {text}",
            self.state(run)
        );
        text
    }

    fn spawn(&self, name: &str, run: &str, bind: &str) -> String {
        let output = self.rtm(&["spawn", name, "--run", run, "--bind", bind]);
        let text = combined(&output);
        assert!(output.status.success(), "spawn succeeds: {text}");
        text.split("spawned run ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .expect("spawn names the child run id")
            .to_owned()
    }

    fn run_dir(&self, run: &str) -> PathBuf {
        self.root.join(format!(".ratmac/runs/{run}"))
    }

    fn record(&self, run: &str) -> String {
        fs::read_to_string(self.run_dir(run).join("run.toml")).expect("read the Run Record")
    }

    /// The State the named Run occupies, read from its Run Record.
    fn state(&self, run: &str) -> String {
        self.record(run)
            .lines()
            .find_map(|line| line.trim().strip_prefix("state = "))
            .unwrap_or_default()
            .trim_matches('"')
            .to_owned()
    }

    /// The goal revision the freeze recorded for this Run.
    fn frozen(&self, run: &str) -> String {
        let evidence =
            fs::read_to_string(self.run_dir(run).join("evidence.toml")).expect("read evidence");
        evidence
            .lines()
            .find_map(|line| line.trim().strip_prefix("frozen = "))
            .expect("the freeze recorded a goal revision")
            .trim_matches('"')
            .to_owned()
    }

    /// The reviewer's transition-input record for a branching State.
    fn write_verdict(&self, run: &str, state: &str, input: &str) {
        fs::write(
            self.run_dir(run).join("verdict.toml"),
            format!(
                "state = \"{state}\"\ninput = \"{input}\"\n\
                 rationale = \"The reviewer read the tree and chose {input}.\"\n"
            ),
        )
        .expect("write the verdict record");
    }

    /// One sensitivity receipt proving a planned test fails before its code.
    fn write_sensitivity(&self, run: &str, planned: &str) {
        let dir = self.root.join(format!(".ratmac/evidence/{run}"));
        fs::create_dir_all(&dir).expect("create evidence directory");
        let body = format!(
            "planned-test-id = \"{planned}\"\n\
             ticket-id = \"t-100\"\n\
             kind = \"baseline-failure\"\n\
             command = \"cargo test --test fixture_test\"\n\
             working-dir = \".\"\n\
             test-file = \"test/fixture_test.rs\"\n\
             test-name = \"the_planned_test\"\n\
             exit-status = 101\n\
             output-sha256 = \"{}\"\n\
             output = \"\"\"\n{RED}\"\"\"\n",
            sha256_text(RED)
        );
        fs::write(dir.join(format!("{planned}.toml")), body).expect("write sensitivity receipt");
    }

    /// Every check the work item declares, recorded green and fresh.
    fn write_completion(&self, run: &str) {
        let dir = self.root.join(format!(".ratmac/evidence/{run}/completion"));
        fs::create_dir_all(&dir).expect("create completion directory");
        let digest = ratmac::completion::tree_digest(&self.root, &["src".to_owned()])
            .expect("source roots are readable");
        for (check, kind, command) in [
            ("PT-100-01", "focused", "cargo test --test fixture_test"),
            ("HT-100-01", "hidden-lane", "cargo test --test fixture_test"),
            ("cargo --version", "quality", "cargo --version"),
        ] {
            let body = format!(
                "ticket-id = \"t-100\"\n\
                 check-id = \"{check}\"\n\
                 kind = \"{kind}\"\n\
                 command = \"{command}\"\n\
                 working-dir = \".\"\n\
                 exit-status = 0\n\
                 output-sha256 = \"{}\"\n\
                 tree-roots = [\"src\"]\n\
                 tree-sha256 = \"{digest}\"\n\
                 output = \"\"\"\n{GREEN}\"\"\"\n",
                sha256_text(GREEN)
            );
            fs::write(
                dir.join(format!("{}.toml", ratmac::completion::check_slug(check))),
                body,
            )
            .expect("write completion receipt");
        }
    }
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// PCRV-001: the shipped runbook is the cycle, and a Run walks it end to end.
#[test]
fn the_cycle_runs_from_intake_to_rest() {
    let class = MachineClass::from_toml(&shipped_runbook())
        .expect("the shipped machine class parses through the one reader");

    let declared: Vec<&str> = class.states().keys().map(String::as_str).collect();
    assert_eq!(
        declared,
        vec![
            "close",
            "cut-tickets",
            "gap-check",
            "intake",
            "rest",
            "ticket-turns"
        ],
        "PCR-001: the shipped States are the cycle's stages"
    );
    assert!(
        class.classes().contains_key("ticket"),
        "PCR-001: the ticket turns are a declared child class"
    );

    let cycle = Cycle::create("traversal");
    let run = cycle.start();
    assert_eq!(cycle.state(&run), "intake", "a Run starts at intake");

    // P1 -> P2: the intake gate passes and the edge freezes the goal.
    cycle.step(&run);
    assert_eq!(cycle.state(&run), "gap-check");
    let frozen = cycle.frozen(&run);

    // P2: the gap record is written, and the reviewer routes on gaps.
    cycle.write_gap("missing", &frozen, &[]);
    cycle.write_verdict(&run, "gap-check", "gaps");
    cycle.step(&run);
    assert_eq!(cycle.state(&run), "cut-tickets");

    // P3: the work item owns the gap.
    cycle.write(".arca/ticket/t-100.md", &cycle.ticket_body());
    cycle.step(&run);
    assert_eq!(cycle.state(&run), "ticket-turns");

    // P4/P5: one turn of the ticket class, bound to that item.
    let child = cycle.spawn("turn", &run, "item=t-100.md");
    assert_eq!(cycle.state(&child), "tests");
    cycle.write_sensitivity(&child, "PT-100-01");
    cycle.step(&child);
    assert_eq!(cycle.state(&child), "implement");

    cycle.write_completion(&child);
    cycle.commit("the green landing");
    cycle.step(&child);
    assert_eq!(
        cycle.state(&child),
        "damage",
        "the checkpoint guard passes on a committed tree"
    );
    cycle.step(&child);
    assert_eq!(cycle.state(&child), "done");
    assert!(
        cycle.record(&child).contains("passed"),
        "entering the child's terminal State passes the turn"
    );

    // The join sees a passed child, so the turn stage can be left.
    cycle.step(&run);
    assert_eq!(cycle.state(&run), "close");

    // Close: the gap is proven and the item takes the archive move.
    cycle.write_gap("satisfied", &frozen, &["src/lib.rs"]);
    fs::rename(
        cycle.root.join(".arca/ticket/t-100.md"),
        cycle.root.join(".arca/ticket/archive/t-100.md"),
    )
    .expect("archive the finished item");
    // `EDN-002`: the closing State may not be left unmarked. The traversal ends
    // its turn on an edition, exactly as this repository's own sprints do.
    cycle.commit("the turn's green landing");
    cycle.git(&[
        "tag",
        "-a",
        "edition-001",
        "-m",
        "fixture edition: every gate green",
    ]);
    cycle.step(&run);
    assert_eq!(cycle.state(&run), "gap-check");

    // The second gap check finds nothing open, so the cycle comes to rest.
    cycle.write_verdict(&run, "gap-check", "clean");
    cycle.step(&run);
    assert_eq!(cycle.state(&run), "rest");
    assert!(
        cycle.record(&run).contains("passed"),
        "rest is terminal, so arriving there completes the Run"
    );
}

/// PCRV-004: the shipped runbook is clean under its own doctor.
#[test]
fn the_doctor_is_clean_on_the_shipped_machine_class() {
    let path = repo_root().join(".ratmac/ratmac.toml");
    let findings = doctor::diagnose(&path);
    let shown: Vec<String> = findings
        .iter()
        .map(|finding| {
            format!(
                "{} {} {} {}",
                finding.code(),
                severity_word(finding.severity()),
                finding.location(),
                finding.message()
            )
        })
        .collect();
    assert!(
        findings.is_empty(),
        "PCR-005: the shipped runbook carries no finding at all: {shown:?}"
    );
    assert_eq!(
        doctor::exit_code(&findings),
        0,
        "PCRV-004: rtm doctor exits 0 on the shipped machine class"
    );

    let class = MachineClass::from_toml(&shipped_runbook()).expect("the shipped class parses");
    let instructions = ratmac::ownership::runbook_instructions(&class, ".ratmac/ratmac.toml");
    assert!(
        ratmac::ownership::audit_ownership(&instructions).is_ok(),
        "PCRV-004: the prompt-and-contract ownership audit returns no violation"
    );
}

fn severity_word(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}
