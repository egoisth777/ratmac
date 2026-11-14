//! t-053 / ORS-002: project-local bootstrap and read-only doctor.
//!
//! PT-053-01 `bootstrap_resolves_offline`
//! PT-053-03 `doctor_is_actionable_and_write_free`
//! HT-053-01 `cli_surface_survives_the_new_entry_point`
//! HT-053-02 `doctor_rejects_extra_arguments`
//! HT-053-04 `doctor_survives_corrupt_state_and_held_lock`
//! HT-053-05 `bootstrap_writes_only_declared_build_output`
//! HT-053-06 (the refusal quotes the recorded identity fields), kept in
//! `lrpv_001`
//! SVEV-004 `svev_004`, SVEV-005 `svev_005` (project root), SVEV-006 `svev_006`
//! (t-144 / SVE-002: the Engine is the executable Cargo reported)
//! LRPV-001 `lrpv_001`, LRPV-002 `lrpv_002`, LRPV-003 `lrpv_003`, LRPV-004
//! `lrpv_004`, LRPV-005 `lrpv_005` (t-145 / LRP-001: the live Run pin check;
//! LRPV-001 supersedes PT-053-02 / ORSV-006, the flat-pin
//! `bootstrap_refuses_pin_mismatch`). Under LRP-001 the bootstrap reads only
//! live Runs' pins, so PT-053-01 expects `Pin: no live Run in`, and SVEV-004
//! and SVEV-005 record their pins in a live Run instead of the flat
//! `.ratmac/evidence.toml`.
//!
//! A fresh session must be able to orient and start without ad-hoc installs:
//! one documented command resolves or builds the Engine offline and refuses
//! when a live Run's pinned Engine file has changed, and the doctor diagnoses
//! the project while writing nothing.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use ratmac::pin::{Evidence, Identity};
use ratmac_qa::support::{self, TempTree};
use ratmac_qa::tempgit::TempRepo;
use sha2::{Digest, Sha256};

/// Cargo writes these while building; everything else must stay byte-identical.
const DECLARED_BUILD_OUTPUT: [&str; 2] = ["target", "Cargo.lock"];

/// The fixture's manifest: a workspace of one bin named exactly as the
/// bootstrap builds it.
const FIXTURE_MANIFEST: &str = "[workspace]\n\n[package]\nname = \"ratmac-bootstrap-fixture\"\n\
     version = \"0.0.0\"\nedition = \"2021\"\n\n\
     [[bin]]\nname = \"rtm\"\npath = \"src/main.rs\"\n";

/// LRP-001: a runbook with a second State and one transition, so the Engine's
/// `rtm start` leaves a live Run (`status = "planned"`); with one State the
/// Run it starts has already passed.
const TWO_STATE_RUNBOOK: &str = "[states.build]\nprompt = \"Build the selected artifact.\"\n\n\
     [states.done]\nprompt = \"Confirm the built artifact.\"\n\n\
     [[transitions]]\nfrom = \"build\"\nto = \"done\"\n";

/// LRP-001's one pin refusal keeps the bootstrap's existing reason.
const PIN_REFUSAL: &str = "bootstrap refused: the resolved Engine is not the pinned Engine";

struct Boot {
    _tree: TempTree,
    root: PathBuf,
}

impl Boot {
    /// A throwaway project root that carries the real `tools/rtm.ps1`, a
    /// Runbook, and a buildable `rtm` binary target.
    fn new(label: &str) -> Self {
        let tree = TempTree::new(&format!("t053-{label}")).expect("create fixture tree");
        let root = tree.path().to_path_buf();
        for dir in [".ratmac", "src", "tools"] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        fs::write(root.join("Cargo.toml"), FIXTURE_MANIFEST).expect("write fixture manifest");
        fs::write(root.join("src/main.rs"), "fn main() {}\n").expect("write fixture source");
        fs::write(
            root.join(".ratmac/ratmac.toml"),
            "[states.build]\nprompt = \"Build the selected artifact.\"\n",
        )
        .expect("write runbook");
        fs::copy(bootstrap_source(), root.join("tools/rtm.ps1")).expect("install the bootstrap");
        Boot { _tree: tree, root }
    }

    /// LRP-001: the same project root with a two-State runbook, so a Run the
    /// Engine starts in it stays live.
    fn two_state(label: &str) -> Self {
        let boot = Self::new(label);
        fs::write(boot.root.join(".ratmac/ratmac.toml"), TWO_STATE_RUNBOOK)
            .expect("write the two-State runbook");
        boot
    }

    /// Put a real executable where the bootstrap resolves it, without building.
    fn prebuild(&self) -> PathBuf {
        plant_engine(&self.root)
    }

    /// The bootstrap with the fixture's own `target` named to Cargo and the
    /// other output redirects cleared (SVE-002: the doctor suite's
    /// planted-binary tests set `CARGO_TARGET_DIR` to the fixture's own
    /// `target` and clear `CARGO_BUILD_TARGET` for the bootstrap they start,
    /// so no inherited redirect or Cargo configuration file moves their
    /// output; `CARGO_BUILD_BUILD_DIR` is cleared for the same reason).
    fn bootstrap(&self) -> Output {
        bootstrap_planted(&self.root, &[])
    }

    /// `pwsh -NoProfile -File tools/rtm.ps1` from the root, with each variable
    /// in `remove` cleared and then each one in `set` given.
    fn bootstrap_env(&self, set: &[(&str, &OsStr)], remove: &[&str]) -> Output {
        bootstrap_in(&self.root, set, remove)
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    /// Every file under the root, minus the declared build output.
    fn snapshot(&self) -> String {
        digest_tree(&self.root, &DECLARED_BUILD_OUTPUT)
    }

    /// Every file under the root, with no exception at all.
    fn whole_snapshot(&self) -> String {
        digest_tree(&self.root, &[])
    }
}

/// A real executable where the bootstrap resolves it under `root`, without
/// building: the test Engine copied to `target/debug`.
fn plant_engine(root: &Path) -> PathBuf {
    let target = root.join("target/debug");
    fs::create_dir_all(&target).expect("create build directory");
    let binary = target.join(engine_file_name());
    fs::copy(ratmac_qa::engine_bin!(), &binary).expect("place a prebuilt Engine");
    binary
}

/// `pwsh -NoProfile -File tools/rtm.ps1` from `root`, with each variable in
/// `remove` cleared and then each one in `set` given.
fn bootstrap_in(root: &Path, set: &[(&str, &OsStr)], remove: &[&str]) -> Output {
    let mut command = Command::new("pwsh");
    command
        .args(["-NoProfile", "-File", "tools/rtm.ps1"])
        .current_dir(root);
    for name in remove {
        command.env_remove(name);
    }
    for (name, value) in set {
        command.env(name, value);
    }
    command.output().expect("invoke the bootstrap")
}

/// The planted-binary bootstrap from `root` (SVE-002): every output redirect
/// cleared, `CARGO_TARGET_DIR` naming `root`'s own `target`, and then each
/// variable in `set` given.
fn bootstrap_planted(root: &Path, set: &[(&str, &OsStr)]) -> Output {
    let own = root.join("target");
    let mut given: Vec<(&str, &OsStr)> = vec![("CARGO_TARGET_DIR", own.as_os_str())];
    given.extend_from_slice(set);
    bootstrap_in(root, &given, &REDIRECTS)
}

fn engine_file_name() -> &'static str {
    if cfg!(windows) {
        "rtm.exe"
    } else {
        "rtm"
    }
}

fn bootstrap_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/rtm.ps1")
        .canonicalize()
        .expect("the bootstrap under test exists")
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root resolves")
}

fn digest_tree(root: &Path, skip: &[&str]) -> String {
    fn walk(directory: &Path, base: &Path, skip: &[&str], rows: &mut Vec<String>) {
        let mut entries: Vec<PathBuf> = fs::read_dir(directory)
            .expect("read fixture tree")
            .map(|entry| entry.expect("read entry").path())
            .collect();
        entries.sort();
        for path in entries {
            let relative = path
                .strip_prefix(base)
                .expect("path under the fixture root")
                .to_string_lossy()
                .replace('\\', "/");
            if skip.iter().any(|name| relative == *name) {
                continue;
            }
            if path.is_dir() {
                walk(&path, base, skip, rows);
            } else {
                let bytes = fs::read(&path).expect("read fixture file");
                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                rows.push(format!("{relative} {:x}", hasher.finalize()));
            }
        }
    }
    let mut rows = Vec::new();
    walk(root, root, skip, &mut rows);
    rows.push(String::new());
    rows.join("\n")
}

fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).expect("read binary");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The one command the contributor guidance documents, read from the guidance
/// itself so the test cannot drift from what a reader is told to run.
fn documented_command() -> String {
    let index = fs::read_to_string(project_root().join(".arca/schema.md")).expect("read index");
    let section = index
        .split("## Bootstrap")
        .nth(1)
        .expect("the guidance documents a bootstrap");
    let section = section.split("\n## ").next().expect("section body");
    section
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("pwsh "))
        .unwrap_or_else(|| panic!("the bootstrap section names a command: {section}"))
        .trim_matches('`')
        .to_owned()
}

/// PT-053-01: from a clean project root the documented command resolves the
/// Engine, reports path and hash, and reaches no network or global state.
#[test]
fn bootstrap_resolves_offline() {
    assert_eq!(
        documented_command(),
        "pwsh -File tools/rtm.ps1",
        "the guidance documents exactly the command this test runs"
    );

    let boot = Boot::new("resolve");
    let path_before = std::env::var_os("PATH");
    let before = boot.snapshot();

    let output = boot.bootstrap();
    let report = text(&output);
    assert!(output.status.success(), "a clean root bootstraps: {report}");

    let binary = boot.root.join("target/debug").join(engine_file_name());
    assert!(
        binary.is_file(),
        "the bootstrap built the project-local Engine: {report}"
    );
    let resolved = binary.canonicalize().expect("resolve the built Engine");
    let reported_path = report
        .lines()
        .find_map(|line| line.strip_prefix("Engine: "))
        .expect("the report names the resolved path");
    assert_eq!(
        Path::new(reported_path.trim())
            .canonicalize()
            .expect("the reported path exists"),
        resolved,
        "the reported path is the binary it resolved: {report}"
    );
    let reported_hash = report
        .lines()
        .find_map(|line| line.strip_prefix("sha256: "))
        .expect("the report names the content hash")
        .trim();
    assert_eq!(
        reported_hash,
        sha256_file(&binary),
        "the reported hash is this binary's content: {report}"
    );
    // LRP-001: with no live Run the one `Pin:` line says so, naming the
    // fixture's `.ratmac`, rather than claiming a match.
    assert!(
        no_live_run(&pin_lines(&output), &boot.root),
        "with no live Run the one Pin: line is `no live Run in` the fixture's .ratmac: {report}"
    );

    // The command is documented as running from the project root, and says so
    // rather than guessing when it is run anywhere else.
    let elsewhere = boot.root.join("src");
    let strayed = Command::new("pwsh")
        .args(["-NoProfile", "-File", "../tools/rtm.ps1"])
        .current_dir(&elsewhere)
        .output()
        .expect("invoke the bootstrap from a subdirectory");
    let strayed_report = text(&strayed);
    assert!(
        !strayed.status.success(),
        "running it from a subdirectory refuses: {strayed_report}"
    );
    assert!(
        strayed_report.contains("project root") && strayed_report.contains("cd "),
        "the refusal says where to stand instead: {strayed_report}"
    );

    assert_eq!(
        std::env::var_os("PATH"),
        path_before,
        "the bootstrap mutates no PATH"
    );
    assert_eq!(
        boot.snapshot(),
        before,
        "outside the declared build output the project is untouched"
    );
}

/// LRPV-001 (LRP-001; supersedes PT-053-02 / ORSV-006 and keeps HT-053-06):
/// in a two-State fixture the Engine planted at `target/debug` starts
/// `run-001`, so the Engine itself writes the live record and the pin. With
/// one byte appended to that binary the bootstrap refuses: exit 1, standard
/// error starting with the reason and naming `run-001`, its `evidence.toml`,
/// the observed and expected path and hash under the field names the Engine
/// records, and the command that retires the Run; no `Engine:` or `Pin:` line
/// on standard output; the whole fixture byte-identical. The same holds with
/// the record's status line changed to `executing`.
#[test]
fn lrpv_001() {
    let boot = Boot::two_state("lrpv001");
    let binary = boot.prebuild();
    start_run(&binary, &boot.root, "LRPV-001");
    let run = boot.root.join(".ratmac/runs/run-001");
    let record = run.join("run.toml");
    let written = fs::read_to_string(&record).expect("read the Run record");
    assert!(
        written
            .lines()
            .any(|line| line.trim() == "status = \"planned\""),
        "LRPV-001: the Engine wrote a live record: {written}"
    );
    let pinned = sha256_file(&binary);
    let pin = fs::read_to_string(run.join("evidence.toml")).expect("read the Run's pin");
    let identity = Evidence::load(&run)
        .engine
        .unwrap_or_else(|| panic!("LRPV-001: the Engine recorded an [engine] pin: {pin}"));
    assert!(
        resolved(Path::new(&identity.resolved)) == resolved(&binary) && identity.sha256 == pinned,
        "LRPV-001: the Engine pinned the planted binary and its hash: {pin}"
    );
    // HT-053-06: the pin carries the identity fields the refusal quotes.
    for field in ["resolved", "sha256"] {
        assert!(
            pin.contains(&format!("{field} = ")),
            "the Engine records {field} in the pin: {pin}"
        );
    }

    // Valid control: the unchanged pinned binary bootstraps and is reported.
    let control = boot.bootstrap();
    let report = text(&control);
    assert!(
        control.status.success(),
        "LRPV-001 control: the pinned binary bootstraps: {report}"
    );
    let engines = report_lines(&report, "Engine: ");
    assert!(
        engines.len() == 1 && resolved(Path::new(engines[0])) == resolved(&binary),
        "LRPV-001 control: the Engine: line names the planted binary: {report}"
    );
    assert_eq!(
        report_lines(&report, "sha256: "),
        [pinned.as_str()],
        "LRPV-001 control: sha256: is the pinned hash: {report}"
    );

    append_byte(&binary);
    let observed = sha256_file(&binary);
    assert_ne!(
        observed, pinned,
        "the fixture really changed the pinned binary"
    );
    for status in ["planned", "executing"] {
        if status == "executing" {
            let planned = fs::read_to_string(&record).expect("read the Run record");
            let executing = planned.replace("status = \"planned\"", "status = \"executing\"");
            assert_ne!(executing, planned, "the record's status line changes");
            fs::write(&record, executing).expect("mark the Run executing");
        }
        let before = boot.whole_snapshot();
        let refused = boot.bootstrap();
        assert_pin_refusal(
            &format!("LRPV-001 ({status})"),
            &refused,
            &[Refused {
                id: "run-001",
                shared: &boot.root,
                engine: &binary,
                observed: &observed,
                pinned: &pinned,
            }],
        );
        assert_eq!(
            boot.whole_snapshot(),
            before,
            "LRPV-001 ({status}): the refusal writes nothing"
        );
    }
}

/// PT-053-03 and HT-053-05: the doctor is actionable with and without a Run,
/// and provably writes nothing.
#[test]
fn doctor_is_actionable_and_write_free() {
    let boot = Boot::new("doctor");

    let before = boot.whole_snapshot();
    let idle = boot.rtm(&["doctor"]);
    let report = text(&idle);
    assert!(idle.status.success(), "doctor runs read-only: {report}");
    // FDC-004: the Scheduler-owned State File is named by its run-directory path.
    assert!(
        report.contains(".ratmac/ratmac.toml") && report.contains(".ratmac/runs/<id>/run.toml"),
        "the report distinguishes the two files by name: {report}"
    );
    assert!(
        report.contains("human-authored") && report.contains("Scheduler-owned"),
        "the report distinguishes them by role: {report}"
    );
    assert!(
        report.contains("no Run") && report.contains("rtm start"),
        "with no Run it names the next legitimate action: {report}"
    );
    assert_eq!(
        boot.whole_snapshot(),
        before,
        "the idle doctor writes nothing at all"
    );
    assert!(
        !boot.root.join(".ratmac/state.toml").exists() && !boot.root.join(".ratmac/runs").exists(),
        "the doctor creates no Run"
    );

    assert!(
        boot.rtm(&["start"]).status.success(),
        "the fixture Run starts"
    );
    let started = boot.whole_snapshot();
    let active = text(&boot.rtm(&["doctor"]));
    assert!(
        active.contains("state: build"),
        "with a Run it reports the state: {active}"
    );
    assert!(
        !active.contains("no Run on the roster"),
        "it does not still claim the Run is absent: {active}"
    );
    assert_eq!(
        boot.whole_snapshot(),
        started,
        "the doctor neither advances nor records the Run"
    );
}

/// PT-070-01: the human doctor report identifies the exact Engine with its
/// complete SHA-256 fingerprint while leaving the fixture untouched.
#[test]
fn doctor_reports_complete_engine_fingerprint_and_is_write_free() {
    let boot = Boot::new("doctor-full-fingerprint");
    let engine = boot.prebuild();
    let expected = sha256_file(&engine);
    let before = boot.whole_snapshot();

    let doctor = boot.rtm(&["doctor"]);
    let report = text(&doctor);
    assert!(
        doctor.status.success(),
        "argument-free doctor succeeds: {report}"
    );
    let reported = report
        .lines()
        .find_map(|line| {
            line.strip_prefix("Engine: ")
                .and_then(|line| line.rsplit_once(" (sha256: "))
                .and_then(|(_, sha256)| sha256.strip_suffix(')'))
        })
        .expect("human doctor report names the Engine SHA-256");
    assert_eq!(
        reported.len(),
        64,
        "the human report carries all 64 lowercase SHA-256 hexadecimal characters: {report}"
    );
    assert!(
        reported
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')),
        "the Engine SHA-256 is lowercase hexadecimal: {reported}"
    );
    assert_eq!(
        reported, expected,
        "the report fingerprint is the independent SHA-256 of the exact test-built Engine"
    );
    assert_eq!(
        boot.whole_snapshot(),
        before,
        "argument-free doctor writes nothing in the complete fixture"
    );
}

/// HT-053-02: nothing can be smuggled into the diagnosis.
///
/// t-053 read this as "no arguments at all". DRD-005 (t-056) is the later
/// accepted decision: `rtm doctor <path>` diagnoses a named runbook, and a path
/// that cannot be read is diagnosed as `RB101` rather than refused, because an
/// authoring loop asks about paths that do not exist yet. What survives is the
/// guarantee that mattered: an argument the interface does not offer refuses by
/// name, nothing prints the project diagnosis it was not asked for, every exit
/// is non-zero, and nothing is written.
#[test]
fn doctor_rejects_extra_arguments() {
    let boot = Boot::new("arguments");
    let before = boot.whole_snapshot();

    for (args, expected) in [
        (
            vec!["doctor", "--verbose"],
            "doctor accepts --json and one runbook path",
        ),
        (
            vec!["doctor", "--fix"],
            "doctor accepts --json and one runbook path",
        ),
        (
            vec!["doctor", ".ratmac/ratmac.toml", ".ratmac/ratmac.toml"],
            "doctor accepts --json and one runbook path",
        ),
        (vec!["doctor", "extra"], "RB101"),
    ] {
        let output = boot.rtm(&args);
        let report = text(&output);
        assert!(
            !output.status.success(),
            "{args:?} refuses deterministically: {report}"
        );
        assert!(
            report.contains(expected),
            "{args:?} names {expected:?}: {report}"
        );
        assert!(
            !report.contains("Engine: "),
            "{args:?} prints no diagnosis that could be read as success: {report}"
        );
    }

    assert_eq!(boot.whole_snapshot(), before, "refusals write nothing");
}

/// HT-053-04: a corrupt state file and a held lock are reported, not repaired
/// and not waited on.
#[test]
fn doctor_survives_corrupt_state_and_held_lock() {
    let boot = Boot::new("corrupt");
    // FDC-004: the State File resides in the run's directory.
    fs::create_dir_all(boot.root.join(".ratmac/runs/run-001")).expect("create run directory");
    fs::write(
        boot.root.join(".ratmac/runs/run-001/run.toml"),
        "state = \"build\nnot toml",
    )
    .expect("write a corrupt state file");
    let lock = boot.root.join(".ratmac/locks/root.lock");
    fs::create_dir_all(lock.parent().expect("lock has parent")).expect("create lock directory");
    fs::write(&lock, "held by another process\n").expect("hold the lock");
    let before = boot.whole_snapshot();

    let started = std::time::Instant::now();
    let output = boot.rtm(&["doctor"]);
    let elapsed = started.elapsed();
    let report = text(&output);

    assert!(
        report.contains(".ratmac/runs/run-001/run.toml") && report.contains("unreadable"),
        "the defect is reported: {report}"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "the doctor never blocks on the lock: {elapsed:?}"
    );
    assert_eq!(
        boot.whole_snapshot(),
        before,
        "neither the corrupt state nor the lock is touched"
    );
}

/// HT-053-01: adding the entry point leaves the documented CLI surface intact.
#[test]
fn cli_surface_survives_the_new_entry_point() {
    let boot = Boot::new("surface");
    let usage = text(&boot.rtm(&[]));
    for command in ["start", "status", "step", "hold", "abandon", "doctor"] {
        assert!(
            usage.contains(command),
            "the usage still lists {command}: {usage}"
        );
    }
    let help = text(&boot.rtm(&["doctor", "--help"]));
    assert!(
        help.contains("Usage: rtm doctor"),
        "doctor keeps its help text: {help}"
    );
    assert!(
        help.contains("Writes nothing"),
        "the help states the read-only contract: {help}"
    );
}

/// HT-053-05: the bootstrap installs nothing, fetches nothing, and mutates no
/// global state - audited over its whole source, not just its happy path.
#[test]
fn bootstrap_writes_only_declared_build_output() {
    let source = fs::read_to_string(bootstrap_source()).expect("read the bootstrap");
    for forbidden in [
        "Invoke-WebRequest",
        "Invoke-RestMethod",
        "Start-BitsTransfer",
        "curl",
        "wget",
        "Install-Module",
        "Install-Package",
        "winget",
        "choco",
        "setx",
        "cargo install",
        "--global",
        "$env:PATH =",
        "$env:Path =",
        "'fetch'",
        "'pull'",
        "'clone'",
    ] {
        assert!(
            !source.contains(forbidden),
            "the bootstrap must never carry {forbidden:?}"
        );
    }
    assert!(
        source.contains("--offline"),
        "the build runs offline by construction"
    );
    for declared in DECLARED_BUILD_OUTPUT {
        assert!(
            source.contains(declared),
            "the bootstrap names its declared output {declared}"
        );
    }

    let boot = Boot::new("output");
    let before = boot.snapshot();
    assert!(boot.bootstrap().status.success(), "the fixture bootstraps");
    assert_eq!(
        boot.snapshot(),
        before,
        "the build touches only the declared build output"
    );

    // A second run resolves the existing binary instead of rebuilding.
    let built = boot.root.join("target/debug").join(engine_file_name());
    let hash = sha256_file(&built);
    let again = text(&boot.bootstrap());
    assert!(
        again.contains(&hash),
        "the second run reports the same Engine: {again}"
    );
    assert_eq!(
        sha256_file(&built),
        hash,
        "the second run rebuilt nothing it did not have to"
    );
}

// --- SVE-002: the Engine is the executable Cargo reported -------------------

/// The variables that move Cargo's output folders. Every bootstrap this suite
/// starts clears all four and then sets only what its check names, so no
/// inherited redirect decides a verdict.
const REDIRECTS: [&str; 4] = [
    "CARGO_TARGET_DIR",
    "CARGO_BUILD_TARGET_DIR",
    "CARGO_BUILD_TARGET",
    "CARGO_BUILD_BUILD_DIR",
];

/// The refusal a failed build keeps.
const BUILD_FAILED: &str = "the project-local build did not produce an Engine";

/// The text after `prefix` on each report line that starts with it.
fn report_lines<'a>(report: &'a str, prefix: &str) -> Vec<&'a str> {
    report
        .lines()
        .filter_map(|line| line.strip_prefix(prefix))
        .map(str::trim)
        .collect()
}

/// Text with `/` separators and no verbatim prefix, and without case on
/// Windows, where paths compare without it.
fn spelled(text: &str) -> String {
    let text = text.replace('\\', "/");
    let text = text.strip_prefix("//?/").unwrap_or(&text);
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text.to_owned()
    }
}

/// `path` as it resolves on disk, spelled for comparison.
fn resolved(path: &Path) -> String {
    let canonical = path
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve {}: {error}", path.display()));
    spelled(&canonical.to_string_lossy())
}

/// Whether `path` lies below `folder`, both as they resolve on disk.
fn inside(path: &Path, folder: &Path) -> bool {
    resolved(path).starts_with(&format!("{}/", resolved(folder).trim_end_matches('/')))
}

/// Record an Engine pin where the bootstrap reads it (LRP-001): the
/// `evidence.toml` of a live Run, `run-001`, in the fixture's `.ratmac`,
/// whose record reads `status = "planned"`, through the Engine's own pin
/// record. SVEV-004 and SVEV-005 recorded their pins in the flat
/// `.ratmac/evidence.toml` until LRP-001 stopped the bootstrap reading it.
fn record_pin(root: &Path, engine: &Path, sha256: &str) {
    let run = root.join(".ratmac/runs/run-001");
    write_pin(&run, engine, sha256);
    fs::write(
        run.join("run.toml"),
        "state = \"build\"\nstatus = \"planned\"\n",
    )
    .expect("record a live Run");
}

/// Change the fixture's `rtm` source so it no longer compiles.
fn break_source(root: &Path) {
    fs::write(
        root.join("src/main.rs"),
        "fn main() { this_does_not_compile }\n",
    )
    .expect("break the fixture source");
}

/// A `.cargo/config.toml` in the fixture whose `build.target-dir` is `folder`.
fn configure_target_dir(root: &Path, folder: &str) {
    fs::create_dir_all(root.join(".cargo")).expect("create the Cargo configuration folder");
    fs::write(
        root.join(".cargo/config.toml"),
        format!("[build]\ntarget-dir = \"{folder}\"\n"),
    )
    .expect("write the Cargo configuration");
}

/// A stand-in `cargo` in `bin`. It answers `cargo metadata` with `folder` as
/// both output folders, and the build with success and no artifact record at
/// all, so Cargo reports no `rtm` executable.
fn stand_in_cargo(bin: &Path, folder: &Path, root: &Path) {
    let slashed = |path: &Path| path.to_string_lossy().replace('\\', "/");
    let metadata = format!(
        "{{\"version\":1,\"packages\":[],\"workspace_members\":[],\
         \"target_directory\":\"{folder}\",\"build_directory\":\"{folder}\",\
         \"workspace_root\":\"{root}\"}}",
        folder = slashed(folder),
        root = slashed(root),
    );
    let finished = "{\"reason\":\"build-finished\",\"success\":true}";
    // Labels, not parenthesised blocks: a `)` in an echoed line would end one.
    #[cfg(windows)]
    {
        fs::write(
            bin.join("cargo.cmd"),
            format!(
                "@echo off\r\nif \"%~1\"==\"metadata\" goto metadata\r\necho {finished}\r\n\
                 exit /b 0\r\n:metadata\r\necho {metadata}\r\nexit /b 0\r\n"
            ),
        )
        .expect("write the stand-in cargo");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script = bin.join("cargo");
        fs::write(
            &script,
            format!(
                "#!/bin/sh\nif [ \"$1\" = metadata ]; then\n  printf '%s\\n' '{metadata}'\n  exit 0\nfi\n\
                 printf '%s\\n' '{finished}'\nexit 0\n"
            ),
        )
        .expect("write the stand-in cargo");
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755))
            .expect("make the stand-in cargo executable");
    }
}

/// A child's `PATH`: `first`, then this process's own.
fn child_path(first: &Path) -> OsString {
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    std::env::join_paths(
        std::iter::once(first.to_path_buf()).chain(std::env::split_paths(&inherited)),
    )
    .expect("join the child PATH")
}

/// SVEV-004 (SVE-002): the fixture's output moved in turn by
/// `CARGO_TARGET_DIR` to a folder outside the fixture, by a `build.target-dir`
/// setting in its `.cargo/config.toml` naming a folder outside it, and by one
/// naming a folder inside it other than `target`. Each first run exits 0, its
/// `Engine:` line names the executable Cargo reported in that folder,
/// `sha256:` is that file's hash, and with a live Run's pin recorded for it
/// (LRP-001) the pin reads `Pin: matches`. With the source then broken, the
/// folder inside the fixture is reused with the same lines; with a folder
/// outside, the second run builds first and refuses as a failed build.
#[test]
fn svev_004() {
    for case in ["environment", "config-outside", "config-inside"] {
        let boot = Boot::new(&format!("svev004-{case}"));
        let outside = TempTree::new(&format!("t053-svev004-{case}-output"))
            .expect("create the folder outside the fixture");
        let mut set: Vec<(&str, &OsStr)> = Vec::new();
        let folder = match case {
            "environment" => {
                set.push(("CARGO_TARGET_DIR", outside.path().as_os_str()));
                outside.path().to_path_buf()
            }
            "config-outside" => {
                let named = outside.path().to_string_lossy().replace('\\', "/");
                configure_target_dir(&boot.root, &named);
                outside.path().to_path_buf()
            }
            _ => {
                configure_target_dir(&boot.root, "out");
                boot.root.join("out")
            }
        };

        let first = boot.bootstrap_env(&set, &REDIRECTS);
        let report = text(&first);
        assert!(
            first.status.success(),
            "SVEV-004 ({case}): the first run exits 0 with the executable Cargo reported in \
             {}: {report}",
            folder.display()
        );
        let engines = report_lines(&report, "Engine: ");
        assert_eq!(
            engines.len(),
            1,
            "SVEV-004 ({case}): one Engine: line: {report}"
        );
        let engine = PathBuf::from(engines[0]);
        assert!(
            engine.is_file() && engine.file_name() == Some(OsStr::new(engine_file_name())),
            "SVEV-004 ({case}): the Engine: line names an rtm executable: {report}"
        );
        assert!(
            inside(&engine, &folder),
            "SVEV-004 ({case}): the Engine is the file Cargo reported in {}, never one at \
             another path: {report}",
            folder.display()
        );
        let hash = sha256_file(&engine);
        assert_eq!(
            report_lines(&report, "sha256: "),
            [hash.as_str()],
            "SVEV-004 ({case}): sha256: is that file's hash: {report}"
        );

        record_pin(&boot.root, &engine, &hash);
        let pinned = boot.bootstrap_env(&set, &REDIRECTS);
        let pinned_report = text(&pinned);
        assert!(
            pinned.status.success(),
            "SVEV-004 ({case}): with the pin recorded for that file the run exits 0: \
             {pinned_report}"
        );
        assert_eq!(
            (
                report_lines(&pinned_report, "Engine: "),
                report_lines(&pinned_report, "sha256: "),
            ),
            (engines.clone(), vec![hash.as_str()]),
            "SVEV-004 ({case}): the pinned run reports the same Engine: {pinned_report}"
        );
        assert!(
            pinned_report.contains("Pin: matches"),
            "SVEV-004 ({case}): the pin reads `Pin: matches`: {pinned_report}"
        );

        break_source(&boot.root);
        let second = boot.bootstrap_env(&set, &REDIRECTS);
        let second_report = text(&second);
        if case == "config-inside" {
            assert!(
                second.status.success(),
                "SVEV-004 ({case}): with the folder inside the fixture a second run reuses \
                 the Engine even with the source broken: {second_report}"
            );
            assert_eq!(
                (
                    report_lines(&second_report, "Engine: "),
                    report_lines(&second_report, "sha256: "),
                ),
                (engines.clone(), vec![hash.as_str()]),
                "SVEV-004 ({case}): the reuse reports the same Engine: and sha256: lines: \
                 {second_report}"
            );
        } else {
            assert_eq!(
                second.status.code(),
                Some(1),
                "SVEV-004 ({case}): with the folder outside the fixture the second run \
                 builds first and refuses: {second_report}"
            );
            assert!(
                second_report.contains(BUILD_FAILED),
                "SVEV-004 ({case}): the refusal is the failed build's: {second_report}"
            );
            assert!(
                report_lines(&second_report, "Engine: ").is_empty(),
                "SVEV-004 ({case}): a refusal prints no Engine: line: {second_report}"
            );
        }
    }
}

/// SVEV-005 (SVE-002), project root: with `CARGO_TARGET_DIR` naming an empty
/// folder outside the fixture and stand-in binaries of different bytes at the
/// fixture's `target/release` and `target/debug`, the bootstrap builds and
/// reports the fresh binary's path and hash, never a stand-in's, and the
/// stand-ins stay byte-identical; a live Run pinning a stand-in's hash for the
/// reported Engine refuses with "the resolved Engine is not the pinned
/// Engine" (LRP-001 refuses only for the pinned file, so the pin names the
/// reported path).
#[test]
fn svev_005() {
    let boot = Boot::new("svev005");
    let output = TempTree::new("t053-svev005-output").expect("create the redirect folder");
    let stand_ins: Vec<(PathBuf, &[u8])> = [
        ("target/release", b"stand-in release\n".as_slice()),
        ("target/debug", b"stand-in debug\n".as_slice()),
    ]
    .into_iter()
    .map(|(folder, bytes)| {
        let folder = boot.root.join(folder);
        fs::create_dir_all(&folder).expect("create the stand-in folder");
        let stand_in = folder.join(engine_file_name());
        fs::write(&stand_in, bytes).expect("plant a stand-in");
        (stand_in, bytes)
    })
    .collect();
    let stand_in_hashes: Vec<String> = stand_ins
        .iter()
        .map(|(stand_in, _)| sha256_file(stand_in))
        .collect();
    let set = [("CARGO_TARGET_DIR", output.path().as_os_str())];

    // Valid control: the first run exits 0 with a real build.
    let built = boot.bootstrap_env(&set, &REDIRECTS);
    let report = text(&built);
    assert!(
        built.status.success(),
        "SVEV-005: the bootstrap builds and exits 0: {report}"
    );
    let engines = report_lines(&report, "Engine: ");
    assert_eq!(engines.len(), 1, "SVEV-005: one Engine: line: {report}");
    let engine = PathBuf::from(engines[0]);
    assert!(
        engine.is_file() && inside(&engine, output.path()),
        "SVEV-005: the Engine is the fresh binary Cargo built in the redirect folder {}, \
         never a stand-in under the fixture's target: {report}",
        output.path().display()
    );
    let hash = sha256_file(&engine);
    assert_eq!(
        report_lines(&report, "sha256: "),
        [hash.as_str()],
        "SVEV-005: sha256: is the fresh binary's hash: {report}"
    );
    assert!(
        !stand_in_hashes.contains(&hash),
        "SVEV-005: the reported hash is no stand-in's: {report}"
    );
    for (stand_in, bytes) in &stand_ins {
        assert_eq!(
            fs::read(stand_in).expect("read a stand-in"),
            *bytes,
            "SVEV-005: the stand-in {} stays byte-identical",
            stand_in.display()
        );
    }

    // A pin naming the release stand-in's hash refuses.
    record_pin(&boot.root, &engine, &stand_in_hashes[0]);
    let refused = boot.bootstrap_env(&set, &REDIRECTS);
    let refusal = text(&refused);
    assert_eq!(
        refused.status.code(),
        Some(1),
        "SVEV-005: a pin naming a stand-in's hash refuses: {refusal}"
    );
    assert!(
        refusal.contains("the resolved Engine is not the pinned Engine"),
        "SVEV-005: the refusal is the pin mismatch: {refusal}"
    );
    assert!(
        report_lines(&String::from_utf8_lossy(&refused.stdout), "Engine: ").is_empty(),
        "SVEV-005: the refusal reports no Engine: {refusal}"
    );
    for (stand_in, bytes) in &stand_ins {
        assert_eq!(
            fs::read(stand_in).expect("read a stand-in"),
            *bytes,
            "SVEV-005: the stand-in {} stays byte-identical",
            stand_in.display()
        );
    }
}

/// SVEV-006 (SVE-002): a stand-in `cargo` first on the child's `PATH` succeeds
/// but reports no `rtm` executable and resolves an empty output folder, and a
/// stand-in binary sits at the fixture's `target/debug`. The bootstrap exits 1
/// with a `bootstrap refused:` line naming what it searched and the repair,
/// prints no `Engine:` line, and never names the stand-in.
#[test]
fn svev_006() {
    let boot = Boot::new("svev006");
    let planted = boot.root.join("target/debug");
    fs::create_dir_all(&planted).expect("create the stand-in folder");
    let stand_in = planted.join(engine_file_name());
    fs::write(&stand_in, b"stand-in\n").expect("plant the stand-in binary");
    let stand_in_hash = sha256_file(&stand_in);

    // Valid control: the same fixture with the toolchain's own cargo exits 0,
    // so the refusal below is Cargo's answer, never a fixture that cannot run.
    let control_output =
        TempTree::new("t053-svev006-control").expect("create the control's output folder");
    let control = boot.bootstrap_env(
        &[("CARGO_TARGET_DIR", control_output.path().as_os_str())],
        &REDIRECTS,
    );
    assert!(
        control.status.success(),
        "SVEV-006 control: without the stand-in cargo the fixture bootstraps: {}",
        text(&control)
    );

    let empty = TempTree::new("t053-svev006-empty").expect("create the empty output folder");
    let bin = TempTree::new("t053-svev006-bin").expect("create the stand-in cargo folder");
    stand_in_cargo(bin.path(), empty.path(), &boot.root);
    let path = child_path(bin.path());
    let refused = boot.bootstrap_env(&[("PATH", path.as_os_str())], &REDIRECTS);
    let report = text(&refused);
    assert_eq!(
        refused.status.code(),
        Some(1),
        "SVEV-006: when Cargo reports no Engine the bootstrap refuses with exit 1: {report}"
    );
    let stderr = String::from_utf8_lossy(&refused.stderr);
    let refusal = stderr
        .lines()
        .find(|line| line.starts_with("bootstrap refused:"))
        .unwrap_or_else(|| {
            panic!("SVEV-006: a `bootstrap refused:` line on standard error: {report}")
        });
    assert!(
        refusal.contains("searched:")
            && refusal.contains("Cargo reported")
            && refusal.contains("repair:"),
        "SVEV-006: the `bootstrap refused:` line names what it searched, what Cargo reported and the repair: \
         {refusal}"
    );
    assert!(
        report_lines(&report, "Engine: ").is_empty(),
        "SVEV-006: a refusal prints no Engine: line: {report}"
    );
    let shown = spelled(&report);
    let target = boot.root.join("target");
    for spelling in [
        spelled(&target.to_string_lossy()),
        resolved(&target),
        spelled(&format!("target/debug/{}", engine_file_name())),
    ] {
        assert!(
            !shown.contains(&spelling),
            "SVEV-006: the bootstrap never names the stand-in ({spelling}): {report}"
        );
    }
    assert!(
        !report.contains(&stand_in_hash),
        "SVEV-006: the refusal never names the stand-in's hash: {report}"
    );
}

// --- LRP-001: the live Run pin check -------------------------------------------

impl Boot {
    /// The planted-binary bootstrap, asserting that every file in the fixture
    /// stays byte-identical around it.
    fn bootstrap_unchanged(&self, check: &str) -> Output {
        let before = self.whole_snapshot();
        let output = self.bootstrap();
        assert_eq!(
            self.whole_snapshot(),
            before,
            "{check}: the bootstrap writes nothing: {}",
            text(&output)
        );
        output
    }
}

/// `rtm start` run by `engine` from `directory` through the shared support's
/// scrubbed command, so the Engine pins the very file that started it.
fn start_run(engine: &Path, directory: &Path, check: &str) {
    let started = support::command(engine, directory)
        .arg("start")
        .output()
        .expect("invoke the planted Engine");
    assert!(
        started.status.success(),
        "{check}: {} starts a Run: {}",
        engine.display(),
        text(&started)
    );
}

/// Append one byte to `path`. An Engine that has just run may still be held
/// on Windows (a scanner reading it: sharing or access errors), so the
/// append retries for up to five seconds.
fn append_byte(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let appended = fs::OpenOptions::new()
            .append(true)
            .open(path)
            .and_then(|mut file| file.write_all(&[0]));
        match appended {
            Ok(()) => return,
            Err(error)
                if matches!(error.raw_os_error(), Some(5 | 32 | 33))
                    && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(error) => panic!("append a byte to {}: {error}", path.display()),
        }
    }
}

/// Record an Engine pin in `folder`'s `evidence.toml` through the Engine's own
/// pin record.
fn write_pin(folder: &Path, engine: &Path, sha256: &str) {
    let mut evidence = Evidence::load(folder);
    evidence.set_engine(Identity {
        resolved: engine.to_string_lossy().into_owned(),
        sha256: sha256.to_owned(),
        ..Identity::default()
    });
    evidence.write(folder).expect("record the Engine pin");
}

/// The text after `Pin: ` on each standard-output line that starts with it.
fn pin_lines(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("Pin: "))
        .map(|rest| rest.trim().to_owned())
        .collect()
}

/// Whether `pins` is the one line `no live Run in` naming `root`'s `.ratmac`.
fn no_live_run(pins: &[String], root: &Path) -> bool {
    pins.len() == 1
        && pins[0]
            .strip_prefix("no live Run in ")
            .is_some_and(|named| resolved(Path::new(named)) == resolved(&root.join(".ratmac")))
}

/// Whether `text` names `suffix` below the folder `base`. Git and PowerShell
/// spell one folder with different separators, so the comparison runs through
/// `spelled` on `base`'s own name (its `TempTree` leaf) and the suffix.
fn names_below(text: &str, base: &Path, suffix: &str) -> bool {
    let leaf = base
        .file_name()
        .expect("a fixture folder has a name")
        .to_string_lossy();
    spelled(text).contains(&spelled(&format!("{leaf}/{suffix}")))
}

/// `toml` without its `[name]` table: the header and every line up to the
/// next table header.
fn without_table(toml: &str, name: &str) -> String {
    let header = format!("[{name}]");
    let mut inside = false;
    toml.lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                inside = trimmed == header;
            }
            !inside
        })
        .map(|line| format!("{line}\n"))
        .collect()
}

/// One Run the pin refusal must name: its id, the checkout whose `.ratmac`
/// holds it, the Engine file it pinned (the resolved one), and the observed
/// and pinned hashes.
struct Refused<'a> {
    id: &'a str,
    shared: &'a Path,
    engine: &'a Path,
    observed: &'a str,
    pinned: &'a str,
}

/// LRP-001's one pin refusal: exit 1; standard error starts with the reason
/// and names each Run in `runs` with its `evidence.toml` under the shared
/// root, the observed and expected path and hash under the field names the
/// Engine records, and the command that retires it, once; standard output
/// has no `Engine:` or `Pin:` line.
fn assert_pin_refusal(check: &str, output: &Output, runs: &[Refused]) {
    let report = text(output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{check}: a changed pinned Engine refuses with exit 1: {report}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with(PIN_REFUSAL),
        "{check}: standard error starts with the pin refusal: {report}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout
            .lines()
            .any(|line| line.starts_with("Engine:") || line.starts_with("Pin:")),
        "{check}: the refusal prints no Engine: or Pin: line: {report}"
    );
    let guidance: Vec<&str> = stderr.lines().map(str::trim).collect();
    for field in ["observed", "expected"] {
        let paths: Vec<&str> = guidance
            .iter()
            .filter_map(|line| line.strip_prefix(&format!("resolved ({field}) = ")))
            .collect();
        assert_eq!(
            paths.len(),
            runs.len(),
            "{check}: one `resolved ({field})` line per refused Run: {report}"
        );
        for (path, run) in paths.iter().zip(runs) {
            assert_eq!(
                resolved(Path::new(path)),
                resolved(run.engine),
                "{check}: `resolved ({field})` names the pinned Engine file: {report}"
            );
        }
    }
    for run in runs {
        let id = run.id;
        let evidence = format!(".ratmac/runs/{id}/evidence.toml");
        assert!(
            guidance
                .iter()
                .any(|line| line.contains(id) && names_below(line, run.shared, &evidence)),
            "{check}: the refusal names {id} with its pin file {evidence} under {}: {report}",
            run.shared.display()
        );
        for (field, hash) in [("observed", run.observed), ("expected", run.pinned)] {
            assert!(
                guidance.contains(&format!("sha256 ({field}) = {hash}").as_str()),
                "{check}: the refusal quotes `sha256 ({field}) = {hash}` for {id}: {report}"
            );
        }
        let retire = format!("rtm abandon --run {id} --confirm \"abandon {id}\"");
        assert_eq!(
            stderr.matches(&retire).count(),
            1,
            "{check}: the refusal names `{retire}` once: {report}"
        );
    }
}

/// LRPV-002 (LRP-001): one `Pin:` line per live Run, in Run-id order. With
/// `run-001` started by the planted Engine the bootstrap exits 0 with
/// `Pin: matches the pin of run-001`; with `run-002` started by a copy at
/// another path it still exits 0 and prints the `run-001` line, then
/// `Pin: run-002 pinned another Engine:` naming the copy. A passed record, a
/// Run folder with no record, and a record that is a folder, each pinning the
/// resolved path with another hash, add no line and no refusal. With no live
/// Run the one `Pin:` line is `Pin: no live Run in` naming the fixture's
/// `.ratmac`. No bootstrap writes anything.
#[test]
fn lrpv_002() {
    let boot = Boot::two_state("lrpv002");
    let binary = boot.prebuild();
    start_run(&binary, &boot.root, "LRPV-002");
    let one = boot.bootstrap_unchanged("LRPV-002 (run-001)");
    let report = text(&one);
    assert!(
        one.status.success(),
        "LRPV-002: the pinned Engine bootstraps: {report}"
    );
    assert_eq!(
        pin_lines(&one),
        ["matches the pin of run-001"],
        "LRPV-002: the bootstrap prints `Pin: matches the pin of run-001`: {report}"
    );

    // The flat `.ratmac/evidence.toml` is never read: while run-001 is live, a
    // flat pin naming the binary with another hash adds no line and no
    // refusal.
    let flat = boot.root.join(".ratmac/evidence.toml");
    let other = format!("{:x}", Sha256::digest(b"another Engine"));
    write_pin(&boot.root.join(".ratmac"), &binary, &other);
    assert!(
        fs::read_to_string(&flat).is_ok_and(|pin| pin.contains("[engine]") && pin.contains(&other)),
        "LRPV-002: the flat pin is planted"
    );
    let beside = boot.bootstrap_unchanged("LRPV-002 (run-001, stale flat pin)");
    let report = text(&beside);
    assert!(
        beside.status.success(),
        "LRPV-002: a stale flat pin beside a live Run refuses nothing: {report}"
    );
    assert_eq!(
        pin_lines(&beside),
        ["matches the pin of run-001"],
        "LRPV-002: a stale flat pin beside a live Run adds no Pin: line: {report}"
    );
    fs::remove_file(&flat).expect("remove the flat pin");

    let copy = boot.root.join("copies").join(engine_file_name());
    fs::create_dir_all(boot.root.join("copies")).expect("create the copies folder");
    fs::copy(&binary, &copy).expect("copy the Engine to another path");
    start_run(&copy, &boot.root, "LRPV-002");
    let two = boot.bootstrap_unchanged("LRPV-002 (run-002)");
    let report = text(&two);
    assert!(
        two.status.success(),
        "LRPV-002: a Run that pinned another file refuses nothing: {report}"
    );
    let pins = pin_lines(&two);
    assert!(
        pins.len() == 2
            && pins[0] == "matches the pin of run-001"
            && pins[1]
                .strip_prefix("run-002 pinned another Engine: ")
                .is_some_and(|named| resolved(Path::new(named)) == resolved(&copy)),
        "LRPV-002: the run-001 line, then run-002's naming the copy {}: {report}",
        copy.display()
    );

    let runs = boot.root.join(".ratmac/runs");
    let stale = "0".repeat(64);
    for id in ["run-003", "run-004", "run-005"] {
        write_pin(&runs.join(id), &binary, &stale);
    }
    let live = fs::read_to_string(runs.join("run-001/run.toml")).expect("read run-001's record");
    let passed = live.replace("status = \"planned\"", "status = \"passed\"");
    assert_ne!(passed, live, "the passed record differs from the live one");
    fs::write(runs.join("run-003/run.toml"), passed).expect("write a passed record");
    fs::create_dir_all(runs.join("run-005/run.toml")).expect("make a record that is a folder");
    let more = boot.bootstrap_unchanged("LRPV-002 (run-003..run-005)");
    let report = text(&more);
    assert!(
        more.status.success(),
        "LRPV-002: Runs that are not live refuse nothing: {report}"
    );
    assert_eq!(
        pin_lines(&more),
        pins,
        "LRPV-002: Runs that are not live add no Pin: line: {report}"
    );
    for id in ["run-003", "run-004", "run-005"] {
        assert!(
            !report.contains(id),
            "LRPV-002: {id} appears nowhere in the report: {report}"
        );
    }

    let idle = Boot::two_state("lrpv002-idle");
    idle.prebuild();
    let output = idle.bootstrap_unchanged("LRPV-002 (no live Run)");
    let report = text(&output);
    assert!(
        output.status.success(),
        "LRPV-002: with no live Run the bootstrap exits 0: {report}"
    );
    assert!(
        no_live_run(&pin_lines(&output), &idle.root),
        "LRPV-002: the one Pin: line is `no live Run in` the fixture's .ratmac: {report}"
    );
}

/// LRPV-003 (LRP-001): the flat `.ratmac/evidence.toml` is never read. With no
/// live Run and a flat `[engine]` table naming the resolved binary with
/// another hash, the bootstrap exits 0, prints `Pin: no live Run in` naming
/// the fixture's `.ratmac`, never names the flat file, and writes nothing.
#[test]
fn lrpv_003() {
    let boot = Boot::new("lrpv003");
    let binary = boot.prebuild();
    let control = boot.bootstrap();
    assert!(
        control.status.success(),
        "LRPV-003 control: the planted Engine bootstraps: {}",
        text(&control)
    );
    let reported = sha256_file(&binary);
    let other = format!("{:x}", Sha256::digest(b"another Engine"));
    // The flat pin the bootstrap read before LRP-001.
    write_pin(&boot.root.join(".ratmac"), &binary, &other);
    let flat =
        fs::read_to_string(boot.root.join(".ratmac/evidence.toml")).expect("read the flat pin");
    assert!(
        flat.contains("[engine]") && flat.contains(&other) && other != reported,
        "LRPV-003: the flat file pins the resolved binary with another hash: {flat}"
    );

    let output = boot.bootstrap_unchanged("LRPV-003");
    let report = text(&output);
    assert!(
        output.status.success(),
        "LRPV-003: the flat pin causes no refusal: {report}"
    );
    assert!(
        no_live_run(&pin_lines(&output), &boot.root),
        "LRPV-003: the one Pin: line is `no live Run in` the fixture's .ratmac: {report}"
    );
    assert!(
        !spelled(&report).contains(".ratmac/evidence.toml") && !report.contains("pinned Engine"),
        "LRPV-003: the bootstrap never names the flat file: {report}"
    );
}

/// LRPV-004 (LRP-001): a linked worktree's shared Engine root is the primary
/// checkout's `.ratmac`. A Git repository with one commit holding the
/// two-State runbook, the source, and `tools/rtm.ps1`, with `target/`
/// ignored, and a linked worktree beside it whose `target/debug` holds the
/// planted Engine; that Engine, run in the worktree, starts `run-001`, whose
/// record lands under the primary checkout. The worktree's bootstrap exits 0
/// with `Pin: matches the pin of run-001`; with one byte appended to the
/// Engine it exits 1 naming `run-001` and its evidence file under the primary
/// checkout. With `GIT_DIR` naming the primary checkout's Git folder in the
/// bootstrap's environment both outcomes stay the same.
#[test]
fn lrpv_004() {
    let repo = TempRepo::new("t145-primary");
    let primary = repo.root().to_path_buf();
    for dir in [".ratmac", "src", "tools"] {
        fs::create_dir_all(primary.join(dir)).expect("create the repository tree");
    }
    fs::write(primary.join("Cargo.toml"), FIXTURE_MANIFEST).expect("write the manifest");
    fs::write(primary.join("src/main.rs"), "fn main() {}\n").expect("write the source");
    fs::write(primary.join(".ratmac/ratmac.toml"), TWO_STATE_RUNBOOK).expect("write the runbook");
    fs::write(primary.join(".gitignore"), "target/\n").expect("ignore the build output");
    fs::copy(bootstrap_source(), primary.join("tools/rtm.ps1")).expect("install the bootstrap");
    repo.commit_all("t-145: a project with a two-State runbook");

    let side = TempTree::new("t145-linked").expect("create the folder beside the repository");
    let worktree = side.path().join("wt");
    let linked = repo.git(&[
        "worktree",
        "add",
        "--detach",
        worktree.to_str().expect("a UTF-8 worktree path"),
    ]);
    assert!(
        linked.status.success(),
        "LRPV-004: link the worktree: {}",
        text(&linked)
    );
    let engine = plant_engine(&worktree);
    // With no live Run yet, the worktree's one `Pin:` line names the shared
    // Engine root, the primary checkout's `.ratmac`, not its own.
    let idle = bootstrap_planted(&worktree, &[]);
    assert!(
        idle.status.success() && no_live_run(&pin_lines(&idle), &primary),
        "LRPV-004: with no live Run the worktree names the primary checkout's .ratmac: {}",
        text(&idle)
    );
    start_run(&engine, &worktree, "LRPV-004");
    assert!(
        primary.join(".ratmac/runs/run-001/run.toml").is_file()
            && !worktree.join(".ratmac/runs").exists(),
        "LRPV-004: the worktree's Run lands under the primary checkout"
    );

    let pinned = sha256_file(&engine);
    let git_dir = primary.join(".git");
    let environments: [(&str, Vec<(&str, &OsStr)>); 2] = [
        ("LRPV-004", Vec::new()),
        ("LRPV-004 (GIT_DIR)", vec![("GIT_DIR", git_dir.as_os_str())]),
    ];
    for (check, set) in &environments {
        let output = bootstrap_planted(&worktree, set);
        let report = text(&output);
        assert!(
            output.status.success(),
            "{check}: the worktree bootstraps: {report}"
        );
        assert_eq!(
            pin_lines(&output),
            ["matches the pin of run-001"],
            "{check}: the worktree reads the primary checkout's Run: {report}"
        );
    }

    append_byte(&engine);
    let observed = sha256_file(&engine);
    for (check, set) in &environments {
        assert_pin_refusal(
            check,
            &bootstrap_planted(&worktree, set),
            &[Refused {
                id: "run-001",
                shared: &primary,
                engine: &engine,
                observed: &observed,
                pinned: &pinned,
            }],
        );
    }
}

/// LRPV-005 (LRP-001): a live Run without a readable pin gets a line, never a
/// refusal or a PowerShell error. `run-001` started as in the
/// changed-pinned-file check, its `evidence.toml` replaced first by a folder
/// of that name, then by a file with no `[engine]` table: each time the
/// bootstrap exits 0, prints `Pin: run-001 has no readable pin in` naming that
/// path, shows no PowerShell error record on standard error, and writes
/// nothing.
#[test]
fn lrpv_005() {
    let boot = Boot::two_state("lrpv005");
    let binary = boot.prebuild();
    start_run(&binary, &boot.root, "LRPV-005");
    let run = boot.root.join(".ratmac/runs/run-001");
    let evidence = run.join("evidence.toml");
    let recorded = fs::read_to_string(&evidence).expect("read the Run's pin");
    // Valid control: the Engine's own reader finds the planted binary's pin,
    // and the bootstrap runs, before the pin is made unreadable.
    assert!(
        Evidence::load(&run)
            .engine
            .is_some_and(|pin| resolved(Path::new(&pin.resolved)) == resolved(&binary)),
        "LRPV-005 control: run-001 pins the planted binary: {recorded}"
    );
    let control = boot.bootstrap();
    assert!(
        control.status.success(),
        "LRPV-005 control: the pinned Engine bootstraps: {}",
        text(&control)
    );

    let unpinned = without_table(&recorded, "engine");
    assert!(
        recorded.contains("[engine]") && !unpinned.contains("[engine]"),
        "LRPV-005: the pin without its [engine] table: {unpinned}"
    );
    fs::remove_file(&evidence).expect("remove the pin");
    fs::create_dir(&evidence).expect("put a folder where the pin was");
    for case in ["a folder", "no [engine] table"] {
        if case == "no [engine] table" {
            fs::remove_dir(&evidence).expect("remove the folder");
            fs::write(&evidence, &unpinned).expect("write a pin with no [engine] table");
        }
        let check = format!("LRPV-005 ({case})");
        let output = boot.bootstrap_unchanged(&check);
        let report = text(&output);
        assert!(
            output.status.success(),
            "{check}: an unreadable pin refuses nothing: {report}"
        );
        let pins = pin_lines(&output);
        assert!(
            pins.len() == 1
                && pins[0].starts_with("run-001 has no readable pin in ")
                && names_below(&pins[0], &boot.root, ".ratmac/runs/run-001/evidence.toml"),
            "{check}: one Pin: line naming run-001's unreadable pin: {report}"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.trim().is_empty(),
            "{check}: no PowerShell error record on standard error: {stderr}"
        );
    }
}
