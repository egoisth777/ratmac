//! ETB-003: the goal revision and its freeze boundary.
//!
//! The goal is the content of `.arca/current/`. Its revision is a content
//! hash over the whole directory - names included - so an added, removed, or
//! renamed file is as visible as an edited one. Run start records the
//! *baseline* revision; the transition that closes intake integration records
//! the *frozen* revision. Between the freeze and batch closure, every
//! transition request re-computes the revision and refuses on a mismatch.

use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

/// The goal directory, relative to the project root.
pub const GOAL_DIR: &str = ".arca/current";

/// Content hash of `.arca/current/`, or `None` when the goal is absent.
///
/// The digest covers every file's path and bytes in sorted order, so it is
/// stable across runs and platforms and sensitive to the directory's shape.
pub fn revision(root: &Path) -> Option<String> {
    let goal = root.join(".arca").join("current");
    if !goal.is_dir() {
        return None;
    }
    let mut files = Vec::new();
    collect(&goal, &goal, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = Sha256::new();
    for (relative, bytes) in &files {
        hasher.update(relative.as_bytes());
        hasher.update(b"\n");
        hasher.update(bytes);
        hasher.update(b"\n");
    }
    Some(format!("{:x}", hasher.finalize()))
}

/// Gather `(relative path, bytes)` for every file under `dir`.
fn collect(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) -> Option<()> {
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        let kind = entry.file_type().ok()?;
        if kind.is_dir() {
            collect(base, &path, out)?;
        } else {
            let relative = path
                .strip_prefix(base)
                .ok()?
                .to_string_lossy()
                .replace('\\', "/");
            out.push((relative, fs::read(&path).ok()?));
        }
    }
    Some(())
}
