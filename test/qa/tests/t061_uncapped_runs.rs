//! t-060 / FDC-006: uncapped runs — never-reissued ids, respawn mints a new id.
//!
//! PT-060-01 `no_active_run_cap_is_enforced`
//! PT-060-02 `abandoned_ids_are_never_reissued`
//!
//! Multi-run is uncapped: `rtm start` no longer refuses while another Run is
//! live, and any number of runs coexist under `.ratmac/runs/`, each addressed
//! by its own id. Within the one run-id namespace an id is never reissued
//! after abandon: the retired run's directory keeps its address, and minting
//! skips every existing run directory — live or retired — so no later Run
//! can occupy a failed Run's evidence. A respawn mints a fresh id rather
//! than reviving the old one, exercised here as a namespace fact only: the
//! `respawn` verb and what the ledger entry records about the superseded id
//! belong to the machine-composition issue (i-018), and no ledger-entry
//! content is read or written here. This supersedes `R-022` (at most one
//! active Run per project) and its check `T-08`; `t012_r022.rs` proves the
//! superseded cap and is deleted in this ticket's green landing.

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
    /// A temp project with a valid two-state runbook, not yet started.
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t060-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
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
        Fixture { root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Listing `.ratmac/runs/` IS the roster: the run ids, read off artifacts.
    fn roster(&self) -> Vec<String> {
        let runs = self.path(".ratmac/runs");
        assert!(
            runs.is_dir(),
            "FDC-006: runs must reside under the plural .ratmac/runs/ path — listing it IS the roster"
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

    /// The exact phrase a human types to retire the named run
    /// (FDC-007: the phrase names the run id, not the project).
    fn confirm_phrase(&self, id: &str) -> String {
        format!("abandon {id}")
    }

    /// The one id on the roster beyond `known` — exactly one start minted it.
    fn newly_minted(&self, known: &[String]) -> String {
        let roster = self.roster();
        let fresh: Vec<&String> = roster.iter().filter(|id| !known.contains(id)).collect();
        assert_eq!(
            fresh.len(),
            1,
            "FDC-006: one start mints exactly one new id; roster {roster:?}, known {known:?}"
        );
        fresh[0].clone()
    }
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn state_state(state: &str) -> String {
    let value: toml::Value = state
        .parse()
        .expect("FDC-006: the run's State File must be valid TOML");
    value["state"]
        .as_str()
        .expect("FDC-006: the run's State File must carry a state")
        .to_owned()
}

/// PT-060-01 (RRV-004): three consecutive `rtm start` invocations on one
/// project each succeed with no cap refusal, each minting a distinct id and
/// its own `.ratmac/runs/<id>/`; each run is independently addressable and
/// steppable via `--run <id>`.
#[test]
fn no_active_run_cap_is_enforced() {
    let fixture = Fixture::new("uncapped");
    let state_rel = |id: &str| format!(".ratmac/runs/{id}/run.toml");

    let mut ids: Vec<String> = Vec::new();
    for nth in 1..=3 {
        let start = fixture.rtm(&["start"]);
        assert!(
            start.status.success(),
            "FDC-006: start {nth} of 3 must succeed — no active-Run cap is enforced: {}",
            combined(&start)
        );
        let minted = fixture.newly_minted(&ids);
        assert!(
            fixture.path(&state_rel(&minted)).is_file(),
            "FDC-006: the minted run {minted} must carry its own State File under .ratmac/runs/{minted}/"
        );
        ids.push(minted);
    }
    assert_eq!(
        fixture.roster().len(),
        3,
        "FDC-006: three starts leave three coexisting runs on the roster"
    );

    // Independently addressable: `status --run <id>` answers for each run and
    // leaves every run's State File byte-identical.
    let before: Vec<Vec<u8>> = ids
        .iter()
        .map(|id| fs::read(fixture.path(&state_rel(id))).expect("run State File is readable"))
        .collect();
    for id in &ids {
        let status = fixture.rtm(&["status", "--run", id]);
        assert!(
            status.status.success(),
            "FDC-006: status --run {id} must address exactly that run: {}",
            combined(&status)
        );
    }
    for (id, bytes) in ids.iter().zip(&before) {
        assert_eq!(
            &fs::read(fixture.path(&state_rel(id))).expect("run State File stays readable"),
            bytes,
            "FDC-006: status must leave run {id} byte-identical"
        );
    }

    // Independently steppable: stepping the middle run advances exactly that
    // run; both siblings stay byte-identical.
    let stepped = ids[1].clone();
    let step = fixture.rtm(&["step", "--run", &stepped]);
    assert!(
        step.status.success(),
        "FDC-006: step --run {stepped} must advance the named run: {}",
        combined(&step)
    );
    let after = fs::read_to_string(fixture.path(&state_rel(&stepped)))
        .expect("the stepped run's State File stays readable");
    assert_eq!(
        state_state(&after),
        "build",
        "FDC-006: stepping {stepped} must advance exactly that run to the next state"
    );
    for (id, bytes) in ids.iter().zip(&before) {
        if id != &stepped {
            assert_eq!(
                &fs::read(fixture.path(&state_rel(id))).expect("sibling State File stays readable"),
                bytes,
                "FDC-006: stepping {stepped} must not touch its sibling {id}"
            );
        }
    }
}

/// PT-060-02 (RRV-004): after the confirmed `abandon`, the retired id's
/// directory still occupies its address; every subsequent start — including
/// one modeling a respawn — mints a fresh id, never a retired one; no
/// ledger-entry content is read or written. Each abandon here retires the
/// highest ordinal on the roster, so minting after the abandoned-highest-id
/// case is exactly what is exercised.
#[test]
fn abandoned_ids_are_never_reissued() {
    let fixture = Fixture::new("reissue");
    // The fixture premise: a project with one abandoned run.
    let start = fixture.rtm(&["start"]);
    assert!(
        start.status.success(),
        "start must succeed on a valid project: {}",
        combined(&start)
    );
    let first = fixture.newly_minted(&[]);
    let abandon = fixture.rtm(&[
        "abandon",
        "--run",
        &first,
        "--confirm",
        &fixture.confirm_phrase(&first),
    ]);
    assert!(
        abandon.status.success(),
        "the confirmed abandon must retire the named run: {}",
        combined(&abandon)
    );

    // The retired id's directory still occupies its address, terminal.
    let first_dir = fixture.path(&format!(".ratmac/runs/{first}"));
    assert!(
        first_dir.is_dir(),
        "FDC-006: the retired run's directory must keep its address on disk"
    );
    assert!(
        !first_dir.join("run.toml").is_file(),
        "the retired run is terminal: its admission state is retired"
    );
    let first_ledger = first_dir.join("spawn-ledger");
    let first_ledger_before = fs::read(&first_ledger)
        .expect("FDC-006: the retired run's reserved spawn-ledger path keeps its address");

    // A subsequent start mints a fresh id, never the retired one.
    let second_start = fixture.rtm(&["start"]);
    assert!(
        second_start.status.success(),
        "FDC-006: start after abandon must succeed: {}",
        combined(&second_start)
    );
    let second = fixture.newly_minted(std::slice::from_ref(&first));
    assert_ne!(
        second, first,
        "FDC-006: an abandoned run's id is never reissued"
    );

    // Model a respawn: retire the live run, then start its successor. The
    // successor mints a fresh id rather than reviving either retired id — a
    // namespace fact; the respawn verb itself is machine composition's.
    let abandon_second = fixture.rtm(&[
        "abandon",
        "--run",
        &second,
        "--confirm",
        &fixture.confirm_phrase(&second),
    ]);
    assert!(
        abandon_second.status.success(),
        "the confirmed abandon must retire the named run: {}",
        combined(&abandon_second)
    );
    let respawn_start = fixture.rtm(&["start"]);
    assert!(
        respawn_start.status.success(),
        "FDC-006: the respawn-modeling start must succeed: {}",
        combined(&respawn_start)
    );
    let third = fixture.newly_minted(&[first.clone(), second.clone()]);
    assert!(
        third != first && third != second,
        "FDC-006: a respawn mints a fresh id, never a retired one — got {third} after retiring {first} and {second}"
    );

    // No address was handed back: every id that ever existed still holds its
    // directory, and the roster lists all three.
    assert_eq!(
        fixture.roster(),
        {
            let mut all = vec![first.clone(), second.clone(), third.clone()];
            all.sort();
            all
        },
        "FDC-006: retired and live run directories all keep their addresses"
    );

    // No ledger-entry content was read or written: the retired run's reserved
    // spawn-ledger is byte-identical across every later mint, and no ledger
    // ever gained content (reserved by name, kept contentless).
    assert_eq!(
        fs::read(&first_ledger).expect("the retired spawn-ledger stays readable"),
        first_ledger_before,
        "FDC-006: later minting must not write into the retired run's spawn-ledger"
    );
    for id in [&first, &second, &third] {
        let ledger = fs::read(fixture.path(&format!(".ratmac/runs/{id}/spawn-ledger")))
            .expect("each run's reserved spawn-ledger path exists by name");
        assert!(
            ledger.is_empty(),
            "FDC-006: no ledger-entry content is written — that contract is machine composition's (i-018), found bytes in {id}'s spawn-ledger"
        );
    }
}
