//! t-039 / AOI-002: archive-aware history-preservation oracle.
//!
//! PT-039-03 `complete_authorized_move_is_preservation`
//! PT-039-04 `mutation_partial_and_inplace_edits_fail`
//! PT-039-06 `schema_states_archive_and_snapshot_rules`
//! HT-039-02 `line_ending_normalization_is_not_mutation`
//!
//! A completed issue folder may move to `.arca/issue/archive/<issue-id>/`
//! with identity, five-file shape, and content preserved except relative-link
//! depth rewrites. Mutation, partial moves, in-place edits, and moves of
//! non-completed issues remain failures.

use ratmac_qa::archive::verify_history_preservation;
use ratmac_qa::tempgit::TempRepo;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FILES: [&str; 5] = [
    "index.md",
    "spec.md",
    "design.md",
    "test-plan.md",
    "ubi-lang.md",
];

fn issue_body(issue_id: &str, name: &str, status: &str) -> String {
    if name == "index.md" {
        format!(
            "# {issue_id}\n\n```yaml\nissue-id: \"{issue_id}\"\nstatus: \"{status}\"\n```\n\n\
             See [goal spec](../../goal/spec.md).\n"
        )
    } else {
        format!("# {name} for {issue_id}\n\nSee [goal spec](../../goal/spec.md).\n")
    }
}

/// Seed a repository with one issue folder under `.arca/issue/<issue-id>/`.
fn seed_issue(repo: &TempRepo, issue_id: &str, status: &str) {
    for name in FILES {
        repo.write(
            &format!(".arca/issue/{issue_id}/{name}"),
            &issue_body(issue_id, name, status),
        );
    }
}

/// Perform the authorized archive move in the working tree: relocate every
/// file and deepen relative links by one level.
fn archive_move(repo: &TempRepo, issue_id: &str, skip: Option<&str>) {
    for name in FILES {
        if Some(name) == skip {
            continue;
        }
        let from = repo.root().join(format!(".arca/issue/{issue_id}/{name}"));
        let content = fs::read_to_string(&from).expect("read issue file");
        let rewritten = content.replace("](../../", "](../../../");
        let to = repo
            .root()
            .join(format!(".arca/issue/archive/{issue_id}/{name}"));
        fs::create_dir_all(to.parent().expect("archive parent")).expect("create archive dir");
        fs::write(&to, rewritten).expect("write archived file");
        fs::remove_file(&from).expect("remove original");
    }
}

fn history_roots() -> Vec<&'static str> {
    vec![".arca/issue"]
}

#[test]
fn complete_authorized_move_is_preservation() {
    let repo = TempRepo::new("archive-move-pass");
    seed_issue(&repo, "i-001-example", "integrated");
    repo.commit_all("seed issue");

    archive_move(&repo, "i-001-example", None);

    verify_history_preservation(repo.root(), &history_roots())
        .expect("a complete authorized archive move is preservation");
}

#[test]
fn mutation_partial_and_inplace_edits_fail() {
    // (a) One byte of preserved content altered during the move.
    let repo = TempRepo::new("archive-move-mutated");
    seed_issue(&repo, "i-002-example", "integrated");
    repo.commit_all("seed issue");
    archive_move(&repo, "i-002-example", None);
    let mutated = repo
        .root()
        .join(".arca/issue/archive/i-002-example/spec.md");
    let content = fs::read_to_string(&mutated).expect("read archived spec");
    fs::write(&mutated, content.replace("# spec.md", "# spec.md!")).expect("mutate archived spec");

    let violations = verify_history_preservation(repo.root(), &history_roots())
        .expect_err("content mutation must fail");
    assert!(
        violations
            .iter()
            .any(|v| v.path.ends_with("i-002-example/spec.md") && v.reason.contains("content")),
        "mutation names the file: {violations:?}"
    );

    // (b) Partial move: one of the five files left behind.
    let repo = TempRepo::new("archive-move-partial");
    seed_issue(&repo, "i-003-example", "integrated");
    repo.commit_all("seed issue");
    archive_move(&repo, "i-003-example", Some("ubi-lang.md"));

    let violations = verify_history_preservation(repo.root(), &history_roots())
        .expect_err("a partial move must fail");
    assert!(
        violations.iter().any(|v| v.reason.contains("partial")),
        "partial move names the gap: {violations:?}"
    );

    // (c) In-place edit of a historical file without any move.
    let repo = TempRepo::new("archive-inplace-edit");
    seed_issue(&repo, "i-004-example", "integrated");
    repo.commit_all("seed issue");
    let inplace = repo.root().join(".arca/issue/i-004-example/design.md");
    fs::write(&inplace, "# rewritten\n").expect("edit in place");

    let violations = verify_history_preservation(repo.root(), &history_roots())
        .expect_err("in-place historical edits must fail");
    assert!(
        violations
            .iter()
            .any(|v| v.path.ends_with("i-004-example/design.md")),
        "in-place edit names the file: {violations:?}"
    );

    // (d) Archive move of an issue that is not completed.
    let repo = TempRepo::new("archive-move-pending");
    seed_issue(&repo, "i-005-example", "pending");
    repo.commit_all("seed issue");
    archive_move(&repo, "i-005-example", None);

    let violations = verify_history_preservation(repo.root(), &history_roots())
        .expect_err("archiving a pending issue must fail");
    assert!(
        violations
            .iter()
            .any(|v| v.reason.contains("not completed")),
        "non-completed move is named: {violations:?}"
    );
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

#[test]
fn schema_states_archive_and_snapshot_rules() {
    let root = repo_root();
    let schema = fs::read_to_string(root.join(".arca/schema.md")).expect("read .arca/schema.md");

    assert!(
        schema.contains(".arca/issue/archive/<issue-id>/"),
        "the archive destination is authorized in the contributor schema"
    );
    assert!(
        schema.to_lowercase().contains("reviewable snapshot"),
        "the reviewable-snapshot evidence rule is stated"
    );
    assert!(
        schema.contains("RATMAC_RELEASE_ACCEPTANCE"),
        "the release-lane opt-in is documented"
    );

    let tracked = Command::new("git")
        .args(["ls-files", "--error-unmatch", ".arca/schema.md"])
        .current_dir(&root)
        .output()
        .expect("git ls-files runs");
    assert!(
        tracked.status.success(),
        "the schema rule lives in tracked content visible to git diff"
    );
}

/// HT-039-02 (Output/Filesystem): a checkout whose files differ from HEAD only
/// by line endings - the normal Windows `core.autocrlf` case - is preservation,
/// while a real content change under the same normalization still fails.
#[test]
fn line_ending_normalization_is_not_mutation() {
    let repo = TempRepo::new("ht-039-02");
    seed_issue(&repo, "i-777-crlf", "integrated");
    repo.commit_all("seed issue with LF content");

    archive_move(&repo, "i-777-crlf", None);
    // Rewrite every archived file with CRLF endings, as a Windows checkout does.
    let archive_dir = repo.root().join(".arca/issue/archive/i-777-crlf");
    for name in FILES {
        let path = archive_dir.join(name);
        let text = fs::read_to_string(&path).expect("read archived file");
        let crlf = text.replace('\n', "\r\n");
        fs::write(&path, crlf).expect("write CRLF archived file");
    }

    verify_history_preservation(repo.root(), &history_roots())
        .expect("line-ending-only differences are not content mutation");

    // The same normalization must not hide a real edit.
    let spec = archive_dir.join("spec.md");
    let text = fs::read_to_string(&spec).expect("read archived spec");
    fs::write(&spec, text.replace("spec.md", "spec.md (edited)")).expect("mutate archived spec");
    let violations = verify_history_preservation(repo.root(), &history_roots())
        .expect_err("a real content change must still fail");
    assert!(
        violations
            .iter()
            .any(|violation| violation.path.ends_with("i-777-crlf/spec.md")),
        "the violation must name the mutated file: {violations:?}"
    );
}

/// PT-039-04 (extension): `.arca/log.md` is append-only history. Growth is
/// preservation; rewriting a recorded line is not.
#[test]
fn append_only_log_growth_is_preservation() {
    let repo = TempRepo::new("ht-039-02b");
    repo.write(".arca/log.md", "- 2026-07-01: first entry\n");
    repo.commit_all("seed history log");

    let log = repo.root().join(".arca/log.md");
    let recorded = fs::read_to_string(&log).expect("read log");
    fs::write(&log, format!("{recorded}- 2026-07-02: second entry\n")).expect("append entry");
    verify_history_preservation(repo.root(), &[".arca/log.md"])
        .expect("appending to the history log is preservation");

    fs::write(
        &log,
        "- 2026-07-01: rewritten entry\n- 2026-07-02: second entry\n",
    )
    .expect("rewrite history");
    let violations = verify_history_preservation(repo.root(), &[".arca/log.md"])
        .expect_err("rewriting a recorded line must fail");
    assert!(
        violations
            .iter()
            .any(|violation| violation.path == ".arca/log.md"
                && violation.reason.contains("append-only")),
        "the violation must name append-only history: {violations:?}"
    );
}
