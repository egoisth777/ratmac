//! t-077 / ENS-009: pre-split Engine residue refuses without migration.
//!
//! ENSV-010 `every_entry_point_refuses_each_presplit_artifact_without_mutation`
//!
//! Each live artifact from the pre-split Engine namespace is planted by itself
//! after a route-valid Run is prepared. Every public Engine entry point must
//! refuse before it can adopt, migrate, or otherwise change that artifact.
//! Archived evidence is the deliberate control: it is history, not runtime.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const TICKET: &str = "t-900";
const BLOCKER: &str = ".arca/issue/i-900-blocker";
const LIFECYCLE_RUNBOOK: &str = r#"
[roots]
ticket = ".arca/ticket"

[classes.reviewer.bindings.ticket]
required = true

[classes.reviewer.phases.review]
prompt = "Review the delegated ticket."

[phases.intake]
prompt = "Integrate the ticket."

[phases.delegate]
prompt = "Delegate the ticket."

[[phases.delegate.spawns]]
class = "reviewer"
name = "review"
bind = ["ticket"]

[phases.done]
prompt = "Finish the ticket."

[[transitions]]
from = "intake"
to = "delegate"

[[transitions]]
from = "delegate"
to = "done"

[[transitions]]
from = "delegate"
to = "intake"
blocked-route = true
"#;
const EVIDENCE_CONTROL_RUNBOOK: &str = r#"
[phases.intake]
prompt = "Inspect archived evidence."

[phases.done]
prompt = "Complete normally."

[[transitions]]
from = "intake"
to = "done"
"#;
const ISSUE_FILES: [&str; 5] = [
    "index.md",
    "spec.md",
    "design.md",
    "test-plan.md",
    "ubi-lang.md",
];

type TreeSnapshot = BTreeMap<String, Option<Vec<u8>>>;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(label: &str, runbook: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t077-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock is after the Unix epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temporary fixture root");

        let fixture = Self { root };
        fixture.write("src/lib.rs", "pub fn fixture_marker() {}\n");
        fixture.write(".ratmac/ratmac.toml", runbook);
        fixture
    }

    fn with_lifecycle_context(label: &str) -> Self {
        let fixture = Self::new(label, LIFECYCLE_RUNBOOK);
        fixture.seed_hold_context();
        fixture
    }

    fn evidence_only(label: &str) -> Self {
        Self::new(label, EVIDENCE_CONTROL_RUNBOOK)
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.path(relative);
        let parent = path.parent().expect("fixture path has a parent");
        fs::create_dir_all(parent).expect("create fixture parent directory");
        fs::write(path, content).expect("write fixture file");
    }

    fn seed_hold_context(&self) {
        let ticket = format!(".arca/ticket/{TICKET}.md");
        self.write(
            &ticket,
            "---\nticket-id: \"t-900\"\nresidual-ids:\n  - \"res-900\"\n\
             planned-test-refs:\n  - \"ENSV-010\"\nstatus: \"executing\"\n---\n\n\
             # Ticket: t-900\n",
        );
        self.write(
            ".arca/residual/res-900.md",
            "# Residual Record\n\n```yaml\nresidual-id: \"res-900\"\nstatus: \"missing\"\n```\n",
        );
        for name in ISSUE_FILES {
            self.write(
                &format!("{BLOCKER}/{name}"),
                &format!("# {name}\n\nFixture blocker record.\n"),
            );
        }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke compiled rtm binary")
    }

    fn only_run(&self) -> String {
        let runs = self.path(".ratmac/runs");
        let mut ids = fs::read_dir(&runs)
            .expect("Engine roster is listable")
            .map(|entry| entry.expect("Engine roster entry is readable"))
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        ids.sort();
        assert_eq!(
            ids.len(),
            1,
            "fixture setup must mint exactly one Run; roster was {ids:?}"
        );
        ids.pop().expect("one minted Run has an id")
    }

    fn start_at_delegate(&self) -> String {
        let start = self.rtm(&["start"]);
        assert!(
            start.status.success(),
            "fixture setup must start normally: {}",
            combined(&start)
        );
        let run = self.only_run();
        let step = self.rtm(&["step", "--run", run.as_str()]);
        assert!(
            step.status.success(),
            "fixture setup must reach the spawn and hold phase: {}",
            combined(&step)
        );
        let state = fs::read_to_string(self.path(&format!(".ratmac/runs/{run}/state.toml")))
            .expect("read fixture Run State File");
        assert!(
            state.contains("phase = \"delegate\""),
            "fixture setup must leave the Run at the spawn and hold phase: {state}"
        );
        run
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

/// Every file and directory below `root`, preserving exact file bytes.
/// Directory entries make creation or deletion of an empty directory observable.
fn tree_snapshot(root: &Path) -> TreeSnapshot {
    fn walk(root: &Path, directory: &Path, snapshot: &mut TreeSnapshot) {
        for entry in fs::read_dir(directory).expect("snapshot directory is listable") {
            let path = entry.expect("snapshot entry is readable").path();
            let relative = path
                .strip_prefix(root)
                .expect("snapshot entry remains below the root")
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                snapshot.insert(format!("{relative}/"), None);
                walk(root, &path, snapshot);
            } else {
                snapshot.insert(
                    relative,
                    Some(fs::read(path).expect("snapshot file is readable")),
                );
            }
        }
    }

    let mut snapshot = TreeSnapshot::new();
    walk(root, root, &mut snapshot);
    snapshot
}

fn plant_residue(fixture: &Fixture, artifact: &str) {
    match artifact {
        ".arca/ratmac.toml" => fixture.write(artifact, LIFECYCLE_RUNBOOK),
        ".arca/runs" => {
            fixture.write(
                ".arca/runs/r000001/state.toml",
                "phase = \"intake\"\nstatus = \"planned\"\n",
            );
        }
        ".arca/rtm.lock" => fixture.write(artifact, "legacy lock holder\n"),
        ".arca/state.toml" => {
            fixture.write(artifact, "phase = \"delegate\"\nstatus = \"executing\"\n")
        }
        _ => panic!("unrecognized ENS-009 residue fixture: {artifact}"),
    }
}

fn assert_residue_refusal(
    fixture: &Fixture,
    artifact: &str,
    entry_point: &str,
    args: &[&str],
    before: &TreeSnapshot,
) {
    let refused = fixture.rtm(args);
    let text = combined(&refused);
    assert!(
        !refused.status.success(),
        "ENS-009: `rtm {entry_point}` must refuse {artifact} before it can act; it exited successfully:\n{text}"
    );

    let normalized = text.replace('\\', "/").to_ascii_lowercase();
    assert!(
        normalized.contains(&artifact.to_ascii_lowercase()),
        "ENS-009: `rtm {entry_point}` must name the observed residue {artifact}: {text}"
    );
    assert!(
        ["remove", "delete", "migrate", "move"]
            .iter()
            .any(|verb| normalized.contains(*verb)),
        "ENS-009: `rtm {entry_point}` must tell the human how to repair {artifact}: {text}"
    );

    let after = tree_snapshot(&fixture.root);
    assert_eq!(
        &after, before,
        "ENS-009: refusing {artifact} through `rtm {entry_point}` must leave the whole project byte-identical"
    );
}

/// ENSV-010: every Engine route refuses each live pre-split artifact without
/// migrating or removing it; `.arca/evidence/` alone remains inert history.
#[test]
fn every_entry_point_refuses_each_presplit_artifact_without_mutation() {
    for (label, artifact) in [
        ("legacy-runbook", ".arca/ratmac.toml"),
        ("legacy-runs", ".arca/runs"),
        ("legacy-lock", ".arca/rtm.lock"),
        ("legacy-state", ".arca/state.toml"),
    ] {
        let fixture = Fixture::with_lifecycle_context(label);
        let run = fixture.start_at_delegate();
        plant_residue(&fixture, artifact);
        let before = tree_snapshot(&fixture.root);

        let abandon_confirmation = format!("abandon {run}");
        let respawn_confirmation = format!("respawn {run}");
        let start = ["start"];
        let status = ["status", "--run", run.as_str()];
        let step = ["step", "--run", run.as_str()];
        let hold = [
            "hold",
            TICKET,
            "--run",
            run.as_str(),
            "--blocker",
            BLOCKER,
            "--confirm",
            "hold t-900",
        ];
        let abandon = [
            "abandon",
            "--run",
            run.as_str(),
            "--confirm",
            abandon_confirmation.as_str(),
        ];
        let spawn = [
            "spawn",
            "review",
            "--run",
            run.as_str(),
            "--bind",
            "ticket=t-900",
        ];
        let respawn = [
            "respawn",
            "--run",
            run.as_str(),
            "--confirm",
            respawn_confirmation.as_str(),
        ];
        let doctor = ["doctor"];

        for (entry_point, args) in [
            ("start", start.as_slice()),
            ("status", status.as_slice()),
            ("step", step.as_slice()),
            ("hold", hold.as_slice()),
            ("abandon", abandon.as_slice()),
            ("spawn", spawn.as_slice()),
            ("respawn", respawn.as_slice()),
            ("doctor", doctor.as_slice()),
        ] {
            assert_residue_refusal(&fixture, artifact, entry_point, args, &before);
        }
    }

    let control = Fixture::evidence_only("archived-evidence");
    const RECEIPT: &str = "archived receipt; not live Engine state\n";
    control.write(".arca/evidence/archive-note.md", RECEIPT);
    let arca_before = tree_snapshot(&control.path(".arca"));

    let start = control.rtm(&["start"]);
    assert!(
        start.status.success(),
        "ENS-009: archived .arca/evidence alone must not block a normal start: {}",
        combined(&start)
    );
    let run = control.only_run();
    let status = control.rtm(&["status", "--run", run.as_str()]);
    assert!(
        status.status.success(),
        "ENS-009: archived .arca/evidence alone must leave normal status available: {}",
        combined(&status)
    );

    let arca_after = tree_snapshot(&control.path(".arca"));
    assert_eq!(
        &arca_after, &arca_before,
        "ENS-009: Engine operation must neither adopt nor remove archived .arca/evidence"
    );
    assert_eq!(
        fs::read_to_string(control.path(".arca/evidence/archive-note.md"))
            .expect("read archived evidence after normal operation"),
        RECEIPT,
        "ENS-009: archived evidence bytes remain historical data"
    );
    assert!(
        !control.path(".arca/runs").exists(),
        "ENS-009: archived evidence must not become a legacy Engine roster"
    );
}
