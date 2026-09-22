//! WCP-002: preserve lane proof while isolating the support each lane uses.

use ratmac_qa::baseline::{self, scenario};
use ratmac_qa::json::Json;
use ratmac_qa::tempgit::TempRepo;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
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
            let bytes = fs::read(root.join(path))
                .unwrap_or_else(|error| panic!("preserved artifact {path}: {error}"));
            assert_eq!(
                digest(&bytes),
                file["sha256"].as_str().unwrap(),
                "historical artifact {path} remains byte-identical"
            );
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
    preserved_fault("before-verdict-archive");
    preserved_fault("before-state-replace");
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
