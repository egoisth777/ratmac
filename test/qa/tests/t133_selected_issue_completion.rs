//! t-133 / WCPV-007, WCPV-008, WCPV-009: selected issue completion (WCP-003).
//!
//! The self-development workflow binds its selected issue and requirement set
//! to its work item, its tested source snapshot, its red and green evidence,
//! and its independent review decision; completion refuses absent, mismatched,
//! stale, fabricated-by-the-working-agent, or unrelated evidence, and the
//! parent consumes only the properly bound terminal review result.
//!
//! Every fixture constructs the edited Machine Class exactly as the accepted
//! contract says - the recorded base runbook bytes with the three tracked
//! boundary diffs applied in ticket order (t-130's
//! `.ratmac/pause-routes.diff`, then t-131's `.ratmac/review-proofs.diff`,
//! then t-133's `.ratmac/issue-binding.diff`) - never by reading the tracked
//! runbook as the edited class. WCPV-007 drives the real `rtm` binary and the
//! real validator against an isolated fixture repository; WCPV-008 and
//! WCPV-009 reuse t-129's in-process `cli::run_from` trust injection with
//! fixture keys exercised through the host's real `ssh-keygen`, exactly as the
//! landed suites do. Every refusal twin first establishes its valid positive
//! control and asserts unchanged State and Status after the refusal.

use ratmac::cli;
use ratmac::doctor::{self, Severity};
use ratmac::machine::{GuardKind, MachineClass};
use ratmac::receipt::sha256_text;
use ratmac::review::test::{install, TestTrust, TrustGuard};
use ratmac_qa::support::{self, CaptureOptions, TempTree};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The host verifier this suite pins, per the accepted design.
const KEYGEN: &str = "C:/WINDOWS/System32/OpenSSH/ssh-keygen.exe";
const REVIEW_NAMESPACE: &str = "ratmac-review-v1";
const APPROVE_NAMESPACE: &str = "ratmac-approve-v1";

/// The green output a receipt records for a check that passed.
const GREEN: &str = "test result: ok. 1 passed; 0 failed\n";

/// The red output a sensitivity receipt records for a planned test that
/// failed before its implementation existed.
const RED: &str = "test result: FAILED. 0 passed; 1 failed\n";

/// The item binding value: the item record's filename.
const ITEM: &str = "t-100.md";
/// The selected issue the positive turns work.
const ISSUE: &str = "i-100-demo";
/// One unrelated issue in the same namespace, for the unrelated-evidence
/// twins: its identifiers resolve in no other bundle.
const OTHER_ISSUE: &str = "i-200-other";
/// An issue id that resolves at no location of the issue namespace.
const MISSING_ISSUE: &str = "i-404-none";

const PLANNED_A: &str = "PT-100-01";
const PLANNED_B: &str = "PT-100-02";
const HIDDEN_LANE: &str = "HT-100-01";
const QUALITY: &str = "cargo --version";

// --- the edited Machine Class --------------------------------------------------

/// The recorded `.ratmac/ratmac.toml` bytes of this batch's merge base, the
/// base the three tracked boundary diffs apply over - a byte-identical copy
/// of the embedded base of t-130 and t-131, embedded so the tracked runbook
/// is never read as the edited class: the cycle boundary wires the diffs into
/// it, and this suite must stay green through that wiring.
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

/// The `implement` State's final prompt body - t-131's contract string, which
/// t-133's hunk keeps byte for byte.
const IMPLEMENT_PROMPT: &str = r#"Make the planned tests pass - the focused public checks green - then obtain
the independent source and test-oracle review and record it through the
turn tool before the expensive proof set: pwsh -File tools/turn.ps1 review
-Item <item> -Verdict <accepted|rejected>, then pwsh -File tools/turn.ps1
proofs -Item <item>. A rejected review returns here for repair. Then run
the item's whole merge gate: its focused tests, the affected regressions,
its hidden lanes, and the quality commands. Commit the green work before
going on - the next stage breaks the tree on purpose and has to have
something to restore.
"#;

/// The `damage` State's final prompt body - t-131's contract string, which
/// t-133's hunk keeps byte for byte.
const DAMAGE_PROMPT: &str = r#"Break the green work on purpose, one mutation at a time, and check that a
test catches each one. Freeze the safety checkpoint through the turn tool
first: pwsh -File tools/turn.ps1 checkpoint -Item <item> -Target <path>...;
then mutate only the declared targets and restore after every mutation:
pwsh -File tools/turn.ps1 restore -Item <item>. Re-run the focused checks
green after every restore; the restored-green rerun is mandatory, not
re-earned proof. A mutation that survives is a hole in the tests, not a
win.
"#;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The edited Machine Class: the recorded base bytes with the three tracked
/// boundary diffs applied in ticket order - t-130's, then t-131's, then
/// t-133's - bytes that equal the tracked runbook after wiring.
fn edited_runbook() -> String {
    assert_eq!(
        sha256_text(BASE_RUNBOOK),
        BASE_RUNBOOK_SHA256,
        "the embedded base copy is the recorded merge-base runbook"
    );
    let tree = TempTree::new("t133-class").expect("create the class wiring tree");
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
    fs::read_to_string(repo.join(".ratmac/ratmac.toml")).expect("read the wired runbook bytes")
}

/// The validator row every working State prepends, matched as the first guard.
fn assert_validator_row(guard: &GuardKind, because: &str) {
    match guard {
        GuardKind::CommandExit {
            program,
            args,
            expected,
            ..
        } => {
            assert_eq!(program, "python", "{because}");
            assert_eq!(
                args,
                &[
                    "tools/verify_issue_binding.py".to_owned(),
                    "check".to_owned()
                ],
                "{because}"
            );
            assert_eq!(*expected, 0, "{because}");
        }
        other => panic!("{because}: the first guard is {other:?}, not the validator row"),
    }
}

fn severity_word(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

/// The edited class's declared shape: the second binding beside `item`, the
/// spawn row binding both, the validator row prepended before the receipt
/// guard it accompanies in every working State, the two replaced prompts, and
/// t-131's prompt bytes kept - and `rtm doctor` clean on the edited bytes.
fn assert_edited_class(edited: &str) {
    let class = MachineClass::from_toml(edited)
        .expect("the edited machine class parses through the one reader");

    let turns = class
        .states()
        .get("ticket-turns")
        .expect("the edited class carries the ticket-turns State");
    assert_eq!(
        turns.prompt(),
        TICKET_TURNS_PROMPT,
        "the ticket-turns prompt is the contract's final string, byte for byte"
    );
    let spawn = turns
        .spawns()
        .iter()
        .find(|spawn| spawn.name() == "turn")
        .expect("the ticket-turns State declares the turn spawn");
    assert_eq!(spawn.class(), "ticket");
    assert_eq!(
        spawn.bind(),
        &["item".to_owned(), "issue".to_owned()],
        "the spawn row's binding list is [\"item\", \"issue\"]"
    );

    let ticket = class
        .classes()
        .get("ticket")
        .expect("the edited class carries the ticket class");
    let bindings: Vec<&str> = ticket.bindings().keys().map(String::as_str).collect();
    assert_eq!(
        bindings,
        vec!["issue", "item"],
        "the ticket class declares the second binding beside item, both required"
    );
    for name in ["item", "issue"] {
        assert!(
            ticket.bindings()[name].required(),
            "the {name} binding is required"
        );
    }

    let tests = ticket
        .states()
        .get("tests")
        .expect("the ticket class carries the tests State");
    assert_eq!(
        tests.prompt(),
        TESTS_PROMPT,
        "the tests prompt is the contract's final string, byte for byte"
    );
    assert_eq!(
        tests.guards().len(),
        2,
        "the tests State carries two guards"
    );
    assert_validator_row(
        &tests.guards()[0],
        "every working State prepends the validator row before the receipt guard it accompanies",
    );
    assert!(matches!(
        tests.guards()[1],
        GuardKind::SensitivityReceipts { .. }
    ));

    let implement = ticket
        .states()
        .get("implement")
        .expect("the ticket class carries the implement State");
    assert_eq!(
        implement.prompt(),
        IMPLEMENT_PROMPT,
        "the implement prompt keeps the bytes t-131's boundary diff writes"
    );
    assert_eq!(implement.guards().len(), 3);
    assert_validator_row(
        &implement.guards()[0],
        "the implement State prepends the validator row",
    );
    assert!(matches!(
        implement.guards()[1],
        GuardKind::CompletionGate { .. }
    ));
    assert!(matches!(
        &implement.guards()[2],
        GuardKind::CommandExit { program, .. } if program == "git"
    ));

    let damage = ticket
        .states()
        .get("damage")
        .expect("the ticket class carries the damage State");
    assert_eq!(
        damage.prompt(),
        DAMAGE_PROMPT,
        "the damage prompt keeps the bytes t-131's boundary diff writes"
    );
    assert_eq!(damage.guards().len(), 2);
    assert_validator_row(
        &damage.guards()[0],
        "the damage State prepends the validator row",
    );
    assert!(matches!(
        damage.guards()[1],
        GuardKind::CompletionGate { .. }
    ));

    let doctor_tree = TempTree::new("t133-doctor").expect("create the doctor tree");
    let path = doctor_tree.path().join("ratmac.toml");
    fs::write(&path, edited).expect("write the edited class for its own doctor");
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
        "rtm doctor exits 0 on the edited class exactly as t092's pin demands: {shown:?}"
    );
    assert_eq!(doctor::exit_code(&findings), 0);
}

// --- shared plumbing -----------------------------------------------------------

/// One in-process Engine invocation: its exit code, its stdout bytes, and the
/// refusal message it returned (empty on success).
struct Ran {
    code: i32,
    stdout: String,
    message: String,
}

/// How a fixture drives the Engine: the real binary, or t-129's in-process
/// library entry when the installed trust reader must participate.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Inv {
    Process,
    Library,
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

/// The declared policy digest exactly as the contract frames it:
/// `ratmac-policy-v1`, one NUL byte, then the Approver label, the minimum in
/// canonical decimal, the role combination, and each reviewer sorted by
/// label as framed label then key.
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

/// The canonical intent's binding digest over the recorded binding pairs,
/// framed exactly as the contract frames it: sorted `name=value` pairs, each
/// length-framed, hashed once. The intent frames the digest as its hex
/// string.
fn binding_digest_hex(pairs: &[(&str, &str)]) -> String {
    let mut joined: Vec<String> = pairs
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    joined.sort();
    let mut body = Vec::new();
    for pair in joined {
        frame(&mut body, pair.as_bytes());
    }
    let mut hasher = Sha256::new();
    hasher.update(&body);
    format!("{:x}", hasher.finalize())
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
struct Keys {
    dir: PathBuf,
    publics: BTreeMap<String, String>,
}

impl Keys {
    fn create(dir: &Path, names: &[&str]) -> Self {
        fs::create_dir_all(dir).expect("create the fixture key directory");
        let mut publics = BTreeMap::new();
        for name in names {
            let path = dir.join(name);
            let generated = ssh(
                dir,
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
                "generate fixture key {name}: {}",
                support::text(&generated)
            );
            let public = fs::read_to_string(dir.join(format!("{name}.pub")))
                .unwrap_or_else(|error| panic!("read {name}.pub: {error}"));
            publics.insert((*name).to_owned(), public.trim().to_owned());
        }
        Self {
            dir: dir.to_path_buf(),
            publics,
        }
    }

    fn public(&self, name: &str) -> &str {
        self.publics
            .get(name)
            .unwrap_or_else(|| panic!("fixture key {name} exists"))
    }

    /// One detached signature over `payload`'s bytes under `namespace`. The
    /// host ssh-keygen never overwrites an existing `.sig`, so every signer
    /// signs a fresh per-signer copy of the same bytes.
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
            "each signer needs a fresh payload copy: {}",
            shown(&copy)
        );
        let bytes = fs::read(payload)
            .unwrap_or_else(|error| panic!("read the payload for {name}: {error}"));
        fs::write(&copy, bytes).unwrap_or_else(|error| panic!("write {name}'s copy: {error}"));
        let key = self.dir.join(name);
        let signed = ssh(
            &self.dir,
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

/// One signed authorization bundle ready to submit.
struct Bundle {
    reviews: Vec<(String, PathBuf)>,
    approve: PathBuf,
}

/// The fixture-declared review policy and its installed trust reader.
struct Governance {
    keys: Keys,
    _trust: TrustGuard,
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
    let primary = text
        .split("\n\n")
        .find(|record| !record.lines().any(|line| line == "bare"))
        .and_then(|record| {
            record
                .lines()
                .find_map(|line| line.strip_prefix("worktree "))
        })
        .unwrap_or_else(|| panic!("the fixture repository reports a primary worktree: {text}"));
    PathBuf::from(primary)
}

/// The project-scope spelling: the platform's canonicalized primary checkout
/// with forward slashes.
fn project_scope(root: &Path) -> String {
    let canonical = fs::canonicalize(resolved_primary(root))
        .unwrap_or_else(|error| panic!("canonicalize the fixture primary checkout: {error}"));
    shown(&canonical)
}

impl Governance {
    /// Generate the bench (reviewers a and b, the Approver lead, one outsider
    /// for the wrong-reviewer twin), write the enrollment manifest and the
    /// signers file, append the fixture-declared review table - whose `work`
    /// array names the tested roots - to the edited ticket class, and install
    /// the trust reader for the fixture's Engine root.
    fn create(owner: &TempTree, root: &Path, runbook: &mut String) -> Self {
        let keys = Keys::create(&owner.join("keys"), &["a", "b", "lead", "outsider"]);
        let reviewers: Vec<(String, String)> = ["a", "b", "lead"]
            .iter()
            .map(|label| ((*label).to_owned(), keys.public(label).to_owned()))
            .collect();
        let mut signers = String::new();
        for (label, public) in &reviewers {
            let parts: Vec<&str> = public.split_whitespace().collect();
            assert!(
                parts.len() >= 2 && parts[0] == "ssh-ed25519",
                "fixture public key for {label} is one OpenSSH line"
            );
            signers.push_str(&format!("{} {} {}\n", label, parts[0], parts[1]));
        }
        let signers_path = owner.join("keys/allowed_signers");
        fs::write(&signers_path, &signers).expect("write the fixture signers file");
        let verifier_digest = sha256_hex(&fs::read(KEYGEN).expect("read the pinned verifier"));
        let signers_digest = sha256_hex(signers.as_bytes());
        let true_digest = policy_digest("lead", 2, false, &reviewers);
        let scope = project_scope(root);
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
        manifest.push_str(&format!(
            "actor = \"{}\"\n",
            builder_actor().replace('\\', "\\\\")
        ));
        manifest.push_str("keys = []\n");
        for (label, public) in &reviewers {
            manifest.push_str("\n[[reviewers]]\n");
            manifest.push_str(&format!("label = \"{label}\"\n"));
            manifest.push_str(&format!("key = \"{public}\"\n"));
        }
        let manifest_bytes = manifest.clone().into_bytes();
        let manifest_path = owner.join("keys/enrollment-manifest.toml");
        fs::write(&manifest_path, &manifest_bytes).expect("write the enrollment manifest");
        runbook.push_str("\n[classes.ticket.review]\n");
        runbook.push_str(&format!("manifest = \"{}\"\n", shown(&manifest_path)));
        runbook.push_str(&format!("policy-digest = \"{true_digest}\"\n"));
        runbook.push_str(&format!("verifier-digest = \"{verifier_digest}\"\n"));
        runbook.push_str(&format!("signers-digest = \"{signers_digest}\"\n"));
        runbook.push_str("minimum-reviews = 2\n");
        runbook.push_str("approver = \"lead\"\n");
        runbook.push_str("approver-may-review = false\n");
        runbook.push_str("work = [\"src\", \"test\", \".arca/issue\", \".arca/ticket\"]\n");
        for (label, public) in &reviewers {
            runbook.push_str("\n[[classes.ticket.review.reviewers]]\n");
            runbook.push_str(&format!("label = \"{label}\"\n"));
            runbook.push_str(&format!("key = \"{public}\"\n"));
        }
        let trust = install(
            &root.join(".ratmac"),
            TestTrust {
                manifest_bytes,
                signers_path,
                verifier_path: PathBuf::from(KEYGEN),
                verifier_sha256: verifier_digest,
            },
        );
        Self {
            keys,
            _trust: trust,
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

/// One isolated fixture repository carrying the edited Machine Class, the
/// workflow validator beside its runbook, the issue namespace, and the cycle
/// artifacts each stage's guards read.
struct Fixture {
    owner: TempTree,
    root: PathBuf,
    /// The edited class bytes this fixture wrote, for the shape oracle.
    runbook: String,
    inv: Inv,
    governed: Option<Governance>,
}

/// The work item P3 cuts for the gap, carrying the declaration lists the
/// completion guard selects. `focused` names the focused check - the
/// declaration the check-mutation twin edits.
fn ticket_body(focused: &str) -> String {
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
         planned-test-refs:\n  - \"PT-100-01\"\n  - \"PT-100-02\"\n\
         focused-tests:\n  - \"{focused}\"\n\
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

/// One issue bundle in the three-location namespace: an index, a spec
/// carrying its requirement row, and a test plan carrying its verification
/// rows. The unrelated issue ends rejected - its ask never reaches the goal,
/// so the intake gate's accepted-ask resolution never applies to it, and the
/// twin's refusal is unaffected: the validator resolves the bundle by
/// location and checks the worked requirement's occurrence in its spec.
fn write_issue_bundle(
    root: &Path,
    id: &str,
    status: &str,
    disposition: &str,
    requirement: &str,
    checks: &[&str],
) {
    let dir = root.join(format!(".arca/issue/{id}"));
    fs::create_dir_all(&dir).expect("create the issue folder");
    fs::write(
        dir.join("index.md"),
        format!(
            "# Issue {id}\n\n\
             ```yaml\nissue-id: \"{id}\"\nstatus: \"{status}\"\n```\n\n\
             See [goal spec](../../goal/spec.md).\n"
        ),
    )
    .expect("write issue index");
    fs::write(
        dir.join("spec.md"),
        format!(
            "# Requirement records\n\n\
             | Req ID | Requirement | Status |\n|---|---|---|\n\
             | `{requirement}` | The {requirement} behaves. | {disposition} |\n"
        ),
    )
    .expect("write issue spec");
    for name in ["design.md", "ubi-lang.md"] {
        fs::write(dir.join(name), format!("# {name}\n")).expect("write issue file");
    }
    let rows: String = checks
        .iter()
        .map(|check| {
            format!("| `{check}` | [`{requirement}`](spec.md#requirement-records) | Planned. |\n")
        })
        .collect();
    fs::write(
        dir.join("test-plan.md"),
        format!(
            "# test-plan.md\n\n\
             | Check | Requirement Refs | Expected evidence |\n|---|---|---|\n{rows}"
        ),
    )
    .expect("write issue test plan");
}

impl Fixture {
    fn create(label: &str, inv: Inv) -> Self {
        let owner = TempTree::new(&format!("t133-{label}")).expect("create the fixture tree");
        let root = owner.join("repo");
        let mut runbook = edited_runbook();
        for dir in [
            ".arca/goal",
            ".arca/issue",
            ".arca/residual",
            ".arca/ticket/archive",
            ".ratmac",
            "src",
            "test",
            "tools",
        ] {
            fs::create_dir_all(root.join(dir)).expect("create the fixture tree");
        }
        let git_ok = |args: &[&str]| {
            let output = support::git(&root).args(args).output().expect("invoke git");
            assert!(
                output.status.success(),
                "git {args:?} seeds the fixture: {}",
                support::text(&output)
            );
        };
        git_ok(&["init", "--quiet"]);
        // The governance bench resolves the project scope through Git itself,
        // so it needs the initialized repository and the tree it scopes.
        let governed =
            (inv == Inv::Library).then(|| Governance::create(&owner, &root, &mut runbook));
        fs::write(root.join(".ratmac/ratmac.toml"), &runbook)
            .expect("write the edited machine class");
        fs::copy(
            repo_root().join("tools/verify_issue_binding.py"),
            root.join("tools/verify_issue_binding.py"),
        )
        .expect("ship the workflow validator beside the fixture runbook");
        fs::write(
            root.join(".gitignore"),
            ".ratmac/runs/\n.ratmac/mint.toml\n.ratmac/locks/\n.ratmac/log.md\n",
        )
        .expect("write fixture gitignore");
        fs::write(root.join("src/lib.rs"), "pub fn work() {}\n").expect("write fixture source");
        fs::write(
            root.join("test/fixture_test.rs"),
            "fn the_planned_test() {}\nfn the_second_planned_test() {}\n",
        )
        .expect("write fixture tests");
        fs::write(
            root.join(".arca/schema.md"),
            "# Working rules\n\n### AUTH-001 - the contributor rule\n\nProse.\n",
        )
        .expect("write fixture rules");
        fs::write(
            root.join(".arca/goal/spec.md"),
            "# Goal spec\n\n\
             | Req ID | Requirement | Source |\n|---|---|---|\n\
             | DEMO-001 | The demo behaves. | \
             [issue DEMO-001](../issue/i-100-demo/spec.md#requirement-records) |\n",
        )
        .expect("write fixture goal");
        write_issue_bundle(
            &root,
            ISSUE,
            "integrated",
            "accepted",
            "DEMO-001",
            &[PLANNED_A, PLANNED_B, HIDDEN_LANE],
        );
        write_issue_bundle(
            &root,
            OTHER_ISSUE,
            "rejected",
            "rejected",
            "DEMO-002",
            &["PT-200-01", "HT-200-01"],
        );
        git_ok(&["config", "user.email", "fixture@example.invalid"]);
        git_ok(&["config", "user.name", "Fixture"]);
        git_ok(&["config", "core.autocrlf", "false"]);
        git_ok(&["add", "-A"]);
        git_ok(&["commit", "--quiet", "-m", "seed the cycle fixture"]);
        Self {
            owner,
            root,
            runbook,
            inv,
            governed,
        }
    }

    fn write(&self, relative: &str, body: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture directory");
        }
        fs::write(path, body).expect("write fixture file");
    }

    /// Write into one of the fixture's linked worktrees.
    fn ws_write(&self, workspace: &Path, relative: &str, body: &str) {
        let path = workspace.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create worktree directory");
        }
        fs::write(path, body).expect("write worktree file");
    }

    fn commit_at(&self, dir: &Path, message: &str) {
        let output = support::git(dir)
            .args(["add", "-A"])
            .output()
            .expect("invoke git add");
        assert!(
            output.status.success(),
            "git add in {message:?}: {}",
            support::text(&output)
        );
        let output = support::git(dir)
            .args(["commit", "--quiet", "-m", message])
            .output()
            .expect("invoke git commit");
        assert!(
            output.status.success(),
            "git commit {message:?}: {}",
            support::text(&output)
        );
    }

    /// The tree's status bytes at `dir` - cycle work legitimately leaves
    /// untracked artifacts, so write-freedom is proven by identical status
    /// bytes, not by an empty status.
    fn status_porcelain(&self, dir: &Path) -> String {
        let output = support::git(dir)
            .args(["status", "--porcelain"])
            .output()
            .expect("invoke git status");
        assert!(
            output.status.success(),
            "git status runs in the fixture tree: {}",
            support::text(&output)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// One linked worktree of the fixture repository, the topology the real
    /// workflow's turns run in: full tracked tree, no Engine state.
    /// Live is the ledger's own meaning (`abandoned = false`), so a finished
    /// turn's entry stays live and one workspace carries one turn - every twin
    /// gets its own worktree, and none shares a workspace with another live
    /// entry unless the ambiguity row is the asserted refusal.
    fn add_worktree(&self, name: &str) -> PathBuf {
        let path = self.owner.join(name);
        let output = support::git(&self.root)
            .args([
                "worktree",
                "add",
                "--quiet",
                "-b",
                &format!("wt-{name}"),
                &path.to_string_lossy(),
            ])
            .output()
            .expect("invoke git worktree add");
        assert!(
            output.status.success(),
            "git worktree add {name}: {}",
            support::text(&output)
        );
        path
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    fn ran(&self, args: &[&str]) -> Ran {
        let mut full = vec!["rtm".to_owned()];
        full.extend(args.iter().map(|arg| (*arg).to_owned()));
        let mut stdout = Vec::new();
        match cli::run_from(full, &self.root, &mut stdout) {
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

    /// One Engine invocation through the fixture's mode, asserting exit 0.
    fn ok(&self, args: &[&str], why: &str) -> String {
        match self.inv {
            Inv::Process => {
                let output = self.rtm(args);
                let text = combined(&output);
                assert!(output.status.success(), "{why}: {text}");
                text
            }
            Inv::Library => {
                let ran = self.ran(args);
                let text = format!("{}{}", ran.stdout, ran.message);
                assert_eq!(ran.code, 0, "{why}: {text}");
                text
            }
        }
    }

    /// One `rtm step` invocation asserting today's refusal report.
    fn step_refused_text(&self, run: &str, why: &str) -> String {
        let text = self.ok(&["step", "--run", run], why);
        assert!(
            text.contains("step refused"),
            "{why}: the step refuses, not passes: {text}"
        );
        text
    }

    fn start(&self) -> String {
        let text = self.ok(&["start"], "the cycle Run is admitted");
        text.split("started run ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .expect("start names the minted run id")
            .to_owned()
    }

    fn step_ok(&self, run: &str, why: &str) {
        self.ok(&["step", "--run", run], why);
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

    /// A refused twin leaves State and Status untouched.
    fn assert_parked(&self, run: &str, state: &str) {
        assert_eq!(
            self.state(run),
            state,
            "the refusal leaves the Run parked at {state}"
        );
        assert_eq!(
            self.status(run),
            "planned",
            "the refusal leaves the Run planned - only resume flips executing, \\
             and only the Engine writes passed"
        );
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

    /// Walk one cycle Run from `rtm start` to the spawning State, seeding the
    /// gap record and the work item each stage's guards read.
    fn walk_to_ticket_turns(&self) -> String {
        let parent = self.start();
        assert_eq!(self.state(&parent), "intake", "a Run starts at intake");
        self.step_ok(&parent, "intake -> gap-check freezes the goal");
        assert_eq!(self.state(&parent), "gap-check");
        let frozen = self.frozen(&parent);
        self.write(
            ".arca/residual/res-100.md",
            &format!(
                "# Residual Record\n\n```yaml\n\
                 residual-id: \"res-100\"\n\
                 goal-requirement-ref: \"DEMO-001\"\n\
                 frozen-goal-bundle-revision: \"goal-sha256:{frozen}\"\n\
                 concrete-evidence-refs: []\n\
                 status: \"missing\"\n```\n"
            ),
        );
        self.write_verdict(&parent, "gap-check", "gaps");
        self.step_ok(&parent, "gap-check -> cut-tickets routes on gaps");
        assert_eq!(self.state(&parent), "cut-tickets");
        self.write(".arca/ticket/t-100.md", &ticket_body(PLANNED_A));
        self.step_ok(&parent, "cut-tickets -> ticket-turns owns the gap");
        assert_eq!(self.state(&parent), "ticket-turns");
        parent
    }

    /// Spawn one turn of the ticket class; `workspace` binds the child to a
    /// directory of the fixture repository when given.
    fn spawn_child(&self, parent: &str, binds: &[&str], workspace: Option<&Path>) -> String {
        let mut args: Vec<String> = vec![
            "spawn".to_owned(),
            "turn".to_owned(),
            "--run".to_owned(),
            parent.to_owned(),
        ];
        for bind in binds {
            args.push("--bind".to_owned());
            args.push((*bind).to_owned());
        }
        if let Some(workspace) = workspace {
            args.push("--workspace".to_owned());
            args.push(workspace.to_string_lossy().into_owned());
        }
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let text = self.ok(&refs, "the coordinator spawns the turn");
        let child = text
            .split("spawned run ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .expect("spawn names the child run id")
            .to_owned();
        assert!(
            text.contains(&format!(
                "rtm: spawned run {child} (spawn turn) from run {parent}; the child is \
                 an ordinary Run at .ratmac/runs/{child}/"
            )),
            "stdout is today's line: {text}"
        );
        child
    }

    /// One sensitivity receipt proving a planned test fails before its code.
    fn write_sensitivity(&self, run: &str, planned: &str, test_name: &str) {
        let dir = self.root.join(format!(".ratmac/evidence/{run}"));
        fs::create_dir_all(&dir).expect("create evidence directory");
        let body = format!(
            "planned-test-id = \"{planned}\"\n\
             ticket-id = \"t-100\"\n\
             kind = \"baseline-failure\"\n\
             command = \"cargo test --test fixture_test\"\n\
             working-dir = \".\"\n\
             test-file = \"test/fixture_test.rs\"\n\
             test-name = \"{test_name}\"\n\
             exit-status = 101\n\
             output-sha256 = \"{}\"\n\
             output = \"\"\"\n{RED}\"\"\"\n",
            sha256_text(RED)
        );
        fs::write(dir.join(format!("{planned}.toml")), body).expect("write sensitivity receipt");
    }

    /// Every check the work item declares, recorded green and fresh against
    /// the tree at `workspace`.
    fn write_completion(&self, run: &str, workspace: &Path) {
        self.write_completion_for(
            run,
            workspace,
            &[
                (PLANNED_A, "focused", "cargo test --test fixture_test"),
                (HIDDEN_LANE, "hidden-lane", "cargo test --test fixture_test"),
                (QUALITY, "quality", "cargo --version"),
            ],
        );
    }

    /// Completion receipts for exactly the given checks against the tree at
    /// `workspace` - the seam the unrelated-receipt and stale-evidence twins
    /// and the check-declaration mutation drive.
    fn write_completion_for(&self, run: &str, workspace: &Path, checks: &[(&str, &str, &str)]) {
        let dir = self.root.join(format!(".ratmac/evidence/{run}/completion"));
        fs::create_dir_all(&dir).expect("create completion directory");
        let digest = ratmac::completion::tree_digest(workspace, &["src".to_owned()])
            .expect("source roots are readable");
        for (check, kind, command) in checks {
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
            .filter(|key| **key != own_log && after.get(*key) != before.get(*key))
            .collect();
        changed.extend(before.keys().filter(|key| !after.contains_key(*key)));
        assert!(
            changed.is_empty(),
            "{because}: the Engine root changed at {changed:?}"
        );
    }

    // --- governed plumbing (t-129's, reused unchanged) -----------------------

    fn keys(&self) -> &Keys {
        &self
            .governed
            .as_ref()
            .expect("the fixture declares a review policy")
            .keys
    }

    fn key_dir(&self) -> PathBuf {
        self.keys().dir.clone()
    }

    /// Render one canonical step intent to a fresh file and prove the pinned
    /// report line.
    fn intent(&self, run: &str, extra: &[String], tag: &str) -> PathBuf {
        let out = self.key_dir().join(format!("{tag}.bin"));
        let mut args = vec!["intent".to_owned(), "--run".to_owned(), run.to_owned()];
        args.extend(extra.iter().cloned());
        args.push("--out".to_owned());
        args.push(out.to_string_lossy().into_owned());
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let rendered = self.ran(&refs);
        assert_eq!(
            rendered.code, 0,
            "render the {tag} intent for {run}: {}{}",
            rendered.stdout, rendered.message
        );
        let bytes = fs::read(&out).unwrap_or_else(|error| panic!("read {tag} bytes: {error}"));
        assert_eq!(
            rendered.stdout,
            format!(
                "rtm: canonical step intent for run {run} written to {} (sha256 {})\n",
                shown(&out),
                sha256_hex(&bytes)
            ),
            "the {tag} intent report is the pinned one line"
        );
        out
    }

    /// One fully signed authorization bundle: the named reviewers' detached
    /// signatures over the review intent, and the Approver's signature over
    /// the approval intent computed over exactly those review files.
    fn authorize(&self, run: &str, reviewers: &[&str], tag: &str) -> Bundle {
        let review_intent = self.intent(run, &[], &format!("{tag}-review"));
        let mut reviews = Vec::new();
        for label in reviewers {
            let signature = self.keys().sign(label, REVIEW_NAMESPACE, &review_intent);
            reviews.push(((*label).to_owned(), signature));
        }
        self.bundle_over(run, &reviews, tag)
    }

    /// The approval bundle computed over an already-signed review set.
    fn bundle_over(&self, run: &str, reviews: &[(String, PathBuf)], tag: &str) -> Bundle {
        let mut extra = Vec::new();
        for (label, signature) in reviews {
            extra.push("--review".to_owned());
            extra.push(format!("{label}:{}", shown(signature)));
        }
        let approval_intent = self.intent(run, &extra, &format!("{tag}-approval"));
        let approve = self
            .keys()
            .sign("lead", APPROVE_NAMESPACE, &approval_intent);
        Bundle {
            reviews: reviews.to_vec(),
            approve,
        }
    }

    fn step_with(&self, run: &str, bundle: &Bundle) -> Ran {
        let mut args = vec!["step".to_owned(), "--run".to_owned(), run.to_owned()];
        for (label, signature) in &bundle.reviews {
            args.push("--review".to_owned());
            args.push(format!("{label}:{}", shown(signature)));
        }
        args.push("--approve".to_owned());
        args.push(shown(&bundle.approve));
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.ran(&refs)
    }

    fn step_plain(&self, run: &str) -> Ran {
        self.ran(&["step", "--run", run])
    }

    /// One field of one accepted authorization record.
    fn authorization_field(&self, run: &str, sequence: u32, field: &str) -> String {
        let path = self
            .run_dir(run)
            .join("authorizations")
            .join(format!("{:06}.toml", sequence));
        let text = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("read the authorization record {}: {error}", shown(&path))
        });
        let value: toml::Value = text
            .parse()
            .expect("the authorization record is valid TOML");
        value[field]
            .as_str()
            .unwrap_or_else(|| panic!("the record carries the string field {field}: {text}"))
            .to_owned()
    }
}

/// Run the workflow validator directly from `dir`, the way the guard does.
fn run_validator(dir: &Path) -> (i32, String, String) {
    let output = Command::new("python")
        .arg("tools/verify_issue_binding.py")
        .arg("check")
        .current_dir(dir)
        .output()
        .expect("invoke the workflow validator with real Python");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

// --- contract text builders (t-129's, reused byte for byte) --------------------

fn signatures_none(minimum: u32, run: &str) -> String {
    format!(
        "signatures: observed none; expected {minimum} distinct review signature(s) and one \
         approval signature over the canonical intent (rtm intent --run {run})"
    )
}

fn review_unverified(label: &str) -> String {
    format!(
        "review: observed a signature at label \"{label}\" that verifies under no enrolled \
         reviewer key; expected a ratmac-review-v1 signature by an enrolled reviewer"
    )
}

/// The exact two stdout lines of one ordinary governed step refusal.
fn step_refused(run: &str, detail: &str) -> String {
    format!("rtm: step refused; authorization: {detail}\nnext: rtm step --run {run}\n")
}

/// One governed step through a signed bundle, asserting it moves.
fn assert_stepped(fx: &Fixture, run: &str, bundle: &Bundle, why: &str) {
    let ran = fx.step_with(run, bundle);
    assert_eq!(ran.code, 0, "{why}: {}{}", ran.stdout, ran.message);
    assert!(
        !ran.stdout.contains("step refused"),
        "{why}: the step moves, not refuses: {}{}",
        ran.stdout,
        ran.message
    );
}

// --- the three checks -----------------------------------------------------------

/// WCPV-007: an isolated self-development run cannot pass using the historic writable-marker shape, unrelated issue receipts, a missing red check, or stale green evidence.
#[test]
fn wcpv_007() {
    let fx = Fixture::create("wcpv007", Inv::Process);
    // The shared construction sentence serves every row: the edited class's
    // declared shape, prompts, and doctor cleanliness are asserted here, on
    // the very bytes this fixture runs.
    assert_edited_class(&fx.runbook);
    let parent = fx.walk_to_ticket_turns();

    // The valid positive case first: one turn bound to its item and its
    // selected issue passes end to end under the edited class.
    let child = fx.spawn_child(&parent, &["item=t-100.md", "issue=i-100-demo"], None);
    assert_eq!(
        fx.state(&child),
        "tests",
        "a turn starts at the tests State"
    );

    // The validator's own success line, invoked from the child's workspace:
    // exit 0, one stdout line, stderr empty, nothing written anywhere.
    let status_before = fx.status_porcelain(&fx.root);
    let (code, out, err) = run_validator(&fx.root);
    assert_eq!(
        code, 0,
        "the validator exits 0 for the bound turn: {out}{err}"
    );
    assert_eq!(
        out,
        format!("issue binding verified: {ITEM} -> {ISSUE} (1 requirement, 2 checks)\n"),
        "on success stdout is exactly the contract's one line: 1 requirement (the \
         residual's cited requirement), 2 checks (the item's declared focused and \
         hidden identifiers, {PLANNED_A} and {HIDDEN_LANE}, the resolved count - \
         planned-test-refs are not resolved), each noun singular exactly when its \
         count is 1, the line ending in one LF"
    );
    assert_eq!(err, "", "on success stderr is empty: {err}");
    assert_eq!(
        fx.status_porcelain(&fx.root),
        status_before,
        "the validator performs no write anywhere"
    );

    // The pending guard row is listed with today's AOP-001 label.
    let status_text = combined(&fx.rtm(&["status", "--run", &child]));
    assert!(
        status_text.contains(
            "pending guard: command_exit program=\"python\" \
             args=[\"tools/verify_issue_binding.py\", \"check\"] expected=0"
        ),
        "rtm status lists the validator row among the State's pending guards: {status_text}"
    );

    // The ledger entry records both binding values.
    let ledger = fs::read_to_string(fx.root.join(format!(".ratmac/runs/{parent}/spawn-ledger")))
        .expect("read the parent's spawn ledger");
    let value: toml::Value = ledger.parse().expect("the spawn ledger is valid TOML");
    let entry = value["children"]
        .as_array()
        .expect("the ledger rows its children")
        .iter()
        .find(|entry| entry["id"].as_str() == Some(child.as_str()))
        .expect("the ledger records the spawned child");
    assert_eq!(
        entry["bind"]["item"].as_str(),
        Some("t-100.md"),
        "the ledger entry records the item binding value"
    );
    assert_eq!(
        entry["bind"]["issue"].as_str(),
        Some(ISSUE),
        "the ledger entry records the issue binding value"
    );
    // The roster row renders the pair as the existing joined clause.
    let roster = combined(&fx.rtm(&["status"]));
    assert!(
        roster.contains(&format!("bind issue=\"{ISSUE}\", item=\"{ITEM}\"")),
        "the roster row renders both bindings as the joined clause: {roster}"
    );

    fx.write_sensitivity(&child, PLANNED_A, "the_planned_test");
    fx.write_sensitivity(&child, PLANNED_B, "the_second_planned_test");
    fx.step_ok(
        &child,
        "the tests boundary passes: validator, then receipts",
    );
    assert_eq!(fx.state(&child), "implement");
    fx.write("src/lib.rs", "pub fn work() { return 7; }\n");
    fx.write_completion(&child, &fx.root);
    fx.commit_at(&fx.root, "the green landing");
    fx.step_ok(&child, "the implement boundary passes");
    assert_eq!(fx.state(&child), "damage");
    fx.step_ok(&child, "the damage boundary passes");
    assert_eq!(fx.state(&child), "done");
    let record = fx.record(&child);
    assert!(
        record.contains("state = \"done\"") && record.contains("status = \"passed\""),
        "the turn ends at done with the Engine-written passed fact: {record}"
    );

    // Twin: the historic writable-marker shape - a marker beside where the
    // receipts belong and a prose line, no receipt bytes.
    let ws = fx.add_worktree("wt-marker");
    let marker_child = fx.spawn_child(&parent, &["item=t-100.md", "issue=i-100-demo"], Some(&ws));
    let evidence = fx.root.join(format!(".ratmac/evidence/{marker_child}"));
    fs::create_dir_all(&evidence).expect("create the evidence directory");
    fs::write(
        evidence.join(format!("{PLANNED_A}.red-done")),
        "red proven for the planned test\n",
    )
    .expect("write the writable success marker");
    fx.ws_write(
        &ws,
        ".arca/ticket/red-notes.txt",
        "both planned tests are red\n",
    );
    let ws_before = fx.status_porcelain(&ws);
    let refused = fx.step_refused_text(&marker_child, "the writable-marker twin");
    assert!(
        refused.contains(&format!(
            "no sensitivity receipt in .ratmac/evidence/{marker_child}"
        )) && refused.contains("prose and file names are not evidence"),
        "the marker is refused by the landed receipt gate: {refused}"
    );
    assert!(
        !refused.contains("command_exit"),
        "the validator passed for the bound twin; the refusal is the receipt row: {refused}"
    );
    fx.assert_parked(&marker_child, "tests");
    assert_eq!(
        fx.status_porcelain(&ws),
        ws_before,
        "the refusal writes nothing"
    );

    // Twin: a missing red check - one declared planned test has no receipt.
    let ws = fx.add_worktree("wt-missing-red");
    let missing_child = fx.spawn_child(&parent, &["item=t-100.md", "issue=i-100-demo"], Some(&ws));
    fx.write_sensitivity(&missing_child, PLANNED_A, "the_planned_test");
    let refused = fx.step_refused_text(&missing_child, "the missing-red twin");
    assert!(
        refused.contains(&format!(
            "{PLANNED_B}: no sensitivity receipt in .ratmac/evidence/{missing_child}"
        )),
        "the refusal names the planned test that lacks its red receipt: {refused}"
    );
    assert!(
        !refused.contains(&format!(
            "{PLANNED_A}: no sensitivity receipt in .ratmac/evidence/{missing_child}"
        )),
        "the receipt that exists is not named: {refused}"
    );
    fx.assert_parked(&missing_child, "tests");

    // Twin: an unrelated issue's tests - the turn bound to another issue
    // whose identifiers resolve in no other bundle.
    let ws = fx.add_worktree("wt-other-issue");
    let other_child = fx.spawn_child(&parent, &["item=t-100.md", "issue=i-200-other"], Some(&ws));
    fx.write_sensitivity(&other_child, PLANNED_A, "the_planned_test");
    fx.write_sensitivity(&other_child, PLANNED_B, "the_second_planned_test");
    let ws_status = fx.status_porcelain(&ws);
    let before = fx.snapshot_engine();
    let refused = fx.step_refused_text(&other_child, "the unrelated-issue twin");
    assert!(
        refused.contains("command_exit: python")
            && refused.contains("observed exit 1")
            && refused.contains("expected exit 0"),
        "the refusal rides today's command-guard envelope: {refused}"
    );
    assert!(
        refused.contains(&format!(
            "verify_issue_binding: requirement DEMO-001 (cited by residual res-100) \
             does not occur in the selected issue {OTHER_ISSUE}'s spec.md"
        )),
        "the validator's requirement row refuses the unrelated issue: {refused}"
    );
    fx.assert_parked(&other_child, "tests");
    fx.assert_engine_untouched(
        &before,
        &other_child,
        "the refused step and the read-only validator write nothing",
    );
    assert_eq!(
        fx.status_porcelain(&ws),
        ws_status,
        "the refusal leaves the tree byte-identical"
    );

    // Twin: unrelated issue receipts at the receipt gate - receipts naming
    // another issue's checks cannot satisfy the addressed declaration.
    let ws = fx.add_worktree("wt-unrelated-receipts");
    let receipts_child = fx.spawn_child(&parent, &["item=t-100.md", "issue=i-100-demo"], Some(&ws));
    fx.write_sensitivity(&receipts_child, PLANNED_A, "the_planned_test");
    fx.write_sensitivity(&receipts_child, PLANNED_B, "the_second_planned_test");
    fx.step_ok(&receipts_child, "the twin reaches the implement boundary");
    assert_eq!(fx.state(&receipts_child), "implement");
    fx.ws_write(&ws, "src/lib.rs", "pub fn work() { return 9; }\n");
    fx.write_completion_for(
        &receipts_child,
        &ws,
        &[
            ("PT-200-01", "focused", "cargo test --test fixture_test"),
            ("HT-200-01", "hidden-lane", "cargo test --test fixture_test"),
            (QUALITY, "quality", "cargo --version"),
        ],
    );
    fx.commit_at(&ws, "the unrelated green landing");
    let refused = fx.step_refused_text(&receipts_child, "the unrelated-receipts twin");
    assert!(
        refused.contains(&format!(
            "{PLANNED_A}: no completion receipt for this focused check; expected {}",
            shown(&fx.root.join(format!(
                ".ratmac/evidence/{receipts_child}/completion/{}.toml",
                ratmac::completion::check_slug(PLANNED_A)
            )))
        )),
        "the receipt gate's per-check row refuses another issue's receipts, naming the \
         expected receipt's own path: {refused}"
    );
    fx.assert_parked(&receipts_child, "implement");

    // Twin: stale green evidence - receipts bound to a tree the work no
    // longer hashes to.
    let ws = fx.add_worktree("wt-stale");
    let stale_child = fx.spawn_child(&parent, &["item=t-100.md", "issue=i-100-demo"], Some(&ws));
    fx.write_sensitivity(&stale_child, PLANNED_A, "the_planned_test");
    fx.write_sensitivity(&stale_child, PLANNED_B, "the_second_planned_test");
    fx.step_ok(&stale_child, "the twin reaches the implement boundary");
    fx.ws_write(&ws, "src/lib.rs", "pub fn work() { return 9; }\n");
    fx.write_completion(&stale_child, &ws);
    fx.commit_at(&ws, "the green landing");
    let reviewed_digest = ratmac::completion::tree_digest(&ws, &["src".to_owned()])
        .expect("the reviewed tree is readable");
    fx.ws_write(&ws, "src/lib.rs", "pub fn work() { return 8; }\n");
    fx.commit_at(&ws, "silent source drift");
    let drifted_digest = ratmac::completion::tree_digest(&ws, &["src".to_owned()])
        .expect("the drifted tree is readable");
    let refused = fx.step_refused_text(&stale_child, "the stale-green twin");
    assert!(
        refused.contains(&format!(
            "receipt is stale: it claims tree {reviewed_digest}, but the work now hashes to {drifted_digest}"
        )),
        "the completion gate refuses stale green evidence by both digests: {refused}"
    );
    fx.assert_parked(&stale_child, "implement");

    // Twin: the missing binding - a spawn without `--bind issue=` admits, and
    // the validator refuses before any receipt work.
    let ws = fx.add_worktree("wt-nobind");
    let unbound_child = fx.spawn_child(&parent, &["item=t-100.md"], Some(&ws));
    let refused = fx.step_refused_text(&unbound_child, "the missing-binding twin");
    assert!(
        refused.contains(&format!(
            "verify_issue_binding: the spawn ledger entry for run {unbound_child} \
             supplies no binding \"issue\"; it supplies \"item\""
        )),
        "the full absent-binding row, sorted supplied names included, refuses the \
         unbound turn: {refused}"
    );
    // Every guard evaluates into one composed refusal; precedence is the
    // defect order - the binding row's defect precedes the receipt guard's.
    let binding_at = refused
        .find("verify_issue_binding: the spawn ledger entry")
        .expect("the binding row's defect is listed");
    let receipts_at = refused
        .find("sensitivity_receipts")
        .expect("the receipt guard's defect is listed");
    assert!(
        binding_at < receipts_at,
        "the binding row refuses before any receipt work, receipt or no receipt: {refused}"
    );
    fx.assert_parked(&unbound_child, "tests");

    // Twin: an issue id that resolves at no location of the namespace.
    let ws = fx.add_worktree("wt-nowhere");
    let nowhere_child = fx.spawn_child(&parent, &["item=t-100.md", "issue=i-404-none"], Some(&ws));
    let refused = fx.step_refused_text(&nowhere_child, "the resolves-nowhere twin");
    assert!(
        refused.contains(&format!(
            "verify_issue_binding: the selected issue {MISSING_ISSUE} resolves at no \
             location of the issue namespace"
        )),
        "the validator's unresolved-issue row refuses the turn: {refused}"
    );
    fx.assert_parked(&nowhere_child, "tests");

    // The join: none of the refused twins' markers, receipts, or bindings
    // satisfy the parent - only the Engine-written passed fact counts, and
    // every unfinished live child is named.
    let refused = fx.step_refused_text(&parent, "the join reads only passed facts");
    assert!(
        refused.contains(
            "join: spawn ledger: observed 1 of 1 required children are passed; unfinished: "
        ) && refused.contains("expected every ledger-recorded live child passed"),
        "the parent's join refuses while the twins are unfinished - the declared min is \\
         met, and the live unfinished children still refuse it: {refused}"
    );
    assert!(
        refused.contains(&format!("{marker_child} is planned at tests"))
            && refused.contains(&format!("{receipts_child} is planned at implement")),
        "the join names each unfinished child with its status and state: {refused}"
    );
    assert_eq!(fx.state(&parent), "ticket-turns", "the parent stays parked");
}

/// WCPV-008: a working agent's forged review record or wrong reviewer is refused; an authorized independent review bound to the tested snapshot can pass.
#[test]
fn wcpv_008() {
    let _lane = support::hook_lane();
    let fx = Fixture::create("wcpv008", Inv::Library);
    let parent = fx.walk_to_ticket_turns();

    // The valid positive case first: an authorized independent review moves
    // the bound turn through its first governed transition.
    let child = fx.spawn_child(&parent, &["item=t-100.md", "issue=i-100-demo"], None);
    fx.write_sensitivity(&child, PLANNED_A, "the_planned_test");
    fx.write_sensitivity(&child, PLANNED_B, "the_second_planned_test");

    // The canonical intent binds the recorded bindings and the tested bytes:
    // its binding digest covers the recorded item and issue values, and the
    // current work digest covers the declared work roots.
    let intent = fx.intent(&child, &[], "w8-binding");
    let bytes = fs::read(&intent).expect("read the canonical intent bytes");
    let digest = binding_digest_hex(&[("item", ITEM), ("issue", ISSUE)]);
    assert!(
        bytes
            .windows(digest.len())
            .any(|window| window == digest.as_bytes()),
        "the canonical intent carries the binding digest of the recorded item and issue values"
    );
    let reviewed_sha = sha256_hex(&bytes);
    fx.write("src/lib.rs", "pub fn work() { return 7; }\n");
    let drifted = fx.intent(&child, &[], "w8-drifted");
    let drifted_bytes = fs::read(&drifted).expect("read the drifted intent bytes");
    assert_ne!(
        sha256_hex(&drifted_bytes),
        reviewed_sha,
        "the intent's current work digest covers the tested roots, so changed bytes change it"
    );
    fx.write("src/lib.rs", "pub fn work() {}\n");

    let bundle = fx.authorize(&child, &["a", "b"], "w8-positive");
    let stepped = fx.step_with(&child, &bundle);
    assert_eq!(
        stepped.code, 0,
        "the authorized step succeeds: {}{}",
        stepped.stdout, stepped.message
    );
    assert!(
        stepped.stdout.contains(
            "Make the planned tests pass - the focused public checks green - then obtain"
        ) && stepped
            .stdout
            .contains(&format!("next: rtm step --run {child}")),
        "a successful governed step prints today's success: {}{}",
        stepped.stdout,
        stepped.message
    );
    assert_eq!(fx.state(&child), "implement");
    assert_eq!(
        fx.authorization_field(&child, 1, "origin"),
        "test-injected",
        "the accepted authorization records the trust seam's origin"
    );
    assert_eq!(fx.authorization_field(&child, 1, "from_state"), "tests");
    assert_eq!(fx.authorization_field(&child, 1, "to_state"), "implement");

    // The twins, on a fresh governed turn with its receipts green. The
    // parent's cut records are committed first: a linked worktree carries
    // only tracked files, and spawn validates the declared root roles in
    // the workspace it is given.
    fx.commit_at(&fx.root, "the cycle's working records");
    let ws = fx.add_worktree("wt-w8");
    let twin = fx.spawn_child(&parent, &["item=t-100.md", "issue=i-100-demo"], Some(&ws));
    fx.write_sensitivity(&twin, PLANNED_A, "the_planned_test");
    fx.write_sensitivity(&twin, PLANNED_B, "the_second_planned_test");

    // Twin: a forged review record - the working agent writes the record
    // itself. The record is not an Engine input: the governed step demands
    // the enrolled signatures regardless of what the record file says.
    fx.ws_write(
        &ws,
        ".arca/ticket/t-100-review-record.txt",
        &format!(
            "review record\n  item:      t-100\n  verdict:   accepted\n  snapshot:  {}\n  reviewed:  src test\n",
            reviewed_sha
        ),
    );
    let before = fx.snapshot_engine();
    let none = fx.step_plain(&twin);
    assert_eq!(none.code, 0, "a refusal is a StepOutcome::Refused");
    assert_eq!(
        none.stdout,
        step_refused(&twin, &signatures_none(2, &twin)),
        "the forged record passes no Engine boundary: the missing-signature refusal is exact"
    );
    fx.assert_parked(&twin, "tests");
    fx.assert_engine_untouched(
        &before,
        &twin,
        "the forged-record refusal writes nothing under the Engine root",
    );

    // Twin: a wrong reviewer - an enrolled label over an outsider's
    // signature, beside one honest review, with an internally consistent
    // approval.
    let review_intent = fx.intent(&twin, &[], "w8-wrong");
    let forged = fx.keys().sign("outsider", REVIEW_NAMESPACE, &review_intent);
    let honest = fx.keys().sign("b", REVIEW_NAMESPACE, &review_intent);
    let bundle = fx.bundle_over(
        &twin,
        &[("a".to_owned(), forged), ("b".to_owned(), honest)],
        "w8-wrong",
    );
    let before = fx.snapshot_engine();
    let wrong = fx.step_with(&twin, &bundle);
    assert_eq!(wrong.code, 0);
    assert_eq!(
        wrong.stdout,
        step_refused(&twin, &review_unverified("a")),
        "the wrong-reviewer refusal is exact"
    );
    fx.assert_parked(&twin, "tests");
    fx.assert_engine_untouched(&before, &twin, "the wrong-reviewer refusal writes nothing");

    // The authorized independent review passes after both refusals.
    let renewed = fx.authorize(&twin, &["a", "b"], "w8-renewed");
    let stepped = fx.step_with(&twin, &renewed);
    assert_eq!(
        stepped.code, 0,
        "the authorized review bound to the tested snapshot can pass: {}{}",
        stepped.stdout, stepped.message
    );
    assert_eq!(fx.state(&twin), "implement");
}

/// WCPV-009: changing source, the issue selection, the requirement records, or the check declarations after the review invalidates completion; an authorized reviewed red/green sequence ends in an Engine-written passed fact.
#[test]
fn wcpv_009() {
    let _lane = support::hook_lane();
    let fx = Fixture::create("wcpv009", Inv::Library);
    let parent = fx.walk_to_ticket_turns();

    // The valid positive case first: the authorized reviewed red/green
    // sequence, end to end, ending in the Engine-written passed fact that
    // the parent's join consumes.
    let child = fx.spawn_child(&parent, &["item=t-100.md", "issue=i-100-demo"], None);
    fx.write_sensitivity(&child, PLANNED_A, "the_planned_test");
    fx.write_sensitivity(&child, PLANNED_B, "the_second_planned_test");
    let tests_bundle = fx.authorize(&child, &["a", "b"], "w9-tests");
    let stepped = fx.step_with(&child, &tests_bundle);
    assert_eq!(
        stepped.code, 0,
        "the reviewed red evidence authorizes the tests boundary: {}{}",
        stepped.stdout, stepped.message
    );
    assert_eq!(fx.state(&child), "implement");
    fx.write("src/lib.rs", "pub fn work() { return 7; }\n");
    fx.write("src/lib.rs", "pub fn work() { return 7; }\n");
    fx.write_completion(&child, &fx.root);
    fx.commit_at(&fx.root, "the green landing");
    let implement_bundle = fx.authorize(&child, &["a", "b"], "w9-implement");
    assert_stepped(
        &fx,
        &child,
        &implement_bundle,
        "the reviewed green evidence authorizes the implement boundary",
    );
    assert_eq!(fx.state(&child), "damage");
    let damage_bundle = fx.authorize(&child, &["a", "b"], "w9-damage");
    assert_stepped(
        &fx,
        &child,
        &damage_bundle,
        "the restored-green rerun authorizes the damage boundary",
    );
    assert_eq!(fx.state(&child), "done");
    let record = fx.record(&child);
    assert!(
        record.contains("state = \"done\"") && record.contains("status = \"passed\""),
        "the turn ends at done with the Engine-written passed fact: {record}"
    );

    // The four post-review mutations, on a fresh governed turn whose review
    // was earned over the exact bytes at the implement boundary.
    let ws = fx.add_worktree("wt-w9");
    let twin = fx.spawn_child(&parent, &["item=t-100.md", "issue=i-100-demo"], Some(&ws));
    fx.write_sensitivity(&twin, PLANNED_A, "the_planned_test");
    fx.write_sensitivity(&twin, PLANNED_B, "the_second_planned_test");
    let red_bundle = fx.authorize(&twin, &["a", "b"], "w9-red");
    assert_stepped(
        &fx,
        &twin,
        &red_bundle,
        "the twin's reviewed red evidence authorizes its tests boundary",
    );
    assert_eq!(fx.state(&twin), "implement");
    fx.ws_write(&ws, "src/lib.rs", "pub fn work() { return 9; }\n");
    fx.write_completion(&twin, &ws);
    fx.commit_at(&ws, "the twin's green landing");
    // The review over the exact bytes: earned, not yet spent.
    let reviewed = fx.authorize(&twin, &["a", "b"], "w9-reviewed");
    let reviewed_digest = ratmac::completion::tree_digest(&ws, &["src".to_owned()])
        .expect("the reviewed tree is readable");

    // Mutation: source changed after the review, receipts untouched - the
    // receipt gate re-derives against the changed bytes.
    fx.ws_write(&ws, "src/lib.rs", "pub fn work() { return 8; }\n");
    fx.commit_at(&ws, "silent source drift");
    let drifted_digest = ratmac::completion::tree_digest(&ws, &["src".to_owned()])
        .expect("the drifted tree is readable");
    let refused = fx.step_with(&twin, &reviewed);
    assert_eq!(refused.code, 0, "a refusal is a StepOutcome::Refused");
    assert!(
        refused.stdout.contains(&format!(
            "receipt is stale: it claims tree {reviewed_digest}, but the work now hashes to {drifted_digest}"
        )),
        "the stale receipts invalidate completion against the changed source: {}{}",
        refused.stdout,
        refused.message
    );
    fx.assert_parked(&twin, "implement");
    fx.ws_write(&ws, "src/lib.rs", "pub fn work() { return 9; }\n");
    fx.commit_at(&ws, "restore the reviewed source");

    // Mutation: source changed after the review and the green evidence
    // re-earned over the changed bytes - the recomputed canonical intent no
    // longer verifies against the approval signature, even with fresh
    // reviews over the changed bytes.
    fx.ws_write(&ws, "src/lib.rs", "pub fn work() { return 8; }\n");
    fx.write_completion(&twin, &ws);
    fx.commit_at(&ws, "regreened after the review");
    let fresh_intent = fx.intent(&twin, &[], "w9-fresh");
    let mut fresh_reviews = Vec::new();
    for label in ["a", "b"] {
        let signature = fx.keys().sign(label, REVIEW_NAMESPACE, &fresh_intent);
        fresh_reviews.push((label.to_owned(), signature));
    }
    let stale_approval = Bundle {
        reviews: fresh_reviews,
        approve: reviewed.approve.clone(),
    };
    let before = fx.snapshot_engine();
    let refused = fx.step_with(&twin, &stale_approval);
    assert_eq!(refused.code, 0);
    assert!(
        refused.stdout.contains(
            "authorization: approval: observed a signature that does not verify against \
             the recomputed canonical intent sha256 "
        ) && refused.stdout.contains(&format!(
            "expected a fresh approval of that exact intent (rtm intent --run {twin})"
        )),
        "the stale approval signature cannot authorize the changed bytes: {}{}",
        refused.stdout,
        refused.message
    );
    fx.assert_parked(&twin, "implement");
    assert_eq!(
        fx.snapshot_engine(),
        before,
        "the authorization refusal writes nothing"
    );
    fx.ws_write(&ws, "src/lib.rs", "pub fn work() { return 9; }\n");
    fx.write_completion(&twin, &ws);
    fx.commit_at(&ws, "restore the reviewed source again");

    // Mutation: the issue selection changed after the review - the selected
    // issue's requirement row is gone, and the validator re-derives.
    let issue_spec = fs::read_to_string(ws.join(".arca/issue/i-100-demo/spec.md"))
        .expect("read the selected issue's spec");
    fx.ws_write(
        &ws,
        ".arca/issue/i-100-demo/spec.md",
        "# Requirement records\n\n| Req ID | Requirement | Status |\n|---|---|---|\n",
    );
    fx.commit_at(&ws, "edit the selected issue");
    let refused = fx.step_with(&twin, &reviewed);
    assert_eq!(refused.code, 0);
    assert!(
        refused.stdout.contains(&format!(
            "verify_issue_binding: requirement DEMO-001 (cited by residual res-100) \
             does not occur in the selected issue {ISSUE}'s spec.md"
        )),
        "the changed issue selection invalidates completion through the validator: {}{}",
        refused.stdout,
        refused.message
    );
    fx.assert_parked(&twin, "implement");
    fx.ws_write(&ws, ".arca/issue/i-100-demo/spec.md", &issue_spec);
    fx.commit_at(&ws, "restore the selected issue");

    // Mutation: the requirement records changed after the review.
    let residual =
        fs::read_to_string(ws.join(".arca/residual/res-100.md")).expect("read the residual record");
    fx.ws_write(
        &ws,
        ".arca/residual/res-100.md",
        &residual.replace("DEMO-001", "DEMO-999"),
    );
    fx.commit_at(&ws, "edit the requirement record");
    let refused = fx.step_with(&twin, &reviewed);
    assert_eq!(refused.code, 0);
    assert!(
        refused.stdout.contains(
            "verify_issue_binding: requirement DEMO-999 (cited by residual res-100) \
             does not occur in the selected issue i-100-demo's spec.md"
        ),
        "the changed requirement record invalidates completion through the validator: {}{}",
        refused.stdout,
        refused.message
    );
    fx.assert_parked(&twin, "implement");
    fx.ws_write(&ws, ".arca/residual/res-100.md", &residual);
    fx.commit_at(&ws, "restore the requirement record");

    // Mutation: the check declarations changed after the review - a declared
    // check the issue's test plan does not carry, with a receipt fabricated
    // for it.
    fx.ws_write(&ws, ".arca/ticket/t-100.md", &ticket_body("PT-999-01"));
    fx.write_completion_for(
        &twin,
        &ws,
        &[
            ("PT-999-01", "focused", "cargo test --test fixture_test"),
            (HIDDEN_LANE, "hidden-lane", "cargo test --test fixture_test"),
            (QUALITY, "quality", "cargo --version"),
        ],
    );
    fx.commit_at(&ws, "edit the check declarations");
    let refused = fx.step_with(&twin, &reviewed);
    assert_eq!(refused.code, 0);
    assert!(
        refused.stdout.contains(
            "verify_issue_binding: declared check PT-999-01 does not occur in the \
             selected issue i-100-demo's test-plan.md"
        ),
        "the changed check declarations invalidate completion through the validator: {}{}",
        refused.stdout,
        refused.message
    );
    fx.assert_parked(&twin, "implement");
    fs::remove_file(
        fx.root
            .join(format!(".ratmac/evidence/{twin}/completion"))
            .join(format!(
                "{}.toml",
                ratmac::completion::check_slug("PT-999-01")
            )),
    )
    .expect("remove the fabricated receipt");
    fx.ws_write(&ws, ".arca/ticket/t-100.md", &ticket_body(PLANNED_A));
    fx.write_completion(&twin, &ws);
    fx.commit_at(&ws, "restore the check declarations");

    // The renewed review earns the remaining transitions after every
    // mutation was refused.
    let renewed = fx.authorize(&twin, &["a", "b"], "w9-renewed");
    assert_stepped(
        &fx,
        &twin,
        &renewed,
        "the renewed review earns the implement boundary",
    );
    assert_eq!(fx.state(&twin), "damage");
    let finish = fx.authorize(&twin, &["a", "b"], "w9-finish");
    assert_stepped(
        &fx,
        &twin,
        &finish,
        "the renewed review earns the damage boundary",
    );
    assert_eq!(fx.state(&twin), "done");
    let record = fx.record(&twin);
    assert!(
        record.contains("state = \"done\"") && record.contains("status = \"passed\""),
        "the renewed review ends the turn in the Engine-written passed fact: {record}"
    );

    // The parent consumes both terminal review results through the join.
    fx.step_ok(&parent, "the join consumes the passed turns");
    assert_eq!(fx.state(&parent), "close", "the parent leaves ticket-turns");
}
