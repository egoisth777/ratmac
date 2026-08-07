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

[classes.reviewer.phases.delegate]
prompt = "Review the delegated ticket."

[classes.reviewer.phases.done]
prompt = "Finish delegated review."

[classes.reviewer.phases.blocked]
prompt = "Record a delegated blocker."

[[classes.reviewer.transitions]]
from = "delegate"
to = "done"

[[classes.reviewer.transitions]]
from = "done"
to = "blocked"

[[classes.reviewer.transitions]]
from = "delegate"
to = "blocked"
blocked-route = true

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

struct LinkedFixture {
    primary: Fixture,
    linked: PathBuf,
}

impl LinkedFixture {
    fn new(label: &str) -> Self {
        let primary = Fixture::with_lifecycle_context(label);
        git_success(&primary.root, &["init"]);
        git_success(&primary.root, &["config", "core.autocrlf", "false"]);
        git_success(
            &primary.root,
            &["config", "user.email", "qa@example.invalid"],
        );
        git_success(&primary.root, &["config", "user.name", "Ratmac QA"]);
        git_success(&primary.root, &["add", "--all"]);
        git_success(&primary.root, &["commit", "-m", "fixture base"]);

        let linked = primary.root.with_file_name(format!(
            "{}-linked",
            primary
                .root
                .file_name()
                .expect("fixture root has a file name")
                .to_string_lossy()
        ));
        let output = Command::new("git")
            .args(["worktree", "add", "-b", "t077-linked"])
            .arg(&linked)
            .current_dir(&primary.root)
            .output()
            .expect("git worktree add is executable");
        assert!(
            output.status.success(),
            "create linked fixture worktree: {}",
            combined(&output)
        );
        Self { primary, linked }
    }

    fn rtm_at(&self, root: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .args(args)
            .current_dir(root)
            .output()
            .expect("invoke compiled rtm binary")
    }

    fn spawn_child(&self, parent: &str) -> String {
        let workspace = self.linked.to_string_lossy().into_owned();
        let spawned = self.primary.rtm(&[
            "spawn",
            "review",
            "--run",
            parent,
            "--bind",
            "ticket=t-900",
            "--workspace",
            workspace.as_str(),
        ]);
        assert!(
            spawned.status.success(),
            "fixture setup must spawn a linked-workspace child: {}",
            combined(&spawned)
        );
        let mut ids = fs::read_dir(self.primary.path(".ratmac/runs"))
            .expect("fixture Engine roster is listable")
            .map(|entry| entry.expect("fixture roster entry is readable"))
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        ids.sort();
        ids.into_iter()
            .find(|id| id != parent)
            .expect("spawn must mint one child distinct from its parent")
    }
}

impl Drop for LinkedFixture {
    fn drop(&mut self) {
        let _ = Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&self.linked)
            .current_dir(&self.primary.root)
            .output();
        let _ = fs::remove_dir_all(&self.linked);
    }
}

fn git_success(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("Git is executable");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        combined(&output)
    );
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
    plant_residue_at(&fixture.root, artifact);
}

fn plant_residue_at(root: &Path, artifact: &str) {
    let write = |relative: &str, contents: &str| {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("residue path has a parent"))
            .expect("create residue parent");
        fs::write(path, contents).expect("plant residue");
    };
    match artifact {
        ".arca/ratmac.toml" => write(artifact, LIFECYCLE_RUNBOOK),
        ".arca/runs" => {
            write(
                ".arca/runs/r000001/state.toml",
                "phase = \"intake\"\nstatus = \"planned\"\n",
            );
        }
        ".arca/rtm.lock" => write(artifact, "legacy lock holder\n"),
        ".arca/state.toml" => write(artifact, "phase = \"delegate\"\nstatus = \"executing\"\n"),
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

/// ENSV-010: direct scaffolding treats an Engine-directory target as owned by
/// the project above it, so residue wins before the existing-file check.
#[test]
fn direct_scaffold_preflights_project_above_engine_directory() {
    let fixture = Fixture::new("direct-scaffold-parent", EVIDENCE_CONTROL_RUNBOOK);
    plant_residue(&fixture, ".arca/runs");
    let target = fixture.path(".ratmac/ratmac.toml");
    let before = tree_snapshot(&fixture.root);

    let refusal = ratmac::scaffold::write_scaffold(&target)
        .expect_err("project-root residue must refuse direct scaffold");
    let text = refusal.to_string().replace('\\', "/").to_ascii_lowercase();
    assert!(
        text.contains(".arca/runs"),
        "ENS-009: direct scaffold must name the project-root residue: {text}"
    );
    assert!(
        ["remove", "delete", "migrate", "move"]
            .iter()
            .any(|verb| text.contains(*verb)),
        "ENS-009: direct scaffold must instruct repair: {text}"
    );
    assert_eq!(
        tree_snapshot(&fixture.root),
        before,
        "ENS-009: direct scaffold must not change the project"
    );
}

/// ENSV-010: doctor must preflight the project owning an addressed external
/// legacy runbook before it diagnoses that path.
#[test]
fn doctor_preflights_addressed_external_project() {
    let invoking = Fixture::new("external-doctor-invoker", EVIDENCE_CONTROL_RUNBOOK);
    let external = Fixture::new("external-doctor-target", EVIDENCE_CONTROL_RUNBOOK);
    plant_residue(&external, ".arca/ratmac.toml");
    let target = external.path(".arca/ratmac.toml");
    let invoking_before = tree_snapshot(&invoking.root);
    let external_before = tree_snapshot(&external.root);

    let doctor = invoking.rtm(&[
        "doctor",
        "--json",
        target
            .to_str()
            .expect("temporary fixture target is valid UTF-8"),
    ]);
    let text = combined(&doctor);
    assert!(
        !doctor.status.success(),
        "ENS-009: doctor must refuse external project residue: {text}"
    );
    let normalized = text.replace('\\', "/").to_ascii_lowercase();
    assert!(
        normalized.contains(".arca/ratmac.toml"),
        "ENS-009: doctor must name external project residue: {text}"
    );
    assert!(
        ["remove", "delete", "migrate", "move"]
            .iter()
            .any(|verb| normalized.contains(*verb)),
        "ENS-009: doctor must instruct external project repair: {text}"
    );
    assert!(
        !text.contains("\"findings\""),
        "ENS-009: doctor must refuse before emitting diagnosis findings: {text}"
    );
    assert_eq!(
        tree_snapshot(&invoking.root),
        invoking_before,
        "ENS-009: external doctor refusal must not change the invoking project"
    );
    assert_eq!(
        tree_snapshot(&external.root),
        external_before,
        "ENS-009: external doctor refusal must not change the addressed project"
    );
}

fn refusal_text<T, E: std::fmt::Display>(result: Result<T, E>, entry_point: &str) -> String {
    match result {
        Err(error) => error.to_string(),
        Ok(_) => panic!("ENS-009: {entry_point} unexpectedly succeeded"),
    }
}

fn assert_named_residue_refusal(entry_point: &str, artifact: &str, text: &str) {
    let normalized = text.replace('\\', "/").to_ascii_lowercase();
    assert!(
        normalized.contains("pre-split engine residue"),
        "ENS-009: {entry_point} must identify the canonical pre-split Engine residue refusal: {text}"
    );
    assert!(
        normalized.contains(&artifact.to_ascii_lowercase()),
        "ENS-009: {entry_point} must name {artifact}: {text}"
    );
    assert!(
        ["remove", "delete", "migrate", "move"]
            .iter()
            .any(|verb| normalized.contains(*verb)),
        "ENS-009: {entry_point} must instruct repair: {text}"
    );
    assert!(
        normalized.contains("nothing was modified"),
        "ENS-009: {entry_point} must state its no-mutation refusal: {text}"
    );
}

#[derive(Clone, Copy)]
enum AddressedEntryPoint {
    Start,
    InitializeState,
    RecordMissingPrerequisite,
    LoadState,
    Status,
    Step,
    Hold,
    Abandon,
    Spawn,
    Respawn,
    Doctor,
    Scaffold,
    Roster,
    Diagnose,
}

/// ENSV-010: this is the public-surface oracle for child-workspace addressing.
/// Each row prepares a valid boundary first, then plants every live residue
/// artifact in the project that the boundary actually addresses.
#[test]
fn every_public_entry_point_preflights_its_addressed_project() {
    const ARTIFACTS: [(&str, &str); 4] = [
        ("legacy-runbook", ".arca/ratmac.toml"),
        ("legacy-runs", ".arca/runs"),
        ("legacy-lock", ".arca/rtm.lock"),
        ("legacy-state", ".arca/state.toml"),
    ];

    for (entry_point, entry) in [
        ("start", AddressedEntryPoint::Start),
        ("initialize_state", AddressedEntryPoint::InitializeState),
        (
            "record_missing_prerequisite",
            AddressedEntryPoint::RecordMissingPrerequisite,
        ),
        ("load_state", AddressedEntryPoint::LoadState),
        ("status", AddressedEntryPoint::Status),
        ("step", AddressedEntryPoint::Step),
        ("hold", AddressedEntryPoint::Hold),
        ("abandon", AddressedEntryPoint::Abandon),
        ("spawn", AddressedEntryPoint::Spawn),
        ("respawn", AddressedEntryPoint::Respawn),
        ("doctor", AddressedEntryPoint::Doctor),
        ("scaffold", AddressedEntryPoint::Scaffold),
        ("roster", AddressedEntryPoint::Roster),
        ("diagnose", AddressedEntryPoint::Diagnose),
    ] {
        for (artifact_label, artifact) in ARTIFACTS {
            let fixture = LinkedFixture::new(&format!("addressed-{entry_point}-{artifact_label}"));
            let parent = fixture.primary.start_at_delegate();
            let child = fixture.spawn_child(&parent);
            let mut scheduler = match entry {
                AddressedEntryPoint::Start => Some(
                    ratmac::Scheduler::open(&fixture.linked)
                        .expect("fixture start scheduler opens before planting residue"),
                ),
                AddressedEntryPoint::InitializeState
                | AddressedEntryPoint::RecordMissingPrerequisite
                | AddressedEntryPoint::LoadState
                | AddressedEntryPoint::Status
                | AddressedEntryPoint::Step => Some(
                    ratmac::Scheduler::open_run(&fixture.primary.root, &child)
                        .expect("fixture child opens before planting residue"),
                ),
                _ => None,
            };
            let state = match entry {
                AddressedEntryPoint::InitializeState
                | AddressedEntryPoint::RecordMissingPrerequisite => Some(
                    scheduler
                        .as_ref()
                        .expect("state operation has an opened Scheduler")
                        .load_state()
                        .expect("fixture child State File loads before planting residue"),
                ),
                _ => None,
            };
            let hold_plan = match entry {
                AddressedEntryPoint::Hold => {
                    let request = ratmac::blocked::HoldRequest {
                        ticket: TICKET.to_owned(),
                        blocker: Some(BLOCKER.to_owned()),
                        confirmation: Some(format!("hold {TICKET}")),
                        run: Some(child.clone()),
                    };
                    Some(
                        ratmac::blocked::plan_hold(&fixture.primary.root, &request)
                            .expect("fixture hold plan succeeds before planting residue"),
                    )
                }
                _ => None,
            };
            let abandon_plan = match entry {
                AddressedEntryPoint::Abandon => {
                    let request = ratmac::abandon::AbandonRequest {
                        confirmation: Some(format!("abandon {child}")),
                        run: Some(child.clone()),
                    };
                    Some(
                        ratmac::abandon::plan_abandon(&fixture.primary.root, &request)
                            .expect("fixture abandon plan succeeds before planting residue"),
                    )
                }
                _ => None,
            };

            plant_residue_at(&fixture.linked, artifact);
            let primary_before = tree_snapshot(&fixture.primary.root);
            let linked_before = tree_snapshot(&fixture.linked);
            let mint_before = match entry {
                AddressedEntryPoint::Respawn => Some(
                    fs::read(fixture.primary.path(".ratmac/mint.toml"))
                        .expect("fixture Engine has its durable mint counter"),
                ),
                _ => None,
            };

            let text = match entry {
                AddressedEntryPoint::Start => refusal_text(
                    scheduler
                        .as_mut()
                        .expect("start has an opened Scheduler")
                        .start(),
                    entry_point,
                ),
                AddressedEntryPoint::InitializeState => refusal_text(
                    scheduler
                        .as_mut()
                        .expect("initialize_state has an opened Scheduler")
                        .initialize_state(state.expect("initialize_state has fixture State")),
                    entry_point,
                ),
                AddressedEntryPoint::RecordMissingPrerequisite => refusal_text(
                    scheduler
                        .as_mut()
                        .expect("record_missing_prerequisite has an opened Scheduler")
                        .record_missing_prerequisite(
                            state.expect("record_missing_prerequisite has fixture State"),
                            "fixture prerequisite",
                        ),
                    entry_point,
                ),
                AddressedEntryPoint::LoadState => refusal_text(
                    scheduler
                        .as_ref()
                        .expect("load_state has an opened Scheduler")
                        .load_state(),
                    entry_point,
                ),
                AddressedEntryPoint::Status => refusal_text(
                    scheduler
                        .as_ref()
                        .expect("status has an opened Scheduler")
                        .status(),
                    entry_point,
                ),
                AddressedEntryPoint::Step => refusal_text(
                    scheduler
                        .as_mut()
                        .expect("step has an opened Scheduler")
                        .step(ratmac::StepRequest::new("")),
                    entry_point,
                ),
                AddressedEntryPoint::Hold => refusal_text(
                    ratmac::blocked::apply_hold(
                        &fixture.primary.root,
                        hold_plan.as_ref().expect("hold has a clean plan"),
                    ),
                    entry_point,
                ),
                AddressedEntryPoint::Abandon => refusal_text(
                    ratmac::abandon::apply_abandon(
                        &fixture.primary.root,
                        abandon_plan.as_ref().expect("abandon has a clean plan"),
                    ),
                    entry_point,
                ),
                AddressedEntryPoint::Spawn => {
                    let mut bindings = BTreeMap::new();
                    bindings.insert("ticket".to_owned(), "t-900".to_owned());
                    refusal_text(
                        ratmac::Scheduler::spawn_to_with_workspace(
                            &fixture.primary.root,
                            &parent,
                            "review",
                            &bindings,
                            Some(&fixture.linked),
                        ),
                        entry_point,
                    )
                }
                AddressedEntryPoint::Respawn => {
                    let request = ratmac::RespawnRequest {
                        run: Some(child.clone()),
                        confirmation: Some(format!("respawn {child}")),
                    };
                    refusal_text(
                        ratmac::Scheduler::respawn(&fixture.primary.root, &request),
                        entry_point,
                    )
                }
                AddressedEntryPoint::Doctor => {
                    let target = fixture.linked.join(".ratmac/ratmac.toml");
                    let output = fixture.rtm_at(
                        &fixture.primary.root,
                        &[
                            "doctor",
                            "--json",
                            target
                                .to_str()
                                .expect("temporary linked target is valid UTF-8"),
                        ],
                    );
                    let text = combined(&output);
                    assert!(
                        !output.status.success(),
                        "ENS-009: doctor unexpectedly succeeded: {text}"
                    );
                    assert!(
                        !text.contains("\"findings\""),
                        "ENS-009: CLI doctor must refuse before rendering a diagnosis report: {text}"
                    );
                    text
                }
                AddressedEntryPoint::Scaffold => refusal_text(
                    ratmac::scaffold::write_scaffold(&fixture.linked.join(".ratmac/scaffold.toml")),
                    entry_point,
                ),
                AddressedEntryPoint::Roster => {
                    refusal_text(ratmac::Scheduler::run_roster(&fixture.linked), entry_point)
                }
                AddressedEntryPoint::Diagnose => {
                    let findings =
                        ratmac::doctor::diagnose(&fixture.linked.join(".ratmac/ratmac.toml"));
                    assert_eq!(
                        findings.len(),
                        1,
                        "ENS-009: direct diagnose must stop at one residue finding: {findings:?}"
                    );
                    assert_eq!(
                        findings[0].severity(),
                        ratmac::doctor::Severity::Error,
                        "ENS-009: direct diagnose must report an error finding"
                    );
                    findings[0].to_string()
                }
            };
            assert_named_residue_refusal(entry_point, artifact, &text);
            // The whole-tree comparison below covers mint.toml; retain this
            // narrower respawn check for failure locality, not additional coverage.
            if let Some(mint_before) = mint_before {
                assert_eq!(
                    fs::read(fixture.primary.path(".ratmac/mint.toml"))
                        .expect("refused respawn leaves the durable mint counter readable"),
                    mint_before,
                    "ENS-009: refused respawn must not advance the Engine mint counter"
                );
            }
            assert_eq!(
                tree_snapshot(&fixture.primary.root),
                primary_before,
                "ENS-009: {entry_point} must not mutate shared primary runtime"
            );
            assert_eq!(
                tree_snapshot(&fixture.linked),
                linked_before,
                "ENS-009: {entry_point} must not mutate its addressed workspace"
            );
        }
    }
}

/// ENSV-010: a linked invocation owns tracked files in its checkout, but the
/// shared Engine belongs to the primary checkout. Primary-only residue must
/// therefore refuse before any linked invocation can read or mint runtime.
#[test]
fn linked_worktree_refuses_each_primary_only_residue() {
    for (label, artifact) in [
        ("primary-legacy-runbook", ".arca/ratmac.toml"),
        ("primary-legacy-runs", ".arca/runs"),
        ("primary-legacy-lock", ".arca/rtm.lock"),
        ("primary-legacy-state", ".arca/state.toml"),
    ] {
        let fixture = LinkedFixture::new(label);
        plant_residue_at(&fixture.primary.root, artifact);
        let primary_before = tree_snapshot(&fixture.primary.root);
        let linked_before = tree_snapshot(&fixture.linked);

        let refused = fixture.rtm_at(&fixture.linked, &["start"]);
        let text = combined(&refused);
        assert!(
            !refused.status.success(),
            "ENS-009: linked start must refuse primary-only {artifact}: {text}"
        );
        assert_named_residue_refusal("linked start", artifact, &text);
        let primary_path = fixture
            .primary
            .path(artifact)
            .to_string_lossy()
            .replace('\\', "/")
            .to_ascii_lowercase();
        assert!(
            text.replace('\\', "/")
                .to_ascii_lowercase()
                .contains(&primary_path),
            "ENS-009: linked refusal must name the primary artifact path: {text}"
        );
        assert_eq!(
            tree_snapshot(&fixture.primary.root),
            primary_before,
            "ENS-009: linked refusal must not mutate primary runtime"
        );
        assert_eq!(
            tree_snapshot(&fixture.linked),
            linked_before,
            "ENS-009: linked refusal must not mutate invoking checkout"
        );
    }
}
