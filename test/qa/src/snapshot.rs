//! AOI-001: reviewable-snapshot evidence audit.
//!
//! Evidence may only claim what a reviewer can reconstruct. [`record_snapshot`]
//! enumerates every file selected by the declared evidence roots — each root
//! is traversed as a directory or contributed directly as a regular file —
//! records its git tracking state and a SHA-256 content digest, and refuses
//! any untracked or unstaged content that the caller did not declare as an
//! explicit exception.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path};
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
    /// True only when the caller declared this exact path as an explicit
    /// exception; rendered as a fourth literal `exception` field.
    pub exception: bool,
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
            text.push_str(&format!("{}\t{}\t{}", row.path, row.tracking, row.digest));
            if row.exception {
                text.push_str("\texception");
            }
            text.push('\n');
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
/// unreviewable path. Operational failures — a missing or unreadable root, a
/// traversal or digest error, or a failed git inventory — surface as named
/// violations too, so an incomplete manifest never records successfully.
pub fn record_snapshot(
    repo_root: &Path,
    roots: &[&str],
    exceptions: &[&str],
) -> Result<SnapshotManifest, Vec<SnapshotViolation>> {
    let states = porcelain_states(repo_root).map_err(|error| vec![error])?;
    let index = cached_index_paths(repo_root).map_err(|error| vec![error])?;

    let mut rows: Vec<ManifestRow> = Vec::new();
    let mut violations: Vec<SnapshotViolation> = Vec::new();
    for root in roots {
        collect_rows(
            repo_root,
            &repo_root.join(root),
            &states,
            &index,
            exceptions,
            &mut rows,
            &mut violations,
        );
    }
    rows.sort_by(|left, right| left.path.cmp(&right.path));
    rows.dedup_by(|left, right| left.path == right.path);

    violations.extend(
        rows.iter()
            .filter(|row| {
                matches!(
                    row.tracking,
                    TrackingState::Untracked | TrackingState::Modified
                ) && !row.exception
            })
            .map(|row| SnapshotViolation {
                path: row.path.clone(),
                reason: format!(
                    "{} content under a declared evidence root is not reviewable from the recorded change",
                    row.tracking
                ),
            }),
    );

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
    try_sha256_file(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// Fallible digest used by snapshot traversal: content that cannot be read
/// must become a named violation, never a panic.
fn try_sha256_file(path: &Path) -> std::io::Result<String> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Parse `git status --porcelain -uall` into path -> tracking state.
fn porcelain_states(
    repo_root: &Path,
) -> Result<BTreeMap<String, TrackingState>, SnapshotViolation> {
    // `core.quotePath=false` keeps non-ASCII paths verbatim; otherwise git
    // escapes them and every such row silently parses to the wrong path.
    let output = match Command::new("git")
        .args([
            "-c",
            "core.quotePath=false",
            "status",
            "--porcelain",
            "-uall",
        ])
        .current_dir(repo_root)
        .output()
    {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            return Err(SnapshotViolation {
                path: repo_root.display().to_string(),
                reason: format!(
                    "git status --porcelain -uall failed with exit status {}",
                    output.status
                ),
            })
        }
        Err(error) => {
            return Err(SnapshotViolation {
                path: repo_root.display().to_string(),
                reason: format!("git status --porcelain -uall must run: {error}"),
            })
        }
    };
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
    Ok(states)
}

/// Read the cached git index (`git ls-files --cached -z`) as the set of
/// repository-relative paths git knows. Membership here is what separates a
/// genuinely clean `Tracked` file from ignored or untracked content that
/// porcelain never mentions.
fn cached_index_paths(repo_root: &Path) -> Result<BTreeSet<String>, SnapshotViolation> {
    // `-z` emits NUL-terminated repository-relative paths verbatim: no
    // quoting, no escaping, and forward separators on every platform.
    let output = match Command::new("git")
        .args(["ls-files", "--cached", "-z"])
        .current_dir(repo_root)
        .output()
    {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            return Err(SnapshotViolation {
                path: repo_root.display().to_string(),
                reason: format!(
                    "git ls-files --cached failed with exit status {}",
                    output.status
                ),
            })
        }
        Err(error) => {
            return Err(SnapshotViolation {
                path: repo_root.display().to_string(),
                reason: format!("git ls-files --cached must run: {error}"),
            })
        }
    };
    Ok(String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(|entry| lexical_repository_path(Path::new(entry)))
        .collect())
}

/// Lexically format a repository-relative path with `/` separators, dropping
/// the current-directory components that root spellings such as `.` or
/// `./src` introduce. Purely lexical — symlinks are never resolved and the
/// filesystem is never touched — so `.`, `./src`, and `src` identify the
/// same indexed file on both the filesystem and the index side.
fn lexical_repository_path(relative: &Path) -> String {
    relative
        .components()
        .filter(|component| !matches!(component, Component::CurDir))
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Repository-relative spelling of a collected filesystem path, or `None`
/// when the path does not live under `repo_root`.
fn repository_relative(repo_root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(repo_root)
        .ok()
        .map(lexical_repository_path)
}

/// Name a path inside a violation: repository-relative when it lives under
/// the repository, otherwise its raw slash-normalized spelling.
fn violation_path(repo_root: &Path, path: &Path) -> String {
    repository_relative(repo_root, path)
        .unwrap_or_else(|| path.to_string_lossy().replace('\\', "/"))
}

/// Classify one repository-relative path: porcelain stays the authority for
/// every path git reports as changed (untracked, staged, modified); a path
/// absent from porcelain but present in the index is cleanly `Tracked`;
/// everything else — ignored content included — is `Untracked`.
fn classify(
    states: &BTreeMap<String, TrackingState>,
    index: &BTreeSet<String>,
    relative: &str,
) -> TrackingState {
    if let Some(state) = states.get(relative) {
        return *state;
    }
    if index.contains(relative) {
        TrackingState::Tracked
    } else {
        TrackingState::Untracked
    }
}

/// Collect the manifest contribution of one declared evidence root. A
/// directory is traversed (`.git` and `target` stay excluded); a regular
/// file contributes itself; every operational failure becomes a named
/// violation instead of a silent gap.
fn collect_rows(
    repo_root: &Path,
    root: &Path,
    states: &BTreeMap<String, TrackingState>,
    index: &BTreeSet<String>,
    exceptions: &[&str],
    rows: &mut Vec<ManifestRow>,
    violations: &mut Vec<SnapshotViolation>,
) {
    let metadata = match fs::metadata(root) {
        Ok(metadata) => metadata,
        Err(error) => {
            violations.push(SnapshotViolation {
                path: violation_path(repo_root, root),
                reason: format!("declared evidence root is missing or unreadable: {error}"),
            });
            return;
        }
    };
    if metadata.is_file() {
        push_file_row(repo_root, root, states, index, exceptions, rows, violations);
    } else if metadata.is_dir() {
        collect_directory_rows(repo_root, root, states, index, exceptions, rows, violations);
    } else {
        violations.push(SnapshotViolation {
            path: violation_path(repo_root, root),
            reason: "declared evidence root is neither a regular file nor a directory".to_owned(),
        });
    }
}

fn collect_directory_rows(
    repo_root: &Path,
    directory: &Path,
    states: &BTreeMap<String, TrackingState>,
    index: &BTreeSet<String>,
    exceptions: &[&str],
    rows: &mut Vec<ManifestRow>,
    violations: &mut Vec<SnapshotViolation>,
) {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            violations.push(SnapshotViolation {
                path: violation_path(repo_root, directory),
                reason: format!("declared directory cannot be listed: {error}"),
            });
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                violations.push(SnapshotViolation {
                    path: violation_path(repo_root, directory),
                    reason: format!("directory entry cannot be read: {error}"),
                });
                continue;
            }
        };
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                violations.push(SnapshotViolation {
                    path: violation_path(repo_root, &path),
                    reason: format!("file metadata cannot be read: {error}"),
                });
                continue;
            }
        };
        if metadata.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if matches!(name.as_str(), ".git" | "target") {
                continue;
            }
            collect_directory_rows(
                repo_root, &path, states, index, exceptions, rows, violations,
            );
        } else if metadata.is_file() {
            push_file_row(
                repo_root, &path, states, index, exceptions, rows, violations,
            );
        }
    }
}

/// Contribute one regular file as a manifest row, or a named violation when
/// its repository-relative spelling or its content digest cannot be read.
fn push_file_row(
    repo_root: &Path,
    path: &Path,
    states: &BTreeMap<String, TrackingState>,
    index: &BTreeSet<String>,
    exceptions: &[&str],
    rows: &mut Vec<ManifestRow>,
    violations: &mut Vec<SnapshotViolation>,
) {
    let Some(relative) = repository_relative(repo_root, path) else {
        violations.push(SnapshotViolation {
            path: path.to_string_lossy().replace('\\', "/"),
            reason: "collected evidence path is not inside the repository".to_owned(),
        });
        return;
    };
    match try_sha256_file(path) {
        Ok(digest) => rows.push(ManifestRow {
            tracking: classify(states, index, &relative),
            exception: exceptions.contains(&relative.as_str()),
            path: relative,
            digest,
        }),
        Err(error) => violations.push(SnapshotViolation {
            path: relative,
            reason: format!("content cannot be read for its digest: {error}"),
        }),
    }
}
