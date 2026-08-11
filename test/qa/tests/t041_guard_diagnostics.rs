//! t-040 / ETB-002: bounded refusal diagnostics for command guards.
//!
//! PT-040-01 `diagnostic_is_captured`
//! PT-040-02 `diagnostic_is_bounded`
//! PT-040-03 `silent_guard_states_no_diagnostic`
//! HT-040-01 `non_utf8_diagnostic_renders_lossily`
//! HT-040-02 `repeated_refusal_is_identical`
//! HT-040-03 `guard_death_reports_partial_and_releases_lock`
//!
//! A refused `command_exit` guard must name the artifact to repair: program
//! identity, expected-vs-observed exit facts, and a bounded capture of the
//! child's stderr with a deterministic bound, an explicit truncation marker,
//! and fixed wording when the child says nothing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The documented bound from `.arca/goal/design.md` (ETB-002).
const DIAGNOSTIC_BOUND: usize = 4096;
const TRUNCATION_MARKER: &str = "…truncated";
const NO_DIAGNOSTIC: &str = "no diagnostic emitted";

struct Fixture {
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A project whose only transition is guarded by the probe in `mode`.
fn fixture(label: &str, mode: &str, extra: Option<&str>) -> Fixture {
    let root = std::env::temp_dir().join(format!(
        "ratmac-t041-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".ratmac")).expect("create fixture project");

    let probe = env!("CARGO_BIN_EXE_guard-probe").replace('\\', "\\\\");
    let args = match extra {
        Some(value) => format!("\"{mode}\", \"{value}\""),
        None => format!("\"{mode}\""),
    };
    let class = format!(
        "[states.prepare]\n\
         prompt = \"Prepare the run.\"\n\
         guards = [{{ kind = \"command_exit\", program = \"{probe}\", args = [{args}], expected = 0 }}]\n\
         \n\
         [states.done]\n\
         prompt = \"Complete the run.\"\n\
         \n\
         [[transitions]]\n\
         from = \"prepare\"\n\
         to = \"done\"\n"
    );
    fs::write(root.join(".ratmac/ratmac.toml"), class).expect("write machine class");
    Fixture { root }
}

fn rtm(fixture: &Fixture, args: &[&str]) -> Output {
    Command::new(ratmac_qa::engine_bin!())
        .args(args)
        .current_dir(&fixture.root)
        .output()
        .expect("invoke rtm")
}

/// FDC-004: the started run's id, read off the plural roster.
fn run_id(fixture: &Fixture) -> String {
    fs::read_dir(fixture.root.join(".ratmac/runs"))
        .expect("list the runs roster")
        .map(|entry| entry.expect("roster entry is readable"))
        .find(|entry| entry.path().is_dir())
        .expect("the started run appears on the roster")
        .file_name()
        .to_string_lossy()
        .into_owned()
}

fn refusal_text(fixture: &Fixture) -> String {
    assert!(
        rtm(fixture, &["start"]).status.success(),
        "start must succeed"
    );
    let id = run_id(fixture);
    let step = rtm(fixture, &["step", "--run", &id]);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&step.stdout),
        String::from_utf8_lossy(&step.stderr)
    );
    assert!(
        text.contains("step refused"),
        "the guard must refuse the step: {text}"
    );
    text
}

/// PT-040-01: the refusal carries the guard's own diagnostic verbatim.
#[test]
fn diagnostic_is_captured() {
    let fixture = fixture("captured", "say", Some("blocking artifact: docs/report.md"));
    let text = refusal_text(&fixture);
    assert!(
        text.contains("blocking artifact: docs/report.md"),
        "the refusal must name the artifact to repair: {text}"
    );
    assert!(
        text.contains("guard-probe"),
        "the refusal must identify the program: {text}"
    );
    assert!(
        text.contains("exit 1") && text.contains("exit 0"),
        "the refusal must state observed and expected exit facts: {text}"
    );
}

/// PT-040-02: an oversize diagnostic is bounded and marked.
#[test]
fn diagnostic_is_bounded() {
    let fixture = fixture("bounded", "flood", Some("65536"));
    let text = refusal_text(&fixture);
    assert!(
        text.contains(TRUNCATION_MARKER),
        "an oversize diagnostic must carry the truncation marker: {text}"
    );
    assert!(
        text.len() < DIAGNOSTIC_BOUND + 512,
        "the refusal must stay within the bound plus fixed framing: {} bytes",
        text.len()
    );
    assert!(
        text.contains("TAIL-MARKER-END"),
        "the retained window must be the last bytes, where the failure ends: {text}"
    );
}

/// PT-040-03: silence is stated, not omitted.
#[test]
fn silent_guard_states_no_diagnostic() {
    let fixture = fixture("silent", "silent", None);
    let text = refusal_text(&fixture);
    assert!(
        text.contains(NO_DIAGNOSTIC),
        "a silent guard must yield the documented wording: {text}"
    );
}

/// HT-040-01 (Input/Routing): invalid UTF-8 and NUL bytes must not panic or
/// corrupt the refusal.
#[test]
fn non_utf8_diagnostic_renders_lossily() {
    let fixture = fixture("binary", "binary", None);
    let text = refusal_text(&fixture);
    assert!(
        text.contains('\u{fffd}') || text.contains("no diagnostic emitted"),
        "invalid bytes must be rendered lossily, not dropped: {text}"
    );
    assert!(
        text.len() < DIAGNOSTIC_BOUND + 512,
        "the bound still applies to binary output"
    );
}

/// HT-040-02 (Regression): a refused step is idempotent in output and state.
#[test]
fn repeated_refusal_is_identical() {
    let fixture = fixture("idempotent", "say", Some("blocking artifact: a.txt"));
    assert!(
        rtm(&fixture, &["start"]).status.success(),
        "start must succeed"
    );
    let id = run_id(&fixture);
    let state_path = fixture.root.join(".ratmac/runs").join(&id).join("run.toml");
    let mut outputs = Vec::new();
    let mut states = Vec::new();
    for _ in 0..5 {
        let step = rtm(&fixture, &["step", "--run", &id]);
        outputs.push(String::from_utf8_lossy(&step.stdout).to_string());
        states.push(fs::read(&state_path).expect("read state"));
    }
    assert!(
        outputs.windows(2).all(|pair| pair[0] == pair[1]),
        "refusal output must be byte-identical across repeats: {outputs:?}"
    );
    assert!(
        states.windows(2).all(|pair| pair[0] == pair[1]),
        "refused steps must not change state"
    );
}

/// HT-040-03 (Durability/Recovery): a guard that dies mid-run still yields its
/// partial diagnostic, releases the lock, and writes no state.
#[test]
fn guard_death_reports_partial_and_releases_lock() {
    let fixture = fixture("partial", "partial", None);
    let text = refusal_text(&fixture);
    assert!(
        text.contains("partial diagnostic before death"),
        "the partial diagnostic must be reported: {text}"
    );
    assert!(
        text.contains("exit 3"),
        "the observed exit code must be reported: {text}"
    );
    let id = run_id(&fixture);
    assert!(
        !lock_path(&fixture.root, &id).exists(),
        "ENS-005 Run lock must be released after a refusal"
    );
    assert!(
        !ratmac::lock::root_path(&fixture.root.join(".ratmac")).exists(),
        "ENS-005 refused motion leaves no root lock behind"
    );
    // ENS-005: a second invocation must still work, proving nothing is stuck.
    let again = rtm(&fixture, &["status", "--run", &id]);
    assert!(again.status.success(), "status after refusal must succeed");
}

fn lock_path(root: &Path, run_id: &str) -> PathBuf {
    ratmac::lock::run_path(&root.join(".ratmac"), run_id)
}
