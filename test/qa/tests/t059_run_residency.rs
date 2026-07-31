//! t-058 / FDC-004: run residency — the plural runs layout and `--run` addressing.
//!
//! PT-058-01 `requirements_trace_to_seed_and_research`
//! PT-058-02 `runs_reside_under_the_plural_path`
//! PT-058-03 `run_addressing_is_always_required`
//!
//! Runs must reside canonically under the plural `.arca/runs/<id>/` path so
//! that listing the directory IS the roster: run identity is read off
//! artifacts, never off a narrated index. Every command that acts on an
//! existing Run takes `--run <id>`, always required; a missing value refuses
//! and prints the roster. The run's verdict slot and per-run spawn-ledger
//! path are reserved under its directory by name only — their contracts stay
//! with the machine-composition issue (i-018) and are not exercised here.
//! This supersedes `R-023` (no run-id in v1) and its check `T-09`.

use ratmac_qa::role::{load_scenario, Outcome};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo(relative: &str) -> String {
    fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|error| panic!("RRV-001: {relative} must exist and be readable: {error}"))
}

/// The body of a `## `-headed section, up to the next section.
fn section<'a>(text: &'a str, heading: &str) -> &'a str {
    let start = text
        .find(heading)
        .unwrap_or_else(|| panic!("RRV-001: the section {heading:?} must exist"));
    let rest = &text[start + heading.len()..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    &rest[..end]
}

struct Fixture {
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    /// A temp project with a valid two-phase runbook, not yet started.
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t058-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".arca")).expect("create fixture tree");
        fs::create_dir_all(root.join("src")).expect("create source tree");
        fs::write(root.join("src/lib.rs"), "pub fn work() {}\n").expect("write source");
        fs::create_dir_all(root.join(".arca/goal")).expect("create goal tree");
        fs::write(root.join(".arca/goal/spec.md"), "# Spec\n").expect("write goal");
        fs::write(
            root.join(".arca/ratmac.toml"),
            "[phases.intake]\nprompt = \"Integrate the issues.\"\n\n\
             [phases.build]\nprompt = \"Build the ticket.\"\n\n\
             [[transitions]]\nfrom = \"intake\"\nto = \"build\"\n",
        )
        .expect("write runbook");
        Fixture { root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Listing `.arca/runs/` IS the roster: the run ids, read off artifacts.
    fn roster(&self) -> Vec<String> {
        let runs = self.path(".arca/runs");
        assert!(
            runs.is_dir(),
            "FDC-004: runs must reside under the plural .arca/runs/ path — listing it IS the roster"
        );
        let mut ids: Vec<String> = fs::read_dir(&runs)
            .expect("the runs directory must be listable")
            .map(|entry| entry.expect("roster entry is readable"))
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        ids.sort();
        ids
    }
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn state_phase(state: &str) -> String {
    let value: toml::Value = state
        .parse()
        .expect("FDC-004: the run's State File must be valid TOML");
    value["phase"]
        .as_str()
        .expect("FDC-004: the run's State File must carry a phase")
        .to_owned()
}

/// PT-058-01 (RRV-001): each of `FDC-004`–`FDC-006` traces to its
/// adopted-default record in the split seed's design (Adopted defaults, batch
/// human sign-off 2026-07-29), and the research sections that record
/// condenses — run identity, the invocation join, migration cost — exist
/// under `.arca/research/re-ratmac-FSM/`.
#[test]
fn requirements_trace_to_seed_and_research() {
    let spec = read_repo(".arca/goal/spec.md");
    let rows = section(&spec, "## Integrated run-residency requirements");
    for id in ["FDC-004", "FDC-005", "FDC-006"] {
        let row = rows
            .lines()
            .find(|line| line.starts_with(&format!("| {id} ")))
            .unwrap_or_else(|| panic!("RRV-001: the goal must carry the {id} requirement row"));
        assert!(
            row.contains("../issue/i-017-run-residency/spec.md#requirement-records"),
            "RRV-001: {id} must cite its requirement record in the run-residency issue"
        );
    }

    let issue = read_repo(".arca/issue/i-017-run-residency/spec.md");
    for id in ["FDC-004", "FDC-005", "FDC-006"] {
        assert!(
            issue.contains(&format!("`{id}`")),
            "RRV-001: the run-residency issue must carry the requirement record for {id}"
        );
    }
    assert!(
        issue.contains("../i-016-fsm-doctrine-convergence/design.md"),
        "RRV-001: the run-residency issue must point back at the split seed's design"
    );

    let seed = read_repo(".arca/issue/i-016-fsm-doctrine-convergence/design.md");
    let defaults = section(
        &seed,
        "## Adopted defaults (batch human sign-off, 2026-07-29)",
    );
    let records: [(&str, &[&str]); 3] = [
        (
            "FDC-004",
            &["plural `runs`", "`--run <id>`, always required"],
        ),
        (
            "FDC-005",
            &["hash-only", "refuses and instructs, never auto-migrates"],
        ),
        (
            "FDC-006",
            &[
                "cap lifts entirely",
                "never reused after abandon",
                "respawn mints a new id",
            ],
        ),
    ];
    for (id, marks) in records {
        for mark in marks {
            assert!(
                defaults.contains(mark),
                "RRV-001: {id}'s adopted-default record must state {mark:?} in the Adopted defaults section"
            );
        }
    }

    for (name, topic) in [
        ("04-run-identity.md", "run identity"),
        ("05-invocation-join.md", "the invocation join"),
        ("06-migration-cost.md", "migration cost"),
    ] {
        let relative = format!(".arca/research/re-ratmac-FSM/{name}");
        let text = fs::read_to_string(repo_root().join(&relative)).unwrap_or_else(|error| {
            panic!("RRV-001: the research section on {topic} must exist at {relative}: {error}")
        });
        assert!(
            !text.trim().is_empty(),
            "RRV-001: the research section on {topic} must not be empty"
        );
    }
}

/// PT-058-02 (RRV-002): `rtm start` creates `.arca/runs/<id>/` carrying the
/// run's State File; the verdict slot and the spawn-ledger path exist under
/// that directory by name only, with no contract exercised; no flat
/// `.arca/state.toml` is written; listing `.arca/runs/` yields the roster.
#[test]
fn runs_reside_under_the_plural_path() {
    let fixture = Fixture::new("plural-path");
    let start = fixture.rtm(&["start"]);
    assert!(
        start.status.success(),
        "start must succeed on a valid project: {}",
        combined(&start)
    );

    let roster = fixture.roster();
    assert_eq!(
        roster.len(),
        1,
        "FDC-004: one start mints exactly one run id; the roster lists it, found {roster:?}"
    );
    let id = &roster[0];
    assert!(
        !id.is_empty(),
        "FDC-004: the minted run id must be non-empty"
    );

    let run_dir = fixture.path(&format!(".arca/runs/{id}"));
    let state = fs::read_to_string(run_dir.join("state.toml")).unwrap_or_else(|error| {
        panic!("FDC-004: the run directory must carry the run's State File: {error}")
    });
    let _ = state_phase(&state);

    // Reserved by name only: existence is asserted, contents never read —
    // the verdict and ledger contracts belong to machine composition (i-018).
    for name in ["verdict.toml", "spawn-ledger"] {
        assert!(
            run_dir.join(name).exists(),
            "FDC-004: {name} must be reserved under the run's directory by name"
        );
    }

    assert!(
        !fixture.path(".arca/state.toml").exists(),
        "FDC-004: the flat .arca/state.toml must no longer be written"
    );
}

/// PT-058-03 (RRV-002): `status` and `step` without `--run` refuse and print
/// the roster — the refusal recorded as a behavioral role scenario under
/// `test/qa/fixtures/role-scenarios/`; with `--run <id>` each acts on exactly
/// the named run.
#[test]
fn run_addressing_is_always_required() {
    let fixture = Fixture::new("addressing");
    let start = fixture.rtm(&["start"]);
    assert!(
        start.status.success(),
        "start must succeed on a valid project: {}",
        combined(&start)
    );

    let roster = fixture.roster();
    let id = roster
        .first()
        .expect("FDC-004: the started run must appear on the roster")
        .clone();
    let state_rel = format!(".arca/runs/{id}/state.toml");
    let before = fs::read(fixture.path(&state_rel))
        .expect("FDC-004: the named run's State File must be readable");

    for command in ["status", "step"] {
        let refused = fixture.rtm(&[command]);
        assert!(
            !refused.status.success(),
            "FDC-004: {command} without --run must refuse — run addressing is always required"
        );
        let text = combined(&refused);
        assert!(
            text.contains(&id),
            "FDC-004: the {command} refusal must print the roster; the run id {id:?} is absent from: {text}"
        );
        assert_eq!(
            fs::read(fixture.path(&state_rel)).expect("the named run's State File stays readable"),
            before,
            "FDC-004: a refused {command} must not touch the named run"
        );
    }

    let status = fixture.rtm(&["status", "--run", &id]);
    assert!(
        status.status.success(),
        "FDC-004: status --run <id> must act on the named run: {}",
        combined(&status)
    );
    assert_eq!(
        fs::read(fixture.path(&state_rel)).expect("the named run's State File stays readable"),
        before,
        "FDC-004: status must remain read-only on the named run"
    );

    let step = fixture.rtm(&["step", "--run", &id]);
    assert!(
        step.status.success(),
        "FDC-004: step --run <id> must act on the named run: {}",
        combined(&step)
    );
    let after = fs::read_to_string(fixture.path(&state_rel))
        .expect("the named run's State File stays readable");
    assert_eq!(
        state_phase(&after),
        "build",
        "FDC-004: stepping the named run must advance exactly that run"
    );
    assert!(
        !fixture.path(".arca/state.toml").exists(),
        "FDC-004: addressed commands must not resurrect the flat path"
    );

    // The refusal is recorded as behavioral evidence: a role-scenario
    // transcript of the attempted no---run commands, authored from the real
    // refusal — prose about the policy satisfies nothing (ORS-003 precedent).
    let transcript = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/role-scenarios/missing-run-refusal.toml");
    let scenario = load_scenario(&transcript).unwrap_or_else(|defect| {
        panic!("RRV-002: the missing---run refusal must be recorded as a behavioral role scenario: {defect}")
    });
    for attempted in ["rtm status", "rtm step"] {
        assert!(
            scenario
                .events
                .iter()
                .any(|event| event.command.as_deref() == Some(attempted)
                    && event.outcome == Outcome::Invoked),
            "RRV-002: the transcript must record the attempted {attempted:?} without --run"
        );
    }
    let recorded = format!(
        "{} {}",
        scenario.description,
        scenario
            .events
            .iter()
            .map(|event| event.reason.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    )
    .to_lowercase();
    for word in ["refus", "roster"] {
        assert!(
            recorded.contains(word),
            "RRV-002: the transcript must record the refusal and the printed roster, missing {word:?}"
        );
    }
}
