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
/// NRR-002: the exception is permanent for as long as `ENS-009` stands, so the
/// declaration must say who owns it. This is the clause the scan requires in
/// the doc comment directly above the declaration.
const LEGACY_WORKFLOW_EXCEPTION_OWNER: &str = "owner: the ENS-009 pre-split residue refusal";

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
        Command::new(ratmac_qa::engine_bin!())
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

[states.verify]
prompt = "Verify the named root."
guards = [{{ kind = "files_exact", root = "{guard_root}", path = "release", entries = ["proof.txt"] }}]

[states.done]
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

/// Which clause of NRR-002 a source tree breaks. Each variant is a separate
/// sentence in the requirement, so the scan names the one it caught.
#[derive(Debug, Eq, PartialEq)]
enum ExceptionDefect {
    /// More than one literal, or none at all.
    LiteralCount(Vec<(String, usize, String)>),
    /// The one literal moved out of the file the exception names.
    WrongFile { path: String, line: usize },
    /// The declaration itself was renamed or reshaped.
    Renamed {
        path: String,
        line: usize,
        found: String,
    },
    /// The declaration no longer says who owns it.
    OwnerClauseMissing { path: String, line: usize },
}

impl ExceptionDefect {
    fn clause(&self) -> &'static str {
        match self {
            Self::LiteralCount(_) => "exactly one literal",
            Self::WrongFile { .. } => "the named file",
            Self::Renamed { .. } => "the named declaration",
            Self::OwnerClauseMissing { .. } => "the owner clause",
        }
    }
}

/// NRRV-003: the whole rule, applied to any source tree, so the same code that
/// judges the shipped Engine can judge a deliberately damaged copy of it.
fn scan_legacy_workflow_exception(source: &Path) -> Result<(), ExceptionDefect> {
    let mut files = Vec::new();
    collect_source_files(source, &mut files);
    files.sort();

    let mut occurrences: Vec<(String, usize, String, Vec<String>)> = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(source)
            .expect("Engine source file remains under the scanned root")
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&path).expect("read Engine source file");
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let count = line
                .as_bytes()
                .windows(LEGACY_WORKFLOW_LITERAL.len())
                .filter(|window| *window == LEGACY_WORKFLOW_LITERAL)
                .count();
            if count == 0 {
                continue;
            }
            // The doc comment directly above the declaration, nearest line first.
            let mut comment = Vec::new();
            for above in lines[..index].iter().rev() {
                let trimmed = above.trim_start();
                if trimmed.starts_with("///") || trimmed.starts_with("//") {
                    comment.push(trimmed.to_owned());
                } else if trimmed.is_empty() && comment.is_empty() {
                    continue;
                } else {
                    break;
                }
            }
            for _ in 0..count {
                occurrences.push((
                    relative.clone(),
                    index + 1,
                    (*line).to_owned(),
                    comment.clone(),
                ));
            }
        }
    }

    if occurrences.len() != 1 {
        return Err(ExceptionDefect::LiteralCount(
            occurrences
                .into_iter()
                .map(|(path, line, text, _)| (path, line, text))
                .collect(),
        ));
    }
    let (path, line, declaration, comment) = occurrences.pop().expect("one occurrence remains");
    if path != LEGACY_WORKFLOW_EXCEPTION_PATH {
        return Err(ExceptionDefect::WrongFile { path, line });
    }
    if declaration.trim() != LEGACY_WORKFLOW_EXCEPTION_DECLARATION {
        return Err(ExceptionDefect::Renamed {
            path,
            line,
            found: declaration.trim().to_owned(),
        });
    }
    if !comment
        .iter()
        .any(|entry| entry.contains(LEGACY_WORKFLOW_EXCEPTION_OWNER))
    {
        return Err(ExceptionDefect::OwnerClauseMissing { path, line });
    }
    Ok(())
}

fn assert_legacy_workflow_literal_exception() {
    let source =
        fs::canonicalize(repo_root().join("src")).expect("canonicalize Engine source directory");
    if let Err(defect) = scan_legacy_workflow_exception(&source) {
        panic!(
            "the shipped Engine source must satisfy the one named ENS-009 exception; \
             {} is broken: {defect:?}",
            defect.clause()
        );
    }
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

/// Copy a source tree into a fresh temporary directory so a mutation can be
/// made against real Engine source without touching the repository.
fn copy_source_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create mutated source directory");
    for entry in fs::read_dir(source).expect("read source directory") {
        let entry = entry.expect("read source entry");
        let target = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(entry.path()).expect("read source metadata");
        if metadata.is_dir() {
            copy_source_tree(&entry.path(), &target);
        } else if metadata.is_file() {
            fs::copy(entry.path(), &target).expect("copy source file");
        }
    }
}

fn mutated_source(label: &str, mutate: impl Fn(&Path)) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ratmac-nrrv003-{label}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    if root.exists() {
        fs::remove_dir_all(&root).expect("clear mutated source directory");
    }
    let source =
        fs::canonicalize(repo_root().join("src")).expect("canonicalize Engine source directory");
    copy_source_tree(&source, &root);
    mutate(&root);
    root
}

/// NRRV-003: Engine source names the retired pre-split folder exactly once, in
/// one declaration that carries its own name and owner, and the check that
/// proves it fails on a second literal, on a rename, and on a lost owner.
#[test]
fn source_scan_pins_the_one_named_legacy_exception() {
    let shipped =
        fs::canonicalize(repo_root().join("src")).expect("canonicalize Engine source directory");
    assert_eq!(
        scan_legacy_workflow_exception(&shipped),
        Ok(()),
        "the shipped Engine source must carry the one named exception, owner clause included"
    );

    let second_literal = mutated_source("second-literal", |root| {
        let path = root.join("root.rs");
        let mut text = fs::read_to_string(&path).expect("read the file that gains a literal");
        text.push_str("\nconst SNEAKED_BACK: &str = \".arca/ticket\";\n");
        fs::write(&path, text).expect("write the second literal");
    });
    let defect = scan_legacy_workflow_exception(&second_literal)
        .expect_err("a second literal must fail the scan");
    assert_eq!(
        defect.clause(),
        "exactly one literal",
        "a second literal must be reported as the count clause: {defect:?}"
    );
    fs::remove_dir_all(&second_literal).expect("clear the second-literal tree");

    let renamed = mutated_source("renamed", |root| {
        let path = root.join(LEGACY_WORKFLOW_EXCEPTION_PATH);
        let text = fs::read_to_string(&path).expect("read the declaration file");
        let renamed = text.replace("LEGACY_WORKFLOW_DIR", "OLD_WORKFLOW_DIR");
        assert_ne!(renamed, text, "the rename must change the declaration");
        fs::write(&path, renamed).expect("write the renamed declaration");
    });
    let defect =
        scan_legacy_workflow_exception(&renamed).expect_err("a renamed declaration must fail");
    assert_eq!(
        defect.clause(),
        "the named declaration",
        "a rename must be reported as the declaration clause: {defect:?}"
    );
    fs::remove_dir_all(&renamed).expect("clear the renamed tree");

    let ownerless = mutated_source("ownerless", |root| {
        let path = root.join(LEGACY_WORKFLOW_EXCEPTION_PATH);
        let text = fs::read_to_string(&path).expect("read the declaration file");
        let stripped: Vec<&str> = text
            .lines()
            .filter(|line| !line.contains(LEGACY_WORKFLOW_EXCEPTION_OWNER))
            .collect();
        assert_ne!(
            stripped.len(),
            text.lines().count(),
            "the owner clause must exist before it can be deleted"
        );
        fs::write(&path, format!("{}\n", stripped.join("\n")))
            .expect("write the ownerless declaration");
    });
    let defect =
        scan_legacy_workflow_exception(&ownerless).expect_err("a lost owner clause must fail");
    assert_eq!(
        defect.clause(),
        "the owner clause",
        "a deleted owner clause must be reported as the owner clause: {defect:?}"
    );
    fs::remove_dir_all(&ownerless).expect("clear the ownerless tree");
}
