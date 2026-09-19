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

//! t-112 capability window (temporary, removed at the post-rest
//! activation): the traversal below is the prepared-runbook proof. It walks
//! the tracked runbook bytes with the tracked `.ratmac/completion-guard.diff`
//! mapping applied inside a throwaway fixture - never the live runbook,
//! which stays unmapped while stable edition-007 drives run-030 to rest.
//!
//! The activation landing applies that mapping to the tracked runbook,
//! deletes every block marked `t-112 capability window`, and restores the
//! exact shipped-byte traversal.

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

// --- t-112 capability window (temporary) ------------------------------------
//
// Everything in this region exists only while the declared-completion
// cutover is staged beside the live runbook. The post-rest activation
// landing deletes this region and the proofs at the bottom of this file,
// and points the traversal back at `shipped_runbook()`.

/// A throwaway fixture carrying the tracked runbook bytes and a copy of the
/// tracked completion-guard patch. Both are copied in - never referenced by
/// absolute path - because Git cannot open Windows verbatim (`\\?\\`) paths
/// from the canonicalized repository root. The `ratmac-t092-cutover-` prefix
/// marks every fixture tree this window creates, so a crashed run leaves
/// nothing unrecognizable behind.
fn stage_cutover_fixture(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ratmac-t092-cutover-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".ratmac")).expect("create cutover fixture tree");
    fs::write(root.join(".ratmac/ratmac.toml"), shipped_runbook())
        .expect("copy the tracked runbook bytes");
    fs::copy(
        repo_root().join(".ratmac/completion-guard.diff"),
        root.join("completion-guard.diff"),
    )
    .expect("stage the tracked cutover patch in the fixture");
    // Line-ending translation would make the recovered bytes differ by
    // platform accident rather than by what the patch changed.
    git_in(&root, &["init", "--quiet"]);
    git_in(&root, &["config", "core.autocrlf", "false"]);
    root
}

/// Git in a fixture that is not a `Cycle`, applying from the fixture's own
/// tree so no Windows verbatim path ever reaches an argument.
fn git_in(root: &std::path::Path, args: &[&str]) -> Output {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("invoke git");
    assert!(
        output.status.success(),
        "git {args:?} succeeds: {}",
        combined(&output)
    );
    output
}

/// The prepared cutover runbook: the tracked bytes with the staged mapping
/// applied by Git itself inside a throwaway fixture, so the traversal proves
/// exactly the patch the activation landing will apply to the live runbook
/// after rest - no inlined expectation, no fallback. The fixture is removed
/// once the bytes are read back.
fn prepared_runbook() -> String {
    let fixture = stage_cutover_fixture("runbook");
    git_in(&fixture, &["apply", "completion-guard.diff"]);
    let patched = fs::read_to_string(fixture.join(".ratmac/ratmac.toml"))
        .expect("read the prepared cutover runbook");
    let _ = fs::remove_dir_all(&fixture);
    patched
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
    /// The Machine Class the fixture carries: the prepared cutover for the
    /// traversal, or the exact shipped bytes for the unmapped-boundary proof
    /// (`t-112 capability window`, temporary).
    fn create(label: &str, runbook: &str) -> Self {
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
        cycle.write(".ratmac/ratmac.toml", runbook);
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

    /// The work item P3 cuts for that gap. During the `t-112 capability
    /// window` (temporary) it also carries the three declaration lists the
    /// prepared mapping selects - `focused-tests`, `hidden-lanes`, and
    /// `quality-commands`, matching the receipt ids `write_completion`
    /// already records - while the legacy `planned-test-refs` list keeps
    /// driving the sensitivity gate.
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
             planned-test-refs:\n  - \"PT-100-01\"\n\
             focused-tests:\n  - \"PT-100-01\"\n\
             hidden-lanes:\n  - \"HT-100-01\"\n\
             quality-commands:\n  - \"cargo --version\"\n\
             dependencies: []\n\
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
    // `t-112 capability window` (temporary): the traversal runs the prepared
    // cutover - the tracked bytes plus the staged mapping - not the exact
    // shipped runbook, which stays unmapped until the post-rest activation.
    let runbook = prepared_runbook();
    let class = MachineClass::from_toml(&runbook)
        .expect("the prepared cutover machine class parses through the one reader");

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

    let cycle = Cycle::create("traversal", &runbook);
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

    // `t-112 capability window` (temporary): the mapped fixture declares
    // exactly three non-empty checks - one focused, one hidden lane, one
    // quality command - and the gate refuses while any of them lacks its
    // receipt, naming the missing check. With no receipts at all the
    // refusal names all three in declaration order.
    let bare = cycle.rtm(&["step", "--run", child.as_str()]);
    let bare_text = combined(&bare);
    assert!(
        bare_text.contains("step refused"),
        "the completion gate refuses a turn with no receipts: {bare_text}"
    );
    assert!(
        bare_text.contains("no completion receipt"),
        "the refusal is the missing-receipt refusal: {bare_text}"
    );
    let positions: Vec<usize> = ["PT-100-01", "HT-100-01", "cargo --version"]
        .iter()
        .filter_map(|check| bare_text.find(check))
        .collect();
    assert_eq!(
        positions.len(),
        3,
        "the refusal names every declared check: {bare_text}"
    );
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "the declared checks are walked focused, hidden, then quality: {bare_text}"
    );
    assert_eq!(
        cycle.state(&child),
        "implement",
        "a refused step leaves the turn executing"
    );

    // One receipt short of the exact set: the refusal names that check.
    cycle.write_completion(&child);
    fs::remove_file(cycle.root.join(format!(
        ".ratmac/evidence/{child}/completion/{}.toml",
        ratmac::completion::check_slug("HT-100-01")
    )))
    .expect("remove the hidden-lane receipt");
    let hidden_refusal = cycle.rtm(&["step", "--run", child.as_str()]);
    let hidden_text = combined(&hidden_refusal);
    assert!(
        hidden_text.contains("step refused")
            && hidden_text.contains("no completion receipt")
            && hidden_text.contains("HT-100-01"),
        "the gate refuses the missing hidden-lane receipt by name: {hidden_text}"
    );

    cycle.write_completion(&child);
    fs::remove_file(cycle.root.join(format!(
        ".ratmac/evidence/{child}/completion/{}.toml",
        ratmac::completion::check_slug("cargo --version")
    )))
    .expect("remove the quality receipt");
    let quality_refusal = cycle.rtm(&["step", "--run", child.as_str()]);
    let quality_text = combined(&quality_refusal);
    assert!(
        quality_text.contains("step refused")
            && quality_text.contains("no completion receipt")
            && quality_text.contains("cargo --version"),
        "the gate refuses the missing quality receipt by name: {quality_text}"
    );

    // The exact declared-check receipt set - one receipt per declared check,
    // nothing more - is what passes the boundary.
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
    // `LNR-003`: the wired close guard reads the lane sweep's verdict, so the
    // fixture carries the sweep tool, a one-crate roster, and a genuinely
    // swept report in which every rostered lane passes.
    fs::create_dir_all(cycle.root.join("tools")).expect("create fixture tools");
    fs::copy(
        repo_root().join("tools/sweep_lanes.py"),
        cycle.root.join("tools/sweep_lanes.py"),
    )
    .expect("ship the sweep beside the fixture runbook");
    cycle.write(
        ".ratmac/lanes.toml",
        "[roots]\nlanes = \"test-hidden\"\nreport = \".ratmac/evidence/lane-sweep/report.md\"\nroster = [\"t-900\"]\n",
    );
    cycle.write(
        "test-hidden/t-900/Cargo.toml",
        "[package]\nname = \"t900-hidden\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    cycle.write("test-hidden/t-900/src/main.rs", "fn main() {}\n");
    cycle.write(
        "test-hidden/t-900/tests/hidden.rs",
        "#[test]\nfn ht_900_01_passes() {}\n",
    );
    let swept = Command::new("python")
        .arg(cycle.root.join("tools/sweep_lanes.py"))
        .arg("sweep")
        .current_dir(&cycle.root)
        .output()
        .expect("invoke the fixture sweep");
    assert!(
        swept.status.success(),
        "the fixture sweep passes its one rostered lane: {}",
        combined(&swept)
    );
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

// --- t-112 capability window (temporary) ------------------------------------
//
// The staged self-host rollout proofs. The activation landing deletes this
// region and the helper region near the top of this file, and restores the
// exact shipped-byte traversal.

/// The four-field mapping group the prepared cutover stages on each
/// completion guard, as authored key/value pairs.
const CUTOVER_MAPPING: [&str; 4] = [
    "declaration-format = \"front-matter-string-lists\"",
    "focused-field = \"focused-tests\"",
    "hidden-lane-field = \"hidden-lanes\"",
    "quality-field = \"quality-commands\"",
];

/// t-112 rollout condition (temporary): the tracked
/// `.ratmac/completion-guard.diff` is the exact prepared mapping. Applied to
/// the tracked runbook bytes inside a throwaway fixture it changes only the
/// two `completion_gate` guards - each gaining the whole four-field mapping
/// group - and reverse-applied it recovers the tracked bytes exactly, so the
/// roots, States, transitions, the edition guard, and the lane-sweep guard
/// survive the cutover untouched.
#[test]
fn the_prepared_completion_mapping_patches_only_the_two_completion_gates() {
    let diff = fs::read_to_string(repo_root().join(".ratmac/completion-guard.diff"))
        .expect("the prepared completion-guard diff lands as a tracked file");
    for pair in CUTOVER_MAPPING {
        assert!(
            diff.lines()
                .any(|line| line.starts_with('+') && line.contains(pair)),
            "the prepared diff adds {pair} to a guard: {diff}"
        );
    }
    let tracked = shipped_runbook();

    let fixture = stage_cutover_fixture("apply");
    git_in(&fixture, &["apply", "completion-guard.diff"]);
    let patched =
        fs::read_to_string(fixture.join(".ratmac/ratmac.toml")).expect("read the patched runbook");

    // Structure: the mapping rewrites the two completion-guard lines in
    // place and touches nothing else.
    let before: Vec<&str> = tracked.lines().collect();
    let after: Vec<&str> = patched.lines().collect();
    assert_eq!(after.len(), before.len(), "the mapping adds no line");
    let changed: Vec<usize> = before
        .iter()
        .zip(&after)
        .enumerate()
        .filter_map(|(index, (was, now))| (was != now).then_some(index))
        .collect();
    assert_eq!(
        changed.len(),
        2,
        "exactly the two completion guards change, in place: {patched}"
    );
    for index in changed {
        assert!(
            before[index].contains("{ kind = \"completion_gate\""),
            "the replaced line is a completion guard: {}",
            before[index]
        );
        for pair in CUTOVER_MAPPING {
            assert!(
                after[index].contains(pair),
                "the guard gains {pair}: {}",
                after[index]
            );
        }
    }
    assert_eq!(
        patched.matches("kind = \"completion_gate\"").count(),
        tracked.matches("kind = \"completion_gate\"").count(),
        "the runbook still carries exactly two completion guards"
    );
    assert_eq!(
        patched.matches("kind = \"command_exit\"").count(),
        tracked.matches("kind = \"command_exit\"").count(),
        "the edition, checkpoint, and lane-sweep guards are untouched"
    );
    for key in [
        "declaration-format",
        "focused-field",
        "hidden-lane-field",
        "quality-field",
    ] {
        assert_eq!(
            patched.matches(key).count(),
            2,
            "{key} appears on both completion guards only"
        );
        assert_eq!(
            tracked.matches(key).count(),
            0,
            "the tracked runbook is unmapped"
        );
    }

    // Reverse: the staged patch comes back off the tracked bytes exactly.
    git_in(&fixture, &["apply", "-R", "completion-guard.diff"]);
    let recovered = fs::read_to_string(fixture.join(".ratmac/ratmac.toml"))
        .expect("read the recovered runbook");
    assert_eq!(
        recovered, tracked,
        "reverse-applying the prepared mapping recovers the tracked bytes exactly"
    );
    let _ = fs::remove_dir_all(&fixture);
}

/// t-112 rollout condition (temporary): while the tracked runbook stays
/// unmapped, the candidate Engine refuses its completion boundary before it
/// reads the addressed item, naming the missing mapping fields - and the
/// doctor adds no missing-mapping lint, because an unmapped runbook is
/// statically valid. The activation landing deletes this proof together
/// with the mapping it proves staged.
#[test]
fn the_unmapped_completion_boundary_refuses_before_reading_the_item() {
    // The real tracked runbook is still unmapped: the live file carries
    // none of the mapping the prepared diff stages.
    let tracked = shipped_runbook();
    for key in [
        "front-matter-string-lists",
        "focused-field",
        "hidden-lane-field",
        "quality-field",
    ] {
        assert!(
            !tracked.contains(key),
            "the tracked runbook remains unmapped until the activation landing: no {key}"
        );
    }
    // No doctor lint for the absent mapping: old runbooks are statically
    // valid, so the shipped file carries no finding at all.
    let findings = doctor::diagnose(&repo_root().join(".ratmac/ratmac.toml"));
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
        "an unmapped runbook doctors clean - no missing-mapping lint: {shown:?}"
    );

    // Drive the exact shipped (unmapped) runbook to the implement boundary
    // of one ticket turn.
    let cycle = Cycle::create("unmapped", &shipped_runbook());
    let run = cycle.start();
    cycle.step(&run);
    let frozen = cycle.frozen(&run);
    cycle.write_gap("missing", &frozen, &[]);
    cycle.write_verdict(&run, "gap-check", "gaps");
    cycle.step(&run);
    cycle.write(".arca/ticket/t-100.md", &cycle.ticket_body());
    cycle.step(&run);
    let child = cycle.spawn("turn", &run, "item=t-100.md");
    cycle.write_sensitivity(&child, "PT-100-01");
    cycle.step(&child);
    assert_eq!(cycle.state(&child), "implement");

    // The addressed item is made unreadable and the tree committed clean,
    // so the only boundary that can refuse is the completion gate. A
    // mapped candidate would have to read the item to judge it; an unmapped
    // one refuses first, naming the mapping it lacks.
    fs::remove_file(cycle.root.join(".arca/ticket/t-100.md")).expect("remove the addressed item");
    cycle.commit("remove the addressed item");
    let refused = cycle.rtm(&["step", "--run", child.as_str()]);
    let text = combined(&refused);
    assert!(
        text.contains("step refused"),
        "the unmapped completion boundary refuses: {text}"
    );
    assert!(
        text.contains("completion_gate"),
        "the refusal is the completion gate's: {text}"
    );
    for key in [
        "declaration-format",
        "focused-field",
        "hidden-lane-field",
        "quality-field",
    ] {
        assert!(
            text.contains(key),
            "the refusal names the missing mapping field {key}: {text}"
        );
    }
    assert!(
        !text.contains("unreadable ticket"),
        "the refusal precedes any reading of the addressed item: {text}"
    );
    assert_eq!(
        cycle.state(&child),
        "implement",
        "the refused turn stays at the implement boundary"
    );
    assert!(
        !cycle
            .root
            .join(format!(".ratmac/evidence/{child}/completion"))
            .exists(),
        "the refusal creates no completion evidence"
    );
}

fn severity_word(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}
