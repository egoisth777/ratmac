//! t-053 / ORS-002: project-local bootstrap and read-only doctor.
//!
//! PT-053-01 `bootstrap_resolves_offline`
//! PT-053-02 `bootstrap_refuses_pin_mismatch`
//! PT-053-03 `doctor_is_actionable_and_write_free`
//! HT-053-01 `cli_surface_survives_the_new_entry_point`
//! HT-053-02 `doctor_rejects_extra_arguments`
//! HT-053-04 `doctor_survives_corrupt_state_and_held_lock`
//! HT-053-05 `bootstrap_writes_only_declared_build_output`
//! HT-053-06 `bootstrap_refusal_quotes_recorded_identity_fields`
//!
//! A fresh session must be able to orient and start without ad-hoc installs:
//! one documented command resolves or builds the Engine offline and refuses on
//! pin mismatch, and the doctor diagnoses the project while writing nothing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ratmac::pin::{Evidence, Identity};
use sha2::{Digest, Sha256};

/// Cargo writes these while building; everything else must stay byte-identical.
const DECLARED_BUILD_OUTPUT: [&str; 2] = ["target", "Cargo.lock"];

struct Boot {
    root: PathBuf,
}

impl Drop for Boot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Boot {
    /// A throwaway project root that carries the real `tools/rtm.ps1`, a
    /// Runbook, and a buildable `rtm` binary target.
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t053-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in [".ratmac", "src", "tools"] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\n\n[package]\nname = \"ratmac-bootstrap-fixture\"\n\
             version = \"0.0.0\"\nedition = \"2021\"\n\n\
             [[bin]]\nname = \"rtm\"\npath = \"src/main.rs\"\n",
        )
        .expect("write fixture manifest");
        fs::write(root.join("src/main.rs"), "fn main() {}\n").expect("write fixture source");
        fs::write(
            root.join(".ratmac/ratmac.toml"),
            "[states.build]\nprompt = \"Build the selected artifact.\"\n",
        )
        .expect("write runbook");
        fs::copy(bootstrap_source(), root.join("tools/rtm.ps1")).expect("install the bootstrap");
        Boot { root }
    }

    /// Put a real executable where the bootstrap resolves it, without building.
    fn prebuild(&self) -> PathBuf {
        let target = self.root.join("target/debug");
        fs::create_dir_all(&target).expect("create build directory");
        let binary = target.join(engine_file_name());
        fs::copy(env!("CARGO_BIN_EXE_rtm"), &binary).expect("place a prebuilt Engine");
        binary
    }

    fn bootstrap(&self) -> Output {
        Command::new("pwsh")
            .args(["-NoProfile", "-File", "tools/rtm.ps1"])
            .current_dir(&self.root)
            .output()
            .expect("invoke the bootstrap")
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
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
    assert!(
        report.contains("no pin recorded"),
        "with no pin the report says so rather than claiming a match: {report}"
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

/// PT-053-02 and HT-053-06: a pin that no longer matches the binary refuses,
/// naming the same identity fields the Engine records.
#[test]
fn bootstrap_refuses_pin_mismatch() {
    let boot = Boot::new("pin");
    let binary = boot.prebuild();
    let pinned = sha256_file(&binary);

    // The bootstrap reads the Engine `.ratmac/evidence.toml` pin;
    // Evidence::load/write take the directory holding `evidence.toml`.
    let mut evidence = Evidence::load(&boot.root.join(".ratmac"));
    evidence.set_engine(Identity {
        resolved: binary.to_string_lossy().into_owned(),
        sha256: pinned.clone(),
    });
    evidence
        .write(&boot.root.join(".ratmac"))
        .expect("record the Engine pin");

    let mut bytes = fs::read(&binary).expect("read the pinned Engine");
    bytes.push(0);
    fs::write(&binary, &bytes).expect("alter the pinned Engine");
    let observed = sha256_file(&binary);
    assert_ne!(observed, pinned, "the fixture really altered the binary");

    let before = boot.snapshot();
    let output = boot.bootstrap();
    let report = text(&output);

    assert!(
        !output.status.success(),
        "a pin mismatch refuses instead of reporting success: {report}"
    );
    assert!(
        report.contains(&observed) && report.contains(&pinned),
        "the refusal names observed and expected identity: {report}"
    );
    assert!(
        report.contains("observed") && report.contains("expected"),
        "the refusal says which hash is which: {report}"
    );
    // HT-053-06: the same field names the Engine writes into its pin record.
    let recorded = fs::read_to_string(boot.root.join(".ratmac/evidence.toml")).expect("read pin");
    for field in ["resolved", "sha256"] {
        assert!(
            recorded.contains(&format!("{field} = ")),
            "the Engine records {field} in the pin: {recorded}"
        );
        assert!(
            report.contains(field),
            "the refusal quotes the recorded field {field}: {report}"
        );
    }
    assert!(
        report.contains(".ratmac/evidence.toml"),
        "the refusal names where the pin lives: {report}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Pin: matches") && !stdout.contains("Engine: "),
        "the refusal prints no report that reads as success: {stdout}"
    );
    assert_eq!(boot.snapshot(), before, "the refusal writes nothing");
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
        active.contains("phase: build"),
        "with a Run it reports the phase: {active}"
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
