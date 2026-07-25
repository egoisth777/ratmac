//! PGE-005: the implementation-completion gate.
//!
//! A ticket is passed by evidence, never by a status edit. Every check the
//! ticket declares - its focused planned tests, its hidden lanes, and the
//! quality commands in its Merge Gate - must resolve to a completion receipt
//! that is:
//!
//! - *green*: the recorded run exited zero;
//! - *self-consistent*: the digest re-derives from the recorded output;
//! - *fresh*: the digest of the source roots it claims still matches the tree;
//! - *declared*: its check ID is one the ticket asks for, and no stray receipt
//!   sits beside it.
//!
//! Completion receipt: `.arca/evidence/<ticket-id>/completion/<check>.toml`
//!
//! ```toml
//! ticket-id = "t-047"
//! check-id = "cargo fmt --all -- --check"
//! kind = "quality"                  # focused | hidden-lane | quality
//! command = "cargo fmt --all -- --check"
//! working-dir = "."
//! exit-status = 0
//! output-sha256 = "<sha256 of output>"
//! tree-roots = ["src", "test"]
//! tree-sha256 = "<digest of those roots when the check ran>"
//! output = """
//! """
//! ```
//!
//! The gate verifies receipts instead of executing the checks itself: ETB-001
//! forbids a gate from compiling or rebuilding project source at evaluation
//! time, and re-derivation is what makes the claim auditable.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::receipt::sha256_text;

/// Where one ticket's completion receipts live, under the evidence directory.
pub const COMPLETION_DIR: &str = "completion";

/// What a declared check proves about the ticket.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckKind {
    /// A planned test from the ticket's P4 plan.
    Focused,
    /// A hidden lane from the ticket's P5 manifest.
    HiddenLane,
    /// A command from the ticket's Merge Gate: tests, formatting, lints.
    Quality,
}

impl CheckKind {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "focused" => Some(Self::Focused),
            "hidden-lane" => Some(Self::HiddenLane),
            "quality" => Some(Self::Quality),
            _ => None,
        }
    }
}

impl fmt::Display for CheckKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Focused => "focused",
            Self::HiddenLane => "hidden-lane",
            Self::Quality => "quality",
        })
    }
}

/// One unit of work a ticket declares it will prove.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclaredCheck {
    pub id: String,
    pub kind: CheckKind,
}

/// Why a ticket cannot be passed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionDefect {
    /// The declared check this defect is about.
    pub check: String,
    pub reason: String,
}

impl fmt::Display for CompletionDefect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.check.is_empty() {
            formatter.write_str(&self.reason)
        } else {
            write!(formatter, "{}: {}", self.check, self.reason)
        }
    }
}

fn defect(check: impl Into<String>, reason: impl Into<String>) -> CompletionDefect {
    CompletionDefect {
        check: check.into(),
        reason: reason.into(),
    }
}

/// The checks a ticket declares, in the order completion must prove them:
/// focused tests, then hidden lanes, then Merge Gate commands.
pub fn declared_checks(ticket_source: &str) -> Vec<DeclaredCheck> {
    let mut checks: Vec<DeclaredCheck> = Vec::new();
    let mut push = |id: String, kind: CheckKind| {
        if id.trim().is_empty() || checks.iter().any(|check| check.id == id) {
            return;
        }
        checks.push(DeclaredCheck { id, kind });
    };

    for planned in crate::receipt::planned_tests(ticket_source) {
        push(planned, CheckKind::Focused);
    }
    for lane in hidden_lane_ids(ticket_source) {
        push(lane, CheckKind::HiddenLane);
    }
    for command in merge_gate_commands(ticket_source) {
        push(command, CheckKind::Quality);
    }
    checks
}

/// Hidden lane IDs (`HT-nnn-nn`) named anywhere in the ticket.
fn hidden_lane_ids(ticket_source: &str) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for token in backticked(ticket_source) {
        let mut parts = token.split('-');
        let is_lane = parts.next() == Some("HT")
            && parts.clone().count() == 2
            && parts.all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
        if is_lane && !ids.contains(&token) {
            ids.push(token);
        }
    }
    ids
}

/// Backticked commands inside the ticket's Merge Gate section.
fn merge_gate_commands(ticket_source: &str) -> Vec<String> {
    let Some(section) = ticket_source.split("## Merge Gate").nth(1) else {
        return Vec::new();
    };
    let section = section.split("\n## ").next().unwrap_or(section);
    let mut commands: Vec<String> = Vec::new();
    for token in backticked(section) {
        let trimmed = token.trim().to_owned();
        // A command has a program and at least one argument; bare identifiers
        // in prose (ticket IDs, test IDs, file names) are not commands.
        if trimmed.contains(' ') && !trimmed.contains('\n') && !commands.contains(&trimmed) {
            commands.push(trimmed);
        }
    }
    commands
}

/// Every single-backtick span in a markdown document.
fn backticked(text: &str) -> Vec<String> {
    let mut spans = Vec::new();
    for (index, part) in text.split('`').enumerate() {
        if index % 2 == 1 && !part.is_empty() {
            spans.push(part.to_owned());
        }
    }
    spans
}

/// The file-name form of a check ID.
pub fn check_slug(check: &str) -> String {
    check
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// SHA-256 over the sorted `path\0digest` rows of every file under `roots`.
///
/// This is what makes a receipt falsifiable: change the work, and the receipt
/// that claims to have checked it stops matching.
pub fn tree_digest(root: &Path, roots: &[String]) -> Result<String, String> {
    let mut rows: Vec<String> = Vec::new();
    for relative in roots {
        let base = root.join(relative);
        if !base.exists() {
            return Err(format!("declared source root {relative} does not exist"));
        }
        collect(root, &base, &mut rows)?;
    }
    rows.sort();
    let mut hasher = Sha256::new();
    for row in rows {
        hasher.update(row.as_bytes());
        hasher.update([0]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn collect(root: &Path, path: &Path, rows: &mut Vec<String>) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("unreadable path {}: {error}", shown(path)))?;
    if metadata.is_dir() {
        let entries = fs::read_dir(path)
            .map_err(|error| format!("unreadable directory {}: {error}", shown(path)))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("unreadable entry: {error}"))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            // Build output and VCS internals are not the work being claimed.
            if name == "target" || name == ".git" {
                continue;
            }
            collect(root, &entry.path(), rows)?;
        }
        return Ok(());
    }
    let bytes =
        fs::read(path).map_err(|error| format!("unreadable file {}: {error}", shown(path)))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let relative = path.strip_prefix(root).unwrap_or(path);
    rows.push(format!(
        "{}\0{:x}",
        shown(relative).replace('\\', "/"),
        hasher.finalize()
    ));
    Ok(())
}

/// One completion receipt, as recorded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionReceipt {
    pub ticket: String,
    pub check: String,
    pub kind: CheckKind,
    pub command: String,
    pub working_dir: String,
    pub exit_status: i64,
    pub output: String,
    pub output_sha256: String,
    pub tree_roots: Vec<String>,
    pub tree_sha256: String,
}

/// Read one completion receipt, refusing anything that is not self-describing.
pub fn load_completion(path: &Path) -> Result<CompletionReceipt, CompletionDefect> {
    let anonymous = |reason: String| defect("", reason);
    let source = fs::read_to_string(path)
        .map_err(|error| anonymous(format!("unreadable receipt {}: {error}", shown(path))))?;
    let document: toml::Value = source.parse().map_err(|error| {
        anonymous(format!(
            "receipt {} is not valid TOML: {error}",
            shown(path)
        ))
    })?;

    let text = |key: &str| {
        document
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    let check = text("check-id")
        .ok_or_else(|| anonymous(format!("receipt {} names no check", shown(path))))?;
    let field = |key: &str| {
        text(key).ok_or_else(|| defect(check.clone(), format!("receipt is missing {key:?}")))
    };

    let kind_text = field("kind")?;
    let kind = CheckKind::parse(&kind_text).ok_or_else(|| {
        defect(
            check.clone(),
            format!("unknown check kind {kind_text:?}; expected focused, hidden-lane, or quality"),
        )
    })?;
    let exit_status = document
        .get("exit-status")
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| defect(check.clone(), "receipt is missing \"exit-status\""))?;
    let tree_roots: Vec<String> = document
        .get("tree-roots")
        .and_then(toml::Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    if tree_roots.is_empty() {
        return Err(defect(
            check,
            "receipt declares no \"tree-roots\", so its freshness cannot be checked",
        ));
    }

    Ok(CompletionReceipt {
        ticket: field("ticket-id")?,
        check: check.clone(),
        kind,
        command: field("command")?,
        working_dir: field("working-dir")?,
        exit_status,
        output: field("output")?,
        output_sha256: field("output-sha256")?,
        tree_roots,
        tree_sha256: field("tree-sha256")?,
    })
}

/// PGE-005: the P5 gate. A ticket may be passed only when every declared check
/// carries a green, self-consistent, fresh receipt.
///
/// Defects are reported in declaration order, so the first one names the first
/// missing receipt. The gate writes nothing: a refusal leaves the ticket
/// executing and its residuals unproven.
pub fn gate_completion(root: &Path, ticket_relative: &str) -> Result<(), Vec<CompletionDefect>> {
    let ticket_path = root.join(ticket_relative);
    let source = fs::read_to_string(&ticket_path).map_err(|error| {
        vec![defect(
            "",
            format!("unreadable ticket {ticket_relative}: {error}"),
        )]
    })?;
    let ticket_id = Path::new(ticket_relative)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let directory = crate::receipt::ticket_evidence_dir(root, &ticket_id).join(COMPLETION_DIR);

    let checks = declared_checks(&source);
    let mut defects: Vec<CompletionDefect> = Vec::new();
    // PGE-006: a held ticket is honestly not-passed; completion is not the
    // route out of it.
    if let Some(blocker) = crate::blocked::held_against(&source) {
        defects.push(defect(
            "",
            format!(
                "ticket {ticket_id} is held against {}; a held ticket cannot be passed",
                if blocker.is_empty() {
                    "an unnamed blocker".to_owned()
                } else {
                    blocker
                }
            ),
        ));
    }
    if checks.is_empty() {
        defects.push(defect(
            "",
            format!("ticket {ticket_id} declares no checks, so completion would prove nothing"),
        ));
    }

    // Index the receipts on disk by the check they claim.
    let mut receipts: BTreeMap<String, (PathBuf, CompletionReceipt)> = BTreeMap::new();
    for path in receipt_files(&directory) {
        match load_completion(&path) {
            Ok(receipt) => {
                if let Some((existing, _)) = receipts.get(&receipt.check) {
                    defects.push(defect(
                        receipt.check.clone(),
                        format!(
                            "two receipts claim the same check: {} and {}",
                            shown(existing),
                            shown(&path)
                        ),
                    ));
                    continue;
                }
                receipts.insert(receipt.check.clone(), (path, receipt));
            }
            Err(problem) => defects.push(problem),
        }
    }

    for check in &checks {
        let Some((path, receipt)) = receipts.get(&check.id) else {
            defects.push(defect(
                check.id.clone(),
                format!(
                    "no completion receipt for this {} check; expected {}",
                    check.kind,
                    shown(&directory.join(format!("{}.toml", check_slug(&check.id))))
                ),
            ));
            continue;
        };
        if let Err(problem) = verify_completion(root, &ticket_id, check, receipt) {
            defects.push(problem);
            continue;
        }
        let expected = directory.join(format!("{}.toml", check_slug(&check.id)));
        if path != &expected {
            defects.push(defect(
                check.id.clone(),
                format!(
                    "receipt is filed at {} but this check indexes to {}",
                    shown(path),
                    shown(&expected)
                ),
            ));
        }
    }

    // A receipt for work the ticket never declared is not completion.
    for (check, (path, _)) in &receipts {
        if !checks.iter().any(|declared| &declared.id == check) {
            defects.push(defect(
                check.clone(),
                format!(
                    "receipt {} claims a check the ticket does not declare",
                    shown(path)
                ),
            ));
        }
    }

    if defects.is_empty() {
        Ok(())
    } else {
        Err(defects)
    }
}

/// Check one completion receipt against the ticket and the tree.
fn verify_completion(
    root: &Path,
    ticket_id: &str,
    check: &DeclaredCheck,
    receipt: &CompletionReceipt,
) -> Result<(), CompletionDefect> {
    let named = |reason: String| defect(check.id.clone(), reason);

    if receipt.ticket != ticket_id {
        return Err(named(format!(
            "receipt belongs to ticket {}, not {ticket_id}",
            receipt.ticket
        )));
    }
    if receipt.kind != check.kind {
        return Err(named(format!(
            "receipt records a {} check, but the ticket declares it as {}",
            receipt.kind, check.kind
        )));
    }
    let recomputed = sha256_text(&receipt.output);
    if recomputed != receipt.output_sha256 {
        return Err(named(format!(
            "digest does not re-derive from the recorded output: observed {recomputed}, recorded {}",
            receipt.output_sha256
        )));
    }
    if receipt.exit_status != 0 {
        return Err(named(format!(
            "recorded run exited {}, so this check did not pass",
            receipt.exit_status
        )));
    }
    let program = receipt
        .command
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned();
    if program.is_empty() {
        return Err(named("receipt records no command".to_owned()));
    }
    // ETB-001: the gate never runs the command; it does check that the command
    // names a program that exists, so an unrunnable declaration cannot pass.
    let located = crate::pin::locate_program(root, &program).map_err(|error| {
        named(format!(
            "command {:?} names no program: {error}",
            receipt.command
        ))
    })?;
    if !located.is_file() {
        return Err(named(format!(
            "command {:?} names no program: {} is not an executable file",
            receipt.command,
            shown(&located)
        )));
    }

    let current = tree_digest(root, &receipt.tree_roots).map_err(&named)?;
    if current != receipt.tree_sha256 {
        return Err(named(format!(
            "receipt is stale: it claims tree {}, but the work now hashes to {current}",
            receipt.tree_sha256
        )));
    }
    Ok(())
}

fn receipt_files(directory: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(directory)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("toml")
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
