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
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ratmac::doctor::{self, Severity};
use ratmac::machine::MachineClass;
use ratmac::receipt::sha256_text;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The recorded `.ratmac/ratmac.toml` bytes of this batch's merge base -
/// the base the three tracked boundary diffs apply over. Embedded (with its
/// sha256 proven on every read) so the tracked runbook is never read as the
/// edited class: t-133's contract keeps the tracked runbook at the base bytes
/// until the cycle boundary wires the diffs, and this suite must hold in
/// every state.
const BASE_RUNBOOK_SHA256: &str =
    "012eaa36c1a895f50d8822d516699029cb461a2d489b8ee2b882ce37a8b440c6";

/// The Machine Class this repository ships. t-133's accepted contract makes
/// `shipped_runbook()` "the recorded base bytes plus the three tracked
/// boundary diffs applied in ticket order" - t-130's
/// `.ratmac/pause-routes.diff`, then t-131's `.ratmac/review-proofs.diff`,
/// then t-133's `.ratmac/issue-binding.diff` - bytes that equal the tracked
/// runbook after wiring, so the traversal, the doctor-clean pin, and the
/// typed-parse and ownership reads hold in every state. Never the tracked
/// runbook read as the edited class.
const BASE_RUNBOOK: &str = r##"# The Plan-Build Runbook - the P1-P5 cycle this repository runs (PCR-001).
#
# Comments carry authoring intent and never reach an agent (R-012). Prompts are
# agent-facing prose; guards are the machine's own reading of what is on disk,
# never an agent's claim. No guard here rests on content the agent under test
# can write (PCR-005), so the file declares no file-shaped guard over a
# workflow root, and no prompt hands an agent a Scheduler-owned job (PGE-004).

[roots]
goal = ".arca/goal"
issue = ".arca/issue"
residual = ".arca/residual"
ticket = ".arca/ticket"
# PCR-008: the working authority. Only the markdown directly under this root is
# read, so an accepted ask may resolve to a requirement-ID heading in the
# working rules instead of a product row in the goal.
authority = ".arca"

# --- P1 ----------------------------------------------------------------------

[states.intake]
prompt = """
Fold every pending issue in the intake area into the frozen goal or the working
rules. Give each ask a stable requirement identifier and exactly one
disposition - accepted, rejected, duplicate, or deferred - and link it both
ways. An accepted ask resolves either to a product requirement row in the goal
specification or to an explicit requirement-identifier heading in the working
rules. Leave no ask undecided and no bundle half moved.
"""
guards = [{ kind = "intake_contract" }]

# --- P2 ----------------------------------------------------------------------

[states.gap-check]
prompt = """
Freeze the goal and measure it. Compare every frozen requirement against what
the tree actually holds, and state each answer as missing, partial, or
satisfied with a pointer to the proof and a short why. Then answer this stage's
question: are there gaps to work, or is the goal clean?
"""
inputs = ["gaps", "clean"]
# The gap check is left only while the intake reading it measures against still
# holds: every ask dispositioned, every bundle where the rules put it. The gap
# records themselves are judged one stage later, when they have owners - the
# record contract demands an owning work item for every unproven gap, which is
# exactly what P3 has not cut yet.
guards = [{ kind = "intake_contract" }]

# --- P3 ----------------------------------------------------------------------

[states.cut-tickets]
prompt = """
Cut one work item per open gap, each with its own vertical outcome, worktree
scope, apparent test plan, hidden coverage manifest, and merge gate. Every open
gap gets exactly one owner and no gap is owned twice. Order the items by their
dependencies and leave no cycle among them.
"""
guards = [{ kind = "record_contract" }]

# --- P4 and P5 ---------------------------------------------------------------

[states.ticket-turns]
prompt = """
Take the ordered work items one at a time. For each, begin one turn of the
ticket class bound to that item, and let the turn reach its end before the next
one begins. This stage is done when every turn it began has passed.
"""
guards = [
  { kind = "record_contract" },
  { kind = "join", require = "all_passed" },
]

[[states.ticket-turns.spawns]]
class = "ticket"
name = "turn"
bind = ["item"]

# --- the turn of the cycle ---------------------------------------------------

[states.close]
prompt = """
Close this turn of the cycle. Judge every gap the finished items claimed, move
each finished item to the archive, and leave the tree clean and green.
"""
guards = [
  { kind = "record_contract" },
  # EDN-002: a sprint may not reach rest unmarked. Exit 0 is an annotated tag
  # named edition-* pointing at the commit being left, so every sprint ends on
  # an edition and therefore every sprint starts from one. Read-only: the
  # Engine learns nothing about version control from it.
  { kind = "command_exit", program = "git", args = [
    "describe",
    "--exact-match",
    "--match",
    "edition-*",
  ], expected = 0 },
  # LNR-003 (t-108, i-037): the lanes this close leaves behind are runnable
  # or visibly expired. Exit 0 is one lane sweep report in which every
  # rostered crate reads pass or expired; the check refuses a red, unexpired
  # verdict, a crate the roster expects and the folder lacks, and a missing
  # or partial report. Read-only: the Engine learns nothing about lanes from
  # it. Prepared beside the runbook because a live Run pins the runbook
  # (FDC-005); wire it with `git apply .ratmac/close-guard.diff` at the
  # first cycle boundary where no Run is open.
  { kind = "command_exit", program = "python", args = [
    "tools/sweep_lanes.py",
    "check",
  ], expected = 0 },
]

[states.rest]
prompt = """
Nothing is open. The cycle is at rest until the next issue arrives.
"""

[[transitions]]
from = "intake"
to = "gap-check"
# ETB-003: intake is complete, so the goal the gap check measures is frozen.
freeze = "goal"

[[transitions]]
from = "gap-check"
to = "cut-tickets"
input = "gaps"

[[transitions]]
from = "gap-check"
to = "rest"
input = "clean"

[[transitions]]
from = "cut-tickets"
to = "ticket-turns"

[[transitions]]
from = "ticket-turns"
to = "close"

[[transitions]]
from = "close"
to = "gap-check"

# --- the ticket class: one turn, one work item -------------------------------

# PCR-007: the turn judges the item its caller bound, so the runbook carries no
# ticket identifier of its own and stays read-only.
[classes.ticket.bindings.item]
required = true

[classes.ticket.states.tests]
prompt = """
Write this item's planned tests first and prove each one red for the stated
reason. Every planned test the item declares needs a sensitivity receipt
showing the test runs and fails before its implementation exists.
"""
guards = [
  { kind = "sensitivity_receipts", ticket-binding = "item", root = "ticket" },
]

[classes.ticket.states.implement]
prompt = """
Make the planned tests pass, then run the item's whole merge gate: its focused
tests, the affected regressions, its hidden lanes, and the quality commands.
Commit the green work before going on - the next stage breaks the tree on
purpose and has to have something to restore.
"""
guards = [
  { kind = "completion_gate", ticket-binding = "item", root = "ticket", declaration-format = "front-matter-string-lists", focused-field = "focused-tests", hidden-lane-field = "hidden-lanes", quality-field = "quality-commands" },
  # PCR-009: the safety commit is a machine check, not prose. Exit 0 is a
  # working tree with no uncommitted change to a tracked file.
  { kind = "command_exit", program = "git", args = [
    "diff",
    "--stat",
    "--exit-code",
    "HEAD",
  ], expected = 0 },
]

[classes.ticket.states.damage]
prompt = """
Break the green work on purpose, one mutation at a time, and check that a test
catches each one. Restore the tree from the safety commit after every mutation.
A mutation that survives is a hole in the tests, not a win.
"""
# Leaving the damage stage means the checks are green and fresh again: the
# completion gate re-derives every receipt against the tree as it stands, so a
# tree left damaged cannot pass.
guards = [
  { kind = "completion_gate", ticket-binding = "item", root = "ticket", declaration-format = "front-matter-string-lists", focused-field = "focused-tests", hidden-lane-field = "hidden-lanes", quality-field = "quality-commands" },
]

[classes.ticket.states.done]
prompt = """
The turn is finished: the item's tests are green, its mutations are killed, and
its landing is merged.
"""

[[classes.ticket.transitions]]
from = "tests"
to = "implement"

[[classes.ticket.transitions]]
from = "implement"
to = "damage"

[[classes.ticket.transitions]]
from = "damage"
to = "done"
"##;

/// One throwaway directory removed on drop, so the runbook wiring fixture
/// never outlives the read that built it.
struct Throwaway(PathBuf);

impl Drop for Throwaway {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn wiring_git(root: &Path, args: &[&str]) -> Output {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("invoke git");
    assert!(
        output.status.success(),
        "git {args:?} succeeds in the runbook wiring fixture: {}",
        combined(&output)
    );
    output
}

/// The Machine Class this repository ships, as this batch wires it: the
/// recorded base bytes with the three tracked boundary diffs applied in
/// ticket order (t-133's regression adjustment; the diffs are read where the
/// repository tracks them, the runbook never).
fn shipped_runbook() -> String {
    assert_eq!(
        sha256_text(BASE_RUNBOOK),
        BASE_RUNBOOK_SHA256,
        "the embedded base copy is the recorded merge-base runbook"
    );
    let work = Throwaway(std::env::temp_dir().join(format!(
        "ratmac-t092-wiring-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    )));
    fs::create_dir_all(work.0.join(".ratmac")).expect("create the wiring fixture");
    fs::write(work.0.join(".ratmac/ratmac.toml"), BASE_RUNBOOK)
        .expect("write a clean copy of the base runbook bytes");
    // t-133's contract: the three tracked boundary diffs, applied in ticket
    // order - t-130's, then t-131's, then t-133's - over the base bytes.
    for diff in [
        "pause-routes.diff",
        "review-proofs.diff",
        "issue-binding.diff",
    ] {
        let bytes = fs::read(repo_root().join(".ratmac").join(diff)).unwrap_or_else(|error| {
            panic!("read the tracked boundary diff .ratmac/{diff}: {error}")
        });
        fs::write(work.0.join(".ratmac").join(diff), bytes)
            .expect("install the boundary diff beside the base runbook");
    }
    for diff in [
        ".ratmac/pause-routes.diff",
        ".ratmac/review-proofs.diff",
        ".ratmac/issue-binding.diff",
    ] {
        wiring_git(&work.0, &["apply", "--check", diff]);
        wiring_git(&work.0, &["apply", diff]);
    }
    fs::read_to_string(work.0.join(".ratmac/ratmac.toml")).expect("read the wired runbook bytes")
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
    /// The fixture carries the exact shipped Machine Class bytes.
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
            "tools",
            "src",
            "test",
        ] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        let cycle = Self { root };
        cycle.write(".ratmac/ratmac.toml", &shipped_runbook());
        // t-133's accepted contract: the fixture copies
        // `tools/verify_issue_binding.py` beside the runbook exactly as it
        // already ships `tools/sweep_lanes.py` for the close guard, because
        // the edited ticket class prepends the validator row to every
        // working State's guard list.
        fs::copy(
            repo_root().join("tools/verify_issue_binding.py"),
            cycle.root.join("tools/verify_issue_binding.py"),
        )
        .expect("ship the workflow validator beside the fixture runbook");
        cycle.write(
            ".gitignore",
            ".ratmac/runs/\n.ratmac/mint.toml\n.ratmac/locks/\n.ratmac/log.md\n",
        );
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
        for name in ["design.md", "ubi-lang.md"] {
            fs::write(dir.join(name), format!("# {name}\n")).expect("write issue file");
        }
        // t-133's accepted contract: `write_issue`'s `test-plan.md` stub
        // gains the fixture's check identifiers (`PT-100-01`, `HT-100-01`)
        // as verification rows; its `spec.md` already carries `DEMO-001`.
        fs::write(
            dir.join("test-plan.md"),
            "# test-plan.md\n\n\
             | Check | Requirement Refs | Expected evidence |\n\
             |---|---|---|\n\
             | `PT-100-01` | [`DEMO-001`](spec.md#requirement-records) | Planned. |\n\
             | `HT-100-01` | [`DEMO-001`](spec.md#requirement-records) | Planned. |\n",
        )
        .expect("write issue test plan");
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

    /// The work item P3 cuts for that gap. It also carries the three
    /// declaration lists the completion guard selects - `focused-tests`,
    /// `hidden-lanes`, and `quality-commands`, matching the receipt ids
    /// `write_completion` records - while the legacy `planned-test-refs`
    /// list keeps driving the sensitivity gate.
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

    fn spawn(&self, name: &str, run: &str, binds: &[&str]) -> String {
        let mut command = Command::new(ratmac_qa::engine_bin!());
        command.args(["spawn", name, "--run", run]);
        for bind in binds {
            command.args(["--bind", bind]);
        }
        let output = command
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm");
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
    let runbook = shipped_runbook();
    let class = MachineClass::from_toml(&runbook)
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
    // t-133's accepted contract: the spawn gains `--bind issue=i-100-demo`
    // beside the item binding, because the edited spawn table binds
    // `item` and `issue` and the validator resolves the pair.
    let child = cycle.spawn("turn", &run, &["item=t-100.md", "issue=i-100-demo"]);
    assert_eq!(cycle.state(&child), "tests");
    cycle.write_sensitivity(&child, "PT-100-01");
    cycle.step(&child);
    assert_eq!(cycle.state(&child), "implement");

    // The fixture declares exactly three non-empty checks - one focused,
    // one hidden lane, one quality command - and the gate refuses while any
    // of them lacks its receipt, naming the missing check. With no receipts
    // at all the refusal names all three in declaration order.
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

fn severity_word(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}
