//! AOI-002: archive-aware history preservation.
//!
//! A completed issue may move from intake to archive with relative-link depth
//! rewrites. An issue archived under the former status-only rule may be
//! restored to the live deferred buffer when its recorded specification
//! already contains a deferred ask. Every other historical difference remains
//! a preservation failure.

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

#[derive(Clone, Debug, Eq, PartialEq)]
enum HeadIssueIdentity {
    Intake { issue_id: String, file_name: String },
    Deferred { issue_id: String, file_name: String },
    Archive { issue_id: String, file_name: String },
}

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
        let issue_identity = issue_identity(head_path);

        if working.is_file() {
            if issue_identity
                .as_ref()
                .is_some_and(|identity| mutable_head_issue(repo_root, identity))
            {
                continue;
            }
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

        // The path is gone from the working tree: only a complete authorized
        // issue move may replace it.
        match issue_identity {
            Some(HeadIssueIdentity::Intake {
                issue_id,
                file_name,
            }) => {
                let deferred_dir = repo_root.join(format!(".arca/issue/deferred/{issue_id}"));
                let violation = if deferred_dir.is_dir() {
                    intake_deferred_move_violation(repo_root, head_path, &issue_id)
                } else {
                    archive_move_violation(
                        repo_root,
                        head_path,
                        &issue_id,
                        &file_name,
                        &head_content,
                    )
                };
                if let Some(violation) = violation {
                    violations.push(violation);
                }
            }
            Some(HeadIssueIdentity::Deferred {
                issue_id,
                file_name: _,
            }) => {
                if let Some(violation) =
                    deferred_selection_violation(repo_root, head_path, &issue_id)
                {
                    violations.push(violation);
                }
            }
            Some(HeadIssueIdentity::Archive {
                issue_id,
                file_name,
            }) => {
                if let Some(violation) = deferred_restore_violation(
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
                reason: "historical file removed without an authorized issue move".to_owned(),
            }),
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Classify one HEAD path in the issue namespace.
fn issue_identity(path: &str) -> Option<HeadIssueIdentity> {
    let rest = path.strip_prefix(".arca/issue/")?;
    let parts: Vec<&str> = rest.split('/').collect();
    match parts.as_slice() {
        [issue_id, file_name] if issue_id.starts_with("i-") => Some(HeadIssueIdentity::Intake {
            issue_id: (*issue_id).to_owned(),
            file_name: (*file_name).to_owned(),
        }),
        ["deferred", issue_id, file_name] if issue_id.starts_with("i-") => {
            Some(HeadIssueIdentity::Deferred {
                issue_id: (*issue_id).to_owned(),
                file_name: (*file_name).to_owned(),
            })
        }
        ["archive", issue_id, file_name] if issue_id.starts_with("i-") => {
            Some(HeadIssueIdentity::Archive {
                issue_id: (*issue_id).to_owned(),
                file_name: (*file_name).to_owned(),
            })
        }
        _ => None,
    }
}

fn mutable_head_issue(repo_root: &Path, identity: &HeadIssueIdentity) -> bool {
    match identity {
        HeadIssueIdentity::Deferred { .. } => true,
        HeadIssueIdentity::Intake { issue_id, .. } => {
            head_issue_status(repo_root, "intake", issue_id) == "pending"
        }
        HeadIssueIdentity::Archive { .. } => false,
    }
}

fn head_issue_status(repo_root: &Path, location: &str, issue_id: &str) -> String {
    let path = match location {
        "intake" => format!(".arca/issue/{issue_id}/index.md"),
        "deferred" => format!(".arca/issue/deferred/{issue_id}/index.md"),
        "archive" => format!(".arca/issue/archive/{issue_id}/index.md"),
        _ => return String::new(),
    };
    let index = head_bytes(repo_root, &path).unwrap_or_default();
    String::from_utf8_lossy(&index)
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("status: \"")
                .and_then(|value| value.strip_suffix('"'))
                .map(str::to_owned)
        })
        .unwrap_or_default()
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

    // A pending intake bundle is mutable while its asks are integrated or
    // rejected, whether changed in place or as part of this move. The
    // destination checks above still require a complete, completed bundle.
    let identity = HeadIssueIdentity::Intake {
        issue_id: issue_id.to_owned(),
        file_name: file_name.to_owned(),
    };
    if mutable_head_issue(repo_root, &identity) {
        return None;
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

fn intake_deferred_move_violation(
    repo_root: &Path,
    head_path: &str,
    issue_id: &str,
) -> Option<ArchiveViolation> {
    let intake_dir = repo_root.join(format!(".arca/issue/{issue_id}"));
    let deferred_dir = repo_root.join(format!(".arca/issue/deferred/{issue_id}"));
    let remaining: Vec<&str> = ISSUE_FILES
        .iter()
        .copied()
        .filter(|name| intake_dir.join(name).is_file())
        .collect();
    let missing: Vec<&str> = ISSUE_FILES
        .iter()
        .copied()
        .filter(|name| !deferred_dir.join(name).is_file())
        .collect();
    if !remaining.is_empty() || !missing.is_empty() {
        return Some(ArchiveViolation {
            path: head_path.to_owned(),
            reason: format!(
                "partial intake-to-deferred move of {issue_id}: remaining [{}], missing [{}]",
                remaining.join(", "),
                missing.join(", ")
            ),
        });
    }
    if head_issue_status(repo_root, "intake", issue_id) != "pending" {
        return Some(ArchiveViolation {
            path: head_path.to_owned(),
            reason: format!("{issue_id} was not pending and cannot enter the deferred buffer"),
        });
    }
    let index = fs::read_to_string(deferred_dir.join("index.md")).unwrap_or_default();
    let spec = fs::read(deferred_dir.join("spec.md")).unwrap_or_default();
    if !index
        .lines()
        .any(|line| line.trim() == "status: \"deferred\"")
        || !has_disposition(&spec, "deferred")
    {
        return Some(ArchiveViolation {
            path: head_path.to_owned(),
            reason: format!(
                "{issue_id} moved to deferred without status deferred and a deferred ask"
            ),
        });
    }
    None
}

fn deferred_selection_violation(
    repo_root: &Path,
    head_path: &str,
    issue_id: &str,
) -> Option<ArchiveViolation> {
    let deferred_dir = repo_root.join(format!(".arca/issue/deferred/{issue_id}"));
    let intake_dir = repo_root.join(format!(".arca/issue/{issue_id}"));
    let remaining: Vec<&str> = ISSUE_FILES
        .iter()
        .copied()
        .filter(|name| deferred_dir.join(name).is_file())
        .collect();
    let missing: Vec<&str> = ISSUE_FILES
        .iter()
        .copied()
        .filter(|name| !intake_dir.join(name).is_file())
        .collect();
    if !remaining.is_empty() || !missing.is_empty() {
        return Some(ArchiveViolation {
            path: head_path.to_owned(),
            reason: format!(
                "partial deferred selection of {issue_id}: remaining [{}], missing [{}]",
                remaining.join(", "),
                missing.join(", ")
            ),
        });
    }
    let index = fs::read_to_string(intake_dir.join("index.md")).unwrap_or_default();
    if !index
        .lines()
        .any(|line| line.trim() == "status: \"pending\"")
    {
        return Some(ArchiveViolation {
            path: head_path.to_owned(),
            reason: format!("{issue_id} selected from deferred without status pending"),
        });
    }
    None
}

fn deferred_restore_violation(
    repo_root: &Path,
    head_path: &str,
    issue_id: &str,
    file_name: &str,
    head_content: &[u8],
) -> Option<ArchiveViolation> {
    let archive_dir = repo_root.join(format!(".arca/issue/archive/{issue_id}"));
    let deferred_dir = repo_root.join(format!(".arca/issue/deferred/{issue_id}"));

    let still_archived: Vec<&str> = ISSUE_FILES
        .iter()
        .copied()
        .filter(|name| archive_dir.join(name).is_file())
        .collect();
    if !still_archived.is_empty() {
        return Some(ArchiveViolation {
            path: head_path.to_owned(),
            reason: format!(
                "partial deferred restoration of {issue_id}: {} still present in archive",
                still_archived.join(", ")
            ),
        });
    }

    let missing: Vec<&str> = ISSUE_FILES
        .iter()
        .copied()
        .filter(|name| !deferred_dir.join(name).is_file())
        .collect();
    if !missing.is_empty() {
        return Some(ArchiveViolation {
            path: head_path.to_owned(),
            reason: format!(
                "partial deferred restoration of {issue_id}: {} missing from deferred destination",
                missing.join(", ")
            ),
        });
    }

    let head_spec_path = format!(".arca/issue/archive/{issue_id}/spec.md");
    let head_spec = head_bytes(repo_root, &head_spec_path).unwrap_or_default();
    if !has_disposition(&head_spec, "deferred") {
        return Some(ArchiveViolation {
            path: head_path.to_owned(),
            reason: format!(
                "{issue_id} has no recorded deferred ask and cannot leave completed archive"
            ),
        });
    }

    let index = fs::read_to_string(deferred_dir.join("index.md")).unwrap_or_default();
    if !index
        .lines()
        .any(|line| line.trim() == "status: \"deferred\"")
    {
        return Some(ArchiveViolation {
            path: head_path.to_owned(),
            reason: format!("{issue_id} restored to deferred without status deferred"),
        });
    }

    let restored = eol_normalized(&fs::read(deferred_dir.join(file_name)).unwrap_or_default());
    let expected = eol_normalized(head_content);
    if deferred_restore_normalized(file_name, &restored) == expected {
        return None;
    }
    Some(ArchiveViolation {
        path: head_path.to_owned(),
        reason:
            "deferred restoration differs from HEAD beyond status and required link-target rewrites"
                .to_owned(),
    })
}

fn has_disposition(bytes: &[u8], wanted: &str) -> bool {
    String::from_utf8_lossy(bytes).lines().any(|line| {
        if !line.trim_start().starts_with('|') {
            return false;
        }
        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().trim_matches('`'))
            .collect();
        let requirement = cells.first().copied().unwrap_or_default();
        requirement.contains('-')
            && requirement
                .chars()
                .any(|character| character.is_ascii_digit())
            && cells.iter().skip(1).any(|cell| *cell == wanted)
    })
}

fn deferred_restore_normalized(file_name: &str, restored: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(restored);
    let mut normalized = String::with_capacity(text.len());
    for line in text.lines() {
        let line = if file_name == "index.md" && line.trim() == "status: \"deferred\"" {
            line.replacen("status: \"deferred\"", "status: \"integrated\"", 1)
        } else {
            line.to_owned()
        };
        normalized.push_str(&line.replace("](../../archive/", "](../"));
        normalized.push('\n');
    }
    if !text.ends_with('\n') {
        normalized.pop();
    }
    normalized.into_bytes()
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
