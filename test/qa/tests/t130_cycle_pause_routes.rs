//! t-130 / WRS-003: cycle pause routes - the public suite.
//!
//! WRSV-003-01 `wrsv_003_01` holds at every working State of the shipped
//! Plan-Build Runbook (its routed copy) and judges the durable pause facts.
//! WRSV-003-02 `wrsv_003_02` judges the routed class's shape and acceptance.
//! WRSV-003-03 `wrsv_003_03` judges hold's refusal twins against a positive
//! control.
//! WRSV-003-04 `wrsv_003_04` judges evidence and pin retention plus the
//! pre-change Run's drift refusal.
//! WRSV-003-05 `wrsv_003_05` judges the governed positive resume for a parent
//! and a child.
//! WRSV-003-06 `wrsv_003_06` judges resume's refusal twins and route-table
//! bytes.
//! WRSV-003-07 `wrsv_003_07` judges stale receipts after a paused source
//! change and the amendment that resumes nothing.
//! WRSV-003-08 `wrsv_003_08` judges the operation journal's publication
//! boundaries, recovery, and readers.
//! WRSV-003-09 `wrsv_003_09` judges pause occurrences and the exact binding
//! of a resolution authorization.
//!
//! Every fixture is its own temporary Git repository carrying a routed copy
//! of the shipped Runbook: the recorded base bytes (the sha256 the ticket
//! pins) plus the eight blocked self-routes WRS-003 declares. The repository's
//! tracked `.ratmac/ratmac.toml` is never read to prove the eight routes. All
//! Engine invocations go through the in-process library entry so the t-129
//! test trust injection participates where a fixture is governed; every
//! signing key is generated with the host's real `ssh-keygen`.

use ratmac::cli;
use ratmac::machine::MachineClass;
use ratmac::review::test::{install, TestTrust};
use ratmac_qa::support::{self, TempTree};
use ratmac_qa::tempgit::TempRepo;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{Duration, Instant};

/// The sha256 of the tracked `.ratmac/ratmac.toml` bytes of this ticket's
/// merge base: the recorded base the pause-routes diff applies to.
const BASE_RUNBOOK_SHA256: &str =
    "012eaa36c1a895f50d8822d516699029cb461a2d489b8ee2b882ce37a8b440c6";

/// The blocker reference every fixture holds against: beneath the declared
/// authority root, opaque to the Engine (NRR-001).
const BLOCKER: &str = ".arca/blocker-t-130.md";

/// The host verifier this suite pins, per the accepted design.
const KEYGEN: &str = "C:/WINDOWS/System32/OpenSSH/ssh-keygen.exe";
const REVIEW_NAMESPACE: &str = "ratmac-review-v1";
const APPROVE_NAMESPACE: &str = "ratmac-approve-v1";
const RESUME_NAMESPACE: &str = "ratmac-resume-v1";

/// The green output a receipt records for a check that passed.
const GREEN: &str = "test result: ok. 1 passed; 0 failed\n";
/// The red output a sensitivity receipt records for a planned test that
/// failed before its implementation existed.
const RED: &str = "test result: FAILED. 0 passed; 1 failed\n";

/// The tracked `.ratmac/ratmac.toml` bytes of this ticket's merge base:
/// the recorded base the permanent pause-routes diff applies over.
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

/// The eight blocked self-routes WRS-003 adds to the recorded base bytes,
/// in the order the permanent `.ratmac/pause-routes.diff` appends them.
const PAUSE_ROUTES: &str = "\n# WRS-003: the human-confirmed pause route for each working State.\n\n[[transitions]]\nfrom = \"intake\"\nto = \"intake\"\nblocked-route = true\n\n[[transitions]]\nfrom = \"gap-check\"\nto = \"gap-check\"\nblocked-route = true\n\n[[transitions]]\nfrom = \"cut-tickets\"\nto = \"cut-tickets\"\nblocked-route = true\n\n[[transitions]]\nfrom = \"ticket-turns\"\nto = \"ticket-turns\"\nblocked-route = true\n\n[[transitions]]\nfrom = \"close\"\nto = \"close\"\nblocked-route = true\n\n[[classes.ticket.transitions]]\nfrom = \"tests\"\nto = \"tests\"\nblocked-route = true\n\n[[classes.ticket.transitions]]\nfrom = \"implement\"\nto = \"implement\"\nblocked-route = true\n\n[[classes.ticket.transitions]]\nfrom = \"damage\"\nto = \"damage\"\nblocked-route = true\n";

// --- shared plumbing ---------------------------------------------------------

/// One in-process Engine invocation: its exit code, its stdout bytes, and
/// its refusal message (empty on success).
struct Ran {
    code: i32,
    stdout: String,
    message: String,
}

impl std::fmt::Display for Ran {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "exit {} stdout {:?} message {:?}",
            self.code, self.stdout, self.message
        )
    }
}

/// Run one `rtm` command line in `root` through the library entry, so an
/// installed trust reader participates.
fn rtm(root: &Path, args: &[&str]) -> Ran {
    let mut full = vec!["rtm".to_owned()];
    full.extend(args.iter().map(|arg| (*arg).to_owned()));
    let mut stdout = Vec::new();
    match cli::run_from(full, root, &mut stdout) {
        Ok(code) => Ran {
            code,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            message: String::new(),
        },
        Err(error) => Ran {
            code: error.exit_code(),
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            message: error.to_string(),
        },
    }
}

/// The repository root this suite is compiled against.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The one spelling paths carry in this suite's expectations.
fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Length-frame one field: unsigned 64-bit big-endian length, then bytes.
fn frame(body: &mut Vec<u8>, field: &[u8]) {
    body.extend_from_slice(&(field.len() as u64).to_be_bytes());
    body.extend_from_slice(field);
}

/// The held context exactly as the contract frames it:
/// `ratmac-held-context-v1`, one NUL byte, then each field framed in order.
fn held_context_digest(
    run: &str,
    state: &str,
    blocker: &str,
    class: &str,
    workspace: &str,
    runbook_sha256: &str,
    goal_identity: &str,
) -> String {
    let mut body = b"ratmac-held-context-v1".to_vec();
    body.push(0);
    frame(&mut body, run.as_bytes());
    frame(&mut body, state.as_bytes());
    frame(&mut body, blocker.as_bytes());
    frame(&mut body, class.as_bytes());
    frame(&mut body, workspace.as_bytes());
    frame(&mut body, runbook_sha256.as_bytes());
    frame(&mut body, goal_identity.as_bytes());
    sha256_hex(&body)
}

fn toml_string(value: &str) -> String {
    format!("{value:?}")
}

// --- the ungoverned routed fixture -------------------------------------------

/// A temporary Git repository carrying a routed (or other) copy of the
/// shipped Runbook and the artifacts each stage's guards read.
struct Cycle {
    repo: TempRepo,
}

impl Cycle {
    /// One fixture whose runbook is the routed copy of the shipped Runbook.
    fn new(label: &str) -> Self {
        Self::with_runbook(label, &routed_runbook())
    }

    /// One fixture carrying exactly the given runbook bytes.
    fn with_runbook(label: &str, runbook: &str) -> Self {
        let repo = TempRepo::new(label);
        for dir in [
            ".arca/goal",
            ".arca/issue/i-100-demo",
            ".arca/residual",
            ".arca/ticket/archive",
            "src",
            "test",
            "notes",
        ] {
            fs::create_dir_all(repo.root().join(dir)).expect("create fixture directory");
        }
        let fixture = Self { repo };
        fixture.write(".ratmac/ratmac.toml", runbook);
        fixture.write("src/lib.rs", "pub fn work() {}\n");
        fixture.write("test/fixture_test.rs", "fn the_planned_test() {}\n");
        fixture.write("notes/outside.txt", "Outside every declared root.\n");
        fixture.write(
            ".arca/schema.md",
            "# Working rules\n\n### AUTH-001 - the contributor rule\n\nProse.\n",
        );
        fixture.write(
            ".arca/goal/spec.md",
            "# Goal spec\n\n| Req ID | Requirement | Source |\n|---|---|---|\n| DEMO-001 | The demo behaves. | [issue DEMO-001](../issue/i-100-demo/spec.md#requirement-records) |\n",
        );
        fixture.write(
            ".arca/issue/i-100-demo/index.md",
            "# Issue i-100-demo\n\n```yaml\nissue-id: \"i-100-demo\"\nstatus: \"integrated\"\n```\n\nSee [goal spec](../../goal/spec.md).\n",
        );
        fixture.write(
            ".arca/issue/i-100-demo/spec.md",
            "# Requirement records\n\n| Req ID | Requirement | Status |\n|---|---|---|\n| `DEMO-001` | The demo behaves. | accepted |\n",
        );
        for name in ["design.md", "test-plan.md", "ubi-lang.md"] {
            fixture.write(
                &format!(".arca/issue/i-100-demo/{name}"),
                &format!("# {name}\n"),
            );
        }
        fixture.write(BLOCKER, "Opaque blocker; its content is not a gate.\n");
        fixture
    }

    fn root(&self) -> &Path {
        self.repo.root()
    }

    fn write(&self, relative: &str, body: &str) {
        self.repo.write(relative, body);
    }

    fn rewrite_runbook(&self, runbook: &str) {
        self.repo.write(".ratmac/ratmac.toml", runbook);
    }

    fn commit(&self, message: &str) {
        self.repo.commit_all(message);
    }

    fn run_dir(&self, run: &str) -> PathBuf {
        self.root().join(format!(".ratmac/runs/{run}"))
    }

    fn record_text(&self, run: &str) -> String {
        fs::read_to_string(self.run_dir(run).join("run.toml"))
            .unwrap_or_else(|error| panic!("read the Run Record of {run}: {error}"))
    }

    fn record_value(&self, run: &str, key: &str) -> String {
        let parsed: toml::Value = self
            .record_text(run)
            .parse()
            .expect("the Run Record is valid TOML");
        match &parsed[key] {
            toml::Value::String(text) => text.clone(),
            other => other.to_string(),
        }
    }

    fn state(&self, run: &str) -> String {
        self.record_value(run, "state")
    }

    fn status(&self, run: &str) -> String {
        self.record_value(run, "status")
    }

    fn blocker(&self, run: &str) -> String {
        self.record_value(run, "blocker")
    }

    /// The Run Record carries exactly the contract's seven fields.
    fn assert_seven_fields(&self, run: &str) {
        let parsed: toml::Value = self.record_text(run).parse().expect("valid Run Record");
        let mut keys: Vec<&str> = parsed
            .as_table()
            .expect("the Run Record is a table")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "active_refs",
                "blocker",
                "goal_revision",
                "input_revision",
                "output_revision",
                "state",
                "status",
            ],
            "WRS-003: the public Run Record of {run} keeps its seven fields"
        );
    }

    fn evidence_text(&self, run: &str) -> String {
        fs::read_to_string(self.run_dir(run).join("evidence.toml"))
            .unwrap_or_else(|error| panic!("read the evidence of {run}: {error}"))
    }

    fn evidence_value(&self, run: &str, table: &str, field: &str) -> String {
        let parsed: toml::Value = self
            .evidence_text(run)
            .parse()
            .expect("the evidence is valid TOML");
        parsed[table][field].as_str().unwrap_or_default().to_owned()
    }

    /// The runbook pin recorded at Run start.
    fn runbook_pin(&self, run: &str) -> String {
        self.evidence_value(run, "runbook", "sha256")
    }

    /// The goal identity the pause record carries: the frozen revision once
    /// frozen, else the baseline, else empty.
    fn goal_identity(&self, run: &str) -> String {
        let frozen = self.evidence_value(run, "goal", "frozen");
        if !frozen.is_empty() {
            return frozen;
        }
        self.evidence_value(run, "goal", "baseline")
    }

    fn history(&self) -> String {
        fs::read_to_string(self.root().join(".ratmac/log.md"))
            .unwrap_or_else(|error| panic!("read the history log: {error}"))
    }

    fn start(&self) -> String {
        let started = rtm(self.root(), &["start"]);
        assert_eq!(
            started.code, 0,
            "WRS-003: admission must succeed: {}{}",
            started.stdout, started.message
        );
        started
            .stdout
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("rtm: started run "))
            .and_then(|rest| rest.split(" at ").next())
            .unwrap_or_default()
            .to_owned()
    }

    fn spawn_child(&self, parent: &str) -> String {
        let spawned = rtm(
            self.root(),
            &["spawn", "turn", "--run", parent, "--bind", "item=t-100.md"],
        );
        assert_eq!(
            spawned.code, 0,
            "WRS-003: spawn must succeed: {}{}",
            spawned.stdout, spawned.message
        );
        spawned
            .stdout
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("rtm: spawned run "))
            .and_then(|rest| rest.split_whitespace().next())
            .unwrap_or_default()
            .to_owned()
    }

    fn step(&self, run: &str) -> Ran {
        rtm(self.root(), &["step", "--run", run])
    }

    /// Advance one Run and require the ordinary motion to succeed.
    fn step_ok(&self, run: &str) {
        let stepped = self.step(run);
        assert_eq!(
            stepped.code, 0,
            "WRS-003: step of {run} must succeed: {}{}",
            stepped.stdout, stepped.message
        );
        assert!(
            !stepped.stdout.contains("step refused"),
            "WRS-003: step of {run} must not refuse: {}",
            stepped.stdout
        );
    }

    fn hold(&self, run: &str, blocker: &str) -> Ran {
        rtm(
            self.root(),
            &[
                "hold",
                "--run",
                run,
                "--blocker",
                blocker,
                "--confirm",
                &format!("hold {run}"),
            ],
        )
    }

    fn hold_with_phrase(&self, run: &str, blocker: Option<&str>, phrase: Option<&str>) -> Ran {
        let mut args = vec!["hold".to_owned(), "--run".to_owned(), run.to_owned()];
        if let Some(blocker) = blocker {
            args.push("--blocker".to_owned());
            args.push(blocker.to_owned());
        }
        if let Some(phrase) = phrase {
            args.push("--confirm".to_owned());
            args.push(phrase.to_owned());
        }
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        rtm(self.root(), &refs)
    }

    fn resume(&self, run: &str, blocker: &str) -> Ran {
        rtm(
            self.root(),
            &[
                "resume",
                "--run",
                run,
                "--blocker",
                blocker,
                "--confirm",
                &format!("resume {run}"),
            ],
        )
    }

    fn resume_with(&self, run: &str, blocker: Option<&str>, phrase: Option<&str>) -> Ran {
        let mut args = vec!["resume".to_owned(), "--run".to_owned(), run.to_owned()];
        if let Some(blocker) = blocker {
            args.push("--blocker".to_owned());
            args.push(blocker.to_owned());
        }
        if let Some(phrase) = phrase {
            args.push("--confirm".to_owned());
            args.push(phrase.to_owned());
        }
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        rtm(self.root(), &refs)
    }

    fn pause_path(&self, run: &str, occurrence: u64) -> PathBuf {
        self.run_dir(run)
            .join("pauses")
            .join(format!("{occurrence}.toml"))
    }

    /// The committed hold of `run` leaves exactly the strict version-1 pause
    /// record the contract names, at the addressed occurrence.
    fn assert_pause_record(&self, run: &str, occurrence: u64, state: &str, class: &str) {
        let path = self.pause_path(run, occurrence);
        let text = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "WRS-003: the hold of {run} at {state} must leave its pause record {}: {error}",
                shown(&path)
            )
        });
        let parsed: toml::Value = text
            .parse()
            .unwrap_or_else(|error| panic!("pause record {occurrence} of {run} is TOML: {error}"));
        let table = parsed
            .as_table()
            .expect("the pause record is a strict table");
        let mut keys: Vec<&str> = table.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "blocker",
                "class",
                "goal-identity",
                "held-context-sha256",
                "run",
                "runbook-sha256",
                "sequence",
                "state",
                "version",
                "workspace",
            ],
            "WRS-003: pause record {occurrence} of {run} holds exactly the contract's fields"
        );
        let field = |name: &str| {
            table[name]
                .as_str()
                .unwrap_or_else(|| panic!("pause record field {name} is a string"))
                .to_owned()
        };
        assert_eq!(table["version"].as_integer(), Some(1), "version = 1");
        assert_eq!(
            table["sequence"].as_integer(),
            Some(i64::try_from(occurrence).unwrap()),
            "the record's sequence is its occurrence"
        );
        assert_eq!(field("run"), run, "the record names its Run");
        assert_eq!(field("state"), state, "the record names the held State");
        assert_eq!(field("blocker"), BLOCKER, "the blocker is verbatim");
        assert_eq!(field("class"), class, "the record names the owning class");
        let workspace = shown(
            &fs::canonicalize(self.root())
                .unwrap_or_else(|error| panic!("canonicalize the fixture root: {error}")),
        );
        assert_eq!(
            field("workspace"),
            workspace,
            "the workspace is the recorded one, canonical, forward slashes"
        );
        let pin = self.runbook_pin(run);
        assert_eq!(
            field("runbook-sha256"),
            pin,
            "the record carries the pinned runbook hash"
        );
        let goal_identity = self.goal_identity(run);
        assert_eq!(
            field("goal-identity"),
            goal_identity,
            "the goal identity is frozen, else baseline, else empty"
        );
        assert_eq!(
            field("held-context-sha256"),
            held_context_digest(run, state, BLOCKER, class, &workspace, &pin, &goal_identity),
            "the digest is the held context's SHA-256"
        );
    }

    /// A confirmed hold of `run` at `state` routes along the blocked
    /// self-route and leaves its pause record at `occurrence`.
    fn hold_ok(&self, run: &str, occurrence: u64, state: &str, class: &str) {
        let held = self.hold(run, BLOCKER);
        assert_eq!(
            held.code, 0,
            "WRS-003: the confirmed hold of {run} at {state} succeeds: {}{}",
            held.stdout, held.message
        );
        assert_eq!(
            held.stdout,
            format!(
                "rtm: run {run} paused against {BLOCKER}; routed {state} -> {state}\n\
                 The Run is not passed; a human resumes it when the blocker clears.\n"
            ),
            "WRS-003: the hold success bytes keep today's two lines"
        );
        assert_eq!(self.state(run), state, "the address and State survive");
        assert_eq!(self.status(run), "blocked");
        assert_eq!(self.blocker(run), BLOCKER);
        self.assert_seven_fields(run);
        self.assert_pause_record(run, occurrence, state, class);
        assert!(
            self.history().contains(&format!(
                "- Hold: run {run} paused against {BLOCKER}; routed {state} -> {state} on an \
                 explicit human confirmation.\n"
            )),
            "WRS-003: the hold appends its history line"
        );
    }

    /// An authorized resume of `run` returns it to executing at the same
    /// State and clears the addressed occurrence.
    fn resume_ok(&self, run: &str, occurrence: u64, state: &str) {
        let resumed = self.resume(run, BLOCKER);
        assert_eq!(
            resumed.code, 0,
            "WRS-003: the authorized resume of {run} succeeds: {}{}",
            resumed.stdout, resumed.message
        );
        assert_eq!(
            resumed.stdout,
            format!(
                "rtm: run {run} resumed against {BLOCKER}; occurrence {occurrence} cleared\n\
                 The Run executes again at the same State; later motion still requires its \
                 normal fresh checks.\n"
            ),
            "WRS-003: the resume success bytes are the contract's two lines"
        );
        assert_eq!(self.state(run), state, "resume advances no State");
        assert_eq!(self.status(run), "executing");
        assert_eq!(self.blocker(run), "");
        self.assert_seven_fields(run);
        assert!(
            self.pause_path(run, occurrence).is_file(),
            "resume preserves every pause record as history"
        );
        assert!(
            self.history().contains(&format!(
                "- Resume: run {run} resumed against {BLOCKER}; occurrence {occurrence} cleared \
                 on an explicit human resolution authorization.\n"
            )),
            "WRS-003: the resume appends its history line"
        );
    }

    /// The goal revision the freeze recorded for this Run.
    fn frozen(&self, run: &str) -> String {
        self.evidence_value(run, "goal", "frozen")
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
                "# Residual Record\n\n```yaml\nresidual-id: \"res-100\"\n\
                 goal-requirement-ref: \"DEMO-001\"\n\
                 frozen-goal-bundle-revision: \"goal-sha256:{frozen}\"\n\
                 concrete-evidence-refs:\n{refs}status: \"{status}\"\n```\n"
            ),
        );
    }

    /// The reviewer's transition-input record for the branching State.
    fn write_verdict(&self, run: &str, input: &str) {
        let state = self.state(run);
        fs::write(
            self.run_dir(run).join("verdict.toml"),
            format!("state = \"{state}\"\ninput = \"{input}\"\nrationale = \"chose {input}.\"\n"),
        )
        .expect("write the verdict record");
    }

    /// The work item P3 cuts for that gap, with the declaration lists the
    /// completion guard selects.
    fn write_ticket(&self) {
        let lanes: String = [
            "Regression",
            "Input/Routing",
            "Lifecycle/Model",
            "Durability/Recovery",
            "Output/Filesystem",
            "Cross-Feature",
        ]
        .iter()
        .map(|lane| format!("| `{lane}` | `covered` | Reason. | `HT-100-01` |\n"))
        .collect();
        self.write(
            ".arca/ticket/t-100.md",
            &format!(
                "---\nticket-id: \"t-100\"\nresidual-ids:\n  - \"res-100\"\n\
                 planned-test-refs:\n  - \"PT-100-01\"\n\
                 focused-tests:\n  - \"PT-100-01\"\n\
                 hidden-lanes:\n  - \"HT-100-01\"\n\
                 quality-commands:\n  - \"cargo --version\"\n\
                 dependencies: []\nstatus: \"approved\"\n---\n\n\
                 # Ticket: t-100\n\n## Vertical Outcome\n\nOutcome.\n\n\
                 ## Worktree Scope\n\nScope.\n\n\
                 ## P4 Apparent Test Plan\n\n| Apparent Test ID |\n|---|\n| `PT-100-01` |\n\n\
                 ## P5 Hidden Test Public Coverage Manifest\n\n\
                 | Lane | Assessment | Rationale | Hidden IDs |\n|---|---|---|---|\n{lanes}\
                 ## Merge Gate\n\n- Quality: `cargo --version` passes.\n"
            ),
        );
    }

    /// One sensitivity receipt proving a planned test fails before its code.
    fn write_sensitivity(&self, run: &str) {
        let dir = self.root().join(format!(".ratmac/evidence/{run}"));
        fs::create_dir_all(&dir).expect("create the evidence directory");
        let body = format!(
            "planned-test-id = \"PT-100-01\"\nticket-id = \"t-100\"\n\
             kind = \"baseline-failure\"\ncommand = \"cargo test --test fixture_test\"\n\
             working-dir = \".\"\ntest-file = \"test/fixture_test.rs\"\n\
             test-name = \"the_planned_test\"\nexit-status = 101\n\
             output-sha256 = \"{}\"\noutput = \"\"\"\n{RED}\"\"\"\n",
            ratmac::receipt::sha256_text(RED)
        );
        fs::write(dir.join("PT-100-01.toml"), body).expect("write the sensitivity receipt");
    }

    /// Every check the work item declares, recorded green against the tree
    /// as it stands.
    fn write_completion(&self, run: &str) {
        let dir = self
            .root()
            .join(format!(".ratmac/evidence/{run}/completion"));
        fs::create_dir_all(&dir).expect("create the completion directory");
        let digest =
            ratmac::completion::tree_digest(self.root(), &["src".to_owned()]).expect("digest src");
        for (check, kind, command) in [
            ("PT-100-01", "focused", "cargo test --test fixture_test"),
            ("HT-100-01", "hidden-lane", "cargo test --test fixture_test"),
            ("cargo --version", "quality", "cargo --version"),
        ] {
            let body = format!(
                "ticket-id = \"t-100\"\ncheck-id = \"{check}\"\nkind = \"{kind}\"\n\
                 command = \"{command}\"\nworking-dir = \".\"\nexit-status = 0\n\
                 output-sha256 = \"{}\"\ntree-roots = [\"src\"]\ntree-sha256 = \"{digest}\"\n\
                 output = \"\"\"\n{GREEN}\"\"\"\n",
                ratmac::receipt::sha256_text(GREEN)
            );
            fs::write(
                dir.join(format!("{}.toml", ratmac::completion::check_slug(check))),
                body,
            )
            .expect("write the completion receipt");
        }
    }

    /// Drive the parent cycle from intake to ticket-turns.
    fn drive_to_ticket_turns(&self, run: &str) {
        self.step_ok(run);
        assert_eq!(self.state(run), "gap-check");
        let frozen = self.frozen(run);
        self.write_gap("missing", &frozen, &[]);
        self.write_verdict(run, "gaps");
        self.step_ok(run);
        assert_eq!(self.state(run), "cut-tickets");
        self.write_ticket();
        self.step_ok(run);
        assert_eq!(self.state(run), "ticket-turns");
    }

    /// Drive one child from tests to the terminal done State.
    fn drive_child_to_done(&self, child: &str) {
        self.write_sensitivity(child);
        self.step_ok(child);
        assert_eq!(self.state(child), "implement");
        self.write_completion(child);
        self.commit("the green landing");
        self.step_ok(child);
        assert_eq!(self.state(child), "damage");
        self.write_completion(child);
        self.step_ok(child);
        assert_eq!(self.state(child), "done");
        assert_eq!(self.status(child), "passed");
    }

    /// Every receipt file recorded for `run`, keyed by its displayed
    /// forward-slash path relative to the Run's evidence directory.
    fn receipt_snapshot(&self, run: &str) -> BTreeMap<String, Vec<u8>> {
        fn visit(at: &Path, prefix: &str, result: &mut BTreeMap<String, Vec<u8>>) {
            if let Ok(entries) = fs::read_dir(at) {
                for entry in entries {
                    let entry = entry.expect("receipt entry is readable");
                    let path = entry.path();
                    let name = format!("{prefix}/{}", entry.file_name().to_string_lossy());
                    if entry.file_type().expect("receipt file kind").is_dir() {
                        visit(&path, &name, result);
                    } else {
                        result.insert(name, fs::read(&path).expect("receipt is readable"));
                    }
                }
            }
        }
        let mut result = BTreeMap::new();
        visit(
            &self.root().join(format!(".ratmac/evidence/{run}")),
            "",
            &mut result,
        );
        result
    }

    /// Every byte under the fixture except Git's own, keyed by displayed
    /// forward-slash path.
    fn snapshot(&self) -> BTreeMap<String, Vec<u8>> {
        let mut result = BTreeMap::new();
        fn visit(root: &Path, at: &Path, result: &mut BTreeMap<String, Vec<u8>>) {
            for entry in fs::read_dir(at).expect("snapshot directory is readable") {
                let entry = entry.expect("snapshot entry is readable");
                if at == root && entry.file_name() == ".git" {
                    continue;
                }
                let path = entry.path();
                if entry.file_type().expect("snapshot file kind").is_dir() {
                    visit(root, &path, result);
                } else {
                    result.insert(
                        shown(path.strip_prefix(root).expect("snapshot stays local")),
                        fs::read(&path).expect("snapshot file is readable"),
                    );
                }
            }
        }
        visit(self.root(), self.root(), &mut result);
        result
    }

    /// One refusal changed no byte under the fixture.
    fn assert_unchanged(&self, before: &BTreeMap<String, Vec<u8>>, what: &str) {
        assert_eq!(
            self.snapshot(),
            *before,
            "WRS-003: the {what} wrote a byte somewhere"
        );
    }
}

// --- the governed fixture ------------------------------------------------------

fn ssh(dir: &Path, args: &[&str]) -> Output {
    assert!(
        Path::new(KEYGEN).is_file(),
        "the host verifier {KEYGEN} must exist"
    );
    support::command(KEYGEN, dir)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("run the host ssh-keygen {args:?}: {error}"))
}

/// Fixture identities generated at test time with the host's real tool.
struct KeySet {
    dir: TempTree,
    publics: BTreeMap<String, String>,
}

impl KeySet {
    fn new(label: &str, names: &[&str]) -> Self {
        let dir = TempTree::new(&format!("{label}-keys")).expect("create the fixture key tree");
        let mut publics = BTreeMap::new();
        for name in names {
            let path = dir.join(name);
            let generated = ssh(
                dir.path(),
                &[
                    "-t",
                    "ed25519",
                    "-f",
                    &path.to_string_lossy(),
                    "-N",
                    "",
                    "-C",
                    name,
                ],
            );
            assert!(
                generated.status.success(),
                "generate the fixture key {name}: {}",
                support::text(&generated)
            );
            let public = fs::read_to_string(dir.join(format!("{name}.pub")))
                .unwrap_or_else(|error| panic!("read {name}.pub: {error}"));
            publics.insert((*name).to_owned(), public.trim().to_owned());
        }
        Self { dir, publics }
    }

    fn public(&self, name: &str) -> &str {
        self.publics
            .get(name)
            .unwrap_or_else(|| panic!("the fixture key {name} exists"))
    }

    /// One detached signature over `payload`'s bytes under `namespace`, on a
    /// fresh per-signer copy so the host ssh-keygen never meets an existing
    /// `.sig` it refuses to overwrite.
    fn sign(&self, name: &str, namespace: &str, payload: &Path) -> PathBuf {
        let stem = payload
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let copy = payload.with_file_name(format!("{stem}.{name}.{namespace}"));
        let signature = PathBuf::from(format!("{}.sig", shown(&copy)));
        assert!(
            !copy.is_file() && !signature.is_file(),
            "each signature needs a fresh payload copy: {}",
            shown(&copy)
        );
        fs::copy(payload, &copy)
            .unwrap_or_else(|error| panic!("copy the payload for {name}: {error}"));
        let key = self.dir.join(name);
        let signed = ssh(
            self.dir.path(),
            &[
                "-Y",
                "sign",
                "-f",
                &key.to_string_lossy(),
                "-n",
                namespace,
                &copy.to_string_lossy(),
            ],
        );
        assert!(
            signed.status.success(),
            "sign with {name} under {namespace}: {}",
            support::text(&signed)
        );
        assert!(
            signature.is_file(),
            "the detached signature {} exists",
            shown(&signature)
        );
        signature
    }
}

/// The declared policy digest exactly as t-129 frames it.
fn policy_digest(
    approver: &str,
    minimum: u32,
    may_review: bool,
    reviewers: &[(String, String)],
) -> String {
    let mut body = b"ratmac-policy-v1".to_vec();
    body.push(0);
    frame(&mut body, approver.as_bytes());
    frame(&mut body, minimum.to_string().as_bytes());
    frame(
        &mut body,
        if may_review { "true" } else { "false" }.as_bytes(),
    );
    let mut sorted = reviewers.to_vec();
    sorted.sort_by(|left, right| left.0.cmp(&right.0));
    for (label, key) in sorted {
        frame(&mut body, label.as_bytes());
        frame(&mut body, key.as_bytes());
    }
    sha256_hex(&body)
}

/// The operating-system identity the Engine renders as the builder actor.
fn builder_actor() -> String {
    if cfg!(windows) {
        format!(
            "{}\\{}",
            std::env::var("USERDOMAIN").unwrap_or_default(),
            std::env::var("USERNAME").unwrap_or_default()
        )
    } else {
        std::env::var("USER").unwrap_or_else(|_| "builder".to_owned())
    }
}

/// The primary checkout Git itself reports for `root`.
fn resolved_primary(root: &Path) -> PathBuf {
    let output = support::git(root)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .expect("list the fixture worktrees");
    assert!(
        output.status.success(),
        "the fixture repository lists its worktrees"
    );
    let text = String::from_utf8_lossy(&output.stdout);
    text.split("\n\n")
        .find(|record| !record.lines().any(|line| line == "bare"))
        .and_then(|record| {
            record
                .lines()
                .find_map(|line| line.strip_prefix("worktree "))
        })
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("the fixture repository reports a primary worktree: {text}"))
}

/// A temporary Git repository carrying the routed shipped Runbook plus a
/// declared review policy over both the top-level and the ticket class, its
/// fixture identities, and the installed trust reader for its Engine root.
struct Governed {
    cycle: Cycle,
    keys: KeySet,
    _trust: TestTrustGuard,
}

/// Keeps the installed trust reader alive for the fixture's whole life.
struct TestTrustGuard {
    _guard: ratmac::review::test::TrustGuard,
}

impl Drop for TestTrustGuard {
    fn drop(&mut self) {}
}

impl Governed {
    fn new(label: &str) -> Self {
        let keys = KeySet::new(label, &["a", "b", "lead", "outsider"]);
        let reviewers: Vec<(String, String)> = ["a", "b", "lead"]
            .iter()
            .map(|label| ((*label).to_owned(), keys.public(label).to_owned()))
            .collect();
        let cycle = Cycle::new(label);
        let verifier_digest = sha256_hex(&fs::read(KEYGEN).expect("read the pinned verifier"));
        let mut signers = String::new();
        for (label, public) in &reviewers {
            let parts: Vec<&str> = public.split_whitespace().collect();
            assert!(
                parts.len() >= 2 && parts[0] == "ssh-ed25519",
                "the fixture public key for {label} is one OpenSSH line"
            );
            signers.push_str(&format!("{} {} {}\n", label, parts[0], parts[1]));
        }
        keys.dir.write("allowed_signers", &signers);
        let signers_path = keys.dir.join("allowed_signers");
        let signers_digest = sha256_hex(signers.as_bytes());
        let true_digest = policy_digest("lead", 2, false, &reviewers);
        let scope = shown(
            &fs::canonicalize(resolved_primary(cycle.root()))
                .expect("canonicalize the fixture primary"),
        );
        let mut manifest = String::new();
        manifest.push_str(&format!("project-scope = \"{scope}\"\n"));
        manifest.push_str(&format!("policy-digest = \"{true_digest}\"\n"));
        manifest.push_str("approver = \"lead\"\n");
        manifest.push_str("minimum-reviews = 2\n");
        manifest.push_str("approver-may-review = false\n");
        manifest.push_str(&format!("verifier-path = \"{KEYGEN}\"\n"));
        manifest.push_str(&format!("verifier-digest = \"{verifier_digest}\"\n"));
        manifest.push_str(&format!("signers-path = \"{}\"\n", shown(&signers_path)));
        manifest.push_str(&format!("signers-digest = \"{signers_digest}\"\n"));
        manifest.push_str("[builder]\n");
        manifest.push_str(&format!("actor = {}\n", toml_string(&builder_actor())));
        manifest.push_str("keys = []\n");
        for (label, public) in &reviewers {
            manifest.push_str("\n[[reviewers]]\n");
            manifest.push_str(&format!("label = {}\n", toml_string(label)));
            manifest.push_str(&format!("key = {}\n", toml_string(public)));
        }
        let manifest_bytes = manifest.into_bytes();
        keys.dir.write("enrollment-manifest.toml", &manifest_bytes);
        let manifest_path = keys.dir.join("enrollment-manifest.toml");

        let mut runbook = routed_runbook();
        for table in ["review", "classes.ticket.review"] {
            runbook.push_str(&format!("\n[{table}]\n"));
            runbook.push_str(&format!("manifest = \"{}\"\n", shown(&manifest_path)));
            runbook.push_str(&format!("policy-digest = \"{true_digest}\"\n"));
            runbook.push_str(&format!("verifier-digest = \"{verifier_digest}\"\n"));
            runbook.push_str(&format!("signers-digest = \"{signers_digest}\"\n"));
            runbook.push_str("minimum-reviews = 2\n");
            runbook.push_str("approver = \"lead\"\n");
            runbook.push_str("approver-may-review = false\n");
            runbook.push_str("work = [\".arca/goal\"]\n");
            for (label, public) in &reviewers {
                runbook.push_str(&format!("\n[[{table}.reviewers]]\n"));
                runbook.push_str(&format!("label = {}\n", toml_string(label)));
                runbook.push_str(&format!("key = {}\n", toml_string(public)));
            }
        }
        cycle.rewrite_runbook(&runbook);
        cycle.commit("fixture baseline");
        let guard = install(
            &cycle.root().join(".ratmac"),
            TestTrust {
                manifest_bytes,
                signers_path,
                verifier_path: PathBuf::from(KEYGEN),
                verifier_sha256: verifier_digest,
            },
        );
        Self {
            cycle,
            keys,
            _trust: TestTrustGuard { _guard: guard },
        }
    }

    fn admit(&self) -> String {
        self.cycle.start()
    }

    /// Render the canonical intent of `run` with the extra purpose args and
    /// prove the pinned report line.
    fn intent(&self, run: &str, extra: &[&str], tag: &str) -> PathBuf {
        let out = self.keys.dir.join(format!("{tag}.bin"));
        let out_argument = shown(&out);
        let mut args = vec!["intent", "--run", run];
        args.extend_from_slice(extra);
        args.push("--out");
        let mut owned: Vec<String> = args.iter().map(|value| (*value).to_owned()).collect();
        owned.push(out_argument);
        let refs: Vec<&str> = owned.iter().map(|value| value.as_str()).collect();
        let rendered = rtm(self.cycle.root(), &refs);
        assert_eq!(
            rendered.code, 0,
            "WRS-003: render the {tag} intent for {run}: {}{}",
            rendered.stdout, rendered.message
        );
        let bytes = fs::read(&out).unwrap_or_else(|error| panic!("read {tag}: {error}"));
        let purpose = if extra.contains(&"--hold") {
            "hold"
        } else if extra.contains(&"--resume") {
            "resume"
        } else {
            "step"
        };
        assert_eq!(
            rendered.stdout,
            format!(
                "rtm: canonical {purpose} intent for run {run} written to {} (sha256 {})\n",
                shown(&out),
                sha256_hex(&bytes)
            ),
            "WRS-003: the {tag} intent report is the pinned one line"
        );
        out
    }

    /// The number of leading `--hold`/`--blocker` purpose arguments.
    fn purpose_len(auth: &[String], blocker: &str) -> usize {
        if auth.len() >= 3 && auth[0] == "--hold" && auth[1] == "--blocker" && auth[2] == blocker {
            3
        } else {
            0
        }
    }

    /// One fully signed authorization bundle's `--review`/`--approve`
    /// arguments over the (optionally blocked) motion intent of `run`.
    fn authorization_args(&self, run: &str, hold: Option<&str>, tag: &str) -> Vec<String> {
        let mut purpose_args: Vec<String> = Vec::new();
        if let Some(blocker) = hold {
            purpose_args.push("--hold".to_owned());
            purpose_args.push("--blocker".to_owned());
            purpose_args.push(blocker.to_owned());
        }
        let purpose_refs: Vec<&str> = purpose_args.iter().map(String::as_str).collect();
        let review_intent = self.intent(run, &purpose_refs, &format!("{tag}-review"));
        let mut offered: Vec<String> = purpose_args.clone();
        for label in ["a", "b"] {
            let signature = self.keys.sign(label, REVIEW_NAMESPACE, &review_intent);
            offered.push("--review".to_owned());
            offered.push(format!("{label}:{}", shown(&signature)));
        }
        let mut approval_args = purpose_args.clone();
        let reviews = &offered[purpose_args.len()..];
        for pair in reviews.chunks(2) {
            approval_args.push(pair[0].clone());
            approval_args.push(pair[1].clone());
        }
        let approval_refs: Vec<&str> = approval_args.iter().map(String::as_str).collect();
        let approval_intent = self.intent(run, &approval_refs, &format!("{tag}-approval"));
        let approve = self.keys.sign("lead", APPROVE_NAMESPACE, &approval_intent);
        offered.push("--approve".to_owned());
        offered.push(shown(&approve));
        offered
    }

    /// One governed ordinary step of `run`.
    fn step_with(&self, run: &str, tag: &str) {
        let auth = self.authorization_args(run, None, tag);
        let mut args = vec!["step".to_owned(), "--run".to_owned(), run.to_owned()];
        args.extend(auth);
        let refs: Vec<&str> = args.iter().map(|value| value.as_str()).collect();
        let stepped = rtm(self.cycle.root(), &refs);
        assert_eq!(
            stepped.code, 0,
            "WRS-003: the governed step of {run} succeeds: {}{}",
            stepped.stdout, stepped.message
        );
        assert!(
            !stepped.stdout.contains("step refused"),
            "WRS-003: the governed step of {run} must not refuse: {}",
            stepped.stdout
        );
    }

    /// One governed hold of `run` against `blocker`.
    fn hold_with(&self, run: &str, blocker: &str, tag: &str) -> Ran {
        let auth = self.authorization_args(run, Some(blocker), tag);
        let mut args = vec![
            "hold".to_owned(),
            "--run".to_owned(),
            run.to_owned(),
            "--blocker".to_owned(),
            blocker.to_owned(),
            "--confirm".to_owned(),
            format!("hold {run}"),
        ];
        let split = Self::purpose_len(&auth, blocker);
        args.extend(auth[split..].to_vec());
        let refs: Vec<&str> = args.iter().map(|value| value.as_str()).collect();
        rtm(self.cycle.root(), &refs)
    }

    /// Render the canonical resolution intent of `run`.
    fn resume_intent(&self, run: &str, tag: &str) -> PathBuf {
        self.intent(run, &["--resume"], tag)
    }

    /// One resume of `run`, optionally carrying one resolution signature.
    fn resume_sig(&self, run: &str, blocker: &str, resolution: Option<&Path>) -> Ran {
        let mut args = vec![
            "resume".to_owned(),
            "--run".to_owned(),
            run.to_owned(),
            "--blocker".to_owned(),
            blocker.to_owned(),
            "--confirm".to_owned(),
            format!("resume {run}"),
        ];
        if let Some(resolution) = resolution {
            args.push("--resolution".to_owned());
            args.push(shown(resolution));
        }
        let refs: Vec<&str> = args.iter().map(|value| value.as_str()).collect();
        rtm(self.cycle.root(), &refs)
    }

    fn authorizations(&self, run: &str) -> BTreeMap<String, String> {
        let dir = self.cycle.run_dir(run).join("authorizations");
        let mut records = BTreeMap::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries {
                let entry = entry.expect("read an authorization entry");
                let name = entry.file_name().to_string_lossy().into_owned();
                let text = fs::read_to_string(entry.path()).expect("read an authorization record");
                records.insert(name, text);
            }
        }
        records
    }
}

/// The routed copy of the shipped Runbook: the recorded base bytes plus the
/// eight blocked self-routes.
fn routed_runbook() -> String {
    format!("{}{}", base_runbook_bytes(), PAUSE_ROUTES)
}

fn base_runbook_bytes() -> &'static str {
    BASE_RUNBOOK
}

// --- WRSV-003-01 ---------------------------------------------------------------

/// WRSV-003-01: drive a fresh copy of the shipped Runbook to each of the five working cycle States and three working ticket States; a correctly confirmed hold retains its address and State and records its blocker.
#[test]
fn wrsv_003_01() {
    let cycle = Cycle::new("wrsv00301");
    let run = cycle.start();
    assert_eq!(cycle.state(&run), "intake");

    // The positive control: the confirmed hold at intake routes the Run along
    // its blocked self-route and leaves its pause record.
    cycle.hold_ok(&run, 1, "intake", "runbook");
    cycle.resume_ok(&run, 1, "intake");

    cycle.step_ok(&run);
    assert_eq!(cycle.state(&run), "gap-check");
    let frozen = cycle.frozen(&run);
    cycle.write_gap("missing", &frozen, &[]);
    cycle.write_verdict(&run, "gaps");
    cycle.hold_ok(&run, 2, "gap-check", "runbook");
    cycle.resume_ok(&run, 2, "gap-check");
    cycle.step_ok(&run);
    assert_eq!(cycle.state(&run), "cut-tickets");

    cycle.write_ticket();
    cycle.hold_ok(&run, 3, "cut-tickets", "runbook");
    cycle.resume_ok(&run, 3, "cut-tickets");
    cycle.step_ok(&run);
    assert_eq!(cycle.state(&run), "ticket-turns");

    let child = cycle.spawn_child(&run);
    assert_eq!(cycle.state(&child), "tests");
    cycle.hold_ok(&child, 1, "tests", "ticket");
    cycle.resume_ok(&child, 1, "tests");

    cycle.write_sensitivity(&child);
    cycle.step_ok(&child);
    assert_eq!(cycle.state(&child), "implement");
    cycle.write_completion(&child);
    cycle.commit("the green landing");
    cycle.hold_ok(&child, 2, "implement", "ticket");
    cycle.resume_ok(&child, 2, "implement");
    cycle.step_ok(&child);
    assert_eq!(cycle.state(&child), "damage");

    cycle.write_completion(&child);
    cycle.hold_ok(&child, 3, "damage", "ticket");
    cycle.resume_ok(&child, 3, "damage");
    cycle.step_ok(&child);
    assert_eq!(cycle.state(&child), "done");
    assert_eq!(cycle.status(&child), "passed");

    cycle.step_ok(&run);
    assert_eq!(cycle.state(&run), "close");
    cycle.hold_ok(&run, 4, "close", "runbook");
    cycle.resume_ok(&run, 4, "close");

    // Every pause record of both Runs survives as history.
    for occurrence in 1..=4 {
        assert!(cycle.pause_path(&run, occurrence).is_file());
    }
    for occurrence in 1..=3 {
        assert!(cycle.pause_path(&child, occurrence).is_file());
    }
}

/// The narrowing control for doctor's RB207: one ordinary (non-blocked)
/// self-route in a minimal class that is otherwise clean, so the routed
/// runbook's zero findings cannot come from RB207 being silenced outright.
const ORDINARY_SELF_ROUTE_RUNBOOK: &str = concat!(
    "[roots]\n",
    "goal = \"work\"\n",
    "residual = \"work\"\n",
    "ticket = \"work\"\n",
    "\n",
    "[states.start]\n",
    "prompt = \"Start.\"\n",
    "\n",
    "[states.done]\n",
    "prompt = \"Done.\"\n",
    "\n",
    "[states.build]\n",
    "prompt = \"Build.\"\n",
    "inputs = [\"self\", \"done\"]\n",
    "guards = [{ kind = \"record_contract\" }]\n",
    "\n",
    "[[transitions]]\n",
    "from = \"build\"\n",
    "to = \"build\"\n",
    "input = \"self\"\n",
    "\n",
    "[[transitions]]\n",
    "from = \"build\"\n",
    "to = \"done\"\n",
    "input = \"done\"\n",
    "\n",
    "[[transitions]]\n",
    "from = \"start\"\n",
    "to = \"build\"\n",
);

// --- WRSV-003-02 ---------------------------------------------------------------

/// WRSV-003-02: check exactly one blocked route per working State and none on rest/done; doctor accepts the runbook and ordinary traversal still chooses only ordinary edges.
#[test]
fn wrsv_003_02() {
    // The embedded base copy is the recorded base bytes.
    assert_eq!(
        sha256_hex(base_runbook_bytes().as_bytes()),
        BASE_RUNBOOK_SHA256,
        "WRS-003: the suite's base copy is the recorded merge-base runbook"
    );
    let routed = routed_runbook();
    let class = MachineClass::from_toml(&routed)
        .expect("the routed copy of the shipped Runbook parses through the one reader");

    let blocked_self_routes = |transitions: &[ratmac::graph::Transition], state: &str| -> usize {
        transitions
            .iter()
            .filter(|transition| {
                transition.is_blocked_route()
                    && transition.from().as_str() == state
                    && transition.to().as_str() == state
            })
            .count()
    };
    for state in [
        "intake",
        "gap-check",
        "cut-tickets",
        "ticket-turns",
        "close",
    ] {
        assert_eq!(
            blocked_self_routes(class.transitions(), state),
            1,
            "WRS-003: the working cycle State {state} declares exactly one blocked self-route"
        );
        assert_eq!(
            class
                .transitions()
                .iter()
                .filter(|transition| transition.is_blocked_route()
                    && transition.from().as_str() == state)
                .count(),
            1,
            "WRS-003: {state} declares no other blocked route"
        );
    }
    assert_eq!(
        blocked_self_routes(class.transitions(), "rest"),
        0,
        "WRS-003: the terminal rest State declares no pause destination"
    );
    let ticket = class
        .classes()
        .get("ticket")
        .expect("the ticket class is declared");
    for state in ["tests", "implement", "damage"] {
        assert_eq!(
            blocked_self_routes(ticket.transitions(), state),
            1,
            "WRS-003: the working ticket State {state} declares exactly one blocked self-route"
        );
        assert_eq!(
            ticket
                .transitions()
                .iter()
                .filter(|transition| transition.is_blocked_route()
                    && transition.from().as_str() == state)
                .count(),
            1,
            "WRS-003: {state} declares no other blocked route"
        );
    }
    assert_eq!(
        blocked_self_routes(ticket.transitions(), "done"),
        0,
        "WRS-003: the terminal done State declares no pause destination"
    );

    // Doctor accepts the routed runbook: no finding at all.
    let scratch = TempTree::new("wrsv00302-doctor").expect("create the doctor scratch tree");
    scratch.write("ratmac.toml", routed.as_bytes());
    let findings = ratmac::doctor::diagnose(&scratch.path().join("ratmac.toml"));
    let rendered: Vec<String> = findings
        .iter()
        .map(|finding| {
            format!(
                "{} {} {} {}",
                finding.code(),
                match finding.severity() {
                    ratmac::doctor::Severity::Error => "error",
                    ratmac::doctor::Severity::Warning => "warning",
                },
                finding.location(),
                finding.message()
            )
        })
        .collect();
    assert!(
        findings.is_empty(),
        "WRS-003: doctor accepts the routed runbook with no finding: {rendered:?}"
    );
    assert_eq!(
        ratmac::doctor::exit_code(&findings),
        0,
        "WRS-003: the routed runbook leaves doctor clean"
    );

    // Ordinary traversal still chooses only ordinary edges.
    let cycle = Cycle::new("wrsv00302");
    let run = cycle.start();
    cycle.step_ok(&run);
    assert_eq!(
        cycle.state(&run),
        "gap-check",
        "intake leaves by its ordinary edge"
    );
    let frozen = cycle.frozen(&run);
    cycle.write_gap("missing", &frozen, &[]);
    cycle.write_verdict(&run, "gaps");
    cycle.step_ok(&run);
    assert_eq!(
        cycle.state(&run),
        "cut-tickets",
        "a blocked self-route never enters ordinary route selection"
    );

    // The permanent pause-routes diff lands beside the runbook and applies
    // over a clean copy of the recorded base bytes.
    let diff = repo_root().join(".ratmac/pause-routes.diff");
    assert!(
        diff.is_file(),
        "WRS-003: the permanent pause-routes diff lands beside the runbook: {}",
        shown(&diff)
    );
    // The diff follows the close-guard precedent: its paths read
    // `a/.ratmac/ratmac.toml`, so it is wired from a tree whose clean base
    // copy sits at `.ratmac/ratmac.toml` exactly as `git apply
    // .ratmac/pause-routes.diff` does from the repository root.
    let apply = TempTree::new("wrsv00302-apply").expect("create the apply tree");
    apply.write(".ratmac/ratmac.toml", base_runbook_bytes().as_bytes());
    fs::copy(&diff, apply.path().join("pause-routes.diff"))
        .expect("copy the tracked diff into the apply tree");
    let checked = support::git(apply.path())
        .args(["apply", "--check", "pause-routes.diff"])
        .output()
        .expect("prove the diff applies over the clean base copy");
    assert!(
        checked.status.success(),
        "WRS-003: git apply --check accepts the pause-routes diff over the recorded base bytes: \
         {}",
        support::text(&checked)
    );
    let applied = support::git(apply.path())
        .args(["apply", "pause-routes.diff"])
        .output()
        .expect("apply the pause-routes diff");
    assert!(
        applied.status.success(),
        "WRS-003: the pause-routes diff applies: {}",
        support::text(&applied)
    );
    let wired =
        fs::read_to_string(apply.path().join(".ratmac/ratmac.toml")).expect("read the wired copy");
    let wired_class = MachineClass::from_toml(&wired)
        .expect("the wired copy of the shipped Runbook parses through the one reader");
    // The applied bytes parse to exactly the routed class this suite composes.
    let shape = |class: &MachineClass| -> Vec<(String, String, bool)> {
        let mut rows: Vec<(String, String, bool)> = class
            .transitions()
            .iter()
            .map(|transition| {
                (
                    transition.from().as_str().to_owned(),
                    transition.to().as_str().to_owned(),
                    transition.is_blocked_route(),
                )
            })
            .collect();
        for (name, child) in class.classes() {
            for transition in child.transitions() {
                rows.push((
                    format!("{name}.{}", transition.from().as_str()),
                    transition.to().as_str().to_owned(),
                    transition.is_blocked_route(),
                ));
            }
        }
        rows.sort();
        rows
    };
    assert_eq!(
        shape(&wired_class),
        shape(&class),
        "WRS-003: the diff's applied bytes parse to exactly the routed class"
    );

    // The narrowing control: an ordinary (non-blocked) self-route keeps
    // today's RB207 warning bytes.
    let ordinary = TempTree::new("wrsv00302-rb207").expect("create the RB207 control tree");
    ordinary.write("ratmac.toml", ORDINARY_SELF_ROUTE_RUNBOOK.as_bytes());
    let control = ratmac::doctor::diagnose(&ordinary.path().join("ratmac.toml"));
    assert_eq!(
        control.len(),
        1,
        "WRS-003: the ordinary self-route control reports exactly one finding: {control:?}"
    );
    assert_eq!(control[0].code(), "RB207");
    assert_eq!(control[0].location(), "transition \"build\" -> \"build\"");
    assert_eq!(
        control[0].message(),
        "this transition leaves and enters the same State, so taking it makes no progress"
    );
    assert_eq!(
        ratmac::doctor::exit_code(&control),
        1,
        "WRS-003: today's RB207 warning still exits 1"
    );
}

// --- WRSV-003-03 ---------------------------------------------------------------

/// WRSV-003-03: unconfirmed hold, missing/outside blocker, and terminal hold refuse without changing runtime or workflow bytes.
#[test]
fn wrsv_003_03() {
    let cycle = Cycle::new("wrsv00303");
    let run = cycle.start();

    // The positive control: the confirmed hold succeeds and leaves its
    // durable pause facts before any refusal is judged.
    cycle.hold_ok(&run, 1, "intake", "runbook");

    // Unconfirmed holds refuse before anything else, one line, writing
    // nothing.
    let before = cycle.snapshot();
    let unconfirmed = cycle.hold_with_phrase(&run, Some(BLOCKER), None);
    assert_eq!(
        unconfirmed.code, 1,
        "an unconfirmed hold refuses: {unconfirmed}"
    );
    assert_eq!(
        unconfirmed.message,
        format!(
            "hold refused; hold is unconfirmed: a human must type --confirm {:?}",
            format!("hold {run}")
        ),
        "WRS-003: the unconfirmed refusal keeps today's bytes"
    );
    cycle.assert_unchanged(&before, "unconfirmed hold");

    let wrong_phrase = cycle.hold_with_phrase(&run, Some(BLOCKER), Some("hold run-999"));
    assert_eq!(wrong_phrase.code, 1);
    assert_eq!(
        wrong_phrase.message,
        format!(
            "hold refused; hold is unconfirmed: confirmation {:?} does not match the required \
             phrase {:?}",
            "hold run-999",
            format!("hold {run}")
        ),
        "WRS-003: the mismatched confirmation keeps today's bytes"
    );
    cycle.assert_unchanged(&before, "mismatched confirmation");

    // A missing or empty blocker refuses before the Run is addressed.
    let missing = cycle.hold_with_phrase(&run, None, Some(&format!("hold {run}")));
    assert_eq!(missing.code, 1);
    assert_eq!(
        missing.message,
        "hold refused; hold has no blocker link: pass --blocker <reference beneath a declared root>",
        "WRS-003: the missing blocker keeps today's bytes"
    );
    cycle.assert_unchanged(&before, "missing blocker");

    let empty = rtm(
        cycle.root(),
        &[
            "hold",
            "--run",
            &run,
            "--blocker",
            "",
            "--confirm",
            &format!("hold {run}"),
        ],
    );
    assert_eq!(empty.code, 1);
    assert_eq!(
        empty.message,
        "hold refused; hold has no blocker link: pass --blocker <reference beneath a declared root>",
        "WRS-003: the empty blocker keeps today's bytes"
    );
    cycle.assert_unchanged(&before, "empty blocker");

    // A blocker outside every declared root refuses with today's families.
    let outside = cycle.hold(&run, "notes/outside.txt");
    assert_eq!(outside.code, 1);
    assert_eq!(
        outside.message,
        "hold refused; blocker notes/outside.txt resolves outside every declared root; \
         declared roots: authority, goal, issue, residual, ticket",
        "WRS-003: the outside blocker keeps today's bytes"
    );
    cycle.assert_unchanged(&before, "outside blocker");

    let escaping = cycle.hold(&run, "../outside.txt");
    assert_eq!(escaping.code, 1);
    assert_eq!(
        escaping.message,
        "hold refused; blocker \"../outside.txt\" must stay beneath a declared root",
        "WRS-003: the escaping blocker keeps today's bytes"
    );
    cycle.assert_unchanged(&before, "escaping blocker");

    let unresolvable = cycle.hold(&run, ".arca/missing.md");
    assert_eq!(unresolvable.code, 1);
    assert!(
        unresolvable.message.starts_with(
            "hold refused; blocker .arca/missing.md does not resolve beneath a declared root: "
        ),
        "WRS-003: the unresolvable blocker keeps today's family: {unresolvable}"
    );
    cycle.assert_unchanged(&before, "unresolvable blocker");

    // A terminal Run refuses the blocked route.
    cycle.resume_ok(&run, 1, "intake");
    cycle.drive_to_ticket_turns(&run);
    let child = cycle.spawn_child(&run);
    cycle.drive_child_to_done(&child);
    let terminal = cycle.hold(&child, BLOCKER);
    assert_eq!(terminal.code, 1);
    assert_eq!(
        terminal.message,
        format!(
            "hold refused; run {child} is terminal (status passed): a blocked route may not move it"
        ),
        "WRS-003: the terminal hold keeps today's bytes"
    );
    let terminal_before = cycle.snapshot();
    let terminal_again = cycle.hold(&child, BLOCKER);
    assert_eq!(terminal_again.code, 1);
    cycle.assert_unchanged(&terminal_before, "terminal hold");
}

// --- WRSV-003-04 ---------------------------------------------------------------

/// WRSV-003-04: retain original evidence and pin bytes after each hold; prove a pre-change Run rejects drift rather than silently acquiring the new routes.
#[test]
fn wrsv_003_04() {
    let cycle = Cycle::new("wrsv00304");
    let run = cycle.start();
    cycle.step_ok(&run);
    assert_eq!(cycle.state(&run), "gap-check");

    let evidence_before = cycle.evidence_text(&run);
    let pin = cycle.runbook_pin(&run);
    assert_eq!(
        pin,
        sha256_hex(routed_runbook().as_bytes()),
        "the pin is the hash of the runbook bytes the Run started under"
    );

    cycle.hold_ok(&run, 1, "gap-check", "runbook");
    assert_eq!(
        cycle.evidence_text(&run),
        evidence_before,
        "WRS-003: the hold retains the original evidence bytes"
    );
    cycle.resume_ok(&run, 1, "gap-check");
    assert_eq!(
        cycle.evidence_text(&run),
        evidence_before,
        "WRS-003: the resume retains the original evidence bytes"
    );

    let frozen = cycle.frozen(&run);
    cycle.write_gap("missing", &frozen, &[]);
    cycle.write_verdict(&run, "gaps");
    cycle.step_ok(&run);
    assert_eq!(cycle.state(&run), "cut-tickets");
    cycle.write_ticket();
    cycle.hold_ok(&run, 2, "cut-tickets", "runbook");
    assert_eq!(
        cycle.evidence_text(&run),
        evidence_before,
        "WRS-003: every hold retains the original evidence bytes"
    );

    // Prior receipts survive hold and resume byte-identical.
    cycle.resume_ok(&run, 2, "cut-tickets");
    cycle.step_ok(&run);
    assert_eq!(cycle.state(&run), "ticket-turns");
    let child = cycle.spawn_child(&run);
    cycle.write_sensitivity(&child);
    cycle.step_ok(&child);
    assert_eq!(cycle.state(&child), "implement");
    cycle.write_completion(&child);
    cycle.commit("the green landing");
    // Prior receipts survive hold and resume byte-identical; the hold's own
    // durable facts (Run Record, pause record, history) are judged above.
    let receipts_before = cycle.receipt_snapshot(&child);
    cycle.hold_ok(&child, 1, "implement", "ticket");
    assert_eq!(
        cycle.receipt_snapshot(&child),
        receipts_before,
        "WRS-003: the hold changed receipt bytes"
    );
    cycle.resume_ok(&child, 1, "implement");
    assert_eq!(
        cycle.receipt_snapshot(&child),
        receipts_before,
        "WRS-003: the resume changed receipt bytes"
    );

    // A Run admitted before the wiring keeps its pin and refuses motion
    // against the changed bytes rather than silently acquiring the routes.
    let aged = Cycle::with_runbook("wrsv00304-base", base_runbook_bytes());
    let old = aged.start();
    assert_eq!(
        aged.runbook_pin(&old),
        BASE_RUNBOOK_SHA256,
        "the pre-change Run pins the base bytes"
    );
    aged.rewrite_runbook(&routed_runbook());
    let aged_before = aged.snapshot();
    let drifted_hold = aged.hold(&old, BLOCKER);
    assert_eq!(
        drifted_hold.code, 1,
        "the drifted hold must refuse: {drifted_hold}"
    );
    assert!(
        drifted_hold.message.contains(&format!(
            "hold refused; runbook pin mismatch: .ratmac/ratmac.toml drifted since rtm start — \
             observed sha256={}; expected sha256={BASE_RUNBOOK_SHA256}; restore the pinned \
             runbook bytes or retire the run (rtm abandon --run {old} --confirm \"abandon \
             {old}\"); nothing was modified",
            sha256_hex(routed_runbook().as_bytes())
        )),
        "WRS-003: the pre-change Run refuses the drift with today's text: {drifted_hold}"
    );
    let drifted_step = aged.step(&old);
    assert_eq!(
        drifted_step.code, 1,
        "WRS-003: the pre-change Run refuses stepped motion against the drift: {}{}",
        drifted_step.stdout, drifted_step.message
    );
    assert!(
        drifted_step.message.contains("runbook pin mismatch"),
        "WRS-003: the pre-change Run refuses stepped motion against the drift: {}{}",
        drifted_step.stdout,
        drifted_step.message
    );
    aged.assert_unchanged(&aged_before, "drifted motion");
}

// --- WRSV-003-05 ---------------------------------------------------------------

/// WRSV-003-05: for parent and child States, provide exact human resolution authorization for the current Run and recorded blocker, with a valid WRA-001 signature under a governed policy; the same address, class, State, pins, and prior receipts remain, executing status returns without advancing, history records the authorized resolution, a resumed parent can spawn, and a resumed child can complete.
#[test]
fn wrsv_003_05() {
    let _lane = support::hook_lane();
    let governed = Governed::new("wrsv00305");
    let cycle = &governed.cycle;
    let run = governed.admit();
    assert_eq!(cycle.state(&run), "intake");

    // The governed hold at the parent's intake State.
    let held = governed.hold_with(&run, BLOCKER, "w5-hold");
    assert_eq!(
        held.code, 0,
        "WRS-003: the governed hold succeeds: {}{}",
        held.stdout, held.message
    );
    assert_eq!(
        held.stdout,
        format!(
            "rtm: run {run} paused against {BLOCKER}; routed intake -> intake\n\
             The Run is not passed; a human resumes it when the blocker clears.\n"
        )
    );
    cycle.assert_pause_record(&run, 1, "intake", "runbook");

    // The exact human resolution authorization: one ratmac-resume-v1
    // signature by the designated Approver over the canonical resume intent.
    let intent = governed.resume_intent(&run, "w5-resume");
    let signature = governed.keys.sign("lead", RESUME_NAMESPACE, &intent);
    let evidence_before = cycle.evidence_text(&run);
    let resumed = governed.resume_sig(&run, BLOCKER, Some(&signature));
    assert_eq!(
        resumed.code, 0,
        "WRS-003: the authorized governed resume succeeds: {}{}",
        resumed.stdout, resumed.message
    );
    assert_eq!(
        resumed.stdout,
        format!(
            "rtm: run {run} resumed against {BLOCKER}; occurrence 1 cleared\n\
             The Run executes again at the same State; later motion still requires its normal \
             fresh checks.\n"
        )
    );
    assert_eq!(cycle.state(&run), "intake", "the resume advances no State");
    assert_eq!(cycle.status(&run), "executing");
    assert_eq!(cycle.blocker(&run), "");
    assert_eq!(
        cycle.evidence_text(&run),
        evidence_before,
        "WRS-003: the governed resume retains the evidence and pin bytes"
    );
    assert!(
        cycle.history().contains(&format!(
            "- Resume: run {run} resumed against {BLOCKER}; occurrence 1 cleared on an explicit \
             human resolution authorization.\n"
        )),
        "WRS-003: history records the authorized resolution"
    );

    // The governed resume archives one record in the shared sequence space.
    let records = governed.authorizations(&run);
    let resolution = records
        .get("000002.toml")
        .expect("the resume archives at the next sequence")
        .parse::<toml::Value>()
        .expect("the resolution record is valid TOML");
    assert_eq!(resolution["purpose"].as_str(), Some("ratmac-resume-v1"));
    assert_eq!(resolution["sequence"].as_integer(), Some(2));
    assert_eq!(resolution["from_state"].as_str(), Some("intake"));
    assert_eq!(resolution["to_state"].as_str(), Some("intake"));
    assert_eq!(resolution["occurrence"].as_integer(), Some(1));
    assert_eq!(resolution["origin"].as_str(), Some("test-injected"));
    for field in [
        "intent_sha256",
        "policy_sha256",
        "enrollment_sha256",
        "resolution_signature_sha256",
    ] {
        assert_eq!(
            resolution[field].as_str().map(str::len),
            Some(64),
            "the resolution record carries the 64-hex {field}"
        );
    }
    let signature_digest =
        sha256_hex(&fs::read(&signature).expect("read the resolution signature"));
    assert_eq!(
        resolution["resolution_signature_sha256"].as_str(),
        Some(signature_digest.as_str()),
        "the archived digest is the resolution signature's"
    );
    assert!(
        resolution.get("reviews").is_none(),
        "the resolution record carries zero review rows"
    );
    let hold_record = records
        .get("000001.toml")
        .expect("the governed hold archived first")
        .parse::<toml::Value>()
        .expect("the hold record is valid TOML");
    assert_eq!(hold_record["purpose"].as_str(), Some("hold"));

    // The resumed parent drives on and spawns a properly checked child.
    governed.step_with(&run, "w5-step1");
    assert_eq!(cycle.state(&run), "gap-check");
    let frozen = cycle.frozen(&run);
    cycle.write_gap("missing", &frozen, &[]);
    cycle.write_verdict(&run, "gaps");
    governed.step_with(&run, "w5-step2");
    assert_eq!(cycle.state(&run), "cut-tickets");
    cycle.write_ticket();
    governed.step_with(&run, "w5-step3");
    assert_eq!(cycle.state(&run), "ticket-turns");

    let child = cycle.spawn_child(&run);
    assert_eq!(cycle.state(&child), "tests");
    assert!(
        cycle.run_dir(&child).join("review.toml").is_file(),
        "the spawned child of the governed class is governed too"
    );

    // The governed hold and authorized resume of the child at its tests
    // State, through the child's own recorded class.
    let child_held = governed.hold_with(&child, BLOCKER, "w5-child-hold");
    assert_eq!(
        child_held.code, 0,
        "WRS-003: the governed child hold succeeds: {}{}",
        child_held.stdout, child_held.message
    );
    assert_eq!(
        child_held.stdout,
        format!(
            "rtm: run {child} paused against {BLOCKER}; routed tests -> tests\n\
             The Run is not passed; a human resumes it when the blocker clears.\n"
        )
    );
    cycle.assert_pause_record(&child, 1, "tests", "ticket");
    let child_intent = governed.resume_intent(&child, "w5-child-resume");
    let child_signature = governed.keys.sign("lead", RESUME_NAMESPACE, &child_intent);
    let child_resumed = governed.resume_sig(&child, BLOCKER, Some(&child_signature));
    assert_eq!(
        child_resumed.code, 0,
        "WRS-003: the authorized child resume succeeds: {}{}",
        child_resumed.stdout, child_resumed.message
    );
    assert_eq!(cycle.state(&child), "tests");
    assert_eq!(cycle.status(&child), "executing");

    // The resumed child completes through fresh normal checks.
    cycle.write_sensitivity(&child);
    governed.step_with(&child, "w5-child-step1");
    assert_eq!(cycle.state(&child), "implement");
    cycle.write_completion(&child);
    cycle.commit("the child's green landing");
    governed.step_with(&child, "w5-child-step2");
    assert_eq!(cycle.state(&child), "damage");
    cycle.write_completion(&child);
    governed.step_with(&child, "w5-child-step3");
    assert_eq!(cycle.state(&child), "done");
    assert_eq!(cycle.status(&child), "passed");
}

// --- WRSV-003-06 ---------------------------------------------------------------

/// WRSV-003-06: missing, wrong, or differently bound resolution authorization; a missing or invalid governed signature; non-held or terminal Run; changed pin; and blocker opacity each refuse with byte-identical records, receipts, ledgers, and history - and the resume route table, help, and intent forms join the public surface.
#[test]
fn wrsv_003_06() {
    let _lane = support::hook_lane();
    let cycle = Cycle::new("wrsv00306");
    let run = cycle.start();

    // The positive control: the hold leaves its pause record, so every
    // refusal twin below is judged against a genuinely held Run.
    cycle.hold_ok(&run, 1, "intake", "runbook");

    let before = cycle.snapshot();
    // Unconfirmed resumes refuse with the resume family's confirmation texts.
    let unconfirmed = cycle.resume_with(&run, Some(BLOCKER), None);
    assert_eq!(unconfirmed.code, 1, "{unconfirmed}");
    assert_eq!(
        unconfirmed.message,
        format!(
            "resume refused; resume is unconfirmed: a human must type --confirm {:?}",
            format!("resume {run}")
        )
    );
    assert_eq!(unconfirmed.message.lines().count(), 1);
    assert!(!unconfirmed.message.contains("next:"));
    cycle.assert_unchanged(&before, "unconfirmed resume");

    let wrong_phrase = cycle.resume_with(&run, Some(BLOCKER), Some("resume run-999"));
    assert_eq!(wrong_phrase.code, 1);
    assert_eq!(
        wrong_phrase.message,
        format!(
            "resume refused; resume is unconfirmed: confirmation {:?} does not match the \
             required phrase {:?}",
            "resume run-999",
            format!("resume {run}")
        )
    );
    cycle.assert_unchanged(&before, "mismatched resume phrase");

    // A missing blocker refuses.
    let missing = cycle.resume_with(&run, None, Some(&format!("resume {run}")));
    assert_eq!(missing.code, 1);
    assert_eq!(
        missing.message,
        "resume refused; resume has no blocker link: pass --blocker <reference beneath a \
         declared root>"
    );
    cycle.assert_unchanged(&before, "missing resume blocker");

    // A blocker argument that differs from the recorded blocker refuses.
    let wrong_blocker = cycle.resume(&run, ".arca/schema.md");
    assert_eq!(wrong_blocker.code, 1);
    assert_eq!(
        wrong_blocker.message,
        format!(
            "resume refused; resume blocker .arca/schema.md does not match the blocker recorded \
             for run {run}: {BLOCKER}"
        )
    );
    cycle.assert_unchanged(&before, "wrong resume blocker");

    // Argument-shape refusals mirror hold's two constructors.
    let needs_run = rtm(
        cycle.root(),
        &[
            "resume",
            "--blocker",
            BLOCKER,
            "--confirm",
            &format!("resume {run}"),
        ],
    );
    assert_eq!(needs_run.code, 1, "{needs_run}");
    assert!(
        needs_run
            .message
            .starts_with("resume refused; resume requires --run <id>; runs: "),
        "WRS-003: the missing --run refusal carries the roster: {needs_run}"
    );
    assert!(needs_run.message.contains(&run));
    cycle.assert_unchanged(&before, "resume without --run");

    let dangling_run = rtm(
        cycle.root(),
        &[
            "resume",
            "--run",
            "--blocker",
            BLOCKER,
            "--confirm",
            &format!("resume {run}"),
        ],
    );
    assert_eq!(dangling_run.code, 2, "{dangling_run}");
    assert!(
        dangling_run
            .message
            .starts_with("resume: --run needs a run id; "),
        "WRS-003: the dangling --run value is a usage refusal: {dangling_run}"
    );
    cycle.assert_unchanged(&before, "resume with a dangling --run");

    let unknown = rtm(
        cycle.root(),
        &[
            "resume",
            "--run",
            "run-999",
            "--blocker",
            BLOCKER,
            "--confirm",
            "resume run-999",
        ],
    );
    assert_eq!(unknown.code, 1, "{unknown}");
    assert!(
        unknown.message.starts_with(
            "resume refused; run id \"run-999\" is not an exact roster member; runs: "
        ),
        "WRS-003: the unknown id refusal carries the roster: {unknown}"
    );
    cycle.assert_unchanged(&before, "resume with an unknown id");

    let stray = rtm(
        cycle.root(),
        &[
            "resume",
            "--run",
            &run,
            "--blocker",
            BLOCKER,
            "--confirm",
            &format!("resume {run}"),
            "stray",
        ],
    );
    assert_eq!(stray.code, 1, "{stray}");
    assert_eq!(
        stray.message, "resume: unexpected extra argument stray",
        "WRS-003: the extra argument refusal mirrors hold's"
    );
    cycle.assert_unchanged(&before, "resume with an extra argument");

    // A resolution signature addresses nothing on an ungoverned Run.
    let resolution = rtm(
        cycle.root(),
        &[
            "resume",
            "--run",
            &run,
            "--blocker",
            BLOCKER,
            "--confirm",
            &format!("resume {run}"),
            "--resolution",
            "nowhere.sig",
        ],
    );
    assert_eq!(resolution.code, 2, "{resolution}");
    assert_eq!(
        resolution.message, "resume: --resolution addresses no declared review policy",
        "WRS-003: an ungoverned --resolution is a usage refusal"
    );
    cycle.assert_unchanged(&before, "ungoverned --resolution");

    // A non-held Run refuses; a terminal Run refuses.
    let other = cycle.start();
    let not_held_before = cycle.snapshot();
    let not_held = cycle.resume(&other, BLOCKER);
    assert_eq!(not_held.code, 1, "{not_held}");
    assert_eq!(
        not_held.message,
        format!("resume refused; run {other} is not held (status planned); nothing to resume")
    );
    cycle.assert_unchanged(&not_held_before, "resume of a non-held Run");

    cycle.drive_to_ticket_turns(&other);
    let child = cycle.spawn_child(&other);
    cycle.drive_child_to_done(&child);
    let terminal_before = cycle.snapshot();
    let terminal = cycle.resume(&child, BLOCKER);
    assert_eq!(terminal.code, 1, "{terminal}");
    assert_eq!(
        terminal.message,
        format!("resume refused; run {child} is terminal (status passed): nothing to resume")
    );
    cycle.assert_unchanged(&terminal_before, "resume of a terminal Run");

    // A held Run whose recorded State its owning class does not declare
    // refuses at class resolution, today's undeclared-State family, before
    // any record check.
    let wrong_class = cycle.start();
    let wrong_held = cycle.hold(&wrong_class, BLOCKER);
    assert_eq!(
        wrong_held.code, 0,
        "WRS-003: the wrong-class fixture holds first: {}{}",
        wrong_held.stdout, wrong_held.message
    );
    let wrong_record = cycle.run_dir(&wrong_class).join("run.toml");
    fs::write(
        &wrong_record,
        cycle
            .record_text(&wrong_class)
            .replacen("state = \"intake\"", "state = \"implement\"", 1),
    )
    .expect("plant the undeclared State in the Run Record");
    let wrong_before = cycle.snapshot();
    let wrong = cycle.resume(&wrong_class, BLOCKER);
    assert_eq!(wrong.code, 1, "{wrong}");
    assert_eq!(
        wrong.message,
        "resume refused; Run Record state \"implement\" is undeclared in the top-level class",
        "WRS-003: the wrong-class resume refuses with today's undeclared-State family"
    );
    cycle.assert_unchanged(&wrong_before, "resume of a wrong-class Run");

    // A held Run with no recorded pause occurrence admits no resume: legacy
    // Runs are never silently upgraded.
    fs::remove_dir_all(cycle.run_dir(&run).join("pauses"))
        .expect("remove the pause records to fake a legacy hold");
    let legacy = cycle.resume(&run, BLOCKER);
    assert_eq!(legacy.code, 1, "{legacy}");
    assert_eq!(
        legacy.message,
        format!(
            "resume refused; run {run} is held with no recorded pause occurrence; a hold from \
             before occurrences were recorded admits no resume"
        )
    );

    // The one public route table gains resume after intent.
    let usage = rtm(cycle.root(), &[]);
    assert_eq!(usage.code, 0);
    assert!(
        usage.stdout.contains(
            "Commands: init, start, status, step, hold, intent, resume, abandon, \
                       spawn, respawn, doctor, scaffold, skill\n"
        ),
        "WRS-003: the general usage's Commands line gains resume after intent: {}",
        usage.stdout
    );

    let resume_help = rtm(cycle.root(), &["resume", "--help"]);
    assert_eq!(resume_help.code, 0, "{}", resume_help.message);
    assert!(
        resume_help.stdout.starts_with(
            "Usage: rtm resume --run <id> --blocker <reference beneath a declared root> \
             --confirm \"resume <id>\" [--resolution <sig>] [--help]\n"
        ),
        "WRS-003: resume's help prints its usage form: {}",
        resume_help.stdout
    );
    assert!(
        resume_help.stdout.contains("ratmac-resume-v1"),
        "WRS-003: resume's help names the signature namespace"
    );
    assert!(
        resume_help
            .stdout
            .contains("(rtm intent --run <id> --resume)"),
        "WRS-003: resume's help names the canonical intent verb"
    );

    let intent_help = rtm(cycle.root(), &["intent", "--help"]);
    assert_eq!(intent_help.code, 0);
    assert!(
        intent_help.stdout.contains(
            "Usage: rtm intent --run <id> [--hold --blocker <ref> | --resume] \
             [--review <label>:<sig>]... --out <path> [--help]"
        ),
        "WRS-003: intent's usage line grows the --resume form: {}",
        intent_help.stdout
    );

    // The two intent purposes refuse to be named together.
    let both = rtm(
        cycle.root(),
        &[
            "intent",
            "--run",
            &run,
            "--hold",
            "--blocker",
            BLOCKER,
            "--resume",
            "--out",
            "both.bin",
        ],
    );
    assert_eq!(both.code, 1, "{both}");
    assert_eq!(
        both.message,
        "intent refused; --hold and --resume name different intents; pass exactly one"
    );

    // An ungoverned Run keeps t-129's ungoverned intent refusal.
    let ungoverned = rtm(
        cycle.root(),
        &["intent", "--run", &other, "--out", "plain.bin"],
    );
    assert_eq!(ungoverned.code, 1, "{ungoverned}");
    assert_eq!(
        ungoverned.message,
        format!(
            "intent refused; run {other} resolves in no review-governed class; the canonical \
             intent exists only under a declared review policy"
        )
    );

    // Under a declared review policy the governed boundary refuses honestly.
    let governed = Governed::new("wrsv00306b");
    let governed_run = governed.admit();
    let held = governed.hold_with(&governed_run, BLOCKER, "w6-hold");
    assert_eq!(held.code, 0, "{}{}", held.stdout, held.message);
    governed
        .cycle
        .assert_pause_record(&governed_run, 1, "intake", "runbook");
    let governed_before = governed.cycle.snapshot();

    let no_signature = governed.resume_sig(&governed_run, BLOCKER, None);
    assert_eq!(no_signature.code, 1, "{no_signature}");
    assert_eq!(
        no_signature.message,
        format!(
            "resume refused; authorization: resolution: observed none; expected one \
             ratmac-resume-v1 signature by the designated approver over the canonical intent \
             (rtm intent --run {governed_run} --resume)"
        )
    );
    assert_eq!(no_signature.message.lines().count(), 1);
    governed
        .cycle
        .assert_unchanged(&governed_before, "governed resume without a signature");

    // A signature by an enrolled member who is not the Approver refuses.
    let member_intent = governed.resume_intent(&governed_run, "w6-member");
    let member_signature = governed.keys.sign("a", RESUME_NAMESPACE, &member_intent);
    let member = governed.resume_sig(&governed_run, BLOCKER, Some(&member_signature));
    assert_eq!(member.code, 1, "{member}");
    assert_eq!(
        member.message,
        "resume refused; authorization: resolution: observed a signature by member \"a\"; \
         expected the designated approver \"lead\""
    );
    governed
        .cycle
        .assert_unchanged(&governed_before, "governed resume by a member");

    // A changed runbook pin refuses the resume before authorization.
    governed
        .cycle
        .rewrite_runbook(&format!("{}\n# drifted\n", routed_runbook()));
    let drifted = governed.resume_sig(&governed_run, BLOCKER, Some(&member_signature));
    assert_eq!(drifted.code, 1, "{drifted}");
    assert!(
        drifted.message.contains(
            "resume refused; runbook pin mismatch: .ratmac/ratmac.toml drifted since \
                       rtm start"
        ),
        "WRS-003: a changed pin refuses the resume: {drifted}"
    );
    assert!(drifted.message.contains("nothing was modified"));

    // Blocker opacity: edited prose and a removed file never substitute for
    // authorization, and the same confirmation still resumes despite them.
    let opaque = cycle.start();
    cycle.hold_ok(&opaque, 1, "intake", "runbook");
    cycle.write(
        BLOCKER,
        "The prose changed; the blocker's identity did not.\n",
    );
    cycle.resume_ok(&opaque, 1, "intake");

    let removed = cycle.start();
    cycle.hold_ok(&removed, 1, "intake", "runbook");
    fs::remove_file(cycle.root().join(BLOCKER)).expect("remove the blocker file");
    cycle.resume_ok(&removed, 1, "intake");
}

// --- WRSV-003-07 ---------------------------------------------------------------

/// WRSV-003-07: source changes while paused leave old completion receipts stale after authorized resume; completion refuses until fresh proof exists, and an accepted rule-fix amendment alone leaves the Run blocked and cannot authorize resume or a transition.
#[test]
fn wrsv_003_07() {
    let cycle = Cycle::new("wrsv00307");
    let run = cycle.start();
    cycle.drive_to_ticket_turns(&run);
    let child = cycle.spawn_child(&run);
    cycle.write_sensitivity(&child);
    cycle.step_ok(&child);
    assert_eq!(cycle.state(&child), "implement");
    cycle.write_completion(&child);
    cycle.commit("the green landing");

    // The pause at implement, then a source change while paused.
    cycle.hold_ok(&child, 1, "implement", "ticket");
    cycle.write("src/lib.rs", "pub fn work() { changed while paused }\n");
    cycle.resume_ok(&child, 1, "implement");

    let stale = cycle.step(&child);
    assert_eq!(stale.code, 0, "{}{}", stale.stdout, stale.message);
    assert!(
        stale.stdout.contains("step refused"),
        "WRS-003: the stale receipt refuses the step: {}",
        stale.stdout
    );
    assert!(
        stale.stdout.contains("receipt is stale"),
        "WRS-003: the refusal names the stale receipt: {}",
        stale.stdout
    );
    assert_eq!(cycle.state(&child), "implement");
    assert_eq!(cycle.status(&child), "executing");

    // Fresh proof exists once the receipts are re-recorded against the tree.
    cycle.write_completion(&child);
    cycle.commit("the fresh green landing");
    cycle.step_ok(&child);
    assert_eq!(cycle.state(&child), "damage");
    cycle.write_completion(&child);
    cycle.step_ok(&child);
    assert_eq!(cycle.state(&child), "done");
    assert_eq!(cycle.status(&child), "passed");

    // An accepted rule-fix amendment alone leaves the Run blocked and
    // authorizes neither resume nor transition.
    cycle.step_ok(&run);
    assert_eq!(cycle.state(&run), "close");
    cycle.hold_ok(&run, 1, "close", "runbook");
    let blocked_record = cycle.record_text(&run);
    cycle.write(
        ".arca/amendment-res-185.md",
        "# Accepted authority amendment\n\nThe pinned rule itself changed; the recovery was \
         accepted under the rule-fix contract.\n",
    );
    assert_eq!(
        cycle.record_text(&run),
        blocked_record,
        "WRS-003: the accepted amendment alone changes no Run Record byte"
    );
    assert_eq!(cycle.status(&run), "blocked");

    let amended_step = cycle.step(&run);
    assert_eq!(
        amended_step.code, 0,
        "{}{}",
        amended_step.stdout, amended_step.message
    );
    assert!(
        amended_step.stdout.contains(&format!(
            "step refused; held: observed run {run} is paused against {BLOCKER} (status \
             blocked); expected a resumed Run (rtm resume --run {run})"
        )),
        "WRS-003: a held Run refuses step before guard evaluation: {}",
        amended_step.stdout
    );
    assert!(
        amended_step
            .stdout
            .contains(&format!("next: rtm step --run {run}\n")),
        "WRS-003: the held refusal keeps today's next line: {}",
        amended_step.stdout
    );

    let unconfirmed = cycle.resume_with(&run, Some(BLOCKER), None);
    assert_eq!(unconfirmed.code, 1, "{unconfirmed}");
    assert!(unconfirmed.message.contains("resume is unconfirmed"));
    assert_eq!(
        cycle.record_text(&run),
        blocked_record,
        "WRS-003: the refused resume changes no Run Record byte"
    );

    cycle.resume_ok(&run, 1, "close");
}

// --- WRSV-003-08 ---------------------------------------------------------------

/// WRSV-003-08: inject interruption before and after hold/resume journal publication and race competing motion; a mutating retry finishes the committed action exactly once, readers report pending reconciliation without repairing, and the public Run Record keeps its seven fields.
#[test]
fn wrsv_003_08() {
    let _lane = support::hook_lane();
    let cycle = Cycle::new("wrsv00308");
    let run = cycle.start();
    let marker = cycle.root().join("qa/hold-marker.md");
    let release = cycle.root().join("qa/hold-release.md");
    fs::create_dir_all(cycle.root().join("qa")).expect("create the barrier directory");

    // An interruption before publication refuses with today's barrier text
    // and leaves every persisted byte unchanged.
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER", "before-journal-publication");
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_MARKER", &marker);
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_RELEASE", &release);
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS", "1");
    let before = cycle.snapshot();
    let refused = cycle.hold(&run, BLOCKER);
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_MARKER");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_RELEASE");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS");
    assert_eq!(
        refused.code, 1,
        "WRS-003: a graceful failure before publication refuses: {}{}",
        refused.stdout, refused.message
    );
    assert_eq!(
        refused.message,
        format!(
            "hold refused; hold test barrier expired before before-journal-publication waiting \
             for {}",
            shown(&release)
        ),
        "WRS-003: the before-publication timeout keeps today's refusal bytes"
    );
    cycle.assert_unchanged(&before, "interrupted hold before publication");

    // An interruption after publication reports the committed action
    // honestly and leaves the journal entry for recovery.
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER", "after-journal-publication");
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_MARKER", &marker);
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_RELEASE", &release);
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS", "1");
    let committed = cycle.hold(&run, BLOCKER);
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS");
    assert_eq!(
        committed.code, 1,
        "WRS-003: a graceful failure after publication reports the commit: {}{}",
        committed.stdout, committed.message
    );
    assert_eq!(
        committed.message,
        format!(
            "hold: committed with recovery pending: hold test barrier expired before \
             after-journal-publication waiting for {}; a later mutating command finishes it \
             exactly once",
            shown(&release)
        ),
        "WRS-003: the post-publication failure reports committed with recovery pending"
    );
    let journal = cycle.run_dir(&run).join("journal");
    assert!(
        journal.join("hold-1.committed").is_file(),
        "WRS-003: the committed entry holds the whole pause action"
    );
    assert_eq!(
        cycle.status(&run),
        "planned",
        "WRS-003: the interrupted hold has not yet written its Run Record"
    );
    assert!(!cycle.pause_path(&run, 1).is_file());

    // Readers report, never repair: status appends one line after next.
    let status = rtm(cycle.root(), &["status", "--run", &run]);
    assert_eq!(status.code, 0, "{}", status.message);
    let recovery_line = "Recovery pending: a committed pause action for this Run awaits \
                         reconciliation; the next mutating command on this Run finishes it \
                         exactly once.";
    assert!(
        status.stdout.contains(recovery_line),
        "WRS-003: status reports the pending reconciliation: {}",
        status.stdout
    );
    let next_position = status
        .stdout
        .find(&format!("next: rtm step --run {run}"))
        .expect("status renders its next line");
    let recovery_position = status
        .stdout
        .find(recovery_line)
        .expect("status renders the recovery line");
    assert!(
        next_position < recovery_position,
        "WRS-003: the recovery line follows the next line"
    );
    assert!(
        journal.join("hold-1.committed").is_file(),
        "WRS-003: the status reader repaired the pending action"
    );

    // The doctor reports one warning finding and exits 1.
    let doctor = rtm(cycle.root(), &["doctor"]);
    assert_eq!(
        doctor.code, 1,
        "WRS-003: doctor warns on the pending reconciliation: {}{}",
        doctor.stdout, doctor.message
    );
    let engine_root = shown(&cycle.root().join(".ratmac"));
    assert!(
        doctor.stdout.contains(&format!(
            "recovery pending: {engine_root}/runs/{run}/journal/ holds a committed pause \
             action awaiting reconciliation"
        )),
        "WRS-003: the doctor warning names the journal: {}",
        doctor.stdout
    );
    assert!(
        doctor.stdout.contains(&format!(
            "RB901 warning: run {run:?}: recovery pending: {engine_root}/runs/{run}/journal/ \
             holds a committed pause action awaiting reconciliation"
        )),
        "WRS-003: the doctor finding carries the pinned RB901 code and run location: {}",
        doctor.stdout
    );
    assert!(
        journal.join("hold-1.committed").is_file(),
        "WRS-003: the doctor repaired the pending action"
    );

    // A retried mutating command whose action matches the pending entry -
    // the same Run, blocker, and phrase, and no other command reconciled it
    // - finishes the committed hold exactly once and reports the ordinary
    // success: the pending entry is the discriminator.
    let history_before_retry = cycle.history();
    let retry = cycle.hold(&run, BLOCKER);
    assert_eq!(
        retry.code, 0,
        "WRS-003: the same-argument retry with its entry pending finishes the hold: {}{}",
        retry.stdout, retry.message
    );
    assert_eq!(
        retry.stdout,
        format!(
            "rtm: run {run} paused against {BLOCKER}; routed intake -> intake\n\
             The Run is not passed; a human resumes it when the blocker clears.\n"
        )
    );
    cycle.assert_pause_record(&run, 1, "intake", "runbook");
    cycle.assert_seven_fields(&run);
    assert_eq!(
        fs::read_dir(&journal)
            .map(|entries| entries.count())
            .unwrap_or(0),
        0,
        "WRS-003: the finished retry leaves the journal empty"
    );
    let hold_lines = cycle
        .history()
        .lines()
        .filter(|line| line.contains(&format!("- Hold: run {run} paused against {BLOCKER};")))
        .count();
    assert_eq!(
        hold_lines, 1,
        "WRS-003: the finished hold is recorded exactly once"
    );
    assert!(
        cycle.history().starts_with(&history_before_retry),
        "WRS-003: recovery never rewrites history"
    );

    // The different-command branch: a second post-publication interruption
    // whose entry a competing mutating command reconciles first.
    cycle.resume_ok(&run, 1, "intake");
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER", "after-journal-publication");
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_MARKER", &marker);
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_RELEASE", &release);
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS", "1");
    let interrupted_hold = cycle.hold(&run, BLOCKER);
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_MARKER");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_RELEASE");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS");
    assert_eq!(
        interrupted_hold.code, 1,
        "WRS-003: the second hold reports its commit: {}{}",
        interrupted_hold.stdout, interrupted_hold.message
    );
    assert!(
        interrupted_hold
            .message
            .starts_with("hold: committed with recovery pending: "),
        "WRS-003: the second hold's failure reports its commit: {interrupted_hold}"
    );
    assert!(
        journal.join("hold-2.committed").is_file(),
        "WRS-003: the second hold's committed entry names its occurrence"
    );

    // The competing mutating command reconciles first and then refuses
    // honestly against the finished state.
    let raced = cycle.step(&run);
    assert_eq!(raced.code, 0, "{}{}", raced.stdout, raced.message);
    assert!(
        raced.stdout.contains(&format!(
            "step refused; held: observed run {run} is paused against {BLOCKER} (status \
             blocked); expected a resumed Run (rtm resume --run {run})"
        )),
        "WRS-003: the reconciling command refuses honestly once the hold is finished: {}",
        raced.stdout
    );
    assert_eq!(cycle.status(&run), "blocked");
    assert!(
        !journal.join("hold-2.committed").is_file(),
        "WRS-003: reconciliation removed the committed entry"
    );
    cycle.assert_pause_record(&run, 2, "intake", "runbook");

    // A fresh hold after a reconciled hold refuses the already-blocked text:
    // once the entry is gone the durable state is complete, so the journal
    // entry's presence is what distinguishes a matching retry from a fresh
    // hold.
    let reconciled_before = cycle.snapshot();
    let fresh = cycle.hold(&run, BLOCKER);
    assert_eq!(
        fresh.code, 1,
        "WRS-003: a fresh hold after a reconciled hold refuses: {}{}",
        fresh.stdout, fresh.message
    );
    assert_eq!(
        fresh.message,
        format!("hold refused; run {run} is already blocked against {BLOCKER}"),
        "WRS-003: the fresh hold keeps today's already-blocked text"
    );
    cycle.assert_unchanged(&reconciled_before, "fresh hold after reconciliation");
    let hold_lines = cycle
        .history()
        .lines()
        .filter(|line| line.contains(&format!("- Hold: run {run} paused against {BLOCKER};")))
        .count();
    assert_eq!(
        hold_lines, 2,
        "WRS-003: each finished hold is recorded exactly once"
    );
    cycle.assert_pause_record(&run, 2, "intake", "runbook");

    // The same boundary serves resume identically.
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER", "after-journal-publication");
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS", "1");
    let resume_committed = cycle.resume(&run, BLOCKER);
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_MARKER");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_RELEASE");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS");
    assert_eq!(
        resume_committed.code, 1,
        "WRS-003: the interrupted resume reports its commit: {}{}",
        resume_committed.stdout, resume_committed.message
    );
    assert!(
        resume_committed
            .message
            .starts_with("resume: committed with recovery pending: "),
        "WRS-003: the resume's post-publication failure names resume: {resume_committed}"
    );
    assert!(
        resume_committed
            .message
            .contains("expired before after-journal-publication"),
        "WRS-003: the resume's recovery line carries today's barrier detail: {resume_committed}"
    );
    assert!(
        resume_committed
            .message
            .contains("a later mutating command finishes it exactly once"),
        "WRS-003: the resume's recovery line teaches the finishing command: {resume_committed}"
    );
    assert!(
        journal.join("resume-2.committed").is_file(),
        "WRS-003: the resume's committed entry names its action"
    );
    assert_eq!(cycle.status(&run), "blocked");
    let resume_retry = cycle.resume(&run, BLOCKER);
    assert_eq!(
        resume_retry.code, 0,
        "WRS-003: the retried resume finishes exactly once: {}{}",
        resume_retry.stdout, resume_retry.message
    );
    assert_eq!(cycle.status(&run), "executing");
    assert_eq!(
        fs::read_dir(&journal)
            .map(|entries| entries.count())
            .unwrap_or(0),
        0,
        "WRS-003: the finished resume leaves the journal empty"
    );

    // The same boundary serves resume before publication: today's barrier
    // refusal bytes under the resume family, writing nothing.
    let reheld = cycle.hold(&run, BLOCKER);
    assert_eq!(
        reheld.code, 0,
        "WRS-003: the third hold for the resume barrier twin succeeds: {}{}",
        reheld.stdout, reheld.message
    );
    let barrier_before = cycle.snapshot();
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER", "before-journal-publication");
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_MARKER", &marker);
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_RELEASE", &release);
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS", "1");
    let refused_resume = cycle.resume(&run, BLOCKER);
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_MARKER");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_RELEASE");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS");
    assert_eq!(
        refused_resume.code, 1,
        "WRS-003: a graceful failure before the resume's publication refuses: {}{}",
        refused_resume.stdout, refused_resume.message
    );
    assert_eq!(
        refused_resume.message,
        format!(
            "resume refused; hold test barrier expired before before-journal-publication \
             waiting for {}",
            shown(&release)
        ),
        "WRS-003: the resume's pre-publication timeout reuses today's barrier bytes"
    );
    cycle.assert_unchanged(&barrier_before, "interrupted resume before publication");
    cycle.resume_ok(&run, 3, "intake");

    // A race: competing motion under the publication barrier contends on the
    // addressed Run lock and refuses honestly, and the released barrier
    // completes the hold exactly once.
    let race_cycle = Cycle::new("wrsv00308-race");
    let race_run = race_cycle.start();
    let race_marker = race_cycle.root().join("qa/race-marker.md");
    let race_release = race_cycle.root().join("qa/race-release.md");
    fs::create_dir_all(race_cycle.root().join("qa")).expect("create the race directory");
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER", "after-journal-publication");
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_MARKER", &race_marker);
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_RELEASE", &race_release);
    // The holder must outlast the competing step's 5 s lock wait, so the
    // barrier's timeout is explicit rather than the default.
    std::env::set_var("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS", "20000");
    let race_root = race_cycle.root().to_path_buf();
    let race_id = race_run.clone();
    let race_blocker = BLOCKER.to_owned();
    let holder = std::thread::spawn(move || {
        let mut full = vec![
            "rtm".to_owned(),
            "hold".to_owned(),
            "--run".to_owned(),
            race_id,
            "--blocker".to_owned(),
            race_blocker,
        ];
        full.push("--confirm".to_owned());
        full.push(format!("hold {}", full[3]));
        let mut stdout = Vec::new();
        let result = match cli::run_from(full, &race_root, &mut stdout) {
            Ok(code) => Ran {
                code,
                stdout: String::from_utf8_lossy(&stdout).into_owned(),
                message: String::new(),
            },
            Err(error) => Ran {
                code: error.exit_code(),
                stdout: String::from_utf8_lossy(&stdout).into_owned(),
                message: error.to_string(),
            },
        };
        result
    });
    let deadline = Instant::now() + Duration::from_secs(4);
    while !race_marker.is_file() && Instant::now() < deadline && !holder.is_finished() {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        race_marker.is_file(),
        "WRS-003: the hold waits at the after-journal-publication boundary while it owns the \
         Run lock"
    );
    let competing = rtm(race_cycle.root(), &["step", "--run", &race_run]);
    assert_eq!(
        competing.code, 1,
        "{}{}",
        competing.stdout, competing.message
    );
    assert!(
        competing.message.contains("lock wait expired")
            || competing.stdout.contains("lock wait expired"),
        "WRS-003: the competing motion contends on the Run lock: {}{}",
        competing.stdout,
        competing.message
    );
    fs::write(&race_release, "released\n").expect("release the barrier");
    let held = holder.join().expect("the holding thread finished");
    assert_eq!(
        held.code, 0,
        "WRS-003: the released hold completes: {}{}",
        held.stdout, held.message
    );
    assert_eq!(race_cycle.status(&race_run), "blocked");
    assert!(race_cycle.pause_path(&race_run, 1).is_file());
    assert_eq!(
        fs::read_dir(race_cycle.run_dir(&race_run).join("journal"))
            .map(|entries| entries.count())
            .unwrap_or(0),
        0,
        "WRS-003: the completed race hold leaves the journal empty"
    );
    let race_holds = race_cycle
        .history()
        .lines()
        .filter(|line| line.contains(&format!("- Hold: run {race_run} paused against")))
        .count();
    assert_eq!(race_holds, 1, "WRS-003: the race hold is recorded once");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_MARKER");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_RELEASE");
    std::env::remove_var("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS");

    // t-128's per-failure rule: the refused held step emits exactly one
    // refusal-code record for the new `held` kind - `run`, never a `path` -
    // inside today's refused-step sequence, and the toggle unset emits no
    // record bytes at all. The binary is driven directly because trace
    // records go to the process's own stderr.
    let mut traced = support::command(ratmac_qa::engine_bin!(), race_cycle.root());
    traced
        .args(["step", "--run", &race_run])
        .env("RATMAC_TRACE", "1");
    let traced = traced.output().expect("invoke the traced refused step");
    assert_eq!(
        traced.status.code(),
        Some(0),
        "WRS-003: the traced held refusal is the ordinary refused step: {}",
        String::from_utf8_lossy(&traced.stderr)
    );
    let stderr_text = String::from_utf8_lossy(&traced.stderr).into_owned();
    let records: Vec<&str> = stderr_text
        .split('\n')
        .filter_map(|line| line.strip_prefix("rtm-trace: "))
        .collect();
    let held_record = format!("code=\"held\" run=\"{race_run}\"");
    assert_eq!(
        records
            .iter()
            .filter(|record| **record == held_record)
            .count(),
        1,
        "WRS-003: exactly one held refusal-code record, spelled t-128's way: {records:?}"
    );
    let held_at = records
        .iter()
        .position(|record| *record == held_record)
        .expect("the held record was just proven present");
    assert!(
        held_at >= 1
            && records[held_at - 1]
                == format!("code=\"lock_acquire\" domain=\"run\" run=\"{race_run}\""),
        "WRS-003: the held record fires under the Run lock inside today's refused-step \
         sequence: {records:?}"
    );
    assert_eq!(
        records.get(held_at + 1).copied(),
        Some(format!("code=\"lock_release\" domain=\"run\" run=\"{race_run}\"").as_str()),
        "WRS-003: the Run lock releases after the held record: {records:?}"
    );
    let untraced = support::command(ratmac_qa::engine_bin!(), race_cycle.root())
        .args(["step", "--run", &race_run])
        .output()
        .expect("invoke the untraced refused step");
    assert_eq!(untraced.status.code(), Some(0));
    assert_eq!(
        untraced.stdout, traced.stdout,
        "WRS-003: tracing adds records to stderr and changes nothing else"
    );
    assert!(
        !String::from_utf8_lossy(&untraced.stderr).contains("rtm-trace: "),
        "WRS-003: the toggle unset emits no record bytes"
    );

    // An unparseable committed entry refuses the next mutating command
    // having modified nothing.
    let garbage = Cycle::new("wrsv00308-unparseable");
    let garbage_run = garbage.start();
    garbage.hold_ok(&garbage_run, 1, "intake", "runbook");
    let entry = garbage
        .run_dir(&garbage_run)
        .join("journal/hold-2.committed");
    fs::create_dir_all(entry.parent().expect("the entry has a parent directory"))
        .expect("create the journal directory");
    fs::write(&entry, "not a strict document\n{\n").expect("plant the unparseable entry");
    let garbage_before = garbage.snapshot();
    let unparseable = garbage.step(&garbage_run);
    assert_eq!(unparseable.code, 1, "{unparseable}");
    assert_eq!(
        unparseable.message,
        format!(
            "step: the pause journal entry {} does not parse; nothing was modified\nnext: \
             rtm doctor",
            shown(&entry)
        ),
        "WRS-003: the unparseable committed entry refuses with its pinned text and \
         today's teaching line"
    );
    garbage.assert_unchanged(&garbage_before, "unparseable committed entry");
}

// --- WRSV-003-09 ---------------------------------------------------------------

/// WRSV-003-09: hold, validly resume, and hold again at the identical State and blocker; the second occurrence strictly increases, the first ratmac-resume-v1 signature fails, changed digest, context, or occurrence fails independently, and a fresh exactly bound resume signature passes.
#[test]
fn wrsv_003_09() {
    let _lane = support::hook_lane();
    let governed = Governed::new("wrsv00309");
    let cycle = &governed.cycle;
    let run = governed.admit();

    // The first hold and authorized resume.
    let held = governed.hold_with(&run, BLOCKER, "w9-hold1");
    assert_eq!(held.code, 0, "{}{}", held.stdout, held.message);
    cycle.assert_pause_record(&run, 1, "intake", "runbook");
    let intent_one = governed.resume_intent(&run, "w9-resume1");
    let signature_one = governed.keys.sign("lead", RESUME_NAMESPACE, &intent_one);
    let resumed = governed.resume_sig(&run, BLOCKER, Some(&signature_one));
    assert_eq!(
        resumed.code, 0,
        "WRS-003: the first authorized resume succeeds: {}{}",
        resumed.stdout, resumed.message
    );
    assert_eq!(cycle.status(&run), "executing");

    // The second hold at the identical State and blocker: the occurrence
    // strictly increases and the first record is never rewritten.
    let first_record =
        fs::read_to_string(cycle.pause_path(&run, 1)).expect("read the first pause record");
    let held_again = governed.hold_with(&run, BLOCKER, "w9-hold2");
    assert_eq!(
        held_again.code, 0,
        "{}{}",
        held_again.stdout, held_again.message
    );
    assert_eq!(
        held_again.stdout,
        format!(
            "rtm: run {run} paused against {BLOCKER}; routed intake -> intake\n\
             The Run is not passed; a human resumes it when the blocker clears.\n"
        )
    );
    cycle.assert_pause_record(&run, 2, "intake", "runbook");
    assert_eq!(
        fs::read_to_string(cycle.pause_path(&run, 1)).expect("reread the first record"),
        first_record,
        "WRS-003: the strictly increasing occurrence never rewrites history"
    );
    let occurrences: Vec<String> = fs::read_dir(cycle.run_dir(&run).join("pauses"))
        .expect("read the pauses directory")
        .map(|entry| {
            entry
                .expect("read a pause entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(
        occurrences,
        vec!["1.toml".to_owned(), "2.toml".to_owned()],
        "WRS-003: the second hold mints occurrence 2"
    );

    let intent_two = governed.resume_intent(&run, "w9-resume2");
    assert_ne!(
        sha256_hex(&fs::read(&intent_one).expect("read the first intent")),
        sha256_hex(&fs::read(&intent_two).expect("read the second intent")),
        "WRS-003: the canonical resume intent binds the live occurrence and held-record digest"
    );

    let before = cycle.snapshot();
    // A signature over the wrong bytes refuses. The stale payload lives in
    // the fixture keys tree, so signing it leaves the fixture untouched.
    governed
        .keys
        .dir
        .write("stale-payload.bin", routed_runbook().as_bytes());
    let stale_payload = governed.keys.dir.join("stale-payload.bin");
    let stale_signature = governed.keys.sign("lead", RESUME_NAMESPACE, &stale_payload);
    let wrong_bytes = governed.resume_sig(&run, BLOCKER, Some(&stale_signature));
    assert_eq!(wrong_bytes.code, 1, "{wrong_bytes}");
    assert!(
        wrong_bytes.message.contains(
            "resume refused; authorization: resolution: observed a signature that does not \
                 verify against the recomputed canonical intent sha256 ",
        ),
        "WRS-003: a signature over the wrong bytes refuses: {wrong_bytes}"
    );
    assert!(wrong_bytes
        .message
        .contains(&format!("(rtm intent --run {run} --resume)")));
    cycle.assert_unchanged(&before, "resume with a wrong-bytes signature");

    // The first resolution signature fails the second time: it is consumed.
    let replayed = governed.resume_sig(&run, BLOCKER, Some(&signature_one));
    assert_eq!(replayed.code, 1, "{replayed}");
    assert_eq!(
        replayed.message,
        format!(
            "resume refused; authorization: replay: observed the resolution already consumed as \
             sequence 2 under {}/runs/{run}/authorizations/; expected a fresh authorization at \
             sequence 3",
            shown(&cycle.root().join(".ratmac"))
        ),
        "WRS-003: the replayed resolution signature refuses with the consuming record's sequence \
         plus one"
    );
    cycle.assert_unchanged(&before, "replayed resolution signature");

    // A signature by an enrolled member refuses; one by no enrolled identity
    // refuses; a fresh exactly bound signature by the Approver passes.
    let member_signature = governed.keys.sign("a", RESUME_NAMESPACE, &intent_two);
    let member = governed.resume_sig(&run, BLOCKER, Some(&member_signature));
    assert_eq!(member.code, 1, "{member}");
    assert_eq!(
        member.message,
        "resume refused; authorization: resolution: observed a signature by member \"a\"; \
         expected the designated approver \"lead\""
    );
    cycle.assert_unchanged(&before, "member resolution signature");

    let outsider_signature = governed
        .keys
        .sign("outsider", RESUME_NAMESPACE, &intent_two);
    let outsider = governed.resume_sig(&run, BLOCKER, Some(&outsider_signature));
    assert_eq!(outsider.code, 1, "{outsider}");
    assert_eq!(
        outsider.message,
        "resume refused; authorization: resolution: observed a signature by no enrolled \
         identity; expected the designated approver \"lead\""
    );
    cycle.assert_unchanged(&before, "outsider resolution signature");

    let signature_two = governed.keys.sign("lead", RESUME_NAMESPACE, &intent_two);
    let resumed_again = governed.resume_sig(&run, BLOCKER, Some(&signature_two));
    assert_eq!(
        resumed_again.code, 0,
        "WRS-003: the fresh exactly bound resume signature passes: {}{}",
        resumed_again.stdout, resumed_again.message
    );
    assert_eq!(cycle.status(&run), "executing");
    for occurrence in 1..=2 {
        assert!(cycle.pause_path(&run, occurrence).is_file());
    }

    // The pause-record family: editing the digest, a context field, or the
    // occurrence fails detection independently.
    let plain = Cycle::new("wrsv00309b");
    let plain_run = plain.start();
    let plain_held = plain.hold(&plain_run, BLOCKER);
    assert_eq!(
        plain_held.code, 0,
        "{}{}",
        plain_held.stdout, plain_held.message
    );
    plain.assert_pause_record(&plain_run, 1, "intake", "runbook");
    let record_path = plain.pause_path(&plain_run, 1);
    let record_bytes = fs::read_to_string(&record_path).expect("read the pause record");

    let digest_edited = record_bytes.replace(
        &record_bytes
            .lines()
            .find(|line| line.starts_with("held-context-sha256"))
            .unwrap_or_default()
            .to_owned(),
        "held-context-sha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\n",
    );
    assert_ne!(
        digest_edited, record_bytes,
        "the fixture really edited the digest field"
    );
    fs::write(&record_path, &digest_edited).expect("plant the edited digest");
    let digest_refusal = plain.resume(&plain_run, BLOCKER);
    assert_eq!(digest_refusal.code, 1, "{digest_refusal}");
    assert!(
        digest_refusal.message.contains(&format!(
            "resume refused; pause record {} is not a strict version-1 record: ",
            shown(&record_path)
        )),
        "WRS-003: an edited digest refuses through the pause-record family: {digest_refusal}"
    );
    fs::write(&record_path, &record_bytes).expect("restore the pause record");

    let sequence_edited = record_bytes.replace("sequence = 1", "sequence = 2");
    fs::write(&record_path, &sequence_edited).expect("plant the edited sequence");
    let sequence_refusal = plain.resume(&plain_run, BLOCKER);
    assert_eq!(sequence_refusal.code, 1, "{sequence_refusal}");
    assert!(
        sequence_refusal
            .message
            .contains("is not a strict version-1 record: "),
        "WRS-003: a sequence that differs from its filename refuses: {sequence_refusal}"
    );
    fs::write(&record_path, &record_bytes).expect("restore the pause record");

    fs::rename(&record_path, record_path.with_file_name("01.toml"))
        .expect("plant a leading-zero occurrence");
    let occurrence_refusal = plain.resume(&plain_run, BLOCKER);
    assert_eq!(occurrence_refusal.code, 1, "{occurrence_refusal}");
    assert_eq!(
        occurrence_refusal.message,
        "resume refused; pause occurrence 01 is not a positive decimal integer without leading \
         zeroes",
        "WRS-003: a non-canonical occurrence name makes the occurrences untrusted"
    );
    fs::rename(record_path.with_file_name("01.toml"), &record_path)
        .expect("restore the canonical occurrence");

    let context_edited = record_bytes.replace(
        &format!("blocker = {BLOCKER:?}"),
        "blocker = \".arca/other-blocker.md\"",
    );
    assert_ne!(
        context_edited, record_bytes,
        "the fixture really edited a context field"
    );
    fs::write(&record_path, &context_edited).expect("plant the edited context");
    let context_refusal = plain.resume(&plain_run, BLOCKER);
    assert_eq!(context_refusal.code, 1, "{context_refusal}");
    assert!(
        context_refusal
            .message
            .contains("is not a strict version-1 record: "),
        "WRS-003: an edited context field refuses through the digest: {context_refusal}"
    );
    fs::write(&record_path, &record_bytes).expect("restore the pause record");

    // An unknown key refuses through the same strict version-1 family; the
    // detail after the fixed prefix is the parser's own.
    let mut unknown_key = record_bytes.clone();
    unknown_key.push_str("mystery = 7\n");
    fs::write(&record_path, &unknown_key).expect("plant the unknown key");
    let unknown_before = plain.snapshot();
    let unknown = plain.resume(&plain_run, BLOCKER);
    assert_eq!(unknown.code, 1, "{unknown}");
    assert!(
        unknown.message.starts_with(&format!(
            "resume refused; pause record {} is not a strict version-1 record: ",
            shown(&record_path)
        )),
        "WRS-003: the unknown key refuses through the strict family: {unknown}"
    );
    plain.assert_unchanged(&unknown_before, "unknown pause-record key");
    fs::write(&record_path, &record_bytes).expect("restore the pause record");

    // The restored record resumes cleanly.
    plain.resume_ok(&plain_run, 1, "intake");
}
