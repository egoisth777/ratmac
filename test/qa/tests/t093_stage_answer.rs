//! t-093 / PCR-002: the stage has one answer.
//!
//! PCRV-002 `the_live_run_names_the_stage_and_the_lookup_is_a_fallback`
//!
//! "Where are we" gets a single owner. The cycle has one entry and one end,
//! so a sprint is one Run; while that Run is live the addressed report names
//! its stage, and the tree-derived lookup a person performs by hand is
//! labelled a no-live-Run fallback wherever a live document still carries it.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use ratmac::machine::MachineClass;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The Machine Class this repository ships, as it stands on disk.
fn shipped_runbook() -> String {
    fs::read_to_string(repo_root().join(".ratmac/ratmac.toml"))
        .expect("read the shipped machine class")
}

/// The live documents a contributor reads for orientation. Archived records
/// are frozen provenance and are not rewritten by a later decision.
const LIVE_DOCUMENTS: [&str; 4] = [
    ".arca/index.md",
    ".arca/schema.md",
    ".arca/steering.md",
    "AGENTS.md",
];

/// The label that demotes the by-hand lookup to the window between sprints.
const FALLBACK_LABEL: &str = "no-live-Run fallback";

/// A temporary repository carrying the shipped runbook, enough for a Run of
/// the cycle to start and be addressed.
struct Sprint {
    root: PathBuf,
}

impl Drop for Sprint {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Sprint {
    fn create(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t093-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in [
            ".arca/goal",
            ".arca/issue/i-100-demo",
            ".arca/residual",
            ".arca/ticket/archive",
            ".ratmac",
            "src",
        ] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        let sprint = Self { root };
        sprint.write(".ratmac/ratmac.toml", &shipped_runbook());
        sprint.write("src/lib.rs", "pub fn work() {}\n");
        sprint.write(
            ".arca/goal/spec.md",
            "# Goal spec\n\n\
             | Req ID | Requirement | Source |\n|---|---|---|\n\
             | DEMO-001 | The demo behaves. | \
             [issue DEMO-001](../issue/i-100-demo/spec.md#requirement-records) |\n",
        );
        let issue = sprint.root.join(".arca/issue/i-100-demo");
        sprint.write(
            ".arca/issue/i-100-demo/index.md",
            "# Issue i-100-demo\n\n\
             ```yaml\nissue-id: \"i-100-demo\"\nstatus: \"integrated\"\n```\n\n\
             See [goal spec](../../goal/spec.md).\n",
        );
        sprint.write(
            ".arca/issue/i-100-demo/spec.md",
            "# Requirement records\n\n\
             | Req ID | Requirement | Status |\n|---|---|---|\n\
             | `DEMO-001` | The demo behaves. | accepted |\n",
        );
        for name in ["design.md", "test-plan.md", "ubi-lang.md"] {
            fs::write(issue.join(name), format!("# {name}\n")).expect("write issue file");
        }
        sprint
    }

    fn write(&self, relative: &str, body: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture directory");
        }
        fs::write(path, body).expect("write fixture file");
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    fn start(&self) -> String {
        let output = self.rtm(&["start"]);
        let text = combined(&output);
        assert!(output.status.success(), "start succeeds: {text}");
        text.split("started run ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .expect("start names the minted run id")
            .to_owned()
    }
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The stage names the shipped cycle declares.
fn stages() -> Vec<String> {
    MachineClass::from_toml(&shipped_runbook())
        .expect("the shipped machine class parses")
        .states()
        .keys()
        .cloned()
        .collect()
}

#[test]
fn the_live_run_names_the_stage_and_the_lookup_is_a_fallback() {
    // One entry and one end, so a sprint is one Run with one stage answer.
    let class = MachineClass::from_toml(&shipped_runbook()).expect("the shipped class parses");
    let ordinary: Vec<(String, String)> = class
        .transitions()
        .iter()
        .filter(|transition| !transition.is_blocked_route())
        .map(|transition| {
            (
                transition.from().as_str().to_owned(),
                transition.to().as_str().to_owned(),
            )
        })
        .collect();
    let initial: Vec<&String> = class
        .states()
        .keys()
        .filter(|name| !ordinary.iter().any(|(_, to)| to == *name))
        .collect();
    let terminal: Vec<&String> = class
        .states()
        .keys()
        .filter(|name| !ordinary.iter().any(|(from, _)| from == *name))
        .collect();
    assert_eq!(
        initial.len(),
        1,
        "PCR-002: a sprint has exactly one entry, so one Run answers for it"
    );
    assert_eq!(
        terminal,
        vec!["rest"],
        "PCR-002: a sprint has exactly one end, and it is the rest State"
    );

    // While the Run is live, the addressed report names its stage.
    let sprint = Sprint::create("live");
    let run = sprint.start();
    let report = combined(&sprint.rtm(&["status", "--run", &run]));
    assert!(
        report.contains(&format!("State: {}", initial[0])),
        "PCR-002: the addressed report names the Run's stage: {report}"
    );

    let stepped = combined(&sprint.rtm(&["step", "--run", &run]));
    assert!(!stepped.contains("step refused"), "the seed step passes");
    let report = combined(&sprint.rtm(&["status", "--run", &run]));
    assert!(
        report.contains("State: gap-check"),
        "PCR-002: the report follows the Run to its next stage: {report}"
    );

    // With no Run, the report names no stage - it offers no second answer of
    // its own.
    let idle = Sprint::create("idle");
    let report = combined(&idle.rtm(&["status"]));
    for stage in stages() {
        assert!(
            !report.contains(&format!("State: {stage}")),
            "PCR-002: with no Run the report invents no stage: {report}"
        );
    }

    // Every live document that still carries the by-hand lookup carries the
    // fallback label in the same place, and none offers it as the answer.
    let mut carriers = Vec::new();
    for document in LIVE_DOCUMENTS {
        let text = fs::read_to_string(repo_root().join(document))
            .unwrap_or_else(|error| panic!("read {document}: {error}"));
        if !text.contains("Where are we") {
            continue;
        }
        carriers.push(document);
        assert!(
            text.contains(FALLBACK_LABEL),
            "PCR-002: {document} answers \"where are we\" without labelling the \
             by-hand lookup a {FALLBACK_LABEL}"
        );
        assert!(
            text.contains("rtm status"),
            "PCR-002: {document} must name `rtm status` as the answer while a Run is live"
        );
        assert!(
            !text.contains("the tree is the oracle"),
            "PCR-002: {document} still presents the tree as the oracle"
        );
    }
    assert!(
        !carriers.is_empty(),
        "PCR-002: the demotion must be proven where the question is actually asked"
    );
}
