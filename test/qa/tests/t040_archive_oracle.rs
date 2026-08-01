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

fn seed_archived_issue(repo: &TempRepo, issue_id: &str, disposition: &str) {
    for name in FILES {
        let content = if name == "index.md" {
            format!(
                "# {issue_id}\n\n```yaml\nissue-id: \"{issue_id}\"\nstatus: \"integrated\"\n```\n"
            )
        } else if name == "spec.md" {
            format!(
                "# Requirement records\n\n\
                 | Req ID | Requirement | Disposition |\n\
                 |---|---|---|\n\
                 | `DEF-001` | Preserve the original carrier. | {disposition} |\n\n\
                 See [archived dependency](../i-900-dependency/design.md).\n"
            )
        } else {
            format!("# {name} for {issue_id}\n")
        };
        repo.write(&format!(".arca/issue/archive/{issue_id}/{name}"), &content);
    }
}

fn restore_deferred(repo: &TempRepo, issue_id: &str, skip: Option<&str>) {
    for name in FILES {
        if Some(name) == skip {
            continue;
        }
        let from = repo
            .root()
            .join(format!(".arca/issue/archive/{issue_id}/{name}"));
        let content = fs::read_to_string(&from).expect("read archived issue file");
        let restored = content
            .replace("status: \"integrated\"", "status: \"deferred\"")
            .replace(
                "](../i-900-dependency/",
                "](../../archive/i-900-dependency/",
            );
        let to = repo
            .root()
            .join(format!(".arca/issue/deferred/{issue_id}/{name}"));
        fs::create_dir_all(to.parent().expect("deferred parent"))
            .expect("create deferred directory");
        fs::write(&to, restored).expect("write deferred issue file");
        fs::remove_file(&from).expect("remove archived source file");
    }
}

fn move_intake_to_deferred(repo: &TempRepo, issue_id: &str) {
    for name in FILES {
        let from = repo.root().join(format!(".arca/issue/{issue_id}/{name}"));
        let content = fs::read_to_string(&from).expect("read intake issue file");
        let mut moved = content
            .replace("status: \"pending\"", "status: \"deferred\"")
            .replace("](../../", "](../../../");
        if name == "spec.md" {
            moved.push_str(
                "\n| Req ID | Requirement | Disposition |\n\
                 |---|---|---|\n\
                 | `DEF-001` | Preserve the original carrier. | deferred |\n",
            );
        }
        let to = repo
            .root()
            .join(format!(".arca/issue/deferred/{issue_id}/{name}"));
        fs::create_dir_all(to.parent().expect("deferred parent"))
            .expect("create deferred directory");
        fs::write(&to, moved).expect("write deferred issue file");
        fs::remove_file(&from).expect("remove intake source file");
    }
}

fn move_deferred_to_intake(repo: &TempRepo, issue_id: &str) {
    for name in FILES {
        let from = repo
            .root()
            .join(format!(".arca/issue/deferred/{issue_id}/{name}"));
        let content = fs::read_to_string(&from).expect("read deferred issue file");
        let moved = content
            .replace("status: \"deferred\"", "status: \"pending\"")
            .replace("](../../../", "](../../");
        let to = repo.root().join(format!(".arca/issue/{issue_id}/{name}"));
        fs::create_dir_all(to.parent().expect("intake parent")).expect("create intake directory");
        fs::write(&to, moved).expect("write selected issue file");
        fs::remove_file(&from).expect("remove deferred source file");
    }
}

fn complete_and_archive(repo: &TempRepo, issue_id: &str) {
    for name in FILES {
        let from = repo.root().join(format!(".arca/issue/{issue_id}/{name}"));
        let content = fs::read_to_string(&from).expect("read pending issue file");
        let mut archived = content
            .replace("status: \"pending\"", "status: \"integrated\"")
            .replace("](../../", "](../../../");
        if name == "spec.md" {
            archived.push_str(
                "\n| Req ID | Requirement | Disposition |\n\
                 |---|---|---|\n\
                 | `REQ-001` | Preserve the original carrier. | accepted |\n",
            );
        }
        let to = repo
            .root()
            .join(format!(".arca/issue/archive/{issue_id}/{name}"));
        fs::create_dir_all(to.parent().expect("archive parent")).expect("create archive dir");
        fs::write(&to, archived).expect("write completed issue file");
        fs::remove_file(&from).expect("remove intake source file");
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
fn complete_deferred_restoration_is_preservation() {
    let repo = TempRepo::new("deferred-restore-pass");
    seed_archived_issue(&repo, "i-010-waiting", "deferred");
    repo.commit_all("seed wrongly archived deferred issue");

    restore_deferred(&repo, "i-010-waiting", None);

    verify_history_preservation(repo.root(), &history_roots())
        .expect("complete status-and-link-only deferred restoration is preservation");
}

#[test]
fn active_issue_lifecycle_moves_are_preservation() {
    let repo = TempRepo::new("intake-to-deferred-pass");
    seed_issue(&repo, "i-020-waiting", "pending");
    repo.commit_all("seed pending issue");
    move_intake_to_deferred(&repo, "i-020-waiting");
    verify_history_preservation(repo.root(), &history_roots())
        .expect("a complete pending-to-deferred move is preservation");

    let repo = TempRepo::new("deferred-to-intake-pass");
    seed_issue(&repo, "i-021-selected", "pending");
    move_intake_to_deferred(&repo, "i-021-selected");
    repo.commit_all("seed deferred issue");
    move_deferred_to_intake(&repo, "i-021-selected");
    verify_history_preservation(repo.root(), &history_roots())
        .expect("selecting the complete deferred bundle is preservation");

    let repo = TempRepo::new("pending-to-archive-pass");
    seed_issue(&repo, "i-022-completed", "pending");
    repo.commit_all("seed pending issue");
    complete_and_archive(&repo, "i-022-completed");
    verify_history_preservation(repo.root(), &history_roots())
        .expect("completing and archiving a pending issue is preservation");
}

#[test]
fn invalid_deferred_restorations_fail() {
    let repo = TempRepo::new("deferred-restore-partial");
    seed_archived_issue(&repo, "i-011-waiting", "deferred");
    repo.commit_all("seed wrongly archived deferred issue");
    restore_deferred(&repo, "i-011-waiting", Some("ubi-lang.md"));
    let violations = verify_history_preservation(repo.root(), &history_roots())
        .expect_err("partial deferred restoration must fail");
    assert!(
        violations
            .iter()
            .any(|violation| violation.reason.contains("partial")),
        "partial restoration names the gap: {violations:?}"
    );

    let repo = TempRepo::new("deferred-restore-no-row");
    seed_archived_issue(&repo, "i-012-complete", "accepted");
    repo.commit_all("seed completed archived issue");
    restore_deferred(&repo, "i-012-complete", None);
    let violations = verify_history_preservation(repo.root(), &history_roots())
        .expect_err("an issue without a deferred ask must stay archived");
    assert!(
        violations
            .iter()
            .any(|violation| violation.reason.contains("no recorded deferred ask")),
        "missing disposition is named: {violations:?}"
    );

    let repo = TempRepo::new("deferred-restore-status");
    seed_archived_issue(&repo, "i-013-waiting", "deferred");
    repo.commit_all("seed wrongly archived deferred issue");
    restore_deferred(&repo, "i-013-waiting", None);
    let index = repo
        .root()
        .join(".arca/issue/deferred/i-013-waiting/index.md");
    let content = fs::read_to_string(&index).expect("read restored index");
    fs::write(
        &index,
        content.replace("status: \"deferred\"", "status: \"integrated\""),
    )
    .expect("write wrong restored status");
    let violations = verify_history_preservation(repo.root(), &history_roots())
        .expect_err("restoration without deferred status must fail");
    assert!(
        violations
            .iter()
            .any(|violation| violation.reason.contains("status deferred")),
        "wrong status is named: {violations:?}"
    );

    let repo = TempRepo::new("deferred-restore-mutated");
    seed_archived_issue(&repo, "i-014-waiting", "deferred");
    repo.commit_all("seed wrongly archived deferred issue");
    restore_deferred(&repo, "i-014-waiting", None);
    let spec = repo
        .root()
        .join(".arca/issue/deferred/i-014-waiting/spec.md");
    let content = fs::read_to_string(&spec).expect("read restored spec");
    fs::write(&spec, format!("{content}\nChanged prose.\n")).expect("mutate restored spec");
    let violations = verify_history_preservation(repo.root(), &history_roots())
        .expect_err("arbitrary content mutation during restoration must fail");
    assert!(
        violations
            .iter()
            .any(|violation| violation.reason.contains("differs")),
        "mutation is named: {violations:?}"
    );
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
        schema.contains(".arca/issue/deferred/<issue-id>/"),
        "the deferred restoration destination is authorized in the contributor schema"
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
