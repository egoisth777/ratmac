//! t-075 / ENS-007: Scheduler-owned Engine transition history.
//!
//! ENSV-008 `scheduler_writes_only_ratmac_transition_log`
//!
//! The test drives the compiled public `rtm` binary in one real temporary Git
//! repository. Two independently addressed Runs exercise the ordinary,
//! refused, held, failed-and-retried, and abandoned paths while the pre-seeded
//! contributor history remains an exact byte sequence.

use ratmac_qa::tempgit::TempRepo;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::SystemTime;

const TICKET: &str = "t-900";
const BLOCKER: &str = ".arca/issue/i-777-blocker";
const RUNBOOK: &str = r#"
[roots]
ticket = ".arca/ticket"

[states.intake]
prompt = "Integrate the issue."

[states.build]
prompt = "Build the ticket."
guards = [{ kind = "file_contains", path = "artifacts/ready.txt", contains = "ready" }]

[states.review]
prompt = "Review the completed ticket."

[[transitions]]
from = "intake"
to = "build"

[[transitions]]
from = "build"
to = "intake"
blocked-route = true

[[transitions]]
from = "build"
to = "review"
"#;
const ENGINE_LOG_PREFIX: &[u8] =
    include_bytes!("../../fixtures/r026-transition-log-existing/.ratmac/log.md");
const HUMAN_HISTORY: &[u8] = b"# Contributor history\n\n- 2026-08-01: established the project.\n- 2026-08-02: reviewed the plan.\n- 2026-08-03: approved the work.\n";

struct Fixture {
    repo: TempRepo,
}

impl Fixture {
    fn new() -> Self {
        let repo = TempRepo::new("t075-engine-transition-log");
        let root = repo.root();

        repo.write(
            ".gitignore",
            &fs::read_to_string(repo_root().join(".gitignore"))
                .expect("read repository ignore policy"),
        );
        repo.write("src/lib.rs", "pub fn fixture_marker() {}\n");
        repo.write(".ratmac/ratmac.toml", RUNBOOK);
        fs::write(root.join(".ratmac/log.md"), ENGINE_LOG_PREFIX)
            .expect("seed pre-existing Engine history bytes");
        repo.write(
            ".arca/log.md",
            std::str::from_utf8(HUMAN_HISTORY).expect("human fixture history is UTF-8"),
        );
        repo.write(".arca/goal/spec.md", "# Fixture goal\n");
        repo.write(
            ".arca/ticket/t-900.md",
            "---\nticket-id: \"t-900\"\nresidual-ids:\n  - \"res-900\"\n\
             planned-test-refs:\n  - \"ENSV-008\"\nstatus: \"executing\"\n---\n\n\
             # Ticket: t-900\n",
        );
        repo.write(
            ".arca/residual/res-900.md",
            "# Residual Record\n\n```yaml\nresidual-id: \"res-900\"\nstatus: \"missing\"\n```\n",
        );
        for name in [
            "index.md",
            "spec.md",
            "design.md",
            "test-plan.md",
            "ubi-lang.md",
        ] {
            repo.write(
                &format!("{BLOCKER}/{name}"),
                &format!("# {name}\n\nFixture blocker record.\n"),
            );
        }
        repo.commit_all("fixture base");

        Self { repo }
    }

    fn root(&self) -> &Path {
        self.repo.root()
    }

    fn rtm(&self, args: &[&str]) -> Output {
        rtm_at(self.root(), args)
    }

    fn rtm_with_log_append_failure(&self, args: &[&str]) -> Output {
        let mut command = rtm_command(self.root(), args);
        command.env("RATMAC_TEST_LOG_APPEND_FAIL", "after-state-write");
        command
            .output()
            .expect("invoke rtm with no-write transition-log append fault")
    }

    fn rtm_with_partial_log_append_failure(&self, args: &[&str]) -> Output {
        let mut command = rtm_command(self.root(), args);
        command.env("RATMAC_TEST_LOG_APPEND_FAIL", "after-partial-write");
        command
            .output()
            .expect("invoke rtm with partial transition-log append fault")
    }

    fn engine_log(&self) -> Vec<u8> {
        fs::read(self.root().join(".ratmac/log.md")).expect("read Engine transition log")
    }

    fn record_path(&self, run_id: &str) -> PathBuf {
        self.root()
            .join(".ratmac/runs")
            .join(run_id)
            .join("run.toml")
    }

    fn state_bytes(&self, run_id: &str) -> Vec<u8> {
        fs::read(self.record_path(run_id)).expect("read addressed Run State File")
    }

    fn state(&self, run_id: &str) -> String {
        fs::read_to_string(self.record_path(run_id))
            .expect("read the addressed Run Record as UTF-8")
            .lines()
            .find_map(|line| line.trim().strip_prefix("state = "))
            .map(|value| value.trim().trim_matches('"').to_owned())
            .expect("addressed Run State File records a phase")
    }

    fn human_history_snapshot(&self) -> HumanHistorySnapshot {
        let path = self.root().join(".arca/log.md");
        HumanHistorySnapshot {
            bytes: fs::read(&path).expect("read seeded contributor history"),
            modified: fs::metadata(path)
                .expect("read contributor history metadata")
                .modified()
                .expect("read contributor history modification time"),
        }
    }
}

struct HumanHistorySnapshot {
    bytes: Vec<u8>,
    modified: SystemTime,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rtm_command(root: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rtm"));
    command
        .args(args)
        .current_dir(root)
        .env_remove("RATMAC_TEST_LOG_APPEND_FAIL")
        .env_remove("RATMAC_TEST_STEP_FAULT");
    command
}

fn rtm_at(root: &Path, args: &[&str]) -> Output {
    rtm_command(root, args)
        .output()
        .expect("invoke compiled rtm binary")
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn started_run_id(output: &Output) -> String {
    let text = combined(output);
    text.split("started run ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or_else(|| panic!("start output must name the minted Run: {text}"))
        .to_owned()
}

fn assert_human_history_unchanged(fixture: &Fixture, before: &HumanHistorySnapshot, event: &str) {
    let path = fixture.root().join(".arca/log.md");
    let after = fs::read(&path).expect("read contributor history after Engine action");
    assert_eq!(
        &after, &before.bytes,
        "{event} must leave the contributor history byte-identical"
    );
    let modified = fs::metadata(path)
        .expect("read contributor history metadata after Engine action")
        .modified()
        .expect("read contributor history modification time after Engine action");
    assert_eq!(
        modified, before.modified,
        "{event} must not modify the contributor history timestamp"
    );
}

fn assert_engine_prefix(log: &[u8], event: &str) {
    assert!(
        log.starts_with(ENGINE_LOG_PREFIX),
        "{event} must preserve all pre-existing Engine transition-log bytes as its exact prefix"
    );
}

fn records_after_prefix(log: &[u8]) -> Vec<String> {
    assert_engine_prefix(log, "Engine transition-log inspection");
    std::str::from_utf8(&log[ENGINE_LOG_PREFIX.len()..])
        .expect("new Engine transition-log records are UTF-8")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

/// ENSV-008: ordinary, held, and abandoned Engine events produce exactly one
/// ordered record in the Engine log; refused and no-write-failed motion produce
/// none; a partial-write failure preserves its incomplete record; and the
/// contributor log never changes.
#[test]
fn scheduler_writes_only_ratmac_transition_log() {
    let fixture = Fixture::new();
    let human_before = fixture.human_history_snapshot();

    let first_start = fixture.rtm(&["start"]);
    assert!(
        first_start.status.success(),
        "first addressed Run starts: {}",
        combined(&first_start)
    );
    let first_run = started_run_id(&first_start);
    assert_human_history_unchanged(&fixture, &human_before, "starting the first Run");

    let second_start = fixture.rtm(&["start"]);
    assert!(
        second_start.status.success(),
        "second addressed Run starts: {}",
        combined(&second_start)
    );
    let second_run = started_run_id(&second_start);
    assert_ne!(
        first_run, second_run,
        "two starts must mint independently addressed Runs"
    );
    assert_human_history_unchanged(&fixture, &human_before, "starting the second Run");
    let after_starts = fixture.engine_log();
    assert_eq!(
        after_starts.as_slice(),
        ENGINE_LOG_PREFIX,
        "starting Runs must not create transition records"
    );

    let first_step = fixture.rtm(&["step", "--run", &first_run]);
    assert!(
        first_step.status.success(),
        "the first addressed Run advances normally: {}",
        combined(&first_step)
    );
    assert_eq!(
        fixture.state(&first_run),
        "build",
        "the first addressed Run reached its guarded build State"
    );
    assert_human_history_unchanged(&fixture, &human_before, "the ordinary first-Run step");
    let after_first_step = fixture.engine_log();
    assert_engine_prefix(&after_first_step, "the ordinary first-Run step");
    assert_eq!(
        records_after_prefix(&after_first_step),
        vec![format!("- Transition: intake -> build; Run {first_run}")],
        "one ordinary step must append one complete transition record"
    );

    let first_state_before_refusal = fixture.state_bytes(&first_run);
    let log_before_refusal = fixture.engine_log();
    let refused = fixture.rtm(&["step", "--run", &first_run]);
    let refusal_text = combined(&refused);
    assert!(
        refusal_text.contains("step refused"),
        "the missing guarded artifact is a step refusal: {refusal_text}"
    );
    assert!(
        refusal_text.contains("artifacts/ready.txt"),
        "the refusal names the missing guarded artifact: {refusal_text}"
    );
    assert_eq!(
        fixture.state_bytes(&first_run),
        first_state_before_refusal,
        "a refused step must preserve the addressed Run State File"
    );
    assert_eq!(
        fixture.engine_log(),
        log_before_refusal,
        "a refused step must append no Engine transition record"
    );
    assert_human_history_unchanged(&fixture, &human_before, "the refused first-Run step");

    let held = fixture.rtm(&[
        "hold",
        TICKET,
        "--blocker",
        BLOCKER,
        "--confirm",
        "hold t-900",
        "--run",
        &first_run,
    ]);
    assert!(
        held.status.success(),
        "the confirmed hold routes the first addressed Run: {}",
        combined(&held)
    );
    assert_eq!(
        fixture.state(&first_run),
        "intake",
        "the hold follows the declared blocked route"
    );
    assert_human_history_unchanged(&fixture, &human_before, "the first-Run hold");
    let after_hold = fixture.engine_log();
    assert_engine_prefix(&after_hold, "the first-Run hold");
    assert!(
        after_hold[after_first_step.len()..].starts_with(b"\n- Hold:"),
        "the shared writer prefixes the hold record with its own separator"
    );
    let records_after_hold = records_after_prefix(&after_hold);
    assert_eq!(
        records_after_hold.len(),
        2,
        "the hold must append exactly one record after the ordinary step"
    );
    assert!(
        records_after_hold[1].starts_with("- Hold:"),
        "the second Engine record is a well-formed hold: {:?}",
        records_after_hold[1]
    );
    assert!(
        records_after_hold[1].contains(TICKET)
            && records_after_hold[1].contains(BLOCKER)
            && records_after_hold[1].contains(&format!("Run {first_run}"))
            && records_after_hold[1].contains("build -> intake"),
        "the hold record names its ticket, blocker, Run, and route: {:?}",
        records_after_hold[1]
    );
    let first_state_after_hold = fixture.state_bytes(&first_run);

    let second_state_before_fault = fixture.state_bytes(&second_run);
    let log_before_fault = fixture.engine_log();
    // QA fault seam contract: `RATMAC_TEST_LOG_APPEND_FAIL=after-state-write`
    // makes the Scheduler's transition append fail after State commit and
    // before any record is written, so both durable artifacts must roll back.
    let injected = fixture.rtm_with_log_append_failure(&["step", "--run", &second_run]);
    let injected_text = combined(&injected);
    assert!(
        !injected.status.success(),
        "RATMAC_TEST_LOG_APPEND_FAIL=after-state-write must refuse the Scheduler append after State File commit: {injected_text}"
    );
    let injected_lowercase = injected_text.to_ascii_lowercase();
    assert!(
        injected_lowercase.contains("append") && injected_lowercase.contains("log"),
        "the injected refusal must name the log append boundary: {injected_text}"
    );
    assert_eq!(
        fixture.state_bytes(&second_run),
        second_state_before_fault,
        "a failed append after State commit must restore the exact prior State File bytes"
    );
    assert_eq!(
        fixture.engine_log(),
        log_before_fault,
        "a failed append after State commit must restore the exact prior Engine-log bytes"
    );
    assert_human_history_unchanged(&fixture, &human_before, "the injected append failure");

    let retry = fixture.rtm(&["step", "--run", &second_run]);
    assert!(
        retry.status.success(),
        "repairing the append destination permits one retry: {}",
        combined(&retry)
    );
    assert_eq!(
        fixture.state(&second_run),
        "build",
        "the retry advances the second addressed Run exactly once"
    );
    assert_eq!(
        fixture.state_bytes(&first_run),
        first_state_after_hold,
        "the second Run's retry must not corrupt the first Run's State File"
    );
    assert_human_history_unchanged(&fixture, &human_before, "the second-Run retry");
    let after_retry = fixture.engine_log();
    assert_engine_prefix(&after_retry, "the second-Run retry");
    let records_after_retry = records_after_prefix(&after_retry);
    assert_eq!(
        records_after_retry.len(),
        3,
        "the repaired retry must append one complete record, not a duplicate or partial record"
    );
    assert_eq!(
        records_after_retry[2],
        format!("- Transition: intake -> build; Run {second_run}"),
        "the retry contributes the second addressed Run's complete transition record"
    );

    let confirmation = format!("abandon {second_run}");
    let abandoned = fixture.rtm(&["abandon", "--confirm", &confirmation, "--run", &second_run]);
    assert!(
        abandoned.status.success(),
        "the confirmed abandonment succeeds: {}",
        combined(&abandoned)
    );
    assert!(
        !fixture.record_path(&second_run).exists(),
        "abandonment retires only the second Run's admission State File"
    );
    assert_eq!(
        fixture.state_bytes(&first_run),
        first_state_after_hold,
        "abandoning the second Run must not corrupt the first Run's State File"
    );
    assert_human_history_unchanged(&fixture, &human_before, "the second-Run abandonment");

    let final_log = fixture.engine_log();
    assert_engine_prefix(&final_log, "the complete two-Run sequence");
    assert!(
        final_log[after_retry.len()..]
            .starts_with(format!("\n- Abandoned: Run {second_run}").as_bytes()),
        "the shared writer prefixes the abandonment record with its own separator"
    );
    let final_records = records_after_prefix(&final_log);
    assert_eq!(
        final_records.len(),
        4,
        "the ordinary step, hold, retry, and abandonment each append exactly one Engine record"
    );
    assert_eq!(
        final_records[0],
        format!("- Transition: intake -> build; Run {first_run}"),
        "the first record is the first Run's ordinary transition"
    );
    assert!(
        final_records[1].starts_with("- Hold:") && final_records[1].contains("build -> intake"),
        "the second record is the first Run's hold: {:?}",
        final_records[1]
    );
    assert_eq!(
        final_records[2],
        format!("- Transition: intake -> build; Run {second_run}"),
        "the third record is the second Run's repaired retry"
    );
    assert!(
        final_records[3].starts_with(&format!("- Abandoned: Run {second_run}"))
            && final_records[3].contains("state build"),
        "the fourth record is the second Run's terminal abandonment: {:?}",
        final_records[3]
    );
    let third_start = fixture.rtm(&["start"]);
    assert!(
        third_start.status.success(),
        "the third addressed Run starts for partial-append recovery: {}",
        combined(&third_start)
    );
    let third_run = started_run_id(&third_start);
    assert_human_history_unchanged(
        &fixture,
        &human_before,
        "starting the third Run for partial-append recovery",
    );
    let third_state_before_partial = fixture.state_bytes(&third_run);
    let log_before_partial = fixture.engine_log();
    let partial_entry = format!("\n- Transition: intake -> build; Run {third_run}\n");
    let expected_fragment = &partial_entry.as_bytes()[..partial_entry.len() / 2];
    assert!(
        !expected_fragment.is_empty() && expected_fragment.len() < partial_entry.len(),
        "the partial-write seam fixture uses a genuine proper record prefix"
    );
    let partial = fixture.rtm_with_partial_log_append_failure(&["step", "--run", &third_run]);
    let partial_text = combined(&partial);
    assert!(
        !partial.status.success(),
        "RATMAC_TEST_LOG_APPEND_FAIL=after-partial-write must refuse after a genuine fragment: {partial_text}"
    );
    let partial_log_path = fixture.root().join(".ratmac/log.md");
    let normalized_log_path = partial_log_path.to_string_lossy().replace('\\', "/");
    assert!(
        partial_text
            .replace('\\', "/")
            .contains(&normalized_log_path),
        "the partial-append refusal must name its transition-log path: {partial_text}"
    );
    let partial_lowercase = partial_text.to_ascii_lowercase();
    assert!(
        partial_lowercase.contains("incomplete record")
            && partial_lowercase.contains("human must resolve"),
        "the partial-append refusal must require human resolution: {partial_text}"
    );
    assert_eq!(
        fixture.state_bytes(&third_run),
        third_state_before_partial,
        "a partial append must restore the exact prior addressed Run State File bytes"
    );
    assert_eq!(
        fixture.engine_log(),
        [log_before_partial.as_slice(), expected_fragment].concat(),
        "a partial append must remain as the exact proper record prefix"
    );
    assert_human_history_unchanged(
        &fixture,
        &human_before,
        "the partial transition-log append failure",
    );
}

/// A later writer must not change how the failed writer classifies its own
/// short append: that outcome comes from the write count, not a shared-log
/// snapshot.
#[test]
fn partial_append_recovery_is_local_to_the_writer() {
    let fixture = Fixture::new();
    let human_before = fixture.human_history_snapshot();

    let first_start = fixture.rtm(&["start"]);
    assert!(
        first_start.status.success(),
        "the first Run starts for local partial-append recovery: {}",
        combined(&first_start)
    );
    let first_run = started_run_id(&first_start);
    let first_state_before_partial = fixture.state_bytes(&first_run);
    let log_before_partial = fixture.engine_log();
    let partial_entry = format!("\n- Transition: intake -> build; Run {first_run}\n");
    let expected_fragment = &partial_entry.as_bytes()[..partial_entry.len() / 2];

    let partial = fixture.rtm_with_partial_log_append_failure(&["step", "--run", &first_run]);
    let partial_text = combined(&partial);
    assert!(
        !partial.status.success(),
        "the first Run's partial append must refuse: {partial_text}"
    );
    let partial_log_path = fixture.root().join(".ratmac/log.md");
    let normalized_log_path = partial_log_path.to_string_lossy().replace('\\', "/");
    assert!(
        partial_text
            .replace('\\', "/")
            .contains(&normalized_log_path),
        "the first Run's partial refusal names its transition-log path: {partial_text}"
    );
    assert!(
        partial_text
            .to_ascii_lowercase()
            .contains("human must resolve"),
        "the first Run's partial refusal requires human resolution: {partial_text}"
    );
    assert_eq!(
        fixture.state_bytes(&first_run),
        first_state_before_partial,
        "the first Run restores its exact prior State File before another writer acts"
    );

    let second_start = fixture.rtm(&["start"]);
    assert!(
        second_start.status.success(),
        "the second Run starts after the first Run's retained fragment: {}",
        combined(&second_start)
    );
    let second_run = started_run_id(&second_start);
    assert_ne!(
        first_run, second_run,
        "the later complete record belongs to a different addressed Run"
    );
    let second_step = fixture.rtm(&["step", "--run", &second_run]);
    assert!(
        second_step.status.success(),
        "the second Run appends a complete record after the fragment: {}",
        combined(&second_step)
    );
    assert_eq!(
        fixture.state(&second_run),
        "build",
        "the second Run advances through its ordinary transition"
    );
    assert_eq!(
        fixture.state_bytes(&first_run),
        first_state_before_partial,
        "the later writer cannot change the first Run's restored State File"
    );

    let complete_second_entry = format!("\n- Transition: intake -> build; Run {second_run}\n");
    let final_log = fixture.engine_log();
    assert_eq!(
        final_log,
        [
            log_before_partial.as_slice(),
            expected_fragment,
            complete_second_entry.as_bytes(),
        ]
        .concat(),
        "the first Run's exact fragment remains while the second Run's complete record remains intact"
    );
    assert_human_history_unchanged(
        &fixture,
        &human_before,
        "a later complete append after a partial transition record",
    );
}

/// Hold and abandonment use the same funnel-owned partial-record diagnostic
/// as step; neither caller may discard the local short-write outcome.
#[test]
fn hold_and_abandon_surface_the_shared_fragment_refusal() {
    let held = Fixture::new();
    let held_human_before = held.human_history_snapshot();
    let held_start = held.rtm(&["start"]);
    assert!(
        held_start.status.success(),
        "the held Run starts: {}",
        combined(&held_start)
    );
    let held_run = started_run_id(&held_start);
    let held_step = held.rtm(&["step", "--run", &held_run]);
    assert!(
        held_step.status.success(),
        "the held Run reaches the blocked route source State: {}",
        combined(&held_step)
    );
    let held_log_before = held.engine_log();
    let held_partial = held.rtm_with_partial_log_append_failure(&[
        "hold",
        TICKET,
        "--blocker",
        BLOCKER,
        "--confirm",
        "hold t-900",
        "--run",
        &held_run,
    ]);
    let held_text = combined(&held_partial);
    assert!(
        !held_partial.status.success(),
        "a partial hold history append must refuse: {held_text}"
    );
    let held_log_path = held.root().join(".ratmac/log.md");
    assert!(
        held_text
            .replace('\\', "/")
            .contains(&held_log_path.to_string_lossy().replace('\\', "/"))
            && held_text.to_ascii_lowercase().contains("incomplete record")
            && held_text
                .to_ascii_lowercase()
                .contains("human must resolve"),
        "the hold refusal carries the funnel-owned fragment diagnostic: {held_text}"
    );
    assert_eq!(
        held.state(&held_run),
        "intake",
        "the hold's already-committed route remains visible after its fragment"
    );
    let held_log_after = held.engine_log();
    assert!(
        held_log_after.starts_with(&held_log_before)
            && held_log_after.len() > held_log_before.len(),
        "the hold fragment remains append-only in the Engine log"
    );
    assert_human_history_unchanged(
        &held,
        &held_human_before,
        "the partial hold transition-log append",
    );

    let abandoned = Fixture::new();
    let abandoned_human_before = abandoned.human_history_snapshot();
    let abandoned_start = abandoned.rtm(&["start"]);
    assert!(
        abandoned_start.status.success(),
        "the abandoned Run starts: {}",
        combined(&abandoned_start)
    );
    let abandoned_run = started_run_id(&abandoned_start);
    let abandoned_state_before = abandoned.state_bytes(&abandoned_run);
    let abandoned_log_before = abandoned.engine_log();
    let confirmation = format!("abandon {abandoned_run}");
    let abandoned_partial = abandoned.rtm_with_partial_log_append_failure(&[
        "abandon",
        "--confirm",
        &confirmation,
        "--run",
        &abandoned_run,
    ]);
    let abandoned_text = combined(&abandoned_partial);
    assert!(
        !abandoned_partial.status.success(),
        "a partial abandonment history append must refuse: {abandoned_text}"
    );
    let abandoned_log_path = abandoned.root().join(".ratmac/log.md");
    assert!(
        abandoned_text
            .replace('\\', "/")
            .contains(&abandoned_log_path.to_string_lossy().replace('\\', "/"))
            && abandoned_text
                .to_ascii_lowercase()
                .contains("incomplete record")
            && abandoned_text
                .to_ascii_lowercase()
                .contains("human must resolve"),
        "the abandonment refusal carries the funnel-owned fragment diagnostic: {abandoned_text}"
    );
    assert_eq!(
        abandoned.state_bytes(&abandoned_run),
        abandoned_state_before,
        "a fragment refuses before abandonment retires the addressed State File"
    );
    let abandoned_log_after = abandoned.engine_log();
    assert!(
        abandoned_log_after.starts_with(&abandoned_log_before)
            && abandoned_log_after.len() > abandoned_log_before.len(),
        "the abandonment fragment remains append-only in the Engine log"
    );
    assert_human_history_unchanged(
        &abandoned,
        &abandoned_human_before,
        "the partial abandonment transition-log append",
    );
}
