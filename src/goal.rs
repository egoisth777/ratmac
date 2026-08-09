//! ETB-003: the goal revision and its freeze boundary.
//!
//! The goal is the content beneath the runbook's declared `goal` root. Its
//! revision is a content hash over the whole directory - names included - so
//! an added, removed, or renamed file is as visible as an edited one. Run
//! start records the *baseline* revision; the transition that closes intake
//! integration records the *frozen* revision. Between the freeze and batch
//! closure, every transition request re-computes the revision and refuses on
//! a mismatch.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

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
            self.path.display(),
            self.detail
        )
    }
}

impl std::error::Error for RevisionError {}

/// Content hash of the resolved goal directory.
///
/// `Ok(None)` means the goal directory is absent. Any directory traversal or
/// file-read failure is an error, never an absent-goal result.
pub fn revision(goal: &Path) -> Result<Option<String>, RevisionError> {
    let metadata = match fs::metadata(goal) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(RevisionError::io("inspect", goal, error)),
    };
    if !metadata.is_dir() {
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

/// Gather `(relative path, bytes)` for every file under `dir`.
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
