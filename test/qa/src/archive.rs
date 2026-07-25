//! AOI-002: archive-aware history preservation.
//!
//! A completed issue folder may move to `.arca/issue/archive/<issue-id>/`
//! with its identity, five-file shape, and content preserved except for the
//! relative-link depth rewrite the extra directory level requires. Every other
//! difference against HEAD — content mutation, partial moves, in-place edits,
//! or archiving an issue that is not completed — is a preservation failure.

use std::fs;
use std::path::Path;
use std::process::Command;

/// History files that legitimately grow in place. Their recorded prefix must
/// never change; only new content may be appended.
pub const APPEND_ONLY: [&str; 1] = [".arca/log.md"];

/// The five files every issue folder must carry.
pub const ISSUE_FILES: [&str; 5] = [
    "index.md",
    "spec.md",
    "design.md",
    "test-plan.md",
    "ubi-lang.md",
];

/// One way history preservation was broken.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveViolation {
    pub path: String,
    pub reason: String,
}

/// Verify that every HEAD path under `history_roots` is preserved, either in
/// place or through a complete authorized archive move.
pub fn verify_history_preservation(
    repo_root: &Path,
    history_roots: &[&str],
) -> Result<(), Vec<ArchiveViolation>> {
    let mut args = vec!["ls-tree", "-r", "--name-only", "HEAD", "--"];
    args.extend_from_slice(history_roots);
    let listing = Command::new("git")
        .args(&args)
        .current_dir(repo_root)
        .output()
        .expect("git ls-tree must run");
    let listing = String::from_utf8_lossy(&listing.stdout);

    let mut violations = Vec::new();
    for head_path in listing.lines().map(str::trim).filter(|p| !p.is_empty()) {
        let head_content = match head_bytes(repo_root, head_path) {
            Some(content) => content,
            None => {
                violations.push(ArchiveViolation {
                    path: head_path.to_owned(),
                    reason: "HEAD content is unreadable".to_owned(),
                });
                continue;
            }
        };

        let working = repo_root.join(head_path);
        if working.is_file() {
            let current = fs::read(&working).unwrap_or_default();
            let current = eol_normalized(&current);
            let recorded = eol_normalized(&head_content);
            if current == recorded {
                continue;
            }
            if APPEND_ONLY.contains(&head_path) && current.starts_with(&recorded) {
                // Append-only growth is the documented way this file changes.
                continue;
            }
            violations.push(ArchiveViolation {
                path: head_path.to_owned(),
                reason: if APPEND_ONLY.contains(&head_path) {
                    "append-only history was rewritten, not appended to".to_owned()
                } else {
                    "historical content changed in place without an authorized archive move"
                        .to_owned()
                },
            });
            continue;
        }

        // The path is gone from the working tree: the only authorized reason
        // is a complete archive move of a completed issue folder.
        match issue_identity(head_path) {
            Some((issue_id, file_name)) => {
                if let Some(violation) = archive_move_violation(
                    repo_root,
                    head_path,
                    &issue_id,
                    &file_name,
                    &head_content,
                ) {
                    violations.push(violation);
                }
            }
            None => violations.push(ArchiveViolation {
                path: head_path.to_owned(),
                reason: "historical file removed without an authorized archive move".to_owned(),
            }),
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// `Some((issue-id, file))` when `path` is `.arca/issue/<issue-id>/<file>`
/// and the folder is not already the archive directory.
fn issue_identity(path: &str) -> Option<(String, String)> {
    let rest = path.strip_prefix(".arca/issue/")?;
    let mut parts = rest.split('/');
    let issue_id = parts.next()?;
    let file_name = parts.next()?;
    if issue_id == "archive" || parts.next().is_some() {
        return None;
    }
    Some((issue_id.to_owned(), file_name.to_owned()))
}

fn head_bytes(repo_root: &Path, path: &str) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .args(["show", &format!("HEAD:{path}")])
        .current_dir(repo_root)
        .output()
        .expect("git show must run");
    output.status.success().then_some(output.stdout)
}

fn archive_move_violation(
    repo_root: &Path,
    head_path: &str,
    issue_id: &str,
    file_name: &str,
    head_content: &[u8],
) -> Option<ArchiveViolation> {
    let archive_dir = repo_root.join(format!(".arca/issue/archive/{issue_id}"));

    // The whole five-file set must move together.
    let missing: Vec<&str> = ISSUE_FILES
        .iter()
        .copied()
        .filter(|name| !archive_dir.join(name).is_file())
        .collect();
    if !missing.is_empty() {
        return Some(ArchiveViolation {
            path: head_path.to_owned(),
            reason: format!(
                "partial archive move of {issue_id}: {} missing from the archive destination",
                missing.join(", ")
            ),
        });
    }

    // Only completed issues may be archived.
    let index = fs::read_to_string(archive_dir.join("index.md")).unwrap_or_default();
    if !(index.contains("status: \"integrated\"") || index.contains("status: \"rejected\"")) {
        return Some(ArchiveViolation {
            path: head_path.to_owned(),
            reason: format!(
                "{issue_id} is not completed: only integrated or rejected issues may be archived"
            ),
        });
    }

    // Content must match except for the relative-link depth rewrite.
    let archived = eol_normalized(&fs::read(archive_dir.join(file_name)).unwrap_or_default());
    let expected = eol_normalized(head_content);
    if archived == expected || link_depth_normalized(&archived) == expected {
        return None;
    }
    Some(ArchiveViolation {
        path: head_path.to_owned(),
        reason: "archived content differs from HEAD beyond relative-link rewrites".to_owned(),
    })
}

/// Compare text by content, not by checkout line endings: a Windows checkout
/// with `core.autocrlf` differs from the LF blob in HEAD without any edit.
fn eol_normalized(bytes: &[u8]) -> Vec<u8> {
    let mut normalized = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
            index += 1;
            continue;
        }
        normalized.push(bytes[index]);
        index += 1;
    }
    normalized
}

/// Undo the one-level-deeper relative link rewrite an archive move requires.
fn link_depth_normalized(archived: &[u8]) -> Vec<u8> {
    String::from_utf8_lossy(archived)
        .replace("](../../../", "](../../")
        .into_bytes()
}
