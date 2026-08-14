//! t-101 / ECP-001..ECP-003: the engine pin carries its channel.
//!
//! ECPV-001: an `[engine]` record with provenance round-trips, and a
//! provenance mismatch refuses with the identity-mismatch diagnostic class
//! while updating nothing.
//! ECPV-002: `stable` resolves from the editions ledger and `nightly` from
//! the current landing, offline; a ledger/tag disagreement refuses.
//! ECPV-003: the doctor names a live Run driven by an off-pin engine -
//! exactly one finding - and a matching stable pin yields none, read-only.

use ratmac::channel::{live_run_findings, resolve_channel};
use ratmac::pin::{evidence_path, Evidence, Identity};
use ratmac_qa::tempgit::TempRepo;
use std::fs;

fn provenanced(source_commit: &str, channel: &str) -> Identity {
    Identity {
        resolved: "target/debug/rtm".into(),
        sha256: "a".repeat(64),
        source_commit: Some(source_commit.into()),
        channel: Some(channel.into()),
    }
}

#[test]
fn a_provenance_mismatch_refuses_and_updates_nothing() {
    let repo = TempRepo::new("t101-pin");
    let run_dir = repo.root().join(".ratmac/runs/run-001");
    fs::create_dir_all(&run_dir).expect("create run directory");

    // Absent -> recorded: the pin is written with its provenance.
    let recorded = provenanced(&"1".repeat(40), "stable");
    let mut evidence = Evidence::default();
    evidence
        .confirm_engine(recorded.clone())
        .expect("an absent pin records the observed identity");
    evidence.write(&run_dir).expect("write run evidence");

    // Round-trip preserves provenance.
    let mut reloaded = Evidence::load(&run_dir);
    assert_eq!(
        reloaded.engine.as_ref(),
        Some(&recorded),
        "provenance fields must survive the round-trip"
    );

    // A differing source-commit refuses in the identity-mismatch class and
    // the recorded bytes do not change.
    let before = fs::read(evidence_path(&run_dir)).expect("read recorded pin");
    let refusal = reloaded
        .confirm_engine(provenanced(&"2".repeat(40), "stable"))
        .expect_err("a differing source-commit must refuse");
    assert!(
        refusal.contains("pin mismatch"),
        "the refusal speaks the identity-mismatch diagnostic class: {refusal}"
    );
    let refusal = reloaded
        .confirm_engine(provenanced(&"1".repeat(40), "nightly"))
        .expect_err("a differing channel must refuse");
    assert!(refusal.contains("pin mismatch"), "channel too: {refusal}");
    reloaded.write(&run_dir).expect("rewrite after refusals");
    let after = fs::read(evidence_path(&run_dir)).expect("re-read recorded pin");
    assert_eq!(before, after, "a refusal updates nothing");
}

/// A repository with a two-row editions ledger and matching tags.
fn ledgered() -> (TempRepo, String, String) {
    let repo = TempRepo::new("t101-ledger");
    repo.write("src/lib.rs", "pub fn one() {}\n");
    repo.commit_all("first edition");
    let first = repo.head();
    repo.git(&["tag", "-a", "edition-001", "-m", "edition-001"]);
    repo.write("src/lib.rs", "pub fn two() {}\n");
    repo.commit_all("second edition");
    let second = repo.head();
    repo.git(&["tag", "-a", "edition-002", "-m", "edition-002"]);
    repo.write(
        ".arca/editions.md",
        &format!(
            "# Editions\n\n| Edition | Commit | What it marks |\n| :--- | :--- | :--- |\n\
             | `edition-001` | `{first}` | The first. |\n\
             | `edition-002` | `{second}` | The second. |\n"
        ),
    );
    repo.commit_all("ledger");
    (repo, first, second)
}

#[test]
fn the_bootstrap_resolves_stable_from_the_ledger() {
    let (repo, _first, second) = ledgered();

    let stable = resolve_channel(repo.root(), "stable").expect("stable resolves from the ledger");
    assert_eq!(stable.commit, second, "stable is the newest ledger row");
    assert_eq!(stable.edition, "edition-002");

    let nightly = resolve_channel(repo.root(), "nightly").expect("nightly resolves from HEAD");
    assert_eq!(
        nightly.commit,
        repo.head(),
        "nightly is the current landing"
    );

    resolve_channel(repo.root(), "beta").expect_err("an unknown channel refuses");

    // Move the newest tag: the ledger and the tag now disagree, and the
    // resolver refuses rather than picking a side.
    repo.git(&["tag", "-f", "-a", "edition-002", "-m", "moved", "HEAD"]);
    let refusal =
        resolve_channel(repo.root(), "stable").expect_err("a ledger/tag disagreement refuses");
    assert!(
        refusal.contains("disagreement"),
        "the refusal names the disagreement: {refusal}"
    );
}

#[test]
fn the_doctor_names_an_off_pin_engine_under_a_live_run() {
    let (repo, _first, second) = ledgered();
    let engine_root = repo.root().join(".ratmac");
    let run_dir = engine_root.join("runs/run-001");
    fs::create_dir_all(&run_dir).expect("create run directory");
    fs::write(
        run_dir.join("run.toml"),
        "state = \"intake\"\nstatus = \"executing\"\n",
    )
    .expect("write a live Run Record");

    // A live Run driven by a nightly engine: exactly one finding.
    let mut evidence = Evidence::default();
    evidence.set_engine(provenanced(&"9".repeat(40), "nightly"));
    evidence.write(&run_dir).expect("record the nightly pin");
    let snapshot = fs::read(evidence_path(&run_dir)).expect("snapshot before the doctor");
    let findings = live_run_findings(repo.root(), &engine_root);
    assert_eq!(findings.len(), 1, "exactly one finding: {findings:?}");
    assert!(
        findings[0].contains("run-001") && findings[0].contains("nightly"),
        "the finding names the run and the channel: {}",
        findings[0]
    );

    // A matching stable pin yields none.
    let mut evidence = Evidence::default();
    evidence.set_engine(provenanced(&second, "stable"));
    evidence.write(&run_dir).expect("record the stable pin");
    let clean = live_run_findings(repo.root(), &engine_root);
    assert!(
        clean.is_empty(),
        "a matching stable pin is clean: {clean:?}"
    );

    // End to end: `rtm doctor` run inside this fixture renders the channel
    // row, the provenance row, and the one finding - and writes nothing.
    let mut evidence = Evidence::default();
    evidence.set_engine(provenanced(&"9".repeat(40), "nightly"));
    evidence.write(&run_dir).expect("restore the nightly pin");
    let doctor = std::process::Command::new(ratmac_qa::engine_bin!())
        .arg("doctor")
        .current_dir(repo.root())
        .output()
        .expect("run rtm doctor in the fixture");
    let report = String::from_utf8_lossy(&doctor.stdout);
    assert!(
        report.contains("Engine provenance: channel="),
        "the doctor reports the running engine's channel and provenance:\n{report}"
    );
    assert!(
        report.contains(&format!(
            "Engine channel: stable is edition-002 at {second}"
        )),
        "the doctor names what stable resolves to:\n{report}"
    );
    assert!(
        report
            .lines()
            .filter(|line| line.starts_with("Engine channel finding:"))
            .count()
            == 1
            && report.contains("run-001")
            && report.contains("nightly"),
        "exactly one rendered finding names the off-pin run:\n{report}"
    );

    // Read-only: the walk changed nothing it read.
    let mut evidence = Evidence::default();
    evidence.set_engine(provenanced(&"9".repeat(40), "nightly"));
    evidence.write(&run_dir).expect("restore the nightly pin");
    let _ = live_run_findings(repo.root(), &engine_root);
    let after = fs::read(evidence_path(&run_dir)).expect("re-read after the doctor");
    assert_eq!(snapshot, after, "the doctor stays read-only");
}
