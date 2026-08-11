//! t-086 / DEB-001, DEB-002: one command proves the suite.
//!
//! DEBV-001 no two build targets in the repository write one file.
//! DEBV-002 a test launches the build it was compiled against.
//! DEBV-003 the shipped command keeps its name and stays free of pause points.
//! DEBV-004 a planted duplicate fails the audit naming both declarations.

use ratmac_qa::engine_bin;
use ratmac_qa::targets::{self, Kind};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The string only a pause-point build carries.
const PAUSE_POINT: &str = "RATMAC_TEST_HOLD_BARRIER";

/// The blocker record a fixture hold cites.
const BLOCKER: &str = ".arca/issue/i-777-blocker";

/// The repository this suite belongs to.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root must resolve")
}

/// A throwaway directory for a planted tree.
fn sandbox(label: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ratmac-t086-{label}-{stamp}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create sandbox");
    path
}

/// Write a file and every directory above it.
fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, body).expect("write file");
}

#[test]
fn no_two_build_targets_write_one_file() {
    let root = repo_root();
    let report = targets::audit(&root).expect("the repository's manifests must all read and parse");

    assert!(
        report.targets.iter().any(|target| target.kind == Kind::Bin
            && target.name == "rtm"
            && target.package == "ratmac"),
        "DEBV-001: the walk must find the shipped command; it found {:?}",
        report
            .targets
            .iter()
            .map(targets::Target::described)
            .collect::<Vec<_>>()
    );

    assert!(
        report.is_clean(),
        "DEBV-001: no two build targets may write one file:\n{}",
        report.collisions.join("\n")
    );
}

#[test]
fn a_test_launches_the_build_it_was_compiled_against() {
    let root = repo_root();
    let launched = PathBuf::from(engine_bin!());
    assert!(
        launched.is_file(),
        "DEBV-002: the compiled-against Engine must exist at {}",
        launched.display()
    );

    let report = targets::audit(&root).expect("manifests read");
    let shipped = report
        .targets
        .iter()
        .find(|target| target.kind == Kind::Bin && target.package == "ratmac")
        .expect("the shipped command is declared");
    let harness = report
        .targets
        .iter()
        .find(|target| {
            target.kind == Kind::Bin
                && target.package == "ratmac-qa"
                && target.source.ends_with("src/bin/rtm.rs")
        })
        .expect("DEBV-002: the harness declares its own copy of the Engine command");

    assert_ne!(
        shipped.name, harness.name,
        "DEBV-002: the harness copy and the shipped command must not share a target name"
    );
    assert_ne!(
        shipped.output(),
        harness.output(),
        "DEBV-002: the harness copy and the shipped command must not write one file"
    );

    let file_name = launched
        .file_name()
        .expect("launched path has a file name")
        .to_string_lossy()
        .into_owned();
    assert!(
        file_name.starts_with(&harness.name),
        "DEBV-002: the suite must launch the harness target `{}`, not `{file_name}`",
        harness.name
    );

    let bytes = fs::read(&launched).expect("read the launched Engine");
    assert!(
        contains(&bytes, PAUSE_POINT.as_bytes()),
        "DEBV-002: the launched Engine must carry the pause-point wiring"
    );

    // Bytes are not behavior. Drive a hold through the launched command and
    // through the shipped one: the first parks at the pre-State pause point,
    // the second has none to park at. The pair is what makes the answer
    // meaningful - a drive that always claimed success would fail the second.
    let build = Command::new(cargo())
        .args(["build", "--offline", "--bin", "rtm"])
        .current_dir(&root)
        .output()
        .expect("build the shipped command");
    assert!(
        build.status.success(),
        "DEBV-002: the shipped build must succeed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let shipped_binary = root
        .join("target/debug")
        .join(format!("rtm{}", std::env::consts::EXE_SUFFIX));

    assert!(
        HoldFixture::new("debv-002-launched", &launched).reaches_the_snapshot_barrier(),
        "DEBV-002: a hold driven through the launched Engine reaches the pre-State snapshot barrier"
    );
    assert!(
        !HoldFixture::new("debv-002-shipped", &shipped_binary).reaches_the_snapshot_barrier(),
        "DEBV-002: the shipped command carries no pause point, so the same drive cannot park"
    );
}

#[test]
fn the_shipped_command_keeps_its_name_and_stays_pause_free() {
    let root = repo_root();
    let report = targets::audit(&root).expect("manifests read");
    let shipped: Vec<_> = report
        .targets
        .iter()
        .filter(|target| target.kind == Kind::Bin && target.package == "ratmac")
        .collect();
    assert_eq!(
        shipped.len(),
        1,
        "DEBV-003: the root package must declare exactly one command, found {:?}",
        shipped.iter().map(|t| t.described()).collect::<Vec<_>>()
    );
    assert_eq!(
        shipped[0].name, "rtm",
        "DEBV-003: the shipped command is `rtm`"
    );
    assert_eq!(
        shipped[0].source, "src/bin/rtm.rs",
        "DEBV-003: the shipped command is built from src/bin/rtm.rs"
    );

    let build = Command::new(cargo())
        .args(["build", "--offline", "--bin", "rtm"])
        .current_dir(&root)
        .output()
        .expect("build the shipped command");
    assert!(
        build.status.success(),
        "DEBV-003: building the shipped command must succeed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    let binary = root
        .join("target/debug")
        .join(format!("rtm{}", std::env::consts::EXE_SUFFIX));
    let bytes = fs::read(&binary).expect("read the shipped command");
    assert!(
        !contains(&bytes, PAUSE_POINT.as_bytes()),
        "DEBV-003: the shipped command must carry no pause-point wiring"
    );

    let doctor = Command::new(&binary)
        .arg("doctor")
        .current_dir(&root)
        .output()
        .expect("run the shipped doctor");
    let report_text = String::from_utf8_lossy(&doctor.stdout).into_owned();
    let expected = sha256(&bytes);
    assert!(
        report_text.contains(&expected),
        "DEBV-003: the shipped command must report the hash of its own bytes ({expected}); it reported:\n{report_text}"
    );
    let paths = report_text
        .lines()
        .filter(|line| line.contains("Engine:"))
        .count();
    assert_eq!(
        paths, 1,
        "DEBV-003: the report names exactly one resolved Engine path:\n{report_text}"
    );
}

#[test]
fn a_planted_duplicate_target_fails_by_name() {
    let base = sandbox("planted");
    let root = base.join("project");
    write(
        &root.join("Cargo.toml"),
        "[package]\nname = \"host\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\nmembers = [\"member\"]\n\n[[bin]]\nname = \"tool\"\npath = \"src/bin/tool.rs\"\n",
    );
    write(&root.join("src/bin/tool.rs"), "fn main() {}\n");
    write(
        &root.join("member/Cargo.toml"),
        "[package]\nname = \"member\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(&root.join("member/src/lib.rs"), "");

    let clean = targets::audit(&root).expect("planted tree reads");
    assert!(
        clean.is_clean(),
        "DEBV-004: a tree with unique target names must pass:\n{}",
        clean.collisions.join("\n")
    );

    write(
        &root.join("member/Cargo.toml"),
        "[package]\nname = \"member\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"tool\"\npath = \"../src/bin/tool.rs\"\n",
    );
    let planted = targets::audit(&root).expect("planted tree reads");
    assert_eq!(
        planted.collisions.len(),
        1,
        "DEBV-004: the duplicate must be reported exactly once:\n{}",
        planted.collisions.join("\n")
    );
    let failure = &planted.collisions[0];
    for fragment in [
        "Cargo.toml",
        "member/Cargo.toml",
        "`tool`",
        "package `host`",
        "package `member`",
    ] {
        assert!(
            failure.contains(fragment),
            "DEBV-004: the failure must name both declarations, missing {fragment}: {failure}"
        );
    }

    write(
        &root.join("member/Cargo.toml"),
        "[package]\nname = \"member\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    let repaired = targets::audit(&root).expect("planted tree reads");
    assert!(
        repaired.is_clean(),
        "DEBV-004: removing the duplicate must pass again:\n{}",
        repaired.collisions.join("\n")
    );
    fs::remove_dir_all(&base).ok();
}

/// The cargo that built this test.
fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

/// Whether `haystack` contains `needle`.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// Lowercase hexadecimal SHA-256 of `bytes`.
fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// A started Run parked in `build`, driven by whichever command the lane
/// hands it, so the two builds can be compared by behavior.
struct HoldFixture {
    root: PathBuf,
    run_id: String,
    /// The command every step of this fixture launches.
    engine: PathBuf,
}

impl Drop for HoldFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl HoldFixture {
    fn new(label: &str, engine: &Path) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t086-{label}-{}-{}",
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

        let mut fixture = HoldFixture {
            root,
            run_id: String::new(),
            engine: engine.to_path_buf(),
        };
        assert!(fixture.rtm(&["start"]), "the fixture Run starts");
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
            fixture.rtm(&["step", "--run", &id]),
            "the Run reaches the ticket-building State"
        );
        fixture
    }

    fn rtm(&self, args: &[&str]) -> bool {
        Command::new(&self.engine)
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("the launched Engine runs")
            .status
            .success()
    }

    /// Start a hold with the pause point armed and report whether it parked
    /// there before writing any State.
    fn reaches_the_snapshot_barrier(&self) -> bool {
        let barrier = self.root.join(".ratmac/test-hold-snapshot");
        let marker = barrier.join("marker");
        let release = barrier.join("release");
        let mut child = Command::new(&self.engine)
            .args([
                "hold",
                "t-900",
                "--blocker",
                BLOCKER,
                "--confirm",
                "hold t-900",
                "--run",
                &self.run_id,
            ])
            .current_dir(&self.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("RATMAC_TEST_HOLD_BARRIER", "before-state-write")
            .env("RATMAC_TEST_HOLD_BARRIER_MARKER", &marker)
            .env("RATMAC_TEST_HOLD_BARRIER_RELEASE", &release)
            .env("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS", "10000")
            .spawn()
            .expect("start the hold");

        // Stop as soon as the answer is known: the pause point appeared, or
        // the command ran to completion without one.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut reached = false;
        while std::time::Instant::now() < deadline {
            if marker.is_file() {
                reached = true;
                break;
            }
            if child.try_wait().expect("poll the hold").is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        let _ = fs::write(&release, "release\n");
        let _ = child.wait();
        reached
    }
}
