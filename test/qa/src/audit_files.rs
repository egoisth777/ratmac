//! Read-only repository audit inputs: the index plus explicitly named extra files.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryKind {
    File,
    Symlink,
    Gitlink,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexEntry {
    pub path: PathBuf,
    pub kind: EntryKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    pub path: PathBuf,
    pub kind: EntryKind,
    /// Working bytes, link text, or no content for a Gitlink boundary.
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Selection {
    pub entries: Vec<Entry>,
    pub extra_inputs: Vec<String>,
    pub exclusions: Vec<String>,
}

fn relative_path(text: &str) -> Result<PathBuf, String> {
    let path = Path::new(text);
    if text.is_empty()
        || text.contains('\0')
        || text.split('/').any(|part| matches!(part, "" | "." | ".."))
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("invalid repository-relative audit path {text:?}"));
    }
    Ok(path.to_owned())
}

/// Parse `git ls-files --cached --stage -z`, retaining index node kinds.
pub fn parse_index_listing(bytes: &[u8]) -> Result<Vec<IndexEntry>, String> {
    if !bytes.is_empty() && bytes.last() != Some(&0) {
        return Err("malformed index listing: missing final NUL separator".to_owned());
    }
    let mut entries = BTreeMap::new();
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    for record in bytes[..bytes.len() - 1].split(|byte| *byte == 0) {
        let tab = record
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or_else(|| "malformed index listing: missing path separator".to_owned())?;
        let header = std::str::from_utf8(&record[..tab])
            .map_err(|_| "malformed index listing: invalid metadata encoding".to_owned())?;
        let fields: Vec<_> = header.split(' ').collect();
        if fields.len() != 3
            || !matches!(fields[1].len(), 40 | 64)
            || !fields[1].bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(format!("malformed index listing metadata {header:?}"));
        }
        let text = std::str::from_utf8(&record[tab + 1..])
            .map_err(|_| "index path is not representable as UTF-8".to_owned())?;
        let path = relative_path(text)?;
        match fields[2] {
            "0" => {}
            "1" | "2" | "3" => {
                return Err(format!("unmerged index conflict at {text:?}"));
            }
            stage => return Err(format!("malformed index stage {stage:?} at {text:?}")),
        }
        let kind = match fields[0] {
            "100644" | "100755" => EntryKind::File,
            "120000" => EntryKind::Symlink,
            "160000" => EntryKind::Gitlink,
            mode => return Err(format!("unsupported index mode {mode:?} at {text:?}")),
        };
        if entries
            .insert(text.to_owned(), IndexEntry { path, kind })
            .is_some()
        {
            return Err(format!("duplicate index path {text:?}"));
        }
    }
    Ok(entries.into_values().collect())
}

fn git(root: &Path) -> Command {
    let mut command = crate::support::command("git", root);
    command.env("GIT_OPTIONAL_LOCKS", "0");
    command
}

fn git_output(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = git(root).args(args).output().map_err(|error| {
        format!(
            "cannot enumerate audit index at {}: {error}",
            root.display()
        )
    })?;
    if !output.status.success() {
        return Err(format!(
            "cannot enumerate audit index at {}: {}",
            root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

fn read_entry(root: &Path, entry: IndexEntry, extra: bool) -> Result<Entry, String> {
    let shown = entry.path.to_string_lossy();
    if entry.kind == EntryKind::Gitlink {
        return Ok(Entry {
            path: entry.path,
            kind: entry.kind,
            bytes: Vec::new(),
        });
    }
    let path = root.join(&entry.path);
    let parent = path
        .parent()
        .expect("a selected path has a repository parent");
    let resolved_parent = fs::canonicalize(parent)
        .map_err(|error| format!("{shown}: cannot inspect audit input: {error}"))?;
    if !resolved_parent.starts_with(root) {
        return Err(format!(
            "{shown}: audit input escapes the repository through its parent"
        ));
    }
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("{shown}: cannot inspect audit input: {error}"))?;
    let bytes = match entry.kind {
        EntryKind::File if metadata.file_type().is_file() => {
            fs::read(&path).map_err(|error| format!("{shown}: cannot read audit input: {error}"))?
        }
        EntryKind::Symlink if !extra && metadata.file_type().is_symlink() => {
            let target = fs::read_link(&path)
                .map_err(|error| format!("{shown}: cannot read indexed link: {error}"))?;
            target
                .to_str()
                .ok_or_else(|| format!("{shown}: indexed link has unrepresentable target text"))?
                .as_bytes()
                .to_vec()
        }
        // Git materializes a link as a regular link-text file when symlinks
        // are disabled, notably on Windows. Never dereference that text.
        EntryKind::Symlink if !extra && metadata.file_type().is_file() => fs::read(&path)
            .map_err(|error| format!("{shown}: cannot read indexed link text: {error}"))?,
        _ => {
            return Err(format!(
                "{shown}: audit input does not have its declared regular-file or link kind"
            ))
        }
    };
    Ok(Entry {
        path: entry.path,
        kind: entry.kind,
        bytes,
    })
}

/// Select indexed working content and an exact caller-supplied extra file list.
/// No ignored directory is walked, and listing or read failures never disappear.
pub fn select(root: &Path, extras: &[PathBuf]) -> Result<Selection, String> {
    let root = fs::canonicalize(root)
        .map_err(|error| format!("cannot resolve audit root {}: {error}", root.display()))?;
    let top = git_output(&root, &["rev-parse", "--show-toplevel"])?;
    let top = std::str::from_utf8(&top)
        .map_err(|_| "Git audit root is not representable as UTF-8".to_owned())?;
    let top = top.strip_suffix('\n').unwrap_or(top);
    let top = top.strip_suffix('\r').unwrap_or(top);
    let top = fs::canonicalize(top)
        .map_err(|error| format!("cannot resolve Git audit root {top:?}: {error}"))?;
    if top != root {
        return Err(format!(
            "audit root {} must be the repository root {}; select there and filter relative paths",
            root.display(),
            top.display()
        ));
    }
    let listing = git_output(
        &root,
        &["ls-files", "--cached", "--stage", "--full-name", "-z"],
    )?;
    let mut indexed: BTreeMap<PathBuf, EntryKind> = parse_index_listing(&listing)?
        .into_iter()
        .map(|entry| (entry.path, entry.kind))
        .collect();
    let mut extra_paths = BTreeSet::new();
    for extra in extras {
        let text = extra
            .to_str()
            .ok_or_else(|| format!("extra audit path {extra:?} is not representable as UTF-8"))?;
        let path = relative_path(text)?;
        if indexed
            .get(&path)
            .is_some_and(|kind| *kind != EntryKind::File)
        {
            return Err(format!(
                "extra audit input {text:?} must be a regular file, not an indexed link or Gitlink"
            ));
        }
        indexed.insert(path.clone(), EntryKind::File);
        extra_paths.insert(path);
    }
    let mut result = Selection::default();
    for (path, kind) in indexed {
        let extra = extra_paths.contains(&path);
        let shown = path
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        if extra {
            result.extra_inputs.push(shown.clone());
        }
        if kind == EntryKind::Gitlink {
            result.exclusions.push(format!(
                "{shown}: Gitlink repository boundary; content not traversed"
            ));
        }
        result
            .entries
            .push(read_entry(&root, IndexEntry { path, kind }, extra)?);
    }
    Ok(result)
}
