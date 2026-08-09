//! t-079 / ENS-010: one spelling for every reported Engine root.
//!
//! ENSV-011 `reported_engine_root_is_spelled_one_way`
//!
//! `t-078` proved the reports name the root the invocation resolved, but its
//! oracle normalized separators on both sides before comparing, so it could
//! not see how the path was spelled.  This test reads the rendered characters:
//! the Git route (which joins a Git-printed forward-slash checkout to the
//! Engine directory) and the no-Git fallback route (which renders whatever the
//! platform hands back) must produce one spelling, and the human and JSON
//! reports of one invocation must agree character for character.
//!
//! Every expectation is fixture-authored: the tail each report must end in is
//! the directory this test created, never a value obtained from the resolver
//! under test.

use ratmac_qa::json::Json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MACHINE_CLASS: &str = r#"
[phases.plan]
prompt = "Plan."

[phases.done]
prompt = "Done."

[[transitions]]
from = "plan"
to = "done"
"#;

struct Sandbox {
    root: PathBuf,
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn fresh_sandbox(label: &str) -> Sandbox {
    let root = std::env::temp_dir().join(format!(
        "ratmac-t079-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock is after the epoch")
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create sandbox");
    Sandbox { root }
}

fn write_machine_class(root: &Path) {
    fs::create_dir_all(root.join(".ratmac")).expect("create Engine directory");
    fs::write(root.join(".ratmac/ratmac.toml"), MACHINE_CLASS).expect("write Machine Class");
}

fn git_success(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("Git is executable for the fixture");
    assert!(
        output.status.success(),
        "fixture git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn rtm_at(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rtm"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("invoke built rtm binary")
}

/// Mint one Run and name it from the fixture's own Engine directory.
fn start_run(invocation_root: &Path, fixture_engine_dir: &Path) -> String {
    let started = rtm_at(invocation_root, &["start"]);
    assert!(
        started.status.success(),
        "fixture `rtm start` must mint a Run: {}{}",
        String::from_utf8_lossy(&started.stdout),
        String::from_utf8_lossy(&started.stderr)
    );
    let mut ids = fs::read_dir(fixture_engine_dir.join("runs"))
        .expect("fixture roster is listable")
        .map(|entry| entry.expect("fixture roster entry is readable"))
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

/// The `Engine root:` facts a report rendered, exactly as written.
fn human_roots(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("Engine root: "))
        .map(str::to_owned)
        .collect()
}

fn json_root(output: &Output) -> String {
    let report = String::from_utf8_lossy(&output.stdout);
    let json = Json::parse(&report).unwrap_or_else(|error| {
        panic!("ENS-010: `rtm doctor --json` must emit parseable JSON: {error:?}\n{report}")
    });
    json.field("engine_root")
        .unwrap_or_else(|| {
            panic!("ENS-010: the JSON report must carry an engine_root member\n{report}")
        })
        .to_owned()
}

/// Every rendered Engine root of one invocation route: one spelling, no
/// platform separator, and the fixture's own Engine directory at its end.
fn assert_one_spelling(route: &str, invocation_root: &Path, fixture_engine_dir: &Path) {
    let run = start_run(invocation_root, fixture_engine_dir);
    let expected_tail = format!(
        "/{}/.ratmac",
        fixture_engine_dir
            .parent()
            .expect("fixture Engine directory has a parent checkout")
            .file_name()
            .expect("fixture checkout has a name")
            .to_string_lossy()
    );

    let status = rtm_at(invocation_root, &["status", "--run", run.as_str()]);
    let doctor = rtm_at(invocation_root, &["doctor"]);
    let doctor_json = rtm_at(invocation_root, &["doctor", "--json"]);

    let mut rendered = human_roots(&status);
    assert_eq!(
        rendered.len(),
        1,
        "ENS-010: `rtm status` in the {route} route must render exactly one Engine root fact: {}{}",
        String::from_utf8_lossy(&status.stdout),
        String::from_utf8_lossy(&status.stderr)
    );
    let doctor_roots = human_roots(&doctor);
    assert_eq!(
        doctor_roots.len(),
        1,
        "ENS-010: `rtm doctor` in the {route} route must render exactly one Engine root fact: {}{}",
        String::from_utf8_lossy(&doctor.stdout),
        String::from_utf8_lossy(&doctor.stderr)
    );
    rendered.extend(doctor_roots);
    rendered.push(json_root(&doctor_json));

    for root in &rendered {
        assert!(
            !root.contains('\\'),
            "ENS-010: the {route} route rendered the Engine root as {root:?}, mixing a platform \
             separator into a reported path; a reported path leaves the Engine spelled with \
             forward slashes only"
        );
        assert!(
            root.ends_with(&expected_tail),
            "ENS-010: the {route} route rendered the Engine root as {root:?}, which does not end \
             in the fixture's own {expected_tail:?}"
        );
    }

    let first = rendered[0].clone();
    for root in &rendered {
        assert_eq!(
            root, &first,
            "ENS-010: the {route} route must report one identical Engine-root spelling from \
             `rtm status`, `rtm doctor`, and `rtm doctor --json`; got {rendered:?}"
        );
    }
}

/// ENSV-011: the resolver's canonical path reaches every report in one
/// spelling, on the Git route and on the no-Git fallback alike.
#[test]
fn reported_engine_root_is_spelled_one_way() {
    let git = fresh_sandbox("git");
    let primary = git.root.join("primary");
    fs::create_dir_all(&primary).expect("create primary checkout");
    write_machine_class(&primary);
    git_success(&primary, &["init"]);
    git_success(&primary, &["config", "core.autocrlf", "false"]);
    git_success(&primary, &["config", "user.email", "qa@example.invalid"]);
    git_success(&primary, &["config", "user.name", "Ratmac QA"]);
    git_success(&primary, &["add", "--", ".ratmac/ratmac.toml"]);
    git_success(&primary, &["commit", "-m", "fixture base"]);

    let linked = git.root.join("linked");
    let added = Command::new("git")
        .args(["worktree", "add", "-b", "t079-linked"])
        .arg(&linked)
        .current_dir(&primary)
        .output()
        .expect("run git worktree add");
    assert!(
        added.status.success(),
        "fixture linked worktree failed: {}",
        String::from_utf8_lossy(&added.stderr)
    );
    write_machine_class(&linked);

    let no_git = fresh_sandbox("no-git");
    let plain = no_git.root.join("plain");
    fs::create_dir_all(&plain).expect("create no-Git checkout");
    write_machine_class(&plain);

    // The primary mints the shared Run; the linked worktree must report that
    // same shared `.ratmac/`, so it addresses the roster the primary created.
    assert_one_spelling("primary Git checkout", &primary, &primary.join(".ratmac"));
    assert_one_spelling("no-Git checkout", &plain, &plain.join(".ratmac"));

    let linked_doctor = rtm_at(&linked, &["doctor"]);
    let linked_roots = human_roots(&linked_doctor);
    assert_eq!(
        linked_roots.len(),
        1,
        "ENS-010: `rtm doctor` in a linked worktree must render exactly one Engine root fact: {}{}",
        String::from_utf8_lossy(&linked_doctor.stdout),
        String::from_utf8_lossy(&linked_doctor.stderr)
    );
    assert!(
        !linked_roots[0].contains('\\'),
        "ENS-010: the linked worktree rendered the Engine root as {:?}, mixing a platform \
         separator into a reported path",
        linked_roots[0]
    );
    assert!(
        linked_roots[0].ends_with("/primary/.ratmac"),
        "ENS-010: the linked worktree must report the shared primary Engine root; got {:?}",
        linked_roots[0]
    );
}

/// A Machine Class whose declared root overlaps the Engine root, so static
/// validation renders both paths into an `rtm doctor` finding (RB604).
const OVERLAPPING_ROOTS: &str = r#"
[roots]
engine = ".ratmac"

[phases.plan]
prompt = "Plan."
guards = [{ kind = "command_exit", program = "no/such-program", args = [], expected = 0 }]

[phases.done]
prompt = "Done."

[[transitions]]
from = "plan"
to = "done"
"#;

/// ENSV-011: one `rtm doctor` report renders every path in one spelling - the
/// Engine binary line and the diagnostic findings included, not only the
/// `Engine root:` fact.
#[test]
fn the_whole_doctor_report_renders_paths_one_way() {
    let sandbox = fresh_sandbox("report");
    let checkout = sandbox.root.join("primary");
    fs::create_dir_all(checkout.join(".ratmac")).expect("create Engine directory");
    fs::write(checkout.join(".ratmac/ratmac.toml"), OVERLAPPING_ROOTS)
        .expect("write overlapping Machine Class");
    git_success(&checkout, &["init"]);
    git_success(&checkout, &["config", "core.autocrlf", "false"]);
    git_success(&checkout, &["config", "user.email", "qa@example.invalid"]);
    git_success(&checkout, &["config", "user.name", "Ratmac QA"]);
    git_success(&checkout, &["add", "--", ".ratmac/ratmac.toml"]);
    git_success(&checkout, &["commit", "-m", "fixture base"]);

    let doctor = rtm_at(&checkout, &["doctor"]);
    let report = String::from_utf8_lossy(&doctor.stdout).into_owned();
    assert!(
        report.contains("RB301"),
        "fixture setup must make an unresolvable guard program render a pinning finding, whose \
         text carries a path built by the Engine; report was:\n{report}"
    );
    assert!(
        report.contains("RB604"),
        "fixture setup must make static validation render a root-overlap finding; report was:\n{report}"
    );

    let engine_line = report
        .lines()
        .find_map(|line| line.strip_prefix("Engine: "))
        .unwrap_or_else(|| panic!("ENS-010: the doctor report names its Engine binary\n{report}"));
    let engine_path = engine_line
        .split(" (sha256: ")
        .next()
        .expect("the Engine line carries its path before the hash");
    assert!(
        !engine_path.contains('\\'),
        "ENS-010: the doctor report rendered its Engine binary as {engine_path:?}, mixing a \
         platform separator into a reported path while the same report spells the Engine root \
         with forward slashes"
    );

    for line in report.lines() {
        assert!(
            !line.contains('\\'),
            "ENS-010: one doctor report renders every path in one spelling, but this line carries \
             a platform separator: {line:?}\nfull report:\n{report}"
        );
    }

    let json = rtm_at(&checkout, &["doctor", "--json"]);
    let rendered = String::from_utf8_lossy(&json.stdout).into_owned();
    let parsed = Json::parse(&rendered).unwrap_or_else(|error| {
        panic!("ENS-010: the JSON report must stay parseable: {error:?}\n{rendered}")
    });
    let root = parsed
        .field("engine_root")
        .expect("the JSON report carries its Engine root");
    assert!(
        !root.contains('\\'),
        "ENS-010: the JSON report rendered the Engine root as {root:?}"
    );
    assert!(
        !rendered.contains("\\\\"),
        "ENS-010: the JSON report escaped a platform separator, so a machine reader still sees a \
         second spelling:\n{rendered}"
    );
}
