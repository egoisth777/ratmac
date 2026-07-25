//! AOI-001: reviewable-snapshot evidence audit.
//!
//! Evidence may only claim what a reviewer can reconstruct. [`record_snapshot`]
//! enumerates every file under the declared evidence roots, records its git
//! tracking state and a SHA-256 content digest, and refuses any untracked or
//! unstaged content that the caller did not declare as an explicit exception.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;

use sha2::{Digest, Sha256};

/// Git tracking state of one file under a declared evidence root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackingState {
    /// Committed and identical to the index and HEAD.
    Tracked,
    /// Staged in the index: reviewable through `git diff --cached`.
    Staged,
    /// Tracked but carrying unstaged worktree modifications.
    Modified,
    /// Not known to git at all.
    Untracked,
}

impl fmt::Display for TrackingState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            TrackingState::Tracked => "tracked",
            TrackingState::Staged => "staged",
            TrackingState::Modified => "modified",
            TrackingState::Untracked => "untracked",
        };
        formatter.write_str(text)
    }
}

/// One manifest row: what was exercised, how reviewable it is, and its digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestRow {
    pub path: String,
    pub tracking: TrackingState,
    pub digest: String,
}

/// The snapshot manifest bound to an evidence claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotManifest {
    pub roots: Vec<String>,
    pub rows: Vec<ManifestRow>,
}

impl SnapshotManifest {
    /// Render the manifest as stable, diffable text.
    pub fn render(&self) -> String {
        let mut text = format!("roots: {}\n", self.roots.join(", "));
        for row in &self.rows {
            text.push_str(&format!("{}\t{}\t{}\n", row.path, row.tracking, row.digest));
        }
        text
    }
}

/// A reason one path makes the snapshot unreviewable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotViolation {
    pub path: String,
    pub reason: String,
}

/// Record a snapshot manifest over `roots`.
///
/// Returns the manifest when every file under the declared roots is tracked,
/// staged, or listed in `exceptions`; otherwise returns one violation per
/// unreviewable path.
pub fn record_snapshot(
    repo_root: &Path,
    roots: &[&str],
    exceptions: &[&str],
) -> Result<SnapshotManifest, Vec<SnapshotViolation>> {
    let states = porcelain_states(repo_root);

    let mut rows: Vec<ManifestRow> = Vec::new();
    for root in roots {
        collect_rows(repo_root, &repo_root.join(root), &states, &mut rows);
    }
    rows.sort_by(|left, right| left.path.cmp(&right.path));
    rows.dedup_by(|left, right| left.path == right.path);

    let violations: Vec<SnapshotViolation> = rows
        .iter()
        .filter(|row| {
            matches!(
                row.tracking,
                TrackingState::Untracked | TrackingState::Modified
            ) && !exceptions.contains(&row.path.as_str())
        })
        .map(|row| SnapshotViolation {
            path: row.path.clone(),
            reason: format!(
                "{} content under a declared evidence root is not reviewable from the recorded change",
                row.tracking
            ),
        })
        .collect();

    if violations.is_empty() {
        Ok(SnapshotManifest {
            roots: roots.iter().map(|root| (*root).to_owned()).collect(),
            rows,
        })
    } else {
        Err(violations)
    }
}

/// SHA-256 of a file's bytes, lowercase hex.
pub fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

/// Parse `git status --porcelain -uall` into path -> tracking state.
fn porcelain_states(repo_root: &Path) -> BTreeMap<String, TrackingState> {
    // `core.quotePath=false` keeps non-ASCII paths verbatim; otherwise git
    // escapes them and every such row silently parses to the wrong path.
    let output = Command::new("git")
        .args([
            "-c",
            "core.quotePath=false",
            "status",
            "--porcelain",
            "-uall",
        ])
        .current_dir(repo_root)
        .output()
        .expect("git status must run");
    let text = String::from_utf8_lossy(&output.stdout);

    let mut states = BTreeMap::new();
    for line in text.lines() {
        if line.len() < 4 {
            continue;
        }
        let bytes = line.as_bytes();
        let (index, worktree) = (bytes[0] as char, bytes[1] as char);
        let rest = &line[3..];
        // Renames report "old -> new"; the new path is the one on disk.
        let path = rest.rsplit(" -> ").next().unwrap_or(rest);
        let path = path.trim().trim_matches('"').replace('\\', "/");

        let state = if index == '?' || worktree == '?' {
            TrackingState::Untracked
        } else if worktree != ' ' {
            TrackingState::Modified
        } else {
            TrackingState::Staged
        };
        states.insert(path, state);
    }
    states
}

fn collect_rows(
    repo_root: &Path,
    directory: &Path,
    states: &BTreeMap<String, TrackingState>,
    rows: &mut Vec<ManifestRow>,
) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            if matches!(name.as_str(), ".git" | "target") {
                continue;
            }
            collect_rows(repo_root, &path, states, rows);
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(repo_root)
                .expect("evidence roots live inside the repository")
                .to_string_lossy()
                .replace('\\', "/");
            let tracking = states
                .get(&relative)
                .copied()
                .unwrap_or(TrackingState::Tracked);
            rows.push(ManifestRow {
                path: relative,
                tracking,
                digest: sha256_file(&path),
            });
        }
    }
}
