//! WCP-002: preserve lane proof while isolating the support each lane uses.

use ratmac_qa::baseline::{self, scenario};
use ratmac_qa::json::Json;
use ratmac_qa::lane_runs;
use ratmac_qa::tempgit::TempRepo;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(0);
const CHILD_MODE: &str = "RATMAC_T126_PUBLIC_CHILD";

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "ratmac-t126-{label}-{}-{stamp}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("create owned fixture directory");
        Self(root)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
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
const EXCEPTIONS: [Exception; 10] = [
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
