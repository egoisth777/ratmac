//! t-041 / ETB-001: pinned gate execution boundary.
//!
//! PT-041-01 `pin_is_recorded`
//! PT-041-02 `tamper_refuses_and_restore_proceeds`
//! PT-041-03 `build_at_evaluation_rejected_probe_exempt`
//! HT-041-01 `symlink_and_directory_programs_refuse`
//! HT-041-02 `recorded_pin_survives_a_crashed_run`
//! HT-041-03 `pin_refusal_carries_identity_and_diagnostic_framing`
//!
//! Guard evaluation runs only pinned or explicitly exempt programs: the
//! resolved path and SHA-256 of every project-derived gate artifact are
//! recorded in Run evidence no later than first guard use and re-verified at
//! every evaluation.

use ratmac_qa::snapshot::sha256_file;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Fixture {
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    /// A project whose single transition is guarded by `guard_table`.
    fn new(label: &str, guard_table: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t042-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".arca")).expect("create fixture project");
        let class = format!(
            "[phases.prepare]\n\
             prompt = \"Prepare the run.\"\n\
             guards = [{guard_table}]\n\
             \n\
             [phases.done]\n\
             prompt = \"Complete the run.\"\n\
             \n\
             [[transitions]]\n\
             from = \"prepare\"\n\
             to = \"done\"\n"
        );
        fs::write(root.join(".arca/ratmac.toml"), class).expect("write machine class");
        Fixture { root }
    }

    /// Copy the QA probe into the project so it is a project-derived artifact.
    fn install_gate(&self, relative: &str) -> PathBuf {
        let target = self.root.join(relative);
        fs::create_dir_all(target.parent().expect("gate parent")).expect("create gate directory");
        fs::copy(env!("CARGO_BIN_EXE_guard-probe"), &target).expect("install gate artifact");
        target
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    /// FDC-004: the live run's id — the newest roster entry carrying a State File.
    fn run_id(&self) -> String {
        let mut ids: Vec<String> = fs::read_dir(self.root.join(".arca/runs"))
            .expect("list the runs roster")
            .map(|entry| entry.expect("roster entry is readable"))
            .filter(|entry| entry.path().join("state.toml").is_file())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        ids.sort();
        ids.pop().expect("a live run appears on the roster")
    }

    fn state_path(&self) -> PathBuf {
        self.root
            .join(".arca/runs")
            .join(self.run_id())
            .join("state.toml")
    }

    fn step_text(&self) -> String {
        let id = self.run_id();
        let step = self.rtm(&["step", "--run", &id]);
        format!(
            "{}{}",
            String::from_utf8_lossy(&step.stdout),
            String::from_utf8_lossy(&step.stderr)
        )
    }

    /// FDC-004: Run evidence resides beside the run's State File.
    fn evidence(&self) -> String {
        let mut ids: Vec<String> = fs::read_dir(self.root.join(".arca/runs"))
            .into_iter()
            .flatten()
            .map(|entry| entry.expect("roster entry is readable"))
            .filter(|entry| entry.path().join("evidence.toml").is_file())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        ids.sort();
        ids.pop()
            .map(|id| {
                fs::read_to_string(self.root.join(".arca/runs").join(id).join("evidence.toml"))
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    fn state(&self) -> Vec<u8> {
        fs::read(self.state_path()).unwrap_or_default()
    }

    fn log(&self) -> Vec<u8> {
        fs::read(self.root.join(".arca/log.md")).unwrap_or_default()
    }
}

fn guard_for(program: &Path, mode: &str) -> String {
    format!(
        "{{ kind = \"command_exit\", program = \"{}\", args = [\"{mode}\"], expected = 0 }}",
        program.to_string_lossy().replace('\\', "\\\\")
    )
}

/// PT-041-01: the pin is recorded no later than first guard use.
#[test]
fn pin_is_recorded() {
    let fixture = Fixture::new("record", "");
    let gate = fixture.install_gate("gate/probe.exe");
    let class = fixture.root.join(".arca/ratmac.toml");
    let source = fs::read_to_string(&class).expect("read class");
    fs::write(
        &class,
        source.replace(
            "guards = []",
            &format!("guards = [{}]", guard_for(&gate, "pass")),
        ),
    )
    .expect("install guard");

    assert!(fixture.rtm(&["start"]).status.success(), "start succeeds");
    // The Stable Engine pin is recorded when the Run is created.
    let evidence = fixture.evidence();
    assert!(
        evidence.contains("[engine]") && evidence.contains("sha256"),
        "Run evidence must record the Stable Engine pin at start: {evidence}"
    );

    let text = fixture.step_text();
    assert!(
        !text.contains("step refused"),
        "a pristine pinned gate must pass: {text}"
    );
    let evidence = fixture.evidence();
    let digest = sha256_file(&gate);
    assert!(
        evidence.contains(&digest),
        "Run evidence must record the gate artifact digest {digest}: {evidence}"
    );
    assert!(
        evidence.contains("probe.exe"),
        "Run evidence must record the resolved gate path: {evidence}"
    );
}

/// PT-041-02: tamper refuses; restoring the exact bytes lets the same request
/// proceed.
#[test]
fn tamper_refuses_and_restore_proceeds() {
    let fixture = Fixture::new("tamper", "");
    let gate = fixture.install_gate("gate/probe.exe");
    let class = fixture.root.join(".arca/ratmac.toml");
    let source = fs::read_to_string(&class).expect("read class");
    fs::write(
        &class,
        source.replace(
            "guards = []",
            &format!("guards = [{}]", guard_for(&gate, "pass")),
        ),
    )
    .expect("install guard");
    let pristine = fs::read(&gate).expect("read gate bytes");

    assert!(fixture.rtm(&["start"]).status.success(), "start succeeds");
    // First use records the pin without refusing.
    let id = fixture.run_id();
    let first = fixture.rtm(&["status", "--run", &id]);
    assert!(first.status.success(), "status works");
    let text = fixture.step_text();
    assert!(
        !text.contains("step refused"),
        "pristine gate passes: {text}"
    );

    // Re-arm: rewind the same Run to the guarded Phase over its recorded
    // evidence. FDC-004 makes evidence run-scoped, so a fresh Run would
    // record a fresh pin; the recorded pin under test lives with this run.
    let state_path = fixture.state_path();
    let rewound = fs::read_to_string(&state_path)
        .expect("read advanced state")
        .replace("phase = \"done\"", "phase = \"prepare\"");
    fs::write(&state_path, rewound).expect("rewind run to the guarded phase");

    let mut tampered = pristine.clone();
    tampered.extend_from_slice(b"tampered");
    fs::write(&gate, &tampered).expect("tamper with gate");
    let state_before = fixture.state();
    let log_before = fixture.log();

    let refusal = fixture.step_text();
    assert!(
        refusal.contains("step refused"),
        "a tampered gate must refuse: {refusal}"
    );
    assert!(
        refusal.contains(&sha256_file(&gate)),
        "the refusal must name the observed identity: {refusal}"
    );
    let expected_digest = {
        let hasher_input = fixture.root.join("gate/probe.pristine");
        fs::write(&hasher_input, &pristine).expect("write pristine copy");
        let digest = sha256_file(&hasher_input);
        let _ = fs::remove_file(&hasher_input);
        digest
    };
    assert!(
        refusal.contains(&expected_digest),
        "the refusal must name the expected identity: {refusal}"
    );
    assert_eq!(state_before, fixture.state(), "refusal mutates no state");
    assert_eq!(log_before, fixture.log(), "refusal writes no history");

    fs::write(&gate, &pristine).expect("restore gate");
    let restored = fixture.step_text();
    assert!(
        !restored.contains("step refused"),
        "restoring the exact bytes lets the identical request proceed: {restored}"
    );
}

/// PT-041-03: a rebuild-style guard is rejected; a marked probe is accepted.
#[test]
fn build_at_evaluation_rejected_probe_exempt() {
    let building = Fixture::new(
        "driver",
        "{ kind = \"command_exit\", program = \"cargo\", args = [\"build\"], expected = 0 }",
    );
    assert!(building.rtm(&["start"]).status.success(), "start succeeds");
    let text = building.step_text();
    assert!(
        text.contains("step refused"),
        "a rebuild-style guard must be rejected: {text}"
    );
    assert!(
        text.to_ascii_lowercase().contains("rebuild")
            || text.to_ascii_lowercase().contains("build at evaluation"),
        "the rejection must name the reason: {text}"
    );
    assert!(
        !building.root.join("target").exists(),
        "nothing may be compiled inside the fixture"
    );

    let probe = Fixture::new(
        "probe",
        "{ kind = \"command_exit\", program = \"rustc\", args = [\"--version\"], \
         expected = 0, exempt = true }",
    );
    assert!(probe.rtm(&["start"]).status.success(), "start succeeds");
    let text = probe.step_text();
    assert!(
        !text.contains("step refused"),
        "a marked toolchain probe must be accepted: {text}"
    );
    let evidence = probe.evidence();
    assert!(
        !evidence.contains("rustc"),
        "an exempt probe is not a pinned gate artifact: {evidence}"
    );
    assert!(
        !probe.root.join("target").exists(),
        "a probe compiles nothing"
    );
}

/// HT-041-01 (Input/Routing): ambiguous or non-executable gate paths refuse.
#[test]
fn symlink_and_directory_programs_refuse() {
    let directory = Fixture::new("directory", "");
    let dir_path = directory.root.join("gate");
    fs::create_dir_all(&dir_path).expect("create directory gate");
    let class = directory.root.join(".arca/ratmac.toml");
    let source = fs::read_to_string(&class).expect("read class");
    fs::write(
        &class,
        source.replace(
            "guards = []",
            &format!("guards = [{}]", guard_for(&dir_path, "pass")),
        ),
    )
    .expect("install guard");
    assert!(directory.rtm(&["start"]).status.success(), "start succeeds");
    let text = directory.step_text();
    assert!(
        text.contains("step refused") && text.to_ascii_lowercase().contains("not a file"),
        "a directory gate must refuse with a named reason: {text}"
    );

    let linked = Fixture::new("indirect", "");
    let real = linked.install_gate("gate/probe.exe");
    let link = linked.root.join("gate/link.exe");
    #[cfg(windows)]
    let created = std::os::windows::fs::symlink_file(&real, &link).is_ok();
    #[cfg(not(windows))]
    let created = std::os::unix::fs::symlink(&real, &link).is_ok();
    assert!(
        created,
        "this lane requires symlink creation (Windows Developer Mode)"
    );
    let class = linked.root.join(".arca/ratmac.toml");
    let source = fs::read_to_string(&class).expect("read class");
    fs::write(
        &class,
        source.replace(
            "guards = []",
            &format!("guards = [{}]", guard_for(&link, "pass")),
        ),
    )
    .expect("install guard");
    assert!(linked.rtm(&["start"]).status.success(), "start succeeds");
    let text = linked.step_text();
    // The reason must come from the refusal, not from the fixture path: the
    // fixture label deliberately avoids the word this assertion looks for.
    assert!(
        text.contains("step refused") && text.contains("no stable identity"),
        "a symlinked gate must refuse with a named reason: {text}"
    );
}

/// HT-041-02 (Durability/Recovery): a pin recorded before a crash is still the
/// authority for the next evaluation.
#[test]
fn recorded_pin_survives_a_crashed_run() {
    let fixture = Fixture::new("crash", "");
    let gate = fixture.install_gate("gate/probe.exe");
    let class = fixture.root.join(".arca/ratmac.toml");
    let source = fs::read_to_string(&class).expect("read class");
    fs::write(
        &class,
        source.replace(
            "guards = []",
            &format!("guards = [{}]", guard_for(&gate, "pass")),
        ),
    )
    .expect("install guard");
    assert!(fixture.rtm(&["start"]).status.success(), "start succeeds");

    // Simulate the crash: evidence written, no transition taken.
    // FDC-004: Run evidence resides beside the run's State File.
    let evidence_path = fixture
        .root
        .join(".arca/runs")
        .join(fixture.run_id())
        .join("evidence.toml");
    let engine_pin = fs::read_to_string(&evidence_path).expect("read evidence");
    let stale = format!(
        "{engine_pin}\n[[gate]]\nprogram = \"{}\"\nresolved = \"{}\"\nsha256 = \"{}\"\n",
        gate.to_string_lossy().replace('\\', "\\\\"),
        gate.to_string_lossy().replace('\\', "\\\\"),
        "0".repeat(64)
    );
    fs::write(&evidence_path, stale).expect("seed crashed-run evidence");

    let text = fixture.step_text();
    assert!(
        text.contains("step refused") && text.contains(&"0".repeat(64)),
        "the recorded pin must remain the expected identity after a crash: {text}"
    );
}

/// HT-041-03 (Cross-Feature): a pin refusal reads like every other refusal.
#[test]
fn pin_refusal_carries_identity_and_diagnostic_framing() {
    let fixture = Fixture::new("framing", "");
    let gate = fixture.install_gate("gate/probe.exe");
    let class = fixture.root.join(".arca/ratmac.toml");
    let source = fs::read_to_string(&class).expect("read class");
    fs::write(
        &class,
        source.replace(
            "guards = []",
            &format!("guards = [{}]", guard_for(&gate, "pass")),
        ),
    )
    .expect("install guard");
    assert!(fixture.rtm(&["start"]).status.success(), "start succeeds");
    assert!(
        !fixture.step_text().contains("step refused"),
        "pin recorded"
    );

    // FDC-004: evidence is run-scoped — rewind this run to the guarded Phase
    // so the recorded pin stays the authority under test.
    let state_path = fixture.state_path();
    let rewound = fs::read_to_string(&state_path)
        .expect("read advanced state")
        .replace("phase = \"done\"", "phase = \"prepare\"");
    fs::write(&state_path, rewound).expect("rewind run to the guarded phase");
    let mut bytes = fs::read(&gate).expect("read gate");
    bytes.extend_from_slice(b"x");
    fs::write(&gate, bytes).expect("tamper");

    let text = fixture.step_text();
    assert!(
        text.contains("observed") && text.contains("expected"),
        "the refusal must state observed and expected identity: {text}"
    );
    assert!(
        text.contains("diagnostic:"),
        "the refusal must reuse the bounded diagnostic framing: {text}"
    );
    assert!(
        text.to_ascii_lowercase().contains("not executed"),
        "the refusal must say the gate never ran: {text}"
    );
}
