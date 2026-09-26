//! WRS-005: the descriptive Run roster.
//!
//! Every refusal that prints the roster summary (`runs: run-001, run-002`)
//! follows its first line with one row per entry: the address, its recorded
//! top-level or child role, the parent, class, bindings, and supersession a
//! spawn ledger records, then its State and status when its Run Record
//! reads. Retired, missing, and unreadable records, damaged ledgers, and
//! stray directories are labeled instead of omitted or guessed.
//!
//! The rows come only from Engine-owned records - each Run Record, each
//! roster member's spawn ledger, and the transition log - each read once.
//! No runbook is read, no guard runs, and nothing is written. Every stored
//! value is rendered either bare (a plain name) or quoted, so no stored byte
//! can end a row or start one.

use crate::ledger::LedgerEntry;
use crate::root::Displayed;
use crate::scheduler::Scheduler;
use crate::state::StateStore;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// The part of a roster summary after `runs: `: the rendered names, or
/// `none_text` when `.ratmac/runs/` is absent or lists nothing, or an
/// explicit `unreadable (...)` when it exists but cannot be listed.
pub(crate) fn summary(engine_root: &Path, roster: &[String], none_text: &str) -> String {
    if !roster.is_empty() {
        return roster
            .iter()
            .map(|name| name_text(name))
            .collect::<Vec<_>>()
            .join(", ");
    }
    let runs = engine_root.join("runs");
    crate::observe::operation("roster");
    match fs::read_dir(&runs) {
        Err(error) if error.kind() != ErrorKind::NotFound => format!(
            "unreadable ({}: {})",
            path_text(&runs),
            quoted(&error.to_string())
        ),
        _ => none_text.to_owned(),
    }
}

/// One line per roster entry, each preceded by a newline, ready to append
/// to a refusal's first line. Empty when there is nothing to describe.
pub(crate) fn rows(engine_root: &Path, roster: &[String]) -> String {
    Records::read(engine_root, roster).rows(roster)
}

/// Whether a Run whose Run Record is absent was retired.
pub(crate) enum Retirement {
    Recorded,
    NotRecorded,
    /// Retirement cannot be told: the rendered reason.
    Unknown(String),
}

/// The Engine-owned records a roster description reads, each read once.
pub(crate) struct Records {
    runs: PathBuf,
    /// Every child a readable spawn ledger records: child id to each
    /// recording parent with its entry, parents in roster order.
    children: BTreeMap<String, Vec<(String, LedgerEntry)>>,
    /// One rendered clause per spawn ledger that did not read.
    unreadable_ledgers: Vec<String>,
    /// The transition log text, or the rendered reason it did not read.
    log: Result<String, String>,
}

impl Records {
    pub(crate) fn read(engine_root: &Path, roster: &[String]) -> Self {
        let runs = engine_root.join("runs");
        let mut children: BTreeMap<String, Vec<(String, LedgerEntry)>> = BTreeMap::new();
        let mut unreadable_ledgers = Vec::new();
        for parent in roster {
            let path = runs.join(parent).join("spawn-ledger");
            match crate::ledger::read_entries(&path) {
                Ok(entries) => {
                    for entry in entries {
                        children
                            .entry(entry.id.clone())
                            .or_default()
                            .push((parent.clone(), entry));
                    }
                }
                Err(error) => unreadable_ledgers.push(format!(
                    "unreadable spawn ledger {}: {}",
                    path_text(&path),
                    quoted(&error.to_string())
                )),
            }
        }
        let shown = path_text(&engine_root.join("log.md"));
        let log = Scheduler::transition_log_path(engine_root)
            .map_err(|error| error.to_string())
            .and_then(|path| match fs::read(&path) {
                Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(String::new()),
                Err(error) => Err(error.to_string()),
            })
            .map_err(|reason| format!("transition log {shown} is unreadable: {}", quoted(&reason)));
        Self {
            runs,
            children,
            unreadable_ledgers,
            log,
        }
    }

    /// Retirement of `id` as recorded: its abandonment event in the
    /// transition log, or the abandoned mark on its spawn-ledger entry.
    pub(crate) fn retirement(&self, id: &str) -> Retirement {
        if self.marked_abandoned(id) {
            return Retirement::Recorded;
        }
        match &self.log {
            Ok(log) => {
                let event = format!("- Abandoned: Run {id} retired at state ");
                if log.lines().any(|line| line.starts_with(&event)) {
                    Retirement::Recorded
                } else {
                    Retirement::NotRecorded
                }
            }
            Err(reason) => Retirement::Unknown(reason.clone()),
        }
    }

    fn marked_abandoned(&self, id: &str) -> bool {
        self.children
            .get(id)
            .is_some_and(|recorded| recorded.iter().any(|(_, entry)| entry.abandoned))
    }

    fn rows(&self, roster: &[String]) -> String {
        let names: BTreeSet<&str> = roster
            .iter()
            .map(String::as_str)
            .chain(self.children.keys().map(String::as_str))
            .collect();
        let mut text = String::new();
        for name in names {
            text.push_str("\n  ");
            text.push_str(&name_text(name));
            text.push_str(": ");
            text.push_str(&self.fields(name, roster.iter().any(|entry| entry == name)));
        }
        text
    }

    fn fields(&self, id: &str, listed: bool) -> String {
        if listed && !Scheduler::is_canonical_run_id(id) {
            return "not a canonical run address; not addressable".to_owned();
        }
        let mut fields = vec![self.role(id)];
        let dir = self.runs.join(id);
        if !listed {
            fields.push(format!("not on the roster ({} is absent)", path_text(&dir)));
            return fields.join("; ");
        }
        let path = dir.join("run.toml");
        crate::observe::operation("record");
        match fs::read(&path) {
            Ok(bytes) => match String::from_utf8(bytes)
                .map_err(|error| error.to_string())
                .and_then(|source| StateStore::parse(source).map_err(|error| error.to_string()))
            {
                Ok(record) => {
                    fields.push(format!("state {}", name_text(&record.state)));
                    fields.push(format!("status {}", record.status));
                    if self.marked_abandoned(id) {
                        fields.push("marked abandoned in its parent's spawn ledger".to_owned());
                    }
                }
                Err(reason) => fields.push(unreadable_record(&path, &reason)),
            },
            Err(error) if error.kind() == ErrorKind::NotFound => {
                fields.push(match self.retirement(id) {
                    Retirement::Recorded => "retired".to_owned(),
                    Retirement::NotRecorded => format!(
                        "Run Record missing ({} is absent and no retirement is recorded)",
                        path_text(&path)
                    ),
                    Retirement::Unknown(reason) => {
                        format!("Run Record absent (retirement unknown: {reason})")
                    }
                });
            }
            Err(error) => fields.push(unreadable_record(&path, &error.to_string())),
        }
        fields.join("; ")
    }

    fn role(&self, id: &str) -> String {
        match self.children.get(id).map(Vec::as_slice) {
            Some([(parent, entry)]) => {
                let mut role = format!(
                    "child of {}; class {}",
                    name_text(parent),
                    name_text(&entry.class)
                );
                if !entry.bind.is_empty() {
                    let bindings = entry
                        .bind
                        .iter()
                        .map(|(name, value)| format!("{}={}", name_text(name), quoted(value)))
                        .collect::<Vec<_>>()
                        .join(", ");
                    role.push_str(&format!("; bind {bindings}"));
                }
                if let Some(superseded) = &entry.supersedes {
                    role.push_str(&format!("; supersedes {}", name_text(superseded)));
                }
                role
            }
            Some(recorded) => {
                // Sort the stored names, then render: quoting must not
                // change the byte order the contract promises.
                let mut parents: Vec<&str> =
                    recorded.iter().map(|(parent, _)| parent.as_str()).collect();
                parents.sort();
                format!(
                    "role unknown (recorded as a child by {})",
                    parents
                        .iter()
                        .map(|parent| name_text(parent))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            None if self.unreadable_ledgers.is_empty() => "top-level".to_owned(),
            None => format!("role unknown ({})", self.unreadable_ledgers.join(", ")),
        }
    }
}

fn unreadable_record(path: &Path, reason: &str) -> String {
    format!(
        "Run Record unreadable ({}: {})",
        path_text(path),
        quoted(reason)
    )
}

/// A plain name stays bare; anything else is quoted.
fn name_text(name: &str) -> String {
    let plain = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if plain {
        name.to_owned()
    } else {
        quoted(name)
    }
}

/// Rust debug quoting: `"`, `\`, and every control character escaped.
fn quoted(value: &str) -> String {
    format!("{value:?}")
}

/// A path in the Engine's forward-slash spelling, control characters escaped.
fn path_text(path: &Path) -> String {
    path.displayed()
        .chars()
        .map(|character| {
            if character.is_control() {
                character.escape_debug().to_string()
            } else {
                character.to_string()
            }
        })
        .collect()
}
