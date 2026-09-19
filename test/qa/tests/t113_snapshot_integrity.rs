//! t-113 / AOI-001: truthful and complete snapshot evidence.
//!
//! PT-113-01 `ignored_file_is_never_tracked`
//! PT-113-02 `tracking_states_and_modified_exception_marker_remain_exact`
//! PT-113-03 `rendered_manifest_marks_only_exact_exceptions`
//! PT-113-04 `declared_file_root_is_included_alone_and_with_overlap`
//! PT-113-05 `unavailable_roots_refuse_without_manifest_output`
//!
//! Ignored content is never labeled tracked; an accepted exception is
//! visibly marked on its rendered row beside path, state, and digest; a
//! declared regular-file root contributes its file; and a missing or
//! unreadable root refuses by name instead of recording an incomplete
//! manifest.

use ratmac_qa::snapshot::{record_snapshot, SnapshotManifest, TrackingState};
use ratmac_qa::tempgit::TempRepo;
use sha2::{Digest, Sha256};
use std::fs;
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use std::process::Command;

/// Bytes a refused command must leave untouched when the destination exists.
const SENTINEL: &[u8] = b"pre-existing manifest bytes; a refusal must not rewrite me\n";

/// Independent digest oracle: re-derive the SHA-256 of a fixture file here,
/// never through `ratmac_qa::snapshot::sha256_file`.
fn independent_digest(path: &Path) -> String {
    let bytes = fs::read(path).expect("read fixture bytes for the independent digest");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

/// The data rows of a rendered manifest: everything after the roots header.
fn data_rows(rendered: &str) -> Vec<&str> {
    rendered
        .lines()
        .filter(|line| !line.starts_with("roots: "))
        .collect()
}

/// Run the real `snapshot-manifest` command from `current_dir`.
fn run_manifest_binary(current_dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_snapshot-manifest"))
        .current_dir(current_dir)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("snapshot-manifest must run: {error}"))
}

/// Every row of `base` appears in `alias` with the same tracking state and
/// digest: a root alias changes spelling, never file identity.
fn assert_alias_preserves_rows(base: &SnapshotManifest, alias: &SnapshotManifest) {
    for expected in &base.rows {
        let twin = alias
            .rows
            .iter()
            .find(|row| row.path == expected.path)
            .unwrap_or_else(|| {
                panic!(
                    "alias must still select {}: {:?}",
                    expected.path, alias.rows
                )
            });
        assert_eq!(
            twin.tracking, expected.tracking,
            "alias keeps the classification of {}",
            expected.path
        );
        assert_eq!(
            twin.digest, expected.digest,
            "alias keeps the digest of {}",
            expected.path
        );
    }
}

/// PT-113-01: an ignored file is untracked content. Without an exception,
/// recording refuses and names the ignored path; with that exact exception,
/// the row is present as `Untracked` with an independently re-derived
/// digest, and `Tracked` is never observed for it.
#[test]
fn ignored_file_is_never_tracked() {
    let repo = TempRepo::new("t113-ignored");
    repo.write(".gitignore", "src/ignored.bin\n");
    repo.write("src/main.rs", "fn main() {}\n");
    repo.commit_all("initial");
    repo.write("src/ignored.bin", "ignored payload\n");
    // Untracked build output: reachable on disk, excluded from traversal.
    repo.write("target/artifact.txt", "stale build output\n");

    let ignored = repo.git(&["check-ignore", "-q", "src/ignored.bin"]);
    assert!(
        ignored.status.success(),
        "fixture control: git must report src/ignored.bin as ignored"
    );

    let violations = record_snapshot(repo.root(), &["src"], &[])
        .expect_err("ignored content is untracked and must refuse, not pass as clean");
    assert!(
        violations
            .iter()
            .any(|violation| violation.path == "src/ignored.bin"
                && violation.reason.contains("untracked")),
        "the refusal must name the ignored path as untracked: {violations:?}"
    );

    let manifest = record_snapshot(repo.root(), &["src"], &["src/ignored.bin"])
        .expect("the exact exception records the ignored file explicitly");
    let row = manifest
        .rows
        .iter()
        .find(|row| row.path == "src/ignored.bin")
        .unwrap_or_else(|| {
            panic!(
                "the excepted ignored file keeps its row: {:?}",
                manifest.rows
            )
        });
    assert_eq!(
        row.tracking,
        TrackingState::Untracked,
        "an ignored path is Untracked, never the clean Tracked fallback"
    );
    assert_eq!(
        row.digest,
        independent_digest(&repo.root().join("src/ignored.bin")),
        "the excepted row's digest re-derives independently"
    );
    let clean = manifest
        .rows
        .iter()
        .find(|row| row.path == "src/main.rs")
        .expect("the committed file keeps its row");
    assert_eq!(clean.tracking, TrackingState::Tracked);

    // Whole-tree control: `.` excludes `.git` and `target`, so the ignored
    // file is the only exception the repository needs.
    let whole = record_snapshot(repo.root(), &["."], &["src/ignored.bin"])
        .expect("root . is reviewable once the ignored file is excepted");
    let paths: Vec<&str> = whole.rows.iter().map(|row| row.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![".gitignore", "src/ignored.bin", "src/main.rs"],
        ".git and target stay excluded from traversal: {paths:?}"
    );
    assert_eq!(whole.roots, vec![".".to_owned()]);
}

/// PT-113-02: with only the modified path excepted, the manifest reports
/// `Tracked`, `Staged`, and `Modified` for the right files, stays sorted,
/// re-derives every digest independently, and marks exactly the excepted
/// modified row with a fourth literal `exception` field. `.` and `./src`
/// keep file identity and classification while the header keeps the
/// supplied spelling.
#[test]
fn tracking_states_and_modified_exception_marker_remain_exact() {
    let repo = TempRepo::new("t113-states");
    repo.write("src/clean.rs", "fn clean() {}\n");
    repo.write("src/modified.rs", "fn modified() {}\n");
    repo.commit_all("initial");
    repo.write("src/staged.rs", "pub fn staged() {}\n");
    repo.stage("src/staged.rs");
    repo.write("src/modified.rs", "fn modified() { /* edited */ }\n");

    let manifest = record_snapshot(repo.root(), &["src"], &["src/modified.rs"])
        .expect("only the modified path is excepted; clean and staged stay reviewable");

    let paths: Vec<&str> = manifest.rows.iter().map(|row| row.path.as_str()).collect();
    assert_eq!(
        paths,
        vec!["src/clean.rs", "src/modified.rs", "src/staged.rs"]
    );
    let state = |path: &str| {
        manifest
            .rows
            .iter()
            .find(|row| row.path == path)
            .unwrap_or_else(|| panic!("row for {path}"))
            .tracking
    };
    assert_eq!(state("src/clean.rs"), TrackingState::Tracked);
    assert_eq!(state("src/modified.rs"), TrackingState::Modified);
    assert_eq!(state("src/staged.rs"), TrackingState::Staged);

    for row in &manifest.rows {
        assert_eq!(
            row.digest,
            independent_digest(&repo.root().join(&row.path)),
            "digest re-derives independently: {}",
            row.path
        );
    }

    let digest = |path: &str| independent_digest(&repo.root().join(path));
    let clean_line = format!("src/clean.rs\ttracked\t{}", digest("src/clean.rs"));
    let modified_line = format!(
        "src/modified.rs\tmodified\t{}\texception",
        digest("src/modified.rs")
    );
    let staged_line = format!("src/staged.rs\tstaged\t{}", digest("src/staged.rs"));

    let rendered = manifest.render();
    assert_eq!(
        rendered.lines().next(),
        Some("roots: src"),
        "the header preserves the supplied root spelling"
    );
    let lines = data_rows(&rendered);
    assert_eq!(
        lines,
        vec![
            clean_line.as_str(),
            modified_line.as_str(),
            staged_line.as_str()
        ],
        "clean and staged rows keep exactly three tab-separated fields; only the excepted \
         modified row adds the fourth literal `exception` field: {rendered:?}"
    );

    // Root aliases: `./src` and `.` select and classify the same files.
    let alias_src = record_snapshot(repo.root(), &["./src"], &["src/modified.rs"])
        .expect("./src is the same evidence root as src");
    assert_eq!(alias_src.roots, vec!["./src".to_owned()]);
    assert_eq!(alias_src.render().lines().next(), Some("roots: ./src"));
    assert_eq!(
        alias_src.rows.len(),
        manifest.rows.len(),
        "./src selects the same files"
    );
    assert_alias_preserves_rows(&manifest, &alias_src);

    let alias_dot = record_snapshot(repo.root(), &["."], &["src/modified.rs"])
        .expect(". is the repository root as an evidence root");
    assert_eq!(alias_dot.roots, vec![".".to_owned()]);
    assert!(alias_dot.rows.iter().any(|row| row.path == ".gitignore"));
    assert!(
        !alias_dot
            .rows
            .iter()
            .any(|row| row.path.starts_with(".git/")),
        ".git stays excluded"
    );
    assert_alias_preserves_rows(&manifest, &alias_dot);
}

/// PT-113-03: paths with spaces and Unicode are excepted only by their exact
/// spelling. Each exact exception row carries the exception marker on its
/// rendered row; same-prefix neighbors and other reviewable rows do not, and
/// near-miss exception spellings excuse nothing.
#[test]
fn rendered_manifest_marks_only_exact_exceptions() {
    let repo = TempRepo::new("t113-exact");
    // Reviewable same-prefix neighbors: tracked and clean.
    repo.write(
        "data/notes spaced.txt.bak",
        "neighbor of an excepted path\n",
    );
    repo.write("data/wîngs ✈ extra.txt", "unicode neighbor\n");
    repo.commit_all("initial");
    // Untracked exact exceptions carrying spaces and Unicode.
    repo.write("data/notes spaced.txt", "untracked spaced payload\n");
    repo.write("data/wîngs ✈.txt", "untracked unicode payload\n");

    let manifest = record_snapshot(
        repo.root(),
        &["data"],
        &["data/notes spaced.txt", "data/wîngs ✈.txt"],
    )
    .expect("the two exact exceptions make the root reviewable");

    let digest = |path: &str| independent_digest(&repo.root().join(path));
    let expected = vec![
        format!(
            "data/notes spaced.txt\tuntracked\t{}\texception",
            digest("data/notes spaced.txt")
        ),
        format!(
            "data/notes spaced.txt.bak\ttracked\t{}",
            digest("data/notes spaced.txt.bak")
        ),
        format!(
            "data/wîngs ✈ extra.txt\ttracked\t{}",
            digest("data/wîngs ✈ extra.txt")
        ),
        format!(
            "data/wîngs ✈.txt\tuntracked\t{}\texception",
            digest("data/wîngs ✈.txt")
        ),
    ];
    let expected_refs: Vec<&str> = expected.iter().map(String::as_str).collect();

    let rendered = manifest.render();
    assert_eq!(rendered.lines().next(), Some("roots: data"));
    assert_eq!(
        data_rows(&rendered),
        expected_refs,
        "exact exception rows carry the fourth `exception` field; same-prefix neighbors do not: \
         {rendered:?}"
    );

    // Exact matching stays exact: a truncated spelling or the directory name
    // excuses neither untracked path.
    for near_miss in ["data/notes spaced.tx", "data/wîngs ✈", "data"] {
        let violations = record_snapshot(repo.root(), &["data"], &[near_miss])
            .expect_err("a near-miss exception spelling must not excuse the untracked paths");
        for path in ["data/notes spaced.txt", "data/wîngs ✈.txt"] {
            assert!(
                violations.iter().any(|violation| violation.path == path),
                "exception {near_miss:?} must not excuse {path}: {violations:?}"
            );
        }
    }
}

/// PT-113-04: the real `snapshot-manifest` command, first with only a
/// regular-file root so a silent file-root skip cannot hide behind a
/// directory also supplying the row, then with that same file also covered
/// by a directory root: same file, exactly once.
#[test]
fn declared_file_root_is_included_alone_and_with_overlap() {
    let repo = TempRepo::new("t113-file-root");
    repo.write("docs/evidence.md", "# evidence payload\n");
    repo.write("docs/other.md", "# other payload\n");
    repo.commit_all("initial");

    // File-only phase: the regular file is the only declared root.
    let output = run_manifest_binary(repo.root(), &["file-only.manifest", "docs/evidence.md"]);
    assert!(
        output.status.success(),
        "a clean tracked regular-file root must record: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let file_only = fs::read_to_string(repo.root().join("file-only.manifest"))
        .expect("a successful command writes the manifest");
    assert_eq!(
        file_only.lines().next(),
        Some("roots: docs/evidence.md"),
        "the header preserves the supplied root spelling"
    );
    let rows = data_rows(&file_only);
    assert_eq!(
        rows.len(),
        1,
        "the regular-file root contributes its own file, not silence: {file_only:?}"
    );
    assert_eq!(
        rows[0],
        format!(
            "docs/evidence.md\ttracked\t{}",
            independent_digest(&repo.root().join("docs/evidence.md"))
        )
    );

    // Overlap twin: the same file is also reached through the directory.
    let output = run_manifest_binary(
        repo.root(),
        &["overlap.manifest", "docs", "docs/evidence.md"],
    );
    assert!(
        output.status.success(),
        "the overlapping twin must record: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let overlap = fs::read_to_string(repo.root().join("overlap.manifest"))
        .expect("a successful command writes the manifest");
    assert_eq!(
        overlap.lines().next(),
        Some("roots: docs, docs/evidence.md")
    );
    let rows = data_rows(&overlap);
    assert_eq!(
        rows.len(),
        2,
        "the overlap deduplicates to the selected files: {overlap:?}"
    );
    assert_eq!(
        rows.iter()
            .filter(|line| line.starts_with("docs/evidence.md\t"))
            .count(),
        1,
        "the file reached through both roots appears exactly once"
    );
    assert_eq!(
        rows[0],
        format!(
            "docs/evidence.md\ttracked\t{}",
            independent_digest(&repo.root().join("docs/evidence.md"))
        )
    );
    assert_eq!(
        rows[1],
        format!(
            "docs/other.md\ttracked\t{}",
            independent_digest(&repo.root().join("docs/other.md"))
        )
    );
}

/// PT-113-05: a missing or unreadable declared root refuses by name through
/// the helper and the command, and a refused command leaves an absent output
/// absent and a pre-existing output byte-identical. An existing readable
/// empty directory stays an honest empty success.
#[test]
fn unavailable_roots_refuse_without_manifest_output() {
    let repo = TempRepo::new("t113-unavailable");
    repo.write(".gitignore", "evidence/vault.locked\n");
    repo.write("src/main.rs", "fn main() {}\n");
    repo.commit_all("initial");
    fs::create_dir_all(repo.root().join("empty")).expect("create a readable empty directory");

    let empty = record_snapshot(repo.root(), &["empty"], &[])
        .expect("an existing readable empty directory is honest emptiness, not a refusal");
    assert!(empty.rows.is_empty(), "no files means no rows: {empty:?}");

    let violations = record_snapshot(repo.root(), &["missing/root"], &[])
        .expect_err("a missing declared root must refuse instead of recording an empty manifest");
    assert!(
        violations
            .iter()
            .any(|violation| violation.path.contains("missing/root")),
        "the refusal must name the missing root: {violations:?}"
    );

    // Absent destination: the refused command creates nothing.
    let absent = repo.root().join("refused-absent.manifest");
    let output = run_manifest_binary(repo.root(), &["refused-absent.manifest", "missing/root"]);
    assert!(
        !output.status.success(),
        "the command must exit nonzero for a missing root: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !absent.exists(),
        "a refused snapshot must not create the manifest output"
    );

    // Pre-existing destination: the refused command rewrites nothing.
    let existing = repo.root().join("refused-existing.manifest");
    fs::write(&existing, SENTINEL).expect("write the sentinel manifest");
    let output = run_manifest_binary(repo.root(), &["refused-existing.manifest", "missing/root"]);
    assert!(
        !output.status.success(),
        "the command must refuse a missing root instead of overwriting the destination"
    );
    assert_eq!(
        fs::read(&existing).expect("the sentinel stays readable"),
        SENTINEL,
        "a refused snapshot leaves an existing manifest byte-identical"
    );

    // A real named access failure: an exclusively held, ignored, exactly
    // excepted file cannot be read during traversal. Windows excludes other
    // readers through a zero share mode; the missing-root cases above stay
    // portable.
    #[cfg(windows)]
    {
        repo.write("evidence/vault.locked", "locked payload\n");
        let ignored = repo.git(&["check-ignore", "-q", "evidence/vault.locked"]);
        assert!(
            ignored.status.success(),
            "fixture control: git must report evidence/vault.locked as ignored"
        );

        let _lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(repo.root().join("evidence/vault.locked"))
            .expect("hold an exclusive handle on the ignored excepted file");

        let violations = record_snapshot(repo.root(), &["evidence"], &["evidence/vault.locked"])
            .expect_err("an excepted file whose bytes cannot be read must refuse by name");
        assert!(
            violations
                .iter()
                .any(|violation| violation.path == "evidence/vault.locked"),
            "the refusal must name the unreadable file: {violations:?}"
        );

        let locked_out = repo.root().join("locked.manifest");
        fs::write(&locked_out, SENTINEL).expect("write the sentinel manifest");
        let output = run_manifest_binary(
            repo.root(),
            &["locked.manifest", "evidence", "--", "evidence/vault.locked"],
        );
        assert!(
            !output.status.success(),
            "the command must refuse when a declared root cannot be read"
        );
        assert_eq!(
            fs::read(&locked_out).expect("the sentinel stays readable"),
            SENTINEL,
            "the refused command must not rewrite the existing manifest"
        );
    }
}
