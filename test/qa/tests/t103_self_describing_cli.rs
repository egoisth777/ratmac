//! t-103 / AOP-001, AOP-002: the engine's own output teaches the loop.
//!
//! AOPV-001 `status_names_what_each_guard_reads`
//! AOPV-002 `every_outcome_ends_in_one_truthful_next_line`
//!
//! AOPV-001 pins, for each of the eight guard kinds the engine supports, the
//! exact line `rtm status` renders for a pending guard: the guard's kind plus
//! what it reads, spelled from the parsed declaration. Path-bearing kinds name
//! their declared root/path/entries/program/args/address; the contract kinds,
//! which declare no fields of their own, name their fixed roles with the
//! paths the runbook's `[roots]` table declares for them - the derivation
//! reaches the whole parsed runbook, never hand-kept prose.
//!
//! AOPV-002 walks every status/step outcome class a fixture can reach -
//! success renders on live and aged Runs, guard refusal, terminal omission,
//! unknown run id, missing `--run`, corrupt Run Record - and judges each
//! rendering against a golden repair table: the exercised class and its
//! stable code map to the one exact repair command shape the engine must
//! render, parameterized only by the addressed run id. The rendered `next:`
//! line must equal its table row, stand as the outcome's last line, and name
//! only subcommands the engine dispatches; then the test EXECUTES the named
//! repair against the fixture and asserts the engine accepts it in that
//! state. Terminal renders are the one deliberate omission and must carry no
//! `next:` line at all - every other rendering owes exactly one, so neither a
//! bogus `rtm status` on a refused id nor a silent zero can pass.
//!
//! Hole-poke notes:
//! - Would AOPV-001 pass with hardcoded artifact names? No. The golden pair
//!   renames `files_exact`'s declared entry in the runbook only and re-renders
//!   with no render-code edit: the pinned line must name `handoff.txt`, the
//!   `proof.txt` rendering must be gone from the output, and the guard's
//!   verdict must name the renamed entry in the same refusal. Hand-kept prose
//!   cannot follow a declaration it never re-read.
//! - Would AOPV-002 pass with a `next:` line naming a refused command? No.
//!   The unknown-id refusal's `next:` must not address `run-999`, the corrupt
//!   record's must not address the corrupt Run, and the terminal Run's must
//!   not name `rtm step` on that Run - each of those is refused in exactly
//!   that state. Every named repair is executed and must exit 0, so a
//!   command the engine would refuse cannot hide behind loose wording. The
//!   one deliberate naming of a just-refused verb is the guard refusal's
//!   `rtm step --run <id>`: R-020 keeps step accepted in that state once the
//!   artifact is repaired, and the retry is pinned byte-identical.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// AOPV-001: one runbook declaring every guard kind the engine supports, one
/// per State, chained so each State is reachable and `stock` is initial.
const RUNBOOK_ALL_KINDS: &str = r#"[roots]
goal = ".arca/goal"
issue = ".arca/issue"
residual = ".arca/residual"
ticket = ".arca/ticket"
work = "artifacts"

[states.stock]
prompt = "Place the declared files."
guards = [{ kind = "files_exact", root = "work", path = "release", entries = ["proof.txt"] }]

[states.scan]
prompt = "Write the marker line."
guards = [{ kind = "file_contains", path = "artifacts/status.txt", contains = "READY" }]

[states.probe]
prompt = "Run the toolchain probe."
guards = [{ kind = "command_exit", program = "rustc", args = ["--version"], expected = 0, exempt = true }]

[states.receipts]
prompt = "Record the receipts."
guards = [{ kind = "sensitivity_receipts", root = "ticket", ticket = "t-900.md" }]

[states.gate]
prompt = "Pass the gate."
guards = [{ kind = "completion_gate", root = "ticket", ticket = "t-900.md" }]

[states.intake]
prompt = "Close the intake."
guards = [{ kind = "intake_contract" }]

[states.records]
prompt = "Close the records."
guards = [{ kind = "record_contract" }]

[states.delegate]
prompt = "Wait for the child."
guards = [{ kind = "join", require = "all_passed", min = 1 }]

[states.done]
prompt = "Done."

[[transitions]]
from = "stock"
to = "scan"

[[transitions]]
from = "scan"
to = "probe"

[[transitions]]
from = "probe"
to = "receipts"

[[transitions]]
from = "receipts"
to = "gate"

[[transitions]]
from = "gate"
to = "intake"

[[transitions]]
from = "intake"
to = "records"

[[transitions]]
from = "records"
to = "delegate"

[[transitions]]
from = "delegate"
to = "done"
"#;

/// AOPV-002: a machine whose one guard fails until the fixture places the
/// artifact, so one Run walks success, guard refusal, and both terminal
/// renders; the remaining outcome classes refuse before motion.
const RUNBOOK_OUTCOMES: &str = r#"[states.prepare]
prompt = "Place the handoff artifact."
guards = [{ kind = "files_exact", path = "handoff/proof.txt" }]

[states.middle]
prompt = "Move through the middle."

[states.done]
prompt = "The run is complete."

[[transitions]]
from = "prepare"
to = "middle"

[[transitions]]
from = "middle"
to = "done"
"#;

/// A Run Record in the exact seven-field shape the engine wrote before this
/// change - the same bytes as the checked-in pre-change fixture record at
/// `test/fixtures/r028-state-prompt/.ratmac/run.toml`. GPH-001: the aged
/// record must still receive the new rendering; nothing about a Run Record
/// had to migrate for the CLI to teach.
const AGED_RECORD_PREFIX: &str = r#"status = "executing"
goal_revision = "goal-r1"
input_revision = "input-r1"
output_revision = "output-r0"
active_refs = []
blocker = ""
"#;

/// The subcommands the engine actually dispatches (`help`, `src/cli.rs`).
const SUBCOMMANDS: [&str; 9] = [
    "start", "status", "step", "hold", "abandon", "spawn", "respawn", "doctor", "scaffold",
];

/// One row of AOPV-002's golden repair table: the outcome class the row pins,
/// the stable code its refusal family carries (empty for success renders),
/// and the exact `next:` line the engine must render for it - the single
/// `{}` takes the addressed run id, so nothing else may vary.
struct RepairRow {
    class: &'static str,
    code: &'static str,
    shape: &'static str,
}

/// AOPV-002's golden repair table. Every exercised outcome class maps to one
/// exact repair command shape; a rendering that names anything else, or
/// nothing, is an invention this table catches.
const REPAIRS: [RepairRow; 8] = [
    RepairRow {
        class: "status on a live Run",
        code: "",
        shape: "next: rtm step --run {}",
    },
    RepairRow {
        class: "step guard refusal",
        code: "R-020",
        shape: "next: rtm step --run {}",
    },
    RepairRow {
        class: "step success onto a non-terminal State",
        code: "",
        shape: "next: rtm step --run {}",
    },
    RepairRow {
        class: "step on a terminal Run",
        code: "R-019",
        shape: "next: rtm status --run {}",
    },
    RepairRow {
        class: "unknown run id",
        code: "FDC-004",
        shape: "next: rtm status --run {}",
    },
    RepairRow {
        class: "missing --run",
        code: "FDC-004",
        shape: "next: rtm status --run {}",
    },
    RepairRow {
        class: "malformed Run Record",
        code: "R-027",
        shape: "next: rtm start",
    },
    RepairRow {
        class: "status on a pre-change record",
        code: "",
        shape: "next: rtm step --run {}",
    },
];

/// The table row an assertion names, by class.
fn repair(class: &str) -> &'static RepairRow {
    REPAIRS
        .iter()
        .find(|row| row.class == class)
        .unwrap_or_else(|| panic!("AOPV-002: unknown repair-table class {class:?}"))
}

struct Fixture {
    base: PathBuf,
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

impl Fixture {
    fn new(label: &str, runbook: &str) -> Self {
        let base = std::env::temp_dir().join(format!(
            "ratmac-t103-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&base);
        let root = base.join("project");
        fs::create_dir_all(root.join(".ratmac")).expect("create the Engine root");
        fs::write(root.join(".ratmac/ratmac.toml"), runbook).expect("write the runbook");
        Fixture { base, root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    /// Seed one addressed Run parked at `state`, in the plain record shape a
    /// pre-change engine wrote. The parked Run renders that State's pending
    /// guards without first satisfying the guards before it.
    fn seed(&self, run_id: &str, state: &str) {
        let dir = self.root.join(".ratmac/runs").join(run_id);
        fs::create_dir_all(&dir).expect("create the seeded Run directory");
        let record = format!("state = \"{state}\"\n{AGED_RECORD_PREFIX}");
        fs::write(dir.join("run.toml"), record).expect("write the seeded Run Record");
    }

    fn status_text(&self, run_id: &str) -> String {
        let output = self.rtm(&["status", "--run", run_id]);
        let text = text(&output);
        assert!(
            output.status.success(),
            "status on {run_id} is a success render: {text}"
        );
        text
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Every line of a rendering that teaches a next act.
fn next_lines(rendered: &str) -> Vec<&str> {
    rendered
        .lines()
        .filter(|line| line.starts_with("next: "))
        .collect()
}

/// The one `next:` line a rendering carries, refusing zero and doubles alike:
/// every outcome class except a terminal render owes exactly one.
fn the_next_line<'a>(rendered: &'a str, context: &str) -> &'a str {
    let lines = next_lines(rendered);
    assert_eq!(
        lines.len(),
        1,
        "AOPV-002: {context} must end in exactly one next: line: {rendered}"
    );
    lines[0]
}

/// The last non-empty line of a rendering - where a `next:` line must stand.
fn last_line(rendered: &str) -> &str {
    rendered
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default()
}

/// Every `rtm <token>` a `next:` line names is a subcommand the engine
/// dispatches, and at least one is named.
fn names_only_real_subcommands(next: &str) -> bool {
    let mut named = false;
    let mut rest = next;
    while let Some(position) = rest.find("rtm ") {
        rest = &rest[position + "rtm ".len()..];
        let token = rest.split_whitespace().next().unwrap_or_default();
        assert!(
            SUBCOMMANDS.contains(&token),
            "AOPV-002: a next: line must name a real subcommand, not {token:?}: {next}"
        );
        named = true;
    }
    named
}

/// AOPV-002's golden oracle: the rendering's one `next:` line equals the
/// repair table's exact shape for `class` (parameterized by `run_id`), stands
/// as the outcome's last line, and names only real subcommands.
fn assert_table_next(rendered: &str, class: &str, run_id: &str, context: &str) {
    let row = repair(class);
    let expected = row.shape.replace("{}", run_id);
    let next = the_next_line(rendered, context);
    assert_eq!(
        next, expected,
        "AOPV-002: {context}: the {} repair (code {}) must be exactly {expected:?}: {rendered}",
        row.class, row.code
    );
    assert_eq!(
        last_line(rendered),
        next,
        "AOPV-002: {context}: the next: line must be the outcome's last line: {rendered}"
    );
    assert!(
        names_only_real_subcommands(next),
        "AOPV-002: {context}: the next: line must name a command the engine dispatches: {rendered}"
    );
}

/// AOPV-002's terminal oracle: the deliberate omission. A terminal render
/// carries no `next:` line at all - nothing can be stood behind, so nothing
/// is guessed.
fn assert_terminal_omits(rendered: &str, context: &str) {
    assert!(
        next_lines(rendered).is_empty(),
        "AOPV-002: {context}: a terminal render omits the next: line rather than guessing: {rendered}"
    );
}

/// Execute the exact `next:` line a rendering taught and assert the engine
/// accepts it in the state that rendered it. Acceptance is exit 0: a refused
/// step renders its refusal and exits 0 (R-020), so even the repair that
/// re-runs a refused verb is proven safe, and any command the engine would
/// actually refuse fails here.
fn assert_repair_accepted(fixture: &Fixture, next: &str, context: &str) -> Output {
    let tokens = next
        .strip_prefix("next: ")
        .unwrap_or_else(|| panic!("AOPV-002: {context}: malformed next: line {next:?}"))
        .split_whitespace()
        .collect::<Vec<_>>();
    let repaired = fixture.rtm(&tokens);
    let repaired_text = text(&repaired);
    assert!(
        repaired.status.success(),
        "AOPV-002: {context}: the engine must accept the taught repair {next:?} in that \
         state: {repaired_text}"
    );
    repaired
}

/// AOPV-001 / AOP-001: for every guard kind the engine supports, `rtm status`
/// names, on the pending-guard line itself, what that guard reads - spelled
/// from the parsed declaration, proven by renaming a declared artifact in the
/// runbook only and watching the rendering and the verdict follow.
#[test]
fn status_names_what_each_guard_reads() {
    let fixture = Fixture::new("guards", RUNBOOK_ALL_KINDS);
    for dir in [
        ".arca/goal",
        ".arca/issue",
        ".arca/residual",
        ".arca/ticket",
        "artifacts/release",
    ] {
        fs::create_dir_all(fixture.root.join(dir)).expect("create the declared roots");
    }
    fs::write(
        fixture.root.join("artifacts/release/proof.txt"),
        "handoff evidence\n",
    )
    .expect("place the declared artifact");

    // The fixture runbook must be structurally sound: the doctor may warn
    // (RB302 flags agent-writable verdicts) but never error.
    let doctor = fixture.rtm(&["doctor"]);
    let doctor_text = text(&doctor);
    assert!(
        doctor.status.success() || doctor.status.code() == Some(1),
        "the fixture runbook is doctor-clean (warnings allowed): {doctor_text}"
    );

    // One parked Run per guard kind; each rendering is pinned as a full
    // golden. The contract kinds name their fixed roles with the paths the
    // [roots] table declares, so their derivation is checked too.
    let parked: [(&str, &str, &str); 8] = [
        (
            "run-001",
            "stock",
            r#"pending guard: files_exact root="work" path="release" entries=["proof.txt"]"#,
        ),
        (
            "run-002",
            "scan",
            r#"pending guard: file_contains path="artifacts/status.txt" contains="READY""#,
        ),
        (
            "run-003",
            "probe",
            r#"pending guard: command_exit program="rustc" args=["--version"] expected=0"#,
        ),
        (
            "run-004",
            "receipts",
            r#"pending guard: sensitivity_receipts root="ticket" ticket="t-900.md""#,
        ),
        (
            "run-005",
            "gate",
            r#"pending guard: completion_gate root="ticket" ticket="t-900.md""#,
        ),
        (
            "run-006",
            "intake",
            r#"pending guard: intake_contract goal=".arca/goal" issue=".arca/issue""#,
        ),
        (
            "run-007",
            "records",
            r#"pending guard: record_contract goal=".arca/goal" residual=".arca/residual" ticket=".arca/ticket""#,
        ),
        (
            "run-008",
            "delegate",
            r#"pending guard: join require="all_passed" min=1"#,
        ),
    ];
    for (run_id, state, golden) in parked {
        fixture.seed(run_id, state);
        let rendered = fixture.status_text(run_id);
        assert!(
            rendered.contains(&format!("State: {state}")),
            "the parked Run renders its State: {rendered}"
        );
        assert!(
            rendered.lines().any(|line| line == golden),
            "AOPV-001: the pending {state} guard must render its declared reads as \
             {golden:?}: {rendered}"
        );
    }

    // A freshly minted Run renders the same teaching, so the parked records
    // are not the only carrier.
    let started = fixture.rtm(&["start"]);
    let started_text = text(&started);
    assert!(
        started.status.success() && started_text.contains("run-009"),
        "the fixture machine starts a live Run: {started_text}"
    );
    let live = fixture.status_text("run-009");
    assert!(
        live.lines().any(|line| line == parked[0].2),
        "AOPV-001: a live Run's status names the same declared reads: {live}"
    );

    // The golden pair: rename the declared artifact in the runbook only. No
    // render code changes - the rendering must follow the declaration, and so
    // must the guard's verdict, from the one parsed source.
    let runbook_path = fixture.root.join(".ratmac/ratmac.toml");
    let renamed = fs::read_to_string(&runbook_path)
        .expect("read the runbook")
        .replace("proof.txt", "handoff.txt");
    assert!(
        renamed != fs::read_to_string(&runbook_path).expect("re-read the runbook"),
        "the rename must change the runbook bytes"
    );
    fs::write(&runbook_path, renamed).expect("write the renamed runbook");

    let re_rendered = fixture.status_text("run-001");
    let renamed_golden =
        r#"pending guard: files_exact root="work" path="release" entries=["handoff.txt"]"#;
    assert!(
        re_rendered.lines().any(|line| line == renamed_golden),
        "AOPV-001: renaming the declared artifact renames the rendering: {re_rendered}"
    );
    assert!(
        !re_rendered.contains(parked[0].2),
        "AOPV-001: the old rendering must be gone after the rename: {re_rendered}"
    );
    assert!(
        !re_rendered.contains("proof.txt"),
        "AOPV-001: the retired artifact name must not survive anywhere in the render: {re_rendered}"
    );

    // The same declaration drives the verdict: with the artifact renamed and
    // the disk unchanged, the guard refuses naming the new entry.
    fixture.seed("run-010", "stock");
    let refused = text(&fixture.rtm(&["step", "--run", "run-010"]));
    assert!(
        refused.contains("step refused"),
        "the renamed declaration refuses the unchanged disk: {refused}"
    );
    assert!(
        refused.contains("handoff.txt"),
        "AOPV-001: the verdict follows the same renamed declaration: {refused}"
    );
}

/// AOPV-002 / AOP-002: every status/step outcome class ends in exactly one
/// truthful `next:` line naming a command the engine accepts in that state -
/// each matched against the golden repair table and then executed - or
/// deliberately omits the line where nothing can be stood behind.
#[test]
fn every_outcome_ends_in_one_truthful_next_line() {
    let fixture = Fixture::new("outcomes", RUNBOOK_OUTCOMES);
    let started = fixture.rtm(&["start"]);
    let started_text = text(&started);
    assert!(
        started.status.success() && started_text.contains("run-001"),
        "the outcome machine starts a live Run: {started_text}"
    );

    // 1. Success render on a live Run: the loop's next act is the work plus
    //    the step that judges it. The taught repair is executed at once: the
    //    artifact is still absent, so the step renders its refusal and exits
    //    0 (R-020) - acceptance, not a crash.
    let live = fixture.status_text("run-001");
    assert!(
        live.contains("State: prepare") && live.contains("pending guard: files_exact"),
        "the live Run stands at the guarded State: {live}"
    );
    assert_table_next(
        &live,
        "status on a live Run",
        "run-001",
        "status on a live Run",
    );
    let live_next = the_next_line(&live, "status on a live Run").to_owned();
    assert_repair_accepted(&fixture, &live_next, "status on a live Run");

    // 2. Guard refusal on step: the artifact is absent, the guard refuses,
    //    and the repair is the artifact plus the same step, which R-020 keeps
    //    safe to re-run in that state. The retry is pinned byte-identical.
    let refused = text(&fixture.rtm(&["step", "--run", "run-001"]));
    assert!(
        refused.contains("step refused"),
        "the absent artifact refuses the step: {refused}"
    );
    assert_table_next(
        &refused,
        "step guard refusal",
        "run-001",
        "step guard refusal",
    );
    let refusal_next = the_next_line(&refused, "step guard refusal").to_owned();
    let retried = assert_repair_accepted(&fixture, &refusal_next, "step guard refusal");
    assert_eq!(
        text(&retried),
        refused,
        "R-020: repeating a refused step renders the identical refusal"
    );

    // 3. Success render on step onto a non-terminal State. Its taught repair
    //    is executed by step 4.
    fs::create_dir_all(fixture.root.join("handoff")).expect("create the handoff directory");
    fs::write(fixture.root.join("handoff/proof.txt"), "handoff\n")
        .expect("place the declared artifact");
    let advanced = text(&fixture.rtm(&["step", "--run", "run-001"]));
    assert!(
        advanced.contains("Move through the middle."),
        "the satisfied guard advances the Run: {advanced}"
    );
    assert_table_next(
        &advanced,
        "step success onto a non-terminal State",
        "run-001",
        "step success onto a non-terminal State",
    );
    let advance_next =
        the_next_line(&advanced, "step success onto a non-terminal State").to_owned();

    // 4. Executing that repair steps into the terminal State: nothing
    //    legitimate remains, so the line is deliberately omitted.
    let finished = assert_repair_accepted(
        &fixture,
        &advance_next,
        "step success onto a non-terminal State",
    );
    let finished_text = text(&finished);
    assert!(
        finished_text.contains("The run is complete."),
        "the Run reaches its terminal State: {finished_text}"
    );
    assert_terminal_omits(&finished_text, "step success onto the terminal State");

    // 5. Status on the terminal Run: same deliberate omission.
    let settled = fixture.status_text("run-001");
    assert!(
        settled.contains("Status: passed"),
        "the terminal Run is passed: {settled}"
    );
    assert_terminal_omits(&settled, "status on a terminal Run");

    // 6. Step on the passed Run: the motion this state always refuses is
    //    refused, and the taught repair is the run's own readable record.
    let beyond = text(&fixture.rtm(&["step", "--run", "run-001"]));
    assert!(
        beyond.contains("step refused"),
        "a passed Run refuses further motion: {beyond}"
    );
    assert_table_next(
        &beyond,
        "step on a terminal Run",
        "run-001",
        "step on a terminal Run",
    );
    let beyond_next = the_next_line(&beyond, "step on a terminal Run").to_owned();
    assert!(
        !beyond_next.contains("rtm step --run run-001"),
        "AOPV-002: the terminal repair never names the refused motion: {beyond}"
    );
    let read_back = assert_repair_accepted(&fixture, &beyond_next, "step on a terminal Run");
    assert!(
        text(&read_back).contains("Status: passed"),
        "the terminal repair reads the passed Run: {}",
        text(&read_back)
    );

    // 7. Unknown run id: the refusal carries exactly one `next:` line whose
    //    repair addresses a real Run - never the refused id itself.
    let unknown = fixture.rtm(&["status", "--run", "run-999"]);
    let unknown_text = text(&unknown);
    assert!(
        !unknown.status.success() && unknown_text.contains("run-999"),
        "an unknown run id refuses, naming the input: {unknown_text}"
    );
    assert_table_next(
        &unknown_text,
        "unknown run id",
        "run-001",
        "unknown run id refusal",
    );
    let unknown_next = the_next_line(&unknown_text, "unknown run id refusal").to_owned();
    assert!(
        !unknown_next.contains("run-999"),
        "AOPV-002: the repair never addresses the refused id: {unknown_text}"
    );
    assert_repair_accepted(&fixture, &unknown_next, "unknown run id refusal");

    // 8. Missing --run: the same addressing family on the step route.
    let unaddressed = fixture.rtm(&["step"]);
    let unaddressed_text = text(&unaddressed);
    assert!(
        !unaddressed.status.success(),
        "step without --run refuses: {unaddressed_text}"
    );
    assert_table_next(
        &unaddressed_text,
        "missing --run",
        "run-001",
        "missing --run refusal",
    );
    let unaddressed_next = the_next_line(&unaddressed_text, "missing --run refusal").to_owned();
    assert_repair_accepted(&fixture, &unaddressed_next, "missing --run refusal");

    // 9. Malformed Run Record: a hard error (R-027) whose repair never
    //    addresses the corrupt Run - no engine verb repairs an Engine-owned
    //    record, so the legitimate act is minting a fresh Run, executed here.
    let second = fixture.rtm(&["start"]);
    assert!(
        second.status.success() && text(&second).contains("run-002"),
        "a second Run starts for the corrupt-record class: {}",
        text(&second)
    );
    fs::write(
        fixture.root.join(".ratmac/runs/run-002/run.toml"),
        "state = \"prepare\"\n",
    )
    .expect("truncate the second Run Record");
    let corrupt = fixture.rtm(&["status", "--run", "run-002"]);
    let corrupt_text = text(&corrupt);
    assert!(
        !corrupt.status.success() && corrupt_text.contains("run.toml"),
        "a malformed Run Record is a hard error naming the file: {corrupt_text}"
    );
    assert_table_next(
        &corrupt_text,
        "malformed Run Record",
        "run-001",
        "malformed Run Record refusal",
    );
    let corrupt_next = the_next_line(&corrupt_text, "malformed Run Record refusal").to_owned();
    assert!(
        !corrupt_next.contains("run-002"),
        "AOPV-002: the corrupt Run's repair never addresses it: {corrupt_text}"
    );
    let minted = assert_repair_accepted(&fixture, &corrupt_next, "malformed Run Record refusal");
    assert!(
        text(&minted).contains("run-003"),
        "the corrupt record's repair mints a fresh Run: {}",
        text(&minted)
    );

    // 10. GPH-001, a Run whose record predates the rendering change: the
    //     aged record still receives the full teaching, and its taught step
    //     is accepted - the handoff artifact is on disk, so the repair is
    //     the designed advance.
    fixture.seed("run-004", "prepare");
    let aged = fixture.status_text("run-004");
    assert!(
        aged.contains("pending guard: files_exact"),
        "the aged record renders its pending guard: {aged}"
    );
    assert_table_next(
        &aged,
        "status on a pre-change record",
        "run-004",
        "status on a pre-change record",
    );
    let aged_next = the_next_line(&aged, "status on a pre-change record").to_owned();
    let aged_repair = assert_repair_accepted(&fixture, &aged_next, "status on a pre-change record");
    assert!(
        text(&aged_repair).contains("Move through the middle."),
        "the aged record's taught step advances the Run: {}",
        text(&aged_repair)
    );
}
