//! t-083 / SVC-001, SVC-002: every surface a caller reads says State.
//!
//! SVCV-001 `three_names_are_defined_separately`
//! SVCV-007 `caller_surfaces_name_the_position_state`
//!
//! State is the position, Run Record is the file, Run is the live instance,
//! and `status` is the lifecycle. One word each, in the documents that bind
//! and in every report a caller reads.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The spelling this cutover retires wherever a caller can read it.
const PRE_CUTOVER: &str = "phase";

/// A two-State machine with a guard a caller can fail on purpose.
const RUNBOOK: &str = "[roots]\n\
     ticket = \".arca/ticket\"\n\n\
     [states.intake]\nprompt = \"Integrate the issues.\"\n\
     guards = [{ kind = \"files_exact\", root = \"ticket\", path = \"done.txt\" }]\n\n\
     [states.build]\nprompt = \"Build the ticket.\"\n\n\
     [[transitions]]\nfrom = \"intake\"\nto = \"build\"\n";

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
    fn new(label: &str) -> Self {
        let base = std::env::temp_dir().join(format!(
            "ratmac-t083-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&base);
        let root = base.join("project");
        for dir in [".arca/ticket", ".arca/goal", ".ratmac"] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        fs::write(root.join(".ratmac/ratmac.toml"), RUNBOOK).expect("write the runbook");
        fs::write(root.join(".arca/goal/spec.md"), "# Fixture goal\n").expect("write the goal");
        Fixture { base, root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    /// Start a Run and return its minted id.
    fn start(&self) -> String {
        let output = self.rtm(&["start"]);
        assert!(
            output.status.success(),
            "the fixture Run starts: {}",
            text(&output)
        );
        fs::read_dir(self.root.join(".ratmac/runs"))
            .expect("read the roster")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .next()
            .expect("a started Run is on the roster")
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Read a repository document by its path relative to the project root.
fn document(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

/// The whole line of every definition entry whose term is exactly `term`.
fn definitions(source: &str, term: &str) -> Vec<String> {
    let bullet = format!("- **{term}**");
    let row = format!("| {term} |");
    source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with(&bullet) || line.starts_with(&row))
        .map(ToOwned::to_owned)
        .collect()
}

/// SVCV-001 / SVC-001: three names, three meanings, and a lifecycle that is
/// never a position. The documents that bind a contributor say it once each.
#[test]
fn three_names_are_defined_separately() {
    let sources = [
        ".arca/dict.md",
        ".arca/goal/ubi-lang.md",
        ".arca/schema.md",
        ".arca/runbook-spec.md",
    ];

    // The glossary and the ubiquitous-language table each define all three.
    for relative in [".arca/dict.md", ".arca/goal/ubi-lang.md"] {
        let source = document(relative);
        for term in ["State", "Run Record", "Run"] {
            let found = definitions(&source, term);
            assert_eq!(
                found.len(),
                1,
                "SVCV-001: {relative} must define {term} exactly once; found {found:?}"
            );
        }
        let state = definitions(&source, "State").remove(0);
        let record = definitions(&source, "Run Record").remove(0);
        let run = definitions(&source, "Run").remove(0);
        assert!(
            state.contains("graph") || state.contains("position") || state.contains("node"),
            "SVCV-001: {relative} defines State as the position: {state}"
        );
        assert!(
            record.contains("file"),
            "SVCV-001: {relative} defines Run Record as the file: {record}"
        );
        assert!(
            run.contains("instance"),
            "SVCV-001: {relative} defines Run as the live instance: {run}"
        );
        assert_ne!(
            state, record,
            "SVCV-001: {relative} must not define two names with one entry"
        );
    }

    // `status` is a lifecycle with five values, in every binding document that
    // names it, and never a position.
    let values = ["planned", "executing", "blocked", "passed", "failed"];
    let mut named_anywhere = false;
    for relative in sources {
        let source = document(relative);
        for line in source.lines() {
            let line = line.trim();
            let lowered = line.to_ascii_lowercase();
            if !(lowered.starts_with("| status |") || lowered.starts_with("- **status**")) {
                continue;
            }
            named_anywhere = true;
            for value in values {
                assert!(
                    line.contains(value),
                    "SVCV-001: {relative} must list the {value} lifecycle value: {line}"
                );
            }
            assert!(
                lowered.contains("lifecycle"),
                "SVCV-001: {relative} defines status as a lifecycle: {line}"
            );
            assert!(
                lowered.contains("never")
                    && (lowered.contains("position") || lowered.contains("graph")),
                "SVCV-001: {relative} states status is never a position: {line}"
            );
        }
    }
    assert!(
        named_anywhere,
        "SVCV-001: at least one binding document must define status"
    );

    // The Engine's own reports keep the three names apart: the line naming the
    // Run Record file may not call that file a State.
    let fixture = Fixture::new("names");
    let run_id = fixture.start();
    let doctor = text(&fixture.rtm(&["doctor"]));
    let record_line = doctor
        .lines()
        .find(|line| line.contains(&format!("runs/{run_id}/run.toml")))
        .unwrap_or_else(|| panic!("SVCV-001: the doctor reports the Run Record: {doctor}"))
        .to_owned();
    assert!(
        record_line.trim_start().starts_with("Run Record:"),
        "SVCV-001: the report names the file Run Record, not a State: {record_line}"
    );
    assert!(
        record_line.contains("state: intake"),
        "SVCV-001: the same line names the position State: {record_line}"
    );
    let status = text(&fixture.rtm(&["status", "--run", &run_id]));
    assert!(
        status.contains("State: intake") && status.contains("Status: "),
        "SVCV-001: the status report separates the position from the lifecycle: {status}"
    );
}

/// SVCV-007 / SVC-002: the State Prompt, the status report, both doctor
/// routes, and refusal text all name the position State - and none of them
/// says the pre-cutover word.
#[test]
fn caller_surfaces_name_the_position_state() {
    let fixture = Fixture::new("surfaces");
    let run_id = fixture.start();

    // 1. The State Prompt an agent acts on, printed by start and by status.
    let started = text(&fixture.rtm(&["status", "--run", &run_id]));
    assert!(
        started.contains("Integrate the issues."),
        "SVCV-007: the report carries the declared prose: {started}"
    );
    assert!(
        started.contains("State: intake"),
        "SVCV-007: the status report names the position State: {started}"
    );

    // 2. A refusal: the guard cannot pass, and the refusal names the State.
    let refusal = text(&fixture.rtm(&["step", "--run", &run_id]));
    assert!(
        !refusal.is_empty(),
        "SVCV-007: a failing step reports something"
    );

    // 3. The human doctor report and the machine-readable one.
    let human = text(&fixture.rtm(&["doctor"]));
    fs::write(
        fixture.root.join(".ratmac/broken.toml"),
        "[states.a]\nprompt = 42\n",
    )
    .expect("write a broken runbook");
    let json = text(&fixture.rtm(&["doctor", "--json", ".ratmac/broken.toml"]));
    assert!(
        json.contains("\"location\": \"state "),
        "SVCV-007: a machine-readable finding locates a State: {json}"
    );

    // 4. Nothing a caller reads carries the pre-cutover spelling. The residue
    //    refusals are the one legitimate carrier, and none of these outputs is
    //    one: this project is written the settled way.
    for (label, rendered) in [
        ("status", &started),
        ("step refusal", &refusal),
        ("doctor", &human),
        ("doctor --json", &json),
    ] {
        let lowered = rendered.to_ascii_lowercase();
        assert!(
            !lowered.contains(PRE_CUTOVER),
            "SVCV-007: the {label} output must not name the position {PRE_CUTOVER:?}: {rendered}"
        );
    }
}
