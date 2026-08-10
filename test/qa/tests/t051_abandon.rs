//! t-049 / PGE-007: safe human-confirmed Run abandonment.
//!
//! PT-049-01 `authorized_abandon_retires_run`
//! PT-049-02 `unauthorized_abandon_refuses_atomically`
//! PT-049-03 `leftover_lock_files_do_not_wedge_fresh_invocation`
//! HT-049-01 `near_miss_confirmation_refuses_before_write`
//! HT-049-02 `interrupted_retirement_completes_idempotently`
//! HT-049-03 `fresh_run_after_abandonment_records_its_own_pin`
//!
//! A broken Run must have a mechanized exit that no agent edit substitutes
//! for: `rtm` itself records the terminal abandoned event, retires the
//! admission state and the lock, and lets a fresh Run start. Everything
//! unconfirmed refuses with the Scheduler-owned files byte-identical. A
//! leftover lock pathname is merely residue: a fresh kernel claim reuses it
//! rather than requiring a bypass or abandonment dance.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn stale_lock_token(guard: &str) -> String {
    // This PID is deliberately outside the practical process-id range on the
    // supported test hosts, so the token models an owner that has exited.
    format!("ratmac-lock-v1\npid=2000000000\nguard={guard}\nnonce=0\n")
}
struct Fixture {
    root: PathBuf,
    /// FDC-004: the started run's id, read off the plural roster.
    run_id: String,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        restore_writable(&self.root);
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn restore_writable(root: &std::path::Path) {
    let mut paths = vec![
        root.join(".ratmac/log.md"),
        root.join(".ratmac/locks/root.lock"),
    ];
    // FDC-004: each run carries its own State File and evidence.
    if let Ok(entries) = fs::read_dir(root.join(".ratmac/runs")) {
        for entry in entries.flatten() {
            paths.push(entry.path().join("state.toml"));
            paths.push(entry.path().join("evidence.toml"));
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
}

impl Fixture {
    /// A started Run sitting in `build`: an ordinary active Run that a human
    /// may decide is broken.
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t051-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".arca")).expect("create workflow tree");
        fs::create_dir_all(root.join(".ratmac")).expect("create Engine tree");
        fs::create_dir_all(root.join("src")).expect("create source tree");
        fs::write(root.join("src/lib.rs"), "pub fn work() {}\n").expect("write source");
        fs::create_dir_all(root.join(".arca/goal")).expect("create goal tree");
        fs::write(root.join(".arca/goal/spec.md"), "# Spec\n").expect("write goal");
        fs::write(
            root.join(".ratmac/ratmac.toml"),
            "[states.intake]\nprompt = \"Integrate the issues.\"\n\n\
             [states.build]\nprompt = \"Build the ticket.\"\n\n\
             [[transitions]]\nfrom = \"intake\"\nto = \"build\"\n",
        )
        .expect("write machine class");

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
            "the fixture Run reaches build"
        );
        fixture
    }

    /// The addressed run's State File, relative to the project root.
    fn state_rel(&self) -> String {
        format!(".ratmac/runs/{}/state.toml", self.run_id)
    }

    /// The addressed run's evidence record, relative to the project root.
    fn evidence_rel(&self) -> String {
        format!(".ratmac/runs/{}/evidence.toml", self.run_id)
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

    /// The exact phrase a human must type to retire the addressed Run
    /// (FDC-007: the phrase names the run id, not the project).
    fn phrase(&self) -> String {
        format!("abandon {}", self.run_id)
    }

    fn abandon(&self, args: &[&str]) -> Output {
        // FDC-004: abandon retires an existing Run — always addressed.
        let mut all = vec!["abandon"];
        all.extend_from_slice(args);
        all.push("--run");
        all.push(&self.run_id);
        self.rtm(&all)
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn exists(&self, relative: &str) -> bool {
        self.path(relative).exists()
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path(relative)).unwrap_or_default()
    }

    /// The bytes of every Scheduler-owned file, so a refusal can be proven to
    /// have written nothing at all.
    fn owned_bytes(&self) -> Vec<(String, Option<Vec<u8>>)> {
        [
            self.state_rel(),
            ".ratmac/log.md".to_owned(),
            ".ratmac/locks/root.lock".to_owned(),
            format!(".ratmac/locks/runs/{}.lock", self.run_id),
        ]
        .iter()
        .map(|relative| (relative.clone(), fs::read(self.path(relative)).ok()))
        .collect()
    }
}

/// PT-049-01: a confirmed abandonment retires the Run and frees the project.
#[test]
fn authorized_abandon_retires_run() {
    let fixture = Fixture::new("retires");
    let log_before = fixture.read(".ratmac/log.md");
    assert!(fixture.exists(&fixture.state_rel()), "the Run is admitted");

    let output = fixture.abandon(&["--confirm", &fixture.phrase()]);
    assert!(
        output.status.success(),
        "a confirmed abandonment succeeds: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let log = fixture.read(".ratmac/log.md");
    assert!(
        log.starts_with(&log_before),
        "history is append-only across abandonment"
    );
    let event = log
        .strip_prefix(&log_before)
        .expect("the terminal event is appended");
    assert!(
        event.to_ascii_lowercase().contains("abandon"),
        "a terminal abandoned event is recorded: {event:?}"
    );
    assert!(
        event.contains("build"),
        "the terminal event names the retired Run's phase: {event:?}"
    );

    assert!(
        !fixture.exists(&fixture.state_rel()),
        "the admission state is retired"
    );
    assert!(
        !fixture.exists(".ratmac/locks/root.lock"),
        "the lock is retired"
    );

    let restart = fixture.rtm(&["start"]);
    assert!(
        restart.status.success(),
        "a fresh Run starts in the same project: {}",
        String::from_utf8_lossy(&restart.stderr)
    );
    // FDC-004: the fresh Run mints a new id on the Engine roster.
    let fresh = fs::read_dir(fixture.path(".ratmac/runs"))
        .expect("list the runs roster")
        .map(|entry| entry.expect("roster entry is readable"))
        .find(|entry| entry.path().join("state.toml").is_file())
        .expect("the fresh run appears on the roster")
        .file_name()
        .to_string_lossy()
        .into_owned();
    assert!(
        fixture
            .read(&format!(".ratmac/runs/{fresh}/state.toml"))
            .contains("intake"),
        "the fresh Run begins at the initial State"
    );
    assert!(
        fixture.read(".ratmac/log.md").contains(event.trim()),
        "the terminal event survives the fresh Run's history"
    );
}

/// PT-049-02: without the confirmation phrase nothing is touched.
#[test]
fn unauthorized_abandon_refuses_atomically() {
    let fixture = Fixture::new("unauthorized");
    let before = fixture.owned_bytes();

    for args in [
        vec![],
        vec!["--confirm"],
        vec!["--confirm", "yes"],
        vec!["--confirm", "abandon"],
    ] {
        let output = fixture.abandon(&args);
        assert!(
            !output.status.success(),
            "an unconfirmed abandonment refuses: {args:?}"
        );
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            text.contains(&fixture.phrase()),
            "the refusal names the required phrase: {text:?}"
        );
        assert_eq!(
            fixture.owned_bytes(),
            before,
            "state, history, and lock stay byte-identical after {args:?}"
        );
    }

    let id = fixture.run_id.clone();
    assert!(
        fixture.rtm(&["step", "--run", &id]).status.success(),
        "the Run is untouched and still steps"
    );
}

/// PT-049-03 / ENS-005: diagnostic bytes without a live kernel claim never
/// wedge the next rightful holder, and no command-line bypass exists.
#[test]
fn leftover_lock_files_do_not_wedge_fresh_invocation() {
    let fixture = Fixture::new("stale-lock");
    let id = fixture.run_id.clone();
    let engine_root = fixture.path(".ratmac");
    let root_lock = ratmac::lock::root_path(&engine_root);
    fs::write(&root_lock, stale_lock_token("root")).expect("seed leftover root lock bytes");

    // Move the existing Run under root residue. This is a real motion, not
    // status: ENS-005 keeps the root and addressed-Run domains independent.
    let state_path = fixture.path(&fixture.state_rel());
    let build_state = fs::read_to_string(&state_path).expect("read build state");
    assert!(
        build_state.contains("phase = \"build\""),
        "fixture reached build before the root-domain check"
    );
    fs::write(
        &state_path,
        build_state.replacen("phase = \"build\"", "phase = \"intake\"", 1),
    )
    .expect("restore a movable intake state");
    let motion = fixture.rtm(&["step", "--run", &id]);
    assert!(
        motion.status.success(),
        "an existing Run moves while only root residue exists: {}",
        String::from_utf8_lossy(&motion.stderr)
    );
    assert!(
        root_lock.exists(),
        "Run motion neither claims nor consumes root-domain residue"
    );

    // A fresh root-domain claimant reuses the leftover file and drops it on
    // release; byte contents do not decide ownership.
    assert!(
        fixture.rtm(&["start"]).status.success(),
        "a stale root pathname never wedges minting"
    );
    assert!(
        !root_lock.exists(),
        "the fresh root claimant releases its path"
    );

    let run_lock = ratmac::lock::run_path(&engine_root, &id);
    fs::create_dir_all(run_lock.parent().expect("Run lock has parent"))
        .expect("create Run lock directory");
    fs::write(&run_lock, stale_lock_token(&format!("run:{id}")))
        .expect("seed leftover addressed-Run lock bytes");
    let build_state = fs::read_to_string(&state_path).expect("read build state again");
    fs::write(
        &state_path,
        build_state.replacen("phase = \"build\"", "phase = \"intake\"", 1),
    )
    .expect("restore a second movable intake state");
    let motion = fixture.rtm(&["step", "--run", &id]);
    assert!(
        motion.status.success(),
        "a stale addressed-Run pathname never wedges motion: {}",
        String::from_utf8_lossy(&motion.stderr)
    );
    assert!(
        !run_lock.exists(),
        "the fresh Run claimant releases its path"
    );

    // The two surviving historical intents remain direct: no bypass flag is
    // accepted, and normal acquisition above is what cleared the residue.
    for bypass in [
        vec!["step", "--run", id.as_str(), "--force"],
        vec!["step", "--run", id.as_str(), "--no-lock"],
        vec!["abandon", "--run", id.as_str(), "--force"],
    ] {
        let output = fixture.rtm(&bypass);
        assert!(
            !output.status.success(),
            "no bypass flag exists: {bypass:?} must not succeed"
        );
    }
}

/// HT-049-01: a near-miss phrase is refused before the first byte is written.
#[test]
fn near_miss_confirmation_refuses_before_write() {
    let fixture = Fixture::new("near-miss");
    let before = fixture.owned_bytes();
    let phrase = fixture.phrase();

    let near_misses = [
        format!("{phrase}."),
        format!(" {phrase}"),
        phrase.to_uppercase(),
        phrase.replace("abandon", "abandon "),
        phrase
            .trim_end_matches(|character: char| character.is_ascii_alphanumeric())
            .to_owned(),
    ];
    for near in near_misses {
        let output = fixture.abandon(&["--confirm", &near]);
        assert!(
            !output.status.success(),
            "a near-miss phrase refuses: {near:?}"
        );
        assert_eq!(
            fixture.owned_bytes(),
            before,
            "nothing is written for {near:?}"
        );
    }
}

/// HT-049-02: an interrupted retirement never half-retires; after the
/// obstruction clears, re-running the confirmed command finishes the job.
#[test]
fn interrupted_retirement_completes_idempotently() {
    let fixture = Fixture::new("interrupted");

    // The terminal event cannot be recorded: nothing is retired.
    let before = fixture.owned_bytes();
    let log_path = fixture.path(".ratmac/log.md");
    let metadata = fs::metadata(&log_path).expect("read log metadata");
    let mut permissions = metadata.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&log_path, permissions).expect("make history unwritable");

    let interrupted = fixture.abandon(&["--confirm", &fixture.phrase()]);
    assert!(
        !interrupted.status.success(),
        "retirement fails when the terminal event cannot be recorded"
    );
    assert_eq!(
        fixture.owned_bytes(),
        before,
        "an interrupted retirement leaves no half-retired Run"
    );
    restore_writable(&fixture.root);

    // An unreadable Run file is refused before the first write, never
    // snapshotted as "absent" and then deleted by its own rollback.
    let evidence_path = fixture.path(&fixture.evidence_rel());
    let evidence_bytes = fs::read(&evidence_path).expect("read Run evidence");
    fs::remove_file(&evidence_path).expect("clear Run evidence");
    fs::create_dir(&evidence_path).expect("make Run evidence unreadable");
    let unreadable_before = fixture.owned_bytes();
    let unreadable = fixture.abandon(&["--confirm", &fixture.phrase()]);
    assert!(
        !unreadable.status.success(),
        "an unreadable Run file refuses the retirement"
    );
    assert_eq!(
        fixture.owned_bytes(),
        unreadable_before,
        "the unreadable-file refusal writes nothing"
    );
    assert!(
        evidence_path.exists(),
        "the unreadable Run file is never deleted by its own rollback"
    );
    let unreadable_text = format!(
        "{}{}",
        String::from_utf8_lossy(&unreadable.stdout),
        String::from_utf8_lossy(&unreadable.stderr)
    );
    assert!(
        unreadable_text.contains("evidence.toml"),
        "the refusal names the file it could not read: {unreadable_text:?}"
    );
    assert!(
        !unreadable_text.contains("rollback incomplete"),
        "the unreadable file is caught before any write, so nothing needs rolling back: {unreadable_text:?}"
    );
    fs::remove_dir(&evidence_path).expect("clear the unreadable Run evidence");
    fs::write(&evidence_path, &evidence_bytes).expect("restore Run evidence");

    // Both pre-write refusals cleared: the same confirmed command now
    // completes the retirement rather than needing any special lock cleanup.
    let completed = fixture.abandon(&["--confirm", &fixture.phrase()]);
    assert!(
        completed.status.success(),
        "the confirmed command completes after the obstruction clears: {}",
        String::from_utf8_lossy(&completed.stderr)
    );
    assert!(
        !fixture.exists(&fixture.state_rel()),
        "admission is retired"
    );

    // Nothing left to retire: an honest refusal, still writing nothing.
    let nothing_before = fixture.owned_bytes();
    let nothing = fixture.abandon(&["--confirm", &fixture.phrase()]);
    assert!(
        !nothing.status.success(),
        "abandoning a project with no active Run refuses"
    );
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&nothing.stdout),
        String::from_utf8_lossy(&nothing.stderr)
    );
    assert!(
        text.to_ascii_lowercase().contains("nothing to retire"),
        "the refusal says there is nothing to retire: {text:?}"
    );
    assert_eq!(
        fixture.owned_bytes(),
        nothing_before,
        "the empty refusal writes nothing"
    );
}

/// HT-049-03: the Run after an abandonment is genuinely new.
#[test]
fn fresh_run_after_abandonment_records_its_own_pin() {
    let fixture = Fixture::new("fresh-pin");
    let stale_evidence = fixture.read(&fixture.evidence_rel());
    assert!(
        stale_evidence.contains("[engine]"),
        "the abandoned Run recorded an Engine pin: {stale_evidence:?}"
    );
    // The abandoned Run also carried a gate pin and a frozen goal.
    fs::write(
        fixture.path(&fixture.evidence_rel()),
        format!("{stale_evidence}\n[[gates]]\nprogram = \"ghost\"\nresolved = \"E:/ghost.exe\"\nsha256 = \"{}\"\n\n[goal]\nfrozen = \"stale-frozen-revision\"\n", "0".repeat(64)),
    )
    .expect("seed stale Run evidence");

    assert!(
        fixture
            .abandon(&["--confirm", &fixture.phrase()])
            .status
            .success(),
        "the Run is abandoned"
    );
    assert!(
        !fixture.exists(&fixture.evidence_rel()),
        "Run evidence is retired with the Run"
    );

    assert!(
        fixture.rtm(&["start"]).status.success(),
        "a fresh Run starts"
    );
    // FDC-004: the fresh Run's evidence lives in its own Engine run directory.
    let fresh = fs::read_dir(fixture.path(".ratmac/runs"))
        .expect("list the runs roster")
        .map(|entry| entry.expect("roster entry is readable"))
        .find(|entry| entry.path().join("state.toml").is_file())
        .expect("the fresh run appears on the roster")
        .file_name()
        .to_string_lossy()
        .into_owned();
    let evidence = fixture.read(&format!(".ratmac/runs/{fresh}/evidence.toml"));
    assert!(
        evidence.contains("[engine]"),
        "the fresh Run records its own Engine pin: {evidence:?}"
    );
    assert!(
        !evidence.contains("ghost"),
        "no gate pin is inherited from the abandoned Run: {evidence:?}"
    );
    assert!(
        !evidence.contains("stale-frozen-revision"),
        "no freeze is inherited from the abandoned Run: {evidence:?}"
    );
    assert!(
        fixture
            .text(&["status", "--run", &fresh])
            .contains("intake"),
        "the fresh Run reports its own baseline position"
    );
}
