//! PGE-004: Engine-owned artifacts are never an agent's job.
//!
//! A State Prompt or gate contract may not instruct an agent to write an
//! Engine-owned file. Per-Run state, evidence, and motion locks live under
//! `.ratmac/runs/<id>/` or `.ratmac/locks/runs/`; project history and the
//! root lock remain under the Engine root. Agent-authored receipts stay in
//! `.ratmac/evidence/`.
//!
//! The audit is executable so the property cannot quietly regress: it takes
//! the already parsed Machine Class and refuses any instruction that pairs a
//! writing verb with an Engine-owned path.

use std::fmt;
use std::fs;
use std::path::Path;

use crate::machine::{GuardKind, MachineClass};

/// Files only the Engine writes (ADR-0003, R-009).
pub const SCHEDULER_OWNED: [&str; 6] = [
    ".ratmac/runs/<id>/run.toml",
    ".ratmac/runs/<id>/evidence.toml",
    ".ratmac/mint.toml",
    ".ratmac/log.md",
    ".ratmac/locks/root.lock",
    ".ratmac/locks/runs/<id>.lock",
];

/// Verbs that turn a mention into an instruction to write.
const WRITE_VERBS: [&str; 10] = [
    "write", "writes", "append", "appends", "edit", "edits", "update", "updates", "record",
    "records",
];

/// One agent-facing instruction: a state prompt, a guard contract, or a
/// template an agent fills in.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instruction {
    pub source: String,
    pub text: String,
}

/// An instruction that gives an agent a Scheduler-owned job.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnershipViolation {
    pub source: String,
    pub path: String,
    pub reason: String,
}

impl fmt::Display for OwnershipViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {} ({})",
            self.source, self.reason, self.path
        )
    }
}

/// Audit instructions for Scheduler-owned writes.
pub fn audit_ownership(instructions: &[Instruction]) -> Result<(), Vec<OwnershipViolation>> {
    let mut violations = Vec::new();
    for instruction in instructions {
        for sentence in sentences(&instruction.text) {
            let lowered = sentence.to_ascii_lowercase();
            for owned in SCHEDULER_OWNED {
                let bare = owned.trim_start_matches(".ratmac/");
                let basename = owned.rsplit('/').next().unwrap_or(owned);
                let mentions_per_run_lock = owned == ".ratmac/locks/runs/<id>.lock"
                    && (lowered.contains(".ratmac/locks/runs/") || lowered.contains("locks/runs/"))
                    && lowered.contains(".lock");
                if !lowered.contains(owned)
                    && !lowered.contains(bare)
                    && !lowered.contains(basename)
                    && !mentions_per_run_lock
                {
                    continue;
                }
                if let Some(verb) = WRITE_VERBS
                    .iter()
                    .find(|verb| contains_word(&lowered, verb))
                {
                    violations.push(OwnershipViolation {
                        source: instruction.source.clone(),
                        path: owned.to_owned(),
                        reason: format!(
                            "instruction tells an agent to {verb} a Scheduler-owned file: {}",
                            sentence.trim()
                        ),
                    });
                }
            }
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Collect the agent-facing instructions of one already parsed Machine Class:
/// every state prompt and every guard contract path.
pub fn runbook_instructions(class: &MachineClass, shown: &str) -> Vec<Instruction> {
    let mut instructions = Vec::new();
    for (name, state) in class.states() {
        instructions.push(Instruction {
            source: format!("{shown} [states.{name}] prompt"),
            text: state.prompt().to_owned(),
        });
        for (index, guard) in state.guards().iter().enumerate() {
            // A gate contract that points at an Engine-owned path makes the
            // agent responsible for it just as surely as a prompt sentence.
            let target = match guard {
                GuardKind::FilesExact { path, .. } | GuardKind::FileContains { path, .. } => {
                    path.as_str()
                }
                _ => "",
            };
            if !target.is_empty() {
                instructions.push(Instruction {
                    source: format!("{shown} [states.{name}] guard {index} path"),
                    text: format!("the gate requires the agent to write {target}"),
                });
            }
        }
    }
    instructions
}

/// Whether a concrete path names an Engine-owned project or per-Run artifact.
pub fn is_scheduler_owned_path(path: &str) -> bool {
    // The comparison below reads a path as text, so it must read it in the
    // one spelling the Engine writes: same owner, same substitution.
    let normalized = crate::root::displayed(path);
    if normalized.ends_with(".ratmac/mint.toml")
        || normalized.ends_with(".ratmac/log.md")
        || normalized.ends_with(".ratmac/locks/root.lock")
    {
        return true;
    }
    if let Some((_, lock_relative)) = normalized.rsplit_once(".ratmac/locks/runs/") {
        return lock_relative.ends_with(".lock")
            && !lock_relative.trim_end_matches(".lock").is_empty()
            && !lock_relative.contains('/');
    }
    let Some((_, run_relative)) = normalized.rsplit_once(".ratmac/runs/") else {
        return false;
    };
    let mut segments = run_relative.split('/');
    matches!(
        (segments.next(), segments.next(), segments.next()),
        (Some(id), Some("run.toml" | "evidence.toml"), None) if !id.is_empty()
    )
}

/// Every `.md` template under a directory tree, as instructions. Templates
/// are forms agents fill in, so they are agent-facing instructions too.
pub fn template_instructions(dir: &Path) -> Vec<Instruction> {
    let mut files = Vec::new();
    collect_markdown(dir, &mut files);
    files.sort();
    files
        .into_iter()
        .filter_map(|path| {
            let text = fs::read_to_string(&path).ok()?;
            Some(Instruction {
                source: crate::root::displayed(path),
                text,
            })
        })
        .collect()
}

fn collect_markdown(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

/// Split text into sentences, so a mention and a verb only pair up when they
/// belong to the same statement. A period is a boundary only when it ends a
/// word, never inside a path like `.ratmac/runs/run-001/run.toml`.
fn sentences(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;
    for (index, character) in text.char_indices() {
        let boundary = match character {
            '\n' | ';' | '!' | '?' => true,
            '.' => bytes
                .get(index + 1)
                .is_none_or(|next| next.is_ascii_whitespace()),
            _ => false,
        };
        if boundary {
            if !text[start..index].trim().is_empty() {
                parts.push(&text[start..index]);
            }
            start = index + character.len_utf8();
        }
    }
    if !text[start..].trim().is_empty() {
        parts.push(&text[start..]);
    }
    parts
}

/// Whole-word match, so "rewritten" is not "write".
fn contains_word(haystack: &str, word: &str) -> bool {
    haystack
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|token| token == word)
}
