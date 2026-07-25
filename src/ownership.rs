//! PGE-004: Scheduler-owned artifacts are never an agent's job.
//!
//! A Phase Prompt or gate contract may not instruct an agent to write a
//! Scheduler-owned file. Agent-authored evidence goes to an agent-writable
//! evidence artifact (`.arca/evidence/`) or through an `rtm` command that
//! performs the Scheduler-owned append itself.
//!
//! The audit is executable so the property cannot quietly regress: it reads
//! the active Runbook's prompts and guard contracts and refuses any
//! instruction that pairs a writing verb with a Scheduler-owned path.

use std::fmt;
use std::fs;
use std::path::Path;

/// Files only the Scheduler writes (ADR-0003, R-009).
pub const SCHEDULER_OWNED: [&str; 3] = [".arca/state.toml", ".arca/log.md", ".arca/rtm.lock"];

/// Verbs that turn a mention into an instruction to write.
const WRITE_VERBS: [&str; 10] = [
    "write", "writes", "append", "appends", "edit", "edits", "update", "updates", "record",
    "records",
];

/// One agent-facing instruction: a phase prompt, a guard contract, or a
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
                let bare = owned.trim_start_matches(".arca/");
                if !lowered.contains(owned) && !lowered.contains(bare) {
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

/// Collect the agent-facing instructions of one Runbook: every phase prompt
/// and every guard contract path.
pub fn runbook_instructions(class_path: &Path) -> Vec<Instruction> {
    let shown = class_path.to_string_lossy().replace('\\', "/");
    let Ok(source) = fs::read_to_string(class_path) else {
        return Vec::new();
    };
    let Ok(document) = source.parse::<toml::Value>() else {
        return Vec::new();
    };
    let mut instructions = Vec::new();
    let Some(phases) = document.get("phases").and_then(toml::Value::as_table) else {
        return instructions;
    };
    for (name, phase) in phases {
        if let Some(prompt) = phase.get("prompt").and_then(toml::Value::as_str) {
            instructions.push(Instruction {
                source: format!("{shown} [phases.{name}] prompt"),
                text: prompt.to_owned(),
            });
        }
        let guards = phase
            .get("guards")
            .and_then(toml::Value::as_array)
            .cloned()
            .unwrap_or_default();
        for (index, guard) in guards.iter().enumerate() {
            // A gate contract that points at a Scheduler-owned path makes the
            // agent responsible for it just as surely as a prompt sentence.
            let target = guard
                .get("path")
                .and_then(toml::Value::as_str)
                .unwrap_or_default();
            if !target.is_empty() {
                instructions.push(Instruction {
                    source: format!("{shown} [phases.{name}] guard {index} path"),
                    text: format!("the gate requires the agent to write {target}"),
                });
            }
        }
    }
    instructions
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
                source: path.to_string_lossy().replace('\\', "/"),
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
/// word, never inside a path like `.arca/state.toml`.
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
