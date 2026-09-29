//! ETB-003 / WEB-001: the goal revision and its freeze boundary.
//!
//! The goal is the content beneath the runbook's declared `goal` root. Its
//! revision is a content hash over the whole directory - names included - so
//! an added, removed, or renamed file is as visible as an edited one. Run
//! start records the *baseline* revision; the transition that closes intake
//! integration records the *frozen* revision. Between the freeze and batch
//! closure, every transition request re-computes the revision and refuses on
//! a mismatch.
//!
//! Two calculations live here. [`revision`] is the versioned, portable one
//! every newly started Run records: relative paths use forward slashes,
//! entries are ordered by bytewise path order, records are length-delimited,
//! and CRLF pairs in text - valid UTF-8 without a NUL byte - become LF while
//! every other content byte keeps its meaning. [`legacy_revision`] is the
//! original unversioned calculation a Run keeps for its whole life when its
//! recorded baseline is a bare digest or records no version at all: the two
//! grammars never verify each other's stamps. The revision string's own
//! form - `v2:` plus 64 lowercase hexadecimal digits, or a bare 64-digit
//! digest - selects the calculation ([`Version`]); no record carries a
//! separate version field.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::root::Displayed;
use sha2::{Digest, Sha256};

/// A failure while reading a declared goal directory.
#[derive(Debug)]
pub struct RevisionError {
    operation: &'static str,
    path: PathBuf,
    detail: String,
}

impl RevisionError {
    fn io(operation: &'static str, path: &Path, error: std::io::Error) -> Self {
        Self {
            operation,
            path: path.to_path_buf(),
            detail: error.to_string(),
        }
    }

    fn other(operation: &'static str, path: &Path, detail: impl Into<String>) -> Self {
        Self {
            operation,
            path: path.to_path_buf(),
            detail: detail.into(),
        }
    }
}

impl fmt::Display for RevisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot {} declared goal {}: {}",
            self.operation,
            self.path.displayed(),
            self.detail
        )
    }
}

impl std::error::Error for RevisionError {}

/// The fingerprint version a revision string's own form selects.
///
/// `v2:` plus exactly 64 lowercase hexadecimal digits selects the portable
/// calculation; a bare 64-digit digest selects the legacy one. Anything else
/// matches neither grammar and selects nothing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Version {
    /// The original unversioned calculation: raw bytes, newline delimited.
    Legacy,
    /// The portable calculation: length-framed, forward-slash paths.
    V2,
}

impl Version {
    /// The version `text` selects, or `None` when it matches neither
    /// revision grammar.
    pub fn of_revision(text: &str) -> Option<Self> {
        let digest = match text.strip_prefix("v2:") {
            Some(digest) => digest,
            None => text,
        };
        (digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()))
        .then_some(if text.starts_with("v2:") {
            Self::V2
        } else {
            Self::Legacy
        })
    }

    /// The version a Run's recorded baseline selects for its whole life.
    ///
    /// A baseline that is absent or empty recorded no version, so the legacy
    /// calculation stays selected; a baseline matching neither grammar
    /// selects nothing and every comparison that needs a version refuses.
    pub fn of_baseline(baseline: Option<&str>) -> Option<Self> {
        match baseline.filter(|text| !text.is_empty()) {
            None => Some(Self::Legacy),
            Some(text) => Self::of_revision(text),
        }
    }
}

/// Resolve the goal directory: `Ok(None)` means absent or not a directory,
/// exactly today's absent-goal behavior; any other inspection failure is an
/// error, never an absent-goal result.
fn goal_directory(goal: &Path) -> Result<Option<()>, RevisionError> {
    let metadata = match fs::metadata(goal) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(RevisionError::io("inspect", goal, error)),
    };
    if !metadata.is_dir() {
        return Ok(None);
    }
    Ok(Some(()))
}

/// The portable (v2) content hash of the resolved goal directory.
///
/// `Ok(None)` means the goal directory is absent. The hashed bytes are the
/// canonical goal serialization: the ASCII bytes `ratmac-goal-v2`, one NUL
/// byte, the file count as an unsigned 64-bit big-endian integer, then per
/// file - in ascending bytewise order of the relative-path bytes - the
/// unsigned 64-bit big-endian path length, the UTF-8 forward-slash path
/// bytes, the unsigned 64-bit big-endian canonical-content length, and those
/// content bytes. No delimiter, terminator, or padding byte is added.
///
/// A goal entry that is not a regular file, or a relative path that is not
/// valid UTF-8, refuses rather than following a host-specific target or
/// converting lossily.
pub fn revision(goal: &Path) -> Result<Option<String>, RevisionError> {
    if goal_directory(goal)?.is_none() {
        return Ok(None);
    }
    let mut files = Vec::new();
    collect_entries(goal, goal, &mut files)?;
    // The seam and every refusal name the first offender in ascending
    // bytewise order of the displayed relative-path bytes, whatever order
    // the host enumerated.
    files.sort_by(|left, right| left.0.cmp(&right.0));
    inject_goal_fault(goal, files.first().map(|file| file.0.as_str()))?;
    for (displayed, slashes, _, regular) in &files {
        if !regular {
            return Err(RevisionError::other(
                "serialize",
                goal,
                format!(
                    "entry {displayed} is not a regular file; \
                     refusing rather than following a host-specific target"
                ),
            ));
        }
        if slashes.is_none() {
            return Err(RevisionError::other(
                "serialize",
                goal,
                format!("relative path {displayed} is not valid UTF-8; refusing lossy conversion"),
            ));
        }
    }
    // The serialized order is the relative-path bytes' own - the exact
    // bytes each entry hashes - never the displayed spelling's.
    files.sort_by(|left, right| left.1.cmp(&right.1));

    let mut hasher = Sha256::new();
    hasher.update(b"ratmac-goal-v2");
    hasher.update([0]);
    hasher.update((files.len() as u64).to_be_bytes());
    for (displayed, slashes, path, _) in &files {
        let Some(relative) = slashes.as_deref() else {
            return Err(RevisionError::other(
                "serialize",
                goal,
                format!("relative path {displayed} is not valid UTF-8; refusing lossy conversion"),
            ));
        };
        let bytes = fs::read(path).map_err(|error| RevisionError::io("read", path, error))?;
        hasher.update((relative.len() as u64).to_be_bytes());
        hasher.update(relative.as_bytes());
        let canonical = canonical_content(&bytes);
        hasher.update((canonical.len() as u64).to_be_bytes());
        hasher.update(canonical.as_ref());
    }
    Ok(Some(format!("v2:{:x}", hasher.finalize())))
}

/// The legacy (unversioned) content hash of the resolved goal directory,
/// exactly the calculation a Run whose recorded baseline is a bare digest
/// keeps for its whole life: gather every entry under the goal root
/// (following links, converting paths lossily, as this producer always has),
/// sort by relative path, hash `path bytes, LF, content bytes, LF` per file,
/// and render 64 lowercase hexadecimal digits.
pub fn legacy_revision(goal: &Path) -> Result<Option<String>, RevisionError> {
    if goal_directory(goal)?.is_none() {
        return Ok(None);
    }
    let mut files = Vec::new();
    collect(goal, goal, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = Sha256::new();
    for (relative, bytes) in &files {
        hasher.update(relative.as_bytes());
        hasher.update(b"\n");
        hasher.update(bytes);
        hasher.update(b"\n");
    }
    Ok(Some(format!("{:x}", hasher.finalize())))
}

/// Gather `(relative path, bytes)` for every file under `dir`, reading any
/// non-directory entry the host offers - the legacy calculation's own
/// traversal semantics, unchanged.
fn collect(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) -> Result<(), RevisionError> {
    let entries = fs::read_dir(dir).map_err(|error| RevisionError::io("read", dir, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| RevisionError::io("enumerate", dir, error))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| RevisionError::io("inspect", &path, error))?;
        if kind.is_dir() {
            collect(base, &path, out)?;
        } else {
            let relative =
                crate::root::displayed(path.strip_prefix(base).map_err(|error| {
                    RevisionError::other("relativize", &path, error.to_string())
                })?);
            let bytes = fs::read(&path).map_err(|error| RevisionError::io("read", &path, error))?;
            out.push((relative, bytes));
        }
    }
    Ok(())
}

/// Gather `(displayed relative path, forward-slash relative-path bytes,
/// path, is a regular file)` for every non-directory entry under `dir`.
/// Directories recurse; nothing is read yet, so regularity and path
/// encoding are judged before any content byte is touched. The path bytes
/// are `None` when the relative path is not valid UTF-8; the displayed
/// spelling stays for refusal naming.
fn collect_entries(
    base: &Path,
    dir: &Path,
    out: &mut Vec<(String, Option<String>, PathBuf, bool)>,
) -> Result<(), RevisionError> {
    let entries = fs::read_dir(dir).map_err(|error| RevisionError::io("read", dir, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| RevisionError::io("enumerate", dir, error))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| RevisionError::io("inspect", &path, error))?;
        if kind.is_dir() {
            collect_entries(base, &path, out)?;
        } else {
            let stripped = path
                .strip_prefix(base)
                .map_err(|error| RevisionError::other("relativize", &path, error.to_string()))?;
            let relative = crate::root::displayed(stripped);
            let slashes = forward_slashes(stripped);
            out.push((relative, slashes, path, kind.is_file()));
        }
    }
    Ok(())
}

/// The relative path as forward-slash UTF-8 bytes, or `None` when any
/// component is not valid UTF-8.
fn forward_slashes(relative: &Path) -> Option<String> {
    let mut text = String::new();
    for component in relative.components() {
        let std::path::Component::Normal(part) = component else {
            continue;
        };
        let part = part.to_str()?;
        if !text.is_empty() {
            text.push('/');
        }
        text.push_str(part);
    }
    Some(text)
}

/// The canonical form of one file's content: text - valid UTF-8 with no NUL
/// byte - keeps every byte except that each CRLF pair becomes LF; every
/// other file is binary and keeps its raw bytes.
fn canonical_content(bytes: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    match std::str::from_utf8(bytes) {
        Ok(text) if !text.contains('\0') => {
            if text.contains("\r\n") {
                std::borrow::Cow::Owned(text.replace("\r\n", "\n").into_bytes())
            } else {
                std::borrow::Cow::Borrowed(bytes)
            }
        }
        _ => std::borrow::Cow::Borrowed(bytes),
    }
}

/// The QA seam for host conditions this build cannot always create for
/// real: `unrepresentable-path` reports the non-UTF-8 refusal and
/// `non-regular-entry` the non-regular refusal, each as if the next
/// enumerated entry - the first file in ascending bytewise order of
/// displayed relative-path bytes - were the offender. Compiled only with
/// `test-fault-injection`; unset, empty, or unrecognized values - or a goal
/// root holding no file - change nothing, and the seam fires at most once
/// per invocation.
#[cfg(feature = "test-fault-injection")]
fn inject_goal_fault(goal: &Path, first: Option<&str>) -> Result<(), RevisionError> {
    use std::sync::atomic::{AtomicBool, Ordering};
    static FIRED: AtomicBool = AtomicBool::new(false);
    let Some(name) = first else {
        return Ok(());
    };
    if FIRED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let detail = match std::env::var("RATMAC_TEST_GOAL_FAULT").ok().as_deref() {
        Some("unrepresentable-path") => {
            format!("relative path {name} is not valid UTF-8; refusing lossy conversion")
        }
        Some("non-regular-entry") => {
            format!(
                "entry {name} is not a regular file; \
                 refusing rather than following a host-specific target"
            )
        }
        _ => return Ok(()),
    };
    Err(RevisionError::other("serialize", goal, detail))
}

/// A build without `test-fault-injection` injects nothing.
#[cfg(not(feature = "test-fault-injection"))]
fn inject_goal_fault(_goal: &Path, _first: Option<&str>) -> Result<(), RevisionError> {
    Ok(())
}
