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
//! SVEV-004 `svev_004`, SVEV-005 `svev_005` (project root), SVEV-006 `svev_006`
//! (t-144 / SVE-002: the Engine is the executable Cargo reported)
//!
//! A fresh session must be able to orient and start without ad-hoc installs:
//! one documented command resolves or builds the Engine offline and refuses on
//! pin mismatch, and the doctor diagnoses the project while writing nothing.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ratmac::pin::{Evidence, Identity};
use ratmac_qa::support::TempTree;
use sha2::{Digest, Sha256};

/// Cargo writes these while building; everything else must stay byte-identical.
const DECLARED_BUILD_OUTPUT: [&str; 2] = ["target", "Cargo.lock"];

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
        Boot { _tree: tree, root }
    }

    /// Put a real executable where the bootstrap resolves it, without building.
    fn prebuild(&self) -> PathBuf {
        let target = self.root.join("target/debug");
        fs::create_dir_all(&target).expect("create build directory");
        let binary = target.join(engine_file_name());
        fs::copy(ratmac_qa::engine_bin!(), &binary).expect("place a prebuilt Engine");
        binary
    }

    /// The bootstrap with the fixture's own `target` named to Cargo and the
    /// other output redirects cleared (SVE-002: the doctor suite's
    /// planted-binary tests set `CARGO_TARGET_DIR` to the fixture's own
    /// `target` and clear `CARGO_BUILD_TARGET` for the bootstrap they start,
    /// so no inherited redirect or Cargo configuration file moves their
    /// output; `CARGO_BUILD_BUILD_DIR` is cleared for the same reason).
    fn bootstrap(&self) -> Output {
        let own = self.root.join("target");
        self.bootstrap_env(&[("CARGO_TARGET_DIR", own.as_os_str())], &REDIRECTS)
    }

    /// `pwsh -NoProfile -File tools/rtm.ps1` from the root, with each variable
    /// in `remove` cleared and then each one in `set` given.
    fn bootstrap_env(&self, set: &[(&str, &OsStr)], remove: &[&str]) -> Output {
        let mut command = Command::new("pwsh");
        command
            .args(["-NoProfile", "-File", "tools/rtm.ps1"])
            .current_dir(&self.root);
        for name in remove {
            command.env_remove(name);
        }
        for (name, value) in set {
            command.env(name, value);
        }
        command.output().expect("invoke the bootstrap")
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
        ..Identity::default()
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

/// Record an Engine pin where the bootstrap reads it when this ticket lands,
/// `.ratmac/evidence.toml`, through the Engine's own pin record.
fn record_pin(root: &Path, engine: &Path, sha256: &str) {
    let mut evidence = Evidence::load(&root.join(".ratmac"));
    evidence.set_engine(Identity {
        resolved: engine.to_string_lossy().into_owned(),
        sha256: sha256.to_owned(),
        ..Identity::default()
    });
    evidence
        .write(&root.join(".ratmac"))
        .expect("record the Engine pin");
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
/// `sha256:` is that file's hash, and with a pin recorded for it the pin reads
/// `Pin: matches`. With the source then broken, the folder inside the fixture
/// is reused with the same lines; with a folder outside, the second run builds
/// first and refuses as a failed build.
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
/// stand-ins stay byte-identical; pinning a stand-in's hash refuses with "the
/// resolved Engine is not the pinned Engine".
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
    record_pin(&boot.root, &stand_ins[0].0, &stand_in_hashes[0]);
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
