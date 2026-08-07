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
[phases.intake]
prompt = "Integrate the issue."

[phases.build]
prompt = "Build the ticket."
guards = [{ kind = "file_contains", path = "artifacts/ready.txt", contains = "ready" }]

[phases.review]
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
            .expect("invoke rtm with transition-log append fault")
    }

    fn engine_log(&self) -> Vec<u8> {
        fs::read(self.root().join(".ratmac/log.md")).expect("read Engine transition log")
    }

    fn state_path(&self, run_id: &str) -> PathBuf {
        self.root()
            .join(".ratmac/runs")
            .join(run_id)
            .join("state.toml")
    }

    fn state_bytes(&self, run_id: &str) -> Vec<u8> {
        fs::read(self.state_path(run_id)).expect("read addressed Run State File")
    }

    fn phase(&self, run_id: &str) -> String {
        fs::read_to_string(self.state_path(run_id))
            .expect("read addressed Run State File as UTF-8")
            .lines()
            .find_map(|line| line.trim().strip_prefix("phase = "))
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
/// ordered record in the Engine log; a refused or injected-failed motion
/// produces none; and the contributor log never changes.
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
        fixture.phase(&first_run),
        "build",
        "the first addressed Run reached its guarded build Phase"
    );
    assert_human_history_unchanged(&fixture, &human_before, "the ordinary first-Run step");
    let after_first_step = fixture.engine_log();
    assert_engine_prefix(&after_first_step, "the ordinary first-Run step");
    assert_eq!(
        records_after_prefix(&after_first_step),
        vec!["- Transition: intake -> build".to_owned()],
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
        fixture.phase(&first_run),
        "intake",
        "the hold follows the declared blocked route"
    );
    assert_human_history_unchanged(&fixture, &human_before, "the first-Run hold");
    let after_hold = fixture.engine_log();
    assert_engine_prefix(&after_hold, "the first-Run hold");
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
            && records_after_hold[1].contains("build -> intake"),
        "the hold record names its ticket, blocker, and route: {:?}",
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
        fixture.phase(&second_run),
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
        records_after_retry[2], "- Transition: intake -> build",
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
        !fixture.state_path(&second_run).exists(),
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
    let final_records = records_after_prefix(&final_log);
    assert_eq!(
        final_records.len(),
        4,
        "the ordinary step, hold, retry, and abandonment each append exactly one Engine record"
    );
    assert_eq!(
        final_records[0], "- Transition: intake -> build",
        "the first record is the first Run's ordinary transition"
    );
    assert!(
        final_records[1].starts_with("- Hold:") && final_records[1].contains("build -> intake"),
        "the second record is the first Run's hold: {:?}",
        final_records[1]
    );
    assert_eq!(
        final_records[2], "- Transition: intake -> build",
        "the third record is the second Run's repaired retry"
    );
    assert!(
        final_records[3].starts_with(&format!("- Abandoned: Run {second_run}"))
            && final_records[3].contains("phase build"),
        "the fourth record is the second Run's terminal abandonment: {:?}",
        final_records[3]
    );
}
