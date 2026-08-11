//! t-048 / PGE-006: the human-authorized blocked route.
//!
//! PT-048-01 `held_with_blocker_routes_onward`
//! PT-048-02 `unauthorized_or_unlinked_refuses`
//! HT-048-01 `unresolvable_blocker_refuses`
//! HT-048-02 `held_ticket_cannot_be_passed`
//! HT-048-03 `interrupted_hold_leaves_no_half_route`
//!
//! An honestly blocked ticket gets an honest exit: a human holds it against a
//! linked blocker record, the Run routes onward, and the ticket stays
//! not-passed with its residuals unproven. Everything else refuses without
//! touching Scheduler-owned files.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};
const TICKET: &str = "t-900";
const BLOCKER: &str = ".arca/issue/i-777-blocker";

struct Fixture {
    root: PathBuf,
    /// FDC-004: the started run's id, read off the plural roster.
    run_id: String,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = restore_writable(&self.root);
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    /// A started Run sitting in `build`, with one executing ticket, one
    /// unproven residual, and one complete blocker issue.
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t050-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in [".arca/ticket", ".arca/residual", BLOCKER, ".ratmac", "src"] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        fs::write(root.join("src/lib.rs"), "pub fn work() {}\n").expect("write source");
        for name in [
            "index.md",
            "spec.md",
            "design.md",
            "test-plan.md",
            "ubi-lang.md",
        ] {
            fs::write(
                root.join(BLOCKER).join(name),
                format!("# {name}\n\n```yaml\nstatus: \"pending\"\n```\n"),
            )
            .expect("write blocker issue file");
        }
        fs::write(
            root.join(".ratmac/ratmac.toml"),
            "[roots]\n\
             ticket = \".arca/ticket\"\n\n\
             [states.intake]\nprompt = \"Integrate the issues.\"\n\n\
             [states.build]\nprompt = \"Build the ticket.\"\n\n\
             [states.build-review]\nprompt = \"Review the ticket.\"\n\n\
             [[transitions]]\nfrom = \"intake\"\nto = \"build\"\n\n\
             [[transitions]]\nfrom = \"build\"\nto = \"intake\"\nblocked-route = true\n\n\
             [[transitions]]\nfrom = \"build\"\nto = \"build-review\"\n",
        )
        .expect("write machine class");
        fs::write(
            root.join(".arca/ticket/t-900.md"),
            "---\nticket-id: t-900\nresidual-ids:\n  - \"res-900\"\n\
             planned-test-refs:\n  - \"PT-900-01\"\nstatus: \"executing\"\n---\n\n\
             # Ticket: t-900\n\n## Merge Gate\n\n- Quality: `cargo --version` passes.\n",
        )
        .expect("write ticket");
        fs::write(
            root.join(".arca/residual/res-900.md"),
            "# Residual Record\n\n```yaml\nresidual-id: \"res-900\"\n\
             goal-requirement-ref: \"DEMO-001\"\nstatus: \"missing\"\n```\n",
        )
        .expect("write residual");

        let mut fixture = Fixture {
            root,
            run_id: String::new(),
        };
        assert!(
            fixture.rtm(&["start"]).status.success(),
            "the fixture Run starts"
        );
        // FDC-004: read the minted id off the Engine roster and address it.
        fixture.run_id = fs::read_dir(fixture.root.join(".ratmac/runs"))
            .expect("list the runs roster")
            .map(|entry| entry.expect("roster entry is readable"))
            .find(|entry| entry.path().is_dir())
            .expect("the started run appears on the roster")
            .file_name()
            .to_string_lossy()
            .into_owned();
        let id = fixture.run_id.clone();
        assert!(
            fixture.rtm(&["step", "--run", &id]).status.success(),
            "the Run reaches build"
        );
        assert_eq!(fixture.state(), "build", "the Run is executing a ticket");
        fixture
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    fn text(&self, args: &[&str]) -> String {
        let output = self.rtm(args);
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }

    fn record_path(&self) -> PathBuf {
        self.root
            .join(".ratmac/runs")
            .join(&self.run_id)
            .join("run.toml")
    }

    fn state(&self) -> String {
        let state = fs::read_to_string(self.record_path()).expect("read state");
        state
            .lines()
            .find_map(|line| line.trim().strip_prefix("state = "))
            .map(|value| value.trim().trim_matches('"').to_owned())
            .expect("state records a state")
    }

    fn ticket(&self) -> String {
        fs::read_to_string(self.root.join(".arca/ticket/t-900.md")).expect("read ticket")
    }

    fn residual(&self) -> String {
        fs::read_to_string(self.root.join(".arca/residual/res-900.md")).expect("read residual")
    }

    /// The bytes of every Scheduler-owned file, so a refusal can be proven
    /// to have written nothing.
    fn owned_bytes(&self) -> Vec<(String, Vec<u8>)> {
        // FDC-004: state and evidence reside in the Engine's run directory.
        let run = format!(".ratmac/runs/{}", self.run_id);
        [
            format!("{run}/run.toml"),
            ".ratmac/log.md".to_owned(),
            format!("{run}/evidence.toml"),
        ]
        .iter()
        .map(|relative| {
            (
                relative.clone(),
                fs::read(self.root.join(relative)).unwrap_or_default(),
            )
        })
        .collect()
    }

    fn hold(&self, args: &[&str]) -> Output {
        // FDC-004: hold acts on an existing Run — always addressed.
        let mut all = vec!["hold"];
        all.extend_from_slice(args);
        all.push("--run");
        all.push(&self.run_id);
        self.rtm(&all)
    }

    fn hold_text(&self, args: &[&str]) -> String {
        let output = self.hold(args);
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

fn restore_writable(root: &Path) -> std::io::Result<()> {
    let mut paths = vec![
        root.join(".arca/ticket/t-900.md"),
        root.join(".ratmac/log.md"),
    ];
    // FDC-004: each run carries its own State File.
    if let Ok(entries) = fs::read_dir(root.join(".ratmac/runs")) {
        for entry in entries.flatten() {
            paths.push(entry.path().join("run.toml"));
        }
    }
    for path in paths {
        if let Ok(metadata) = fs::metadata(&path) {
            let mut permissions = metadata.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            let _ = fs::set_permissions(&path, permissions);
        }
    }
    Ok(())
}

fn set_readonly(path: &Path, readonly: bool) {
    let metadata = fs::metadata(path).expect("read metadata");
    let mut permissions = metadata.permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(readonly);
    fs::set_permissions(path, permissions).expect("set permissions");
}

fn wait_for_file(path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if path.is_file() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

/// PT-048-01: an authorized, linked hold routes the Run onward.
#[test]
fn held_with_blocker_routes_onward() {
    let fixture = Fixture::new("routes");
    let log_before = fs::read_to_string(fixture.root.join(".ratmac/log.md")).expect("read log");
    let output = fixture.hold(&[TICKET, "--blocker", BLOCKER, "--confirm", "hold t-900"]);
    assert!(
        output.status.success(),
        "an authorized hold succeeds: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        fixture.state(),
        "intake",
        "the Run routes to the declared blocked-route destination"
    );
    let ticket = fixture.ticket();
    assert!(
        ticket.contains("status: \"held\""),
        "the ticket is held, not passed: {ticket}"
    );
    assert!(
        ticket.contains(&format!("blocker-ref: \"{BLOCKER}\"")),
        "the ticket records what blocks it: {ticket}"
    );
    assert!(
        fixture.residual().contains("status: \"missing\""),
        "the residual stays unproven: {}",
        fixture.residual()
    );

    let log = fs::read_to_string(fixture.root.join(".ratmac/log.md")).expect("read log");
    assert!(
        log.starts_with(&log_before),
        "history is appended, never rewritten"
    );
    let appended = &log[log_before.len()..];
    assert!(
        appended.contains(TICKET) && appended.contains(BLOCKER) && appended.contains("intake"),
        "the log records the hold, its blocker, and where the Run routed: {appended}"
    );

    // The Run really is at intake: its prompt is the intake prompt.
    assert!(
        fixture
            .text(&["status", "--run", &fixture.run_id])
            .contains("Integrate the issues."),
        "the routed Run prompts for intake"
    );
}

/// PT-048-02: without authorization or without a link, nothing moves.
#[test]
fn unauthorized_or_unlinked_refuses() {
    // No confirmation phrase at all.
    let fixture = Fixture::new("unauthorized");
    let before = fixture.owned_bytes();
    let refusal = fixture.hold_text(&[TICKET, "--blocker", BLOCKER]);
    assert!(
        refusal.to_ascii_lowercase().contains("confirm"),
        "the refusal says the human authorization is missing: {refusal}"
    );
    assert_eq!(fixture.state(), "build", "the Run did not route");
    assert!(
        fixture.ticket().contains("status: \"executing\""),
        "the ticket was not held"
    );
    assert_eq!(
        before,
        fixture.owned_bytes(),
        "Scheduler-owned files are byte-identical across the refusal"
    );

    // A confirmation phrase for a different ticket is not authorization.
    let refusal = fixture.hold_text(&[TICKET, "--blocker", BLOCKER, "--confirm", "hold t-901"]);
    assert!(
        refusal.contains("hold t-900"),
        "the refusal states the phrase it required: {refusal}"
    );
    assert_eq!(
        before,
        fixture.owned_bytes(),
        "a wrong phrase writes nothing either"
    );

    // Authorized, but with no blocker link.
    let fixture = Fixture::new("unlinked");
    let before = fixture.owned_bytes();
    let refusal = fixture.hold_text(&[TICKET, "--confirm", "hold t-900"]);
    assert!(
        refusal.to_ascii_lowercase().contains("blocker"),
        "the refusal says the blocker link is missing: {refusal}"
    );
    assert_eq!(fixture.state(), "build", "the Run did not route");
    assert_eq!(
        before,
        fixture.owned_bytes(),
        "Scheduler-owned files are byte-identical across the refusal"
    );
}

/// HT-048-01 (Input/Routing): a blocker that does not resolve is named.
#[test]
fn unresolvable_blocker_refuses() {
    let fixture = Fixture::new("missing-blocker");
    let before = fixture.owned_bytes();

    let refusal = fixture.hold_text(&[
        TICKET,
        "--blocker",
        ".arca/issue/i-404-gone",
        "--confirm",
        "hold t-900",
    ]);
    assert!(
        refusal.contains("i-404-gone"),
        "the refusal names the unresolvable blocker: {refusal}"
    );
    assert_eq!(before, fixture.owned_bytes(), "nothing was written");

    // An issue folder that exists but is not a complete five-file record is
    // not a blocker record either.
    fs::create_dir_all(fixture.root.join(".arca/issue/i-778-partial"))
        .expect("create partial issue");
    fs::write(
        fixture.root.join(".arca/issue/i-778-partial/index.md"),
        "# partial\n",
    )
    .expect("write partial issue");
    let refusal = fixture.hold_text(&[
        TICKET,
        "--blocker",
        ".arca/issue/i-778-partial",
        "--confirm",
        "hold t-900",
    ]);
    assert!(
        refusal.contains("i-778-partial") && refusal.contains("spec.md"),
        "the refusal names the incomplete record and what it lacks: {refusal}"
    );

    // A named residual is an acceptable blocker record.
    let residual = fixture.hold_text(&[
        TICKET,
        "--blocker",
        ".arca/residual/res-900.md",
        "--confirm",
        "hold t-900",
    ]);
    assert!(
        !residual.to_ascii_lowercase().contains("refused"),
        "a named residual is a valid blocker record: {residual}"
    );
    assert_eq!(fixture.state(), "intake", "the authorized hold routed");
}

/// PGE-006: blocker inspection never escapes the invoking project.
#[test]
fn blocker_reference_must_stay_beneath_project_root() {
    let fixture = Fixture::new("confined-blocker");
    let outside_name = format!(
        "{}-outside",
        fixture
            .root
            .file_name()
            .expect("fixture root has a basename")
            .to_string_lossy()
    );
    let outside = fixture.root.with_file_name(&outside_name);
    fs::create_dir_all(&outside).expect("create external issue");
    for name in [
        "index.md",
        "spec.md",
        "design.md",
        "test-plan.md",
        "ubi-lang.md",
    ] {
        fs::write(outside.join(name), format!("# {name}\n")).expect("write external issue");
    }
    let before = fixture.owned_bytes();
    let escaped = format!("../{outside_name}");
    let absolute = outside.to_string_lossy().into_owned();

    for blocker in [&escaped, &absolute] {
        let refusal = fixture.hold_text(&[TICKET, "--blocker", blocker, "--confirm", "hold t-900"]);
        assert!(
            refusal.contains("project root"),
            "an external complete issue is rejected before inspection: {refusal}"
        );
    }

    let _ = fs::remove_dir_all(&outside);
    assert_eq!(fixture.state(), "build", "the Run did not route");
    assert_eq!(
        before,
        fixture.owned_bytes(),
        "external blocker references leave Scheduler-owned files unchanged"
    );
}

/// HT-048-02 (Lifecycle/Model): held is a ticket state, and it never passes.
#[test]
fn held_ticket_cannot_be_passed() {
    // Ordinary routing never takes the escape, even though the Runbook
    // declares it first: a blocked route is human-authorized or nothing.
    let ordinary = Fixture::new("ordinary-routing");
    let ordinary_id = ordinary.run_id.clone();
    assert!(
        ordinary
            .rtm(&["step", "--run", &ordinary_id])
            .status
            .success(),
        "the Run steps on"
    );
    assert_eq!(
        ordinary.state(),
        "build-review",
        "rtm step follows the ordinary transition, never the blocked route"
    );

    let fixture = Fixture::new("not-passed");
    assert!(fixture
        .hold(&[TICKET, "--blocker", BLOCKER, "--confirm", "hold t-900"])
        .status
        .success());

    // The Machine state is a State; `held` never appears there.
    let state = fs::read_to_string(fixture.record_path()).expect("read state");
    assert!(
        !state.contains("held"),
        "held is a ticket state, never a Machine state: {state}"
    );

    // And the completion gate still refuses the held ticket.
    let defects = ratmac::completion::gate_completion(
        &fixture.root,
        &fixture.root.join(".ratmac"),
        &fixture.run_id,
        ".arca/ticket/t-900.md",
    )
    .expect_err("a held ticket cannot be completed");
    let text = defects
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ");
    assert!(
        text.contains("held"),
        "the completion gate says the ticket is held: {text}"
    );
    assert!(
        fixture.residual().contains("status: \"missing\""),
        "no residual became satisfied"
    );
}

/// HT-048-03 (Durability/Recovery): the route is all or nothing.
#[test]
fn interrupted_hold_leaves_no_half_route() {
    let fixture = Fixture::new("interrupted");
    let before = fixture.owned_bytes();
    let ticket_before = fixture.ticket();

    // The last write cannot land: the ticket file is read-only.
    let ticket_path = fixture.root.join(".arca/ticket/t-900.md");
    set_readonly(&ticket_path, true);
    let refusal = fixture.hold_text(&[TICKET, "--blocker", BLOCKER, "--confirm", "hold t-900"]);
    set_readonly(&ticket_path, false);

    assert!(
        refusal.to_ascii_lowercase().contains("hold"),
        "the interrupted hold reports itself: {refusal}"
    );
    assert_eq!(
        fixture.state(),
        "build",
        "an interrupted hold leaves the Run pre-route"
    );
    assert_eq!(
        ticket_before,
        fixture.ticket(),
        "an interrupted hold leaves the ticket untouched"
    );

    assert_eq!(
        before,
        fixture.owned_bytes(),
        "an interrupted hold rolls Scheduler-owned files back to their bytes"
    );

    // Recovery: with the obstruction gone, the same hold applies fully.
    assert!(fixture
        .hold(&[TICKET, "--blocker", BLOCKER, "--confirm", "hold t-900"])
        .status
        .success());
    assert_eq!(fixture.state(), "intake", "the retried hold routes fully");
    assert!(fixture.ticket().contains("status: \"held\""));
}
/// ENS-008: a runbook swapped after hold planning refuses before State writes.
#[test]
fn runbook_swap_before_hold_state_write_refuses_without_a_half_route() {
    let fixture = Fixture::new("runbook-swap");
    let owned_before = fixture.owned_bytes();
    let ticket_before = fixture.ticket();
    let barrier_dir = fixture.root.join(".ratmac/test-hold-snapshot");
    let marker = barrier_dir.join("marker");
    let release = barrier_dir.join("release");
    let mut child = Command::new(env!("CARGO_BIN_EXE_rtm"))
        .args([
            "hold",
            TICKET,
            "--blocker",
            BLOCKER,
            "--confirm",
            "hold t-900",
            "--run",
            &fixture.run_id,
        ])
        .current_dir(&fixture.root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("RATMAC_TEST_HOLD_BARRIER", "before-state-write")
        .env("RATMAC_TEST_HOLD_BARRIER_MARKER", &marker)
        .env("RATMAC_TEST_HOLD_BARRIER_RELEASE", &release)
        .env("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS", "10000")
        .spawn()
        .expect("start hold at the snapshot barrier");

    if !wait_for_file(&marker, Duration::from_secs(5)) {
        let _ = fs::write(&release, "release\n");
        let _ = child.kill();
        let _ = child.wait();
        panic!("hold did not reach the pre-State snapshot barrier");
    }

    let runbook = fixture.root.join(".ratmac/ratmac.toml");
    let original_runbook = fs::read_to_string(&runbook).expect("read runbook");
    fs::write(
        &runbook,
        format!("{original_runbook}\n# changed while hold was planned\n"),
    )
    .expect("swap valid runbook bytes");
    fs::write(&release, "release\n").expect("release hold");

    let output = child.wait_with_output().expect("reap hold");
    let refusal = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success() && refusal.contains("runbook changed during operation"),
        "the swapped runbook refuses before a State write (status={}): {refusal}",
        output.status
    );
    assert_eq!(fixture.state(), "build", "the Run did not route");
    assert_eq!(fixture.ticket(), ticket_before, "the ticket was not held");
    assert_eq!(
        fixture.owned_bytes(),
        owned_before,
        "the refusal leaves Scheduler-owned files byte-identical"
    );
}
