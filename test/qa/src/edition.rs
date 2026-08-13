//! The edition audit - a shared oracle over a repository's `edition-*` tags.
//!
//! `EDN-001` says an edition is an annotated tag named `edition-NNN` whose
//! message records what was proven. The closing guard `EDN-002` can only prove
//! that some tag matching the pattern points at a commit - it says so in its own
//! stated limit. This module is the check that reads the tags themselves.
//!
//! It lives in the harness library rather than inside one test file so that both
//! the public suite and the private adversarial lanes judge the same code: an
//! oracle each suite reimplements would agree with itself by construction.
//!
//! Every operation is a read. Nothing here creates, moves, or deletes a tag.

use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

/// The name pattern that makes a tag a candidate edition.
pub const EDITION_PREFIX: &str = "edition-";

/// The phrases an edition's message must carry for the bar in `EDN-001` to be
/// recorded rather than remembered. Each names a check a reader can re-run.
pub const RECORDED_BAR: [&str; 6] = [
    "Proven at this commit:",
    "cargo test",
    "cargo fmt",
    "cargo clippy",
    "check_links.py",
    "rtm doctor",
];

/// A message recording the bar, for fixtures that need a well-formed edition.
pub const EXAMPLE_BAR_MESSAGE: &str = "edition test\n\nProven at this commit:\n- cargo test --workspace green; cargo fmt --check clean; cargo clippy clean; tools/check_links.py clean\n- rtm doctor exits 0\n";

/// One thing wrong with one edition, named so a reader knows what to fix.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuditFinding {
    pub tag: String,
    pub property: String,
    pub detail: String,
}

impl std::fmt::Display for AuditFinding {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}: {}: {}",
            self.tag, self.property, self.detail
        )
    }
}

/// Run a read-only version-control command, or say why the answer is
/// unavailable. A missing answer is never silently a pass.
pub fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("git {args:?} could not run: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {args:?} exited {}: {}",
            output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "by signal".to_owned()),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Every tag matching the edition pattern, in name order.
pub fn edition_tags(root: &Path) -> Result<Vec<String>, String> {
    let listed = git(root, &["tag", "--list", &format!("{EDITION_PREFIX}*")])?;
    let mut tags: Vec<String> = listed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    tags.sort();
    Ok(tags)
}

/// The line of development an edition must sit on, so the verdict does not
/// depend on which branch a contributor happens to have checked out.
pub fn trunk_name(root: &Path) -> Result<String, String> {
    if git(root, &["rev-parse", "--verify", "--quiet", "main"]).is_ok() {
        return Ok("main".to_owned());
    }
    Err("the repository has no main branch, so reachability has no stated answer".to_owned())
}

/// The commit an edition marks.
pub fn edition_commit(root: &Path, tag: &str) -> Result<String, String> {
    Ok(git(root, &["rev-list", "-n", "1", tag])?.trim().to_owned())
}

/// Audit one repository's editions: annotated, documented, reachable.
///
/// A repository with no editions is a stated pass - it has cut none wrong.
pub fn audit_editions(root: &Path) -> Result<Vec<AuditFinding>, String> {
    let mut findings = Vec::new();
    let tags = edition_tags(root)?;
    if tags.is_empty() {
        return Ok(findings);
    }
    let trunk = trunk_name(root)?;
    for tag in tags {
        let kind = git(root, &["cat-file", "-t", &tag])?.trim().to_owned();
        if kind != "tag" {
            findings.push(AuditFinding {
                tag: tag.clone(),
                property: "annotated".to_owned(),
                detail: format!("the tag object is a {kind}, so nothing records what was proven"),
            });
            // A lightweight tag carries no message at all: the message checks
            // below would only repeat this finding in other words.
            continue;
        }

        let message = git(root, &["tag", "--list", "--format=%(contents)", &tag])?;
        if message.trim().is_empty() {
            findings.push(AuditFinding {
                tag: tag.clone(),
                property: "documented".to_owned(),
                detail: "the message is empty, so the bar is recorded nowhere".to_owned(),
            });
        } else {
            let missing: Vec<&str> = RECORDED_BAR
                .iter()
                .copied()
                .filter(|phrase| !message.contains(phrase))
                .collect();
            if !missing.is_empty() {
                findings.push(AuditFinding {
                    tag: tag.clone(),
                    property: "documented".to_owned(),
                    detail: format!("the message does not record {}", missing.join(", ")),
                });
            }
        }

        let commit = edition_commit(root, &tag)?;
        let reachable = Command::new("git")
            .args(["merge-base", "--is-ancestor", &commit, &trunk])
            .current_dir(root)
            .output()
            .map_err(|error| format!("git merge-base could not run: {error}"))?;
        if !reachable.status.success() {
            findings.push(AuditFinding {
                tag: tag.clone(),
                property: "reachable".to_owned(),
                detail: format!("commit {commit} is not an ancestor of {trunk}"),
            });
        }
    }
    findings.sort();
    Ok(findings)
}

/// Render findings one per line for a failure message.
pub fn report(findings: &[AuditFinding]) -> String {
    let mut text = String::new();
    for finding in findings {
        let _ = writeln!(text, "  {finding}");
    }
    text
}

/// The repository this suite is compiled inside.
pub fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The tracked record of what each edition marks, as `EDN-003` requires. A move
/// is only visible as a difference against something committed.
pub const LEDGER_PATH: &str = ".arca/editions.md";

/// One recorded edition: its number, its name as written, and the commit it was
/// cut at.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LedgerRow {
    pub number: u32,
    pub tag: String,
    pub commit: String,
}

/// Read the committed record. A missing, empty, or unparseable ledger is an
/// error, never an agreement: absence must not read as "nothing moved".
pub fn read_ledger(root: &Path) -> Result<Vec<LedgerRow>, String> {
    let path = root.join(LEDGER_PATH);
    let text = std::fs::read_to_string(&path).map_err(|error| {
        format!("{LEDGER_PATH} is unreadable, so no edition is recorded: {error}")
    })?;
    let mut rows = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("| `edition-") {
            continue;
        }
        let cells: Vec<&str> = trimmed
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().trim_matches('`').trim())
            .collect();
        let (Some(tag), Some(commit)) = (cells.first(), cells.get(1)) else {
            return Err(format!(
                "{LEDGER_PATH}: row {trimmed:?} is not a complete record"
            ));
        };
        let number = edition_number(tag).ok_or_else(|| {
            format!("{LEDGER_PATH}: {tag:?} is not an edition name of the form edition-NNN")
        })?;
        if commit.len() != 40
            || !commit
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            return Err(format!(
                "{LEDGER_PATH}: {tag} records {commit:?}, which is not a whole commit hash"
            ));
        }
        rows.push(LedgerRow {
            number,
            tag: (*tag).to_owned(),
            commit: (*commit).to_owned(),
        });
    }
    if rows.is_empty() {
        return Err(format!("{LEDGER_PATH} records no edition at all"));
    }
    rows.sort();
    Ok(rows)
}

/// The number an edition name carries, or `None` when the name is not exactly
/// `edition-NNN` with three digits. `EDN-001` fixes that shape, so `edition-1`
/// and `edition-0001` are malformed rather than alternative spellings.
pub fn edition_number(tag: &str) -> Option<u32> {
    let digits = tag.strip_prefix(EDITION_PREFIX)?;
    if digits.len() != 3 || !digits.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Audit the sequence and the immutability of a repository's editions:
/// well-formed names, no hole, no duplicate number, and every edition still on
/// the commit the ledger records.
pub fn audit_sequence(root: &Path) -> Result<Vec<AuditFinding>, String> {
    let mut findings = Vec::new();
    if root.exists() {
        return Ok(findings); // RED STUB: no sequence or immutability rule is checked yet
    }

    // A name that resembles an edition but is not one must be reported here,
    // because the pattern the rest of the audit uses will not see it.
    let resembling = git(root, &["tag", "--list", "edition*", "Edition*"])?;
    for tag in resembling
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if edition_number(tag).is_none() {
            findings.push(AuditFinding {
                tag: tag.to_owned(),
                property: "named".to_owned(),
                detail:
                    "the name is not exactly edition-NNN with three digits, so it marks nothing"
                        .to_owned(),
            });
        }
    }

    let tags = edition_tags(root)?;
    let mut numbered: Vec<(u32, String)> = Vec::new();
    for tag in &tags {
        if let Some(number) = edition_number(tag) {
            numbered.push((number, tag.clone()));
        }
    }
    numbered.sort();

    // Duplicates: two names resolving to the same number.
    for window in numbered.windows(2) {
        if window[0].0 == window[1].0 {
            findings.push(AuditFinding {
                tag: window[1].1.clone(),
                property: "sequence".to_owned(),
                detail: format!(
                    "number {:03} is claimed twice, by {} and {}",
                    window[0].0, window[0].1, window[1].1
                ),
            });
        }
    }

    // Holes: the sequence runs from 001 to the highest with nothing skipped.
    if let Some((highest, _)) = numbered.last() {
        let present: std::collections::BTreeSet<u32> =
            numbered.iter().map(|(number, _)| *number).collect();
        for expected in 1..=*highest {
            if !present.contains(&expected) {
                findings.push(AuditFinding {
                    tag: format!("{EDITION_PREFIX}{expected:03}"),
                    property: "sequence".to_owned(),
                    detail: format!(
                        "the sequence reaches {highest:03} with no {expected:03}, so a citation to it resolves to nothing"
                    ),
                });
            }
        }
    }

    // Immutability: the ledger is the committed expectation.
    let ledger = read_ledger(root)?;
    for row in &ledger {
        if !tags.contains(&row.tag) {
            findings.push(AuditFinding {
                tag: row.tag.clone(),
                property: "immutable".to_owned(),
                detail: format!(
                    "{LEDGER_PATH} records commit {} but the tag is gone",
                    row.commit
                ),
            });
            continue;
        }
        let actual = edition_commit(root, &row.tag)?;
        if actual != row.commit {
            findings.push(AuditFinding {
                tag: row.tag.clone(),
                property: "immutable".to_owned(),
                detail: format!(
                    "{LEDGER_PATH} records commit {} but the tag now marks {actual}",
                    row.commit
                ),
            });
        }
    }
    let recorded: std::collections::BTreeSet<&str> =
        ledger.iter().map(|row| row.tag.as_str()).collect();
    for tag in &tags {
        if edition_number(tag).is_some() && !recorded.contains(tag.as_str()) {
            findings.push(AuditFinding {
                tag: tag.clone(),
                property: "immutable".to_owned(),
                detail: format!(
                    "{LEDGER_PATH} records nothing for it, so a move would be invisible"
                ),
            });
        }
    }

    findings.sort();
    Ok(findings)
}
