//! t-135 / WCPV-007 (WCPV-019, WCPV-020, WCPV-021): the distributable
//! Plan-Build profile.
//!
//! WCPV-019 judges the positive whole: `rtm profile plan-build` exports one
//! deterministic offline bundle whose manifest inventories exactly the
//! contract's sixteen artifacts, and a clean repository - no `.arca/schema.md`,
//! no `.ratmac/`, no inherited prompt - initializes from the bundle's
//! `OPERATING.md` and drives one issue to proven completion at `rest` using
//! only the bundle's data and the Engine's own renders. The Machine Class
//! template is judged against the composed class - the recorded base runbook
//! bytes with the three tracked boundary diffs applied in ticket order
//! (t-130's `.ratmac/pause-routes.diff`, then t-131's
//! `.ratmac/review-proofs.diff`, then t-133's `.ratmac/issue-binding.diff`) -
//! exactly the way t-133's fixtures construct it, never by reading the
//! tracked `.ratmac/ratmac.toml` as the template: the template is that class
//! plus the four deltas the contract names, and nothing else.
//!
//! WCPV-020 judges the packaged checker's refusal rows: every twin first
//! holds the valid positive control (export, initialization, `rtm start`),
//! derives its artifact paths and digests from the shipped manifest itself -
//! never a separately invented list - then damages exactly one artifact and
//! proves the cycle's first boundary refuses by artifact name with no Run
//! byte moving.
//!
//! WCPV-021 judges the export's write discipline through t-104's patterns -
//! one folder at a fresh path, never overwriting, the hidden partial sibling,
//! the leftover refusal - plus two-export determinism and the runtime ignore
//! boundaries that hold after initialization.
//!
//! The sprint's whole dependency inventory is git, python (3.11 or newer,
//! for `tomllib`), and the Engine binary this suite was compiled against:
//! no other program is invoked, nothing is fetched, and no sprint step
//! reads any file outside the repository or the bundle.

use ratmac::completion::tree_digest;
use ratmac::machine::{GuardKind, MachineClass};
use ratmac::receipt::sha256_text;
use ratmac_qa::support::{self, CaptureOptions, TempTree};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

// --- the composed class, constructed the way t-133's fixtures do ---------------

/// The recorded `.ratmac/ratmac.toml` bytes of this batch's merge base, the
/// base the three tracked boundary diffs apply over - a byte-identical copy
/// of the embedded base of t-130, t-131, and t-133, embedded so the tracked
/// runbook is never read as the composed class: the cycle boundary wires the
/// diffs into it, and this suite must stay green through that wiring.
const BASE_RUNBOOK_SHA256: &str =
    "012eaa36c1a895f50d8822d516699029cb461a2d489b8ee2b882ce37a8b440c6";

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

/// The `ticket-turns` prompt's final body - t-133's contract string.
const TICKET_TURNS_PROMPT: &str = "\
Take the ordered work items one at a time. For each, begin one turn of the
ticket class bound to that item and to the issue that item works, and let
the turn reach its end before the next one begins. This stage is done when
every turn it began has passed.
";

/// The ticket class's `tests` prompt's final body - t-133's contract string.
const TESTS_PROMPT: &str = "\
Write this item's planned tests first and prove each one red for the stated
reason. Every planned test the item declares needs a sensitivity receipt
showing the test runs and fails before its implementation exists. The
planned tests are the selected issue's verification checks: declare no
check the issue's test plan does not carry, and work the issue the turn is
bound to.
";

/// The repository root this suite was compiled against, the one place the
/// tracked boundary artifacts are read from.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The composed Machine Class: the recorded base bytes with the three tracked
/// boundary diffs applied in ticket order - t-130's, then t-131's, then
/// t-133's - never the tracked runbook read as the composed class.
fn composed_runbook() -> String {
    assert_eq!(
        sha256_text(BASE_RUNBOOK),
        BASE_RUNBOOK_SHA256,
        "the embedded base copy is the recorded merge-base runbook"
    );
    let tree = TempTree::new("t135-class").expect("create the class wiring tree");
    let repo = tree.path();
    fs::create_dir_all(repo.join(".ratmac")).expect("create the class wiring directory");
    fs::write(repo.join(".ratmac/ratmac.toml"), BASE_RUNBOOK)
        .expect("write a clean copy of the base runbook bytes");
    for diff in [
        "pause-routes.diff",
        "review-proofs.diff",
        "issue-binding.diff",
    ] {
        let bytes = fs::read(repo_root().join(".ratmac").join(diff)).unwrap_or_else(|error| {
            panic!("read .ratmac/{diff} - the tracked boundary artifact applied in ticket order: {error}")
        });
        fs::write(repo.join(".ratmac").join(diff), bytes)
            .expect("install the boundary diff beside the base runbook");
    }
    for diff in [
        ".ratmac/pause-routes.diff",
        ".ratmac/review-proofs.diff",
        ".ratmac/issue-binding.diff",
    ] {
        let checked = support::git(repo)
            .args(["apply", "--check", diff])
            .output()
            .expect("run git apply --check");
        assert!(
            checked.status.success(),
            "git apply --check {diff} over the base bytes (ticket order t-130, t-131, t-133) exits 0: {}",
            support::text(&checked)
        );
        let applied = support::git(repo)
            .args(["apply", diff])
            .output()
            .expect("run git apply");
        assert!(
            applied.status.success(),
            "git apply {diff} in ticket order succeeds: {}",
            support::text(&applied)
        );
    }
    fs::read_to_string(repo.join(".ratmac/ratmac.toml")).expect("read the composed runbook bytes")
}

// --- the bundle's declared inventory --------------------------------------------

/// The bundle's declared inventory: sixteen artifacts in the manifest's
/// declared order, each with its role.
const ARTIFACTS: [(&str, &str); 16] = [
    ("OPERATING.md", "instructions"),
    ("terms.md", "terms"),
    ("machine-class.toml", "machine-class"),
    ("schemas/issue.toml", "schema"),
    ("schemas/residual.toml", "schema"),
    ("schemas/ticket.toml", "schema"),
    ("schemas/goal.toml", "schema"),
    ("checkers/verify_profile.py", "checker"),
    ("checkers/verify_issue_binding.py", "checker"),
    ("blanks/issue/index.md", "blank"),
    ("blanks/issue/spec.md", "blank"),
    ("blanks/issue/design.md", "blank"),
    ("blanks/issue/test-plan.md", "blank"),
    ("blanks/issue/ubi-lang.md", "blank"),
    ("blanks/residual.md", "blank"),
    ("blanks/ticket.md", "blank"),
];

/// The terms `terms.md` defines, one line each.
const TERMS: [&str; 23] = [
    "Plan-Build profile",
    "profile compatibility",
    "manifest",
    "artifact",
    "packaged checker",
    "installed profile",
    "dialect",
    "clean repository",
    "initialization",
    "Machine Class",
    "State",
    "Run",
    "Run Record",
    "Exit Guard",
    "goal bundle",
    "issue bundle",
    "residual record",
    "work item",
    "sensitivity receipt",
    "completion receipt",
    "edition",
    "blocked route",
    "pause destination",
];

/// The exact markers `OPERATING.md` carries.
const OPERATING_MARKERS: [&str; 14] = [
    "# Operating the Plan-Build profile",
    "rtm profile plan-build <path>",
    ".ratmac/ratmac.toml",
    "[roots]",
    "python profile/checkers/verify_profile.py check",
    "rtm status",
    "edition-",
    "never moved or overwritten",
    "no global configuration",
    "git",
    "python",
    "3.11",
    "tomllib",
    "never by copying what it printed",
];

/// The t-121 policy bytes a fresh start installs at `.ratmac/.gitignore`.
const FRESH_POLICY: &[u8] = b"/runs/\n/mint.toml\n/locks/\n/log.md\n";

const ISSUE: &str = "i-200-demo";
const ITEM: &str = "t-200.md";
const ITEM_ID: &str = "t-200";
const REQUIREMENT: &str = "DEMO-201";
const RESIDUAL: &str = "res-200";
const PLANNED: &str = "PT-200-01";
const HIDDEN: &str = "HT-200-01";
const QUALITY: &str = "cargo --version";
const RUN: &str = "run-001";

/// The green output a receipt records for a check that passed.
const GREEN: &str = "test result: ok. 1 passed; 0 failed\n";

/// The red output a sensitivity receipt records for a planned test that
/// failed before its implementation existed.
const RED: &str = "test result: FAILED. 0 passed; 1 failed\n";

/// The export's one success line, for the displayed spelling of `target`.
fn export_success_line(target: &str) -> String {
    format!(
        "Wrote {target}: the plan-build profile (format 1, 16 artifacts). Initialize from its \
         OPERATING.md; the export is deterministic, offline, and never overwrites.\n"
    )
}

/// The packaged self-check's one success line.
const SELF_CHECK_LINE: &str = "profile verified: plan-build (format 1, 16 artifacts)\n";

// --- small shared plumbing -------------------------------------------------------

fn out_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn err_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Every regular file under `root`, keyed by `/`-separated relative path.
fn walk_files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in
            fs::read_dir(dir).unwrap_or_else(|error| panic!("read {}: {error}", shown(dir)))
        {
            let entry = entry.unwrap_or_else(|error| panic!("read {}: {error}", shown(dir)));
            let path = entry.path();
            if entry
                .file_type()
                .unwrap_or_else(|error| panic!("inspect {}: {error}", shown(&path)))
                .is_dir()
            {
                walk(base, &path, files);
            } else {
                let relative = shown(&path.strip_prefix(base).expect("entry under root"));
                files.insert(relative, fs::read(&path).expect("read file"));
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

/// Fill one YAML scalar field whose value still carries a `{{...}}`
/// placeholder, whatever the blank spells inside the braces.
fn fill_scalar(text: &str, key: &str, value: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut filled = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix(key).unwrap_or("");
        if !filled && !rest.is_empty() && rest.trim_start().starts_with(':') && line.contains("{{")
        {
            out.push_str(&format!("{key}: \"{value}\"\n"));
            filled = true;
        } else {
            out.push_str(line);
        }
    }
    assert!(
        filled,
        "the blank carries a {key} scalar placeholder to fill; the packaged bytes drifted"
    );
    out
}

/// Replace the `- ` entry list under a YAML `key:` block with exactly the
/// given values, contiguous under the key so every list reader resolves them.
fn set_list(text: &str, key: &str, values: &[&str]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut lines = text.split_inclusive('\n').peekable();
    let mut replaced = false;
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if !replaced
            && trimmed.starts_with(key)
            && trimmed[key.len()..].trim_start().starts_with(':')
        {
            out.push(line.to_owned());
            replaced = true;
            // Drop the placeholder entries that follow the key line.
            while let Some(next) = lines.peek() {
                let next_trimmed = next.trim();
                if next_trimmed.starts_with("- ") {
                    let _ = lines.next();
                } else {
                    break;
                }
            }
            for value in values {
                out.push(format!("  - \"{value}\"\n"));
            }
            continue;
        }
        out.push(line.to_owned());
    }
    assert!(
        replaced,
        "the blank carries a {key} list to set; the packaged bytes drifted"
    );
    out.join("")
}

// --- the clean-repository fixture --------------------------------------------------

/// A temporary Git repository holding nothing but `src/lib.rs` and
/// `test/fixture_test.rs` - no `.arca/`, no `.ratmac/`, no inherited prompt
/// (every child process runs through the scrubbed `support::command`).
struct Clean {
    _owner: TempTree,
    root: PathBuf,
}

impl Clean {
    fn create(label: &str) -> Self {
        let owner = TempTree::new(&format!("t135-{label}")).expect("create the fixture tree");
        let root = owner.join("repo");
        for dir in ["src", "test"] {
            fs::create_dir_all(root.join(dir)).expect("create the fixture tree");
        }
        fs::write(root.join("src/lib.rs"), "pub fn work() {}\n").expect("write fixture source");
        fs::write(root.join("test/fixture_test.rs"), "fn pt_200_01_red() {}\n")
            .expect("write fixture tests");
        let git_ok = |args: &[&str]| {
            let output = support::git(&root).args(args).output().expect("invoke git");
            assert!(
                output.status.success(),
                "git {args:?} seeds the clean repository: {}",
                support::text(&output)
            );
        };
        git_ok(&["init", "--quiet"]);
        git_ok(&["config", "user.email", "fixture@example.invalid"]);
        git_ok(&["config", "user.name", "Fixture"]);
        git_ok(&["config", "core.autocrlf", "false"]);
        git_ok(&["add", "-A"]);
        git_ok(&["commit", "--quiet", "-m", "seed the clean repository"]);
        // Clean by construction, judged not assumed.
        assert!(
            !root.join(".arca/schema.md").exists(),
            "the fixture repository carries no working rules"
        );
        assert!(
            !root.join(".ratmac").exists(),
            "the fixture repository carries no Engine root"
        );
        Self {
            _owner: owner,
            root,
        }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        support::command(ratmac_qa::engine_bin!(), &self.root)
            .args(args)
            .output()
            .expect("invoke rtm")
    }

    fn python(&self, args: &[&str]) -> Output {
        support::command("python", &self.root)
            .args(args)
            .output()
            .expect("invoke python")
    }

    fn git_ok(&self, args: &[&str]) {
        let output = support::git(&self.root)
            .args(args)
            .output()
            .expect("invoke git");
        assert!(
            output.status.success(),
            "git {args:?} in the fixture repository: {}",
            support::text(&output)
        );
    }

    fn write(&self, relative: &str, body: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture directory");
        }
        fs::write(path, body).expect("write fixture file");
    }

    fn step_ok(&self, run: &str, why: &str) -> String {
        let output = self.rtm(&["step", "--run", run]);
        let text = support::text(&output);
        assert!(output.status.success(), "{why}: {}", text);
        text
    }

    fn run_dir(&self, run: &str) -> PathBuf {
        self.root.join(format!(".ratmac/runs/{run}"))
    }

    fn record(&self, run: &str) -> String {
        fs::read_to_string(self.run_dir(run).join("run.toml")).expect("read the Run Record")
    }

    fn state(&self, run: &str) -> String {
        self.record(run)
            .lines()
            .find_map(|line| line.trim().strip_prefix("state = "))
            .unwrap_or_default()
            .trim_matches('"')
            .to_owned()
    }

    fn status(&self, run: &str) -> String {
        self.record(run)
            .lines()
            .find_map(|line| line.trim().strip_prefix("status = "))
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

    /// Every byte under the Engine root, tagged by kind and path.
    fn snapshot_engine(&self) -> BTreeMap<String, Vec<u8>> {
        support::capture(&self.root.join(".ratmac"), CaptureOptions::default())
            .expect("capture the fixture Engine root")
            .tagged()
    }

    /// Nothing under the Engine root changed except the addressed Run's own
    /// `evidence.toml` - the Engine's audit line for the observed refusal is
    /// its durability, never an agent's write (R-017 keeps State and Status
    /// untouched; this keeps every other byte untouched).
    fn assert_engine_untouched(
        &self,
        before: &BTreeMap<String, Vec<u8>>,
        run: &str,
        because: &str,
    ) {
        let after = self.snapshot_engine();
        let own_log = format!("runs/{run}/evidence.toml");
        let mut changed: Vec<&String> = after
            .keys()
            .chain(before.keys())
            .filter(|key| before.get(*key) != after.get(*key))
            .collect();
        changed.retain(|key| **key != own_log);
        assert!(
            changed.is_empty(),
            "{because}: the refused step moves no Engine byte beyond {own_log}: {changed:?}"
        );
    }
}

/// One clean repository's export and initialization, the positive control
/// every refusal twin holds first: export, the operator's placement from
/// `OPERATING.md`, the self-check, the doctor read, and `rtm start`.
fn export_and_initialize(label: &str) -> Clean {
    let fx = Clean::create(label);

    // The export: exit 0, one stdout line, stderr empty, no Engine root.
    let exported = fx.rtm(&["profile", "plan-build", "profile"]);
    let exported_text = support::text(&exported);
    assert!(
        exported.status.success(),
        "the positive control export exits 0: {exported_text}"
    );
    assert_eq!(out_text(&exported), export_success_line("profile"));
    assert_eq!(err_text(&exported), "");
    assert!(
        !fx.root.join(".ratmac").exists(),
        "the export creates no Engine root: {exported_text}"
    );

    // The operator's initialization from the bundle's OPERATING.md, with
    // its own self-check and the doctor read on the installed template.
    initialize_in_place(&fx);

    fx
}

/// The issue bundle, built from the packaged blanks: each of the five files
/// copied and filled, the requirement row accepted, the verification rows
/// carrying the identifiers the work item will declare.
fn place_issue_bundle(fx: &Clean) {
    let source = fx.root.join("profile/blanks/issue");
    let bundle = fx.root.join(format!(".arca/issue/{ISSUE}"));
    fs::create_dir_all(&bundle).expect("create the issue folder");
    for name in [
        "design.md",
        "index.md",
        "spec.md",
        "test-plan.md",
        "ubi-lang.md",
    ] {
        let text = fs::read_to_string(source.join(name))
            .unwrap_or_else(|error| panic!("read the blank blanks/issue/{name}: {error}"));
        assert!(
            text.contains("{{"),
            "the blank blanks/issue/{name} keeps its placeholder structure"
        );
        let filled = match name {
            "index.md" => {
                let filled = fill_scalar(&text, "issue-id", ISSUE);
                let filled = fill_scalar(&filled, "provenance", "the t-135 public suite fixture");
                fill_scalar(&filled, "status", "integrated")
            }
            .replace("{{issue-title}}", "Demo issue")
            .replace("{{issue-summary}}", "One demo ask, accepted into the goal."),
            "spec.md" => text
                .replace("{{req-id}}", REQUIREMENT)
                .replace("{{requirement-text}}", "The demo behaves.")
                .replace("{{accepted\\|rejected\\|duplicate\\|deferred}}", "accepted")
                .replace("{{accepted|rejected|duplicate|deferred}}", "accepted")
                .replace("{{rationale-text}}", "The sprint works it.")
                .replace("{{authority-refs-if-accepted}}", "goal/spec.md"),
            "test-plan.md" => text
                .replace("{{check-id}}", PLANNED)
                .replace("{{issue-requirement-refs}}", REQUIREMENT)
                .replace(
                    "{{expected-evidence}}",
                    "A sensitivity receipt, then a green receipt.",
                ),
            _ => text,
        };
        fs::write(bundle.join(name), filled).expect("write the issue file");
    }
    let spec = fs::read_to_string(bundle.join("spec.md")).expect("read the filled spec");
    assert!(
        spec.contains(&format!("`{REQUIREMENT}`")) && spec.contains("accepted"),
        "the filled spec carries the accepted requirement row: {spec}"
    );
    let plan = fs::read_to_string(bundle.join("test-plan.md")).expect("read the filled plan");
    let plan = format!(
        "{plan}\n## Sprint checks\n\n\
         | Check | Requirement Refs | Expected evidence |\n|---|---|---|\n\
         | `{HIDDEN}` | `{REQUIREMENT}` | A hidden-lane receipt. |\n"
    );
    fs::write(bundle.join("test-plan.md"), &plan).expect("append the hidden-lane row");
    assert!(
        plan.contains(PLANNED) && plan.contains(HIDDEN),
        "the filled test plan carries both declared identifiers"
    );
}

/// The gap record, written from the packaged blank with the frozen revision
/// and the open status.
fn place_residual(fx: &Clean, frozen: &str) {
    let text = fs::read_to_string(fx.root.join("profile/blanks/residual.md"))
        .expect("read the residual blank");
    assert!(
        text.contains("{{"),
        "the blank blanks/residual.md keeps its placeholder structure"
    );
    let filled = fill_scalar(&text, "residual-id", RESIDUAL);
    let filled = fill_scalar(&filled, "goal-requirement-ref", REQUIREMENT);
    let filled = fill_scalar(
        &filled,
        "frozen-goal-bundle-revision",
        &format!("goal-sha256:{frozen}"),
    );
    let filled = fill_scalar(&filled, "implementation-revision", "");
    let filled = fill_scalar(
        &filled,
        "classification-rationale",
        "The tree holds no demo behavior yet.",
    );
    let filled = fill_scalar(&filled, "status", "missing");
    let filled = set_list(&filled, "concrete-evidence-refs", &[]);
    let filled = set_list(&filled, "mutation-kill", &[]);
    let filled = set_list(&filled, "required-test-refs", &[PLANNED]);
    fx.write(&format!(".arca/residual/{RESIDUAL}.md"), &filled);
    assert!(
        filled.contains(&format!("status: \"missing\""))
            && filled.contains(&format!("goal-requirement-ref: \"{REQUIREMENT}\"")),
        "the filled residual carries its gate-read fields: {filled}"
    );
}

/// Flip the gap record to satisfied with concrete evidence, the close-time
/// rewrite the operator performs from the same blank.
fn flip_residual_satisfied(fx: &Clean, receipts: &[&str]) {
    let path = fx.root.join(format!(".arca/residual/{RESIDUAL}.md"));
    let text = fs::read_to_string(&path).expect("read the residual record");
    let flipped = set_list(
        &fill_scalar(&text, "status", "satisfied"),
        "concrete-evidence-refs",
        receipts,
    );
    fs::write(&path, flipped).expect("write the satisfied residual");
}

/// The work item, cut from the packaged blank: the front-matter check lists
/// the completion gate reads, plus the sections the record contract demands
/// that the blank leaves to the item's author.
fn place_ticket(fx: &Clean) {
    let text = fs::read_to_string(fx.root.join("profile/blanks/ticket.md"))
        .expect("read the ticket blank");
    assert!(
        text.contains("{{"),
        "the blank blanks/ticket.md keeps its placeholder structure"
    );
    let filled = fill_scalar(&text, "ticket-id", ITEM_ID);
    let filled = fill_scalar(&filled, "landed-commit", "");
    let filled = fill_scalar(&filled, "blocker-ref", "");
    let filled = fill_scalar(&filled, "status", "approved");
    let filled = set_list(&filled, "planned-test-refs", &[PLANNED]);
    let filled = set_list(&filled, "focused-tests", &[PLANNED]);
    let filled = set_list(&filled, "hidden-lanes", &[HIDDEN]);
    let filled = set_list(&filled, "quality-commands", &[QUALITY]);
    let filled = set_list(&filled, "residual-ids", &[RESIDUAL]);
    let filled = set_list(&filled, "dependencies", &[]);
    // `{{ticket-id}}` also names the item in the body; `{{residual-id}}` the
    // gap it owns. The remaining table placeholders name this sprint's row.
    let filled = filled
        .replace("{{ticket-id}}", ITEM_ID)
        .replace("{{residual-id}}", RESIDUAL)
        .replace("{{planned-test-id}}", PLANNED)
        .replace("{{path/to/contract.md#anchor}}", "goal/spec.md")
        .replace("{{fixture-ref}}", "the clean repository")
        .replace("{{executable-target}}", "this suite")
        .replace("{{observable-oracle}}", "the sprint below")
        .replace("{{category}}", "Regression")
        .replace("{{covered\\|not-applicable}}", "covered")
        .replace("{{covered|not-applicable}}", "covered")
        .replace(
            "{{assessment-rationale}}",
            "The sprint proves the export whole.",
        )
        .replace("{{hidden-ids-or-none}}", HIDDEN)
        .replace("{{hidden-id}}", HIDDEN)
        .replace(
            "{{why-the-oracle-follows-from-the-contract}}",
            "Derived from the contract.",
        )
        .replace("{{oracle}}", "This sprint.")
        .replace("{{owner}}", "the operator");
    // The blank authors the P4 and P5 sections; the record contract's other
    // three sections are the item author's to write.
    let filled = format!(
        "{filled}\n## Vertical Outcome\n\nOne demo outcome, worked to proven completion.\n\n\
         ## Worktree Scope\n\nThis repository, initialized from the bundle.\n\n\
         ## Merge Gate\n\n- `{QUALITY}` passes.\n"
    );
    fx.write(&format!(".arca/ticket/{ITEM}"), &filled);
    for section in [
        "## Vertical Outcome",
        "## Worktree Scope",
        "## P4 Apparent Test Plan",
        "## P5 Hidden Test Public Coverage Manifest",
        "## Merge Gate",
    ] {
        assert!(
            filled.contains(section),
            "the cut item carries the record contract's section {section}"
        );
    }
    assert!(
        filled.contains(&format!("focused-tests:\n  - \"{PLANNED}\""))
            && filled.contains(&format!("hidden-lanes:\n  - \"{HIDDEN}\""))
            && filled.contains(&format!("quality-commands:\n  - \"{QUALITY}\"")),
        "the cut item declares its three check lists: {filled}"
    );
}

/// One sensitivity receipt proving the planned test fails before its code.
fn write_sensitivity(fx: &Clean, run: &str) {
    let dir = fx.root.join(format!(".ratmac/evidence/{run}"));
    fs::create_dir_all(&dir).expect("create evidence directory");
    let body = format!(
        "planned-test-id = \"{PLANNED}\"\n\
         ticket-id = \"{ITEM_ID}\"\n\
         kind = \"baseline-failure\"\n\
         command = \"cargo test --test fixture_test\"\n\
         working-dir = \".\"\n\
         test-file = \"test/fixture_test.rs\"\n\
         test-name = \"pt_200_01_red\"\n\
         exit-status = 101\n\
         output-sha256 = \"{}\"\n\
         output = \"\"\"\n{RED}\"\"\"\n",
        sha256_text(RED)
    );
    fs::write(dir.join(format!("{PLANNED}.toml")), body).expect("write sensitivity receipt");
}

/// Every check the work item declares, recorded green and fresh against the
/// tree as it stands.
fn write_completion(fx: &Clean, run: &str) {
    let dir = fx.root.join(format!(".ratmac/evidence/{run}/completion"));
    fs::create_dir_all(&dir).expect("create completion directory");
    let digest = tree_digest(&fx.root, &["src".to_owned()]).expect("source roots are readable");
    for (check, kind, command) in [
        (PLANNED, "focused", "cargo test --test fixture_test"),
        (HIDDEN, "hidden-lane", "cargo test --test fixture_test"),
        (QUALITY, "quality", "cargo --version"),
    ] {
        let body = format!(
            "ticket-id = \"{ITEM_ID}\"\n\
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

// --- the bundle's shape and the template's four deltas -----------------------------

/// The manifest rows of the exported bundle: path, role, sha256 - read from
/// the shipped manifest itself, never a separately invented list.
fn manifest_rows(fx: &Clean) -> Vec<(String, String, String)> {
    let text = fs::read_to_string(fx.root.join("profile/manifest.toml"))
        .expect("read the exported manifest");
    let value: toml::Value = text.parse().expect("the manifest parses as strict TOML");
    let keys: Vec<&str> = value
        .as_table()
        .expect("the manifest is a table")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        vec!["artifact", "compatibility", "profile"],
        "the manifest holds exactly the profile, compatibility, and artifact tables"
    );
    assert_eq!(
        value["profile"]["name"].as_str(),
        Some("plan-build"),
        "the manifest names the profile"
    );
    assert_eq!(
        value["profile"]["format"].as_integer(),
        Some(1),
        "the manifest declares format 1"
    );
    assert_eq!(
        value["compatibility"]["runbook"].as_str(),
        Some("states-v1"),
        "the manifest declares the states-v1 dialect"
    );
    assert_eq!(
        value["compatibility"]["checkers"].as_integer(),
        Some(1),
        "the manifest declares checker contract 1"
    );
    value["artifact"]
        .as_array()
        .expect("the manifest rows its artifacts")
        .iter()
        .map(|row| {
            let keys: Vec<&str> = row
                .as_table()
                .expect("an artifact row is a table")
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(
                keys,
                vec!["path", "role", "sha256"],
                "an artifact row's fields"
            );
            (
                row["path"].as_str().expect("path").to_owned(),
                row["role"].as_str().expect("role").to_owned(),
                row["sha256"].as_str().expect("sha256").to_owned(),
            )
        })
        .collect()
}

/// The bundle is exactly its manifest inventory: sixteen artifacts in the
/// declared order, every digest equal over the file's exact bytes, every
/// tracked file LF-terminated, and nothing else in the folder.
fn assert_bundle_shape(fx: &Clean) {
    let rows = manifest_rows(fx);
    assert_eq!(
        rows.len(),
        ARTIFACTS.len(),
        "the manifest rows 16 artifacts"
    );
    for (row, (path, role)) in rows.iter().zip(ARTIFACTS) {
        assert_eq!(row.0, path, "the manifest declares the artifact in order");
        assert_eq!(row.1, role, "the artifact's role is the contract's");
        assert!(
            row.2.len() == 64
                && row
                    .2
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "the digest of {path} is 64 lowercase hex characters: {}",
            row.2
        );
        let bytes = fs::read(fx.root.join("profile").join(path))
            .unwrap_or_else(|error| panic!("read the artifact {path}: {error}"));
        assert_eq!(
            sha256_hex(&bytes),
            row.2,
            "the recorded digest equals the artifact's exact bytes: {path}"
        );
        assert!(
            !bytes.contains(&b'\r') && bytes.last() == Some(&b'\n'),
            "the tracked artifact {path} is LF-terminated with no carriage return"
        );
    }
    let mut expected: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    expected.insert(
        "manifest.toml".to_owned(),
        fs::read(fx.root.join("profile/manifest.toml")).expect("read the manifest"),
    );
    for (path, _) in ARTIFACTS {
        expected.insert(
            path.to_owned(),
            fs::read(fx.root.join("profile").join(path))
                .unwrap_or_else(|error| panic!("read the artifact {path}: {error}")),
        );
    }
    assert_eq!(
        walk_files(&fx.root.join("profile")),
        expected,
        "the bundle is exactly the manifest plus its sixteen artifacts, no more"
    );
}

/// One `command_exit` validator row, matched as a guard.
fn assert_validator_row(guard: &GuardKind, script: &str, because: &str) {
    match guard {
        GuardKind::CommandExit {
            program,
            args,
            expected,
            ..
        } => {
            assert_eq!(program, "python", "{because}");
            assert_eq!(args, &[script.to_owned(), "check".to_owned()], "{because}");
            assert_eq!(*expected, 0, "{because}");
        }
        other => panic!("{because}: the guard is {other:?}, not the validator row"),
    }
}

/// The template is the composed class plus exactly the contract's four
/// deltas, judged through the one reader over both classes side by side.
fn assert_template_is_composed_class(template: &str) {
    let composed_text = composed_runbook();
    let composed = MachineClass::from_toml(&composed_text)
        .expect("the composed class parses through the one reader");
    let base = MachineClass::from_toml(BASE_RUNBOOK)
        .expect("the base class parses through the one reader");
    let class = MachineClass::from_toml(template)
        .expect("the Machine Class template parses through the one reader");

    // A header comment marks the file as the profile's template.
    assert!(
        template.lines().next().unwrap_or_default().starts_with('#'),
        "the template opens with its header comment"
    );

    // The state shape and the roots table are the composed class's own.
    let states: Vec<&str> = class.states().keys().map(String::as_str).collect();
    assert_eq!(
        states,
        vec![
            "close",
            "cut-tickets",
            "gap-check",
            "intake",
            "rest",
            "ticket-turns"
        ],
        "the template declares the cycle's States"
    );
    assert!(
        class.classes().contains_key("ticket"),
        "the template declares the ticket class"
    );
    for role in ["goal", "issue", "residual", "ticket", "authority"] {
        assert_eq!(
            class.roots().path(role),
            composed.roots().path(role),
            "the roots table keeps the composed class's declared root for {role}"
        );
    }

    // The base prompts stay: intake, gap-check, cut-tickets, close, rest,
    // and the ticket class's done - byte for byte the composed class's.
    for state in ["intake", "gap-check", "cut-tickets", "close", "rest"] {
        assert_eq!(
            class.states()[state].prompt(),
            composed.states()[state].prompt(),
            "the {state} prompt keeps the composed class's bytes"
        );
    }
    let ticket = class.classes().get("ticket").expect("the ticket class");
    let composed_ticket = composed
        .classes()
        .get("ticket")
        .expect("the composed ticket");
    let base_ticket = base.classes().get("ticket").expect("the base ticket");
    assert_eq!(
        ticket.states()["done"].prompt(),
        composed_ticket.states()["done"].prompt(),
        "the done prompt keeps the composed class's bytes"
    );

    // t-133's final prompt bodies stay byte for byte.
    assert_eq!(
        class.states()["ticket-turns"].prompt(),
        TICKET_TURNS_PROMPT,
        "the ticket-turns prompt is t-133's final string"
    );
    assert_eq!(
        ticket.states()["tests"].prompt(),
        TESTS_PROMPT,
        "the tests prompt is t-133's final string"
    );

    // Delta 4: implement and damage keep the recorded base bytes, not
    // t-131's review-proofs finals.
    assert_eq!(
        ticket.states()["implement"].prompt(),
        base_ticket.states()["implement"].prompt(),
        "the implement prompt keeps the recorded base bytes"
    );
    assert_eq!(
        ticket.states()["damage"].prompt(),
        base_ticket.states()["damage"].prompt(),
        "the damage prompt keeps the recorded base bytes"
    );

    // Delta 3: the intake State prepends the profile validator before
    // intake_contract.
    let intake_guards = class.states()["intake"].guards();
    assert_eq!(
        intake_guards.len(),
        2,
        "the intake State carries two guards"
    );
    assert_validator_row(
        &intake_guards[0],
        "profile/checkers/verify_profile.py",
        "the intake State prepends the profile validator",
    );
    assert!(
        matches!(intake_guards[1], GuardKind::IntakeContract),
        "intake_contract follows the profile validator"
    );

    // Every other top-level guard list is the composed class's own.
    for state in ["gap-check", "cut-tickets", "ticket-turns"] {
        assert_eq!(
            class.states()[state].guards(),
            composed.states()[state].guards(),
            "the {state} guard list is the composed class's own"
        );
    }

    // Delta 1: the close State's lane-sweep row is absent; its guards are
    // exactly record_contract then the edition describe.
    let close_guards = class.states()["close"].guards();
    assert_eq!(
        composed.states()["close"].guards().len(),
        3,
        "the composed close State still carries the sweep row"
    );
    assert_eq!(
        close_guards.len(),
        2,
        "the template's close guards are exactly two"
    );
    assert!(matches!(close_guards[0], GuardKind::RecordContract));
    match &close_guards[1] {
        GuardKind::CommandExit {
            program,
            args,
            expected,
            ..
        } => {
            assert_eq!(program, "git");
            assert_eq!(
                args,
                &[
                    "describe".to_owned(),
                    "--exact-match".to_owned(),
                    "--match".to_owned(),
                    "edition-*".to_owned()
                ]
            );
            assert_eq!(*expected, 0);
        }
        other => panic!("the second close guard is the edition describe, not {other:?}"),
    }

    // The spawn row binds both names; the bindings tables declare both
    // required - RB505's equality.
    let turns = &class.states()["ticket-turns"];
    assert_eq!(turns.spawns().len(), 1, "ticket-turns declares one spawn");
    let spawn = &turns.spawns()[0];
    assert_eq!(spawn.class(), "ticket");
    assert_eq!(spawn.name(), "turn");
    assert_eq!(
        spawn.bind(),
        &["item".to_owned(), "issue".to_owned()],
        "the spawn row binds item and issue"
    );
    let bindings: Vec<&str> = ticket.bindings().keys().map(String::as_str).collect();
    assert_eq!(
        bindings,
        vec!["issue", "item"],
        "the ticket class declares both bindings"
    );
    for name in ["item", "issue"] {
        assert!(
            ticket.bindings()[name].required(),
            "the {name} binding is required"
        );
    }

    // Delta 2: the ticket class's validator rows name the packaged copy,
    // prepended before the receipt guard they accompany, and the rest of
    // each working State's list is the composed class's own.
    for state in ["tests", "implement", "damage"] {
        let guards = ticket.states()[state].guards();
        assert_validator_row(
            &guards[0],
            "profile/checkers/verify_issue_binding.py",
            "the {state} State prepends the packaged binding validator",
        );
        assert_eq!(
            &guards[1..],
            &composed_ticket.states()[state].guards()[1..],
            "the rest of the {state} guard list is the composed class's own"
        );
    }

    // t-130's eight blocked self-routes and the ordinary edges stay.
    let mut blocked: Vec<(String, String)> = class
        .transitions()
        .iter()
        .chain(ticket.transitions().iter().map(|transition| transition))
        .filter(|transition| transition.is_blocked_route())
        .map(|transition| {
            (
                transition.from().as_str().to_owned(),
                transition.to().as_str().to_owned(),
            )
        })
        .collect();
    blocked.sort();
    let mut expected_blocked: Vec<(String, String)> = [
        ("intake", "intake"),
        ("gap-check", "gap-check"),
        ("cut-tickets", "cut-tickets"),
        ("ticket-turns", "ticket-turns"),
        ("close", "close"),
        ("tests", "tests"),
        ("implement", "implement"),
        ("damage", "damage"),
    ]
    .iter()
    .map(|(from, to)| ((*from).to_owned(), (*to).to_owned()))
    .collect();
    expected_blocked.sort();
    assert_eq!(
        blocked, expected_blocked,
        "the eight blocked self-routes stay"
    );
    let ordinary: Vec<(&str, &str)> = class
        .transitions()
        .iter()
        .filter(|transition| !transition.is_blocked_route())
        .map(|transition| (transition.from().as_str(), transition.to().as_str()))
        .collect();
    let composed_ordinary: Vec<(&str, &str)> = composed
        .transitions()
        .iter()
        .filter(|transition| !transition.is_blocked_route())
        .map(|transition| (transition.from().as_str(), transition.to().as_str()))
        .collect();
    assert_eq!(ordinary, composed_ordinary, "the ordinary cycle edges stay");
    let turn_edges: Vec<(&str, &str)> = ticket
        .transitions()
        .iter()
        .filter(|transition| !transition.is_blocked_route())
        .map(|transition| (transition.from().as_str(), transition.to().as_str()))
        .collect();
    assert_eq!(
        turn_edges,
        vec![
            ("tests", "implement"),
            ("implement", "damage"),
            ("damage", "done")
        ]
    );
}

/// WCPV-019: export the plan-build profile, judge its bundle and Machine
/// Class template, initialize a clean repository from the shipped
/// instructions, and drive one issue to proven completion at rest.
#[test]
fn wcpv_019() {
    let fx = Clean::create("wcpv019");

    // The dependency inventory, as the fixture opens it: python 3.11+ (the
    // checkers' tomllib baseline), git (configured above), and the Engine
    // binary - nothing else is invoked anywhere in this sprint.
    let version = fx.python(&["--version"]);
    let version_text = support::text(&version);
    assert!(
        version.status.success(),
        "the inventory reads the python baseline: {version_text}"
    );
    let numbers: Vec<u32> = version_text
        .trim()
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect();
    assert!(
        numbers.first() == Some(&3) && numbers.get(1).is_some_and(|minor| *minor >= 11),
        "the sprint's python is 3.11 or newer: {version_text}"
    );

    // The export: exit 0, one stdout line, stderr empty, no Engine root, and
    // every added byte under the one target folder.
    let before = support::capture(
        &fx.root,
        CaptureOptions {
            excluded_names: &[".git"],
            ..CaptureOptions::default()
        },
    )
    .expect("snapshot the clean repository")
    .tagged();
    let exported = fx.rtm(&["profile", "plan-build", "profile"]);
    let exported_text = support::text(&exported);
    assert!(
        exported.status.success(),
        "WCPV-019: rtm profile plan-build exits 0 on a fresh path: {exported_text}"
    );
    assert_eq!(
        out_text(&exported),
        export_success_line("profile"),
        "WCPV-019: stdout is exactly the contract's success line"
    );
    assert_eq!(err_text(&exported), "", "WCPV-019: stderr is empty");
    assert!(
        !fx.root.join(".ratmac").exists(),
        "WCPV-019: the export leaves .ratmac/ absent: {exported_text}"
    );
    let after = support::capture(
        &fx.root,
        CaptureOptions {
            excluded_names: &[".git"],
            ..CaptureOptions::default()
        },
    )
    .expect("snapshot the exported repository")
    .tagged();
    for key in after.keys() {
        assert!(
            key.starts_with("profile/") || before.contains_key(key),
            "WCPV-019: every written byte sits under the one target folder, not {key:?}"
        );
    }
    for (key, bytes) in &before {
        assert_eq!(
            after.get(key),
            Some(bytes),
            "WCPV-019: no pre-existing byte changes: {key:?}"
        );
    }

    // The bundle's shape: sixteen artifacts in the declared order, digests
    // over the exact bytes, LF-terminated, nothing else.
    assert_bundle_shape(&fx);

    // The packaged content's own markers.
    let operating = fs::read_to_string(fx.root.join("profile/OPERATING.md"))
        .expect("read the operating instructions");
    for marker in OPERATING_MARKERS {
        assert!(
            operating.contains(marker),
            "WCPV-019: OPERATING.md carries the exact marker {marker:?}"
        );
    }
    let terms = fs::read_to_string(fx.root.join("profile/terms.md")).expect("read the terms");
    for term in TERMS {
        assert!(
            terms.contains(term),
            "WCPV-019: terms.md defines the term {term:?}"
        );
    }
    for (path, _) in ARTIFACTS {
        if path.starts_with("blanks/") {
            let text = fs::read_to_string(fx.root.join("profile").join(path))
                .unwrap_or_else(|error| panic!("read the blank {path}: {error}"));
            assert!(
                text.contains("{{"),
                "WCPV-019: the blank {path} keeps its placeholder structure"
            );
        }
    }

    // The Machine Class template is the composed class plus the four named
    // deltas, and the packaged binding checker is byte-identical to t-133's
    // landed contributor tool.
    let template = fs::read_to_string(fx.root.join("profile/machine-class.toml"))
        .expect("read the Machine Class template");
    assert_template_is_composed_class(&template);
    assert_eq!(
        fs::read(fx.root.join("profile/checkers/verify_issue_binding.py"))
            .expect("read the packaged binding checker"),
        fs::read(repo_root().join("tools/verify_issue_binding.py"))
            .expect("read t-133's landed validator"),
        "WCPV-019: the packaged binding checker is byte-identical to the landed tool"
    );

    // Initialization and the sprint.
    initialize_in_place(&fx);
    let started = fx.rtm(&["start"]);
    let started_text = support::text(&started);
    assert!(
        started.status.success(),
        "WCPV-019: rtm start admits the first Run: {started_text}"
    );
    assert_eq!(
        out_text(&started),
        format!("rtm: started run {RUN} at .ratmac/runs/{RUN}/\n"),
        "WCPV-019: start's stdout is today's single line"
    );
    assert_eq!(
        fs::read(fx.root.join(".ratmac/.gitignore")).expect("read the installed policy"),
        FRESH_POLICY.to_vec(),
        "WCPV-019: start installs t-121's four policy lines"
    );
    assert_eq!(
        fx.state(RUN),
        "intake",
        "WCPV-019: the Run starts at intake"
    );

    // Intake: the profile validator then intake_contract, and the edge
    // freezes the goal.
    fx.step_ok(
        RUN,
        "WCPV-019: intake -> gap-check validates the bundle and the records",
    );
    assert_eq!(fx.state(RUN), "gap-check");
    let frozen = fx.frozen(RUN);

    // Gap analysis: the record from the blank, the reviewer routes on gaps.
    place_residual(&fx, &frozen);
    fx.write_verdict(RUN, "gap-check", "gaps");
    fx.step_ok(RUN, "WCPV-019: gap-check -> cut-tickets routes on gaps");
    assert_eq!(fx.state(RUN), "cut-tickets");

    // The work item from the blank, declaring its three check lists.
    place_ticket(&fx);
    fx.step_ok(RUN, "WCPV-019: cut-tickets -> ticket-turns owns the gap");
    assert_eq!(fx.state(RUN), "ticket-turns");

    // The turn: spawned bound to the item and the selected issue, minted at
    // tests with t-133's spawn line and roster clause.
    let spawned = fx.rtm(&[
        "spawn",
        "turn",
        "--run",
        RUN,
        "--bind",
        &format!("item={ITEM}"),
        "--bind",
        &format!("issue={ISSUE}"),
    ]);
    let spawned_text = support::text(&spawned);
    assert!(
        spawned.status.success(),
        "WCPV-019: the coordinator spawns the turn: {spawned_text}"
    );
    let child = spawned_text
        .split("spawned run ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .expect("spawn names the child run id")
        .to_owned();
    assert!(
        spawned_text.contains(&format!(
            "rtm: spawned run {child} (spawn turn) from run {RUN}; the child is an \
             ordinary Run at .ratmac/runs/{child}/"
        )),
        "WCPV-019: stdout is today's spawn line: {spawned_text}"
    );
    assert_eq!(
        fx.state(&child),
        "tests",
        "WCPV-019: a turn starts at the tests State"
    );
    let roster = support::text(&fx.rtm(&["status"]));
    assert!(
        roster.contains(&format!("bind issue=\"{ISSUE}\", item=\"{ITEM}\"")),
        "WCPV-019: the roster renders the joined binding clause: {roster}"
    );

    // Work-item checks: the sensitivity receipt, then the step through the
    // packaged binding validator to implement.
    write_sensitivity(&fx, &child);
    fx.step_ok(&child, "WCPV-019: the tests boundary passes");
    assert_eq!(fx.state(&child), "implement");

    // Proven completion: green work committed, every declared check receipt.
    fx.write("src/lib.rs", "pub fn work() { return 7; }\n");
    write_completion(&fx, &child);
    fx.git_ok(&["add", "-A"]);
    fx.git_ok(&["commit", "--quiet", "-m", "the green landing"]);
    fx.step_ok(&child, "WCPV-019: the implement boundary passes");
    assert_eq!(fx.state(&child), "damage");
    fx.step_ok(&child, "WCPV-019: the damage boundary passes");
    assert_eq!(fx.state(&child), "done");
    let record = fx.record(&child);
    assert!(
        record.contains("state = \"done\"") && record.contains("status = \"passed\""),
        "WCPV-019: the turn ends done with the Engine-written passed fact: {record}"
    );

    // The join leaves ticket-turns; the close flips the gap, archives the
    // item, and the operator cuts the edition mark.
    fx.step_ok(RUN, "WCPV-019: the join reads the passed child");
    assert_eq!(fx.state(RUN), "close");
    let receipts = [
        format!(
            ".ratmac/evidence/{child}/completion/{}.toml",
            ratmac::completion::check_slug(PLANNED)
        ),
        format!(
            ".ratmac/evidence/{child}/completion/{}.toml",
            ratmac::completion::check_slug(HIDDEN)
        ),
        format!(
            ".ratmac/evidence/{child}/completion/{}.toml",
            ratmac::completion::check_slug(QUALITY)
        ),
    ];
    let receipt_refs: Vec<&str> = receipts.iter().map(String::as_str).collect();
    flip_residual_satisfied(&fx, &receipt_refs);
    fs::rename(
        fx.root.join(format!(".arca/ticket/{ITEM}")),
        fx.root.join(format!(".arca/ticket/archive/{ITEM}")),
    )
    .expect("archive the finished item");
    fx.git_ok(&["add", "-A"]);
    fx.git_ok(&["commit", "--quiet", "-m", "close the sprint turn"]);
    fx.git_ok(&[
        "tag",
        "-a",
        "edition-001",
        "-m",
        "fixture edition: every gate green",
    ]);
    fx.step_ok(
        RUN,
        "WCPV-019: close -> gap-check proves the gap and the mark",
    );
    assert_eq!(fx.state(RUN), "gap-check");

    // The clean verdict brings the cycle to rest.
    fx.write_verdict(RUN, "gap-check", "clean");
    fx.step_ok(RUN, "WCPV-019: the clean verdict steps to rest");
    assert_eq!(fx.state(RUN), "rest");
    let record = fx.record(RUN);
    assert!(
        record.contains("status = \"passed\""),
        "WCPV-019: rest is terminal, so arriving there passes the Run: {record}"
    );
}

/// The operator's placement for the sprint fixture: the declared roots, the
/// goal bundle, the issue bundle built from the packaged blanks, and the
/// Machine Class copied to `.ratmac/ratmac.toml`, with the bundle's own
/// self-check and the doctor read on the installed template.
fn initialize_in_place(fx: &Clean) {
    for dir in [
        ".arca/goal",
        ".arca/issue",
        ".arca/residual",
        ".arca/ticket/archive",
        ".ratmac",
    ] {
        fs::create_dir_all(fx.root.join(dir)).expect("create the declared root");
    }
    fs::write(
        fx.root.join(".arca/goal/spec.md"),
        format!(
            "# Goal spec\n\n\
             | Req ID | Requirement | Source |\n|---|---|---|\n\
             | `{REQUIREMENT}` | The demo behaves. | \
             [the demo issue](../issue/{ISSUE}/spec.md) |\n"
        ),
    )
    .expect("write the goal bundle");
    place_issue_bundle(fx);
    assert!(
        !fx.root.join(".ratmac/ratmac.toml").exists(),
        "an existing runbook is the operator's to refuse, never the Engine's to overwrite"
    );
    fs::copy(
        fx.root.join("profile/machine-class.toml"),
        fx.root.join(".ratmac/ratmac.toml"),
    )
    .expect("place the Machine Class template");

    let checked = fx.python(&["profile/checkers/verify_profile.py", "check"]);
    assert!(
        checked.status.success(),
        "the self-check exits 0: {}{}",
        out_text(&checked),
        err_text(&checked)
    );
    assert_eq!(out_text(&checked), SELF_CHECK_LINE);
    assert_eq!(err_text(&checked), "");

    let doctor = fx.rtm(&["doctor", "--json", ".ratmac/ratmac.toml"]);
    let doctor_text = support::text(&doctor);
    assert!(
        doctor.status.success(),
        "rtm doctor --json on the installed template exits 0: {doctor_text}"
    );
    assert!(
        out_text(&doctor).contains("\"findings\": []"),
        "the doctor reports an empty findings array: {doctor_text}"
    );
}

/// WCPV-020: the packaged checker's refusal rows - every twin holds the
/// valid positive control first, derives its facts from the shipped
/// manifest, and proves one refused intake step moves no Run byte.
#[test]
fn wcpv_020() {
    let fx = export_and_initialize("wcpv020");
    let started = fx.rtm(&["start"]);
    assert!(
        started.status.success(),
        "WCPV-020: the positive control start admits run-001: {}",
        support::text(&started)
    );
    assert_eq!(
        fx.state(RUN),
        "intake",
        "WCPV-020: the control Run waits at intake"
    );

    // A wrong CLI shape refuses exit 2 with the usage line.
    for wrong in [
        Vec::<&str>::new(),
        vec!["profile/checkers/verify_profile.py", "check", "extra"],
    ] {
        let mut args = vec!["profile/checkers/verify_profile.py"];
        args.extend(wrong.clone());
        let output = fx.python(&args);
        let code = output.status.code().unwrap_or(-1);
        assert_eq!(
            code,
            2,
            "WCPV-020: the wrong CLI shape {wrong:?} refuses exit 2: {}{}",
            out_text(&output),
            err_text(&output)
        );
        assert_eq!(
            err_text(&output),
            "verify_profile: usage: verify_profile.py check\n",
            "WCPV-020: the usage line is t-133's shape"
        );
    }

    // The pristine bundle, for damage and restoration.
    let pristine = walk_files(&fx.root.join("profile"));
    let rows = manifest_rows(&fx);
    let row_for = |path: &str| {
        rows.iter()
            .find(|row| row.0 == path)
            .unwrap_or_else(|| panic!("the manifest lists {path}"))
            .clone()
    };

    // One damaged bundle and its expected refusal row, driven in turn.
    struct Twin {
        because: &'static str,
        damage: Box<dyn FnOnce(&Clean)>,
        expected: String,
    }
    let schema_row = row_for("schemas/issue.toml");
    let blank_row = row_for("blanks/ticket.md");
    let checker_row = row_for("checkers/verify_issue_binding.py");
    let corrupted_row = |row: (String, String, String), observed: String| {
        format!(
            "verify_profile: the artifact {} (role {}) hashes to {observed}, but the manifest \
             records {}; restore the artifact or export a fresh bundle\n",
            row.0, row.1, row.2
        )
    };
    let schema_path = fx.root.join("profile/schemas/issue.toml");
    let blank_path = fx.root.join("profile/blanks/ticket.md");
    let checker_path = fx.root.join("profile/checkers/verify_issue_binding.py");
    let manifest_path = fx.root.join("profile/manifest.toml");
    let pristine_checker = fs::read(&checker_path).expect("read the packaged checker");
    let doctored_digest = {
        let mut digest = checker_row.2.clone();
        let first = digest.as_bytes()[0];
        digest.replace_range(0..1, if first == b'0' { "1" } else { "0" });
        digest
    };
    let truncated = {
        let bytes = fs::read(&blank_path).expect("read the blank");
        bytes[..bytes.len() / 2].to_vec()
    };
    let twins = vec![
        Twin {
            because: "the deleted schema artifact",
            damage: Box::new({
                let schema_path = schema_path.clone();
                move |_fx: &Clean| {
                    fs::remove_file(&schema_path).expect("delete the schema artifact");
                }
            }),
            expected: format!(
                "verify_profile: the manifest lists {} (role {}), but the bundle holds no such \
                 file\n",
                schema_row.0, schema_row.1
            ),
        },
        Twin {
            because: "the truncated blank",
            damage: Box::new({
                let blank_path = blank_path.clone();
                let truncated = truncated.clone();
                move |_fx: &Clean| {
                    fs::write(&blank_path, truncated).expect("truncate the blank");
                }
            }),
            expected: corrupted_row(blank_row, sha256_hex(&truncated)),
        },
        Twin {
            because: "the corrupted binding checker",
            damage: Box::new({
                let checker_path = checker_path.clone();
                move |_fx: &Clean| {
                    let mut bytes = fs::read(&checker_path).expect("read the checker");
                    bytes.extend_from_slice(b"# drifted\n");
                    fs::write(&checker_path, bytes).expect("corrupt the checker");
                }
            }),
            expected: corrupted_row(
                checker_row.clone(),
                sha256_hex(&[pristine_checker.as_slice(), b"# drifted\n".as_slice()].concat()),
            ),
        },
        Twin {
            because: "the doctored manifest digest for the binding checker",
            damage: Box::new({
                let manifest_path = manifest_path.clone();
                let recorded = checker_row.2.clone();
                let doctored = doctored_digest.clone();
                move |_fx: &Clean| {
                    let text = fs::read_to_string(&manifest_path).expect("read the manifest");
                    assert!(text.contains(&recorded), "the manifest records the digest");
                    fs::write(&manifest_path, text.replace(&recorded, &doctored))
                        .expect("doctor the manifest");
                }
            }),
            expected: corrupted_row(checker_row.clone(), sha256_hex(&pristine_checker)),
        },
        Twin {
            because: "the incompatible profile format",
            damage: Box::new({
                let manifest_path = manifest_path.clone();
                move |_fx: &Clean| {
                    let text = fs::read_to_string(&manifest_path).expect("read the manifest");
                    assert!(
                        text.contains("format = 1"),
                        "the manifest declares format 1"
                    );
                    fs::write(&manifest_path, text.replace("format = 1", "format = 2"))
                        .expect("edit the format");
                }
            }),
            expected: "verify_profile: the installed profile declares format 2, not 1; this \
                       checker reads format 1 - export a fresh bundle with a matching engine\n"
                .to_owned(),
        },
        Twin {
            because: "the unlisted extra file",
            damage: Box::new(|fx: &Clean| {
                fs::write(fx.root.join("profile/extra.txt"), "unlisted\n")
                    .expect("plant the unlisted file");
            }),
            expected: "which the manifest does not list".to_owned(),
        },
    ];

    for twin in twins {
        // Damage, then the checker's own refusal: exit 1, one stderr line,
        // nothing written anywhere.
        (twin.damage)(&fx);
        let bundle_before = walk_files(&fx.root.join("profile"));
        let checked = fx.python(&["profile/checkers/verify_profile.py", "check"]);
        let code = checked.status.code().unwrap_or(-1);
        assert_eq!(
            code,
            1,
            "WCPV-020: {} refuses exit 1: {}{}",
            twin.because,
            out_text(&checked),
            err_text(&checked)
        );
        assert!(
            out_text(&checked).is_empty(),
            "WCPV-020: {}: the checker prints nothing on stdout on refusal",
            twin.because
        );
        let stderr = err_text(&checked);
        assert_eq!(
            stderr.lines().count(),
            1,
            "WCPV-020: {}: the refusal is exactly one LF-terminated line: {stderr}",
            twin.because
        );
        assert!(
            stderr.contains(&twin.expected),
            "WCPV-020: {}: the checker's own row refuses: {stderr}",
            twin.because
        );
        if twin.because == "the unlisted extra file" {
            assert!(
                stderr.contains("extra.txt")
                    && stderr.contains(
                        "a bundle is exactly its manifest inventory - remove the file or extend \
                         the manifest through the profile's owner"
                    ),
                "WCPV-020: the unlisted row names the file and the inventory rule: {stderr}"
            );
        }
        assert_eq!(
            walk_files(&fx.root.join("profile")),
            bundle_before,
            "WCPV-020: {}: the checker writes nothing anywhere",
            twin.because
        );

        // One refused step: today's report, the command-guard envelope, the
        // checker's line as the bounded diagnostic, and no Run byte moving.
        let engine_before = fx.snapshot_engine();
        let step = fx.rtm(&["step", "--run", RUN]);
        let step_text = support::text(&step);
        assert!(
            step.status.success(),
            "WCPV-020: {}: the refused step is today's report at exit 0: {step_text}",
            twin.because
        );
        assert!(
            step_text.contains("step refused"),
            "WCPV-020: {}: the step refuses, not passes: {step_text}",
            twin.because
        );
        assert!(
            step_text.contains("command_exit: python")
                && step_text.contains("observed exit 1")
                && step_text.contains("expected exit 0"),
            "WCPV-020: {}: the refusal rides today's command-guard envelope: {step_text}",
            twin.because
        );
        assert!(
            step_text.contains(stderr.trim_end()),
            "WCPV-020: {}: the checker's line is the bounded diagnostic: {step_text}",
            twin.because
        );
        assert_eq!(
            fx.state(RUN),
            "intake",
            "WCPV-020: {}: the refusal leaves the Run parked at intake",
            twin.because
        );
        assert_eq!(
            fx.status(RUN),
            "planned",
            "WCPV-020: {}: the refusal leaves the Run planned",
            twin.because
        );
        fx.assert_engine_untouched(&engine_before, RUN, twin.because);

        // Restore exactly, so the next twin starts from the shipped bundle.
        let profile = fx.root.join("profile");
        fs::remove_dir_all(&profile).expect("remove the damaged bundle");
        for (relative, bytes) in &pristine {
            let path = profile.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("restore the bundle's directories");
            }
            fs::write(&path, bytes).expect("restore the bundle's files");
        }
        assert_eq!(
            walk_files(&profile),
            pristine,
            "WCPV-020: {}: the restoration returns the shipped bytes",
            twin.because
        );
    }

    // No false pass: the restored bundle verifies again.
    let checked = fx.python(&["profile/checkers/verify_profile.py", "check"]);
    assert!(
        checked.status.success() && out_text(&checked) == SELF_CHECK_LINE,
        "WCPV-020: the restored bundle passes its own self-check: {}{}",
        out_text(&checked),
        err_text(&checked)
    );
}

/// WCPV-021: the export's atomic write discipline - never overwriting, the
/// hidden partial sibling, the leftover refusal - two-export determinism,
/// and the runtime ignore boundaries after initialization.
#[test]
fn wcpv_021() {
    // The plain tree: no Git, no Engine root - the export may run anywhere.
    let owner = TempTree::new("t135-wcpv021").expect("create the plain tree");
    let plain = owner.path();
    fs::write(plain.join("keep.txt"), "pre-existing byte\n").expect("write a sentinel");

    // The positive control: one fresh-path export, every added byte under
    // the target, every pre-existing byte unchanged.
    let before = support::capture(plain, CaptureOptions::default())
        .expect("snapshot the plain tree")
        .tagged();
    let exported = support::command(ratmac_qa::engine_bin!(), plain)
        .args(["profile", "plan-build", "profile"])
        .output()
        .expect("invoke rtm");
    let exported_text = support::text(&exported);
    assert!(
        exported.status.success(),
        "WCPV-021: the fresh-path export exits 0: {exported_text}"
    );
    assert_eq!(out_text(&exported), export_success_line("profile"));
    assert_eq!(err_text(&exported), "");
    let after = support::capture(plain, CaptureOptions::default())
        .expect("snapshot the exported tree")
        .tagged();
    for key in after.keys() {
        assert!(
            key.starts_with("profile/") || before.contains_key(key),
            "WCPV-021: every written byte sits under the one target folder, not {key:?}"
        );
    }
    for (key, bytes) in &before {
        assert_eq!(
            after.get(key),
            Some(bytes),
            "WCPV-021: no pre-existing byte changes: {key:?}"
        );
    }
    let first_export = walk_files(&plain.join("profile"));

    // The clean-repository initialization control: export, place, start.
    let fx = Clean::create("wcpv021");
    let repo_export = fx.rtm(&["profile", "plan-build", "profile"]);
    assert!(
        repo_export.status.success(),
        "WCPV-021: the clean repository's export exits 0: {}",
        support::text(&repo_export)
    );
    for dir in [
        ".arca/goal",
        ".arca/issue",
        ".arca/residual",
        ".arca/ticket/archive",
        ".ratmac",
    ] {
        fs::create_dir_all(fx.root.join(dir)).expect("create the declared root");
    }
    fs::copy(
        fx.root.join("profile/machine-class.toml"),
        fx.root.join(".ratmac/ratmac.toml"),
    )
    .expect("place the Machine Class template");
    let bundle_at_start = walk_files(&fx.root.join("profile"));
    let started = fx.rtm(&["start"]);
    assert!(
        started.status.success(),
        "WCPV-021: the initialized repository starts a Run: {}",
        support::text(&started)
    );

    // An occupied target - pre-created empty, so only an existence check can
    // refuse it - refuses having written nothing.
    fs::create_dir_all(plain.join("occupied")).expect("pre-create the occupied path");
    let settled = support::capture(plain, CaptureOptions::default())
        .expect("snapshot around the occupied refusal")
        .tagged();
    let occupied = support::command(ratmac_qa::engine_bin!(), plain)
        .args(["profile", "plan-build", "occupied"])
        .output()
        .expect("invoke rtm");
    let occupied_text = support::text(&occupied);
    assert_eq!(
        occupied.status.code(),
        Some(2),
        "WCPV-021: an occupied path refuses at the usage exit: {occupied_text}"
    );
    assert!(
        out_text(&occupied).is_empty(),
        "WCPV-021: the refusal prints nothing on stdout: {occupied_text}"
    );
    assert_eq!(
        occupied_text.lines().count(),
        1,
        "WCPV-021: the refusal is exactly one line: {occupied_text}"
    );
    assert!(
        occupied_text.starts_with("rtm: profile: ")
            && occupied_text.contains(
                "profile: occupied already exists; the profile export never overwrites - choose a \
                 new path or delete that one yourself"
            ),
        "WCPV-021: the occupied refusal names the path and the never-overwrite rule: {occupied_text}"
    );
    assert_eq!(
        support::capture(plain, CaptureOptions::default())
            .expect("re-snapshot the plain tree")
            .tagged(),
        settled,
        "WCPV-021: the refused write leaves the whole tree byte-identical"
    );

    // A missing parent refuses: the export creates exactly one folder and no
    // directories.
    let no_parent = support::command(ratmac_qa::engine_bin!(), plain)
        .args(["profile", "plan-build", "no/such/parent/profile"])
        .output()
        .expect("invoke rtm");
    let no_parent_text = support::text(&no_parent);
    assert_eq!(
        no_parent.status.code(),
        Some(2),
        "WCPV-021: a missing parent refuses at the usage exit: {no_parent_text}"
    );
    assert!(
        out_text(&no_parent).is_empty()
            && no_parent_text.starts_with("rtm: profile: ")
            && no_parent_text.contains(
                "profile: the directory for no/such/parent/profile does not exist; the profile \
                 export creates exactly one folder and no directories"
            ),
        "WCPV-021: the no-parent refusal names the rule: {no_parent_text}"
    );
    assert!(
        !plain.join("no").exists(),
        "WCPV-021: the refused export creates no directory"
    );

    // An interrupted export: the fault seam kills the process after the
    // manifest lands in the partial sibling - the target stays absent, every
    // pre-existing byte is unchanged, and the leftover sits beside it.
    let partial = plain.join(".broken.rtm-profile-partial");
    let settled = support::capture(
        plain,
        CaptureOptions {
            excluded_paths: &[partial.clone()],
            ..CaptureOptions::default()
        },
    )
    .expect("snapshot around the interruption")
    .tagged();
    let interrupted = support::command(ratmac_qa::engine_bin!(), plain)
        .env("RATMAC_TEST_PROFILE_FAULT", "after-manifest")
        .args(["profile", "plan-build", "broken"])
        .output()
        .expect("spawn the interrupted export");
    assert!(
        !interrupted.status.success(),
        "WCPV-021: the fault seam kills the export mid-flight"
    );
    assert!(
        !plain.join("broken").exists(),
        "WCPV-021: an interrupted export never leaves a half folder at the target"
    );
    assert!(
        partial.is_dir(),
        "WCPV-021: the half-built bundle sits at the hidden partial sibling"
    );
    let partial_files = walk_files(&partial);
    assert_eq!(
        partial_files.keys().collect::<Vec<_>>(),
        vec!["manifest.toml"],
        "WCPV-021: the seam dies right after the manifest lands: {partial_files:?}"
    );
    assert_eq!(
        support::capture(
            plain,
            CaptureOptions {
                excluded_paths: &[partial.clone()],
                ..CaptureOptions::default()
            }
        )
        .expect("re-snapshot after the interruption")
        .tagged(),
        settled,
        "WCPV-021: the interrupted export leaves every pre-existing byte unchanged"
    );

    // The next attempt refuses Leftover naming it; the Engine deletes
    // nothing and writes past nothing.
    let leftover = support::command(ratmac_qa::engine_bin!(), plain)
        .args(["profile", "plan-build", "broken"])
        .output()
        .expect("invoke rtm");
    let leftover_text = support::text(&leftover);
    assert_eq!(
        leftover.status.code(),
        Some(2),
        "WCPV-021: the leftover blocks the write: {leftover_text}"
    );
    assert!(
        out_text(&leftover).is_empty()
            && leftover_text.starts_with("rtm: profile: ")
            && leftover_text.contains(".broken.rtm-profile-partial")
            && leftover_text.contains(
                "an interrupted write left ./.broken.rtm-profile-partial behind; no partial \
                 folder ever sits at the requested path - remove the leftover yourself and run \
                 the command again"
            ),
        "WCPV-021: the leftover refusal names the leftover and the rule: {leftover_text}"
    );

    // Once the operator clears the leftover, the retry succeeds and its
    // bytes equal an uninterrupted export's.
    fs::remove_dir_all(&partial).expect("the operator removes the leftover");
    let retried = support::command(ratmac_qa::engine_bin!(), plain)
        .args(["profile", "plan-build", "broken"])
        .output()
        .expect("invoke rtm");
    let retried_text = support::text(&retried);
    assert!(
        retried.status.success(),
        "WCPV-021: the retry after cleanup succeeds: {retried_text}"
    );
    assert_eq!(out_text(&retried), export_success_line("broken"));
    assert_eq!(
        walk_files(&plain.join("broken")),
        first_export,
        "WCPV-021: the retried export's bytes equal an uninterrupted export's"
    );

    // Two exports of the same version contain the same bytes.
    let twin = support::command(ratmac_qa::engine_bin!(), plain)
        .args(["profile", "plan-build", "twin"])
        .output()
        .expect("invoke rtm");
    assert!(
        twin.status.success(),
        "WCPV-021: the second export succeeds: {}",
        support::text(&twin)
    );
    assert_eq!(
        walk_files(&plain.join("twin")),
        first_export,
        "WCPV-021: two exports of the same version contain the same file set with the same bytes"
    );

    // The runtime ownership boundaries after initialization and start: the
    // four policy lines, the ignored runtime probes, the visible authoring
    // probes - the bundle included - and no Engine write touching it.
    assert_eq!(
        fs::read(fx.root.join(".ratmac/.gitignore")).expect("read the installed policy"),
        FRESH_POLICY.to_vec(),
        "WCPV-021: .ratmac/.gitignore carries exactly t-121's four policy lines"
    );
    let ignored = [
        ".ratmac/runs/run-001",
        ".ratmac/mint.toml",
        ".ratmac/locks/root.lock",
        ".ratmac/log.md",
    ];
    let visible = [
        ".ratmac/.gitignore",
        ".ratmac/ratmac.toml",
        ".ratmac/evidence/run-001",
        "profile/",
    ];
    let mut probes: Vec<&str> = Vec::new();
    probes.extend(ignored);
    probes.extend(visible);
    let check = support::git(&fx.root)
        .args({
            let mut args = vec!["check-ignore", "-v", "--"];
            args.extend(probes);
            args
        })
        .output()
        .expect("invoke git check-ignore");
    assert!(
        check.status.success(),
        "WCPV-021: git check-ignore runs over the probes: {}",
        support::text(&check)
    );
    let listed = String::from_utf8_lossy(&check.stdout).into_owned();
    for probe in ignored {
        assert!(
            listed.contains(probe),
            "WCPV-021: {probe} answers ignored: {listed}"
        );
    }
    for probe in visible {
        assert!(
            !listed.contains(probe),
            "WCPV-021: {probe} stays visible to Git: {listed}"
        );
    }
    assert_eq!(
        walk_files(&fx.root.join("profile")),
        bundle_at_start,
        "WCPV-021: no Engine write touches the bundle"
    );
    assert_eq!(
        bundle_at_start, first_export,
        "WCPV-021: the installed bundle is the deterministic export's bytes"
    );
}
