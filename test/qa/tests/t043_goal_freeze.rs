//! t-042 / ETB-003: post-integration goal freeze and drift refusal.
//!
//! PT-042-01 `freeze_is_post_integration`
//! PT-042-02 `drift_refuses_and_revert_clears`
//! HT-042-01 `added_and_removed_goal_files_are_drift`
//! HT-042-02 `interrupted_freeze_leaves_readable_state`
//! HT-042-03 `drift_and_pin_tamper_are_both_reported`
//!
//! The goal revision cited by gap analysis is frozen at the intake-completion
//! boundary, not at Run start; after the freeze a change under
//! `.arca/goal/` is refused as goal drift naming both revisions.

use std::fs;
use std::path::PathBuf;
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
    /// A project whose `intake -> gaps` transition closes intake integration.
    fn new(label: &str, gaps_guards: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t043-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".arca/goal")).expect("create fixture project");
        fs::create_dir_all(root.join(".ratmac")).expect("create Engine directory");
        let class = format!(
            "[roots]\n\
             goal = \".arca/goal\"\n\
             \n\
             [phases.intake]\n\
             prompt = \"Integrate the issues.\"\n\
             [phases.gaps]\n\
             prompt = \"Find the gaps.\"\n\
             guards = [{gaps_guards}]\n\
             \n\
             [phases.tickets]\n\
             prompt = \"Cut the tickets.\"\n\
             \n\
             [[transitions]]\n\
             from = \"intake\"\n\
             to = \"gaps\"\n\
             freeze = \"goal\"\n\
             \n\
             [[transitions]]\n\
             from = \"gaps\"\n\
             to = \"tickets\"\n"
        );
        fs::write(root.join(".ratmac/ratmac.toml"), class).expect("write machine class");
        let fixture = Fixture { root };
        fixture.write_goal("spec.md", "# Spec\n\nrequirement one\n");
        fixture.write_goal("test-list.md", "# Test list\n\ncheck one\n");
        fixture
    }

    fn write_goal(&self, name: &str, content: &str) {
        fs::write(self.root.join(".arca/goal").join(name), content).expect("write goal file");
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    fn text_of(output: &Output) -> String {
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }

    /// FDC-004: the live run's id, read off the plural roster.
    fn run_id(&self) -> String {
        let mut ids: Vec<String> = fs::read_dir(self.root.join(".ratmac/runs"))
            .expect("list the runs roster")
            .map(|entry| entry.expect("roster entry is readable"))
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        ids.sort();
        ids.pop().expect("a run appears on the roster")
    }

    fn run_dir(&self) -> PathBuf {
        self.root.join(".ratmac/runs").join(self.run_id())
    }

    fn step_text(&self) -> String {
        let id = self.run_id();
        Self::text_of(&self.rtm(&["step", "--run", &id]))
    }

    fn status_text(&self) -> String {
        let id = self.run_id();
        Self::text_of(&self.rtm(&["status", "--run", &id]))
    }

    fn evidence(&self) -> String {
        fs::read_to_string(self.run_dir().join("evidence.toml")).unwrap_or_default()
    }

    fn state(&self) -> Vec<u8> {
        fs::read(self.run_dir().join("state.toml")).unwrap_or_default()
    }

    fn log(&self) -> Vec<u8> {
        fs::read(self.root.join(".ratmac/log.md")).unwrap_or_default()
    }
}

/// Read one `key = "value"` string out of a TOML section body.
fn field(section: &str, key: &str) -> Option<String> {
    section.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name.trim() == key).then(|| value.trim().trim_matches('"').to_owned())
    })
}

/// The `[goal]` table of Run evidence.
fn goal_table(evidence: &str) -> String {
    evidence
        .split("[goal]")
        .nth(1)
        .map(|rest| {
            rest.split("\n[")
                .next()
                .unwrap_or_default()
                .trim()
                .to_owned()
        })
        .unwrap_or_default()
}

/// PT-042-01: the freeze happens at intake completion, not at Run start, and
/// the two revisions are recorded as distinct fields.
#[test]
fn freeze_is_post_integration() {
    let fixture = Fixture::new("post-integration", "");
    assert!(fixture.rtm(&["start"]).status.success(), "start succeeds");

    let at_start = goal_table(&fixture.evidence());
    let baseline = field(&at_start, "baseline").expect("start records a baseline revision");
    assert_eq!(
        field(&at_start, "frozen").unwrap_or_default(),
        "",
        "Run start must not freeze the goal: {at_start}"
    );
    assert!(
        !fixture.status_text().contains(&baseline),
        "gap analysis must not cite an unfrozen revision"
    );

    // The intake phase does what intake does: it rewrites `.arca/goal/`.
    fixture.write_goal("spec.md", "# Spec\n\nrequirement one\nrequirement two\n");
    fixture.write_goal("design.md", "# Design\n\nintegrated\n");

    let advance = fixture.step_text();
    assert!(
        !advance.contains("step refused"),
        "the freezing transition itself must not be drift: {advance}"
    );

    let after = goal_table(&fixture.evidence());
    let frozen = field(&after, "frozen").expect("intake completion freezes the goal");
    assert_eq!(
        field(&after, "baseline").as_deref(),
        Some(baseline.as_str()),
        "the start baseline stays as recorded: {after}"
    );
    assert_ne!(
        frozen, baseline,
        "the frozen revision is the post-integration content, not the start content"
    );
    assert!(
        fixture.status_text().contains(&frozen),
        "gap analysis cites the frozen revision"
    );
}

/// PT-042-02: after the freeze, editing the goal refuses the next transition
/// naming both revisions; reverting clears it; Scheduler state is untouched.
#[test]
fn drift_refuses_and_revert_clears() {
    let fixture = Fixture::new("drift", "");
    assert!(fixture.rtm(&["start"]).status.success(), "start succeeds");
    assert!(
        !fixture.step_text().contains("step refused"),
        "intake completes"
    );
    let frozen = field(&goal_table(&fixture.evidence()), "frozen").expect("goal is frozen");

    let spec = fixture.root.join(".arca/goal/spec.md");
    let pristine = fs::read_to_string(&spec).expect("read goal file");
    fs::write(&spec, format!("{pristine}requirement three\n")).expect("edit the frozen goal");

    let state_before = fixture.state();
    let log_before = fixture.log();
    let refusal = fixture.step_text();
    assert!(
        refusal.contains("step refused") && refusal.contains("goal drift"),
        "an edited goal refuses the next transition: {refusal}"
    );
    assert!(
        refusal.contains(&frozen),
        "the refusal names the frozen revision: {refusal}"
    );
    let observed = field(&goal_table(&fixture.evidence()), "frozen").expect("frozen field stays");
    assert_eq!(observed, frozen, "a refusal must not re-freeze the goal");
    let drifted = ratmac::goal::revision(&fixture.root.join(".arca/goal"))
        .expect("goal is readable")
        .expect("goal directory is present");
    assert_ne!(drifted, frozen, "the edit changed the goal revision");
    assert!(
        refusal.contains(&drifted),
        "the refusal names the observed revision: {refusal}"
    );
    assert_eq!(
        state_before,
        fixture.state(),
        "a drift refusal leaves state.toml byte-identical"
    );
    assert_eq!(
        log_before,
        fixture.log(),
        "a drift refusal leaves log.md byte-identical"
    );

    fs::write(&spec, &pristine).expect("revert the goal");
    let after_revert = fixture.step_text();
    assert!(
        !after_revert.contains("step refused"),
        "reverting the goal clears the drift: {after_revert}"
    );
}

/// A fixture whose intake integration is complete: the goal is frozen and one
/// clean transition remains.
fn frozen_fixture(label: &str) -> (Fixture, String) {
    let fixture = Fixture::new(label, "");
    assert!(fixture.rtm(&["start"]).status.success(), "start succeeds");
    assert!(
        !fixture.step_text().contains("step refused"),
        "intake completes"
    );
    let frozen = field(&goal_table(&fixture.evidence()), "frozen").expect("goal is frozen");
    (fixture, frozen)
}

/// HT-042-01 (Input/Routing): the goal's shape is part of its revision, so an
/// added, renamed, or removed file is drift even when no file is edited.
#[test]
fn added_and_removed_goal_files_are_drift() {
    let (fixture, frozen) = frozen_fixture("added");
    let added = fixture.root.join(".arca/goal/ubi-lang.md");
    fs::write(&added, "# Words\n").expect("add a goal file");
    let refusal = fixture.step_text();
    assert!(
        refusal.contains("step refused") && refusal.contains("goal drift"),
        "an added goal file is drift: {refusal}"
    );
    assert!(
        refusal.contains(&frozen) && refusal.contains("observed"),
        "the refusal names the observed revision: {refusal}"
    );
    fs::remove_file(&added).expect("remove the added file");
    assert!(
        !fixture.step_text().contains("step refused"),
        "removing the addition restores the frozen goal"
    );

    // A rename with identical bytes: only a revision that covers the goal's
    // shape can see this.
    let (fixture, _) = frozen_fixture("renamed");
    let spec = fixture.root.join(".arca/goal/spec.md");
    let renamed = fixture.root.join(".arca/goal/spec-v2.md");
    fs::rename(&spec, &renamed).expect("rename a goal file");
    let refusal = fixture.step_text();
    assert!(
        refusal.contains("step refused") && refusal.contains("goal drift"),
        "a renamed goal file is drift: {refusal}"
    );
    fs::rename(&renamed, &spec).expect("undo the rename");
    assert!(
        !fixture.step_text().contains("step refused"),
        "undoing the rename restores the frozen goal"
    );

    let (fixture, _) = frozen_fixture("removed");
    fs::remove_file(fixture.root.join(".arca/goal/test-list.md")).expect("delete a goal file");
    let refusal = fixture.step_text();
    assert!(
        refusal.contains("step refused") && refusal.contains("goal drift"),
        "a deleted goal file is drift: {refusal}"
    );
}

/// HT-042-02 (Durability/Recovery): a failed freeze write leaves a readable
/// state file that is either fully frozen or unchanged.
#[test]
fn interrupted_freeze_leaves_readable_state() {
    let fixture = Fixture::new("interrupt", "");
    assert!(fixture.rtm(&["start"]).status.success(), "start succeeds");

    // Interrupt the freeze deterministically: no platform can write a file
    // over a directory, so the evidence write at the boundary must fail.
    // FDC-004: Run evidence resides beside the run's State File.
    let evidence_path = fixture.run_dir().join("evidence.toml");
    let saved = fs::read_to_string(&evidence_path).expect("evidence exists after start");
    fs::remove_file(&evidence_path).expect("remove evidence file");
    fs::create_dir(&evidence_path).expect("block the evidence path");

    let interrupted = fixture.step_text();
    assert!(
        interrupted.contains("evidence.toml"),
        "the interrupted freeze must report the write it could not make: {interrupted}"
    );
    let state = String::from_utf8(fixture.state()).expect("state stays valid UTF-8");
    let parsed: toml::Value = state.parse().expect("state stays parseable TOML");
    let phase = parsed
        .get("phase")
        .and_then(toml::Value::as_str)
        .expect("state keeps its phase field");
    let revision = parsed
        .get("goal_revision")
        .and_then(toml::Value::as_str)
        .expect("state keeps its goal_revision field");
    assert!(
        (phase == "intake" && revision.is_empty()) || (phase == "gaps" && !revision.is_empty()),
        "an interrupted freeze is all or nothing: phase {phase}, revision {revision:?}, \
         output: {interrupted}"
    );
    assert_eq!(
        phase, "intake",
        "a freeze that could not record its evidence must not advance the Run"
    );
    assert!(
        fixture.log().is_empty() || !String::from_utf8_lossy(&fixture.log()).contains("intake ->"),
        "an interrupted freeze writes no transition to the log"
    );

    fs::remove_dir(&evidence_path).expect("unblock the evidence path");
    fs::write(&evidence_path, saved).expect("restore evidence");
    let retried = fixture.step_text();
    assert!(
        !retried.contains("step refused") && !retried.contains("evidence.toml"),
        "the freeze completes once the interruption is removed: {retried}"
    );
    assert!(
        field(&goal_table(&fixture.evidence()), "frozen").is_some(),
        "the retried freeze records the frozen revision"
    );
}

/// HT-042-03 (Cross-Feature): drift refusal and pin refusal compose; neither
/// hides the other.
#[test]
fn drift_and_pin_tamper_are_both_reported() {
    let gate_source = PathBuf::from(env!("CARGO_BIN_EXE_guard-probe"));
    let fixture = Fixture::new("compose", "");
    let gate = fixture.root.join("gate/probe.exe");
    fs::create_dir_all(gate.parent().expect("gate parent")).expect("create gate directory");
    fs::copy(&gate_source, &gate).expect("install gate artifact");
    let escaped = gate.to_string_lossy().replace('\\', "\\\\");
    let class = fixture.root.join(".ratmac/ratmac.toml");
    let source = fs::read_to_string(&class).expect("read class");
    fs::write(
        &class,
        source.replace(
            "guards = []",
            &format!(
                "guards = [{{ kind = \"command_exit\", program = \"{escaped}\", \
                 args = [\"pass\"], expected = 0 }}]"
            ),
        ),
    )
    .expect("install gaps guard");

    assert!(fixture.rtm(&["start"]).status.success(), "start succeeds");
    assert!(
        !fixture.step_text().contains("step refused"),
        "intake completes"
    );

    // A stale pin (as if the artifact were rebuilt) and an edited goal, in the
    // same transition request.
    let evidence_path = fixture.run_dir().join("evidence.toml");
    let evidence = fs::read_to_string(&evidence_path).expect("read evidence");
    fs::write(
        &evidence_path,
        format!(
            "{evidence}\n[[gate]]\nprogram = \"{escaped}\"\nresolved = \"{escaped}\"\n\
             sha256 = \"{}\"\n",
            "0".repeat(64)
        ),
    )
    .expect("seed a stale pin");
    let spec = fixture.root.join(".arca/goal/spec.md");
    let pristine = fs::read_to_string(&spec).expect("read goal file");
    fs::write(&spec, format!("{pristine}drifted\n")).expect("edit the frozen goal");

    let refusal = fixture.step_text();
    assert!(
        refusal.contains("goal drift"),
        "the goal drift is reported: {refusal}"
    );
    assert!(
        refusal.contains("command_exit") && refusal.contains(&"0".repeat(64)),
        "the pin mismatch is reported in the same refusal: {refusal}"
    );
}
