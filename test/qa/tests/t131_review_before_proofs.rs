//! t-131 / WCP-004: review before proofs - the turn tool's `review` and
//! `proofs` verbs put the independent review of the exact snapshot ahead of
//! the expensive proof set, one earned proof set per accepted review, while
//! the mandatory restored-green reruns and the main verification from main
//! stay ungated and once-only earning never suppresses them.
//!
//! Every fixture is a throwaway repository under the temp directory built by
//! the turn-lifecycle family (`ratmac_qa::turn`) - the WCPV-010 isolation
//! device this contract names - carrying the real `tools/turn.ps1` and its
//! declared data. The fixture's declared `lanes-rerun` command is a trace
//! command that appends its working directory to a file outside every
//! repository, so the proof-earning event and the main-verification event
//! are separately observable behavior, not wording. Git in a fixture - the
//! tool's own children included - reads a fixture-local empty global
//! configuration file through `GIT_CONFIG_GLOBAL` with
//! `GIT_CONFIG_NOSYSTEM=1`. Each check opens with today's `status` verb as
//! its positive control.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use ratmac::machine::{MachineClass, StateDefinition};
use ratmac_qa::snapshot::sha256_file;
use ratmac_qa::support::{command, TempTree};
use ratmac_qa::turn::{Turn, ITEM_RECORDS, LANDING_LOG, LANES, SKIP, STAMP_FIELD, TRUNK};
use sha2::{Digest, Sha256};

/// The item string every fixture addresses.
const ITEM: &str = "item-1";

/// The fixed safety subject the checkpoint verb records (t-125's row).
const SAFETY_SUBJECT: &str = "item-1: checkpoint - not a landing";

/// The sha256 of the recorded `.ratmac/ratmac.toml` bytes of this ticket's
/// merge base: the recorded base the two tracked boundary diffs apply over.
const BASE_RUNBOOK_SHA256: &str =
    "012eaa36c1a895f50d8822d516699029cb461a2d489b8ee2b882ce37a8b440c6";

/// The recorded `.ratmac/ratmac.toml` bytes of the merge base of this
/// ticket: the recorded base the two tracked boundary diffs apply over -
/// a byte-identical copy of the embedded base of t-130 (its sha256
/// asserted below), embedded so the tracked runbook of the repository is
/// never read: the cycle boundary wires both diffs into it, and this
/// suite must stay green through that wiring.
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

/// The `implement` State's final prompt body - the contract's final string,
/// which the tracked review-proofs diff wires beside t-130's eight routes.
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

/// The `damage` State's final prompt body - the contract's final string.
const DAMAGE_PROMPT: &str = r#"Break the green work on purpose, one mutation at a time, and check that a
test catches each one. Freeze the safety checkpoint through the turn tool
first: pwsh -File tools/turn.ps1 checkpoint -Item <item> -Target <path>...;
then mutate only the declared targets and restore after every mutation:
pwsh -File tools/turn.ps1 restore -Item <item>. Re-run the focused checks
green after every restore; the restored-green rerun is mandatory, not
re-earned proof. A mutation that survives is a hole in the tests, not a
win.
"#;

// --- isolated fixture plumbing ------------------------------------------------

/// An owned tree holding nothing but the empty global Git configuration file
/// every fixture-local child reads instead of the host's, plus this suite's
/// trace files - always outside the fixture repositories.
fn isolation(label: &str) -> (TempTree, PathBuf) {
    let tree = TempTree::new(&format!("t131-{label}")).expect("create the isolation tree");
    let global = tree.join("git-global.cfg");
    fs::write(&global, b"").expect("write the empty global config");
    (tree, global)
}

/// The tool under test, run in `directory` with Git configuration isolated
/// to the fixture-local empty global file.
fn turn_isolated(global: &Path, directory: &Path, args: &[&str]) -> Output {
    let mut all = vec!["-NoProfile", "-File", "tools/turn.ps1"];
    all.extend_from_slice(args);
    let mut invocation = command("pwsh", directory);
    invocation
        .args(&all)
        .env("GIT_CONFIG_GLOBAL", global)
        .env("GIT_CONFIG_NOSYSTEM", "1");
    invocation.output().expect("invoke pwsh")
}

/// Fixture Git with the same isolation.
fn git_isolated(global: &Path, directory: &Path, args: &[&str]) -> Output {
    let mut invocation = command("git", directory);
    invocation
        .args(args)
        .env("GIT_CONFIG_GLOBAL", global)
        .env("GIT_CONFIG_NOSYSTEM", "1");
    invocation.output().expect("invoke git")
}

/// Successful isolated Git, its stdout trimmed.
fn git_ok(global: &Path, directory: &Path, args: &[&str]) -> String {
    let output = git_isolated(global, directory, args);
    assert!(
        output.status.success(),
        "t131: git {args:?} fails: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

// --- exact-output helpers -----------------------------------------------------

/// Whole lines of one captured stream, each stripped of the console newline's
/// carriage return, so every assertion is exact per line.
fn lines(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(|line| line.to_owned())
        .collect()
}

/// A stream must be exactly these lines, byte for byte per line.
fn assert_lines(bytes: &[u8], expected: &[&str], what: &str) {
    let actual = lines(bytes);
    assert_eq!(actual, expected, "t131: {what} must be exactly these lines");
}

/// A refusal: exit non-zero, stdout empty, stderr exactly these lines.
fn assert_refusal(output: &Output, stderr: &[String], what: &str) {
    assert!(
        !output.status.success(),
        "t131: {what} refuses: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_lines(&output.stdout, &[], "the refusal prints no success line");
    let expected: Vec<&str> = stderr.iter().map(String::as_str).collect();
    assert_lines(&output.stderr, &expected, what);
}

/// Today's `status` verb stays green - the positive control that precedes
/// every new-verb assertion.
fn status_control(global: &Path, turn: &Turn) {
    let output = turn_isolated(global, &turn.root, &["status", "-Item", ITEM]);
    assert!(
        output.status.success(),
        "t131: today's status verb stays green: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        lines(&output.stdout).first().map(String::as_str),
        Some("turn status (read-only; nothing below has been applied)"),
        "t131: the status report keeps its first line"
    );
}

/// The addressed checkout's displayed spelling: git's own root answer with
/// forward slashes - the same fact the tool's one spelling resolves.
fn checkout_text(global: &Path, directory: &Path) -> String {
    git_ok(global, directory, &["rev-parse", "--show-toplevel"]).replace('\\', "/")
}

// --- the fixture's declared trace command --------------------------------------

/// The trace file's lines, one per invocation of the declared command, each
/// carrying the working directory the command ran from.
fn trace_lines(trace: &Path) -> Vec<String> {
    if !trace.exists() {
        return Vec::new();
    }
    fs::read_to_string(trace)
        .expect("read the declared command's trace")
        .lines()
        .map(|line| line.replace('\\', "/"))
        .collect()
}

/// The declared `lanes-rerun` trace command: one appended line carrying the
/// working directory per invocation, written outside every repository, so
/// earning the proof set changes no reviewed row.
fn trace_command(trace: &Path) -> String {
    let path = trace.to_string_lossy().replace('\\', "/");
    format!("Add-Content -Path '{path}' -Value $PWD.Path")
}

/// Rewrite the fixture's declaration so its `lanes-rerun` is the trace
/// command (plus an optional rerun scope), committed before any turn opens.
fn declare_trace_command(global: &Path, turn: &Turn, command: &str, scope: Option<&str>) {
    let mut declaration = String::new();
    declaration.push_str("# The turn lifecycle's declared data (THK-001..THK-004).\n[roots]\n");
    declaration.push_str(&format!(
        "trunk = \"{TRUNK}\"\nlanes = \"{LANES}\"\nitem-records = \"{ITEM_RECORDS}\"\n"
    ));
    declaration.push_str(&format!(
        "landing-log = \"{LANDING_LOG}\"\nstamp-field = \"{STAMP_FIELD}\"\nskip = [\"{SKIP}\"]\n"
    ));
    declaration.push_str(&format!("lanes-rerun = \"{command}\"\n"));
    if let Some(value) = scope {
        declaration.push_str(&format!("lanes-rerun-scope = \"{value}\"\n"));
    }
    fs::write(turn.root.join(".ratmac/turn.toml"), declaration)
        .expect("rewrite the fixture declaration");
    git_ok(global, &turn.root, &["add", ".ratmac/turn.toml"]);
    git_ok(
        global,
        &turn.root,
        &["commit", "-m", "fixture: declare the trace command"],
    );
}

// --- the snapshot digest oracle ------------------------------------------------

/// The item's own evidence under the declared item-records root: never a row
/// of the reviewed inputs, whatever its tracked state.
fn is_excluded_evidence(relative: &str) -> bool {
    [
        format!("{ITEM}-review-record.txt"),
        format!("{ITEM}-proof-receipt.txt"),
        format!("{ITEM}-checkpoint-manifest.txt"),
    ]
    .iter()
    .any(|name| {
        relative == format!("{ITEM_RECORDS}/{name}")
            || relative == format!("{ITEM_RECORDS}/{name}.tmp")
    })
}

/// The lanes-root-relative lane rows, skipping the declared skip names at
/// any depth - the rows the tool's existing tree walker emits.
fn collect_lane_rows(lanes_root: &Path, prefix: &str, rows: &mut Vec<String>) {
    let directory = if prefix.is_empty() {
        lanes_root.to_path_buf()
    } else {
        lanes_root.join(prefix)
    };
    let mut entries: Vec<PathBuf> = fs::read_dir(&directory)
        .expect("read the lanes root")
        .map(|entry| entry.expect("read entry").path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path
            .file_name()
            .expect("a lanes entry carries a name")
            .to_string_lossy()
            .into_owned();
        let child = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        if path.is_dir() {
            if name == SKIP {
                continue;
            }
            collect_lane_rows(lanes_root, &child, rows);
        } else {
            rows.push(format!("lane {child}\t{}", sha256_file(&path)));
        }
    }
}

/// The reviewed inputs of one addressed checkout: every work row plus every
/// lane row, rendered `work <path>\t<sha256>` / `lane <path>\t<sha256>`, all
/// rows together sorted bytewise ascending over the whole rendered line -
/// this test's own oracle, independent of the tool under test.
fn reviewed_rows(global: &Path, directory: &Path) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    for arguments in [
        vec!["ls-files", "--cached"],
        vec!["ls-files", "--others", "--exclude-standard"],
    ] {
        for line in lines(&git_isolated(global, directory, &arguments).stdout) {
            if !line.is_empty() && !paths.contains(&line) {
                paths.push(line);
            }
        }
    }
    paths.sort();
    let mut rows: Vec<String> = Vec::new();
    for path in &paths {
        if is_excluded_evidence(path) {
            continue;
        }
        rows.push(format!(
            "work {path}\t{}",
            sha256_file(&directory.join(path))
        ));
    }
    collect_lane_rows(&directory.join(LANES), "", &mut rows);
    rows.sort();
    rows
}

/// The snapshot digest: the SHA-256 of `ratmac-review-v1`, one LF, then
/// every rendered row, each line LF-terminated.
fn snapshot_digest_of(rows: &[String]) -> String {
    let mut bytes: Vec<u8> = b"ratmac-review-v1\n".to_vec();
    for row in rows {
        bytes.extend_from_slice(row.as_bytes());
        bytes.push(b'\n');
    }
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

/// The `inputs:` row's phrase, with its own plural per count.
fn inputs_phrase(rows: &[String]) -> String {
    let work = rows.iter().filter(|row| row.starts_with("work ")).count();
    let lane = rows.iter().filter(|row| row.starts_with("lane ")).count();
    format!(
        "{} work file{}, {} lane file{}",
        work,
        if work == 1 { "" } else { "s" },
        lane,
        if lane == 1 { "" } else { "s" }
    )
}

/// The review record's path inside one addressed checkout.
fn record_path(directory: &Path) -> PathBuf {
    directory.join(format!("{ITEM_RECORDS}/{ITEM}-review-record.txt"))
}

/// The proof receipt's path inside one addressed checkout.
fn receipt_path(directory: &Path) -> PathBuf {
    directory.join(format!("{ITEM_RECORDS}/{ITEM}-proof-receipt.txt"))
}

/// The review record's exact bytes: three header lines, then every row in
/// one bytewise sort over the whole rendered line, LF-terminated.
fn expected_record(verdict: &str, digest: &str, rows: &[String]) -> Vec<u8> {
    let mut text = format!("item {ITEM}\nverdict {verdict}\nsnapshot {digest}\n");
    for row in rows {
        text.push_str(row);
        text.push('\n');
    }
    text.into_bytes()
}

/// The proof receipt's exact bytes: three lines only, the proof-set row
/// carrying the declared lanes-rerun string verbatim.
fn expected_receipt(digest: &str, declared: &str) -> Vec<u8> {
    format!("item {ITEM}\nsnapshot {digest}\nproof-set {declared}\n").into_bytes()
}

// --- the wired-runbook shape oracle ------------------------------------------

/// One State's rows for the shape comparison: its inputs, guards, and
/// spawns, plus its prompt body - except the two prompt bodies this
/// contract replaces, which the shape deliberately excludes so the
/// comparison proves everything else is unchanged.
fn state_rows(name: &str, definition: &StateDefinition) -> Vec<String> {
    let guards: Vec<String> = definition
        .guards()
        .iter()
        .map(|guard| format!("{guard:?}"))
        .collect();
    let spawns: Vec<String> = definition
        .spawns()
        .iter()
        .map(|spawn| format!("{spawn:?}"))
        .collect();
    let mut rows = vec![format!(
        "state {name} inputs {:?} guards [{}] spawns [{}]",
        definition.inputs(),
        guards.join(", "),
        spawns.join(", "),
    )];
    let leaf = name.rsplit('.').next().unwrap_or(name);
    if leaf != "implement" && leaf != "damage" {
        rows.push(format!("state {name} prompt {}", definition.prompt()));
    }
    rows
}

/// One transition's row: both States, the input label, and the
/// blocked-route flag - the eight blocked routes among them.
fn transition_row(prefix: &str, transition: &ratmac::graph::Transition) -> String {
    format!(
        "{prefix}transition {} -> {} input {:?} blocked {}",
        transition.from().as_str(),
        transition.to().as_str(),
        transition.input(),
        transition.is_blocked_route()
    )
}

/// The machine's whole shape beside the two prompt bodies this contract
/// replaces: roots, every State's inputs, guards, spawns, and prompt, every
/// transition with its input label and blocked-route flag, and every child
/// class's same rows plus its bindings - t-130's shape helper's pattern,
/// widened to the facts this contract freezes.
fn machine_shape(class: &MachineClass) -> Vec<String> {
    let mut rows: Vec<String> = vec![format!("roots {:?}", class.roots())];
    for (name, definition) in class.states() {
        rows.extend(state_rows(name, definition));
    }
    for transition in class.transitions() {
        rows.push(transition_row("", transition));
    }
    for (name, child) in class.classes() {
        for (state, definition) in child.states() {
            rows.extend(state_rows(&format!("{name}.{state}"), definition));
        }
        for transition in child.transitions() {
            rows.push(transition_row(&format!("{name}."), transition));
        }
        for (binding, declaration) in child.bindings() {
            rows.push(format!(
                "class {name} binding {binding} required {}",
                declaration.required()
            ));
        }
    }
    rows.sort();
    rows
}

// --- the checks ----------------------------------------------------------------

/// WCPV-010: a review rejection invokes no expensive proof command; repair and approval precede the first private-lane command (a behavioral command trace in a minimal workflow fixture, not document scanning), plus the contract's additional `git apply --check` pair over the base runbook bytes.
#[test]
fn wcpv_010() {
    let (isolation, global) = isolation("wcpv010");
    let turn = Turn::new("wcpv010");
    status_control(&global, &turn);
    let trace = isolation.join("wcpv010.trace");
    declare_trace_command(&global, &turn, &trace_command(&trace), None);
    let declared = trace_command(&trace);

    // The rejection is recorded over the exact bytes this invocation stands
    // on: the five report lines and the record's exact bytes.
    let rows = reviewed_rows(&global, &turn.root);
    let rejected_digest = snapshot_digest_of(&rows);
    let review = turn_isolated(
        &global,
        &turn.root,
        &["review", "-Item", ITEM, "-Verdict", "rejected"],
    );
    assert!(
        review.status.success(),
        "t131 WCPV-010: review records the rejected verdict (the review verb and its -Verdict parameter must exist): {}{}",
        String::from_utf8_lossy(&review.stdout),
        String::from_utf8_lossy(&review.stderr)
    );
    assert!(
        review.stderr.is_empty(),
        "t131: a successful review prints no stderr: {}",
        String::from_utf8_lossy(&review.stderr)
    );
    assert_lines(
        &review.stdout,
        &[
            "turn review recorded",
            "  item:      item-1",
            "  verdict:   rejected",
            &format!("  snapshot:  {rejected_digest}"),
            &format!("  inputs:    {}", inputs_phrase(&rows)),
        ],
        "the rejected review's report",
    );
    assert_eq!(
        fs::read(record_path(&turn.root)).expect("the review record exists"),
        expected_record("rejected", &rejected_digest, &rows),
        "t131 WCPV-010: the rejected review record's exact bytes"
    );

    // The rejection invokes no expensive proof command: proofs refuses
    // before the declared command, the trace stays empty, and no receipt
    // exists.
    let refused = turn_isolated(&global, &turn.root, &["proofs", "-Item", ITEM]);
    assert_refusal(
        &refused,
        &[
            "turn refused; the review record items/item-1-review-record.txt records verdict rejected, not accepted"
                .to_owned(),
            "  repair the review findings, then record the accepted review and earn the proof set;"
                .to_owned(),
            "  nothing ran.".to_owned(),
        ],
        "proofs behind the rejected review",
    );
    assert!(
        trace_lines(&trace).is_empty(),
        "t131 WCPV-010: the rejected review invokes no proof command - the declared command's trace stays empty"
    );
    assert!(
        !receipt_path(&turn.root).exists(),
        "t131 WCPV-010: the rejected review earns no receipt"
    );

    // Repair, then approval: the renewed review replaces the record over
    // the repaired bytes, and only then does the first private-lane command
    // run.
    fs::write(turn.root.join("README.md"), "# fixture\nrepaired\n")
        .expect("repair the reviewed bytes");
    let repaired_rows = reviewed_rows(&global, &turn.root);
    let accepted_digest = snapshot_digest_of(&repaired_rows);
    assert_ne!(
        accepted_digest, rejected_digest,
        "t131 WCPV-010: the repaired snapshot differs from the rejected one"
    );
    let accepted = turn_isolated(
        &global,
        &turn.root,
        &["review", "-Item", ITEM, "-Verdict", "accepted"],
    );
    assert!(
        accepted.status.success(),
        "t131 WCPV-010: the renewed review records the accepted verdict over the repaired bytes: {}{}",
        String::from_utf8_lossy(&accepted.stdout),
        String::from_utf8_lossy(&accepted.stderr)
    );
    assert_lines(
        &accepted.stdout,
        &[
            "turn review recorded",
            "  item:      item-1",
            "  verdict:   accepted",
            &format!("  snapshot:  {accepted_digest}"),
            &format!("  inputs:    {}", inputs_phrase(&repaired_rows)),
        ],
        "the accepted review's report",
    );
    assert_eq!(
        fs::read(record_path(&turn.root)).expect("the review record exists"),
        expected_record("accepted", &accepted_digest, &repaired_rows),
        "t131 WCPV-010: the accepted record replaces the rejected one in the one rename"
    );

    let earned = turn_isolated(&global, &turn.root, &["proofs", "-Item", ITEM]);
    assert!(
        earned.status.success(),
        "t131 WCPV-010: proofs earns the proof set behind the accepted review (the proofs verb must exist): {}{}",
        String::from_utf8_lossy(&earned.stdout),
        String::from_utf8_lossy(&earned.stderr)
    );
    assert!(
        earned.stderr.is_empty(),
        "t131: a successful proofs prints no stderr: {}",
        String::from_utf8_lossy(&earned.stderr)
    );
    let checkout = checkout_text(&global, &turn.root);
    assert_lines(
        &earned.stdout,
        &[
            "turn proofs earned",
            "  item:      item-1",
            &format!("  snapshot:  {accepted_digest}"),
            &format!("  command:   {declared}"),
            &format!("  ran:       once from {checkout}"),
            "  next:      commit the green work as 'item-1: checkpoint - not a landing', then run: pwsh -File tools/turn.ps1 checkpoint -Item item-1 -Target <path>...",
        ],
        "the earned proof set's report",
    );
    assert_eq!(
        fs::read(receipt_path(&turn.root)).expect("the proof receipt exists"),
        expected_receipt(&accepted_digest, &declared),
        "t131 WCPV-010: the proof receipt's exact bytes"
    );
    assert_eq!(
        trace_lines(&trace),
        vec![checkout],
        "t131 WCPV-010: approval precedes the first private-lane command - exactly one invocation, from the addressed checkout"
    );

    // The contract's additional assertion for this file: "The `git apply
    // --check` pair over the base bytes is an additional assertion in
    // `test/qa/tests/t131_review_before_proofs.rs` beside the three
    // declared checks - no adjustment to any existing suite." The base
    // bytes are the recorded merge-base runbook embedded in this suite
    // (its sha256 asserted below): the repository's tracked
    // `.ratmac/ratmac.toml` is never read, because the cycle boundary
    // wires both diffs into it and this suite must stay green through
    // that wiring; the two tracked diff artifacts are read.
    assert_eq!(
        {
            let mut hasher = Sha256::new();
            hasher.update(BASE_RUNBOOK.as_bytes());
            format!("{:x}", hasher.finalize())
        },
        BASE_RUNBOOK_SHA256,
        "t131 WCPV-010: the suite's embedded base copy is the recorded merge-base runbook"
    );
    let worktree = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pause_routes = fs::read(worktree.join(".ratmac/pause-routes.diff"))
        .expect("read the tracked pause-routes diff (t-130's landed artifact)");
    let review_proofs = fs::read(worktree.join(".ratmac/review-proofs.diff"))
        .expect("read .ratmac/review-proofs.diff - the tracked artifact this contract ships");
    let pair = TempTree::new("t131-runbookpair").expect("create the runbook pair fixture");
    let repo = pair.join("repo");
    fs::create_dir_all(repo.join(".ratmac")).expect("create the pair fixture's data directory");
    fs::write(repo.join(".ratmac/ratmac.toml"), BASE_RUNBOOK)
        .expect("write a clean copy of the base runbook bytes");
    fs::write(repo.join(".ratmac/pause-routes.diff"), &pause_routes)
        .expect("install the pause-routes diff");
    fs::write(repo.join(".ratmac/review-proofs.diff"), &review_proofs)
        .expect("install the review-proofs diff");
    let pair_global = pair.join("git-global.cfg");
    fs::write(&pair_global, b"").expect("write the pair fixture's empty global config");
    let init = git_isolated(
        &pair_global,
        &repo,
        &["init", "--initial-branch", "main", "."],
    );
    assert!(
        init.status.success(),
        "t131: the pair fixture initializes: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    let checked = git_isolated(
        &pair_global,
        &repo,
        &["apply", "--check", ".ratmac/pause-routes.diff"],
    );
    assert!(
        checked.status.success(),
        "t131 WCPV-010: over a clean copy of the base runbook bytes, git apply --check .ratmac/pause-routes.diff exits 0: {}",
        String::from_utf8_lossy(&checked.stderr)
    );
    let applied = git_isolated(&pair_global, &repo, &["apply", ".ratmac/pause-routes.diff"]);
    assert!(
        applied.status.success(),
        "t131: applying the pause-routes diff succeeds: {}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let routed = fs::read_to_string(repo.join(".ratmac/ratmac.toml"))
        .expect("read the pause-routes-wired copy");
    let routed_class = MachineClass::from_toml(&routed)
        .expect("the pause-routes-wired runbook parses through the one reader");
    let paired = git_isolated(
        &pair_global,
        &repo,
        &["apply", "--check", ".ratmac/review-proofs.diff"],
    );
    assert!(
        paired.status.success(),
        "t131 WCPV-010: applying it and then running git apply --check .ratmac/review-proofs.diff exits 0: {}",
        String::from_utf8_lossy(&paired.stderr)
    );
    let rewired = git_isolated(
        &pair_global,
        &repo,
        &["apply", ".ratmac/review-proofs.diff"],
    );
    assert!(
        rewired.status.success(),
        "t131: applying the review-proofs diff succeeds: {}",
        String::from_utf8_lossy(&rewired.stderr)
    );
    let wired = fs::read_to_string(repo.join(".ratmac/ratmac.toml")).expect("read the wired copy");
    // The bytes both produce parse through the one reader and carry exactly
    // the two final prompt strings beside t-130's eight routes.
    let wired_class = MachineClass::from_toml(&wired)
        .expect("t131 WCPV-010: the bytes both produce parse through the one reader");
    let ticket_class = wired_class
        .classes()
        .get("ticket")
        .expect("the wired runbook carries the ticket class");
    assert_eq!(
        ticket_class
            .states()
            .get("implement")
            .expect("the ticket class carries the implement State")
            .prompt(),
        IMPLEMENT_PROMPT,
        "t131 WCPV-010: the wired implement prompt is the contract's final string, byte for byte"
    );
    assert_eq!(
        ticket_class
            .states()
            .get("damage")
            .expect("the ticket class carries the damage State")
            .prompt(),
        DAMAGE_PROMPT,
        "t131 WCPV-010: the wired damage prompt is the contract's final string, byte for byte"
    );
    assert_eq!(
        machine_shape(&wired_class),
        machine_shape(&routed_class),
        "t131 WCPV-010: beside the two replaced prompts, the wired runbook changes no State, guard, transition (the eight blocked routes included), binding, or root versus the pause-routes-wired bytes"
    );
}

/// WCPV-011: one unchanged approved snapshot earns its private/damage proof set once after review, while every required restored-green rerun and the declared post-merge verification from main still execute - the three command-order events instrumented separately, so removing any mandatory rerun fails this check.
#[test]
fn wcpv_011() {
    let (isolation, global) = isolation("wcpv011");
    let turn = Turn::new("wcpv011");
    status_control(&global, &turn);
    let trace = isolation.join("wcpv011.trace");
    declare_trace_command(&global, &turn, &trace_command(&trace), Some("root-once"));
    let declared = trace_command(&trace);

    // One clean turn: the green work lives in the ticket worktree, committed
    // under the safety subject, and the review, the proof set, and the
    // damage run all live there.
    let worktree = turn.open_with_work(ITEM, "the reviewed green work");
    git_ok(
        &global,
        &worktree,
        &["commit", "--amend", "-m", SAFETY_SUBJECT],
    );
    let head = git_ok(&global, &worktree, &["rev-parse", "HEAD"]);
    let worktree_checkout = checkout_text(&global, &worktree);
    let readme = worktree.join("README.md");

    let rows = reviewed_rows(&global, &worktree);
    let digest = snapshot_digest_of(&rows);
    let review = turn_isolated(
        &global,
        &worktree,
        &["review", "-Item", ITEM, "-Verdict", "accepted"],
    );
    assert!(
        review.status.success(),
        "t131 WCPV-011: the ticket worktree's review records the accepted verdict (the review verb must exist): {}{}",
        String::from_utf8_lossy(&review.stdout),
        String::from_utf8_lossy(&review.stderr)
    );
    assert_lines(
        &review.stdout,
        &[
            "turn review recorded",
            "  item:      item-1",
            "  verdict:   accepted",
            &format!("  snapshot:  {digest}"),
            &format!("  inputs:    {}", inputs_phrase(&rows)),
        ],
        "the accepted review's report",
    );
    assert_eq!(
        fs::read(record_path(&worktree)).expect("the review record exists"),
        expected_record("accepted", &digest, &rows),
        "t131 WCPV-011: the review record's exact bytes"
    );

    // The proof-earning event: the declared command runs exactly once from
    // the ticket worktree, and the receipt binds that snapshot.
    let earned = turn_isolated(&global, &worktree, &["proofs", "-Item", ITEM]);
    assert!(
        earned.status.success(),
        "t131 WCPV-011: proofs earns the proof set behind the accepted review (the proofs verb must exist): {}{}",
        String::from_utf8_lossy(&earned.stdout),
        String::from_utf8_lossy(&earned.stderr)
    );
    assert_lines(
        &earned.stdout,
        &[
            "turn proofs earned",
            "  item:      item-1",
            &format!("  snapshot:  {digest}"),
            &format!("  command:   {declared}"),
            &format!("  ran:       once from {worktree_checkout}"),
            "  next:      commit the green work as 'item-1: checkpoint - not a landing', then run: pwsh -File tools/turn.ps1 checkpoint -Item item-1 -Target <path>...",
        ],
        "the earned proof set's report",
    );
    assert_eq!(
        fs::read(receipt_path(&worktree)).expect("the proof receipt exists"),
        expected_receipt(&digest, &declared),
        "t131 WCPV-011: the proof receipt's exact bytes"
    );
    assert_eq!(
        trace_lines(&trace),
        vec![worktree_checkout.clone()],
        "t131 WCPV-011: the proof-earning event is the declared command's one invocation from the ticket worktree"
    );
    let mut events = vec![format!("proof-earning: {worktree_checkout}")];

    // The freeze takes only reviewed, proven green work (the inserted
    // checkpoint gate), and commits no byte change: the record stays fresh.
    let frozen = turn_isolated(
        &global,
        &worktree,
        &["checkpoint", "-Item", ITEM, "-Target", "README.md"],
    );
    assert!(
        frozen.status.success(),
        "t131 WCPV-011: the checkpoint freezes the reviewed, proven green work: {}{}",
        String::from_utf8_lossy(&frozen.stdout),
        String::from_utf8_lossy(&frozen.stderr)
    );
    assert_lines(
        &frozen.stdout,
        &[
            "turn checkpoint frozen",
            "  item:      item-1",
            &format!("  commit:    {head}"),
            "  manifest:  items/item-1-checkpoint-manifest.txt (1 target, round-trip proven)",
            "  damage:    mutate only the declared targets, then run: pwsh -File tools/turn.ps1 restore -Item item-1",
        ],
        "the checkpoint report",
    );

    // Deliberate damage, one mutation at a time: the snapshot is
    // deliberately stale, proofs refuses and invokes no command, and the
    // restored-green rerun after the restore is mandatory - never a
    // re-earned proof set.
    let green = fs::read_to_string(&readme).expect("read the green bytes");
    for (round, damage) in ["# damaged one\n", "# damaged two\n"].iter().enumerate() {
        fs::write(&readme, damage).expect("damage the declared target");
        let stale = turn_isolated(&global, &worktree, &["proofs", "-Item", ITEM]);
        assert_refusal(
            &stale,
            &[
                "turn refused; the reviewed snapshot changed since the review recorded it"
                    .to_owned(),
                format!(
                    "  the review record items/{ITEM}-review-record.txt binds snapshot {digest}; the addressed checkout now hashes {};",
                    snapshot_digest_of(&reviewed_rows(&global, &worktree))
                ),
                "  changed inputs:".to_owned(),
                "    work README.md".to_owned(),
                format!(
                    "  renew the review over the repaired bytes: pwsh -File tools/turn.ps1 review -Item {ITEM} -Verdict <accepted|rejected>"
                ),
                "  nothing ran.".to_owned(),
            ],
            "proofs mid-damage",
        );
        assert_eq!(
            trace_lines(&trace),
            vec![worktree_checkout.clone()],
            "t131 WCPV-011: the mid-damage stale refusal invokes no proof command"
        );
        let verified = turn_isolated(&global, &worktree, &["restore", "-Item", ITEM]);
        assert!(
            verified.status.success(),
            "t131 WCPV-011: the mandatory restore returns the damaged bytes (round {}): {}{}",
            round + 1,
            String::from_utf8_lossy(&verified.stdout),
            String::from_utf8_lossy(&verified.stderr)
        );
        // The restored-green rerun: the fixture's own focused check, rerun
        // green after every restore.
        assert_eq!(
            fs::read_to_string(&readme).expect("reread the target"),
            green,
            "t131 WCPV-011: the restored-green rerun (round {}) finds the green bytes",
            round + 1
        );
        events.push(format!("restored-green: {}", round + 1));
    }

    // The once-only row: the bytes match the recorded digest again, the same
    // receipt still binds them, and no re-earn is needed or allowed.
    let again = turn_isolated(&global, &worktree, &["proofs", "-Item", ITEM]);
    assert_refusal(
        &again,
        &[
            format!(
                "turn refused; the proof set for {ITEM} is already earned at snapshot {digest}"
            ),
            "  the once-only rule earns one complete proof set per accepted review;".to_owned(),
            "  a renewed proof set needs a renewed review over changed bytes.".to_owned(),
            "  nothing ran.".to_owned(),
        ],
        "the second proofs behind the unchanged snapshot",
    );
    assert_eq!(
        fs::read(receipt_path(&worktree)).expect("reread the receipt"),
        expected_receipt(&digest, &declared),
        "t131 WCPV-011: the once-only refusal leaves the receipt binding the same snapshot"
    );
    assert_eq!(
        trace_lines(&trace),
        vec![worktree_checkout.clone()],
        "t131 WCPV-011: the once-only refusal runs no command"
    );

    // The amended contract: the records "are the turn's own gate state, so
    // the operator deletes them before `close`: `close` keeps its landed
    // bytes, and both its only-copy rule (THK-003) and `git worktree
    // remove` refuse while an untracked file remains in the worktree".
    // The turn's three untracked records are deleted, never committed.
    for untracked in [
        record_path(&worktree),
        receipt_path(&worktree),
        worktree.join(format!("{ITEM_RECORDS}/{ITEM}-checkpoint-manifest.txt")),
    ] {
        assert!(
            untracked.exists(),
            "t131 WCPV-011: the turn's untracked record {} exists before the operator deletes it",
            untracked.display()
        );
        fs::remove_file(&untracked)
            .unwrap_or_else(|error| panic!("delete {}: {error}", untracked.display()));
    }

    // Landing and merge; close step 7 runs the main verification ungated
    // from the primary checkout.
    let closed = turn_isolated(
        &global,
        &turn.root,
        &["close", "-Item", ITEM, "-Line", "t-131 fixture landing"],
    );
    assert!(
        closed.status.success(),
        "t131 WCPV-011: the close lands the reviewed work: {}{}",
        String::from_utf8_lossy(&closed.stdout),
        String::from_utf8_lossy(&closed.stderr)
    );
    assert!(
        lines(&closed.stdout).contains(
            &"step 7/7 lanes rerun: ran once from the primary checkout (scope root-once)"
                .to_owned()
        ),
        "t131 WCPV-011: the main-verification event is close step 7's ungated root-once line: {}",
        String::from_utf8_lossy(&closed.stdout)
    );
    let primary = checkout_text(&global, &turn.root);
    assert_eq!(
        trace_lines(&trace),
        vec![worktree_checkout.clone(), primary.clone()],
        "t131 WCPV-011: exactly one proof-earning event and one main-verification event - removing any mandatory rerun fails this check"
    );
    events.push(format!("main-verification: {primary}"));
    assert_eq!(
        events,
        vec![
            format!("proof-earning: {worktree_checkout}"),
            "restored-green: 1".to_owned(),
            "restored-green: 2".to_owned(),
            format!("main-verification: {primary}"),
        ],
        "t131 WCPV-011: the three command-order events, separately instrumented"
    );
}

/// WCPV-012: an edit after review or proof invalidates the corresponding evidence and requires a renewed review/proof; missing private lanes still refuse through the declared command's own failure.
#[test]
fn wcpv_012() {
    let (isolation, global) = isolation("wcpv012");
    let turn = Turn::new("wcpv012");
    status_control(&global, &turn);
    let trace = isolation.join("wcpv012.trace");
    declare_trace_command(&global, &turn, &trace_command(&trace), None);
    let declared = trace_command(&trace);

    // The earned state: an accepted review and one earned proof set.
    let mut rows = reviewed_rows(&global, &turn.root);
    let mut digest = snapshot_digest_of(&rows);
    let review = turn_isolated(
        &global,
        &turn.root,
        &["review", "-Item", ITEM, "-Verdict", "accepted"],
    );
    assert!(
        review.status.success(),
        "t131 WCPV-012: the accepted review records the snapshot (the review verb must exist): {}{}",
        String::from_utf8_lossy(&review.stdout),
        String::from_utf8_lossy(&review.stderr)
    );
    let earned = turn_isolated(&global, &turn.root, &["proofs", "-Item", ITEM]);
    assert!(
        earned.status.success(),
        "t131 WCPV-012: proofs earns the proof set behind the accepted review (the proofs verb must exist): {}{}",
        String::from_utf8_lossy(&earned.stdout),
        String::from_utf8_lossy(&earned.stderr)
    );
    assert_eq!(
        trace_lines(&trace).len(),
        1,
        "t131 WCPV-012: earning ran the declared command once"
    );

    // An edit after review and proof invalidates both gates: proofs refuses
    // the stale row naming the changed work path, running nothing.
    fs::write(turn.root.join("README.md"), "# fixture\nedited\n")
        .expect("edit a reviewed source byte");
    let edited_rows = reviewed_rows(&global, &turn.root);
    let edited_digest = snapshot_digest_of(&edited_rows);
    let record_bytes = fs::read(record_path(&turn.root)).expect("keep the record bytes");
    let receipt_bytes = fs::read(receipt_path(&turn.root)).expect("keep the receipt bytes");
    let stale = turn_isolated(&global, &turn.root, &["proofs", "-Item", ITEM]);
    assert_refusal(
        &stale,
        &[
            "turn refused; the reviewed snapshot changed since the review recorded it".to_owned(),
            format!(
                "  the review record items/{ITEM}-review-record.txt binds snapshot {digest}; the addressed checkout now hashes {edited_digest};"
            ),
            "  changed inputs:".to_owned(),
            "    work README.md".to_owned(),
            format!(
                "  renew the review over the repaired bytes: pwsh -File tools/turn.ps1 review -Item {ITEM} -Verdict <accepted|rejected>"
            ),
            "  nothing ran.".to_owned(),
        ],
        "proofs over the edited snapshot",
    );
    assert_eq!(
        fs::read(record_path(&turn.root)).expect("reread the record"),
        record_bytes,
        "t131 WCPV-012: the stale refusal leaves the record bytes as they read"
    );
    assert_eq!(
        fs::read(receipt_path(&turn.root)).expect("reread the receipt"),
        receipt_bytes,
        "t131 WCPV-012: the stale refusal leaves the receipt bytes as they read"
    );
    assert_eq!(
        trace_lines(&trace).len(),
        1,
        "t131 WCPV-012: the stale refusal invokes no proof command"
    );

    // The same edit refuses the checkpoint gate with its own last line.
    git_ok(&global, &turn.root, &["add", "--", "README.md"]);
    git_ok(&global, &turn.root, &["commit", "-m", SAFETY_SUBJECT]);
    let gate = turn_isolated(
        &global,
        &turn.root,
        &["checkpoint", "-Item", ITEM, "-Target", "README.md"],
    );
    assert_refusal(
        &gate,
        &[
            "turn refused; the reviewed snapshot changed since the review recorded it".to_owned(),
            format!(
                "  the review record items/{ITEM}-review-record.txt binds snapshot {digest}; the addressed checkout now hashes {edited_digest};"
            ),
            "  changed inputs:".to_owned(),
            "    work README.md".to_owned(),
            format!(
                "  renew the review over the repaired bytes: pwsh -File tools/turn.ps1 review -Item {ITEM} -Verdict <accepted|rejected>"
            ),
            "  nothing was written.".to_owned(),
        ],
        "the checkpoint gate over the edited snapshot",
    );
    assert!(
        !turn
            .root
            .join(format!("{ITEM_RECORDS}/{ITEM}-checkpoint-manifest.txt"))
            .exists(),
        "t131 WCPV-012: the refused gate freezes no manifest"
    );

    // Renewal: a renewed review records the repaired bytes and a renewed
    // proofs earns over them, the old receipt superseded and replaced.
    rows = edited_rows;
    digest = edited_digest;
    let renewed = turn_isolated(
        &global,
        &turn.root,
        &["review", "-Item", ITEM, "-Verdict", "accepted"],
    );
    assert!(
        renewed.status.success(),
        "t131 WCPV-012: the renewed review records the repaired bytes: {}{}",
        String::from_utf8_lossy(&renewed.stdout),
        String::from_utf8_lossy(&renewed.stderr)
    );
    assert_eq!(
        fs::read(record_path(&turn.root)).expect("reread the record"),
        expected_record("accepted", &digest, &rows),
        "t131 WCPV-012: the renewed review record replaces the stale one"
    );
    let reearned = turn_isolated(&global, &turn.root, &["proofs", "-Item", ITEM]);
    assert!(
        reearned.status.success(),
        "t131 WCPV-012: the renewed proofs earns over the renewed review: {}{}",
        String::from_utf8_lossy(&reearned.stdout),
        String::from_utf8_lossy(&reearned.stderr)
    );
    assert_eq!(
        fs::read(receipt_path(&turn.root)).expect("reread the receipt"),
        expected_receipt(&digest, &declared),
        "t131 WCPV-012: the renewed receipt replaces the superseded one and binds the new snapshot"
    );
    assert_eq!(
        trace_lines(&trace).len(),
        2,
        "t131 WCPV-012: the renewed proofs ran the declared command again over the changed bytes"
    );

    // A lane edit is the same invalidation, naming the lane row: source,
    // public oracle, or private lane - every reviewed byte counts.
    let lane = turn.root.join(format!("{LANES}/crate-a/lane.txt"));
    fs::write(&lane, "lane a edited\n").expect("edit a reviewed lane byte");
    let lane_rows = reviewed_rows(&global, &turn.root);
    let lane_digest = snapshot_digest_of(&lane_rows);
    let lane_stale = turn_isolated(&global, &turn.root, &["proofs", "-Item", ITEM]);
    assert_refusal(
        &lane_stale,
        &[
            "turn refused; the reviewed snapshot changed since the review recorded it".to_owned(),
            format!(
                "  the review record items/{ITEM}-review-record.txt binds snapshot {digest}; the addressed checkout now hashes {lane_digest};"
            ),
            "  changed inputs:".to_owned(),
            "    lane crate-a/lane.txt".to_owned(),
            format!(
                "  renew the review over the repaired bytes: pwsh -File tools/turn.ps1 review -Item {ITEM} -Verdict <accepted|rejected>"
            ),
            "  nothing ran.".to_owned(),
        ],
        "proofs over the edited lane byte",
    );
    assert_eq!(
        trace_lines(&trace).len(),
        2,
        "t131 WCPV-012: the lane-edit refusal invokes no proof command"
    );

    // A missing private lane refuses through the declared command's own
    // failure: no receipt is written, and the checkpoint keeps refusing.
    let lane_fixture = Turn::new("wcpv012missing");
    let missing_command = "if (Test-Path lanes/crate-b/lane.txt) { exit 0 } else { exit 3 }";
    declare_trace_command(&global, &lane_fixture, missing_command, None);
    git_ok(
        &global,
        &lane_fixture.root,
        &["commit", "--allow-empty", "-m", SAFETY_SUBJECT],
    );
    let missing_checkout = checkout_text(&global, &lane_fixture.root);
    let missing_review = turn_isolated(
        &global,
        &lane_fixture.root,
        &["review", "-Item", ITEM, "-Verdict", "accepted"],
    );
    assert!(
        missing_review.status.success(),
        "t131 WCPV-012: the review records over the bytes that are present: {}{}",
        String::from_utf8_lossy(&missing_review.stdout),
        String::from_utf8_lossy(&missing_review.stderr)
    );
    let missing = turn_isolated(&global, &lane_fixture.root, &["proofs", "-Item", ITEM]);
    assert_refusal(
        &missing,
        &[
            format!("turn refused; the proof-set command failed in {missing_checkout} (exit 3)"),
            format!("  the verification command was: {missing_command}"),
            format!("  it ran once from {missing_checkout}"),
            "  no receipt was written; repair the reported failure, then run proofs again."
                .to_owned(),
        ],
        "proofs behind a missing private lane",
    );
    assert!(
        !receipt_path(&lane_fixture.root).exists(),
        "t131 WCPV-012: the failed declared command writes no receipt"
    );
    let unfreezable = turn_isolated(
        &global,
        &lane_fixture.root,
        &["checkpoint", "-Item", ITEM, "-Target", "README.md"],
    );
    assert_refusal(
        &unfreezable,
        &[
            "turn refused; the proof receipt items/item-1-proof-receipt.txt does not exist"
                .to_owned(),
            "  the checkpoint freezes only reviewed, proven green work;".to_owned(),
            "  run proofs -Item item-1, then checkpoint again.".to_owned(),
            "  nothing was written.".to_owned(),
        ],
        "the checkpoint behind the missing private lane",
    );
    assert!(
        !lane_fixture
            .root
            .join(format!("{ITEM_RECORDS}/{ITEM}-checkpoint-manifest.txt"))
            .exists(),
        "t131 WCPV-012: the refused checkpoint freezes no manifest"
    );
}
