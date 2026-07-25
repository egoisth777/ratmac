//! t-039 / AOI-001: reviewable-snapshot evidence audit.
//!
//! PT-039-01 `manifest_matches_independent_rehash`
//! PT-039-02 `untracked_under_declared_root_refuses`
//! HT-039-01 `manifest_parses_renames_spaces_and_unicode`
//!
//! Evidence recorded over declared roots must carry a manifest of per-path
//! tracking state plus content digest that an independent re-hash reproduces,
//! and must refuse undeclared untracked or unstaged content in those roots.

use ratmac_qa::snapshot::{record_snapshot, TrackingState};
use ratmac_qa::tempgit::TempRepo;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Independent digest implementation: the manifest must agree with a hash
/// computed here, not with itself.
fn independent_digest(path: &Path) -> String {
    let bytes = fs::read(path).expect("read file for independent digest");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

#[test]
fn manifest_matches_independent_rehash() {
    let repo = TempRepo::new("snapshot-clean");
    repo.write("src/main.rs", "fn main() {}\n");
    repo.write(".arca/index.md", "# Index\n");
    repo.commit_all("initial");

    // A staged-but-uncommitted file is still reviewable.
    repo.write("src/added.rs", "pub fn added() {}\n");
    repo.stage("src/added.rs");

    let manifest =
        record_snapshot(repo.root(), &["src", ".arca"], &[]).expect("clean roots must record");

    let paths: Vec<&str> = manifest.rows.iter().map(|row| row.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![".arca/index.md", "src/added.rs", "src/main.rs"],
        "manifest is sorted and covers every file under the declared roots"
    );
    assert_eq!(manifest.roots, vec!["src".to_owned(), ".arca".to_owned()]);

    for row in &manifest.rows {
        assert_eq!(
            row.digest,
            independent_digest(&repo.root().join(&row.path)),
            "digest must re-derive independently: {}",
            row.path
        );
        assert!(
            matches!(row.tracking, TrackingState::Tracked | TrackingState::Staged),
            "every reviewable row is tracked or staged: {row:?}"
        );
    }

    let added = manifest
        .rows
        .iter()
        .find(|row| row.path == "src/added.rs")
        .expect("staged file is present");
    assert_eq!(added.tracking, TrackingState::Staged);

    let rendered = manifest.render();
    assert!(
        rendered.contains("src/main.rs") && rendered.contains(&manifest.rows[0].digest),
        "rendered manifest carries paths and digests: {rendered}"
    );
}

#[test]
fn untracked_under_declared_root_refuses() {
    let repo = TempRepo::new("snapshot-untracked");
    repo.write("src/main.rs", "fn main() {}\n");
    repo.commit_all("initial");

    repo.write("src/scratch.rs", "// not reviewable\n");

    let violations = record_snapshot(repo.root(), &["src"], &[])
        .expect_err("untracked content under a declared root must refuse");
    assert_eq!(violations.len(), 1, "exactly one violation: {violations:?}");
    assert_eq!(violations[0].path, "src/scratch.rs");
    assert!(
        violations[0].reason.contains("untracked"),
        "violation names why: {:?}",
        violations[0]
    );

    // An unstaged modification to a tracked file is equally unreviewable.
    repo.write("src/main.rs", "fn main() { /* edited */ }\n");
    let violations = record_snapshot(repo.root(), &["src"], &["src/scratch.rs"])
        .expect_err("unstaged modification must refuse");
    assert_eq!(
        violations
            .iter()
            .map(|violation| violation.path.as_str())
            .collect::<Vec<_>>(),
        vec!["src/main.rs"],
        "the declared exception is accepted, the modification is not: {violations:?}"
    );

    // Declaring both as exceptions records them explicitly instead of refusing.
    let manifest = record_snapshot(repo.root(), &["src"], &["src/scratch.rs", "src/main.rs"])
        .expect("explicit exceptions are recorded, not silently dropped");
    let scratch = manifest
        .rows
        .iter()
        .find(|row| row.path == "src/scratch.rs")
        .expect("exception row is present in the manifest");
    assert_eq!(scratch.tracking, TrackingState::Untracked);
}

/// HT-039-01 (Input/Routing): porcelain rows must survive a staged rename, a
/// path containing spaces, and a non-ASCII path. No row may be dropped or
/// mis-split into the wrong path.
#[test]
fn manifest_parses_renames_spaces_and_unicode() {
    let repo = TempRepo::new("ht-039-01");
    let src = repo.root().join("src");
    fs::create_dir_all(&src).expect("create src");
    repo.write("src/plain.rs", "fn plain() {}\n");
    repo.write("src/with space.rs", "fn spaced() {}\n");
    repo.write("src/ünïcode.rs", "fn unicode() {}\n");
    repo.commit_all("initial");

    // A staged rename, a staged edit, and an unstaged edit.
    let renamed = repo.git(&["mv", "src/plain.rs", "src/renamed.rs"]);
    assert!(renamed.status.success(), "git mv must succeed");
    repo.write("src/with space.rs", "fn spaced() { /* edited */ }\n");
    repo.stage("src/with space.rs");
    repo.write("src/ünïcode.rs", "fn unicode() { /* edited */ }\n");

    // The unstaged edit is refused and named exactly.
    let violations = record_snapshot(repo.root(), &["src"], &[])
        .expect_err("an unstaged edit under a declared root must refuse");
    assert!(
        violations
            .iter()
            .any(|violation| violation.path == "src/ünïcode.rs"),
        "the violation must name the non-ASCII path: {violations:?}"
    );

    // With that path declared as an exception, every row parses correctly.
    let manifest = record_snapshot(repo.root(), &["src"], &["src/ünïcode.rs"])
        .expect("declared exception makes the snapshot reviewable");
    let paths: Vec<&str> = manifest.rows.iter().map(|row| row.path.as_str()).collect();
    assert_eq!(
        paths,
        vec!["src/renamed.rs", "src/with space.rs", "src/ünïcode.rs"],
        "renamed, spaced, and non-ASCII paths must each appear once"
    );
    let state = |path: &str| {
        manifest
            .rows
            .iter()
            .find(|row| row.path == path)
            .unwrap_or_else(|| panic!("row for {path}"))
            .tracking
    };
    assert_eq!(state("src/renamed.rs"), TrackingState::Staged);
    assert_eq!(state("src/with space.rs"), TrackingState::Staged);
    assert_eq!(state("src/ünïcode.rs"), TrackingState::Modified);
}
