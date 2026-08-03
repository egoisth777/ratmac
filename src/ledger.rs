//! FDC-011: the spawn ledger - the Scheduler-owned, append/annotate-only
//! per-run record of spawned children, at the `spawn-ledger` path FDC-004
//! reserves under the parent Run's directory.
//!
//! The ledger fixes the join guard's expected set. Entries are appended by
//! `rtm spawn` (and by confirmed respawn, which appends the successor entry
//! naming the superseded id); confirmed abandonment flips only the addressed
//! entry's abandoned mark. Prior entries are never rewritten: every append
//! preserves the file's existing bytes as a prefix, and the annotate edit
//! touches exactly one mark inside exactly one entry. Reads are strict - an
//! unknown key, a malformed entry, or invalid TOML refuses by name rather
//! than yielding a smaller expected set.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::Path;

/// One recorded spawn: the child's identity and everything the parent must
/// remember about it (FDC-011).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerEntry {
    /// The child run id, unique per ledger.
    pub id: String,
    /// The child Machine Class the runbook declared at spawn.
    pub class: String,
    /// The binding values supplied at invocation, keyed by binding name.
    pub bind: BTreeMap<String, String>,
    /// The git revision at spawn; `"none"` when the project has none.
    pub spawned_at: String,
    /// The child workspace path, present only when one is created.
    pub workspace: Option<String>,
    /// Flipped only by human-confirmed abandonment or respawn retirement.
    pub abandoned: bool,
    /// Present only on a successor entry: the run id it supersedes.
    pub supersedes: Option<String>,
}

/// A ledger defect, named. Never a guess and never a silently smaller set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerError {
    message: String,
}

impl LedgerError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LedgerError {}

/// Strictly read every entry. An absent or empty file is an empty ledger
/// (the path is reserved at mint); anything unreadable or malformed refuses.
pub fn read_entries(path: &Path) -> Result<Vec<LedgerEntry>, LedgerError> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(LedgerError::new(format!(
                "spawn ledger {} is unreadable: {error}",
                path.display()
            )))
        }
    };
    if source.trim().is_empty() {
        return Ok(Vec::new());
    }
    let value: toml::Value = source.parse().map_err(|error| {
        LedgerError::new(format!(
            "spawn ledger {} is not valid TOML: {error}",
            path.display()
        ))
    })?;
    let table = value
        .as_table()
        .ok_or_else(|| LedgerError::new("spawn ledger top level must be a table"))?;
    for key in table.keys() {
        if key != "children" {
            return Err(LedgerError::new(format!(
                "spawn ledger declares an unknown top-level key {key:?}; only \"children\" is recorded"
            )));
        }
    }
    let Some(children) = table.get("children") else {
        return Ok(Vec::new());
    };
    let children = children
        .as_array()
        .ok_or_else(|| LedgerError::new("spawn ledger \"children\" must be an array of tables"))?;
    let mut entries = Vec::with_capacity(children.len());
    let mut seen: Vec<String> = Vec::new();
    for (index, child) in children.iter().enumerate() {
        let entry = child.as_table().ok_or_else(|| {
            LedgerError::new(format!("spawn ledger entry {index} must be a table"))
        })?;
        let mut id = None;
        let mut class = None;
        let mut bind = BTreeMap::new();
        let mut spawned_at = None;
        let mut workspace = None;
        let mut abandoned = None;
        let mut supersedes = None;
        for (key, value) in entry {
            match key.as_str() {
                "id" => id = Some(entry_string(index, key, value)?),
                "class" => class = Some(entry_string(index, key, value)?),
                "spawned_at" => spawned_at = Some(entry_string(index, key, value)?),
                "workspace" => workspace = Some(entry_string(index, key, value)?),
                "supersedes" => supersedes = Some(entry_string(index, key, value)?),
                "abandoned" => {
                    abandoned = Some(value.as_bool().ok_or_else(|| {
                        LedgerError::new(format!(
                            "spawn ledger entry {index}: \"abandoned\" must be a boolean"
                        ))
                    })?)
                }
                "bind" => {
                    let values = value.as_table().ok_or_else(|| {
                        LedgerError::new(format!(
                            "spawn ledger entry {index}: \"bind\" must be a table of strings"
                        ))
                    })?;
                    for (name, bound) in values {
                        let bound = bound.as_str().ok_or_else(|| {
                            LedgerError::new(format!(
                                "spawn ledger entry {index}: binding {name:?} must be a string"
                            ))
                        })?;
                        bind.insert(name.clone(), bound.to_owned());
                    }
                }
                other => {
                    return Err(LedgerError::new(format!(
                        "spawn ledger entry {index} declares an unknown key {other:?}"
                    )))
                }
            }
        }
        let id = id.ok_or_else(|| {
            LedgerError::new(format!("spawn ledger entry {index} is missing \"id\""))
        })?;
        if seen.contains(&id) {
            return Err(LedgerError::new(format!(
                "spawn ledger records {id:?} twice; child ids are unique per ledger"
            )));
        }
        seen.push(id.clone());
        entries.push(LedgerEntry {
            class: class.ok_or_else(|| {
                LedgerError::new(format!("spawn ledger entry {index} is missing \"class\""))
            })?,
            bind,
            spawned_at: spawned_at.ok_or_else(|| {
                LedgerError::new(format!(
                    "spawn ledger entry {index} is missing \"spawned_at\""
                ))
            })?,
            workspace,
            abandoned: abandoned.ok_or_else(|| {
                LedgerError::new(format!(
                    "spawn ledger entry {index} is missing \"abandoned\""
                ))
            })?,
            supersedes,
            id,
        });
    }
    Ok(entries)
}

fn entry_string(index: usize, key: &str, value: &toml::Value) -> Result<String, LedgerError> {
    let text = value.as_str().ok_or_else(|| {
        LedgerError::new(format!(
            "spawn ledger entry {index}: {key:?} must be a string"
        ))
    })?;
    if text.trim().is_empty() {
        return Err(LedgerError::new(format!(
            "spawn ledger entry {index}: {key:?} must be non-empty"
        )));
    }
    Ok(text.to_owned())
}

/// Append one entry after the preserved prior bytes. The strict read runs
/// first: a corrupt ledger is never extended, and a duplicate child id
/// refuses before any write.
pub fn append_entry(path: &Path, entry: &LedgerEntry) -> Result<(), LedgerError> {
    let prior = read_entries(path)?;
    if prior.iter().any(|recorded| recorded.id == entry.id) {
        return Err(LedgerError::new(format!(
            "spawn ledger already records {:?}; child ids are unique per ledger",
            entry.id
        )));
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| {
            LedgerError::new(format!(
                "spawn ledger {} cannot open for append: {error}",
                path.display()
            ))
        })?;
    file.write_all(render(entry).as_bytes()).map_err(|error| {
        LedgerError::new(format!(
            "spawn ledger {} cannot append: {error}",
            path.display()
        ))
    })
}

/// Flip exactly one entry's abandoned mark from `false` to `true`, leaving
/// every other byte in place. Returns `false` when no live entry names the
/// child (nothing to annotate). Already-flipped entries are left alone.
pub fn annotate_abandoned(path: &Path, child_id: &str) -> Result<bool, LedgerError> {
    let entries = read_entries(path)?;
    let Some(entry) = entries.iter().find(|entry| entry.id == child_id) else {
        return Ok(false);
    };
    if entry.abandoned {
        return Ok(true);
    }
    let source = fs::read_to_string(path).map_err(|error| {
        LedgerError::new(format!(
            "spawn ledger {} is unreadable: {error}",
            path.display()
        ))
    })?;
    let needle = format!("id = {}", toml_string(child_id));
    let at = source.find(&needle).ok_or_else(|| {
        LedgerError::new(format!(
            "spawn ledger names {child_id:?} but the entry's id line is not found verbatim; refusing a byte edit on unrecognized shape"
        ))
    })?;
    let block_end = source[at..]
        .find("[[children]]")
        .map_or(source.len(), |offset| at + offset);
    let mark = "abandoned = false";
    let mark_at = source[at..block_end].find(mark).ok_or_else(|| {
        LedgerError::new(format!(
            "spawn ledger entry {child_id:?} carries no \"abandoned = false\" mark to flip"
        ))
    })?;
    let mut flipped = String::with_capacity(source.len() + 1);
    flipped.push_str(&source[..at + mark_at]);
    flipped.push_str("abandoned = true");
    flipped.push_str(&source[at + mark_at + mark.len()..]);
    fs::write(path, flipped).map_err(|error| {
        LedgerError::new(format!(
            "spawn ledger {} cannot record the abandoned mark: {error}",
            path.display()
        ))
    })?;
    Ok(true)
}

/// Render one entry as the TOML block `append_entry` writes.
fn render(entry: &LedgerEntry) -> String {
    let mut block = String::from("[[children]]\n");
    block.push_str(&format!("id = {}\n", toml_string(&entry.id)));
    block.push_str(&format!("class = {}\n", toml_string(&entry.class)));
    let bind = entry
        .bind
        .iter()
        .map(|(name, value)| format!("{} = {}", toml_key(name), toml_string(value)))
        .collect::<Vec<_>>()
        .join(", ");
    block.push_str(&format!("bind = {{ {bind} }}\n"));
    block.push_str(&format!(
        "spawned_at = {}\n",
        toml_string(&entry.spawned_at)
    ));
    if let Some(workspace) = &entry.workspace {
        block.push_str(&format!("workspace = {}\n", toml_string(workspace)));
    }
    if let Some(supersedes) = &entry.supersedes {
        block.push_str(&format!("supersedes = {}\n", toml_string(supersedes)));
    }
    block.push_str(&format!("abandoned = {}\n\n", entry.abandoned));
    block
}

/// A TOML basic string: quotes, backslashes, and control characters escaped.
fn toml_string(value: &str) -> String {
    let mut rendered = String::with_capacity(value.len() + 2);
    rendered.push('"');
    for character in value.chars() {
        match character {
            '"' => rendered.push_str("\\\""),
            '\\' => rendered.push_str("\\\\"),
            '\n' => rendered.push_str("\\n"),
            '\r' => rendered.push_str("\\r"),
            '\t' => rendered.push_str("\\t"),
            control if control.is_control() => {
                rendered.push_str(&format!("\\u{:04X}", control as u32));
            }
            plain => rendered.push(plain),
        }
    }
    rendered.push('"');
    rendered
}

/// A TOML key: bare when it needs no quoting, quoted otherwise.
fn toml_key(name: &str) -> String {
    let bare = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if bare {
        name.to_owned()
    } else {
        toml_string(name)
    }
}
