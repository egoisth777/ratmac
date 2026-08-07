//! t-076 / ENS-008: named repository roots for path-bearing guards.
//!
//! ENSV-009 `roots_table_validates_named_paths_with_distinct_diagnostics`
//!
//! One temporary Git repository supplies an ordinary artifact tree, a
//! deliberately conflicting workspace-level path, and its resolved Engine
//! root. The valid run proves that `files_exact` reads beneath its named root;
//! the three invalid variants must be diagnosed before they mint another
//! Run.

use ratmac_qa::json::Json;
use ratmac_qa::tempgit::TempRepo;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const LEGACY_WORKFLOW_LITERAL: &[u8] = b".arca";
const LEGACY_WORKFLOW_EXCEPTION_PATH: &str = "scheduler.rs";
const LEGACY_WORKFLOW_EXCEPTION_DECLARATION: &str = r#"const LEGACY_WORKFLOW_DIR: &str = ".arca";"#;

struct Fixture {
    repo: TempRepo,
}

impl Fixture {
    fn new() -> Self {
        let repo = TempRepo::new("t076-roots-table");
        repo.write("artifacts/release/proof.txt", "named-root evidence\n");
        // This sentinel makes a workspace-root lookup observably different
        // from the declared `artifacts/` root.
        repo.write("release/wrong.txt", "wrong root\n");
        repo.write(".ratmac/ratmac.toml", &runbook("artifacts", "work"));
        Self { repo }
    }

    fn root(&self) -> &Path {
        self.repo.root()
    }

    fn write_runbook(&self, source: &str) {
        self.repo.write(".ratmac/ratmac.toml", source);
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .args(args)
            .current_dir(self.root())
            .output()
            .expect("invoke compiled rtm binary")
    }

    fn run_ids(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(self.root().join(".ratmac/runs")) else {
            return Vec::new();
        };
        let mut ids = entries
            .map(|entry| entry.expect("read Engine Run roster entry"))
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }
}

fn runbook(declared_path: &str, guard_root: &str) -> String {
    format!(
        r#"[roots]
work = "{declared_path}"

[phases.verify]
prompt = "Verify the named root."
guards = [{{ kind = "files_exact", root = "{guard_root}", path = "release", entries = ["proof.txt"] }}]

[phases.done]
prompt = "Named-root guard passed."

[[transitions]]
from = "verify"
to = "done"
"#
    )
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn doctor_codes(report: &str) -> Vec<String> {
    let json = Json::parse(report).unwrap_or_else(|error| panic!("doctor JSON: {error}\n{report}"));
    json.as_object()
        .and_then(|object| object.get("findings"))
        .and_then(Json::as_array)
        .expect("doctor JSON has a findings array")
        .iter()
        .map(|finding| {
            finding
                .field("code")
                .expect("doctor finding has a code")
                .to_owned()
        })
        .collect()
}

fn assert_static_refusal(
    fixture: &Fixture,
    source: &str,
    code: &str,
    role: &str,
    runs_before: &[String],
) {
    fixture.write_runbook(source);

    let doctor = fixture.rtm(&["doctor", "--json"]);
    let report = String::from_utf8_lossy(&doctor.stdout).into_owned();
    assert!(
        !doctor.status.success(),
        "{code} must make doctor refuse: {report}"
    );
    assert!(
        doctor_codes(&report).iter().any(|found| found == code),
        "doctor must report {code}: {report}"
    );
    assert!(
        report.contains(role),
        "doctor's {code} finding must name root {role:?}: {report}"
    );

    let start = fixture.rtm(&["start"]);
    let refusal = combined(&start);
    assert!(
        !start.status.success(),
        "{code} must refuse start: {refusal}"
    );
    assert!(
        refusal.contains(code),
        "start refusal must include {code}: {refusal}"
    );
    assert!(
        refusal.contains(role),
        "start refusal must name root {role:?}: {refusal}"
    );
    let runs_after = fixture.run_ids();
    assert_eq!(
        runs_after.as_slice(),
        runs_before,
        "{code} must not mint a Run"
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn collect_source_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read Engine source directory") {
        let path = entry.expect("read Engine source entry").path();
        let metadata = fs::symlink_metadata(&path).expect("read Engine source metadata");
        if metadata.is_dir() {
            collect_source_files(&path, files);
        } else if metadata.is_file() {
            files.push(path);
        }
    }
}

fn assert_legacy_workflow_literal_exception() {
    let source =
        fs::canonicalize(repo_root().join("src")).expect("canonicalize Engine source directory");
    let mut files = Vec::new();
    collect_source_files(&source, &mut files);
    files.sort();

    let mut occurrences = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(&source)
            .expect("Engine source file remains under src")
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&path).expect("read Engine source file");
        for (index, line) in text.lines().enumerate() {
            let count = line
                .as_bytes()
                .windows(LEGACY_WORKFLOW_LITERAL.len())
                .filter(|window| *window == LEGACY_WORKFLOW_LITERAL)
                .count();
            for _ in 0..count {
                occurrences.push((relative.clone(), index + 1, line.to_owned()));
            }
        }
    }

    assert_eq!(
        occurrences.len(),
        1,
        "the source audit permits exactly one ENS-009 legacy-literal exception; \
         a second literal is forbidden and a missing literal means its named exception vanished: \
         {occurrences:?}"
    );
    let (path, line, declaration) = occurrences
        .pop()
        .expect("the exact named exception is required");
    assert_eq!(
        path, LEGACY_WORKFLOW_EXCEPTION_PATH,
        "the sole legacy literal must stay in the named exception source line {line}: {declaration}"
    );
    assert_eq!(
        declaration.trim(),
        LEGACY_WORKFLOW_EXCEPTION_DECLARATION,
        "the sole legacy literal must be the named exception declaration at {path}:{line}"
    );
}

/// ENSV-009: a declared root routes its guard below that repository-relative
/// path; each named root defect is statically distinct and leaves no new Run.
#[test]
fn roots_table_validates_named_paths_with_distinct_diagnostics() {
    let fixture = Fixture::new();

    let valid_start = fixture.rtm(&["start"]);
    assert!(
        valid_start.status.success(),
        "a declared named root must let the Run start: {}",
        combined(&valid_start)
    );
    let mut valid_runs = fixture.run_ids();
    assert_eq!(
        valid_runs.len(),
        1,
        "the valid named-root Run must appear on the roster"
    );
    let valid_run = valid_runs.pop().expect("the valid Run has an id");
    let step = fixture.rtm(&["step", "--run", &valid_run]);
    assert!(
        step.status.success(),
        "files_exact must evaluate under artifacts/release: {}",
        combined(&step)
    );
    let status = fixture.rtm(&["status", "--run", &valid_run]);
    let visible_status = combined(&status);
    assert!(
        status.status.success() && visible_status.contains("Named-root guard passed."),
        "the named-root guard must reach its ordinary verdict: {visible_status}"
    );

    let runs_before_invalid = fixture.run_ids();
    assert_static_refusal(
        &fixture,
        &runbook("artifacts", "undeclared"),
        "RB602",
        "undeclared",
        &runs_before_invalid,
    );
    assert_static_refusal(
        &fixture,
        &runbook("no-such-directory", "work"),
        "RB603",
        "work",
        &runs_before_invalid,
    );
    assert_static_refusal(
        &fixture,
        &runbook(".ratmac", "work"),
        "RB604",
        "work",
        &runs_before_invalid,
    );

    assert_legacy_workflow_literal_exception();
}
