//! WCP-002: preserve lane proof while isolating the support each lane uses.

use ratmac_qa::baseline::{self, scenario};
use ratmac_qa::json::Json;
use ratmac_qa::lane_runs;
use ratmac_qa::support::{self, TempTree};
use ratmac_qa::tempgit::TempRepo;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

const CHILD_MODE: &str = "RATMAC_T126_PUBLIC_CHILD";

/// An owned fixture folder: its path, and the shared helper's tree that
/// removes it when the fixture drops.
struct Scratch(PathBuf, #[allow(dead_code)] TempTree);

impl Scratch {
    fn new(label: &str) -> Self {
        let tree = TempTree::new(&format!("t126-{label}")).expect("create owned fixture directory");
        Self(tree.path().to_path_buf(), tree)
    }
}

fn clean_git(root: &Path, args: &[&str]) -> Output {
    let mut command = Command::new("git");
    command.args(args).current_dir(root);
    clear_git_environment(&mut command);
    let output = command.output().expect("run fixture Git");
    assert!(
        output.status.success(),
        "fixture Git {args:?}: {}",
        text(&output)
    );
    output
}

fn clear_git_environment(command: &mut Command) {
    for (key, _) in std::env::vars_os() {
        if key
            .to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("GIT_")
        {
            command.env_remove(key);
        }
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, rows: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).expect("read owned fixture directory") {
            let entry = entry.expect("read owned fixture entry");
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            let kind = entry.file_type().expect("inspect owned fixture entry");
            if kind.is_dir() {
                rows.insert(relative, b"D\0".to_vec());
                walk(root, &path, rows);
            } else if kind.is_symlink() {
                let mut bytes = b"L\0".to_vec();
                bytes.extend(fs::read_link(&path).unwrap().as_os_str().as_encoded_bytes());
                rows.insert(relative, bytes);
            } else {
                assert!(
                    kind.is_file(),
                    "unexpected fixture node: {}",
                    path.display()
                );
                let mut bytes = b"F\0".to_vec();
                bytes.extend(fs::read(path).expect("read owned fixture bytes"));
                rows.insert(relative, bytes);
            }
        }
    }
    let mut rows = BTreeMap::new();
    walk(root, root, &mut rows);
    rows
}

fn victim(label: &str) -> Scratch {
    let fixture = Scratch::new(label);
    clean_git(&fixture.0, &["init", "--quiet"]);
    clean_git(
        &fixture.0,
        &["symbolic-ref", "HEAD", "refs/heads/victim-original"],
    );
    clean_git(&fixture.0, &["config", "core.autocrlf", "false"]);
    clean_git(
        &fixture.0,
        &["config", "user.name", "untouched fixture owner"],
    );
    clean_git(
        &fixture.0,
        &["config", "user.email", "untouched@fixture.invalid"],
    );
    fs::write(fixture.0.join("victim.txt"), "must remain unchanged\n").unwrap();
    clean_git(&fixture.0, &["add", "--", "victim.txt"]);
    fixture
}

fn probe(cwd: &Path, poison: Option<&Path>) -> Child {
    let mut command = Command::new(std::env::current_exe().expect("public test executable"));
    command
        .args(["--exact", "wcpv_005", "--nocapture"])
        .current_dir(cwd)
        .env(CHILD_MODE, "git-isolation")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    clear_git_environment(&mut command);
    if let Some(victim) = poison {
        command
            .env("GIT_DIR", victim.join(".git"))
            .env("GIT_WORK_TREE", victim)
            .env("GIT_INDEX_FILE", victim.join(".git/index"));
    }
    command.spawn().expect("launch isolated public-test probe")
}

fn git_isolation_child() {
    let repo = TempRepo::new("t126-existing-helper");
    assert!(repo.root().join(".git").is_dir(),
        "TempRepo must initialize its own .git; inherited Git overrides must not redirect initialization");
    let output = repo.git(&["rev-parse", "--show-toplevel"]);
    assert!(
        output.status.success(),
        "the helper's Git resolves: {}",
        text(&output)
    );
    let resolved = PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
    assert_eq!(
        fs::canonicalize(resolved).unwrap(),
        fs::canonicalize(repo.root()).unwrap(),
        "every helper Git operation must address the fixture it belongs to"
    );
    repo.write("inside.txt", "this fixture only\n");
    repo.stage("inside.txt");
    let indexed = repo.git(&["ls-files", "--cached", "-z"]);
    assert!(indexed.status.success());
    assert_eq!(
        indexed.stdout, b"inside.txt\0",
        "the fixture owns its index, not the caller's inherited index"
    );
}

fn fault_probe(cwd: &Path, inherited_fault: bool) -> Child {
    let mut command = Command::new(std::env::current_exe().expect("public test executable"));
    command
        .args(["--exact", "wcpv_005", "--nocapture"])
        .current_dir(cwd)
        .env(CHILD_MODE, "fault-isolation")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command_environment(&mut command);
    if inherited_fault {
        command.env("RATMAC_TEST_STEP_FAULT", "before-verdict-archive");
    }
    command
        .spawn()
        .expect("launch isolated fault-environment probe")
}

fn fault_isolation_child() {
    // Fixture preparation explicitly clears injected faults. The scenario
    // below deliberately has no env override: inherited fault state must not
    // become an undeclared input to the maintained process helper.
    let (fixture, id) = started_branch("undeclared-fault");
    let normal = baseline::run(
        Path::new(ratmac_qa::engine_bin!()),
        &fixture.0,
        &scenario(
            "normal scenario ignores ambient fault",
            &["step", "--run", &id],
        ),
        false,
    );
    assert_eq!(
        normal.code,
        Some(0),
        "an undeclared inherited fault must not interrupt a normal scenario: {}",
        normal.text
    );
    assert_eq!(
        branch_state(&fixture.0, &id),
        "accepted",
        "the valid branching verdict was consumed normally"
    );
}

const BRANCH: &str = r#"
[states.review]
prompt = "Review the fixture."
inputs = ["accept", "revise"]
[states.accepted]
prompt = "Accepted."
[states.revised]
prompt = "Revised."
[[transitions]]
from = "review"
to = "accepted"
input = "accept"
[[transitions]]
from = "review"
to = "revised"
input = "revise"
"#;

fn command_environment(command: &mut Command) {
    clear_git_environment(command);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("RATMAC_TEST_") {
            command.env_remove(key);
        }
    }
}

fn started_branch(label: &str) -> (Scratch, String) {
    let fixture = Scratch::new(label);
    fs::create_dir(fixture.0.join(".ratmac")).unwrap();
    fs::write(fixture.0.join(".ratmac/ratmac.toml"), BRANCH).unwrap();
    let mut command = Command::new(ratmac_qa::engine_bin!());
    command.arg("start").current_dir(&fixture.0);
    command_environment(&mut command);
    let output = command.output().expect("start an isolated branch fixture");
    assert!(output.status.success(), "start branch: {}", text(&output));
    let id = String::from_utf8(output.stdout)
        .unwrap()
        .split("started run ")
        .nth(1)
        .and_then(|tail| tail.split_whitespace().next())
        .unwrap()
        .to_owned();
    let verdict = fixture
        .0
        .join(".ratmac/runs")
        .join(&id)
        .join("verdict.toml");
    fs::write(
        verdict,
        "state = \"review\"\ninput = \"accept\"\nrationale = \"fixture approval\"\n",
    )
    .unwrap();
    (fixture, id)
}

fn branch_state(root: &Path, id: &str) -> String {
    let source = fs::read_to_string(root.join(".ratmac/runs").join(id).join("run.toml")).unwrap();
    source.parse::<toml::Value>().unwrap()["state"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// Existing fault requests remain effective and leave the same recoverable
/// state; an ordinary retry must then consume the original verdict and pass.
fn preserved_fault(boundary: &str) {
    let (fixture, id) = started_branch(boundary);
    let run = fixture.0.join(".ratmac/runs").join(&id);
    let original_state = fs::read(run.join("run.toml")).unwrap();
    let original_verdict = fs::read(run.join("verdict.toml")).unwrap();
    let before = snapshot(&fixture.0);
    let faulted = scenario("explicit fixture fault", &["step", "--run", &id])
        .with_env("RATMAC_TEST_STEP_FAULT", boundary);
    let output = baseline::run(
        Path::new(ratmac_qa::engine_bin!()),
        &fixture.0,
        &faulted,
        false,
    );
    assert_ne!(
        output.code,
        Some(0),
        "the explicit fault must refuse: {}",
        output.text
    );
    assert!(
        output.text.contains(boundary),
        "refusal names the intended boundary: {}",
        output.text
    );
    assert_eq!(
        fs::read(run.join("run.toml")).unwrap(),
        original_state,
        "a fault before State replacement preserves the whole Run Record"
    );
    if boundary == "before-verdict-archive" {
        assert_eq!(
            snapshot(&fixture.0),
            before,
            "a pre-archive fault changes no fixture bytes"
        );
    } else {
        // This existing boundary intentionally preserves the consumed archive,
        // not an invented rollback of evidence already archived by the Engine.
        assert!(!run.join("verdict.toml").exists());
        assert_eq!(
            fs::read(run.join("verdicts/000001.toml")).unwrap(),
            original_verdict,
            "the already consumed verdict remains exact immutable evidence"
        );
        let consumed = snapshot(&fixture.0);
        let no_replay = baseline::run(
            Path::new(ratmac_qa::engine_bin!()),
            &fixture.0,
            &scenario("consumed input refuses replay", &["step", "--run", &id]),
            false,
        );
        assert!(no_replay.text.contains("step refused"));
        assert_eq!(snapshot(&fixture.0), consumed);
        fs::write(
            run.join("verdict.toml"),
            "state = \"review\"\ninput = \"accept\"\nrationale = \"fresh recovery evidence\"\n",
        )
        .unwrap();
    }
    assert_eq!(branch_state(&fixture.0, &id), "review");
    let retry = baseline::run(
        Path::new(ratmac_qa::engine_bin!()),
        &fixture.0,
        &scenario("ordinary retry", &["step", "--run", &id]),
        false,
    );
    assert_eq!(retry.code, Some(0), "ordinary retry passes: {}", retry.text);
    assert_eq!(branch_state(&fixture.0, &id), "accepted");
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn cargo_in(root: &Path, args: &[&str]) -> Output {
    let mut command = Command::new("cargo");
    command.args(args).current_dir(root).stdin(Stdio::null());
    for (key, _) in std::env::vars_os() {
        if key
            .to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("CARGO_")
        {
            command.env_remove(key);
        }
    }
    command_environment(&mut command);
    command.output().expect("run an owned fixture crate")
}

fn aggregate_fixture_control() {
    let fixture = Scratch::new("aggregate");
    fs::create_dir_all(fixture.0.join(".ratmac")).unwrap();
    let mut expected_names = Vec::new();
    for id in ["901", "902"] {
        let root = fixture.0.join(format!("lanes/t-{id}"));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("Cargo.toml"), format!(
            "[package]\nname = \"t126-fixture-{id}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[workspace]\n"
        )).unwrap();
        let name = format!("ht_{id}_fixture");
        fs::write(root.join("src/lib.rs"), format!(
            "#[test]\nfn {name}() {{ assert_eq!(std::fs::read_to_string(\"owned.txt\").unwrap(), \"{id}\"); }}\n"
        )).unwrap();
        fs::write(root.join("owned.txt"), id).unwrap();
        let direct = cargo_in(&root, &["test", "--offline"]);
        assert!(
            direct.status.success(),
            "direct fixture lane passes: {}",
            text(&direct)
        );
        assert!(
            text(&direct).contains(&format!("test {name} ... ok")),
            "the direct run executes its named lane"
        );
        expected_names.push(name);
    }
    let config = fixture.0.join(".ratmac/lanes.toml");
    fs::write(
        &config,
        "[roots]\nlanes = \"lanes\"\nreport = \"report.md\"\nroster = [\"t-901\", \"t-902\"]\n",
    )
    .unwrap();
    let run = |verb: &str| {
        let mut command = Command::new("python");
        command
            .arg(repo_root().join("tools/sweep_lanes.py"))
            .arg(verb)
            .arg("--config")
            .arg(&config)
            .current_dir(&fixture.0);
        command_environment(&mut command);
        command
            .output()
            .expect("run shipped aggregate tool on owned lanes")
    };
    let swept = run("sweep");
    assert!(
        swept.status.success(),
        "the same fixture lanes pass in aggregate: {}",
        text(&swept)
    );
    let report = fs::read_to_string(fixture.0.join("report.md")).unwrap();
    let rows = report
        .lines()
        .filter(|line| line.starts_with("| t-"))
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        [
            "| t-901 | pass | 1 lane(s) green |",
            "| t-902 | pass | 1 lane(s) green |"
        ]
    );
    assert_eq!(
        expected_names.len(),
        rows.len(),
        "every direct lane is represented in the aggregate"
    );
    assert!(report.contains("2 pass, 0 expired, 0 red, 0 missing - 2 crates"));
    let checked = run("check");
    assert!(
        checked.status.success(),
        "the aggregate report is complete: {}",
        text(&checked)
    );
}

/// Aggregate reuse: a lane observation is reused only while the checkout is
/// byte-identical to what its run saw from start to finish. Build output is
/// not input; a new build script, in-checkout Cargo configuration, or a
/// regular file named `target` is.
fn observation_reuse_control() {
    let fixture = Scratch::new("observation-reuse");
    let root = fixture.0.join("checkout");
    fs::create_dir_all(root.join("test-hidden/t-901/src")).unwrap();
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"engine\"\n").unwrap();
    fs::write(root.join("test-hidden/t-901/src/lib.rs"), "// lane\n").unwrap();
    let runs = AtomicU64::new(0);
    let observations = lane_runs::Observations::new();
    let observe = || observations.observe(&root, "t-901", || runs.fetch_add(1, Ordering::SeqCst));
    let count = || runs.load(Ordering::SeqCst);
    let first = observe();
    assert!(
        Arc::ptr_eq(&first, &observe()),
        "an unchanged checkout reuses the executed observation"
    );
    assert_eq!(count(), 1, "reuse executes nothing");
    fs::create_dir_all(root.join("test-hidden/t-901/target/debug")).unwrap();
    fs::write(root.join("test-hidden/t-901/target/debug/out"), "built").unwrap();
    observe();
    assert_eq!(count(), 1, "build output under target/ is not a lane input");
    for (step, (file, body)) in [
        (root.join("build.rs"), "fn main() {}\n"),
        (root.join(".cargo/config.toml"), "[build]\n"),
        // Outside the checkout, in an owned ancestor: Cargo discovers it too.
        (
            fixture.0.join(".cargo/config.toml"),
            "[net]\noffline = true\n",
        ),
        (
            root.join("test-hidden/t-901/target-file/target"),
            "a file, not build output\n",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, body).unwrap();
        observe();
        assert_eq!(
            count(),
            step as u64 + 2,
            "adding {} forces a fresh run",
            file.display()
        );
        observe();
        assert_eq!(count(), step as u64 + 2, "the fresh run is then reused");
    }
    // Change an input so the next observation must run, then move the inputs
    // while it runs: that run is returned but never kept for reuse. Putting
    // the pre-run bytes back afterwards must not revive it: the inputs then
    // hash exactly as they did when that run started.
    fs::write(root.join("test-hidden/t-901/src/lib.rs"), "// lane v2\n").unwrap();
    observations.observe(&root, "t-901", || {
        fs::write(
            root.join("test-hidden/t-901/src/lib.rs"),
            "// edited mid-run\n",
        )
        .unwrap();
        runs.fetch_add(1, Ordering::SeqCst)
    });
    assert_eq!(count(), 6, "the changed input forced a fresh run");
    fs::write(root.join("test-hidden/t-901/src/lib.rs"), "// lane v2\n").unwrap();
    observe();
    assert_eq!(
        count(),
        7,
        "an observation whose inputs moved while it ran is never reused, \
         even once the inputs return to their pre-run bytes"
    );
}

const INVENTORY: &str = ".ratmac/evidence/run-037/baseline-inventory.toml";
const AFTER_INVENTORY: &str = ".ratmac/evidence/run-037/migration-inventory.toml";

fn inventory(path: &str, stage: &str) -> toml::Value {
    let value: toml::Value = fs::read_to_string(repo_root().join(path))
        .expect("the coordinator's frozen migration inventory exists")
        .parse()
        .expect("the frozen migration inventory is TOML");
    assert_eq!(value["version"].as_integer(), Some(1));
    assert_eq!(
        value["stage"].as_str(),
        Some(stage),
        "inventory role is explicit"
    );
    value
}

fn baseline_mode() -> bool {
    match std::env::var("RATMAC_MIGRATION_BASELINE").ok().as_deref() {
        None | Some("") | Some("0") => false,
        Some("1") => {
            assert!(
                !repo_root().join(AFTER_INVENTORY).exists(),
                "baseline mode is unavailable after real migration evidence exists"
            );
            eprintln!("preservation baseline control only; this is not completion proof");
            true
        }
        Some(other) => panic!("invalid preservation baseline mode {other:?}"),
    }
}

fn engine_hooks(record: &toml::Value) -> bool {
    strings(record, "features")
        .iter()
        .find(|entry| entry.starts_with("ratmac@"))
        .expect("frozen Engine feature record")
        .split_once('|')
        .unwrap()
        .1
        .split(',')
        .any(|feature| feature == "test-fault-injection")
}

fn matching_crate<'a>(value: &'a toml::Value, id: &str) -> &'a toml::Value {
    records(value, "crates")
        .iter()
        .find(|record| record["id"].as_str() == Some(id))
        .unwrap_or_else(|| panic!("migration inventory omitted {id}"))
}

fn records<'a>(value: &'a toml::Value, key: &str) -> &'a [toml::Value] {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("inventory needs {key} rows"))
}

fn strings(value: &toml::Value, key: &str) -> Vec<String> {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("inventory needs {key}"))
        .iter()
        .map(|value| value.as_str().expect("inventory string entry").to_owned())
        .collect()
}

fn shown_relative(root: &Path, value: &str) -> String {
    let path = Path::new(value);
    let resolved = fs::canonicalize(path).unwrap_or_else(|_| {
        fs::canonicalize(path.parent().expect("metadata path has a parent"))
            .unwrap()
            .join(
                path.file_name()
                    .expect("metadata path has a final component"),
            )
    });
    resolved
        .strip_prefix(root)
        .expect("metadata path remains in this checkout")
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn metadata_boundaries(root: &Path, id: &str, before: &toml::Value, target: &str) {
    let crate_root = root.join("test-hidden").join(id);
    let output = cargo_in(
        &crate_root,
        &[
            "metadata",
            "--offline",
            "--locked",
            "--format-version",
            "1",
            "--filter-platform",
            target,
        ],
    );
    assert!(
        output.status.success(),
        "{id}: Cargo metadata reads: {}",
        text(&output)
    );
    let metadata =
        Json::parse(&String::from_utf8(output.stdout).unwrap()).expect("Cargo metadata is JSON");
    assert_eq!(
        shown_relative(root, metadata.field("workspace_root").unwrap()),
        before["workspace_root"].as_str().unwrap(),
        "{id}: the private crate keeps its workspace boundary"
    );
    assert_eq!(
        shown_relative(root, metadata.field("target_directory").unwrap()),
        before["target_directory"].as_str().unwrap(),
        "{id}: the private crate keeps its output directory"
    );
    let packages = metadata.as_object().unwrap()["packages"]
        .as_array()
        .unwrap();
    let expected_manifest = fs::canonicalize(crate_root.join("Cargo.toml")).unwrap();
    let package = packages
        .iter()
        .find(|package| {
            fs::canonicalize(package.field("manifest_path").unwrap())
                .ok()
                .as_ref()
                == Some(&expected_manifest)
        })
        .expect("metadata includes the addressed private package");
    let mut targets = package.as_object().unwrap()["targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|target| {
            let mut kinds = target.as_object().unwrap()["kind"]
                .as_array()
                .unwrap()
                .iter()
                .map(|kind| kind.as_str().unwrap())
                .collect::<Vec<_>>();
            kinds.sort();
            format!(
                "{}|{}|{}",
                kinds.join(","),
                target.field("name").unwrap(),
                shown_relative(root, target.field("src_path").unwrap())
            )
        })
        .collect::<Vec<_>>();
    targets.sort();
    let mut expected_targets = strings(before, "targets");
    expected_targets.sort();
    assert_eq!(
        targets, expected_targets,
        "{id}: package targets retain their identities and sources"
    );

    let engine = packages.iter().find(|package| {
        package.field("name") == Some("ratmac")
            && Path::new(package.field("manifest_path").unwrap()) == root.join("Cargo.toml")
    });
    let engine = engine
        .or_else(|| {
            packages
                .iter()
                .find(|package| package.field("name") == Some("ratmac"))
        })
        .expect("private proof resolves the Engine");
    let resolve = metadata.as_object().unwrap()["resolve"]
        .as_object()
        .unwrap();
    let engine_node = resolve["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node.field("id") == engine.field("id"))
        .expect("resolved Engine feature node");
    let enabled = engine_node.as_object().unwrap()["features"]
        .as_array()
        .unwrap()
        .iter()
        .any(|feature| feature.as_str() == Some("test-fault-injection"));
    let frozen_features = strings(before, "features");
    let frozen_engine = frozen_features
        .iter()
        .find(|entry| entry.starts_with("ratmac@"))
        .expect("frozen Engine feature record");
    let expected_enabled = frozen_engine
        .split_once('|')
        .unwrap()
        .1
        .split(',')
        .any(|feature| feature == "test-fault-injection");
    assert_eq!(
        enabled, expected_enabled,
        "{id}: sharing support cannot silently change Engine test hooks"
    );
}

/// Preservation: retain every landed lane identity and its requirement mapping,
/// with the direct/aggregate command control on isolated owned fixtures. The
/// coordinator's mandatory live sweep supplies whole-roster execution proof;
/// this check does not launch another full sweep or claim the fixture is one.
#[test]
fn wcpv_004() {
    let root = repo_root();
    let before = inventory(INVENTORY, "before");
    let baseline = baseline_mode();
    let after = (!baseline).then(|| inventory(AFTER_INVENTORY, "after"));
    let declaration: toml::Value = fs::read_to_string(root.join(".ratmac/lanes.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let roster = declaration["roots"]["roster"].as_array().unwrap();
    let crates = records(&before, "crates");
    assert_eq!(
        crates.len(),
        58,
        "the frozen starting roster has 58 landed crates"
    );
    for record in crates {
        let id = record["id"].as_str().unwrap();
        assert!(
            roster.iter().any(|entry| entry.as_str() == Some(id)),
            "{id}: no landed crate disappears"
        );
        let prefix = PathBuf::from("test-hidden").join(id);
        let mut names = Vec::new();
        let mut mapping = String::new();
        for file in records(&before, "files") {
            let path = Path::new(file["path"].as_str().unwrap());
            if !path.starts_with(&prefix) {
                continue;
            }
            let bytes = match fs::read(root.join(path)) {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => panic!("{id}: read current artifact {}: {error}", path.display()),
            };
            if let Ok(source) = std::str::from_utf8(&bytes) {
                mapping.push_str(source);
                mapping.push('\n');
                if path.extension().is_some_and(|extension| extension == "rs") {
                    names.extend(baseline::test_names(source));
                }
            }
        }
        for ticket in [
            format!(".arca/ticket/{id}.md"),
            format!(".arca/ticket/archive/{id}.md"),
        ] {
            if let Ok(source) = fs::read_to_string(root.join(ticket)) {
                mapping.push_str(&source);
            }
        }
        names.sort();
        let mut expected = strings(record, "test_names");
        expected.sort();
        for expected_name in &expected {
            assert!(
                names.contains(expected_name),
                "{id}: landed lane {expected_name} remains declared"
            );
        }
        for requirement in strings(record, "requirement_refs") {
            assert!(
                mapping.contains(&requirement),
                "{id}: requirement mapping {requirement} remains visible"
            );
        }
        if let Some(after) = &after {
            let migrated = matching_crate(after, id);
            let mut migrated_names = strings(migrated, "test_names");
            migrated_names.sort();
            assert_eq!(
                migrated_names, expected,
                "{id}: this migration preserves the exact lane inventory"
            );
            for key in [
                "workspace_root",
                "target_directory",
                "targets",
                "features",
                "manifest_sha256",
                "lock_sha256",
                "expired",
            ] {
                assert_eq!(
                    record[key], migrated[key],
                    "{id}: migration preserves Cargo {key}"
                );
            }
            assert_eq!(
                engine_hooks(record),
                engine_hooks(migrated),
                "{id}: migration preserves Engine hook features"
            );
            let migrated_refs = strings(migrated, "requirement_refs");
            for requirement in strings(record, "requirement_refs") {
                assert!(
                    migrated_refs.contains(&requirement),
                    "{id}: migration keeps requirement {requirement}"
                );
            }
        } else {
            metadata_boundaries(
                &root,
                id,
                record,
                before["target_triple"]
                    .as_str()
                    .expect("recorded host target"),
            );
        }
    }
    aggregate_fixture_control();
    observation_reuse_control();
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Preservation: frozen evidence stays reconstructible and representative
/// pre-existing fault boundaries retain their exact refusal/recovery behavior.
#[test]
fn wcpv_006() {
    let root = repo_root();
    let before = inventory(INVENTORY, "before");
    let after = (!baseline_mode()).then(|| inventory(AFTER_INVENTORY, "after"));
    for file in records(&before, "files") {
        let path = file["before_path"].as_str().expect("before-source path");
        let bytes = fs::read(root.join(path))
            .unwrap_or_else(|error| panic!("frozen before-source {path}: {error}"));
        assert_eq!(
            digest(&bytes),
            file["sha256"].as_str().unwrap(),
            "frozen source artifact {path} remains unchanged"
        );
        if let Some(after) = &after {
            let original_path = file["path"].as_str().unwrap();
            let owner = Path::new(original_path)
                .components()
                .nth(1)
                .unwrap()
                .as_os_str()
                .to_str()
                .unwrap();
            let original_crate = matching_crate(&before, owner);
            if original_crate["expired"].as_bool() == Some(true)
                || Path::new(original_path)
                    .file_name()
                    .is_some_and(|name| name == "Cargo.toml" || name == "Cargo.lock")
            {
                let migrated = records(after, "files")
                    .iter()
                    .find(|entry| entry["path"].as_str() == Some(original_path))
                    .unwrap_or_else(|| {
                        panic!("migration omitted preserved private input {original_path}")
                    });
                assert_eq!(
                    file["sha256"], migrated["sha256"],
                    "migration preserved private input {original_path}"
                );
            }
        }
    }
    for file in records(&before, "preserved") {
        let path = file["path"].as_str().unwrap();
        // Expired sources and private manifests/locks describe this migration,
        // not a ban on a later authorized port or dependency update.
        if after.is_none() || !Path::new(path).starts_with("test-hidden") {
            let current = fs::read(root.join(path));
            let unchanged = current
                .as_ref()
                .is_ok_and(|bytes| digest(bytes) == file["sha256"].as_str().unwrap());
            if !unchanged && !reopened_gap_record(&root, &before, path) {
                let bytes =
                    current.unwrap_or_else(|error| panic!("preserved artifact {path}: {error}"));
                assert_eq!(
                    digest(&bytes),
                    file["sha256"].as_str().unwrap(),
                    "historical artifact {path} remains byte-identical"
                );
            }
        }
        if let Some(after) = &after {
            let preserved = records(after, "preserved")
                .iter()
                .find(|entry| entry["path"].as_str() == Some(path))
                .unwrap_or_else(|| panic!("after inventory omitted historical artifact {path}"));
            assert_eq!(
                preserved["sha256"], file["sha256"],
                "migration preserves historical artifact {path}"
            );
        }
    }
    for prefix in records(&before, "prefixes") {
        let path = prefix["path"].as_str().unwrap();
        let bytes = fs::read(root.join(path)).unwrap();
        let length = usize::try_from(prefix["byte_length"].as_integer().unwrap()).unwrap();
        assert!(
            bytes.len() >= length,
            "append-only history {path} was truncated"
        );
        assert_eq!(
            digest(&bytes[..length]),
            prefix["sha256"].as_str().unwrap(),
            "append-only prefix {path} remains unchanged"
        );
        if let Some(after) = &after {
            let preserved = records(after, "prefixes")
                .iter()
                .find(|entry| entry["path"].as_str() == Some(path))
                .unwrap_or_else(|| panic!("after inventory omitted the frozen prefix of {path}"));
            assert_eq!(
                preserved["byte_length"], prefix["byte_length"],
                "after capture checks the same frozen prefix"
            );
            assert_eq!(
                preserved["sha256"], prefix["sha256"],
                "migration preserves the original history prefix"
            );
        }
    }
    if let Some(after) = &after {
        if migration_unlanded(&root) {
            if let Err(error) = bound_to_current(&root, after) {
                panic!("the after inventory must describe this tree: {error}");
            }
        }
    }
    after_binding_control();
    preserved_fault("before-verdict-archive");
    preserved_fault("before-state-replace");
}

/// The schema reopens a satisfied gap record by moving it from the residual
/// archive back to the active folder, and it may return to the archive with new
/// bytes once satisfied again. Such a frozen archived record may differ only
/// when a line appended to `.arca/log.md` after the frozen prefix names it and
/// a reopen, and exactly one copy - archived or active, same name - keeps its
/// `residual-id`.
fn reopened_gap_record(root: &Path, before: &toml::Value, path: &str) -> bool {
    let Some(name) = path.strip_prefix(".arca/residual/archive/") else {
        return false;
    };
    let Some(id) = name.strip_suffix(".md").filter(|id| id.starts_with("res-")) else {
        return false;
    };
    let prefix = records(before, "prefixes")
        .iter()
        .find(|entry| entry["path"].as_str() == Some(".arca/log.md"))
        .expect("the inventory freezes the log prefix");
    let length = usize::try_from(prefix["byte_length"].as_integer().unwrap()).unwrap();
    let log = fs::read(root.join(".arca/log.md")).expect("read the append-only log");
    let appended = String::from_utf8_lossy(log.get(length..).unwrap_or_default());
    // A lone CR ends a line too, so one displayed line never lends its
    // reopen to a record named on the next.
    let logged = appended
        .split(['\n', '\r'])
        .any(|line| line.to_lowercase().contains("reopen") && names_record(line, id));
    let copies: Vec<Vec<u8>> = [root.join(path), root.join(".arca/residual").join(name)]
        .iter()
        .filter_map(|copy| fs::read(copy).ok())
        .collect();
    // The record's own id is its first `residual-id:` line, the reading the
    // citation writers use.
    let wanted = format!("residual-id: \"{id}\"");
    logged
        && copies.len() == 1
        && String::from_utf8_lossy(&copies[0])
            .lines()
            .find(|line| line.trim_start().starts_with("residual-id:"))
            .is_some_and(|line| line.trim() == wanted)
}

/// Whether `line` names record `id` as a whole token, so `res-11` never
/// matches `res-110`.
fn names_record(line: &str, id: &str) -> bool {
    line.match_indices(id).any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + id.len()..].chars().next();
        !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '-')
            && !after.is_some_and(|c| c.is_ascii_alphanumeric())
    })
}

/// Whether this migration is still the latest word on the private crates:
/// its ticket record carries no landing stamp yet. Until then every
/// after-inventory row must be the current bytes. Once it lands the rows are
/// history: a later authorized port or dependency update may change live
/// files, and the protected rows stay bound through the before-copy corpus,
/// whose bytes they must equal.
fn migration_unlanded(root: &Path) -> bool {
    let record = [".arca/ticket/t-126.md", ".arca/ticket/archive/t-126.md"]
        .iter()
        .find_map(|path| fs::read_to_string(root.join(path)).ok())
        .expect("the migration's ticket record exists");
    let stamp = record
        .lines()
        .find_map(|line| line.strip_prefix("landed-commit:"))
        .expect("the ticket record declares its landing stamp");
    stamp.trim().trim_matches('"').is_empty()
}

/// Every `files` and `preserved` row of `after` against the bytes now under
/// `root`; the first absent or different file is named.
fn bound_to_current(root: &Path, after: &toml::Value) -> Result<(), String> {
    for key in ["files", "preserved"] {
        for row in records(after, key) {
            let path = row["path"].as_str().expect("inventory row path");
            let bytes = fs::read(root.join(path))
                .map_err(|error| format!("{key} row {path} is not on disk: {error}"))?;
            if Some(digest(&bytes).as_str()) != row["sha256"].as_str() {
                return Err(format!("{key} row {path} differs from the current bytes"));
            }
        }
    }
    Ok(())
}

/// The binding refuses an expired source changed after capture and accepts
/// its unchanged twin.
fn after_binding_control() {
    for changed in [false, true] {
        let fixture = Scratch::new("after-binding");
        let mut rows = String::from("preserved = []\n");
        for (path, body) in [
            (
                "test-hidden/t-901/EXPIRED.toml",
                "edition = \"edition-001\"\n",
            ),
            ("test-hidden/t-901/src/lib.rs", "// expired source\n"),
        ] {
            let file = fixture.0.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, body).unwrap();
            rows.push_str(&format!(
                "[[files]]\npath = \"{path}\"\nsha256 = \"{}\"\n",
                digest(body.as_bytes())
            ));
        }
        let after: toml::Value = rows.parse().unwrap();
        if changed {
            fs::write(
                fixture.0.join("test-hidden/t-901/src/lib.rs"),
                "// edited after capture\n",
            )
            .unwrap();
        }
        match bound_to_current(&fixture.0, &after) {
            Ok(()) => assert!(!changed, "a changed expired source must be refused"),
            Err(error) => {
                assert!(changed, "the unchanged twin must pass: {error}");
                assert!(
                    error.contains("test-hidden/t-901/src/lib.rs"),
                    "the refusal names the changed file: {error}"
                );
            }
        }
    }
}

/// A real existing helper must ignore foreign Git addressing in child processes.
/// The parent environment is never changed, even while probes run concurrently.
#[test]
fn wcpv_005() {
    match std::env::var(CHILD_MODE).ok().as_deref() {
        Some("git-isolation") => {
            git_isolation_child();
            return;
        }
        Some("fault-isolation") => {
            fault_isolation_child();
            return;
        }
        None => {}
        Some(other) => panic!("unknown isolated probe mode {other:?}"),
    }
    let first = victim("victim-a");
    let second = victim("victim-b");
    let first_before = snapshot(&first.0);
    let second_before = snapshot(&second.0);

    let positive = probe(&first.0, None).wait_with_output().unwrap();
    assert!(
        text(&positive).contains("running 1 test")
            && text(&positive).contains("test wcpv_005 ... ok"),
        "the exact positive Git probe executed: {}",
        text(&positive)
    );
    assert!(
        positive.status.success(),
        "clean positive control must pass: {}",
        text(&positive)
    );
    assert_eq!(
        snapshot(&first.0),
        first_before,
        "a clean helper leaves its caller directory alone"
    );

    let left = probe(&second.0, Some(&first.0));
    let right = probe(&first.0, Some(&second.0));
    let left = left.wait_with_output().expect("reap first isolation probe");
    let right = right
        .wait_with_output()
        .expect("reap second isolation probe");
    let mut failures = Vec::new();
    for (label, output) in [("first", &left), ("second", &right)] {
        if !text(output).contains("running 1 test") || !text(output).contains("test wcpv_005 ...") {
            failures.push(format!(
                "{label} poisoned child did not execute the exact test:\n{}",
                text(output)
            ));
        }
        if !output.status.success() {
            failures.push(format!("{label} poisoned child failed:\n{}", text(output)));
        }
    }
    if snapshot(&first.0) != first_before {
        failures.push("first foreign repository was changed".to_owned());
    }
    if snapshot(&second.0) != second_before {
        failures.push("second foreign repository was changed".to_owned());
    }
    assert!(
        failures.is_empty(),
        "fixture isolation failed:\n{}",
        failures.join("\n")
    );

    // Keep the existing Git leak as the initial red result. Once it is fixed,
    // both a clean positive scenario and an ambient-fault twin must advance.
    for inherited_fault in [false, true] {
        let output = fault_probe(&first.0, inherited_fault)
            .wait_with_output()
            .expect("reap fault-environment probe");
        assert!(
            text(&output).contains("running 1 test"),
            "the named fault probe actually ran"
        );
        assert!(
            output.status.success(),
            "fault isolation (inherited={inherited_fault}): {}",
            text(&output)
        );
    }
    assert_eq!(
        snapshot(&first.0),
        first_before,
        "fault probes preserve their caller's repository"
    );
    assert_eq!(
        snapshot(&second.0),
        second_before,
        "independent victim remains unchanged"
    );
}

// --- SVEV-003: one reader of Cargo's build messages ---------------------------

/// What one SVEV-003 inventory finding is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Finding {
    /// `--message-format` or `compiler-artifact`, comments included: a source
    /// reading Cargo's build messages itself.
    Reader,
    /// A test of whether a build-redirect variable is set: a lane refusing
    /// (or falling back) because Cargo's output may have moved.
    Refusal,
    /// A `debug` or `release` build-output folder joined into a path.
    FixedJoin,
}

impl Finding {
    fn label(self) -> &'static str {
        match self {
            Self::Reader => "build-message reader",
            Self::Refusal => "redirect refusal",
            Self::FixedJoin => "fixed join",
        }
    }
}

/// Words only a source reading Cargo's build messages carries.
const BUILD_MESSAGE_WORDS: [&str; 2] = ["--message-format", "compiler-artifact"];

/// The variables that move Cargo's build output.
const REDIRECT_VARIABLES: [&str; 3] = [
    "CARGO_TARGET_DIR",
    "CARGO_BUILD_TARGET_DIR",
    "CARGO_BUILD_TARGET",
];

/// The calls whose answer says whether a variable is set.
const PRESENCE_TESTS: [&str; 8] = [
    "is_none",
    "is_some",
    "is_ok",
    "is_err",
    "is_some_and",
    "is_none_or",
    "is_ok_and",
    "is_err_and",
];

/// Cargo's build-output profile folders.
const PROFILES: [&str; 2] = ["debug", "release"];

/// The binaries this repository builds and its test helpers launch.
const LAUNCHED_BINARIES: [&str; 4] = ["rtm", "rtm-qa", "edition-cut", "snapshot-manifest"];

/// One named exception: a file, or every file under a folder ending in `/`;
/// the finding kinds it may hold; and why.
struct Exception {
    path: &'static str,
    kinds: &'static [Finding],
    reason: &'static str,
}

/// SVEV-003's exceptions, file by file. The baseline support is exempt only as
/// the one reader: its fixed joins stay findings. No exception covers a
/// redirect refusal.
const EXCEPTIONS: [Exception; 12] = [
    Exception {
        path: "test/qa/src/baseline.rs",
        kinds: &[Finding::Reader],
        reason: "the one shared reader of Cargo's build messages (`built_executable`)",
    },
    Exception {
        path: "test/qa/tests/t045_bootstrap_doctor.rs",
        kinds: &[Finding::FixedJoin],
        reason: "out of scope (i-063): inspects the bootstrap, which belongs to \
                 i-064-cargo-built-bootstrap-engine",
    },
    Exception {
        path: "test/qa/tests/t102_stable_bootstrap.rs",
        kinds: &[Finding::FixedJoin],
        reason: "out of scope (i-063): inspects the bootstrap, which belongs to \
                 i-064-cargo-built-bootstrap-engine",
    },
    Exception {
        path: "test-hidden/t-102/",
        kinds: &[Finding::FixedJoin],
        reason: "the stable bootstrap suite's private lane: the same bootstrap product, \
                 i-064's (a site i-063's evidence does not name)",
    },
    Exception {
        path: "test/qa/tests/t101_engine_channels.rs",
        kinds: &[Finding::FixedJoin],
        reason: "out of scope (i-063): `target/debug/rtm` is a made-up recorded identity \
                 that no build produces",
    },
    Exception {
        path: "test-hidden/t-101/",
        kinds: &[Finding::FixedJoin],
        reason: "the Engine channels suite's private lane: the same made-up recorded \
                 identity (a site i-063's evidence does not name)",
    },
    Exception {
        path: "test/qa/tests/t110_post_split_ports.rs",
        kinds: &[Finding::FixedJoin],
        reason: "out of scope (i-063): only names a path for text normalization and runs nothing",
    },
    Exception {
        path: "test/qa/tests/t126_shared_private_test_support.rs",
        kinds: &[Finding::Reader, Finding::FixedJoin],
        reason: "this inventory's own patterns and controls, and a fixture lane's folder \
                 (out of scope, i-063)",
    },
    Exception {
        path: "test/qa/tests/t137_build_message_test_engine.rs",
        kinds: &[Finding::Reader, Finding::FixedJoin],
        reason: "the t-137 public oracle: its independent comparison build reads Cargo's \
                 messages, and its scratch copies plant stand-ins at the fixed folder",
    },
    Exception {
        path: "test-hidden/t-137/",
        kinds: &[Finding::Reader, Finding::FixedJoin],
        reason: "the t-137 private lane: the same independent comparison builds and \
                 planted stand-ins",
    },
    Exception {
        path: "test-hidden/t-144/",
        kinds: &[Finding::Reader, Finding::FixedJoin],
        reason: "the t-144 private lane (i-064): its stand-in cargo prints the build report the \
                 bootstrap reads, and it plants stand-ins at the fixed folders the bootstrap must \
                 never report",
    },
    Exception {
        path: "test-hidden/t-145/",
        kinds: &[Finding::FixedJoin],
        reason: "the t-145 private lane (i-074): it plants the Engine at target/debug, where the \
                 bootstrap finds it without building, so a live Run can pin that very file",
    },
];

/// SVEV-003 / SVE-001: one reader of Cargo's build messages. A source
/// inventory over the test crate and every live private lane on the roster
/// finds `--message-format` and `compiler-artifact` only in the baseline
/// support, no lane testing whether a build-redirect variable is set, and no
/// `debug` or `release` build-output folder joined into a path outside the
/// exceptions named file by file. Every `target/` folder and the frozen
/// before-source corpus are not sources. The matchers first prove themselves
/// on known shapes, so a blind matcher cannot pass.
#[test]
fn svev_003() {
    inventory_controls();
    let root = repo_root();
    let (sources, lanes) = build_message_inventory(&root);
    let mut findings = Vec::new();
    let mut set_aside: BTreeMap<(String, Finding), (Vec<usize>, &str)> = BTreeMap::new();
    for (path, text) in &sources {
        let lines: Vec<&str> = text.lines().collect();
        for (line, kind) in source_findings(text) {
            match exception_for(path, kind) {
                Some(reason) => set_aside
                    .entry((path.clone(), kind))
                    .or_insert_with(|| (Vec::new(), reason))
                    .0
                    .push(line),
                None => findings.push(format!(
                    "  {path}:{line}: {}: {}",
                    kind.label(),
                    clip(lines.get(line - 1).copied().unwrap_or_default())
                )),
            }
        }
    }
    let exempt: Vec<String> = set_aside
        .iter()
        .map(|((path, kind), (lines, reason))| {
            let lines: Vec<String> = lines.iter().map(ToString::to_string).collect();
            format!("  {path}:{} ({}): {reason}", lines.join(","), kind.label())
        })
        .collect();
    assert!(
        findings.is_empty(),
        "SVEV-003: Cargo's build messages must be read only in test/qa/src/baseline.rs, no lane \
         may test whether a build-redirect variable is set, and no debug or release build-output \
         folder may be joined into a path outside the named exceptions; the inventory of {} \
         sources (test/qa/src, test/qa/tests, and {lanes} roster lanes) found {}:\n{}\n\
         set aside by named exceptions:\n{}",
        sources.len(),
        findings.len(),
        findings.join("\n"),
        exempt.join("\n")
    );
}

/// The matchers on known shapes first: each control is a source text and
/// exactly the findings it must yield, by line.
fn inventory_controls() {
    use Finding::{FixedJoin, Reader, Refusal};
    let controls: &[(&str, &[(usize, Finding)])] = &[
        // Readers, comments included.
        (
            "//! from cargo's own `compiler-artifact` message\nlet x = 1;\n",
            &[(1, Reader)],
        ),
        ("command.arg(\"--message-format=json\");\n", &[(1, Reader)]),
        // Refusals: a presence test on a redirect variable, split or not.
        (
            "assert!(\n    std::env::var_os(\"CARGO_TARGET_DIR\").is_none(),\n    \"unset it\"\n);\n",
            &[(2, Refusal)],
        ),
        (
            "if std::env::var(\"CARGO_BUILD_TARGET_DIR\")\n    .is_ok()\n{\n    panic!();\n}\n",
            &[(1, Refusal)],
        ),
        // ... the `_and`/`_or` presence tests ...
        (
            "assert!(!std::env::var_os(\"CARGO_TARGET_DIR\").is_some_and(|dir| !dir.is_empty()));\n",
            &[(1, Refusal)],
        ),
        (
            "assert!(std::env::var_os(\"CARGO_BUILD_TARGET\").is_none_or(|target| target.is_empty()));\n",
            &[(1, Refusal)],
        ),
        (
            "if std::env::var(\"CARGO_BUILD_TARGET_DIR\").is_ok_and(|dir| !dir.is_empty()) {}\n",
            &[(1, Refusal)],
        ),
        (
            "if std::env::var(\"CARGO_TARGET_DIR\").is_err_and(|error| error != NotPresent) {}\n",
            &[(1, Refusal)],
        ),
        // ... and a read bound by `let Some(..)`/`let Ok(..)`, or matched on.
        (
            "if let Some(dir) = std::env::var_os(\"CARGO_TARGET_DIR\") {\n    panic!(\"{dir:?}\");\n}\n",
            &[(1, Refusal)],
        ),
        (
            "let Ok(dir) = std::env::var(\"CARGO_BUILD_TARGET_DIR\") else {\n    return;\n};\n",
            &[(1, Refusal)],
        ),
        (
            "while let Some(_) = env::var_os(\"CARGO_BUILD_TARGET\") {}\n",
            &[(1, Refusal)],
        ),
        (
            "match std::env::var(\"CARGO_TARGET_DIR\").as_deref() {\n    Ok(_) => panic!(),\n    Err(_) => {}\n}\n",
            &[(1, Refusal)],
        ),
        // Not refusals: a comparison, a child's setting, a comment, a plain
        // read, an equality test, and a lookup that is not the environment.
        (
            "assert_eq!(\n    std::env::var_os(\"CARGO_TARGET_DIR\"),\n    expected\n);\n",
            &[],
        ),
        ("command.env(\"CARGO_TARGET_DIR\", target);\n", &[]),
        (
            "// std::env::var_os(\"CARGO_TARGET_DIR\").is_none()\n",
            &[],
        ),
        ("let dir = std::env::var_os(\"CARGO_TARGET_DIR\");\n", &[]),
        (
            "let same = Some(dir) == std::env::var_os(\"CARGO_TARGET_DIR\");\n",
            &[],
        ),
        (
            "if let Some(dir) = settings.get(\"CARGO_TARGET_DIR\") {}\n",
            &[],
        ),
        // Fixed joins in every spelling the tree uses.
        (
            "let binary = root\n    .join(\"target/debug\")\n    .join(format!(\"rtm-qa{}\", SUFFIX));\n",
            &[(2, FixedJoin)],
        ),
        (
            "let binary = build_dir\n    .join(\"debug\")\n    .join(name);\n",
            &[(2, FixedJoin)],
        ),
        (
            "let binary = root.join(\"target\").join(\"release\").join(name);\n",
            &[(1, FixedJoin)],
        ),
        (
            "let path = PathBuf::from(format!(\"{}/target/debug/{name}\", root.display()));\n",
            &[(1, FixedJoin)],
        ),
        ("resolved: \"target/debug/rtm\".into(),\n", &[(1, FixedJoin)]),
        (
            "let cut = Path::new(r\"out\\release\\edition-cut.exe\");\n",
            &[(1, FixedJoin)],
        ),
        (
            "let quote = '\"';\nlet built = root.join(\"target/debug\");\n",
            &[(2, FixedJoin)],
        ),
        // A bare `debug` in any path-building call.
        ("exe.push(\"debug\");\n", &[(1, FixedJoin)]),
        (
            "let built = out.join(Path::new(\"debug\"));\n",
            &[(1, FixedJoin)],
        ),
        (
            "let built = out.join(if release { \"release\" } else { \"debug\" });\n",
            &[(1, FixedJoin)],
        ),
        (
            "let built = out.join(format!(\"debug/{name}{}\", EXE_SUFFIX));\n",
            &[(1, FixedJoin)],
        ),
        // Not fixed joins: release markers, other folders, fixture text,
        // prose, a `debug` word outside a path, a `debug.log` file, and
        // comments.
        (
            "let release = barrier_dir.join(\"release\");\nlet held = base.path().join(\"release\");\n",
            &[],
        ),
        ("let marker = scratch.join(\"release\");\n", &[]),
        (
            "let proof = fixture.linked.join(\"artifacts/release/proof.txt\");\n",
            &[],
        ),
        ("tree.write(\"target/debug/build.log\", &carrier);\n", &[]),
        (
            "panic!(\"selecting the hard-coded target/debug binary\");\n",
            &[],
        ),
        ("let level = \"debug\";\n", &[]),
        ("let log = scratch.join(\"debug.log\");\n", &[]),
        (
            "/// see `root.join(\"target/debug\").join(\"rtm-qa\")`\n/* root.join(\"debug\") */\n",
            &[],
        ),
    ];
    for (source, expected) in controls {
        let found: Vec<(usize, Finding)> = source_findings(source).into_iter().collect();
        assert_eq!(
            found, *expected,
            "SVEV-003 control: the inventory's matchers misread this source:\n{source}"
        );
    }
}

/// Every Rust source of the test crate and of each live lane on the roster in
/// `.ratmac/lanes.toml`, keyed by its `/`-separated path under `root`, and
/// the number of roster lanes. Every `target/` folder and the frozen
/// before-source corpus are left out; a roster lane without a folder, or any
/// inventoried folder without a Rust source, fails the check, so the
/// inventory is never vacuous.
fn build_message_inventory(root: &Path) -> (BTreeMap<String, String>, usize) {
    let declaration = root.join(".ratmac").join("lanes.toml");
    let lanes: toml::Value = fs::read_to_string(&declaration)
        .unwrap_or_else(|error| panic!("SVEV-003: read {}: {error}", declaration.display()))
        .parse()
        .unwrap_or_else(|error| panic!("SVEV-003: parse {}: {error}", declaration.display()));
    let roots = lanes.get("roots");
    let folder = roots
        .and_then(|roots| roots.get("lanes"))
        .and_then(toml::Value::as_str)
        .expect("SVEV-003: .ratmac/lanes.toml names the lane folder (roots.lanes)");
    let roster: Vec<&str> = roots
        .and_then(|roots| roots.get("roster"))
        .and_then(toml::Value::as_array)
        .expect("SVEV-003: .ratmac/lanes.toml declares the lane roster (roots.roster)")
        .iter()
        .map(|id| {
            id.as_str()
                .expect("SVEV-003: every roster entry is a lane id")
        })
        .collect();
    assert!(
        !roster.is_empty(),
        "SVEV-003: the lane roster names no lane"
    );
    let missing: Vec<String> = roster
        .iter()
        .filter(|id| !root.join(folder).join(id).is_dir())
        .map(|id| format!("{folder}/{id}"))
        .collect();
    assert!(
        missing.is_empty(),
        "SVEV-003: every roster lane must have its folder, or the inventory is vacuous; \
         missing: {missing:?}"
    );
    let frozen = root
        .join(folder)
        .join("t-126")
        .join("fixtures")
        .join("before");
    let mut files = Vec::new();
    for start in [root.join("test/qa/src"), root.join("test/qa/tests")]
        .into_iter()
        .chain(roster.iter().map(|id| root.join(folder).join(id)))
    {
        rust_sources(&start, &frozen, &mut files);
    }
    let mut sources = BTreeMap::new();
    for file in files {
        let relative: Vec<String> = file
            .strip_prefix(root)
            .expect("SVEV-003: inventoried sources lie under the repository")
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect();
        let bytes = fs::read(&file)
            .unwrap_or_else(|error| panic!("SVEV-003: read {}: {error}", file.display()));
        sources.insert(
            relative.join("/"),
            String::from_utf8_lossy(&bytes).into_owned(),
        );
    }
    let empty: Vec<String> = ["test/qa/src/", "test/qa/src/bin/", "test/qa/tests/"]
        .into_iter()
        .map(str::to_owned)
        .chain(roster.iter().map(|id| format!("{folder}/{id}/")))
        .filter(|prefix| !sources.keys().any(|path| path.starts_with(prefix.as_str())))
        .collect();
    assert!(
        empty.is_empty(),
        "SVEV-003: every inventoried folder must hold Rust sources; none under {empty:?}"
    );
    assert!(
        sources.contains_key("test/qa/src/baseline.rs"),
        "SVEV-003: the inventory must read the baseline support"
    );
    (sources, roster.len())
}

/// The `.rs` files under `folder`, skipping every `target/` folder and the
/// frozen corpus.
fn rust_sources(folder: &Path, frozen: &Path, found: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(folder)
        .unwrap_or_else(|error| panic!("SVEV-003: list {}: {error}", folder.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|error| panic!("SVEV-003: list {}: {error}", folder.display()));
        let path = entry.path();
        let kind = entry
            .file_type()
            .unwrap_or_else(|error| panic!("SVEV-003: inspect {}: {error}", path.display()));
        if kind.is_dir() {
            if entry.file_name() != "target" && path != frozen {
                rust_sources(&path, frozen, found);
            }
        } else if kind.is_file() && path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

/// The exception that covers a `kind` finding in `path`, by its reason.
fn exception_for(path: &str, kind: Finding) -> Option<&'static str> {
    EXCEPTIONS
        .iter()
        .find(|exception| {
            exception.kinds.contains(&kind)
                && if exception.path.ends_with('/') {
                    path.starts_with(exception.path)
                } else {
                    path == exception.path
                }
        })
        .map(|exception| exception.reason)
}

/// One source line, trimmed and cut short for a failure message.
fn clip(line: &str) -> String {
    let line = line.trim();
    match line.char_indices().nth(110) {
        Some((cut, _)) => format!("{}...", &line[..cut]),
        None => line.to_owned(),
    }
}

/// The findings in one Rust source, by 1-based line. Readers count anywhere,
/// comments included; refusals and fixed joins count only in code.
fn source_findings(source: &str) -> BTreeSet<(usize, Finding)> {
    let mut found = BTreeSet::new();
    for (index, line) in source.lines().enumerate() {
        if BUILD_MESSAGE_WORDS.iter().any(|word| line.contains(word)) {
            found.insert((index + 1, Finding::Reader));
        }
    }
    let tokens = rust_tokens(source);
    // One entry per open parenthesis: whether that call forms a path.
    let mut calls: Vec<bool> = Vec::new();
    for (at, (line, token)) in tokens.iter().enumerate() {
        match token {
            Token::Mark('(') => calls.push(forms_path(&tokens[..at])),
            Token::Mark(')') => {
                calls.pop();
            }
            Token::Text(text) => {
                let segments: Vec<&str> = text
                    .split(['/', '\\'])
                    .filter(|segment| !segment.is_empty())
                    .collect();
                let in_path = calls.iter().any(|forms| *forms);
                // A bare `debug` is a build-output folder in any path; a bare
                // `release` only on a build folder (see `bare_profile_join`).
                if (in_path && (profile_under_target(&segments) || segments.contains(&"debug")))
                    || profile_then_binary(&segments)
                    || bare_profile_join(&tokens, at)
                {
                    found.insert((*line, Finding::FixedJoin));
                }
                if presence_test(&tokens, at) {
                    found.insert((*line, Finding::Refusal));
                }
            }
            _ => {}
        }
    }
    found
}

/// `target` then `debug` or `release`: a build-output folder named in a path.
fn profile_under_target(segments: &[&str]) -> bool {
    segments
        .windows(2)
        .any(|pair| pair[0] == "target" && PROFILES.contains(&pair[1]))
}

/// `debug` or `release` then a launched binary (`rtm-qa.exe`, `rtm{}`, ...).
fn profile_then_binary(segments: &[&str]) -> bool {
    segments.windows(2).any(|pair| {
        let stem: String = pair[1]
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '-')
            .collect();
        PROFILES.contains(&pair[0]) && LAUNCHED_BINARIES.contains(&stem.as_str())
    })
}

/// Whether the call opened by the next `(` forms a path from its arguments:
/// `.join(`, `.push(`, `Path::new(`, `PathBuf::from(`, and the like.
fn forms_path(before: &[(usize, Token)]) -> bool {
    let back = |count: usize| {
        before
            .len()
            .checked_sub(count)
            .map(|index| &before[index].1)
    };
    match back(1) {
        Some(Token::Word(name)) if name == "join" || name == "push" => {
            back(2) == Some(&Token::Mark('.'))
        }
        Some(Token::Word(name)) if name == "new" || name == "from" => {
            back(2) == Some(&Token::Mark(':'))
                && back(3) == Some(&Token::Mark(':'))
                && matches!(back(4), Some(Token::Word(owner)) if owner == "Path" || owner == "PathBuf")
        }
        _ => false,
    }
}

/// `.join("release")` on a receiver naming a build folder (`target`,
/// `build_dir`, `.join("target")`, ...). A `release` marker beside a barrier
/// or a scratch folder is not a build folder, so a bare `release` counts only
/// here; a bare `debug` counts in any path-building call.
fn bare_profile_join(tokens: &[(usize, Token)], at: usize) -> bool {
    let token = |index: Option<usize>| {
        index
            .and_then(|index| tokens.get(index))
            .map(|(_, token)| token)
    };
    let back = |count: usize| token(at.checked_sub(count));
    let Some(Token::Text(text)) = token(Some(at)) else {
        return false;
    };
    let component = text.trim_end_matches(['/', '\\']);
    let joined = component == "release"
        && back(1) == Some(&Token::Mark('('))
        && matches!(back(2), Some(Token::Word(call)) if call == "join")
        && back(3) == Some(&Token::Mark('.'))
        && token(Some(at + 1)) == Some(&Token::Mark(')'));
    if !joined {
        return false;
    }
    match back(4) {
        Some(Token::Word(name)) => names_build_folder(name),
        Some(Token::Mark(')')) => match (back(5), back(6)) {
            (Some(Token::Text(folder)), Some(Token::Mark('('))) => {
                folder.trim_end_matches(['/', '\\']) == "target"
            }
            (Some(Token::Mark('(')), Some(Token::Word(name))) => names_build_folder(name),
            _ => false,
        },
        _ => false,
    }
}

fn names_build_folder(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.contains("target") || name.contains("build")
}

/// A redirect variable whose being set decides a branch: `"CARGO_TARGET_DIR")`
/// (or another redirect variable) followed by `.is_none()`, `.is_some()`,
/// `.is_ok()`, `.is_err()`, or their `_and`/`_or` forms; or a `std::env::var`
/// or `var_os` read of it bound by `let Some(..) =` or `let Ok(..) =` (`if
/// let`, `while let`, let-else) or matched on.
fn presence_test(tokens: &[(usize, Token)], at: usize) -> bool {
    let token = |index: Option<usize>| {
        index
            .and_then(|index| tokens.get(index))
            .map(|(_, token)| token)
    };
    let named = matches!(token(Some(at)), Some(Token::Text(name)) if REDIRECT_VARIABLES.contains(&name.as_str()))
        && token(Some(at + 1)) == Some(&Token::Mark(')'));
    if !named {
        return false;
    }
    if token(Some(at + 2)) == Some(&Token::Mark('.'))
        && matches!(token(Some(at + 3)), Some(Token::Word(test)) if PRESENCE_TESTS.contains(&test.as_str()))
    {
        return true;
    }
    let reads = token(at.checked_sub(1)) == Some(&Token::Mark('('))
        && matches!(token(at.checked_sub(2)), Some(Token::Word(call)) if call == "var" || call == "var_os");
    if !reads {
        return false;
    }
    // Back over the read's path (`std::env::`, `env::`) to where it starts.
    let mut start = at - 2;
    while start >= 3
        && token(Some(start - 1)) == Some(&Token::Mark(':'))
        && token(Some(start - 2)) == Some(&Token::Mark(':'))
        && matches!(token(Some(start - 3)), Some(Token::Word(_)))
    {
        start -= 3;
    }
    match token(start.checked_sub(1)) {
        Some(Token::Word(keyword)) => keyword == "match",
        Some(Token::Mark('=')) => binds_presence(tokens, start - 1),
        _ => false,
    }
}

/// Whether the `=` at `equals` closes a `let Some(..)` or `let Ok(..)`
/// pattern. The `=` of `==`, `!=`, `<=`, `+=`, and the like has another mark
/// before it, never the pattern's `)`.
fn binds_presence(tokens: &[(usize, Token)], equals: usize) -> bool {
    let token = |index: Option<usize>| {
        index
            .and_then(|index| tokens.get(index))
            .map(|(_, token)| token)
    };
    let Some(mut cursor) = equals.checked_sub(1) else {
        return false;
    };
    if token(Some(cursor)) != Some(&Token::Mark(')')) {
        return false;
    }
    // Back to the `(` that opens the pattern's payload.
    let mut depth = 0usize;
    loop {
        match token(Some(cursor)) {
            Some(Token::Mark(')')) => depth += 1,
            Some(Token::Mark('(')) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        let Some(previous) = cursor.checked_sub(1) else {
            return false;
        };
        cursor = previous;
    }
    matches!(token(cursor.checked_sub(1)), Some(Token::Word(variant)) if variant == "Some" || variant == "Ok")
        && matches!(token(cursor.checked_sub(2)), Some(Token::Word(keyword)) if keyword == "let")
}

/// One token of Rust source.
#[derive(Debug, PartialEq)]
enum Token {
    /// An identifier, keyword, or number.
    Word(String),
    /// A string literal's text, raw and byte forms included.
    Text(String),
    /// Any other character.
    Mark(char),
}

/// Rust source as tokens, each with the 1-based line it starts on. Comments
/// are dropped, string literals are decoded enough to read paths, and
/// character literals and lifetimes never open a string.
fn rust_tokens(source: &str) -> Vec<(usize, Token)> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut line = 1;
    let mut at = 0;
    while let Some(&character) = chars.get(at) {
        let next = chars.get(at + 1).copied();
        if character == '\n' {
            line += 1;
            at += 1;
        } else if character.is_whitespace() {
            at += 1;
        } else if character == '/' && next == Some('/') {
            while chars.get(at).is_some_and(|c| *c != '\n') {
                at += 1;
            }
        } else if character == '/' && next == Some('*') {
            let mut depth = 0usize;
            while let Some(&c) = chars.get(at) {
                let after = chars.get(at + 1).copied();
                if c == '/' && after == Some('*') {
                    depth += 1;
                    at += 2;
                } else if c == '*' && after == Some('/') {
                    depth -= 1;
                    at += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    line += usize::from(c == '\n');
                    at += 1;
                }
            }
        } else if character == '"' {
            let start = line;
            at += 1;
            let text = quoted(&chars, &mut at, &mut line);
            tokens.push((start, Token::Text(text)));
        } else if character == '\'' {
            at = past_quote(&chars, at);
        } else if character.is_alphanumeric() || character == '_' {
            let begin = at;
            while chars
                .get(at)
                .is_some_and(|c| c.is_alphanumeric() || *c == '_')
            {
                at += 1;
            }
            let word: String = chars[begin..at].iter().collect();
            let start = line;
            match (word.as_str(), chars.get(at)) {
                ("r" | "br" | "cr", Some('"' | '#')) => {
                    match raw_text(&chars, &mut at, &mut line) {
                        Some(text) => tokens.push((start, Token::Text(text))),
                        None => tokens.push((start, Token::Word(word))),
                    }
                }
                ("b" | "c", Some('"')) => {
                    at += 1;
                    let text = quoted(&chars, &mut at, &mut line);
                    tokens.push((start, Token::Text(text)));
                }
                _ => tokens.push((start, Token::Word(word))),
            }
        } else {
            tokens.push((line, Token::Mark(character)));
            at += 1;
        }
    }
    tokens
}

/// The text of a string literal whose opening quote is just behind `at`;
/// leaves `at` past the closing quote.
fn quoted(chars: &[char], at: &mut usize, line: &mut usize) -> String {
    let mut text = String::new();
    while let Some(&c) = chars.get(*at) {
        *at += 1;
        match c {
            '"' => break,
            '\\' => {
                let Some(&escaped) = chars.get(*at) else {
                    break;
                };
                *at += 1;
                match escaped {
                    'n' => text.push('\n'),
                    't' => text.push('\t'),
                    'r' => text.push('\r'),
                    '0' => text.push('\0'),
                    '\r' | '\n' => {
                        // A line continuation: the break and the next
                        // line's leading whitespace are not text.
                        *line += usize::from(escaped == '\n');
                        while let Some(&space) = chars.get(*at) {
                            if !space.is_whitespace() {
                                break;
                            }
                            *line += usize::from(space == '\n');
                            *at += 1;
                        }
                    }
                    other => text.push(other),
                }
            }
            other => {
                *line += usize::from(other == '\n');
                text.push(other);
            }
        }
    }
    text
}

/// The text of a raw string literal whose hashes or opening quote start at
/// `at`, or `None` (with `at` unmoved) for a raw identifier such as `r#type`.
fn raw_text(chars: &[char], at: &mut usize, line: &mut usize) -> Option<String> {
    let mut cursor = *at;
    let mut hashes = 0;
    while chars.get(cursor) == Some(&'#') {
        hashes += 1;
        cursor += 1;
    }
    if chars.get(cursor) != Some(&'"') {
        return None;
    }
    cursor += 1;
    let mut text = String::new();
    while let Some(&c) = chars.get(cursor) {
        cursor += 1;
        if c == '"' && (0..hashes).all(|offset| chars.get(cursor + offset) == Some(&'#')) {
            cursor += hashes;
            break;
        }
        *line += usize::from(c == '\n');
        text.push(c);
    }
    *at = cursor;
    Some(text)
}

/// Past a character literal at `at`, or just past the quote of a lifetime or
/// label, so neither can open a string.
fn past_quote(chars: &[char], at: usize) -> usize {
    match (chars.get(at + 1), chars.get(at + 2)) {
        (Some('\\'), _) => {
            let mut cursor = at + 3;
            while chars.get(cursor).is_some_and(|c| *c != '\'' && *c != '\n') {
                cursor += 1;
            }
            cursor + 1
        }
        (Some(_), Some('\'')) => at + 3,
        _ => at + 1,
    }
}

// --- SVEV-018: case-scoped test hooks ----------------------------------------

/// The prefix of every Engine test hook setting.
const HOOK_PREFIX: &str = "RATMAC_TEST_";

/// The shared support file. Its `hook_lane`, `HookLane`, and `StepFault`
/// definitions are the one central implementation of the hook lane, so the
/// settings they set and remove are exempt; every other place is not.
const HOOK_SUPPORT: &str = "test/qa/src/support.rs";

/// Words that stand before a parenthesis without calling a function.
const NOT_CALLS: [&str; 18] = [
    "as", "break", "dyn", "else", "fn", "for", "if", "impl", "in", "let", "loop", "match", "move",
    "mut", "ref", "return", "unsafe", "while",
];

/// Conversions that leave a setting's name as it was (`key.as_str()`).
const NAME_CONVERSIONS: [&str; 6] = [
    "as_ref",
    "as_str",
    "as_os_str",
    "clone",
    "to_owned",
    "to_string",
];

/// How a test case runs a place that sets or removes a hook setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum HookScope {
    /// In the shared test process, with no hook lane held.
    Open,
    /// In the shared test process, under the hook lane.
    Lane,
    /// In a child of the test binary that runs exactly this one case.
    Exact,
    /// Inside `support::isolated_test` for the case's own name.
    Isolated,
}

impl HookScope {
    fn label(self) -> &'static str {
        match self {
            Self::Open => "no lane",
            Self::Lane => "hook lane",
            Self::Exact => "--exact child",
            Self::Isolated => "isolated_test",
        }
    }
}

/// What the name argument of a setting call reads as.
#[derive(Clone, Debug, PartialEq, Eq)]
enum HookName {
    /// A `RATMAC_TEST_*` name.
    Hook,
    /// The enclosing function's parameter at this index (`self` not counted).
    Param(usize),
    /// Any other name, or one the scan cannot read.
    Other,
}

/// One function item of a source: its name, the type of the `impl` or
/// `trait` block it belongs to, and its signature and body.
struct HookItem {
    name: String,
    owner: Option<String>,
    test: bool,
    at: usize,
    params: Vec<(String, Option<String>)>,
    has_self: bool,
    body: Option<(usize, usize)>,
}

/// One parsed Rust source: its tokens, `use` names, string constants,
/// `mod` declarations (with any `#[path]`), and function items.
struct HookSource {
    tokens: Vec<(usize, Token)>,
    uses: BTreeMap<String, Vec<String>>,
    globs: Vec<Vec<String>>,
    consts: BTreeMap<String, String>,
    mods: Vec<(String, Option<String>)>,
    items: Vec<HookItem>,
    /// The innermost function whose body holds each token.
    within: Vec<Option<usize>>,
}

/// One compiled crate target: a library or a test binary, by its root
/// source; its files with their module names; and the libraries it uses.
struct HookUnit {
    library: Option<String>,
    files: Vec<(String, String)>,
    deps: Vec<usize>,
}

/// One call inside a function: where, what it may reach, and its arguments.
struct HookCall {
    line: usize,
    targets: Vec<usize>,
    args: Vec<HookName>,
    /// A path call (`Type::method(value, ...)`) passes a method's receiver first.
    receiver_first: bool,
    step_fault: bool,
    engine: bool,
    /// The call's value is kept by a named `let` binding (not `_`) or is the
    /// body's tail: a lane it takes stays held from this line on.
    bound: bool,
}

/// One function of one unit, with what its body does.
struct HookNode {
    unit: usize,
    file: String,
    module: String,
    item: usize,
    label: String,
    exempt: bool,
    direct: Vec<(usize, HookName)>,
    calls: Vec<HookCall>,
    isolated: Vec<String>,
    /// Closures passed to a call, by first and last line, with that call's
    /// index: their bodies run inside the callee.
    closures: Vec<(usize, usize, usize)>,
    /// The lines of a test's child-role branches: blocks guarded by an
    /// environment read that end the case (`return`, `exit`) or have an
    /// `else`. Only these run in an `--exact` child of the case.
    children: Vec<(usize, usize)>,
    /// The body ends by returning the shared hook lane (or a wrapper's).
    taker: bool,
    wraps: BTreeSet<usize>,
    sites: Vec<usize>,
}

/// One hook-setting place: path, line, and the function it is in.
type HookSite = (String, usize, String);

/// What the SVEV-018 scan saw: each hook-setting place a test case reaches,
/// with every case and how it runs it; and the findings.
struct HookReport {
    sites: BTreeMap<HookSite, BTreeSet<(String, HookScope)>>,
    findings: Vec<String>,
}

/// SVEV-018 / SVE-006: case-scoped test hooks. A source inventory over the
/// test crate and every declared, non-expired lane on the roster (no
/// `target/` folder, not the frozen before-source corpus) finds every place
/// a `RATMAC_TEST_*` setting is set or removed in the test process -
/// `set_var` or `remove_var` with that name, `StepFault::set`, or a wrapper
/// that passes such a name on - and follows calls inside the same test
/// binary up to its `#[test]` cases. Each place runs only inside
/// `support::isolated_test` for its own case, inside a child running exactly
/// one named case (`--exact`), or under the shared hook lane in a binary
/// whose every in-process Engine case also holds it. The matchers first
/// prove themselves on synthetic binaries and on known clean places of this
/// repository, so a blind scan cannot pass.
#[test]
fn svev_018() {
    hook_controls();
    let root = repo_root();
    let (sources, lanes) = build_message_inventory(&root);
    let expired = expired_lanes(&root);
    let folder = lane_folder(&root);
    let scanned: BTreeMap<String, String> = sources
        .into_iter()
        .filter(|(path, _)| {
            !expired
                .iter()
                .any(|prefix| path.starts_with(prefix.as_str()))
        })
        .collect();
    let report = hook_report(&scanned, &folder);
    hook_witnesses(&report, &scanned, &folder);
    let mut scopes: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, scope) in report.sites.values().flatten() {
        *scopes.entry(scope.label()).or_default() += 1;
    }
    eprintln!(
        "SVEV-018: {} hook places reached; case-place pairs by scope: {scopes:?}",
        report.sites.len()
    );
    assert!(
        report.findings.is_empty(),
        "SVEV-018: every place that sets or removes a RATMAC_TEST_* setting in the test process \
         must run inside support::isolated_test for its own case, in an --exact child of one \
         named case, or under the shared hook lane in a test binary whose every in-process \
         Engine case also holds it; the inventory of {} sources (test/qa/src, test/qa/tests, \
         and {} of {lanes} roster lanes; expired: {expired:?}) reached {} hook places and found \
         {}:\n{}",
        scanned.len(),
        lanes - expired.len(),
        report.sites.len(),
        report.findings.len(),
        report.findings.join("\n")
    );
}

/// The lane folder `.ratmac/lanes.toml` names (`roots.lanes`).
fn lane_folder(root: &Path) -> String {
    let declaration = root.join(".ratmac").join("lanes.toml");
    let lanes: toml::Value = fs::read_to_string(&declaration)
        .unwrap_or_else(|error| panic!("SVEV-018: read {}: {error}", declaration.display()))
        .parse()
        .unwrap_or_else(|error| panic!("SVEV-018: parse {}: {error}", declaration.display()));
    lanes
        .get("roots")
        .and_then(|roots| roots.get("lanes"))
        .and_then(toml::Value::as_str)
        .expect("SVEV-018: .ratmac/lanes.toml names the lane folder (roots.lanes)")
        .to_owned()
}

/// The scan on known shapes first: each control is a small repository of
/// sources and exactly the findings it must yield.
fn hook_controls() {
    const SUPPORT: &str = "use std::sync::{Mutex, MutexGuard};\n\
         static LANES: Mutex<()> = Mutex::new(());\n\
         pub struct HookLane(MutexGuard<'static, ()>);\n\
         pub fn hook_lane() -> HookLane {\n    std::env::remove_var(\"RATMAC_TEST_ANY\");\n    \
         HookLane(LANES.lock().unwrap())\n}\n\
         impl Drop for HookLane {\n    fn drop(&mut self) {\n        \
         std::env::remove_var(\"RATMAC_TEST_ANY\");\n    }\n}\n\
         pub struct StepFault;\n\
         impl StepFault {\n    pub fn set(point: &str) -> Self {\n        \
         std::env::set_var(\"RATMAC_TEST_STEP_FAULT\", point);\n        StepFault\n    }\n}\n\
         pub fn isolated_test(name: &str) -> bool {\n    name.is_empty()\n}\n";
    const LIB: &str = "pub mod support;\n";
    const BIN: &str = "test/qa/tests/control.rs";
    const LANE_LIB: &str = "lanes/t-900/src/lib.rs";
    const LANE_BIN: &str = "lanes/t-900/tests/hidden.rs";
    // Each control: its name, its sources by path, and its findings.
    type Control<'a> = (&'a str, &'a [(&'a str, &'a str)], &'a [&'a str]);
    let controls: &[Control<'_>] = &[
        (
            "an unlaned setter",
            &[(
                BIN,
                "#[test]\nfn a() {\n    std::env::set_var(\"RATMAC_TEST_A\", \"1\");\n}\n",
            )],
            &["unlaned: test/qa/tests/control.rs:3 a <- a"],
        ),
        (
            "a laned case beside an unlaned Engine sibling",
            &[(
                BIN,
                "use ratmac_qa::support;\n#[test]\nfn a() {\n    let _lane = support::hook_lane();\n    \
                 let _fault = support::StepFault::set(\"x\");\n}\n#[test]\nfn b() {\n    \
                 ratmac::cli::run_from(vec![]);\n}\n",
            )],
            &[
                "beside an unlaned Engine case: test/qa/tests/control.rs:5 a <- a",
                "Engine case without the lane: test/qa/tests/control.rs:9 b <- b",
            ],
        ),
        (
            "an Engine entry through an imported type",
            &[(
                BIN,
                "use ratmac::machine::MachineClass;\nuse ratmac_qa::support;\n#[test]\nfn a() {\n    \
                 let _lane = support::hook_lane();\n    std::env::set_var(\"RATMAC_TEST_A\", \"1\");\n}\n\
                 #[test]\nfn b() {\n    let _ = MachineClass::from_toml(\"\");\n}\n",
            )],
            &[
                "beside an unlaned Engine case: test/qa/tests/control.rs:6 a <- a",
                "Engine case without the lane: test/qa/tests/control.rs:10 b <- b",
            ],
        ),
        (
            "every arming case laned, and the static help text outside the lane",
            &[(
                BIN,
                "use ratmac::cli;\nuse ratmac_qa::support;\nfn rtm() {\n    cli::run_from(vec![]);\n}\n\
                 #[test]\nfn a() {\n    let _lane = support::hook_lane();\n    \
                 std::env::set_var(\"RATMAC_TEST_A\", \"1\");\n    rtm();\n}\n\
                 #[test]\nfn b() {\n    let _lane = support::hook_lane();\n    rtm();\n}\n\
                 #[test]\nfn c() {\n    let _ = cli::help(\"step\");\n}\n",
            )],
            &[
                "beside an unlaned Engine case: test/qa/tests/control.rs:9 a <- a",
                "Engine case without the lane: test/qa/tests/control.rs:19 c <- c",
            ],
        ),
        (
            "an isolated case of its own name, and one of another name",
            &[(
                BIN,
                "use ratmac_qa::support;\n#[test]\nfn a() {\n    if support::isolated_test(\"a\") {\n        \
                 arm();\n    }\n}\n#[test]\nfn b() {\n    if support::isolated_test(\"a\") {\n        \
                 arm();\n    }\n}\nfn arm() {\n    std::env::set_var(\"RATMAC_TEST_A\", \"1\");\n}\n",
            )],
            &["unlaned: test/qa/tests/control.rs:15 arm <- b"],
        ),
        (
            "an exact child of one named case",
            &[(
                BIN,
                "#[test]\nfn a() {\n    if std::env::var_os(\"ROLE\").is_some() {\n        \
                 std::env::set_var(\"RATMAC_TEST_A\", \"1\");\n    } else {\n        \
                 std::process::Command::new(\"x\").args([\"--exact\", \"a\"]);\n    }\n}\n",
            )],
            &[],
        ),
        (
            "an env_set wrapper in a lane library, unlaned",
            &[
                (
                    LANE_LIB,
                    "pub struct EnvVar {\n    key: String,\n}\n\
                     pub fn env_set(key: &str, value: &str) -> EnvVar {\n    \
                     std::env::set_var(key, value);\n    EnvVar { key: key.to_owned() }\n}\n\
                     impl Drop for EnvVar {\n    fn drop(&mut self) {\n        \
                     std::env::remove_var(&self.key);\n    }\n}\n",
                ),
                (
                    LANE_BIN,
                    "use ratmac_t900_hidden::env_set;\nconst HOOK: &str = \"RATMAC_TEST_B\";\n#[test]\n\
                     fn a() {\n    let _hook = env_set(HOOK, \"1\");\n    let _path = env_set(\"PATH\", \"x\");\n}\n",
                ),
            ],
            &["unlaned: lanes/t-900/tests/hidden.rs:5 a <- a"],
        ),
        (
            "a lane_guard wrapper and a private mutex named hook_lane",
            &[
                (
                    LANE_LIB,
                    "#[path = \"../../../test/qa/src/support.rs\"]\npub mod support;\n\
                     pub fn lane_guard() -> support::HookLane {\n    support::hook_lane()\n}\n",
                ),
                (
                    LANE_BIN,
                    "use ratmac_t900_hidden::{lane_guard, support::StepFault};\n\
                     use std::sync::{Mutex, MutexGuard};\nstatic PRIVATE: Mutex<()> = Mutex::new(());\n\
                     fn hook_lane() -> MutexGuard<'static, ()> {\n    PRIVATE.lock().unwrap()\n}\n\
                     #[test]\nfn guarded() {\n    let _lane = lane_guard();\n    let _fault = StepFault::set(\"x\");\n}\n\
                     #[test]\nfn private() {\n    let _lane = hook_lane();\n    let _fault = StepFault::set(\"y\");\n}\n",
                ),
            ],
            &["unlaned: lanes/t-900/tests/hidden.rs:15 private <- private"],
        ),
        (
            "the shared lane through a library re-export, and a function named as a value",
            &[
                (LANE_LIB, "pub use ratmac_qa::support;\n"),
                (
                    LANE_BIN,
                    "use ratmac_t900_hidden::support;\nfn arm(point: &str) -> support::StepFault {\n    \
                     support::StepFault::set(point)\n}\n#[test]\nfn laned() {\n    \
                     let _lane = support::hook_lane();\n    for run in [arm] {\n        let _fault = run(\"x\");\n    }\n}\n\
                     #[test]\nfn bare() {\n    let cases = [(\"c\", arm)];\n    let _ = cases;\n}\n",
                ),
            ],
            &["unlaned: lanes/t-900/tests/hidden.rs:3 arm <- bare"],
        ),
        (
            "an --exact case's parent body outside its child-role branch",
            &[(
                BIN,
                "#[test]\nfn a() {\n    if std::env::var_os(\"ROLE\").is_some() {\n        \
                 std::env::set_var(\"RATMAC_TEST_A\", \"1\");\n        return;\n    }\n    \
                 std::env::set_var(\"RATMAC_TEST_B\", \"1\");\n    \
                 std::process::Command::new(\"x\").args([\"--exact\", \"a\"]);\n}\n",
            )],
            &["unlaned: test/qa/tests/control.rs:7 a <- a"],
        ),
        (
            "an Engine method behind a field of an unknown receiver",
            &[
                (
                    LANE_LIB,
                    "pub use ratmac_qa::support;\npub struct Fixture;\nimpl Fixture {\n    \
                     pub fn run(&self) {\n        ratmac::cli::run_from(vec![]);\n    }\n}\n",
                ),
                (
                    LANE_BIN,
                    "use ratmac_t900_hidden::{support, Fixture};\nstruct Gov {\n    fx: Fixture,\n}\n\
                     fn gov() -> Gov {\n    Gov { fx: Fixture }\n}\n#[test]\nfn a() {\n    \
                     let _lane = support::hook_lane();\n    let _fault = support::StepFault::set(\"x\");\n}\n\
                     #[test]\nfn b() {\n    let state = gov();\n    state.fx.run();\n}\n",
                ),
            ],
            &[
                "beside an unlaned Engine case: lanes/t-900/tests/hidden.rs:11 a <- a",
                "Engine case without the lane: lanes/t-900/src/lib.rs:5 Fixture::run <- b",
            ],
        ),
        (
            "setting names a helper returns, set through a tuple loop",
            &[(
                BIN,
                "fn hooks() -> [(String, String); 1] {\n    \
                 [(\"RATMAC_TEST_H\".to_owned(), \"1\".to_owned())]\n}\n#[test]\nfn a() {\n    \
                 let pairs = hooks();\n    for (key, value) in &pairs {\n        \
                 std::env::set_var(key, value);\n    }\n}\n",
            )],
            &["unlaned: test/qa/tests/control.rs:8 a <- a"],
        ),
        (
            "a closure run by a callee holding the lane",
            &[(
                BIN,
                "use ratmac_qa::support;\nfn launch(body: impl FnOnce()) {\n    \
                 let _lane = support::hook_lane();\n    body();\n}\n#[test]\nfn a() {\n    \
                 launch(|| {\n        std::env::set_var(\"RATMAC_TEST_C\", \"1\");\n    });\n}\n",
            )],
            &[],
        ),
        (
            "an isolated name by constant, and a lane taken late or dropped at once",
            &[(
                BIN,
                "use ratmac_qa::support;\nconst CHILD: &str = \"a\";\n#[test]\nfn a() {\n    \
                 if support::isolated_test(CHILD) {\n        \
                 std::env::set_var(\"RATMAC_TEST_A\", \"1\");\n    }\n}\n#[test]\nfn b() {\n    \
                 std::env::set_var(\"RATMAC_TEST_B\", \"1\");\n    let _lane = support::hook_lane();\n}\n\
                 #[test]\nfn c() {\n    support::hook_lane();\n    \
                 std::env::set_var(\"RATMAC_TEST_C\", \"1\");\n}\n",
            )],
            &[
                "unlaned: test/qa/tests/control.rs:11 b <- b",
                "unlaned: test/qa/tests/control.rs:17 c <- c",
            ],
        ),
        (
            "a guard whose drop removes a setting, and names that must not link",
            &[(
                BIN,
                "use std::collections::BTreeMap;\nstruct Armed {\n    on: bool,\n}\n\
                 fn armed() -> Armed {\n    Armed { on: true }\n}\n\
                 impl Drop for Armed {\n    fn drop(&mut self) {\n        \
                 std::env::remove_var(\"RATMAC_TEST_E\");\n    }\n}\n\
                 fn entry() {\n    std::env::set_var(\"RATMAC_TEST_C\", \"1\");\n}\n\
                 struct Quiet;\nimpl Quiet {\n    fn run(&self) {}\n}\n\
                 struct Loud;\nimpl Loud {\n    fn run(&self) {\n        \
                 std::env::set_var(\"RATMAC_TEST_D\", \"1\");\n    }\n}\n\
                 #[test]\nfn a() {\n    let _armed = armed();\n    let mut map = BTreeMap::new();\n    \
                 map.entry(1).or_insert(2);\n    let quiet = Quiet;\n    quiet.run();\n}\n",
            )],
            &[
                "unlaned: test/qa/tests/control.rs:10 Armed::drop <- a",
                "unreached: test/qa/tests/control.rs:14 entry",
                "unreached: test/qa/tests/control.rs:23 Loud::run",
            ],
        ),
    ];
    for (name, files, expected) in controls {
        let mut sources: BTreeMap<String, String> = BTreeMap::new();
        sources.insert(HOOK_SUPPORT.to_owned(), SUPPORT.to_owned());
        sources.insert("test/qa/src/lib.rs".to_owned(), LIB.to_owned());
        for (path, text) in *files {
            sources.insert((*path).to_owned(), (*text).to_owned());
        }
        let report = hook_report(&sources, "lanes");
        let expected: Vec<String> = expected.iter().map(|line| (*line).to_owned()).collect();
        assert_eq!(
            report.findings, expected,
            "SVEV-018 control ({name}): the hook scan misread these sources:\n{files:#?}"
        );
    }
}

/// Known clean places of this repository, each by its file, the function or
/// line text that marks it, and how every case reaching it must run it.
const HOOK_WITNESSES: [(&str, &str, &str, HookScope); 6] = [
    (
        "test/qa/tests/t129_authorized_review_transitions.rs",
        "fault_step",
        "StepFault::set",
        HookScope::Lane,
    ),
    (
        "test/qa/tests/t132_rule_fix_recovery.rs",
        "",
        "RATMAC_TEST_RECOVER_FAULT",
        HookScope::Lane,
    ),
    (
        "/t-119/tests/hidden.rs",
        "answered",
        "",
        HookScope::Isolated,
    ),
    (
        "/t-132/tests/hidden.rs",
        "kill_child_role",
        "",
        HookScope::Exact,
    ),
    (
        "/t-062/tests/hidden.rs",
        "",
        "StepFault::set",
        HookScope::Lane,
    ),
    (
        "/t-069/tests/hidden.rs",
        "",
        "StepFault::set",
        HookScope::Lane,
    ),
];

/// The scan on this repository's known clean places: each is found, every
/// case reaches it only the way it is known to, and no finding names it.
fn hook_witnesses(report: &HookReport, sources: &BTreeMap<String, String>, folder: &str) {
    for (file, function, text, scope) in HOOK_WITNESSES {
        let path = if file.starts_with('/') {
            format!("{folder}{file}")
        } else {
            file.to_owned()
        };
        let lines: Vec<&str> = sources
            .get(&path)
            .unwrap_or_else(|| panic!("SVEV-018 witness: {path} is inventoried"))
            .lines()
            .collect();
        let matched: Vec<(&HookSite, &BTreeSet<(String, HookScope)>)> = report
            .sites
            .iter()
            .filter(|((site, line, label), _)| {
                *site == path
                    && (function.is_empty()
                        || label == function
                        || label.ends_with(&format!("::{function}")))
                    && lines
                        .get(line - 1)
                        .is_some_and(|source| source.contains(text))
            })
            .collect();
        assert!(
            !matched.is_empty(),
            "SVEV-018 witness: the scan must reach the hook places of {path} {function} {text:?}"
        );
        for ((site, line, label), roots) in matched {
            assert!(
                roots.iter().all(|(_, found)| *found == scope),
                "SVEV-018 witness: {site}:{line} {label} runs only under {}, yet the scan saw {roots:?}",
                scope.label()
            );
            let named = format!(" {site}:{line} ");
            assert!(
                !report
                    .findings
                    .iter()
                    .any(|finding| finding.contains(&named)),
                "SVEV-018 witness: {site}:{line} {label} is clean, yet a finding names it"
            );
        }
    }
}

/// The SVEV-018 scan of `sources` (keyed by `/`-separated path), whose lanes
/// sit under `folder`.
fn hook_report(sources: &BTreeMap<String, String>, folder: &str) -> HookReport {
    let parsed: BTreeMap<String, HookSource> = sources
        .iter()
        .map(|(path, text)| (path.clone(), hook_source(text)))
        .collect();
    let units = hook_units(&parsed, folder);
    let index = HookIndex::new(&parsed, units);
    index.report()
}

/// The parsed shape of one Rust source.
fn hook_source(source: &str) -> HookSource {
    let tokens = rust_tokens(source);
    let mut uses = BTreeMap::new();
    let mut globs = Vec::new();
    let mut consts = BTreeMap::new();
    let mut mods = Vec::new();
    let mut items: Vec<HookItem> = Vec::new();
    let mut owners: Vec<(usize, usize, String)> = Vec::new();
    for (at, (_, token)) in tokens.iter().enumerate() {
        let Token::Word(word) = token else {
            continue;
        };
        match word.as_str() {
            "use" => {
                let mut cursor = at + 1;
                hook_use(&tokens, &mut cursor, Vec::new(), &mut uses, &mut globs);
            }
            "const" | "static" => {
                let item_position = at == 0
                    || matches!(tokens[at - 1].1, Token::Mark('}' | ';' | ']' | '{' | ')'))
                    || matches!(&tokens[at - 1].1, Token::Word(before) if before == "pub");
                if !item_position {
                    continue;
                }
                let mut cursor = at + 1;
                if hook_word(&tokens, cursor) == Some("mut") {
                    cursor += 1;
                }
                let Some(name) = hook_word(&tokens, cursor) else {
                    continue;
                };
                if name == "fn" {
                    continue;
                }
                let end = (cursor..tokens.len())
                    .find(|index| hook_mark(&tokens, *index, ';'))
                    .unwrap_or(tokens.len());
                if let Some(equals) = (cursor..end).find(|index| hook_mark(&tokens, *index, '=')) {
                    if let [(_, Token::Text(value))] = &tokens[equals + 1..end] {
                        consts.insert(name.to_owned(), value.clone());
                    }
                }
            }
            "mod" => {
                if let (Some(name), true) =
                    (hook_word(&tokens, at + 1), hook_mark(&tokens, at + 2, ';'))
                {
                    let path = hook_attributes(&tokens, at)
                        .into_iter()
                        .find_map(|attribute| match attribute {
                            [(_, Token::Word(key)), (_, Token::Mark('=')), (_, Token::Text(path))]
                                if key == "path" =>
                            {
                                Some(path.clone())
                            }
                            _ => None,
                        });
                    mods.push((name.to_owned(), path));
                }
            }
            "impl" | "trait" => {
                let item_position = at == 0
                    || matches!(tokens[at - 1].1, Token::Mark('}' | ';' | ']' | '{'))
                    || matches!(&tokens[at - 1].1, Token::Word(before) if before == "unsafe" || before == "pub");
                if !item_position {
                    continue;
                }
                let name = if word == "impl" {
                    impl_name(&tokens[at + 1..])
                } else {
                    match hook_word(&tokens, at + 1) {
                        Some(name) => name.to_owned(),
                        None => continue,
                    }
                };
                let open = (at + 1..tokens.len())
                    .take_while(|index| !hook_mark(&tokens, *index, ';'))
                    .find(|index| hook_mark(&tokens, *index, '{'));
                if let Some(open) = open {
                    owners.push((open, hook_closing(&tokens, open), name));
                }
            }
            "fn" => {
                if let Some(item) = hook_item(&tokens, at) {
                    items.push(item);
                }
            }
            _ => {}
        }
    }
    let mut within = vec![None; tokens.len()];
    for (index, item) in items.iter().enumerate() {
        if let Some((open, close)) = item.body {
            for slot in &mut within[open..=close] {
                *slot = Some(index);
            }
        }
    }
    for item in &mut items {
        // A function nested in another function's body belongs to no type.
        if within[item.at].is_none() {
            item.owner = owners
                .iter()
                .filter(|(open, close, _)| *open < item.at && item.at < *close)
                .max_by_key(|(open, _, _)| *open)
                .map(|(_, _, name)| name.clone());
        }
    }
    HookSource {
        tokens,
        uses,
        globs,
        consts,
        mods,
        items,
        within,
    }
}

/// The word token at `at`, if it is one.
fn hook_word(tokens: &[(usize, Token)], at: usize) -> Option<&str> {
    match tokens.get(at) {
        Some((_, Token::Word(word))) => Some(word.as_str()),
        _ => None,
    }
}

/// Whether the token at `at` is the mark `mark`.
fn hook_mark(tokens: &[(usize, Token)], at: usize, mark: char) -> bool {
    matches!(tokens.get(at), Some((_, Token::Mark(found))) if *found == mark)
}

/// Whether a name starts as a type does (an upper-case letter).
fn hook_type_like(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
}

/// The index of the bracket closing the one at `open` (or the last token).
fn hook_closing(tokens: &[(usize, Token)], open: usize) -> usize {
    let (opening, closing) = match tokens.get(open) {
        Some((_, Token::Mark('('))) => ('(', ')'),
        Some((_, Token::Mark('['))) => ('[', ']'),
        _ => ('{', '}'),
    };
    let mut depth = 0usize;
    for (index, (_, token)) in tokens.iter().enumerate().skip(open) {
        match token {
            Token::Mark(mark) if *mark == opening => depth += 1,
            Token::Mark(mark) if *mark == closing => {
                depth -= 1;
                if depth == 0 {
                    return index;
                }
            }
            _ => {}
        }
    }
    tokens.len().saturating_sub(1)
}

/// The contents of the outer attributes (`#[...]`) right before the item
/// whose keyword is at `at`, past any visibility and qualifiers.
fn hook_attributes(tokens: &[(usize, Token)], at: usize) -> Vec<&[(usize, Token)]> {
    let mut cursor = at;
    while cursor > 0 {
        match &tokens[cursor - 1].1 {
            Token::Word(word)
                if matches!(
                    word.as_str(),
                    "pub"
                        | "crate"
                        | "super"
                        | "self"
                        | "in"
                        | "async"
                        | "unsafe"
                        | "const"
                        | "extern"
                ) =>
            {
                cursor -= 1;
            }
            Token::Mark('(' | ')') | Token::Text(_) => cursor -= 1,
            _ => break,
        }
    }
    let mut attributes = Vec::new();
    while cursor > 1 && hook_mark(tokens, cursor - 1, ']') {
        let mut depth = 0usize;
        let mut open = None;
        for (index, (_, token)) in tokens[..cursor].iter().enumerate().rev() {
            match token {
                Token::Mark(']') => depth += 1,
                Token::Mark('[') => {
                    depth -= 1;
                    if depth == 0 {
                        open = Some(index);
                        break;
                    }
                }
                _ => {}
            }
        }
        match open {
            Some(open) if open > 0 && hook_mark(tokens, open - 1, '#') => {
                attributes.push(&tokens[open + 1..cursor - 1]);
                cursor = open - 1;
            }
            _ => break,
        }
    }
    attributes
}

/// One `use` tree from `at`, recording each name it brings in by its full
/// path and each glob by its prefix.
fn hook_use(
    tokens: &[(usize, Token)],
    at: &mut usize,
    prefix: Vec<String>,
    uses: &mut BTreeMap<String, Vec<String>>,
    globs: &mut Vec<Vec<String>>,
) {
    let mut path = prefix;
    while let Some((_, token)) = tokens.get(*at) {
        match token {
            Token::Mark(':') => *at += 1,
            Token::Mark('*') => {
                *at += 1;
                globs.push(path);
                return;
            }
            Token::Mark('{') => {
                *at += 1;
                while *at < tokens.len() && !hook_mark(tokens, *at, '}') {
                    let before = *at;
                    hook_use(tokens, at, path.clone(), uses, globs);
                    if hook_mark(tokens, *at, ',') {
                        *at += 1;
                    } else if *at == before {
                        break;
                    }
                }
                *at += 1;
                return;
            }
            Token::Word(word) if word == "as" => {
                if let Some(alias) = hook_word(tokens, *at + 1) {
                    uses.insert(alias.to_owned(), path);
                }
                *at += 2;
                return;
            }
            Token::Word(word) => {
                if !(word == "self" && !path.is_empty()) {
                    path.push(word.clone());
                }
                *at += 1;
            }
            _ => break,
        }
    }
    if let Some(last) = path.last() {
        uses.insert(last.clone(), path.clone());
    }
}

/// The function item whose `fn` keyword is at `at`.
fn hook_item(tokens: &[(usize, Token)], at: usize) -> Option<HookItem> {
    let name = hook_word(tokens, at + 1)?.to_owned();
    let mut cursor = at + 2;
    if hook_mark(tokens, cursor, '<') {
        let mut depth = 0usize;
        while let Some((_, token)) = tokens.get(cursor) {
            match token {
                Token::Mark('<') => depth += 1,
                Token::Mark('>') if !hook_mark(tokens, cursor - 1, '-') => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            cursor += 1;
        }
        cursor += 1;
    }
    if !hook_mark(tokens, cursor, '(') {
        return None;
    }
    let close = hook_closing(tokens, cursor);
    let mut params = Vec::new();
    let mut has_self = false;
    for param in hook_split(&tokens[cursor + 1..close]) {
        let colon = (0..param.len()).find(|index| {
            hook_mark(param, *index, ':')
                && !hook_mark(param, index + 1, ':')
                && !(*index > 0 && hook_mark(param, index - 1, ':'))
        });
        let pattern = &param[..colon.unwrap_or(param.len())];
        if pattern
            .iter()
            .any(|(_, token)| matches!(token, Token::Word(word) if word == "self"))
        {
            has_self = true;
            continue;
        }
        let name = pattern
            .iter()
            .find_map(|(_, token)| match token {
                Token::Word(word) if word != "mut" => Some(word.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let kind = colon.and_then(|colon| hook_type(&param[colon + 1..]));
        params.push((name, kind));
    }
    let mut body = None;
    let mut index = close + 1;
    let mut depth = 0usize;
    while let Some((_, token)) = tokens.get(index) {
        match token {
            Token::Mark('(' | '[') => depth += 1,
            Token::Mark(')' | ']') => depth = depth.saturating_sub(1),
            Token::Mark(';') if depth == 0 => break,
            Token::Mark('{') if depth == 0 => {
                body = Some((index, hook_closing(tokens, index)));
                break;
            }
            _ => {}
        }
        index += 1;
    }
    let test = hook_attributes(tokens, at)
        .iter()
        .any(|attribute| matches!(attribute, [(_, Token::Word(word))] if word == "test"));
    Some(HookItem {
        name,
        owner: None,
        test,
        at,
        params,
        has_self,
        body,
    })
}

/// The top-level comma-separated parts of an argument or parameter list.
fn hook_split(tokens: &[(usize, Token)]) -> Vec<&[(usize, Token)]> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut angle = 0usize;
    let mut start = 0;
    for (index, (_, token)) in tokens.iter().enumerate() {
        match token {
            Token::Mark('(' | '[' | '{') => depth += 1,
            Token::Mark(')' | ']' | '}') => depth = depth.saturating_sub(1),
            Token::Mark('<') => angle += 1,
            Token::Mark('>') if !(index > 0 && hook_mark(tokens, index - 1, '-')) => {
                angle = angle.saturating_sub(1);
            }
            Token::Mark(',') if depth == 0 && angle == 0 => {
                parts.push(&tokens[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < tokens.len() {
        parts.push(&tokens[start..]);
    }
    parts
}

/// The type a type expression names: its first type-like word.
fn hook_type(tokens: &[(usize, Token)]) -> Option<String> {
    tokens.iter().find_map(|(_, token)| match token {
        Token::Word(word) if hook_type_like(word) => Some(word.clone()),
        _ => None,
    })
}

/// The units this inventory compiles: the QA library and its test binaries,
/// and each lane's library and test binaries, every one with the files its
/// `mod` declarations reach.
fn hook_units(parsed: &BTreeMap<String, HookSource>, folder: &str) -> Vec<HookUnit> {
    let mut units = Vec::new();
    let qa = parsed.contains_key("test/qa/src/lib.rs").then(|| {
        units.push(HookUnit {
            library: Some("ratmac_qa".to_owned()),
            files: hook_files("test/qa/src/lib.rs", parsed),
            deps: Vec::new(),
        });
        units.len() - 1
    });
    let binary = |path: &str, prefix: &str| {
        path.strip_prefix(prefix)
            .is_some_and(|name| !name.contains('/') && name.ends_with(".rs"))
    };
    for path in parsed.keys().filter(|path| binary(path, "test/qa/tests/")) {
        units.push(HookUnit {
            library: None,
            files: hook_files(path, parsed),
            deps: qa.into_iter().collect(),
        });
    }
    let lanes: BTreeSet<&str> = parsed
        .keys()
        .filter_map(|path| {
            path.strip_prefix(folder)?
                .strip_prefix('/')?
                .split('/')
                .next()
        })
        .collect();
    for lane in lanes {
        let base = format!("{folder}/{lane}/");
        let library = format!("{base}src/lib.rs");
        let mut deps: Vec<usize> = qa.into_iter().collect();
        if parsed.contains_key(&library) {
            units.push(HookUnit {
                library: Some(format!("ratmac_{}_hidden", lane.replace('-', ""))),
                files: hook_files(&library, parsed),
                deps: deps.clone(),
            });
            deps.push(units.len() - 1);
        }
        for path in parsed
            .keys()
            .filter(|path| binary(path, &format!("{base}tests/")))
        {
            units.push(HookUnit {
                library: None,
                files: hook_files(path, parsed),
                deps: deps.clone(),
            });
        }
    }
    units
}

/// The files one crate root reaches through `mod` declarations (`#[path]`
/// included), each with its module name.
fn hook_files(root: &str, parsed: &BTreeMap<String, HookSource>) -> Vec<(String, String)> {
    let mut files = vec![(root.to_owned(), "crate".to_owned())];
    let mut at = 0;
    while let Some((path, _)) = files.get(at).cloned() {
        let folder = path
            .rsplit_once('/')
            .map_or("", |(folder, _)| folder)
            .to_owned();
        let own_folder = at == 0 || path.ends_with("/mod.rs");
        let stem = path
            .rsplit('/')
            .next()
            .and_then(|name| name.strip_suffix(".rs"))
            .unwrap_or_default()
            .to_owned();
        for (name, attribute) in parsed
            .get(&path)
            .map(|source| &source.mods)
            .into_iter()
            .flatten()
        {
            let candidates = match attribute {
                Some(relative) => vec![hook_normal(&format!("{folder}/{relative}"))],
                None => {
                    let base = if own_folder {
                        folder.clone()
                    } else {
                        format!("{folder}/{stem}")
                    };
                    vec![format!("{base}/{name}.rs"), format!("{base}/{name}/mod.rs")]
                }
            };
            if let Some(found) = candidates
                .into_iter()
                .find(|candidate| parsed.contains_key(candidate))
            {
                if !files.iter().any(|(file, _)| *file == found) {
                    files.push((found, name.clone()));
                }
            }
        }
        at += 1;
    }
    files
}

/// A `/`-separated path with `.` and `..` resolved.
fn hook_normal(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// What one function knows of its names: parameters, the types of its
/// bindings, and what each binding holds as a setting's name.
struct HookLocals<'a> {
    params: Vec<String>,
    types: BTreeMap<String, String>,
    names: BTreeMap<String, HookName>,
    consts: &'a BTreeMap<String, String>,
    /// The functions of this unit whose bodies name a `RATMAC_TEST_*` setting,
    /// directly or through another such function of their file: what they
    /// return may be a setting's name (`kind.hooks(...)`).
    mentions: &'a BTreeSet<String>,
}

impl HookLocals<'_> {
    /// What an argument reads as a setting's name.
    fn name(&self, argument: &[(usize, Token)]) -> HookName {
        let mut tokens = argument;
        while let [(_, Token::Mark('&' | '*')), rest @ ..] = tokens {
            tokens = rest;
        }
        if let [(_, Token::Word(word)), rest @ ..] = tokens {
            if word == "mut" {
                tokens = rest;
            }
        }
        while let [rest @ .., (_, Token::Mark('.')), (_, Token::Word(method)), (_, Token::Mark('(')), (_, Token::Mark(')'))] =
            tokens
        {
            if !NAME_CONVERSIONS.contains(&method.as_str()) {
                break;
            }
            tokens = rest;
        }
        let hook = |text: &str| {
            if text.to_ascii_uppercase().starts_with(HOOK_PREFIX) {
                HookName::Hook
            } else {
                HookName::Other
            }
        };
        if let [(_, Token::Text(text))] = tokens {
            return hook(text);
        }
        let path = !tokens.is_empty()
            && tokens.iter().enumerate().all(|(index, (_, token))| {
                if index % 3 == 0 {
                    matches!(token, Token::Word(_))
                } else {
                    matches!(token, Token::Mark(':'))
                }
            });
        if path {
            if let Some((_, Token::Word(last))) = tokens.last() {
                if let Some(index) = self.params.iter().position(|param| param == last) {
                    return HookName::Param(index);
                }
                if let Some(name) = self.names.get(last) {
                    return name.clone();
                }
                if let Some(value) = self.consts.get(last) {
                    return hook(value);
                }
            }
        }
        if tokens
            .iter()
            .any(|(_, token)| matches!(token, Token::Text(text) if hook(text) == HookName::Hook))
        {
            return HookName::Hook;
        }
        let returned = tokens.windows(2).any(|pair| match pair {
            [(_, Token::Word(called)), (_, Token::Mark('('))] => self.mentions.contains(called),
            _ => false,
        });
        if returned {
            HookName::Hook
        } else {
            HookName::Other
        }
    }
}

/// What one function body does: its setter calls, its resolved calls, the
/// `isolated_test` names it asks for, the closures it passes, and (for a
/// test) its child-role branches.
struct HookBody {
    direct: Vec<(usize, HookName)>,
    calls: Vec<HookCall>,
    isolated: Vec<String>,
    closures: Vec<(usize, usize, usize)>,
    children: Vec<(usize, usize)>,
}

/// The functions of one file whose bodies name a `RATMAC_TEST_*` setting (by
/// literal or constant), closed over calls to each other inside the file.
fn hook_mentions(source: &HookSource, consts: &BTreeMap<String, String>) -> BTreeSet<String> {
    let hook = |text: &str| text.to_ascii_uppercase().starts_with(HOOK_PREFIX);
    let bodies: Vec<(&str, Vec<&Token>)> = source
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.test)
        .filter_map(|(index, item)| {
            let (open, close) = item.body?;
            let body = (open + 1..close)
                .filter(|at| source.within[*at] == Some(index))
                .map(|at| &source.tokens[at].1)
                .collect();
            Some((item.name.as_str(), body))
        })
        .collect();
    let mut found: BTreeSet<String> = bodies
        .iter()
        .filter(|(_, body)| {
            body.iter().any(|token| match token {
                Token::Text(text) => hook(text),
                Token::Word(word) => consts.get(word).is_some_and(|value| hook(value)),
                _ => false,
            })
        })
        .map(|(name, _)| (*name).to_owned())
        .collect();
    loop {
        let more: Vec<String> = bodies
            .iter()
            .filter(|(name, body)| {
                !found.contains(*name)
                    && body.windows(2).any(|pair| {
                        matches!(pair, [Token::Word(called), Token::Mark('(')] if found.contains(called))
                    })
            })
            .map(|(name, _)| (*name).to_owned())
            .collect();
        if more.is_empty() {
            return found;
        }
        found.extend(more);
    }
}

/// Whether the call from `start` (its first path segment) to `end` (its
/// closing parenthesis) is kept: the whole value of a `let` naming a binding
/// other than `_`, or the tail of the body closing at `close`.
fn hook_bound(tokens: &[(usize, Token)], start: usize, end: usize, close: usize) -> bool {
    let statement = start == 0 || matches!(tokens[start - 1].1, Token::Mark('{' | ';' | '}'));
    if statement && end + 1 == close {
        return true;
    }
    if !hook_mark(tokens, end + 1, ';') || start < 3 || !hook_mark(tokens, start - 1, '=') {
        return false;
    }
    let mut cursor = start - 1;
    while cursor > 0 {
        cursor -= 1;
        match &tokens[cursor].1 {
            Token::Word(word) if word == "let" => {
                let mut name = cursor + 1;
                if hook_word(tokens, name) == Some("mut") {
                    name += 1;
                }
                return hook_word(tokens, name).is_some_and(|name| name != "_");
            }
            Token::Mark('{' | ';' | '}') => return false,
            _ => {}
        }
    }
    false
}

/// The child-role branch of a test at the `if` or `match` at `at`: a block
/// whose condition reads the environment (`var`, `var_os`) and that ends the
/// case (`return`, `exit`) or, for an `if`, has an `else`; by its lines.
fn hook_child_branch(
    tokens: &[(usize, Token)],
    at: usize,
    close: usize,
) -> Option<Vec<(usize, usize)>> {
    let mut depth = 0usize;
    let mut brace = None;
    for (index, (_, token)) in tokens.iter().enumerate().take(close).skip(at + 1) {
        match token {
            Token::Mark('(' | '[') => depth += 1,
            Token::Mark(')' | ']') => depth = depth.saturating_sub(1),
            Token::Mark('{') if depth == 0 => {
                brace = Some(index);
                break;
            }
            Token::Mark(';' | '}') if depth == 0 => return None,
            _ => {}
        }
    }
    let brace = brace?;
    let reads = tokens[at + 1..brace].windows(2).any(|pair| {
        matches!(pair, [(_, Token::Word(read)), (_, Token::Mark('('))] if read == "var" || read == "var_os")
    });
    if !reads {
        return None;
    }
    let end = hook_closing(tokens, brace);
    let ends = |from: usize, to: usize| {
        tokens[from..to].iter().any(
            |(_, token)| matches!(token, Token::Word(word) if word == "return" || word == "exit"),
        )
    };
    if hook_word(tokens, at) == Some("if") {
        return (ends(brace, end) || hook_word(tokens, end + 1) == Some("else"))
            .then(|| vec![(tokens[brace].0, tokens[end].0)]);
    }
    let mut regions = Vec::new();
    let mut index = brace + 1;
    while index < end {
        if hook_mark(tokens, index, '=')
            && hook_mark(tokens, index + 1, '>')
            && hook_mark(tokens, index + 2, '{')
        {
            let arm = hook_closing(tokens, index + 2);
            if ends(index + 2, arm) {
                regions.push((tokens[index + 2].0, tokens[arm].0));
            }
            index = arm;
        }
        index += 1;
    }
    (!regions.is_empty()).then_some(regions)
}

/// Every unit's functions with their calls resolved.
struct HookIndex<'a> {
    parsed: &'a BTreeMap<String, HookSource>,
    units: Vec<HookUnit>,
    nodes: Vec<HookNode>,
    free: BTreeMap<(usize, String), Vec<usize>>,
    methods: BTreeMap<(usize, String, String), Vec<usize>>,
    owners: BTreeMap<(usize, String), BTreeSet<String>>,
    libraries: BTreeMap<String, usize>,
    consts: Vec<BTreeMap<String, String>>,
    mentions: Vec<BTreeSet<String>>,
}

impl<'a> HookIndex<'a> {
    fn new(parsed: &'a BTreeMap<String, HookSource>, units: Vec<HookUnit>) -> Self {
        let mut index = HookIndex {
            parsed,
            units: Vec::new(),
            nodes: Vec::new(),
            free: BTreeMap::new(),
            methods: BTreeMap::new(),
            owners: BTreeMap::new(),
            libraries: BTreeMap::new(),
            consts: Vec::new(),
            mentions: Vec::new(),
        };
        for (unit, crate_unit) in units.iter().enumerate() {
            if let Some(name) = &crate_unit.library {
                index.libraries.insert(name.clone(), unit);
            }
            let mut consts = BTreeMap::new();
            for (file, _) in crate_unit.files.iter().chain(
                crate_unit
                    .deps
                    .iter()
                    .flat_map(|dep| units[*dep].files.iter()),
            ) {
                for (name, value) in &parsed[file].consts {
                    consts.entry(name.clone()).or_insert_with(|| value.clone());
                }
            }
            index.consts.push(consts);
            let mentions = crate_unit
                .files
                .iter()
                .flat_map(|(file, _)| hook_mentions(&parsed[file], &index.consts[unit]))
                .collect();
            index.mentions.push(mentions);
            for (file, module) in &crate_unit.files {
                for (item, function) in parsed[file].items.iter().enumerate() {
                    if function.body.is_none() {
                        continue;
                    }
                    let id = index.nodes.len();
                    let label = match &function.owner {
                        Some(owner) => format!("{owner}::{}", function.name),
                        None => function.name.clone(),
                    };
                    let exempt = file == HOOK_SUPPORT
                        && match function.owner.as_deref() {
                            None => function.name == "hook_lane",
                            Some(owner) => owner == "HookLane" || owner == "StepFault",
                        };
                    match &function.owner {
                        Some(owner) => {
                            index
                                .methods
                                .entry((unit, owner.clone(), function.name.clone()))
                                .or_default()
                                .push(id);
                            index
                                .owners
                                .entry((unit, function.name.clone()))
                                .or_default()
                                .insert(owner.clone());
                        }
                        None => index
                            .free
                            .entry((unit, function.name.clone()))
                            .or_default()
                            .push(id),
                    }
                    index.nodes.push(HookNode {
                        unit,
                        file: file.clone(),
                        module: module.clone(),
                        item,
                        label,
                        exempt,
                        direct: Vec::new(),
                        calls: Vec::new(),
                        isolated: Vec::new(),
                        closures: Vec::new(),
                        children: Vec::new(),
                        taker: false,
                        wraps: BTreeSet::new(),
                        sites: Vec::new(),
                    });
                }
            }
        }
        index.units = units;
        for id in 0..index.nodes.len() {
            let body = index.scan(id);
            let node = &mut index.nodes[id];
            node.direct = body.direct;
            node.calls = body.calls;
            node.isolated = body.isolated;
            node.closures = body.closures;
            node.children = body.children;
        }
        index.settle();
        index
    }

    fn item(&self, id: usize) -> &HookItem {
        let node = &self.nodes[id];
        &self.parsed[&node.file].items[node.item]
    }

    /// The units a call from `unit` may reach: itself, then its libraries.
    fn reach(&self, unit: usize) -> impl Iterator<Item = usize> + '_ {
        std::iter::once(unit).chain(self.units[unit].deps.iter().copied())
    }

    /// The library unit a crate name stands for, seen from `unit`.
    fn library(&self, unit: usize, name: &str) -> Option<usize> {
        if matches!(name, "crate" | "self" | "super") {
            return Some(unit);
        }
        let found = *self.libraries.get(name)?;
        self.reach(unit)
            .any(|reached| reached == found)
            .then_some(found)
    }

    /// The methods `owner::name` the first reachable unit defines.
    fn methods_of(&self, unit: usize, owner: &str, name: &str) -> Vec<usize> {
        self.reach(unit)
            .find_map(|reached| {
                self.methods
                    .get(&(reached, owner.to_owned(), name.to_owned()))
                    .filter(|found| !found.is_empty())
                    .cloned()
            })
            .unwrap_or_default()
    }

    /// The free functions `name` of `unit`, those of module `module` first.
    fn free_of(&self, unit: usize, name: &str, module: Option<&str>) -> Vec<usize> {
        let all = self
            .free
            .get(&(unit, name.to_owned()))
            .cloned()
            .unwrap_or_default();
        let preferred: Vec<usize> = all
            .iter()
            .copied()
            .filter(|id| Some(self.nodes[*id].module.as_str()) == module)
            .collect();
        if preferred.is_empty() {
            all
        } else {
            preferred
        }
    }

    /// Whether a full path names the Engine library; every entry counts.
    fn engine(full: &[String]) -> bool {
        full.first().is_some_and(|head| head == "ratmac")
    }

    /// The targets of a full path (`crate::module::function`,
    /// `library::Type::method`), and whether it is an Engine entry.
    fn resolve_full(&self, unit: usize, full: &[String]) -> (Vec<usize>, bool) {
        self.resolve_through(unit, full, 0)
    }

    /// [`Self::resolve_full`], following a library's own re-exports (`pub use
    /// ratmac_qa::support;` in a lane library) a bounded number of times.
    fn resolve_through(&self, unit: usize, full: &[String], depth: usize) -> (Vec<usize>, bool) {
        if Self::engine(full) {
            return (Vec::new(), true);
        }
        let (Some(head), Some(name)) = (full.first(), full.last()) else {
            return (Vec::new(), false);
        };
        let Some(target) = self.library(unit, head) else {
            return (Vec::new(), false);
        };
        let before = full.len().checked_sub(2).map(|index| full[index].as_str());
        let found = match before {
            Some(owner) if hook_type_like(owner) => (self.methods_of(target, owner, name), false),
            _ => (self.free_of(target, name, before), false),
        };
        if !found.0.is_empty() || depth > 4 || target == unit || full.len() < 2 {
            return found;
        }
        // The library's root re-exports the second segment under that name.
        let root = &self.units[target].files[0].0;
        let Some(exported) = self.parsed[root].uses.get(&full[1]) else {
            return found;
        };
        if exported
            .first()
            .is_some_and(|head| matches!(head.as_str(), "self" | "super"))
        {
            return found;
        }
        let mut rewritten = exported.clone();
        if rewritten.first().is_some_and(|head| head == "crate") {
            rewritten[0] = full[0].clone();
        }
        rewritten.extend(full[2..].iter().cloned());
        if rewritten == full {
            return found;
        }
        self.resolve_through(target, &rewritten, depth + 1)
    }

    /// The parsed calls, setter calls, and `isolated_test` names in a body.
    fn scan(&self, id: usize) -> HookBody {
        let node = &self.nodes[id];
        let source = &self.parsed[&node.file];
        let function = self.item(id);
        let tokens = &source.tokens;
        let mut body = HookBody {
            direct: Vec::new(),
            calls: Vec::new(),
            isolated: Vec::new(),
            closures: Vec::new(),
            children: Vec::new(),
        };
        let Some((open, close)) = function.body else {
            return body;
        };
        let owner = function.owner.clone();
        let mut locals = HookLocals {
            params: function
                .params
                .iter()
                .map(|(name, _)| name.clone())
                .collect(),
            types: function
                .params
                .iter()
                .filter_map(|(name, kind)| Some((name.clone(), kind.clone()?)))
                .collect(),
            names: BTreeMap::new(),
            consts: &self.consts[node.unit],
            mentions: &self.mentions[node.unit],
        };
        if let Some(owner) = &owner {
            locals.types.insert("self".to_owned(), owner.clone());
        }
        let HookBody {
            direct,
            calls,
            isolated,
            closures,
            children,
        } = &mut body;
        for at in open + 1..close {
            if source.within[at] != Some(node.item) {
                continue;
            }
            let (line, token) = &tokens[at];
            let Token::Word(word) = token else {
                continue;
            };
            let previous = at.checked_sub(1).map(|before| &tokens[before].1);
            if word == "let" {
                Self::bind(tokens, at, owner.as_deref(), &mut locals);
                continue;
            }
            if word == "for" {
                // `for key in [...]` or `for (key, value) in &pairs`: the
                // pattern's first name holds a setting's name when the
                // iterated value does.
                let within = (at + 1..close).find(|index| hook_word(tokens, *index) == Some("in"));
                if let Some(within) = within {
                    let end = (within + 1..close)
                        .find(|index| hook_mark(tokens, *index, '{'))
                        .unwrap_or(close);
                    let held = locals.name(&tokens[within + 1..end]);
                    let names: Vec<&str> = (at + 1..within)
                        .filter_map(|index| hook_word(tokens, index))
                        .filter(|name| !matches!(*name, "mut" | "ref" | "_"))
                        .collect();
                    for (index, name) in names.iter().enumerate() {
                        let name_held = if index == 0 {
                            held.clone()
                        } else {
                            HookName::Other
                        };
                        locals.names.insert((*name).to_owned(), name_held);
                    }
                }
                continue;
            }
            if function.test && (word == "if" || word == "match") {
                if let Some(region) = hook_child_branch(tokens, at, close) {
                    children.extend(region);
                }
            }
            if hook_type_like(word) {
                // A value of a type built here drops here: its `Drop` runs.
                let built = (hook_mark(tokens, at + 1, '{') || hook_mark(tokens, at + 1, '('))
                    && !matches!(previous, Some(Token::Mark('>' | ':')))
                    && !matches!(previous, Some(Token::Word(before)) if matches!(before.as_str(), "impl" | "for" | "struct" | "enum" | "trait" | "union" | "in" | "fn"));
                if built {
                    let kind = if word == "Self" {
                        owner.clone()
                    } else {
                        Some(word.clone())
                    };
                    if let Some(kind) = kind {
                        let targets = self.methods_of(node.unit, &kind, "drop");
                        if !targets.is_empty() {
                            calls.push(HookCall {
                                line: *line,
                                targets,
                                args: Vec::new(),
                                receiver_first: false,
                                step_fault: false,
                                engine: false,
                                bound: false,
                            });
                        }
                    }
                }
                continue;
            }
            // A function named as a value (`[("case", recover_case)]`,
            // `.map(answer)`) runs wherever that value is called.
            let value_position = matches!(previous, Some(Token::Mark('(' | ',' | '[' | '=')))
                && matches!(
                    tokens.get(at + 1),
                    Some((_, Token::Mark(',' | ')' | ']' | ';')))
                );
            if value_position
                && !locals.params.contains(word)
                && !locals.types.contains_key(word)
                && !locals.names.contains_key(word)
            {
                let (targets, engine) = self.resolve_free(id, word);
                if !targets.is_empty() || engine {
                    calls.push(HookCall {
                        line: *line,
                        targets,
                        args: Vec::new(),
                        receiver_first: false,
                        step_fault: false,
                        engine,
                        bound: false,
                    });
                }
                continue;
            }
            if !hook_mark(tokens, at + 1, '(')
                || NOT_CALLS.contains(&word.as_str())
                || matches!(previous, Some(Token::Word(before)) if before == "fn")
            {
                continue;
            }
            let end = hook_closing(tokens, at + 1);
            let parts = hook_split(&tokens[at + 2..end]);
            let args: Vec<HookName> = parts.iter().map(|part| locals.name(part)).collect();
            let method = matches!(previous, Some(Token::Mark('.')));
            let path = !method
                && hook_mark(tokens, at.wrapping_sub(1), ':')
                && hook_mark(tokens, at.wrapping_sub(2), ':');
            let mut segments: Vec<String> = Vec::new();
            if path {
                let mut cursor = at;
                while cursor >= 3
                    && hook_mark(tokens, cursor - 1, ':')
                    && hook_mark(tokens, cursor - 2, ':')
                {
                    match hook_word(tokens, cursor - 3) {
                        Some(segment) => segments.push(segment.to_owned()),
                        None => {
                            segments.clear();
                            break;
                        }
                    }
                    cursor -= 3;
                }
                segments.reverse();
            }
            if !method && (word == "set_var" || word == "remove_var") {
                direct.push((*line, args.first().cloned().unwrap_or(HookName::Other)));
                continue;
            }
            if word == "isolated_test" {
                match parts.first() {
                    Some([(_, Token::Text(name))]) => isolated.push(name.clone()),
                    Some([(_, Token::Word(name))]) => {
                        if let Some(value) = locals.consts.get(name) {
                            isolated.push(value.clone());
                        }
                    }
                    _ => {}
                }
            }
            let step_fault =
                word == "set" && segments.last().is_some_and(|last| last == "StepFault");
            let (targets, engine) = if method {
                // A call result's method (`OpenOptions::new().create(true)`)
                // names no receiver: it links nothing.
                let named = matches!(
                    at.checked_sub(2).map(|before| &tokens[before].1),
                    Some(Token::Word(_))
                );
                let receiver = match (
                    at.checked_sub(2).map(|before| &tokens[before].1),
                    at.checked_sub(3),
                ) {
                    (Some(Token::Word(name)), before)
                        if !before.is_some_and(|before| hook_mark(tokens, before, '.')) =>
                    {
                        locals.types.get(name).cloned()
                    }
                    _ => None,
                };
                if named {
                    self.resolve_method(id, receiver.as_deref(), word)
                } else {
                    (Vec::new(), false)
                }
            } else if path {
                self.resolve_path(id, &segments, word)
            } else {
                self.resolve_free(id, word)
            };
            if targets.is_empty() && !engine && !step_fault {
                continue;
            }
            let bound = !method && hook_bound(tokens, at - 3 * segments.len(), end, close);
            for part in &parts {
                let closure = matches!(part.first(), Some((_, Token::Mark('|'))))
                    || (matches!(part.first(), Some((_, Token::Word(word))) if word == "move")
                        && matches!(part.get(1), Some((_, Token::Mark('|')))));
                if let (true, Some((first, _)), Some((last, _))) =
                    (closure, part.first(), part.last())
                {
                    closures.push((*first, *last, calls.len()));
                }
            }
            calls.push(HookCall {
                line: *line,
                targets,
                args,
                receiver_first: path,
                step_fault,
                engine,
                bound,
            });
        }
        body
    }

    /// Records a `let` binding's type and what it holds as a setting's name.
    fn bind(
        tokens: &[(usize, Token)],
        at: usize,
        owner: Option<&str>,
        locals: &mut HookLocals<'_>,
    ) {
        let mut cursor = at + 1;
        if hook_word(tokens, cursor) == Some("mut") {
            cursor += 1;
        }
        let Some(name) = hook_word(tokens, cursor) else {
            return;
        };
        let mut depth = 0usize;
        let mut equals = None;
        let mut end = tokens.len();
        for (index, (_, token)) in tokens.iter().enumerate().skip(cursor + 1) {
            match token {
                Token::Mark('(' | '[' | '{') => depth += 1,
                Token::Mark(')' | ']' | '}') => {
                    if depth == 0 {
                        end = index;
                        break;
                    }
                    depth -= 1;
                }
                // The first `=` of a `let` is its binding's.
                Token::Mark('=') if depth == 0 && equals.is_none() => {
                    equals = Some(index);
                }
                Token::Mark(';') if depth == 0 => {
                    end = index;
                    break;
                }
                _ => {}
            }
        }
        let annotated = hook_mark(tokens, cursor + 1, ':')
            .then(|| hook_type(&tokens[cursor + 2..equals.unwrap_or(end)]))
            .flatten();
        let Some(equals) = equals else {
            if let Some(kind) = annotated {
                locals.types.insert(name.to_owned(), kind);
            }
            return;
        };
        let value = &tokens[equals + 1..end];
        // The first type a leading path names: `Type::new()`,
        // `module::Type::new()`, `Self { .. }`.
        let mut cursor = value
            .iter()
            .position(|(_, token)| {
                !matches!(token, Token::Mark('&'))
                    && !matches!(token, Token::Word(word) if word == "mut")
            })
            .unwrap_or(value.len());
        let mut built = None;
        while let Some((_, Token::Word(word))) = value.get(cursor) {
            if word == "Self" {
                built = owner.map(str::to_owned);
                break;
            }
            if hook_type_like(word) {
                built = Some(word.clone());
                break;
            }
            if !hook_mark(value, cursor + 1, ':') || !hook_mark(value, cursor + 2, ':') {
                break;
            }
            cursor += 3;
        }
        match annotated.or(built) {
            Some(kind) => {
                locals.types.insert(name.to_owned(), kind);
            }
            None => {
                locals.types.remove(name);
            }
        }
        let held = locals.name(value);
        locals.names.insert(name.to_owned(), held);
    }

    fn resolve_free(&self, id: usize, name: &str) -> (Vec<usize>, bool) {
        let node = &self.nodes[id];
        let source = &self.parsed[&node.file];
        if let Some(local) = self.free.get(&(node.unit, name.to_owned())) {
            let same: Vec<usize> = local
                .iter()
                .copied()
                .filter(|other| self.nodes[*other].file == node.file)
                .collect();
            return (if same.is_empty() { local.clone() } else { same }, false);
        }
        if let Some(full) = source.uses.get(name) {
            return self.resolve_full(node.unit, full);
        }
        for glob in &source.globs {
            let mut full = glob.clone();
            full.push(name.to_owned());
            let found = self.resolve_full(node.unit, &full);
            if found.1 || !found.0.is_empty() {
                return found;
            }
        }
        (Vec::new(), false)
    }

    fn resolve_path(&self, id: usize, segments: &[String], name: &str) -> (Vec<usize>, bool) {
        let node = &self.nodes[id];
        let source = &self.parsed[&node.file];
        let Some(head) = segments.first() else {
            return (Vec::new(), false);
        };
        if head == "Self" {
            return match &self.item(id).owner {
                Some(owner) => (self.methods_of(node.unit, owner, name), false),
                None => (Vec::new(), false),
            };
        }
        let mut full: Vec<String> = match source.uses.get(head) {
            Some(used) if !matches!(head.as_str(), "crate" | "self" | "super") => {
                used.iter().chain(&segments[1..]).cloned().collect()
            }
            _ => segments.to_vec(),
        };
        full.push(name.to_owned());
        if Self::engine(&full) {
            return (Vec::new(), true);
        }
        if self.library(node.unit, &full[0]).is_some() {
            return self.resolve_full(node.unit, &full);
        }
        let before = &segments[segments.len() - 1];
        if hook_type_like(before) {
            return (self.methods_of(node.unit, before, name), false);
        }
        let module = self.units[node.unit]
            .files
            .iter()
            .any(|(_, module)| module == head);
        if segments.len() == 1 && module {
            return (self.free_of(node.unit, name, Some(head)), false);
        }
        (Vec::new(), false)
    }

    fn resolve_method(&self, id: usize, receiver: Option<&str>, name: &str) -> (Vec<usize>, bool) {
        let node = &self.nodes[id];
        let source = &self.parsed[&node.file];
        match receiver {
            Some(kind) => {
                if let Some(full) = source.uses.get(kind) {
                    let mut full = full.clone();
                    full.push(name.to_owned());
                    if Self::engine(&full) {
                        return (Vec::new(), true);
                    }
                }
                (self.methods_of(node.unit, kind, name), false)
            }
            None => {
                // An unknown receiver (`gov.fx.run(...)`) reaches only a
                // method that exactly one type defines, in the nearest unit
                // defining any: this one, then its own lane library, then
                // the QA library. Never a free function.
                let nearest = std::iter::once(node.unit)
                    .chain(self.units[node.unit].deps.iter().rev().copied())
                    .find_map(|unit| Some((unit, self.owners.get(&(unit, name.to_owned()))?)));
                match nearest.map(|(unit, owners)| (unit, owners.iter().collect::<Vec<_>>())) {
                    Some((unit, owners)) if owners.len() == 1 => (
                        self.methods
                            .get(&(unit, owners[0].clone(), name.to_owned()))
                            .cloned()
                            .unwrap_or_default(),
                        false,
                    ),
                    _ => (Vec::new(), false),
                }
            }
        }
    }

    /// Lane takers, wrappers, and hook places, to a fixed point.
    fn settle(&mut self) {
        for id in 0..self.nodes.len() {
            let function = self.item(id);
            self.nodes[id].taker = self.nodes[id].file == HOOK_SUPPORT
                && function.owner.is_none()
                && function.name == "hook_lane";
        }
        loop {
            let mut changed = false;
            for id in 0..self.nodes.len() {
                if self.nodes[id].taker || self.nodes[id].exempt {
                    continue;
                }
                // A wrapper returns the shared lane: its body ends with a
                // call that reaches it (`fn lane_guard() -> ... { hook_lane() }`).
                let Some((_, close)) = self.item(id).body else {
                    continue;
                };
                let tokens = &self.parsed[&self.nodes[id].file].tokens;
                if close > 0 && hook_mark(tokens, close - 1, ')') && self.tail_call(id, close - 1) {
                    self.nodes[id].taker = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        for id in 0..self.nodes.len() {
            if self.nodes[id].exempt {
                continue;
            }
            let wraps: BTreeSet<usize> = self.nodes[id]
                .direct
                .iter()
                .filter_map(|(_, name)| match name {
                    HookName::Param(index) => Some(*index),
                    _ => None,
                })
                .collect();
            self.nodes[id].wraps = wraps;
        }
        loop {
            let mut changed = false;
            for id in 0..self.nodes.len() {
                if self.nodes[id].exempt {
                    continue;
                }
                let mut found = Vec::new();
                for call in &self.nodes[id].calls {
                    for target in &call.targets {
                        let shift = usize::from(call.receiver_first && self.item(*target).has_self);
                        for param in &self.nodes[*target].wraps {
                            if let Some(HookName::Param(index)) = call.args.get(param + shift) {
                                found.push(*index);
                            }
                        }
                    }
                }
                for index in found {
                    changed |= self.nodes[id].wraps.insert(index);
                }
            }
            if !changed {
                break;
            }
        }
        for id in 0..self.nodes.len() {
            if self.nodes[id].exempt {
                continue;
            }
            let node = &self.nodes[id];
            let mut sites: Vec<usize> = node
                .direct
                .iter()
                .filter(|(_, name)| *name == HookName::Hook)
                .map(|(line, _)| *line)
                .collect();
            for call in &node.calls {
                let wrapped = call.targets.iter().any(|target| {
                    let shift = usize::from(call.receiver_first && self.item(*target).has_self);
                    self.nodes[*target]
                        .wraps
                        .iter()
                        .any(|param| call.args.get(param + shift) == Some(&HookName::Hook))
                });
                if call.step_fault || wrapped {
                    sites.push(call.line);
                }
            }
            sites.sort_unstable();
            sites.dedup();
            self.nodes[id].sites = sites;
        }
    }

    /// Whether the call whose closing parenthesis is at `close` is the whole
    /// tail expression of the function's body, and reaches a lane taker.
    fn tail_call(&self, id: usize, close: usize) -> bool {
        let source = &self.parsed[&self.nodes[id].file];
        let tokens = &source.tokens;
        let mut depth = 0usize;
        let mut open = None;
        for (index, (_, token)) in tokens[..=close].iter().enumerate().rev() {
            match token {
                Token::Mark(')') => depth += 1,
                Token::Mark('(') => {
                    depth -= 1;
                    if depth == 0 {
                        open = Some(index);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(open) = open else {
            return false;
        };
        let Some(callee) = open.checked_sub(1) else {
            return false;
        };
        let mut start = callee;
        while start >= 3 && hook_mark(tokens, start - 1, ':') && hook_mark(tokens, start - 2, ':') {
            start -= 3;
        }
        let statement_start =
            start == 0 || matches!(tokens[start - 1].1, Token::Mark('{' | ';' | '}'));
        let line = tokens[callee].0;
        let name = hook_word(tokens, callee);
        statement_start
            && self.nodes[id].calls.iter().any(|call| {
                call.line == line
                    && call.targets.iter().any(|target| {
                        self.nodes[*target].taker && Some(self.item(*target).name.as_str()) == name
                    })
            })
    }

    /// Follows every test case of every unit to the hook places and Engine
    /// entries it reaches, and judges each binary.
    fn report(&self) -> HookReport {
        let mut sites: BTreeMap<HookSite, BTreeSet<(String, HookScope)>> = BTreeMap::new();
        let mut grouped: BTreeMap<(usize, HookSite), BTreeSet<String>> = BTreeMap::new();
        let site = |id: usize, line: usize| -> HookSite {
            let node = &self.nodes[id];
            (node.file.clone(), line, node.label.clone())
        };
        // The first line from which each function holds the lane: a kept
        // (`let _lane = ...`) call reaching a taker.
        let taken: Vec<Option<usize>> = self
            .nodes
            .iter()
            .map(|node| {
                node.calls
                    .iter()
                    .filter(|call| {
                        call.bound && call.targets.iter().any(|target| self.nodes[*target].taker)
                    })
                    .map(|call| call.line)
                    .min()
            })
            .collect();
        // Whether `line` of function `id` runs under the lane: held from an
        // earlier line, or inside a closure handed to a function holding it.
        let held = |id: usize, line: usize| {
            let node = &self.nodes[id];
            taken[id].is_some_and(|from| from <= line)
                || node.closures.iter().any(|(first, last, call)| {
                    (*first..=*last).contains(&line)
                        && node.calls[*call]
                            .targets
                            .iter()
                            .any(|target| taken[*target].is_some())
                })
        };
        for (unit, crate_unit) in self.units.iter().enumerate() {
            let exact: BTreeSet<&str> =
                crate_unit
                    .files
                    .iter()
                    .flat_map(|(file, _)| {
                        self.parsed[file].tokens.windows(3).filter_map(|window| match window {
                        [(_, Token::Text(flag)), (_, Token::Mark(',')), (_, Token::Text(name))]
                            if flag == "--exact" =>
                        {
                            Some(name.as_str())
                        }
                        _ => None,
                    })
                    })
                    .collect();
            let mut laned: BTreeMap<HookSite, BTreeSet<String>> = BTreeMap::new();
            let mut open_engine: BTreeMap<String, HookSite> = BTreeMap::new();
            for root in (0..self.nodes.len()).filter(|id| {
                self.nodes[*id].unit == unit && !self.nodes[*id].exempt && self.item(*id).test
            }) {
                let name = self.item(root).name.clone();
                // Only a named `--exact` case's child-role branches run in
                // its child; the rest of its body runs in the shared process.
                let child = |line: usize| {
                    exact.contains(name.as_str())
                        && self.nodes[root]
                            .children
                            .iter()
                            .any(|(first, last)| (*first..=*last).contains(&line))
                };
                let mut seen: BTreeSet<(usize, bool, bool)> = BTreeSet::new();
                let mut stack = vec![(root, false, false)];
                let mut hits: Vec<(usize, usize, bool, bool)> = Vec::new();
                let mut engine: Vec<(usize, usize, bool, bool)> = Vec::new();
                let mut isolated = false;
                while let Some((id, inherited, in_child)) = stack.pop() {
                    if !seen.insert((id, inherited, in_child)) {
                        continue;
                    }
                    let node = &self.nodes[id];
                    let scope = |line: usize| {
                        (
                            inherited || held(id, line),
                            in_child || (id == root && child(line)),
                        )
                    };
                    isolated |= node.isolated.contains(&name);
                    hits.extend(node.sites.iter().map(|line| {
                        let (here, in_child) = scope(*line);
                        (id, *line, here, in_child)
                    }));
                    for call in &node.calls {
                        let (here, in_child) = scope(call.line);
                        if call.engine {
                            engine.push((id, call.line, here, in_child));
                        }
                        stack.extend(call.targets.iter().map(|target| (*target, here, in_child)));
                    }
                }
                for (id, line, here, in_child) in hits {
                    let scope = if isolated {
                        HookScope::Isolated
                    } else if in_child {
                        HookScope::Exact
                    } else if here {
                        HookScope::Lane
                    } else {
                        HookScope::Open
                    };
                    sites
                        .entry(site(id, line))
                        .or_default()
                        .insert((name.clone(), scope));
                    match scope {
                        HookScope::Open => {
                            grouped
                                .entry((0, site(id, line)))
                                .or_default()
                                .insert(name.clone());
                        }
                        HookScope::Lane => {
                            laned
                                .entry(site(id, line))
                                .or_default()
                                .insert(name.clone());
                        }
                        _ => {}
                    }
                }
                if !isolated {
                    if let Some((id, line, _, _)) = engine
                        .iter()
                        .filter(|(_, _, here, in_child)| !here && !in_child)
                        .min_by_key(|(id, line, _, _)| site(*id, *line))
                    {
                        open_engine.insert(name.clone(), site(*id, *line));
                    }
                }
            }
            if !laned.is_empty() && !open_engine.is_empty() {
                for (place, roots) in laned {
                    grouped.entry((1, place)).or_default().extend(roots);
                }
                for (root, place) in open_engine {
                    grouped.entry((2, place)).or_default().insert(root);
                }
            }
        }
        let libraries: BTreeSet<&str> = self
            .units
            .iter()
            .filter(|crate_unit| crate_unit.library.is_some())
            .flat_map(|crate_unit| crate_unit.files.iter().map(|(file, _)| file.as_str()))
            .collect();
        for (id, node) in self.nodes.iter().enumerate() {
            if libraries.contains(node.file.as_str()) {
                continue;
            }
            for line in &node.sites {
                let place = site(id, *line);
                if !sites.contains_key(&place) {
                    grouped.entry((3, place)).or_default();
                }
            }
        }
        let findings = grouped
            .into_iter()
            .map(|((kind, (path, line, label)), roots)| {
                let roots: Vec<&str> = roots.iter().map(String::as_str).collect();
                match kind {
                    0 => format!("unlaned: {path}:{line} {label} <- {}", roots.join(", ")),
                    1 => format!(
                        "beside an unlaned Engine case: {path}:{line} {label} <- {}",
                        roots.join(", ")
                    ),
                    2 => format!(
                        "Engine case without the lane: {path}:{line} {label} <- {}",
                        roots.join(", ")
                    ),
                    _ => format!("unreached: {path}:{line} {label}"),
                }
            })
            .collect();
        HookReport { sites, findings }
    }
}

// --- SVEV-010 to SVEV-012: one helper hands out every scratch folder ---------

/// The clock value the SVE-004 checks pin: one no real request carries, so
/// requests that carry it share one tick by construction.
const PINNED_STAMP: u128 = 141;

/// How long one request for a scratch folder may take. A helper whose names
/// rest on the clock alone retries a taken name forever and never returns.
const DEADLINE: Duration = Duration::from_secs(30);

/// Set only in an isolated SVE-004 child: the folder its parent made and
/// handed it as its system temporary folder.
const SCRATCH_HOME: &str = "RATMAC_T126_SCRATCH_HOME";

/// SVEV-010 / SVE-004: two requests in one clock tick. With the shared
/// helper's clock pinned, two threads that a barrier releases together each
/// ask for a folder for one label; each receives a different folder that
/// exists, is empty, and was not there before the requests, all within one
/// deadline. A helper that drops the counter from its names hands the second
/// request the first one's name forever and fails at the deadline.
#[test]
fn svev_010() {
    const LABEL: &str = "svev-010";
    let temp = std::env::temp_dir();
    let prefix = pinned_prefix(LABEL);
    let before = entries_named(&temp, &prefix);
    let gate = Arc::new(Barrier::new(2));
    let (sender, receiver) = mpsc::channel();
    for request in 0..2 {
        let gate = Arc::clone(&gate);
        let sender = sender.clone();
        thread::spawn(move || {
            gate.wait();
            let _ = sender.send((request, TempTree::with_stamp(LABEL, PINNED_STAMP)));
        });
    }
    drop(sender);
    let deadline = Instant::now() + DEADLINE;
    let mut trees = Vec::new();
    while trees.len() < 2 {
        match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok((request, Ok(tree))) => trees.push((request, tree)),
            Ok((request, Err(error))) => panic!(
                "SVEV-010: request {request} for a folder with the clock pinned to \
                 {PINNED_STAMP} failed: {error}"
            ),
            Err(mpsc::RecvTimeoutError::Timeout) => panic!(
                "SVEV-010: {} of two requests in one pinned tick returned within {DEADLINE:?}; \
                 the other keeps retrying a taken name",
                trees.len()
            ),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                panic!("SVEV-010: a request panicked before it returned a folder")
            }
        }
    }
    assert_ne!(
        trees[0].1.path(),
        trees[1].1.path(),
        "SVEV-010: two requests in one pinned tick were handed the same folder"
    );
    for (request, tree) in &trees {
        assert_fresh(
            &format!("SVEV-010 request {request}"),
            tree.path(),
            &temp,
            &prefix,
            &before,
        );
    }
}

/// SVEV-011 / SVE-004: a taken name moves on. In an isolated child whose
/// system temporary folder is a new folder this test owns, a pinned control
/// request shows the counter's next value, and a folder holding a sentinel
/// file is planted at exactly the name the helper tries first. The request
/// returns a different, empty folder, and the planted folder keeps every
/// byte. A helper that adopts, empties, or deletes the planted folder, or
/// panics on the taken name, fails.
#[test]
fn svev_011() {
    match std::env::var_os(SCRATCH_HOME) {
        Some(home) => taken_name_child(Path::new(&home)),
        None => in_scratch_home("svev_011"),
    }
}

/// Run the case `name` again as an isolated child whose system temporary
/// folder (`TMP`, `TEMP`, `TMPDIR`) is a new, empty folder this process owns
/// and removes, so nothing the child plants reaches the real one.
fn in_scratch_home(name: &str) {
    let home = TempTree::new("t126-scratch-home")
        .unwrap_or_else(|error| panic!("{name}: make the child's temporary folder: {error}"));
    let path = home.path().as_os_str();
    let envs = [
        ("TMP", path),
        ("TEMP", path),
        ("TMPDIR", path),
        (SCRATCH_HOME, path),
    ];
    if let Err(output) = support::isolated_test(name, &envs) {
        panic!("{name}: the isolated child failed:\n{output}");
    }
}

/// The SVEV-011 body, inside the isolated child.
fn taken_name_child(home: &Path) {
    assert_eq!(
        support::isolated_test("svev_011", &[]),
        Ok(true),
        "SVEV-011: this child must be the isolated case its parent started"
    );
    let temp = std::env::temp_dir();
    assert_eq!(
        fs::canonicalize(&temp).ok(),
        fs::canonicalize(home).ok(),
        "SVEV-011 control: the child's system temporary folder {} is not the folder {} its \
         parent made",
        temp.display(),
        home.display()
    );
    let control = pinned_request("SVEV-011 control", "svev-011-control");
    let next = counter_of(control.path(), &pinned_prefix("svev-011-control")) + 1;
    let prefix = pinned_prefix("svev-011");
    let planted = home.join(format!("{prefix}{next}"));
    fs::create_dir(&planted)
        .unwrap_or_else(|error| panic!("SVEV-011: plant {}: {error}", planted.display()));
    fs::write(
        planted.join("sentinel.txt"),
        b"planted at the first name the helper tries\n",
    )
    .unwrap_or_else(|error| panic!("SVEV-011: plant the sentinel file: {error}"));
    let planted_bytes = tagged_tree(&planted);
    let before = entries_named(home, &prefix);
    let tree = pinned_request("SVEV-011", "svev-011");
    assert_ne!(
        tree.path(),
        planted.as_path(),
        "SVEV-011: the request was handed the planted folder"
    );
    assert_fresh("SVEV-011", tree.path(), &temp, &prefix, &before);
    assert!(
        counter_of(tree.path(), &prefix) > next,
        "SVEV-011: the request never tried the planted name"
    );
    assert_eq!(
        tagged_tree(&planted),
        planted_bytes,
        "SVEV-011: the request changed the planted folder at its first name"
    );
    drop(tree);
    assert_eq!(
        tagged_tree(&planted),
        planted_bytes,
        "SVEV-011: dropping the request's owner changed the planted folder"
    );
    drop(control);
}

/// The start of every name the shared helper tries for `label` in this
/// process with the clock pinned - `ratmac-<label>-<process id>-<stamp>-` -
/// which its counter completes.
fn pinned_prefix(label: &str) -> String {
    format!("ratmac-{label}-{}-{PINNED_STAMP}-", std::process::id())
}

/// The names of the entries of `folder` that start with `prefix`.
fn entries_named(folder: &Path, prefix: &str) -> BTreeSet<String> {
    fs::read_dir(folder)
        .unwrap_or_else(|error| panic!("list {}: {error}", folder.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("list {}: {error}", folder.display()))
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.starts_with(prefix))
        .collect()
}

/// One request for a `label` folder with the clock pinned, on its own thread,
/// failing once `DEADLINE` passes, or if the request fails or panics.
fn pinned_request(check: &str, label: &'static str) -> TempTree {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(TempTree::with_stamp(label, PINNED_STAMP));
    });
    match receiver.recv_timeout(DEADLINE) {
        Ok(Ok(tree)) => tree,
        Ok(Err(error)) => panic!(
            "{check}: a request for a {label:?} folder with the clock pinned to {PINNED_STAMP} \
             failed: {error}"
        ),
        Err(mpsc::RecvTimeoutError::Timeout) => panic!(
            "{check}: a request for a {label:?} folder returned nothing within {DEADLINE:?}; \
             the helper keeps retrying a taken name"
        ),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            panic!("{check}: a request for a {label:?} folder panicked")
        }
    }
}

/// The counter that completes `path`'s name after `prefix`.
fn counter_of(path: &Path, prefix: &str) -> u64 {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    name.strip_prefix(prefix)
        .and_then(|counter| counter.parse().ok())
        .unwrap_or_else(|| {
            panic!("{name} does not follow the helper's pinned naming {prefix}<counter>")
        })
}

/// Every node under `folder`, tagged by kind with its bytes.
fn tagged_tree(folder: &Path) -> BTreeMap<String, Vec<u8>> {
    support::capture(folder, support::CaptureOptions::default())
        .unwrap_or_else(|error| panic!("capture {}: {error}", folder.display()))
        .tagged()
}

/// `path` is a folder this request made: directly inside `temp`, named
/// `prefix` and a counter, absent from `before`, and empty.
fn assert_fresh(check: &str, path: &Path, temp: &Path, prefix: &str, before: &BTreeSet<String>) {
    assert_eq!(
        path.parent(),
        Some(temp),
        "{check}: {} is not directly inside the system temporary folder",
        path.display()
    );
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    assert!(
        name.starts_with(prefix),
        "{check}: {name} does not carry the pinned stamp in the helper's naming \
         {prefix}<counter>"
    );
    assert!(
        !before.contains(&name),
        "{check}: {name} was already there before the request; the helper adopted it"
    );
    let entries = fs::read_dir(path)
        .unwrap_or_else(|error| panic!("{check}: list {}: {error}", path.display()))
        .count();
    assert_eq!(entries, 0, "{check}: {} is not empty", path.display());
}

/// Where a folder under the system temporary folder may still be made
/// without the shared helper, by file and item (a type covers each of its
/// methods), and why. The freeze build folder (`fn build`, SVE-008) needs no
/// entry: it is made inside its caller's lane folder, never directly under
/// the system temporary folder.
const SCRATCH_EXCEPTIONS: [(&str, &str, &str); 2] = [
    (
        "test/qa/src/support.rs",
        "TempTree",
        "the shared helper itself",
    ),
    (
        "test-hidden/t-126/src/lib.rs",
        "Sandbox",
        "the shared-support oracle lane's own allocator, which puts a counter in each name \
         and stops on a taken name instead of retrying",
    ),
];

/// SVEV-012 / SVE-004: no test makes its own scratch folder. A source
/// inventory over `test/qa/src`, `test/qa/tests`, and every non-expired
/// roster lane finds no path made under the system temporary folder outside
/// the named exceptions; reading that folder's path is allowed. Every
/// `target/` folder and the frozen before-source corpus are not sources. The
/// matcher first proves itself on known shapes, and every exception must
/// still match a site, so a blind matcher cannot pass.
#[test]
fn svev_012() {
    scratch_controls();
    let root = repo_root();
    let (mut sources, roster) = build_message_inventory(&root);
    let expired = expired_lanes(&root);
    sources.retain(|path, _| !expired.iter().any(|lane| path.starts_with(lane.as_str())));
    let mut sites = Vec::new();
    let mut files = BTreeSet::new();
    let mut set_aside = Vec::new();
    let mut matched = [false; SCRATCH_EXCEPTIONS.len()];
    for (path, text) in &sources {
        let lines: Vec<&str> = text.lines().collect();
        for (line, item) in scratch_sites(text) {
            let shown = format!(
                "  {path}:{line}: {item}: {}",
                clip(lines.get(line - 1).copied().unwrap_or_default())
            );
            let exception = SCRATCH_EXCEPTIONS.iter().position(|(file, owner, _)| {
                path == file
                    && (item == *owner
                        || item
                            .strip_prefix(owner)
                            .is_some_and(|rest| rest.starts_with("::")))
            });
            match exception {
                Some(index) => {
                    matched[index] = true;
                    set_aside.push(format!("{shown} ({})", SCRATCH_EXCEPTIONS[index].2));
                }
                None => {
                    files.insert(path.as_str());
                    sites.push(shown);
                }
            }
        }
    }
    let stale: Vec<String> = SCRATCH_EXCEPTIONS
        .iter()
        .zip(matched)
        .filter(|(_, matched)| !matched)
        .map(|((file, owner, _), _)| format!("{file}: {owner}"))
        .collect();
    assert!(
        stale.is_empty(),
        "SVEV-012: every named exception must still match a site, or the matcher has gone \
         blind; matched nothing: {stale:?}"
    );
    assert!(
        sites.is_empty(),
        "SVEV-012: every folder under the system temporary folder must come from the shared \
         helper (`TempTree` in test/qa/src/support.rs); the inventory of {} sources \
         (test/qa/src, test/qa/tests, and {} non-expired roster lanes; expired, not read: \
         {expired:?}) found {} sites in {} files:\n{}\nset aside by named exceptions:\n{}",
        sources.len(),
        roster - expired.len(),
        sites.len(),
        files.len(),
        sites.join("\n"),
        set_aside.join("\n")
    );
}

/// The scratch matcher on known shapes first: each control is a source text
/// and exactly the sites it must yield, by line and item.
fn scratch_controls() {
    let controls: &[(&str, &[(usize, &str)])] = &[
        // Joined at once, on one line or split across two.
        (
            "fn a() {\n    let root = std::env::temp_dir().join(format!(\"x-{}\", 1));\n}\n",
            &[(2, "a")],
        ),
        (
            "impl Pair {\n    fn new() {\n        let r = std::env::temp_dir()\n            .join(\"p\");\n    }\n}\n",
            &[(3, "Pair::new")],
        ),
        // Bound by `let`, then joined in the same block.
        (
            "fn b() {\n    let mut base: PathBuf = env::temp_dir();\n    let p = base.join(\"x\");\n}\n",
            &[(2, "b")],
        ),
        // A trait impl names its type; an `impl` argument does not rename a function.
        (
            "impl Drop for Tree {\n    fn drop(&mut self) {\n        let p = temp_dir().join(\"x\");\n    }\n}\n",
            &[(3, "Tree::drop")],
        ),
        (
            "fn c(x: impl AsRef<Path>) {\n    let p = std::env::temp_dir().join(x);\n}\n",
            &[(2, "c")],
        ),
        // Reading the path is allowed.
        (
            "fn d() {\n    let temp = std::env::temp_dir();\n    assert!(outside(&temp));\n    \
             support::command(\"taskkill\", std::env::temp_dir());\n    \
             let _ = fs::read_dir(std::env::temp_dir());\n    \
             let _ = std::env::temp_dir().canonicalize();\n}\n",
            &[],
        ),
        // A binding ends with its block; a field of the same name is not it.
        (
            "fn e() {\n    let base = std::env::temp_dir();\n}\n\
             fn f(base: &Path) {\n    let p = base.join(\"x\");\n    let q = self.base.join(\"y\");\n}\n",
            &[],
        ),
        // Comments and strings never count.
        (
            "// std::env::temp_dir().join(\"x\")\nlet s = \"temp_dir().join(\";\n/* temp_dir().join(1) */\n",
            &[],
        ),
    ];
    for (source, expected) in controls {
        let expected: Vec<(usize, String)> = expected
            .iter()
            .map(|(line, item)| (*line, (*item).to_owned()))
            .collect();
        assert_eq!(
            scratch_sites(source),
            expected,
            "SVEV-012 control: the scratch matcher misread this source:\n{source}"
        );
    }
}

/// The roster lanes an expiry marker (`EXPIRED.toml`) sets aside, as
/// `/`-separated folder prefixes such as `test-hidden/t-078/`.
fn expired_lanes(root: &Path) -> Vec<String> {
    let declaration = root.join(".ratmac").join("lanes.toml");
    let lanes: toml::Value = fs::read_to_string(&declaration)
        .unwrap_or_else(|error| panic!("SVEV-012: read {}: {error}", declaration.display()))
        .parse()
        .unwrap_or_else(|error| panic!("SVEV-012: parse {}: {error}", declaration.display()));
    let roots = lanes.get("roots");
    let folder = roots
        .and_then(|roots| roots.get("lanes"))
        .and_then(toml::Value::as_str)
        .expect("SVEV-012: .ratmac/lanes.toml names the lane folder (roots.lanes)");
    roots
        .and_then(|roots| roots.get("roster"))
        .and_then(toml::Value::as_array)
        .expect("SVEV-012: .ratmac/lanes.toml declares the lane roster (roots.roster)")
        .iter()
        .filter_map(toml::Value::as_str)
        .filter(|id| root.join(folder).join(id).join("EXPIRED.toml").is_file())
        .map(|id| format!("{folder}/{id}/"))
        .collect()
}

/// The places one Rust source makes a path under the system temporary
/// folder, by 1-based line and enclosing item (`Type::method`, `function`):
/// `temp_dir()` joined at once, or bound by `let` to a name the same block
/// later joins onto. Reading the folder's path - comparing it, listing it,
/// starting a process there - is not a site; comments and strings never are.
fn scratch_sites(source: &str) -> Vec<(usize, String)> {
    let tokens = rust_tokens(source);
    let mark = |index: usize, character: char| matches!(tokens.get(index), Some((_, Token::Mark(found))) if *found == character);
    let word = |index: usize, text: &str| matches!(tokens.get(index), Some((_, Token::Word(found))) if found == text);
    let mut sites = Vec::new();
    // Open items, each with the brace depth its body opened at.
    let mut items: Vec<(usize, String)> = Vec::new();
    let mut pending: Option<String> = None;
    let mut depth = 0usize;
    let mut nesting = 0usize;
    // Live `let` bindings of the folder: name, depth, line, item, joined yet.
    let mut bound: Vec<(String, usize, usize, String, bool)> = Vec::new();
    for (at, (line, token)) in tokens.iter().enumerate() {
        match token {
            Token::Word(keyword) if matches!(keyword.as_str(), "fn" | "mod" | "trait") => {
                if let Some((_, Token::Word(name))) = tokens.get(at + 1) {
                    pending = Some(name.clone());
                }
            }
            Token::Word(keyword) if keyword == "impl" => {
                if nesting == 0 && pending.is_none() {
                    pending = Some(impl_name(&tokens[at + 1..]));
                }
            }
            Token::Mark('(' | '[') => nesting += 1,
            Token::Mark(')' | ']') => nesting = nesting.saturating_sub(1),
            Token::Mark(';') if nesting == 0 => pending = None,
            Token::Mark('{') => {
                depth += 1;
                if let Some(name) = pending.take() {
                    items.push((depth, name));
                }
            }
            Token::Mark('}') => {
                if items.last().is_some_and(|(open, _)| *open == depth) {
                    items.pop();
                }
                bound.retain(|binding| binding.1 < depth);
                depth = depth.saturating_sub(1);
            }
            Token::Word(name) if name == "temp_dir" && mark(at + 1, '(') && mark(at + 2, ')') => {
                let item = item_name(&items);
                if mark(at + 3, '.') && word(at + 4, "join") {
                    sites.push((*line, item));
                } else if mark(at + 3, ';') {
                    if let Some(name) = let_binding(&tokens, at) {
                        bound.push((name, depth, *line, item, false));
                    }
                }
            }
            Token::Word(name)
                if mark(at + 1, '.') && word(at + 2, "join") && !(at > 0 && mark(at - 1, '.')) =>
            {
                if let Some(binding) = bound.iter_mut().rev().find(|binding| &binding.0 == name) {
                    if !binding.4 {
                        binding.4 = true;
                        sites.push((binding.2, binding.3.clone()));
                    }
                }
            }
            _ => {}
        }
    }
    sites.sort();
    sites
}

/// The open items as one name, outermost first (`Type::method`).
fn item_name(items: &[(usize, String)]) -> String {
    if items.is_empty() {
        return "(top level)".to_owned();
    }
    items
        .iter()
        .map(|(_, name)| name.as_str())
        .collect::<Vec<_>>()
        .join("::")
}

/// The type an `impl` block is for: its last name outside angle brackets
/// before the block or a `where` clause opens (`impl Drop for Tree` is `Tree`).
fn impl_name(after: &[(usize, Token)]) -> String {
    let mut angle = 0usize;
    let mut name = String::from("impl");
    for (_, token) in after {
        match token {
            Token::Mark('<') => angle += 1,
            Token::Mark('>') => angle = angle.saturating_sub(1),
            Token::Mark('{') => break,
            Token::Word(word) if word == "where" => break,
            Token::Word(word) if angle == 0 && word != "for" => name = word.clone(),
            _ => {}
        }
    }
    name
}

/// The name a `let` statement binds when its whole value is the `temp_dir()`
/// call at `at` (`let base = std::env::temp_dir();`, `mut` and a type
/// included).
fn let_binding(tokens: &[(usize, Token)], at: usize) -> Option<String> {
    let token = |index: usize| tokens.get(index).map(|(_, token)| token);
    // Back over the call's path (`std::env::`, `env::`) to where it starts.
    let mut start = at;
    while start >= 3
        && token(start - 1) == Some(&Token::Mark(':'))
        && token(start - 2) == Some(&Token::Mark(':'))
        && matches!(token(start - 3), Some(Token::Word(_)))
    {
        start -= 3;
    }
    if start < 2 || token(start - 1) != Some(&Token::Mark('=')) {
        return None;
    }
    if matches!(token(start - 2), Some(Token::Mark('=' | '!' | '<' | '>'))) {
        return None;
    }
    let mut cursor = start - 1;
    while cursor > 0 {
        cursor -= 1;
        match token(cursor) {
            Some(Token::Word(keyword)) if keyword == "let" => {
                return tokens[cursor + 1..start - 1]
                    .iter()
                    .find_map(|(_, token)| match token {
                        Token::Word(name) if name != "mut" => Some(name.clone()),
                        _ => None,
                    });
            }
            Some(Token::Mark(';' | '{' | '}')) => return None,
            _ => {}
        }
    }
    None
}

// --- SVEV-013: one shared whole-line pause-marker wait -----------------------

/// One SVEV-013 step: the bytes staged at the marker path (`None` is no
/// file), the wait's limit, and the result the shared wait must give.
type StagedMarker = (
    Option<&'static [u8]>,
    Duration,
    Result<String, support::MarkerWait>,
);

/// SVEV-013 / SVE-005: the shared whole-line wait reports a missing marker,
/// an empty one, and boundary text without its closing newline as not yet
/// published, and returns the whole line once it is there. Each state is
/// written before the call and held fixed for the whole call, so no result
/// depends on the limit or on timing; the wait only reads, so the folder
/// afterwards holds exactly the marker as it was staged. A zero limit still
/// reads once, so it gives the same results for the partial and whole line.
#[test]
fn svev_013() {
    const LIMIT: Duration = Duration::from_millis(200);
    let tree = TempTree::new("t126-svev013").expect("SVEV-013: own the marker folder");
    let marker = tree.join("pause.marker");
    let expected = support::MarkerLine::Exact("after-validate\n");
    let states: [StagedMarker; 6] = [
        (None, LIMIT, Err(support::MarkerWait::NotYetPublished(None))),
        (
            Some(&b""[..]),
            LIMIT,
            Err(support::MarkerWait::NotYetPublished(Some(Vec::new()))),
        ),
        (
            Some(&b"after-validate"[..]),
            LIMIT,
            Err(support::MarkerWait::NotYetPublished(Some(
                b"after-validate".to_vec(),
            ))),
        ),
        // A zero limit still reads once (t086, t127, and t130 rely on it).
        (
            Some(&b"after-validate"[..]),
            Duration::ZERO,
            Err(support::MarkerWait::NotYetPublished(Some(
                b"after-validate".to_vec(),
            ))),
        ),
        // The valid positive case: the whole line is returned with its `\n`.
        (
            Some(&b"after-validate\n"[..]),
            LIMIT,
            Ok("after-validate\n".to_owned()),
        ),
        (
            Some(&b"after-validate\n"[..]),
            Duration::ZERO,
            Ok("after-validate\n".to_owned()),
        ),
    ];
    for (planted, limit, wanted) in states {
        if let Some(bytes) = planted {
            fs::write(&marker, bytes).expect("SVEV-013: stage the marker state");
        }
        let shown = planted.map(String::from_utf8_lossy);
        assert_eq!(
            support::await_marker_line(&marker, expected, limit),
            wanted,
            "SVEV-013: the shared wait misjudged the staged marker state {shown:?} at limit {limit:?}"
        );
        let mut entries = Vec::new();
        for entry in fs::read_dir(tree.path()).expect("SVEV-013: list the marker folder") {
            let entry = entry.expect("SVEV-013: read a marker folder entry");
            let bytes = fs::read(entry.path()).expect("SVEV-013: read a marker folder file");
            entries.push((entry.file_name().to_string_lossy().into_owned(), bytes));
        }
        let held: Vec<(String, Vec<u8>)> = planted
            .map(|bytes| vec![("pause.marker".to_owned(), bytes.to_vec())])
            .unwrap_or_default();
        assert_eq!(
            entries, held,
            "SVEV-013: the wait only reads; the folder holds exactly the staged marker {shown:?}"
        );
    }
}

// --- SVEV-015: every pause-marker waiter uses the shared whole-line wait ----

/// File, enclosing item (including a particular polling closure), and the
/// reason its filesystem polling is not a pause-marker wait. A row covers
/// that item and its nested closures, never its callers. Every row must
/// still match a finding, as with `SCRATCH_EXCEPTIONS`.
const NON_MARKER_WAITS: &[(&str, &str, &str)] = &[
    (
        "test-hidden/t-073/tests/hidden.rs",
        "wait_for_files",
        "QA barrier markers contain pid=<n> without a newline; the other paths are Windows \
         claim/handshake files and release sentinels, not RATMAC_TEST_*_MARKER paths",
    ),
    (
        "test-hidden/t-073/tests/hidden.rs",
        "wait_for_any_file",
        "Windows claim files, not pause markers",
    ),
    (
        "test-hidden/t-073/src/bin/abandon_racer.rs",
        "wait_for_release",
        "the racer's release sentinel, not its pause marker",
    ),
    (
        "test-hidden/t-073/src/bin/hold_racer.rs",
        "wait_for_release",
        "the racer's release sentinel, not its pause marker",
    ),
    (
        "test-hidden/t-134/src/lib.rs",
        "wait_for_child_run",
        "Run admission records (invocation.toml and run.toml), not pause markers",
    ),
    (
        "test-hidden/t-136/tests/hidden.rs",
        "wait_for_child_run",
        "Run admission records (invocation.toml and run.toml), not pause markers",
    ),
    (
        "test/qa/tests/t134_safe_nested_invocations.rs",
        "wait_for_run",
        "a child's admitted Run Record, not a pause marker",
    ),
    (
        "test-hidden/t-136/tests/hidden.rs",
        "park_external_holder::wait_until#1",
        "the external window-holder's marker is a command argument, not a RATMAC_TEST setting",
    ),
    (
        "test-hidden/t-136/tests/hidden.rs",
        "ht_136_05_output_filesystem::wait_until#1",
        "the external window-holder's marker is a command argument, not a RATMAC_TEST setting",
    ),
    (
        "test-hidden/t-136/tests/hidden.rs",
        "ht_136_01_regression::wait_until#1",
        "the runbook guard's external holder reports its guard marker, not a RATMAC_TEST setting",
    ),
    (
        "test-hidden/t-136/tests/hidden.rs",
        "ht_136_04_durability_recovery::wait_until#1",
        "a killed child's Run lock pathname disappears after process exit; it is not a pause marker",
    ),
    (
        "test-hidden/t-136/tests/hidden.rs",
        "ht_136_04_durability_recovery::wait_until#2",
        "a killed writer's gitignore.policy.stage disappears after process exit; it is not a pause marker",
    ),
    (
        "test-hidden/t-132/tests/hidden.rs",
        "ht_132_05_output_filesystem::read[recover-intent-l05.bin]",
        "the rendered recovery-intent bytes, not a pause marker; the crate-wide .keys field \
         also constructs marker paths, so its other derived paths need an exact named exception",
    ),
    (
        "test-hidden/t-132/tests/hidden.rs",
        "ht_132_06_cross_feature::read[recover-intent-l06.bin]",
        "the rendered recovery-intent bytes, not a pause marker; the crate-wide .keys field \
         also constructs marker paths, so its other derived paths need an exact named exception",
    ),
    (
        "test-hidden/t-136/tests/hidden.rs",
        "ht_136_02_input_routing::park_external_holder#1",
        "the claim-retry external window-holder takes its marker as a command argument, not a setting",
    ),
    (
        "test-hidden/t-136/tests/hidden.rs",
        "ht_136_02_input_routing::park_external_holder#2",
        "the indexed-first external window-holder takes its marker as a command argument, not a setting",
    ),
    (
        "test-hidden/t-136/tests/hidden.rs",
        "ht_136_02_input_routing::park_external_holder#3",
        "the successor's external window-holder takes its marker as a command argument, not a setting",
    ),
];

/// A crate that sets a pause-marker key but never waits needs an explicit,
/// still-matching explanation. There are no such crates in this inventory.
const MARKER_NO_WAITS: &[(&str, &str)] = &[];

/// File, `item::callee#N` call, and the reason a shared-wait call (or a
/// call of a wrapper that hands its path to the shared wait) waits on a
/// file no RATMAC_TEST_*_MARKER setting names. Every other such call must
/// have its path recognised as a marker, so a raw wait on the same path
/// beside it cannot go unseen. A row covers exactly one call and must
/// still match one.
const UNKEYED_SHARED_WAITS: &[(&str, &str, &str)] = &[
    (
        "test/qa/tests/t126_shared_private_test_support.rs",
        "svev_013::await_marker_line#1",
        "SVEV-013 stages each marker state itself in an owned folder; no setting names the file",
    ),
    (
        T142,
        "ht_142_02_input_routing::await_marker_line#1",
        HT_142_02_OWN,
    ),
    (
        T142,
        "ht_142_02_input_routing::await_marker_line#2",
        HT_142_02_OWN,
    ),
    (
        T142,
        "ht_142_02_input_routing::await_marker_line#3",
        HT_142_02_OWN,
    ),
    (
        T142,
        "ht_142_02_input_routing::await_marker_line#4",
        HT_142_02_OWN,
    ),
    (
        T142,
        "ht_142_02_input_routing::not_published#1",
        HT_142_02_OWN,
    ),
    (
        T142,
        "ht_142_03_lifecycle_model::await_marker_line#1",
        HT_142_03_OWN,
    ),
    (
        T142,
        "ht_142_03_lifecycle_model::await_marker_line#2",
        HT_142_03_OWN,
    ),
    (
        T142,
        "ht_142_03_lifecycle_model::not_published#1",
        HT_142_03_OWN,
    ),
    (
        T142,
        "ht_142_03_lifecycle_model::not_published#2",
        HT_142_03_OWN,
    ),
    (
        T142,
        "ht_142_03_lifecycle_model::live_stopped#1",
        HT_142_03_OWN,
    ),
    (
        T142,
        "ht_142_04_durability_recovery::await_marker_line#1",
        HT_142_04_OWN,
    ),
    (
        T142,
        "ht_142_04_durability_recovery::not_published#1",
        HT_142_04_OWN,
    ),
    (
        T142,
        "ht_142_04_durability_recovery::not_published#2",
        HT_142_04_OWN,
    ),
    (
        T142,
        "ht_142_04_durability_recovery::not_published#3",
        HT_142_04_OWN,
    ),
    (
        T142,
        "ht_142_04_durability_recovery::live_stopped#1",
        HT_142_04_OWN,
    ),
    (
        T142,
        "ht_142_05_output_filesystem::await_marker_line#1",
        HT_142_05_OWN,
    ),
    (
        T142,
        "ht_142_05_output_filesystem::not_published#1",
        HT_142_05_OWN,
    ),
    (
        T142,
        "ht_142_05_output_filesystem::not_published#2",
        HT_142_05_OWN,
    ),
    (
        T142,
        "ht_142_05_output_filesystem::not_published#3",
        HT_142_05_OWN,
    ),
];

const T142: &str = "test-hidden/t-142/tests/hidden.rs";
const HT_142_02_OWN: &str = "HT-142-02 writes this marker itself to route each input through the \
     shared wait; no setting names it";
const HT_142_03_OWN: &str =
    "HT-142-03's own writer threads publish this marker; no setting names it";
const HT_142_04_OWN: &str = "HT-142-04 stages this stopped marker, or its live stopped writers, \
     itself; no setting names it";
const HT_142_05_OWN: &str = "HT-142-05 rewrites this marker in place itself, or names a missing \
     path; no setting names it";

/// Every scope unit that sets a RATMAC_TEST_*_MARKER key. The scanned set
/// must match exactly, so a lane the inventory silently skips (or a new
/// key-setting crate nobody reviewed here) fails instead of passing blind.
const MARKER_KEY_CRATES: &[&str] = &[
    "test-hidden/t-073",
    "test-hidden/t-080",
    "test-hidden/t-081",
    "test-hidden/t-086",
    "test-hidden/t-087",
    "test-hidden/t-122",
    "test-hidden/t-124",
    "test-hidden/t-125",
    "test-hidden/t-127",
    "test-hidden/t-128",
    "test-hidden/t-129",
    "test-hidden/t-130",
    "test-hidden/t-131",
    "test-hidden/t-132",
    "test-hidden/t-134",
    "test-hidden/t-136",
    "test-hidden/t-142",
    "test-hidden/t-143",
    "test-hidden/t-148",
    "test/qa/tests/t050_blocked_route.rs",
    "test/qa/tests/t086_single_engine_binary.rs",
    "test/qa/tests/t104_operator_skill.rs",
    "test/qa/tests/t122_successor_ownership.rs",
    "test/qa/tests/t125_checkpoint_only_restoration.rs",
    "test/qa/tests/t127_edition_proof_preflight.rs",
    "test/qa/tests/t128_deterministic_trace_channel.rs",
    "test/qa/tests/t130_cycle_pause_routes.rs",
    "test/qa/tests/t134_safe_nested_invocations.rs",
    "test/qa/tests/t136_lock_contention.rs",
];

/// SVEV-015 / SVE-005: every wait on, or read of, a path supplied as a
/// RATMAC_TEST_*_MARKER setting uses the shared whole-line wait. A single
/// post-exit existence check is allowed. The roster, expiry markers, frozen
/// before-source exclusion, and target-folder exclusion are the same ones
/// used by SVEV-012. Synthetic controls run before reading the repository;
/// named exceptions and the per-key-setting-crate shared-call floor keep
/// this inventory from passing by going blind.
#[test]
fn svev_015() {
    // Every named exception keeps its file, its item, and a plain reason.
    let blank: Vec<String> = NON_MARKER_WAITS
        .iter()
        .filter(|(file, owner, reason)| {
            [file, owner, reason]
                .iter()
                .any(|text| text.trim().is_empty())
        })
        .map(|(file, owner, _)| format!("{file}: {owner}"))
        .chain(
            MARKER_NO_WAITS
                .iter()
                .filter(|(unit, reason)| unit.trim().is_empty() || reason.trim().is_empty())
                .map(|(unit, _)| format!("{unit}: no marker wait")),
        )
        .chain(
            UNKEYED_SHARED_WAITS
                .iter()
                .filter(|(file, call, reason)| {
                    [file, call, reason]
                        .iter()
                        .any(|text| text.trim().is_empty())
                })
                .map(|(file, call, _)| format!("{file}: {call}")),
        )
        .collect();
    assert!(
        blank.is_empty(),
        "SVEV-015: every named exception needs its file, its item, and a plain reason; blank: {blank:?}"
    );
    marker_controls();
    let root = repo_root();
    let (mut sources, roster) = build_message_inventory(&root);
    let expired = expired_lanes(&root);
    sources.retain(|path, _| !expired.iter().any(|lane| path.starts_with(lane.as_str())));
    let mut crates: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for (path, text) in sources {
        let unit = if path.starts_with("test/qa/tests/") {
            path.clone()
        } else if path.starts_with("test/qa/src/") {
            "test/qa/src".to_owned()
        } else {
            // The inventory paths already came from the declared roster;
            // all files below a lane folder belong to the same scope unit.
            path.split('/').take(2).collect::<Vec<_>>().join("/")
        };
        crates.entry(unit).or_default().insert(path, text);
    }
    let source_count: usize = crates.values().map(BTreeMap::len).sum();
    let mut sites = BTreeSet::new();
    let mut set_aside = Vec::new();
    let mut matched = vec![false; NON_MARKER_WAITS.len()];
    let mut no_wait_matched = vec![false; MARKER_NO_WAITS.len()];
    let mut unkeyed_matched = vec![false; UNKEYED_SHARED_WAITS.len()];
    let mut keyed = BTreeSet::new();
    for (unit, sources) in &crates {
        let inventory = marker_inventory(sources);
        if inventory.first_key.is_some() {
            keyed.insert(unit.as_str());
        }
        for (path, line, item) in inventory.sites {
            let exception = NON_MARKER_WAITS.iter().position(|(file, owner, _)| {
                path == *file
                    && (item == *owner
                        || item
                            .strip_prefix(owner)
                            .is_some_and(|rest| rest.starts_with("::")))
            });
            let source_line = sources[&path].lines().nth(line - 1).unwrap_or_default();
            let shown = format!("  {path}:{line}: {item}: {}", clip(source_line));
            if let Some(index) = exception {
                matched[index] = true;
                set_aside.push(format!("{shown} ({})", NON_MARKER_WAITS[index].2));
            } else {
                sites.insert((path, line, item, shown));
            }
        }
        for (path, line, call) in inventory.unkeyed {
            let source_line = sources[&path].lines().nth(line - 1).unwrap_or_default();
            let shown = format!(
                "  {path}:{line}: {call}: {} (shared wait on a path no marker setting is seen to name)",
                clip(source_line)
            );
            let exception = UNKEYED_SHARED_WAITS
                .iter()
                .position(|(file, owner, _)| path == *file && call == *owner);
            if let Some(index) = exception {
                unkeyed_matched[index] = true;
                set_aside.push(format!("{shown} ({})", UNKEYED_SHARED_WAITS[index].2));
            } else {
                sites.insert((path, line, call, shown));
            }
        }
        if let Some((path, line, item)) = inventory.first_key {
            if inventory.shared_calls == 0 {
                if let Some(index) = MARKER_NO_WAITS.iter().position(|(name, _)| name == unit) {
                    no_wait_matched[index] = true;
                    set_aside.push(format!(
                        "  {unit}: no marker wait ({})",
                        MARKER_NO_WAITS[index].1
                    ));
                } else {
                    sites.insert((path, line, item, format!(
                        "  {unit}: sets a pause-marker key but never calls support::await_marker_line"
                    )));
                }
            }
        }
    }
    let stale: Vec<String> = NON_MARKER_WAITS
        .iter()
        .zip(matched)
        .filter(|(_, matched)| !matched)
        .map(|((file, owner, _), _)| format!("{file}: {owner}"))
        .chain(
            MARKER_NO_WAITS
                .iter()
                .zip(no_wait_matched)
                .filter(|(_, matched)| !matched)
                .map(|((unit, _), _)| format!("{unit}: no marker wait")),
        )
        .chain(
            UNKEYED_SHARED_WAITS
                .iter()
                .zip(unkeyed_matched)
                .filter(|(_, matched)| !matched)
                .map(|((file, call, _), _)| format!("{file}: {call}")),
        )
        .collect();
    let pinned: BTreeSet<&str> = MARKER_KEY_CRATES.iter().copied().collect();
    let unscanned: Vec<&str> = pinned.difference(&keyed).copied().collect();
    let unpinned: Vec<&str> = keyed.difference(&pinned).copied().collect();
    let files: BTreeSet<&str> = sites.iter().map(|(path, _, _, _)| path.as_str()).collect();
    let shown: Vec<&str> = sites
        .iter()
        .map(|(_, _, _, shown)| shown.as_str())
        .collect();
    assert!(
        sites.is_empty() && stale.is_empty() && unscanned.is_empty() && unpinned.is_empty(),
        "SVEV-015: every pause-marker wait/read must use support::await_marker_line; \
         the inventory of {source_count} sources (test/qa/src, test/qa/tests, and {} \
         non-expired roster lanes; expired, not scanned: {expired:?}) found {} sites in {} files:\n{}\n\
         stale named exceptions (the matcher must still see every one): {stale:?}\n\
         pinned key-setting crates the scan did not see: {unscanned:?}\n\
         key-setting crates missing from MARKER_KEY_CRATES: {unpinned:?}\n\
         set aside by named exceptions:\n{}",
        roster - expired.len(), sites.len(), files.len(), shown.join("\n"), set_aside.join("\n")
    );
}

/// Identifiers in a marker expression. A data field is crate-wide; a local
/// variable stays in its enclosing item. Method/function names and path
/// qualifiers are never marker names (`raw(marker)`, `marker.as_os_str()`,
/// `markers.join("x")`, and `&hold.marker` are distinct useful shapes).
#[derive(Default)]
struct MarkerNames {
    words: BTreeSet<String>,
    fields: BTreeSet<String>,
}

struct MarkerItem {
    name: String,
    short: String,
    declaration: usize,
    body: (usize, usize),
    params: Vec<Option<String>>,
    names: BTreeSet<String>,
    /// Path expressions the function returns (its tail and each `return`).
    returns: Vec<(MarkerChain, MarkerNames)>,
    has_probe: bool,
    has_pause: bool,
    shared_call: bool,
    polling: bool,
}

struct MarkerCall {
    at: usize,
    name: String,
    owner: Option<usize>,
    args: Vec<(usize, usize)>,
    ordinal: usize,
    in_poll: bool,
}

/// A function's `fn` token and its parameter names, in order.
type MarkerSignature = (usize, Vec<Option<String>>);

/// One call or tuple of two or more arguments: its owner, then the token
/// spans of its first and second arguments.
type MarkerSetting = (usize, (usize, usize), (usize, usize));

/// `let NAME = <path chain>;` inside a function.
struct MarkerLet {
    owner: usize,
    name: String,
    chain: MarkerChain,
    names: MarkerNames,
}

/// Any `let PATTERN = EXPR;` inside a function, for the shapes a path chain
/// does not cover: names bound by tuple position (`let (marker, release) =
/// fx.hold_paths(..);`, `let marker = fx.hold_paths(..).0;`) to the call
/// whose result they take apart, and the environment-variable names the
/// expression mentions (`let marker = role_path("RATMAC_PRIVATE_..");`).
struct MarkerBind {
    owner: usize,
    /// Bound names; a tuple pattern or a `.N` suffix gives each a position.
    names: Vec<(Option<usize>, String)>,
    /// The call whose result is bound, past `?`, `.unwrap()`, `.expect(..)`.
    callee: Option<String>,
    texts: Vec<String>,
}

/// One field of a struct literal inside a function (`Held { marker, .. }`,
/// `Self { marker: path.clone() }`) and the names its value mentions.
struct MarkerInit {
    owner: usize,
    field: String,
    names: MarkerNames,
}

/// The shape of a path-valued expression: `&`/`*` prefixes, a root (a name
/// or field path, `PathBuf::from(..)`/`Path::new(..)` around a chain, or a
/// call), then only path-keeping methods. `identity` is false once `join`
/// moves below the root; `call` names a root call.
struct MarkerChain {
    identity: bool,
    call: Option<String>,
}

struct MarkerProbe {
    at: usize,
    owner: Option<usize>,
    names: MarkerNames,
    read: bool,
    in_poll: bool,
}

struct MarkerSource {
    path: String,
    tokens: Vec<(usize, Token)>,
    pairs: Vec<Option<usize>>,
    owners: Vec<Option<usize>>,
    items: Vec<MarkerItem>,
    calls: Vec<MarkerCall>,
    probes: Vec<MarkerProbe>,
    lets: Vec<MarkerLet>,
    binds: Vec<MarkerBind>,
    inits: Vec<MarkerInit>,
}

#[derive(Default)]
struct MarkerInventory {
    sites: BTreeSet<(String, usize, String)>,
    /// Coverage floor: shared-wait calls (or, through a wrapper parameter,
    /// the wrapper's callers) whose path the recogniser cannot tie to a
    /// RATMAC_TEST_*_MARKER setting. A raw wait beside one would go unseen.
    unkeyed: BTreeSet<(String, usize, String)>,
    first_key: Option<(String, usize, String)>,
    shared_calls: usize,
}

fn marker_mark(tokens: &[(usize, Token)], at: usize, want: char) -> bool {
    matches!(tokens.get(at), Some((_, Token::Mark(found))) if *found == want)
}

fn marker_word(tokens: &[(usize, Token)], at: usize) -> Option<&str> {
    match tokens.get(at) {
        Some((_, Token::Word(word))) => Some(word),
        _ => None,
    }
}

fn marker_key(text: &str) -> bool {
    text.strip_prefix("RATMAC_TEST_")
        .and_then(|text| text.strip_suffix("_MARKER"))
        .is_some_and(|middle| {
            !middle.is_empty()
                && middle
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        })
}

/// A `format!` literal with a marker key's shape: `RATMAC_TEST_`, then key
/// characters or `{..}` placeholders, then `_MARKER`.
fn marker_key_shape(text: &str) -> bool {
    let Some(middle) = text
        .strip_prefix("RATMAC_TEST_")
        .and_then(|text| text.strip_suffix("_MARKER"))
    else {
        return false;
    };
    let mut placeholder = false;
    let shaped = middle.chars().all(|character| match character {
        '{' if !placeholder => {
            placeholder = true;
            true
        }
        '}' if placeholder => {
            placeholder = false;
            true
        }
        _ if placeholder => !character.is_whitespace(),
        _ => character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_',
    });
    !middle.is_empty() && shaped && !placeholder
}

/// A `format!` literal `{name}_MARKER` (or `{name}_PART_MARKER`) whose
/// placeholder is a selector: a value known to start with `RATMAC_TEST_`.
fn marker_selector_shape(text: &str, selector: &dyn Fn(&str) -> bool) -> bool {
    let Some((name, rest)) = text.strip_prefix('{').and_then(|text| text.split_once('}')) else {
        return false;
    };
    rest.strip_suffix("_MARKER").is_some_and(|middle| {
        (middle.is_empty() || middle.starts_with('_'))
            && middle
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
    }) && selector(name)
}

/// An environment-variable name: upper-case words joined by `_`.
fn marker_env_name(text: &str) -> bool {
    text.starts_with(|character: char| character.is_ascii_uppercase())
        && text.contains('_')
        && text
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

/// Whether the first argument `start..end` of a call or tuple is a
/// pause-marker key: a key literal, a `const` holding one, a relayed name
/// the receiving process re-sets as a key, or a `format!` literal with a
/// key's shape (`RATMAC_TEST_{kind}_MARKER`, `{selector}_MARKER`), each
/// optionally followed by `.to_owned()`-style conversions.
fn marker_key_argument(
    tokens: &[(usize, Token)],
    start: usize,
    end: usize,
    keys: &BTreeSet<String>,
    relays: &BTreeSet<String>,
    selector: &dyn Fn(&str) -> bool,
) -> bool {
    let mut at = start;
    while at < end && marker_mark(tokens, at, '&') {
        at += 1;
    }
    let key = match tokens.get(at) {
        Some((_, Token::Text(text))) => marker_key(text) || relays.contains(text),
        // `format!("RATMAC_TEST_{kind}_MARKER", ..)` keeps the key's shape.
        Some((_, Token::Word(name)))
            if name == "format"
                && marker_mark(tokens, at + 1, '!')
                && marker_mark(tokens, at + 2, '(') =>
        {
            let shaped = matches!(tokens.get(at + 3), Some((_, Token::Text(text)))
                if marker_key_shape(text) || marker_selector_shape(text, selector));
            let mut depth = 0usize;
            at += 2;
            while at < end {
                match tokens[at].1 {
                    Token::Mark('(' | '[' | '{') => depth += 1,
                    Token::Mark(')' | ']' | '}') => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                at += 1;
            }
            shaped && at < end
        }
        Some((_, Token::Word(name))) => keys.contains(name),
        _ => false,
    };
    if !key {
        return false;
    }
    at += 1;
    while at < end {
        if marker_mark(tokens, at, '.')
            && matches!(
                marker_word(tokens, at + 1),
                Some("to_owned" | "to_string" | "into" | "as_str" | "as_ref")
            )
            && marker_mark(tokens, at + 2, '(')
            && marker_mark(tokens, at + 3, ')')
        {
            at += 4;
        } else {
            return false;
        }
    }
    true
}

fn marker_names(tokens: &[(usize, Token)], start: usize, end: usize) -> MarkerNames {
    let mut names = MarkerNames::default();
    for at in start..end {
        let Some(word) = marker_word(tokens, at) else {
            continue;
        };
        if word
            .as_bytes()
            .first()
            .is_none_or(|byte| byte.is_ascii_digit())
            || matches!(word, "as" | "mut" | "ref" | "move" | "true" | "false")
            || marker_mark(tokens, at + 1, '(')
            || marker_mark(tokens, at + 1, ':')
            || (at > start && marker_mark(tokens, at - 1, ':'))
        {
            continue;
        }
        if at > start && marker_mark(tokens, at - 1, '.') {
            names.fields.insert(word.to_owned());
        } else if !(marker_mark(tokens, at + 1, '.')
            && marker_word(tokens, at + 2).is_some()
            && !marker_mark(tokens, at + 3, '('))
        {
            names.words.insert(word.to_owned());
        }
    }
    names
}

fn marker_pairs(tokens: &[(usize, Token)]) -> Vec<Option<usize>> {
    let mut pairs = vec![None; tokens.len()];
    let mut stack = Vec::new();
    for (at, (_, token)) in tokens.iter().enumerate() {
        match token {
            Token::Mark('(' | '[' | '{') => stack.push(at),
            Token::Mark(')' | ']' | '}') => {
                if let Some(open) = stack.pop() {
                    pairs[open] = Some(at);
                    pairs[at] = Some(open);
                }
            }
            _ => {}
        }
    }
    pairs
}

fn marker_chain(
    tokens: &[(usize, Token)],
    pairs: &[Option<usize>],
    start: usize,
    end: usize,
) -> Option<MarkerChain> {
    let mut at = start;
    while at < end
        && (marker_mark(tokens, at, '&')
            || marker_mark(tokens, at, '*')
            || marker_word(tokens, at) == Some("mut"))
    {
        at += 1;
    }
    let root = marker_word(tokens, at)?;
    let mut chain = MarkerChain {
        identity: true,
        call: None,
    };
    if matches!(root, "PathBuf" | "Path")
        && marker_mark(tokens, at + 1, ':')
        && marker_mark(tokens, at + 2, ':')
        && matches!(marker_word(tokens, at + 3), Some("from" | "new"))
        && marker_mark(tokens, at + 4, '(')
    {
        let close = pairs[at + 4].filter(|&close| close < end)?;
        chain = marker_chain(tokens, pairs, at + 5, close)?;
        at = close + 1;
    } else if marker_mark(tokens, at + 1, '(') {
        let close = pairs[at + 1].filter(|&close| close < end)?;
        chain = MarkerChain {
            identity: false,
            call: Some(root.to_owned()),
        };
        at = close + 1;
    } else {
        at += 1;
        while marker_mark(tokens, at, '.')
            && marker_word(tokens, at + 1).is_some()
            && !marker_mark(tokens, at + 2, '(')
        {
            at += 2;
        }
    }
    while at < end {
        if marker_mark(tokens, at, '?') {
            at += 1;
            continue;
        }
        if !(marker_mark(tokens, at, '.') && marker_mark(tokens, at + 2, '(')) {
            return None;
        }
        let close = pairs[at + 2].filter(|&close| close < end)?;
        match marker_word(tokens, at + 1)? {
            "clone" | "to_owned" | "to_path_buf" | "as_path" | "as_ref" | "as_os_str"
            | "borrow" | "into" | "unwrap" | "expect" => {}
            "join" => chain.identity = false,
            _ => return None,
        }
        at = close + 1;
    }
    Some(chain)
}

/// Top-level comma-separated arguments of one call or tuple. Nested calls,
/// blocks, and arrays remain one argument, including closure bodies.
fn marker_args(
    tokens: &[(usize, Token)],
    pairs: &[Option<usize>],
    open: usize,
) -> Vec<(usize, usize)> {
    let Some(close) = pairs[open] else {
        return Vec::new();
    };
    let mut args = Vec::new();
    let mut start = open + 1;
    let mut at = start;
    while at < close {
        if marker_mark(tokens, at, ',') {
            args.push((start, at));
            start = at + 1;
        } else if matches!(tokens[at].1, Token::Mark('(' | '[' | '{')) {
            if let Some(end) = pairs[at] {
                at = end;
            }
        }
        at += 1;
    }
    if start < close {
        args.push((start, close));
    }
    args
}

fn marker_receiver(tokens: &[(usize, Token)], pairs: &[Option<usize>], end: usize) -> usize {
    let mut at = end;
    while at > 0 {
        let previous = at - 1;
        match &tokens[previous].1 {
            Token::Mark(')' | ']') => {
                at = pairs[previous].unwrap_or(previous);
            }
            Token::Mark(';' | ',' | '=' | '{' | '}' | '|' | '<' | '>') => break,
            Token::Word(word)
                if matches!(word.as_str(), "if" | "while" | "return" | "let" | "else") =>
            {
                break
            }
            _ => at -= 1,
        }
    }
    at
}

fn marker_source(path: &str, source: &str) -> MarkerSource {
    let tokens = rust_tokens(source);
    let pairs = marker_pairs(&tokens);
    let mut owners = vec![None; tokens.len()];
    let mut items = Vec::new();
    let mut stack = Vec::new();
    let mut pending: Option<(String, Option<MarkerSignature>)> = None;
    let mut depth = 0usize;
    let mut nesting = 0usize;
    for at in 0..tokens.len() {
        match &tokens[at].1 {
            Token::Word(keyword) if keyword == "fn" => {
                if let Some(name) = marker_word(&tokens, at + 1) {
                    if let Some(open) =
                        (at + 2..tokens.len()).find(|&cursor| marker_mark(&tokens, cursor, '('))
                    {
                        let params = marker_args(&tokens, &pairs, open)
                            .into_iter()
                            .map(|(start, end)| {
                                (start..end)
                                    .filter_map(|cursor| marker_word(&tokens, cursor))
                                    .find(|word| !matches!(*word, "mut" | "ref"))
                                    .map(str::to_owned)
                            })
                            .collect();
                        pending = Some((name.to_owned(), Some((at, params))));
                    }
                }
            }
            Token::Word(keyword) if matches!(keyword.as_str(), "mod" | "trait") => {
                if let Some(name) = marker_word(&tokens, at + 1) {
                    pending = Some((name.to_owned(), None));
                }
            }
            Token::Word(keyword) if keyword == "impl" && nesting == 0 && pending.is_none() => {
                pending = Some((impl_name(&tokens[at + 1..]), None));
            }
            Token::Mark('(' | '[') => nesting += 1,
            Token::Mark(')' | ']') => nesting = nesting.saturating_sub(1),
            Token::Mark(';') if nesting == 0 => pending = None,
            Token::Mark('{') => {
                depth += 1;
                if let Some((name, function)) = pending.take() {
                    stack.push((depth, name.clone()));
                    if let Some((declaration, params)) = function {
                        let index = items.len();
                        let end = pairs[at].unwrap_or(at);
                        owners[at..=end].fill(Some(index));
                        items.push(MarkerItem {
                            name: item_name(&stack),
                            short: name,
                            declaration,
                            body: (at, end),
                            params,
                            names: BTreeSet::new(),
                            returns: Vec::new(),
                            has_probe: false,
                            has_pause: false,
                            shared_call: false,
                            polling: false,
                        });
                    }
                }
            }
            Token::Mark('}') => {
                if stack.last().is_some_and(|(open, _)| *open == depth) {
                    stack.pop();
                }
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    let mut in_poll = vec![false; tokens.len()];
    // A while condition belongs to its polling block too. A `for` loop in a
    // function polls when its body pauses or breaks out (`for _ in 0..n {
    // if marker.exists() { break } sleep }`); plain iteration over
    // post-exit checks is not a wait.
    for at in 0..tokens.len() {
        let Some(keyword @ ("loop" | "while" | "for")) = marker_word(&tokens, at) else {
            continue;
        };
        let mut open = at + 1;
        let mut header_in = false;
        while open < tokens.len() && !marker_mark(&tokens, open, '{') {
            header_in |= marker_word(&tokens, open) == Some("in");
            if matches!(tokens[open].1, Token::Mark('(' | '[')) {
                open = pairs[open].unwrap_or(open);
            }
            open += 1;
        }
        let Some(end) = pairs.get(open).copied().flatten() else {
            continue;
        };
        let pauses = || {
            (open..end).any(|cursor| {
                marker_word(&tokens, cursor) == Some("break")
                    || (matches!(marker_word(&tokens, cursor), Some("sleep" | "yield_now"))
                        && marker_mark(&tokens, cursor + 1, '('))
            })
        };
        if keyword != "for" || (owners[at].is_some() && header_in && pauses()) {
            in_poll[at..=end].fill(true);
        }
    }
    let mut calls = Vec::new();
    let mut ordinals: BTreeMap<(Option<usize>, String), usize> = BTreeMap::new();
    for at in 0..tokens.len() {
        if !marker_mark(&tokens, at, '(') {
            continue;
        }
        let args = marker_args(&tokens, &pairs, at);
        for &(start, end) in &args {
            let start = if marker_word(&tokens, start) == Some("move") {
                start + 1
            } else {
                start
            };
            if start < end && marker_mark(&tokens, start, '|') {
                in_poll[start..end].fill(true);
            }
        }
        let Some(callee) = at.checked_sub(1) else {
            continue;
        };
        let Some(name) = marker_word(&tokens, callee) else {
            continue;
        };
        if callee > 0 && marker_word(&tokens, callee - 1) == Some("fn") {
            continue;
        }
        let owner = owners[callee];
        let ordinal = ordinals.entry((owner, name.to_owned())).or_default();
        *ordinal += 1;
        calls.push(MarkerCall {
            at: callee,
            name: name.to_owned(),
            owner,
            args,
            ordinal: *ordinal,
            in_poll: false,
        });
    }
    // Closures bound to locals are not call arguments. Count those too;
    // `||` used as a boolean operator does not start after `=` or `move`.
    for at in 1..tokens.len() {
        if !marker_mark(&tokens, at, '|')
            || !(marker_mark(&tokens, at - 1, '=') || marker_word(&tokens, at - 1) == Some("move"))
        {
            continue;
        }
        let Some(close) = (at + 1..tokens.len()).find(|&end| marker_mark(&tokens, end, '|')) else {
            continue;
        };
        let body = close + 1;
        let mut end = body;
        if marker_mark(&tokens, body, '{') {
            end = pairs[body].map(|end| end + 1).unwrap_or(body);
        } else {
            while end < tokens.len() && !matches!(tokens[end].1, Token::Mark(';' | ',' | ')')) {
                if matches!(tokens[end].1, Token::Mark('(' | '[' | '{')) {
                    end = pairs[end].unwrap_or(end);
                }
                end += 1;
            }
        }
        in_poll[at..end].fill(true);
    }
    for call in &mut calls {
        call.in_poll = in_poll[call.at];
    }
    // `let NAME = <path chain>;` bindings, for marker aliases.
    let mut lets = Vec::new();
    for (at, owner) in owners.iter().enumerate() {
        if marker_word(&tokens, at) != Some("let") {
            continue;
        }
        let Some(owner) = *owner else { continue };
        let mut cursor = at + 1;
        if marker_word(&tokens, cursor) == Some("mut") {
            cursor += 1;
        }
        let Some(name) = marker_word(&tokens, cursor) else {
            continue;
        };
        cursor += 1;
        if marker_mark(&tokens, cursor, ':') {
            while cursor < tokens.len()
                && !marker_mark(&tokens, cursor, '=')
                && !marker_mark(&tokens, cursor, ';')
            {
                if matches!(tokens[cursor].1, Token::Mark('(' | '[')) {
                    cursor = pairs[cursor].unwrap_or(cursor);
                }
                cursor += 1;
            }
        }
        if !marker_mark(&tokens, cursor, '=') || marker_mark(&tokens, cursor + 1, '=') {
            continue;
        }
        let start = cursor + 1;
        let mut end = start;
        while end < tokens.len() && !marker_mark(&tokens, end, ';') {
            if matches!(tokens[end].1, Token::Mark('(' | '[' | '{')) {
                end = pairs[end].unwrap_or(end);
            }
            end += 1;
        }
        if let Some(chain) = marker_chain(&tokens, &pairs, start, end) {
            lets.push(MarkerLet {
                owner,
                name: name.to_owned(),
                chain,
                names: marker_names(&tokens, start, end),
            });
        }
    }
    // Every `let` in a function, whatever its pattern (see `MarkerBind`).
    let mut binds = Vec::new();
    for (at, owner) in owners.iter().enumerate() {
        if marker_word(&tokens, at) != Some("let") {
            continue;
        }
        let Some(owner) = *owner else { continue };
        let mut cursor = at + 1;
        let mut names = Vec::new();
        if marker_mark(&tokens, cursor, '(') {
            let Some(close) = pairs[cursor] else { continue };
            for (position, (start, end)) in
                marker_args(&tokens, &pairs, cursor).into_iter().enumerate()
            {
                let words: Vec<&str> = (start..end)
                    .filter_map(|cursor| marker_word(&tokens, cursor))
                    .filter(|word| !matches!(*word, "mut" | "ref"))
                    .collect();
                if let [name] = words[..] {
                    if name != "_"
                        && (start..end).all(|cursor| marker_word(&tokens, cursor).is_some())
                    {
                        names.push((Some(position), name.to_owned()));
                    }
                }
            }
            cursor = close + 1;
        } else {
            if marker_word(&tokens, cursor) == Some("mut") {
                cursor += 1;
            }
            let Some(name) = marker_word(&tokens, cursor) else {
                continue;
            };
            names.push((None, name.to_owned()));
            cursor += 1;
        }
        if marker_mark(&tokens, cursor, ':') {
            while cursor < tokens.len()
                && !marker_mark(&tokens, cursor, '=')
                && !marker_mark(&tokens, cursor, ';')
            {
                if matches!(tokens[cursor].1, Token::Mark('(' | '[')) {
                    cursor = pairs[cursor].unwrap_or(cursor);
                }
                cursor += 1;
            }
        }
        if !marker_mark(&tokens, cursor, '=') || marker_mark(&tokens, cursor + 1, '=') {
            continue;
        }
        let start = cursor + 1;
        let mut end = start;
        while end < tokens.len() && !marker_mark(&tokens, end, ';') {
            if matches!(tokens[end].1, Token::Mark('(' | '[' | '{')) {
                end = pairs[end].unwrap_or(end);
            }
            end += 1;
        }
        let texts = tokens[start..end]
            .iter()
            .filter_map(|(_, token)| match token {
                Token::Text(text) if marker_env_name(text) => Some(text.clone()),
                _ => None,
            })
            .collect();
        // `helper(..).0` binds one position of the helper's result.
        let mut tail = end;
        if names.len() == 1
            && names[0].0.is_none()
            && tail >= start + 2
            && marker_mark(&tokens, tail - 2, '.')
        {
            if let Some(position) =
                marker_word(&tokens, tail - 1).and_then(|word| word.parse::<usize>().ok())
            {
                names[0].0 = Some(position);
                tail -= 2;
            }
        }
        let call_open = |tail: usize| {
            tail.checked_sub(1)
                .filter(|&close| close > start && marker_mark(&tokens, close, ')'))
                .and_then(|close| pairs[close])
                .filter(|&open| open > start)
        };
        loop {
            if tail > start + 1 && marker_mark(&tokens, tail - 1, '?') {
                tail -= 1;
                continue;
            }
            match call_open(tail) {
                Some(open)
                    if open >= start + 2
                        && matches!(marker_word(&tokens, open - 1), Some("unwrap" | "expect"))
                        && marker_mark(&tokens, open - 2, '.') =>
                {
                    tail = open - 2;
                }
                _ => break,
            }
        }
        let callee = call_open(tail)
            .and_then(|open| marker_word(&tokens, open - 1))
            .map(str::to_owned);
        binds.push(MarkerBind {
            owner,
            names,
            callee,
            texts,
        });
    }
    // Struct literal fields: `Name {` or `Self {` inside a function, not a
    // function's own body and not a type (`-> Held {`). A block after an
    // upper-case name (`if n == LIMIT { .. }`) fails the field shape.
    let mut inits = Vec::new();
    for (open, owner) in owners.iter().enumerate().skip(1) {
        if !marker_mark(&tokens, open, '{') {
            continue;
        }
        let Some(owner) = *owner else { continue };
        if items[owner].body.0 == open
            || !marker_word(&tokens, open - 1).is_some_and(|name| {
                name.starts_with(|character: char| character.is_ascii_uppercase())
            })
        {
            continue;
        }
        let mut first = open - 1;
        while first >= 3
            && marker_mark(&tokens, first - 1, ':')
            && marker_mark(&tokens, first - 2, ':')
            && marker_word(&tokens, first - 3).is_some()
        {
            first -= 3;
        }
        if first.checked_sub(1).is_some_and(|before| {
            (marker_mark(&tokens, before, '>')
                && before > 0
                && marker_mark(&tokens, before - 1, '-'))
                || matches!(
                    marker_word(&tokens, before),
                    Some(
                        "struct"
                            | "enum"
                            | "union"
                            | "impl"
                            | "for"
                            | "trait"
                            | "mod"
                            | "fn"
                            | "dyn"
                            | "where"
                            | "match"
                            | "if"
                            | "while"
                            | "in"
                    )
                )
        }) {
            continue;
        }
        let mut fields = Vec::new();
        let shaped = marker_args(&tokens, &pairs, open)
            .into_iter()
            .all(|(start, end)| {
                if marker_mark(&tokens, start, '.') {
                    return true;
                }
                let Some(field) = marker_word(&tokens, start) else {
                    return false;
                };
                let value = if end == start + 1 {
                    start
                } else if marker_mark(&tokens, start + 1, ':')
                    && !marker_mark(&tokens, start + 2, ':')
                    && start + 2 < end
                {
                    start + 2
                } else {
                    return false;
                };
                fields.push((field.to_owned(), marker_names(&tokens, value, end)));
                true
            });
        if shaped {
            inits.extend(fields.into_iter().map(|(field, names)| MarkerInit {
                owner,
                field,
                names,
            }));
        }
    }
    // Each function's returned path expressions: its tail and each `return`.
    for (index, item) in items.iter_mut().enumerate() {
        let (open, close) = item.body;
        let mut tail = open + 1;
        let mut at = open + 1;
        while at < close {
            match tokens[at].1 {
                Token::Mark(';') => tail = at + 1,
                Token::Mark('{') => {
                    at = pairs[at].unwrap_or(at);
                    tail = at + 1;
                }
                Token::Mark('(' | '[') => at = pairs[at].unwrap_or(at),
                _ => {}
            }
            at += 1;
        }
        let mut ranges = vec![(tail, close)];
        for (at, owner) in owners.iter().enumerate().take(close).skip(open + 1) {
            if *owner != Some(index) || marker_word(&tokens, at) != Some("return") {
                continue;
            }
            let mut end = at + 1;
            while end < close && !matches!(tokens[end].1, Token::Mark(';' | '}' | ')' | ']' | ','))
            {
                if matches!(tokens[end].1, Token::Mark('(' | '[' | '{')) {
                    end = pairs[end].unwrap_or(end);
                }
                end += 1;
            }
            ranges.push((at + 1, end));
        }
        item.returns = ranges
            .into_iter()
            .filter(|&(start, end)| start < end)
            .filter_map(|(start, end)| {
                marker_chain(&tokens, &pairs, start, end)
                    .map(|chain| (chain, marker_names(&tokens, start, end)))
            })
            .collect();
    }
    let mut probes = Vec::new();
    for call in &calls {
        let at = call.at;
        let namespace = at
            .checked_sub(3)
            .and_then(|index| marker_word(&tokens, index));
        let qualified =
            at >= 2 && marker_mark(&tokens, at - 1, ':') && marker_mark(&tokens, at - 2, ':');
        let method = at > 0 && marker_mark(&tokens, at - 1, '.');
        let read = (qualified
            && namespace == Some("fs")
            && matches!(
                call.name.as_str(),
                "read" | "read_to_string" | "metadata" | "symlink_metadata"
            ))
            || (qualified && namespace == Some("File") && call.name == "open")
            // `OpenOptions::new().read(true).open(path)`, `File::options()..`
            || (method && call.name == "open")
            || (call.name == "read_to_string" && !method);
        let check = matches!(
            call.name.as_str(),
            "exists" | "is_file" | "try_exists" | "metadata" | "symlink_metadata"
        );
        let presence = check && method;
        // `Path::exists(&marker)` checks its first argument.
        let qualified_check = check && qualified && matches!(namespace, Some("Path" | "PathBuf"));
        if read || presence || qualified_check {
            let names = if presence {
                marker_names(&tokens, marker_receiver(&tokens, &pairs, at - 1), at - 1)
            } else if let Some(&(start, end)) = call.args.first() {
                marker_names(&tokens, start, end)
            } else {
                MarkerNames::default()
            };
            probes.push(MarkerProbe {
                at,
                owner: call.owner,
                names,
                read,
                in_poll: in_poll[at],
            });
            if let Some(owner) = call.owner {
                items[owner].has_probe = true;
            }
        }
        if let Some(owner) = call.owner {
            if matches!(call.name.as_str(), "sleep" | "yield_now") {
                items[owner].has_pause = true;
                items[owner].polling = true;
            }
            if call.name == "await_marker_line" {
                items[owner].shared_call = true;
            }
        }
    }
    MarkerSource {
        path: path.to_owned(),
        tokens,
        pairs,
        owners,
        items,
        calls,
        probes,
        lets,
        binds,
        inits,
    }
}

fn marker_shared(source: &MarkerSource, item: &MarkerItem) -> bool {
    source.path == "test/qa/src/support.rs" && item.short == "await_marker_line"
}

fn marker_delegator(item: &MarkerItem) -> bool {
    item.shared_call && !item.has_pause && !item.has_probe
}

fn marker_mentions(
    names: &MarkerNames,
    local: &BTreeSet<String>,
    fields: &BTreeSet<String>,
) -> bool {
    names.words.iter().any(|name| local.contains(name))
        || names.fields.iter().any(|field| fields.contains(field))
}

/// One crate's token inventory. Keys seed local names and crate-wide data
/// fields. Marker parameters propagate backwards through local callers to
/// a fixpoint. Polling propagates through local callees too (yield_now is a
/// pause just as sleep is); an opaque wait*/await* filesystem poller is a
/// finding even when its caller's marker expression hides the local name.
///
/// Known limits: the recogniser is token-level, one scope unit at a time.
/// It does not follow a marker into a collection (`vec![marker]` then
/// `for m in &all`), through struct or `Some` patterns (`let Held { marker:
/// m, .. }`, `if let Some(m)`), into a closure parameter, or as a field into
/// a one-shot read helper; nor a key spelled another way (`concat!`, an
/// indexed key array), a path read back with `env::var(KEY)`, a poll of the
/// marker's folder (`fs::read_dir`), a pause-free recursion, or a helper
/// defined in another scope unit. A one-shot existence check always counts
/// as post-exit. A shared wait on a path the recogniser misses is still
/// reported, by the coverage floor.
fn marker_inventory(sources: &BTreeMap<String, String>) -> MarkerInventory {
    let mut parsed: Vec<MarkerSource> = sources
        .iter()
        .map(|(path, text)| marker_source(path, text))
        .collect();
    let mut inventory = MarkerInventory::default();
    let mut keys = BTreeSet::new();
    for source in &parsed {
        for at in 0..source.tokens.len() {
            if !matches!(marker_word(&source.tokens, at), Some("const" | "static")) {
                continue;
            }
            let skip = usize::from(marker_word(&source.tokens, at + 1) == Some("mut"));
            let Some(name) = marker_word(&source.tokens, at + 1 + skip) else {
                continue;
            };
            let end = (at + 2..source.tokens.len())
                .find(|&end| marker_mark(&source.tokens, end, ';'))
                .unwrap_or(source.tokens.len());
            if source.tokens[at + 2..end]
                .iter()
                .any(|(_, token)| matches!(token, Token::Text(text) if marker_key(text)))
            {
                keys.insert(name.to_owned());
            }
        }
    }
    // Every call or tuple with two or more arguments inside a function: its
    // owner, its first argument, and its second. Only a key expression in
    // the first top-level argument is a setting; a diagnostic containing
    // the key, or a one-argument read of an environment variable by its
    // publisher, is not.
    let settings: Vec<Vec<MarkerSetting>> = parsed
        .iter()
        .map(|source| {
            (0..source.tokens.len())
                .filter(|&open| marker_mark(&source.tokens, open, '('))
                .filter_map(|open| {
                    let owner = source.owners[open]?;
                    let args = marker_args(&source.tokens, &source.pairs, open);
                    (args.len() >= 2).then(|| (owner, args[0], args[1]))
                })
                .collect()
        })
        .collect();
    let mut fields = BTreeSet::new();
    // Names a receiving process reads and re-sets as a key, `(function,
    // parameter)` selectors known to start with `RATMAC_TEST_`, and
    // `(helper, N)` result positions known to hold a marker.
    let mut relays: BTreeSet<String> = BTreeSet::new();
    let mut selectors: BTreeSet<(String, String)> = BTreeSet::new();
    let mut positions: BTreeSet<(String, usize)> = BTreeSet::new();
    // Keys seed marker names. A parameter used by a key-setting wrapper
    // makes every matching argument at each crate-local caller a marker
    // expression. Repeating this is necessary for wrappers around wrappers
    // and cross-file calls. The same fixed point carries marker names
    // through `let` aliases, through the result of a helper that sets a key
    // on a path it returns, through a helper's result position, through a
    // struct field filled from a marker, through a relayed environment
    // name, and through a selector a `format!` key is built from.
    let mut returning: BTreeSet<String> = BTreeSet::new();
    loop {
        let mut changed = false;
        for (source, settings) in parsed.iter_mut().zip(&settings) {
            for &(owner, (start, end), (value_start, value_end)) in settings {
                let key = {
                    let short = source.items[owner].short.as_str();
                    marker_key_argument(&source.tokens, start, end, &keys, &relays, &|name| {
                        selectors
                            .iter()
                            .any(|(item, parameter)| item == short && parameter == name)
                    })
                };
                if !key {
                    continue;
                }
                inventory.first_key.get_or_insert_with(|| {
                    (
                        source.path.clone(),
                        source.tokens[start].0,
                        source.items[owner].name.clone(),
                    )
                });
                let names = marker_names(&source.tokens, value_start, value_end);
                for word in names.words {
                    changed |= source.items[owner].names.insert(word);
                }
                for field in names.fields {
                    changed |= fields.insert(field);
                }
            }
        }
        let mut additions = Vec::new();
        let mut found = Vec::new();
        for source in &parsed {
            for call in &source.calls {
                let Some(owner) = call.owner else { continue };
                let caller = source.items[owner].short.as_str();
                for target in &parsed {
                    for item in target.items.iter().filter(|item| item.short == call.name) {
                        let offset = usize::from(
                            item.params.first().and_then(Option::as_deref) == Some("self")
                                && call.at > 0
                                && marker_mark(&source.tokens, call.at - 1, '.'),
                        );
                        for (index, parameter) in item.params.iter().enumerate().skip(offset) {
                            let Some(parameter) = parameter else { continue };
                            let Some(&(start, end)) = call.args.get(index - offset) else {
                                continue;
                            };
                            if item.names.contains(parameter) {
                                additions.push((
                                    source.path.clone(),
                                    owner,
                                    marker_names(&source.tokens, start, end),
                                ));
                            }
                            // A selector arrives as a `RATMAC_TEST_` literal
                            // or as one of the caller's own selectors.
                            let selector = source.tokens[start..end].iter().any(|(_, token)| {
                                matches!(token, Token::Text(text) if text.starts_with("RATMAC_TEST_"))
                            }) || (start..end)
                                .filter_map(|cursor| marker_word(&source.tokens, cursor))
                                .any(|word| {
                                    selectors
                                        .iter()
                                        .any(|(item, name)| item == caller && name == word)
                                });
                            if selector {
                                found.push((item.short.clone(), parameter.clone()));
                            }
                        }
                    }
                }
            }
        }
        for selector in found {
            changed |= selectors.insert(selector);
        }
        for (path, owner, names) in additions {
            let source = parsed
                .iter_mut()
                .find(|source| source.path == path)
                .unwrap();
            let before = source.items[owner].names.len() + fields.len();
            source.items[owner].names.extend(names.words);
            fields.extend(names.fields);
            changed |= before != source.items[owner].names.len() + fields.len();
        }
        for source in &mut parsed {
            let MarkerSource { lets, items, .. } = source;
            for binding in lets.iter() {
                let item = &mut items[binding.owner];
                let forward = match &binding.chain.call {
                    Some(call) => returning.contains(call),
                    None => marker_mentions(&binding.names, &item.names, &fields),
                };
                if forward {
                    changed |= item.names.insert(binding.name.clone());
                }
                // The same file under another name marks its source too.
                if binding.chain.identity
                    && binding.chain.call.is_none()
                    && item.names.contains(&binding.name)
                {
                    for word in &binding.names.words {
                        changed |= item.names.insert(word.clone());
                    }
                    for field in &binding.names.fields {
                        changed |= fields.insert(field.clone());
                    }
                }
            }
        }
        let newly: Vec<String> = parsed
            .iter()
            .flat_map(|source| source.items.iter())
            .filter(|item| {
                !returning.contains(&item.short)
                    && item.returns.iter().any(|(chain, names)| match &chain.call {
                        Some(call) => returning.contains(call),
                        None => marker_mentions(names, &item.names, &fields),
                    })
            })
            .map(|item| item.short.clone())
            .collect();
        for name in newly {
            changed |= returning.insert(name);
        }
        // A helper's result taken apart by position: once one call binds a
        // marker at position N, position N is a marker at every call.
        let learned: Vec<(String, usize)> = parsed
            .iter()
            .flat_map(|source| {
                source.binds.iter().flat_map(move |bind| {
                    bind.names.iter().filter_map(move |(position, name)| {
                        let callee = bind.callee.as_ref()?;
                        let position = (*position)?;
                        source.items[bind.owner]
                            .names
                            .contains(name)
                            .then(|| (callee.clone(), position))
                    })
                })
            })
            .collect();
        for entry in learned {
            changed |= positions.insert(entry);
        }
        for source in &mut parsed {
            let MarkerSource { binds, items, .. } = source;
            for bind in binds.iter() {
                let Some(callee) = &bind.callee else { continue };
                for (position, name) in &bind.names {
                    if position.is_some_and(|position| {
                        positions
                            .iter()
                            .any(|(helper, at)| helper == callee && *at == position)
                    }) {
                        changed |= items[bind.owner].names.insert(name.clone());
                    }
                }
            }
        }
        // A marker bound from a value that mentions an environment-variable
        // name (`role_path("RATMAC_PRIVATE_..")`) was relayed under that
        // name, so setting that name passes a marker too.
        for source in &parsed {
            for bind in &source.binds {
                if bind.names.iter().any(|(position, name)| {
                    position.is_none() && source.items[bind.owner].names.contains(name)
                }) {
                    for text in bind.texts.iter().filter(|text| !marker_key(text)) {
                        changed |= relays.insert(text.clone());
                    }
                }
            }
        }
        // A struct field filled from a local marker is a marker field
        // crate-wide (`Self { marker, .. }`, later `held.marker`).
        for source in &parsed {
            for init in &source.inits {
                if init
                    .names
                    .words
                    .iter()
                    .any(|word| source.items[init.owner].names.contains(word))
                {
                    changed |= fields.insert(init.field.clone());
                }
            }
        }
        if !changed {
            break;
        }
    }
    loop {
        let polling: BTreeSet<String> = parsed
            .iter()
            .flat_map(|source| {
                source
                    .items
                    .iter()
                    .filter(|item| item.polling && !marker_shared(source, item))
                    .map(|item| item.short.clone())
            })
            .collect();
        let mut changed = false;
        for source in &mut parsed {
            for call in &source.calls {
                if polling.contains(&call.name) {
                    if let Some(owner) = call.owner {
                        if !source.items[owner].polling {
                            source.items[owner].polling = true;
                            changed = true;
                        }
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    let polling: BTreeSet<String> = parsed
        .iter()
        .flat_map(|source| {
            source
                .items
                .iter()
                .filter(|item| {
                    item.polling && !marker_shared(source, item) && !marker_delegator(item)
                })
                .map(|item| item.short.clone())
        })
        .collect();
    let fs_polling: BTreeSet<String> = parsed
        .iter()
        .flat_map(|source| {
            source
                .items
                .iter()
                .filter(|item| {
                    item.polling
                        && item.has_probe
                        && !marker_shared(source, item)
                        && !marker_delegator(item)
                })
                .map(|item| item.short.clone())
        })
        .collect();
    let empty = BTreeSet::new();
    // Read-through helpers: a same-crate function whose path parameter (or
    // its `let` alias) reaches a read [0], a check inside a polling block
    // [1], or a lone check [2], to a fixed point over same-crate calls. The
    // shared wait is never such a helper. A call passing a marker to a read
    // or polling parameter, or to a checking one from inside a polling block,
    // is a site. Only a local marker name counts at such a call: a
    // crate-wide field also names containers (t-132's `.keys` folder holds
    // signing keys and intents beside its markers), so passing one is not
    // followed into a helper; direct reads and checks of fields still are.
    let mut by_name: BTreeMap<&str, Vec<(usize, usize)>> = BTreeMap::new();
    for (index, source) in parsed.iter().enumerate() {
        for (item_index, item) in source.items.iter().enumerate() {
            if !marker_shared(source, item) {
                by_name
                    .entry(item.short.as_str())
                    .or_default()
                    .push((index, item_index));
            }
        }
    }
    let aliases: Vec<Vec<Vec<BTreeSet<String>>>> = parsed
        .iter()
        .map(|source| {
            source
                .items
                .iter()
                .enumerate()
                .map(|(item_index, item)| {
                    let lets: Vec<&MarkerLet> = source
                        .lets
                        .iter()
                        .filter(|binding| {
                            binding.owner == item_index && binding.chain.call.is_none()
                        })
                        .collect();
                    item.params
                        .iter()
                        .map(|param| {
                            let mut names: BTreeSet<String> = param.iter().cloned().collect();
                            loop {
                                let before = names.len();
                                for binding in &lets {
                                    if binding.names.words.iter().any(|word| names.contains(word)) {
                                        names.insert(binding.name.clone());
                                    }
                                }
                                if names.len() == before {
                                    break names;
                                }
                            }
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    let offset_of = |source: &MarkerSource, call: &MarkerCall, item: &MarkerItem| {
        usize::from(
            item.params.first().and_then(Option::as_deref) == Some("self")
                && call.at > 0
                && marker_mark(&source.tokens, call.at - 1, '.'),
        )
    };
    let mut reach: Vec<Vec<[BTreeSet<usize>; 3]>> = parsed
        .iter()
        .map(|source| vec![Default::default(); source.items.len()])
        .collect();
    for (index, source) in parsed.iter().enumerate() {
        for probe in &source.probes {
            let Some(owner) = probe.owner else { continue };
            if marker_shared(source, &source.items[owner]) {
                continue;
            }
            let kind = if probe.read {
                0
            } else if probe.in_poll {
                1
            } else {
                2
            };
            for (param, names) in aliases[index][owner].iter().enumerate() {
                if probe.names.words.iter().any(|word| names.contains(word)) {
                    reach[index][owner][kind].insert(param);
                }
            }
        }
    }
    loop {
        let mut additions = Vec::new();
        for (index, source) in parsed.iter().enumerate() {
            for call in &source.calls {
                let Some(owner) = call.owner else { continue };
                if marker_shared(source, &source.items[owner]) {
                    continue;
                }
                for &(target, item_index) in by_name.get(call.name.as_str()).into_iter().flatten() {
                    let offset = offset_of(source, call, &parsed[target].items[item_index]);
                    for (kind, params) in reach[target][item_index].iter().enumerate() {
                        let kind = if kind == 2 && call.in_poll { 1 } else { kind };
                        for &param in params {
                            let Some(&(start, end)) =
                                param.checked_sub(offset).and_then(|arg| call.args.get(arg))
                            else {
                                continue;
                            };
                            let words = marker_names(&source.tokens, start, end).words;
                            for (own, names) in aliases[index][owner].iter().enumerate() {
                                if words.iter().any(|word| names.contains(word)) {
                                    additions.push((index, owner, kind, own));
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut changed = false;
        for (index, owner, kind, param) in additions {
            changed |= reach[index][owner][kind].insert(param);
        }
        if !changed {
            break;
        }
    }
    for source in &parsed {
        let item_at = |owner: Option<usize>| {
            owner
                .map(|owner| source.items[owner].name.clone())
                .unwrap_or_else(|| "(top level)".to_owned())
        };
        let closure_item =
            |call: &MarkerCall| format!("{}::{}#{}", item_at(call.owner), call.name, call.ordinal);
        let closure_poll = |call: &MarkerCall| {
            (call.name.starts_with("wait") || call.name.starts_with("await"))
                && polling.contains(&call.name)
                && call.args.iter().any(|&(start, end)| {
                    source
                        .probes
                        .iter()
                        .any(|probe| probe.in_poll && start <= probe.at && probe.at < end)
                })
        };
        for call in &source.calls {
            if call.name == "await_marker_line" {
                inventory.shared_calls += 1;
            }
            let Some(owner) = call.owner else { continue };
            if marker_shared(source, &source.items[owner]) {
                continue;
            }
            let has_closure_probe = inventory.first_key.is_some() && closure_poll(call);
            let local = &source.items[owner].names;
            let through_helper = by_name.get(call.name.as_str()).into_iter().flatten().any(
                |&(target, item_index)| {
                    let offset = offset_of(source, call, &parsed[target].items[item_index]);
                    reach[target][item_index]
                        .iter()
                        .enumerate()
                        .any(|(kind, params)| {
                            (kind < 2 || call.in_poll)
                                && params.iter().any(|&param| {
                                    param
                                        .checked_sub(offset)
                                        .and_then(|arg| call.args.get(arg))
                                        .is_some_and(|&(start, end)| {
                                            marker_names(&source.tokens, start, end)
                                                .words
                                                .iter()
                                                .any(|word| local.contains(word))
                                        })
                                })
                        })
                },
            );
            if has_closure_probe
                || through_helper
                || (fs_polling.contains(&call.name)
                    && call.args.iter().any(|&(start, end)| {
                        marker_mentions(&marker_names(&source.tokens, start, end), local, &fields)
                    }))
            {
                inventory.sites.insert((
                    source.path.clone(),
                    source.tokens[call.at].0,
                    closure_item(call),
                ));
            }
        }
        for probe in &source.probes {
            if probe
                .owner
                .is_some_and(|owner| marker_shared(source, &source.items[owner]))
            {
                continue;
            }
            let local = probe
                .owner
                .map(|owner| &source.items[owner].names)
                .unwrap_or(&empty);
            if (probe.read || probe.in_poll) && marker_mentions(&probe.names, local, &fields) {
                let closure = source
                    .calls
                    .iter()
                    .filter(|call| call.owner == probe.owner && closure_poll(call))
                    .find(|call| {
                        call.args
                            .iter()
                            .any(|&(start, end)| start <= probe.at && probe.at < end)
                    });
                let item = if let Some(closure) = closure {
                    closure_item(closure)
                } else if probe.read {
                    // A literal derived path gives a narrow, stable name
                    // when a marker container also holds non-marker files.
                    let call = source
                        .calls
                        .iter()
                        .find(|call| call.at == probe.at)
                        .unwrap();
                    let literal = call.args.first().and_then(|&(start, end)| {
                        source.tokens[start..end]
                            .iter()
                            .find_map(|(_, token)| match token {
                                Token::Text(text) => Some(text),
                                _ => None,
                            })
                    });
                    literal
                        .map(|text| format!("{}::{}[{text}]", item_at(probe.owner), call.name))
                        .unwrap_or_else(|| item_at(probe.owner))
                } else {
                    item_at(probe.owner)
                };
                inventory
                    .sites
                    .insert((source.path.clone(), source.tokens[probe.at].0, item));
            }
        }
        if inventory.first_key.is_some() {
            for item in &source.items {
                if (item.short.starts_with("wait") || item.short.starts_with("await"))
                    && item.polling
                    && item.has_probe
                    && !marker_shared(source, item)
                    && !marker_delegator(item)
                {
                    inventory.sites.insert((
                        source.path.clone(),
                        source.tokens[item.declaration].0,
                        item.name.clone(),
                    ));
                }
            }
        }
    }
    // Coverage floor: the path of every shared-wait call must be a marker the
    // recogniser above sees - directly, or through a wrapper parameter at
    // each of the wrapper's callers (reported at the caller that passes an
    // unrecognised path, so one wrapper row cannot hide a new caller).
    let mut queue: Vec<(usize, usize, usize)> = Vec::new();
    for (index, source) in parsed.iter().enumerate() {
        for (number, call) in source.calls.iter().enumerate() {
            if call.name == "await_marker_line"
                && call
                    .owner
                    .is_some_and(|owner| !marker_shared(source, &source.items[owner]))
            {
                queue.push((index, number, 0));
            }
        }
    }
    let mut visited = BTreeSet::new();
    while let Some((index, number, argument)) = queue.pop() {
        if !visited.insert((index, number, argument)) {
            continue;
        }
        let source = &parsed[index];
        let call = &source.calls[number];
        let Some(owner) = call.owner else { continue };
        let item = &source.items[owner];
        let unkeyed = (
            source.path.clone(),
            source.tokens[call.at].0,
            format!("{}::{}#{}", item.name, call.name, call.ordinal),
        );
        let Some(&(start, end)) = call.args.get(argument) else {
            inventory.unkeyed.insert(unkeyed);
            continue;
        };
        let names = marker_names(&source.tokens, start, end);
        if marker_mentions(&names, &item.names, &fields) {
            continue;
        }
        let params: Vec<usize> = aliases[index][owner]
            .iter()
            .enumerate()
            .filter(|(_, aliases)| names.words.iter().any(|word| aliases.contains(word)))
            .map(|(param, _)| param)
            .collect();
        if params.is_empty() {
            inventory.unkeyed.insert(unkeyed);
            continue;
        }
        for (caller_index, caller) in parsed.iter().enumerate() {
            for (caller_number, wrapper_call) in caller.calls.iter().enumerate() {
                if wrapper_call.name != item.short || wrapper_call.owner.is_none() {
                    continue;
                }
                let offset = offset_of(caller, wrapper_call, item);
                for &param in &params {
                    if let Some(position) = param.checked_sub(offset) {
                        queue.push((caller_index, caller_number, position));
                    }
                }
            }
        }
    }
    inventory
}

/// Each source is synthetic code, not a runtime fixture: the exact sites
/// must match before the real inventory can make any claim. Strings and
/// comments are opaque; source locations are 1-based line plus item.
fn marker_controls() {
    // (f) Four indirect settings, each beside its shared wait: a struct
    // field filled from a marker, a path helper's result reused by
    // position, a name relayed to a process that re-sets it as a key, and
    // `format!("{selector}_MARKER")` with a `RATMAC_TEST_` selector.
    const STRUCT_FIELD: &str = "fn paused() -> Held {\nlet marker = dir.join(\"m\");\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nHeld { child: Some(child), marker, release }\n}\nfn a(held: &Held) {\nawait_marker_line(&held.marker, expected, limit);\nwhile !held.marker.exists() {}\n}\n";
    const POSITION: &str = "fn paths(&self) -> (PathBuf, PathBuf) {\n(self.dir.join(\"m\"), self.dir.join(\"r\"))\n}\nfn arm(&self) {\nlet (marker, release) = self.paths();\ncmd.env(\"RATMAC_TEST_LOCK_MARKER\", &marker);\n}\nfn a(fx: &Fx) {\nlet marker = fx.paths().0;\nawait_marker_line(&marker, expected, limit);\nwhile !marker.exists() {}\n}\n";
    const RELAY: &str = "fn child() {\nlet marker = role_path(\"RATMAC_PRIVATE_MARKER\");\nstd::env::set_var(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\n}\nfn a() {\ncmd.env(\"RATMAC_PRIVATE_MARKER\", &relayed);\nawait_marker_line(&relayed, expected, limit);\nwhile !relayed.exists() {}\n}\n";
    const SELECTOR: &str = "fn arm(command: &mut Command, selector: &str, marker: &Path) {\ncommand.env(format!(\"{selector}_MARKER\"), marker);\n}\nfn a() {\narm(&mut command, \"RATMAC_TEST_TURN_PAUSE\", &marker);\nawait_marker_line(&marker, expected, limit);\nwhile !marker.exists() {}\n}\n";
    let controls: &[(&str, &[(usize, &str)])] = &[
        (
            "fn a() {\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nwhile !marker.exists() {}\n}\n",
            &[(3, "a")],
        ),
        (
            "fn a() {\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nawait_marker_line(&marker, expected, limit);\nlet bytes = fs::read(&marker);\n}\n",
            &[(4, "a")],
        ),
        (
            "fn a() {\nlet envs = [(\"RATMAC_TEST_PAUSE_MARKER\".to_owned(), raw(marker))];\nloop { if marker.is_file() { break; } thread::sleep(limit); }\n}\n",
            &[(3, "a")],
        ),
        (
            "const PAUSE_MARKER_VAR: &str = \"RATMAC_TEST_PAUSE_MARKER\";\nfn a() {\ncmd.env(PAUSE_MARKER_VAR, &m);\nfs::read_to_string(&m);\n}\n",
            &[(4, "a")],
        ),
        (
            "fn set(path: &Path) { cmd.env(\"RATMAC_TEST_PAUSE_MARKER\", raw(path)); }\nfn wrap(marker: &Path) { set(marker); }\nfn a() {\nwrap(&local);\nFile::open(&local);\n}\n",
            &[(5, "a")],
        ),
        (
            "fn await_own(marker: &Path) { support::await_marker_line(marker, expected, limit) }\nfn a() { cmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker); await_own(&marker); }\n",
            &[],
        ),
        (
            "fn a() { cmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker); child.wait(); assert!(!marker.exists()); }\n",
            &[],
        ),
        (
            "// cmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker); while marker.exists() {}\nfn a() { let text = r#\"cmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker); fs::read(marker)\"#; }\n/* fs::read(marker); */\n",
            &[],
        ),
        (
            "fn wait_file(path: &Path) { loop { if path.is_file() { break; } thread::yield_now(); } }\nfn a() { cmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker); wait_file(&marker); }\n",
            &[(1, "wait_file"), (2, "a::wait_file#1")],
        ),
        (
            "fn wait_until(predicate: impl FnMut() -> bool) { loop { if predicate() { break; } thread::yield_now(); } }\nfn wait_file(path: &Path) { wait_until(|| path.exists()); }\nfn a() { cmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker); wait_file(&marker); }\n",
            &[(2, "wait_file"), (2, "wait_file::wait_until#1"), (3, "a::wait_file#1")],
        ),
        (
            "fn a() { cmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &hold.marker); }\nfn b() { loop { if other.marker.try_exists() {} } }\nfn c() { while other.release.exists() {} }\n",
            &[(2, "b")],
        ),
        (
            "fn a() { cmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker); }\nfn b() { while marker.exists() {} }\n",
            &[],
        ),
        (
            "fn wait_file(path: &Path) { loop { fs::metadata(path); thread::sleep(limit); } }\nfn a() { cmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker); wait_file(traced.hold_paths(\"hold\").0); }\n",
            &[(1, "wait_file")],
        ),
        // A generic polling function's filesystem closure is a named site
        // even when its marker parameter never enters a RATMAC_TEST setting.
        (
            "fn wait_until(predicate: impl FnMut() -> bool) { loop { if predicate() { break; } thread::sleep(limit); } }\nfn setting() { cmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker); }\nfn park_external_holder(marker: &Path) { wait_until(|| marker.is_file()); }\n",
            &[(3, "park_external_holder::wait_until#1")],
        ),
        (
            "fn a() { cmd.env(\"RATMAC_TEST_PAUSE_MARKER\", marker_path.as_os_str());\nlet callback = move || { marker_path.metadata(); };\n}\n",
            &[(2, "a")],
        ),
        // (a) A bounded `for` poll waits like `loop` and `while`; a `for`
        // over post-exit checks that never pauses or breaks is not a wait.
        (
            "fn a() {\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nfor _ in 0..50 { if marker.exists() { break; } thread::sleep(limit); }\n}\n",
            &[(3, "a")],
        ),
        (
            "fn a() { cmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker); child.wait(); for what in [\"x\"] { assert!(!marker.exists(), \"{what}\"); } }\n",
            &[],
        ),
        // (b) Marker names carry through `let` aliases to a fixed point: the
        // same file (`&`, `clone`, `PathBuf::from`, `to_path_buf`, `as_path`)
        // both ways, a path below it (`join`) only forwards.
        (
            "fn a() {\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nlet seen = marker.clone();\nwhile !seen.exists() { thread::sleep(limit); }\n}\n",
            &[(4, "a")],
        ),
        (
            "fn a() {\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", marker.as_os_str());\nlet first = &marker;\nlet second = PathBuf::from(first).to_path_buf();\nlet bytes = fs::read(second.as_path());\n}\n",
            &[(5, "a")],
        ),
        (
            "fn a() {\nlet marker = root.join(\"pause.marker\");\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nlet nested = marker.join(\"\");\nwhile !root.exists() {}\nwhile !nested.is_file() {}\n}\n",
            &[(6, "a")],
        ),
        (
            "fn arm(cmd: &mut Command, marker: &Path) { let owned = marker.to_path_buf(); cmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &owned); }\nfn a() {\narm(&mut cmd, &m);\nlet bytes = fs::read_to_string(&m);\n}\n",
            &[(4, "a")],
        ),
        // (c) A same-crate helper whose path parameter reaches a read, or a
        // check inside a poll, makes the call that passes the marker the
        // site; a lone post-exit check through a helper stays allowed.
        (
            "fn slurp(path: &Path) -> Vec<u8> { fs::read(path).unwrap_or_default() }\nfn a() {\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nloop { if slurp(&marker).ends_with(b\"\\n\") { break; } thread::sleep(limit); }\n}\n",
            &[(4, "a::slurp#1")],
        ),
        (
            "fn slurp(path: &Path) -> Vec<u8> { fs::read(path).unwrap_or_default() }\nfn a() {\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nchild.wait();\nlet bytes = slurp(&marker);\n}\n",
            &[(5, "a::slurp#1")],
        ),
        (
            "fn slurp(path: &Path) -> Vec<u8> { fs::read(path).unwrap_or_default() }\nfn wait_line(marker: &Path) -> Vec<u8> { loop { let bytes = slurp(marker); if bytes.ends_with(b\"\\n\") { return bytes; } thread::sleep(limit); } }\nfn a() {\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nwait_line(&marker);\n}\n",
            &[(5, "a::wait_line#1")],
        ),
        (
            "fn present(path: &Path) -> bool { path.is_file() }\nfn a() {\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nwhile !present(&marker) { thread::sleep(limit); }\nchild.wait();\nassert!(present(&marker));\n}\n",
            &[(4, "a::present#1")],
        ),
        // (d) Every std access form of the marker path.
        (
            "fn a() {\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nlet file = OpenOptions::new().read(true).open(&marker);\n}\n",
            &[(3, "a")],
        ),
        (
            "fn a() {\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nlet file = std::fs::File::open(&marker);\nlet meta = fs::metadata(&marker);\n}\n",
            &[(3, "a"), (4, "a")],
        ),
        (
            "fn a() {\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nwhile !Path::exists(&marker) { thread::sleep(limit); }\n}\n",
            &[(3, "a")],
        ),
        (
            "fn a() {\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nlet link = fs::symlink_metadata(&marker);\nloop { if marker.symlink_metadata().is_ok() { break; } }\n}\n",
            &[(3, "a"), (4, "a")],
        ),
        // (e) Keys held in a `static` or built by `format!` with the key's
        // literal shape, and a helper that sets the key on its own local
        // and returns that path.
        (
            "static PAUSE_KEY: &str = \"RATMAC_TEST_PAUSE_MARKER\";\nfn a() {\ncmd.env(PAUSE_KEY, &marker);\nwhile !marker.exists() {}\n}\n",
            &[(4, "a")],
        ),
        (
            "fn a() {\ncmd.env(format!(\"RATMAC_TEST_{}_MARKER\", kind), &marker);\nwhile !marker.exists() {}\n}\n",
            &[(3, "a")],
        ),
        (
            "fn arm(cmd: &mut Command, dir: &Path) -> PathBuf {\nlet marker = dir.join(\"pause.marker\");\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nmarker\n}\nfn a() {\nlet seen = arm(&mut cmd, &dir);\nwhile !seen.exists() {}\n}\n",
            &[(8, "a")],
        ),
        (STRUCT_FIELD, &[(8, "a")]),
        (POSITION, &[(11, "a")]),
        (RELAY, &[(8, "a")]),
        (SELECTOR, &[(7, "a")]),
    ];
    let mut misread = Vec::new();
    for (source, expected) in controls {
        let sources = BTreeMap::from([("control.rs".to_owned(), (*source).to_owned())]);
        let actual: Vec<(usize, String)> = marker_inventory(&sources)
            .sites
            .into_iter()
            .map(|(_, line, item)| (line, item))
            .collect();
        let expected: Vec<(usize, String)> = expected
            .iter()
            .map(|(line, item)| (*line, (*item).to_owned()))
            .collect();
        if actual != expected {
            misread.push(format!(
                "{source}  expected {expected:?}\n  found    {actual:?}"
            ));
        }
    }
    // Coverage floor: every shared-wait call's path is a recognised marker,
    // directly or through a wrapper parameter at each caller; otherwise the
    // call (or that caller) is reported. The four indirect shapes are
    // recognised, so a raw wait beside their shared waits cannot hide.
    let floor: &[(&str, &[(usize, &str)])] = &[
        (STRUCT_FIELD, &[]),
        (POSITION, &[]),
        (RELAY, &[]),
        (SELECTOR, &[]),
        (
            "fn a() {\nawait_marker_line(&scratch, expected, limit);\n}\n",
            &[(2, "a::await_marker_line#1")],
        ),
        (
            "fn w(path: &Path) {\nawait_marker_line(path, expected, limit);\n}\nfn a() {\ncmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker);\nw(&marker);\nw(&other);\n}\n",
            &[(7, "a::w#2")],
        ),
    ];
    for (source, expected) in floor {
        let sources = BTreeMap::from([("control.rs".to_owned(), (*source).to_owned())]);
        let actual: Vec<(usize, String)> = marker_inventory(&sources)
            .unkeyed
            .into_iter()
            .map(|(_, line, call)| (line, call))
            .collect();
        let expected: Vec<(usize, String)> = expected
            .iter()
            .map(|(line, call)| (*line, (*call).to_owned()))
            .collect();
        if actual != expected {
            misread.push(format!(
                "{source}  expected unrecognised {expected:?}\n  found    {actual:?}"
            ));
        }
    }
    assert!(
        misread.is_empty(),
        "SVEV-015 control: the marker matcher misread {} of {} sources:\n{}",
        misread.len(),
        controls.len() + floor.len(),
        misread.join("\n")
    );
    // Source scope is the crate, not the file: parameter propagation must
    // cross from a lane's src/lib.rs helper into its tests/hidden.rs caller.
    let sources = BTreeMap::from([
        (
            "src/lib.rs".to_owned(),
            "fn settings(marker: &Path) { hold_env(\"RATMAC_TEST_LOCK_MARKER\", marker); }"
                .to_owned(),
        ),
        (
            "tests/hidden.rs".to_owned(),
            "fn a() {\nsettings(&my_marker);\nlet _ = fs::read(&my_marker);\n}".to_owned(),
        ),
    ]);
    assert_eq!(
        marker_inventory(&sources).sites,
        BTreeSet::from([("tests/hidden.rs".to_owned(), 3, "a".to_owned())]),
        "SVEV-015 control: marker parameters propagate across files in one crate"
    );
    // Only the shared implementation is exempt from its own filesystem
    // reads (and a call through it is no read-through helper), not another
    // locally named copy in a lane.
    let source = "fn await_marker_line(path: &Path) { loop { fs::read(path); thread::sleep(limit); } }\nfn setting() { cmd.env(\"RATMAC_TEST_PAUSE_MARKER\", &marker); await_marker_line(&marker); }";
    assert!(
        marker_inventory(&BTreeMap::from([(
            "test/qa/src/support.rs".to_owned(),
            source.to_owned()
        )]))
        .sites
        .is_empty(),
        "SVEV-015 control: the shared wait itself is not a bypass"
    );
    assert!(
        !marker_inventory(&BTreeMap::from([(
            "test-hidden/t-999/src/lib.rs".to_owned(),
            source.to_owned()
        )]))
        .sites
        .is_empty(),
        "SVEV-015 control: a lane-local copy of the shared wait is a bypass"
    );
    let inventory = marker_inventory(&BTreeMap::from([("control.rs".to_owned(),
        "fn a() { env_set(\"RATMAC_TEST_PAUSE_MARKER\", &marker); support::await_marker_line(&marker, expected, limit); }".to_owned())]));
    assert!(
        inventory.first_key.is_some() && inventory.shared_calls == 1,
        "SVEV-015 control: a setting plus a shared call satisfies the crate floor"
    );
    let inventory = marker_inventory(&BTreeMap::from([("control.rs".to_owned(),
        "fn await_marker_line() {}\nfn a() { let text = r#\"env_set(\"RATMAC_TEST_PAUSE_MARKER\", &marker); await_marker_line(marker);\"#; }\n// await_marker_line(marker);".to_owned())]));
    assert!(
        inventory.first_key.is_none() && inventory.shared_calls == 0,
        "SVEV-015 control: declarations, comments, and strings do not satisfy the crate floor"
    );
}
