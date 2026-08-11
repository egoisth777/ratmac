//! t-064 / FDC-009: one runbook can declare a composed machine.
//!
//! PT-064-01 `composed_declaration_parses_and_is_doctor_clean`
//! PT-064-02 `malformed_composition_refuses_by_stable_code`
//! PT-064-03 `blocked_route_spelling_stays_canonical`

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// A composed machine: one declared child class, one spawning State, one
/// join-guarded State. The declarations are dormant this increment.
const COMPOSED_RUNBOOK: &str = r#"
[classes.reviewer.bindings.ticket]
required = true

[classes.reviewer.states.review]
prompt = "Review the delegated ticket."

[states.plan]
prompt = "Plan."

[states.delegate]
prompt = "Delegate and wait."
guards = [{ kind = "join", require = "all_passed", min = 1 }]

[[states.delegate.spawns]]
class = "reviewer"
name = "rev"
bind = ["ticket"]

[states.done]
prompt = "Done."

[[transitions]]
from = "plan"
to = "delegate"

[[transitions]]
from = "delegate"
to = "done"
"#;

/// The composed runbook with a blocked route beside the new tables.
const COMPOSED_BLOCKED_RUNBOOK: &str = r#"
[classes.reviewer.states.review]
prompt = "Review."

[states.plan]
prompt = "Plan."

[states.done]
prompt = "Done."

[[transitions]]
from = "plan"
to = "done"

[[transitions]]
from = "done"
to = "plan"
blocked-route = true
"#;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn create(label: &str, runbook: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t064-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".arca/goal")).expect("create fixture goal tree");
        fs::create_dir_all(root.join(".ratmac")).expect("create fixture Engine tree");
        fs::create_dir_all(root.join("src")).expect("create fixture source tree");
        fs::write(root.join(".arca/goal/spec.md"), "# Fixture goal\n").expect("write fixture goal");
        fs::write(root.join(".ratmac/ratmac.toml"), runbook).expect("write fixture machine class");
        fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").expect("write fixture source");
        Self { root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    /// Rewrite the machine class in place.
    fn write_runbook(&self, runbook: &str) {
        fs::write(self.root.join(".ratmac/ratmac.toml"), runbook)
            .expect("rewrite fixture machine class");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// PT-064-01: the composed declaration parses, is doctor-clean, and stays
/// dormant - `rtm start` still begins ordinarily at the initial State.
#[test]
fn composed_declaration_parses_and_is_doctor_clean() {
    let fixture = Fixture::create("clean", COMPOSED_RUNBOOK);

    let doctor = fixture.rtm(&["doctor"]);
    let report = combined(&doctor);
    assert!(
        doctor.status.success(),
        "composed runbook must be doctor-clean, got:\n{report}"
    );
    assert!(
        report.contains("No findings."),
        "composed runbook must produce zero findings, got:\n{report}"
    );

    let start = fixture.rtm(&["start"]);
    assert!(
        start.status.success(),
        "start on a composed runbook proceeds ordinarily, got:\n{}",
        combined(&start)
    );
    let runs = fs::read_dir(fixture.root.join(".ratmac/runs"))
        .expect("started fixture has a runs roster")
        .map(|entry| entry.expect("read roster entry").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    assert_eq!(runs.len(), 1, "start mints exactly one Run");
    let state = fs::read_to_string(runs[0].join("run.toml")).expect("read State File");
    assert!(
        state.contains("state = \"plan\""),
        "the Run begins at the initial State; declarations never route:\n{state}"
    );
    assert!(
        state.contains("status = \"planned\""),
        "a composed declaration is dormant - no spawn, no status change:\n{state}"
    );
}

/// PT-064-02: each malformed declaration refuses with its stable RB5xx code
/// naming the offending table and key; nothing partially parses.
#[test]
fn malformed_composition_refuses_by_stable_code() {
    let fixture = Fixture::create("malformed", COMPOSED_RUNBOOK);

    // (a) A spawn whose bind set misses the class's required binding name.
    fixture.write_runbook(
        r#"
[classes.reviewer.bindings.ticket]
required = true

[classes.reviewer.states.review]
prompt = "Review."

[states.done]
prompt = "Done."

[[states.done.spawns]]
class = "reviewer"
name = "rev"
"#,
    );
    let doctor = fixture.rtm(&["doctor"]);
    let report = combined(&doctor);
    assert!(
        !doctor.status.success(),
        "a spawn missing a required binding name must refuse:\n{report}"
    );
    assert!(
        report.contains("RB505") && report.contains("ticket") && report.contains("reviewer"),
        "the refusal names RB505, the class, and the missing binding name:\n{report}"
    );

    // (b) A spawn entry naming an undeclared class.
    fixture.write_runbook(
        r#"
[states.done]
prompt = "Done."

[[states.done.spawns]]
class = "ghost"
name = "g"
"#,
    );
    let doctor = fixture.rtm(&["doctor"]);
    let report = combined(&doctor);
    assert!(
        !doctor.status.success(),
        "a spawn naming an undeclared class must refuse:\n{report}"
    );
    assert!(
        report.contains("RB504") && report.contains("ghost"),
        "the refusal names RB504 and the undeclared class:\n{report}"
    );

    // (c) A join value outside the closed vocabulary.
    fixture.write_runbook(
        r#"
[states.wait]
prompt = "Wait."
guards = [{ kind = "join", require = "any_passed" }]

[states.done]
prompt = "Done."

[[transitions]]
from = "wait"
to = "done"
"#,
    );
    let doctor = fixture.rtm(&["doctor"]);
    let report = combined(&doctor);
    assert!(
        !doctor.status.success(),
        "a join value outside the closed vocabulary must refuse:\n{report}"
    );
    assert!(
        report.contains("RB506") && report.contains("any_passed"),
        "the refusal names RB506 and the foreign value:\n{report}"
    );

    // A min below the least legal count refuses by the same class.
    fixture.write_runbook(
        r#"
[states.wait]
prompt = "Wait."
guards = [{ kind = "join", require = "all_passed", min = 0 }]

[states.done]
prompt = "Done."

[[transitions]]
from = "wait"
to = "done"
"#,
    );
    let doctor = fixture.rtm(&["doctor"]);
    let report = combined(&doctor);
    assert!(
        !doctor.status.success() && report.contains("RB506"),
        "join min below 1 refuses as RB506:\n{report}"
    );
}

/// PT-064-03: the hyphen spelling stays canonical beside the new tables;
/// underscore and space variants refuse exactly as before.
#[test]
fn blocked_route_spelling_stays_canonical() {
    let fixture = Fixture::create("blocked", COMPOSED_BLOCKED_RUNBOOK);

    let doctor = fixture.rtm(&["doctor"]);
    let report = combined(&doctor);
    assert!(
        doctor.status.success(),
        "the hyphen spelling parses beside the new tables:\n{report}"
    );

    for (label, variant) in [
        ("underscore", "blocked_route = true"),
        ("space", "\"blocked route\" = true"),
    ] {
        let broken = COMPOSED_BLOCKED_RUNBOOK.replace("blocked-route = true", variant);
        fixture.write_runbook(&broken);
        let doctor = fixture.rtm(&["doctor"]);
        let report = combined(&doctor);
        assert!(
            !doctor.status.success(),
            "{label} spelling must refuse:\n{report}"
        );
        assert!(
            report.contains("RB103"),
            "{label} spelling is an unknown key (RB103):\n{report}"
        );
    }
}
