//! Durable Run-identifier minting.
//!
//! The record at `<engine-root>/mint.toml` is the one durable high-water mark
//! for the repository-wide Run namespace. It records a non-negative ordinal in
//! TOML's signed 64-bit integer domain, `0..=i64::MAX`, rather than a Run
//! directory name, so removing a directory cannot make its identifier
//! available again.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(unix)]
use std::fs::File;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

use crate::state::StateError;

const RECORD_FILE: &str = "mint.toml";
static TEMP_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

/// Reserve and return the next canonical Run identifier.
///
/// # Preconditions
///
/// The caller **must already hold** the Engine root's `locks/root.lock` for
/// the whole call. Minting intentionally acquires no lock of its own: the
/// Scheduler's root lock serializes this record with roster mutation.
///
/// The stored ordinal is a non-negative TOML integer in `0..=i64::MAX`. If the
/// namespace is exhausted, this returns a named refusal without changing the
/// record. Otherwise, the returned identifier has already been durably
/// recorded. If later Run creation fails, its ordinal remains reserved rather
/// than being reissued.
pub fn next(engine_root: &Path) -> Result<String, StateError> {
    let record_path = engine_root.join(RECORD_FILE);
    let recorded = read_highest(&record_path)?;
    let roster_max = highest_roster_ordinal(engine_root)?;
    let highest = recorded.unwrap_or(roster_max).max(roster_max);
    let next = highest.checked_add(1).ok_or_else(|| {
        StateError::new(format!(
            "mint namespace exhausted: record {} is already at the largest TOML integer ({}); no Run id was minted",
            record_path.display(),
            i64::MAX,
        ))
    })?;

    persist(&record_path, next)?;
    Ok(format!("run-{next:03}"))
}

/// Read the one-key durable record strictly.  A record defect is never
/// interpreted as a zero high-water mark, because doing so could reissue an
/// identifier after a restart.
fn read_highest(path: &Path) -> Result<Option<i64>, StateError> {
    match fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(StateError::new(format!(
                "inspect mint record {}: {error}",
                path.display()
            )))
        }
    }
    let source = fs::read_to_string(path).map_err(|error| {
        StateError::new(format!("read mint record {}: {error}", path.display()))
    })?;
    let document: toml::Value = source.parse().map_err(|error| {
        StateError::new(format!(
            "invalid mint record {}: malformed TOML: {error}",
            path.display()
        ))
    })?;
    let table = document.as_table().ok_or_else(|| {
        StateError::new(format!(
            "invalid mint record {}: expected a top-level table",
            path.display()
        ))
    })?;
    for key in table.keys() {
        if key != "highest" {
            return Err(StateError::new(format!(
                "invalid mint record {}: unknown key {key:?}; only \"highest\" is allowed",
                path.display()
            )));
        }
    }
    let highest = table.get("highest").ok_or_else(|| {
        StateError::new(format!(
            "invalid mint record {}: missing required key \"highest\"",
            path.display()
        ))
    })?;
    let highest = highest.as_integer().ok_or_else(|| {
        StateError::new(format!(
            "invalid mint record {}: \"highest\" must be a non-negative integer",
            path.display()
        ))
    })?;
    if highest < 0 {
        Err(StateError::new(format!(
            "invalid mint record {}: \"highest\" must be a non-negative integer",
            path.display()
        )))
    } else {
        Ok(Some(highest))
    }
}

/// Return the greatest canonical `run-NNN` ordinal in the direct roster.
/// Missing `runs/` is the empty roster; every other read failure is named
/// rather than treated as an empty namespace.
fn highest_roster_ordinal(engine_root: &Path) -> Result<i64, StateError> {
    let runs_dir = engine_root.join("runs");
    match fs::symlink_metadata(&runs_dir) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(StateError::new(format!(
                "inspect Run roster {}: {error}",
                runs_dir.display()
            )))
        }
    }
    let entries = fs::read_dir(&runs_dir).map_err(|error| {
        StateError::new(format!("read Run roster {}: {error}", runs_dir.display()))
    })?;

    let mut highest = 0;
    for entry in entries {
        let entry = entry.map_err(|error| {
            StateError::new(format!("read Run roster {}: {error}", runs_dir.display()))
        })?;
        let file_type = entry.file_type().map_err(|error| {
            StateError::new(format!(
                "inspect Run roster entry {}: {error}",
                entry.path().display()
            ))
        })?;
        if !file_type.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        if let Some(ordinal) = canonical_run_ordinal(&id) {
            highest = highest.max(ordinal);
        }
    }
    Ok(highest)
}

fn canonical_run_ordinal(run_id: &str) -> Option<i64> {
    let digits = run_id.strip_prefix("run-")?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let ordinal = digits.parse::<i64>().ok()?;
    if ordinal == 0 || format!("run-{ordinal:03}") != run_id {
        return None;
    }
    Some(ordinal)
}

// Crash-safety contract:
// - Unix preserves the existing sequence: sync a sibling temporary file,
//   rename it, then sync the parent directory. A crash before the final sync
//   can leave either on-disk version.
// - Windows syncs a sibling temporary file, then requests a write-through
//   replacement. It makes no separate directory-sync or stronger post-crash
//   promise. If that replacement leaves no readable target, the synced
//   temporary record remains for recovery.
#[cfg(not(windows))]
fn persist(path: &Path, highest: i64) -> Result<(), StateError> {
    let parent = path
        .parent()
        .ok_or_else(|| StateError::new("mint record has no parent directory"))?;
    fs::create_dir_all(parent)
        .map_err(|error| StateError::new(format!("create mint record directory: {error}")))?;

    let (temp_path, mut temp) = create_temporary_record(parent)?;
    let result = (|| {
        temp.write_all(format!("highest = {highest}\n").as_bytes())
            .map_err(|error| StateError::new(format!("write temporary mint record: {error}")))?;
        temp.sync_all()
            .map_err(|error| StateError::new(format!("flush temporary mint record: {error}")))?;
        drop(temp);

        match fs::rename(&temp_path, path) {
            Ok(()) => sync_parent(parent)
                .map_err(|error| StateError::new(format!("sync mint record parent: {error}"))),
            Err(_) if path.exists() => {
                replace_existing(&temp_path, path).map_err(|error| {
                    StateError::new(format!("replace mint record {}: {error}", path.display()))
                })?;
                sync_parent(parent)
                    .map_err(|error| StateError::new(format!("sync mint record parent: {error}")))
            }
            Err(error) => Err(StateError::new(format!(
                "replace mint record {}: {error}",
                path.display()
            ))),
        }
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(windows)]
fn persist(path: &Path, highest: i64) -> Result<(), StateError> {
    let parent = path
        .parent()
        .ok_or_else(|| StateError::new("mint record has no parent directory"))?;
    fs::create_dir_all(parent)
        .map_err(|error| StateError::new(format!("create mint record directory: {error}")))?;

    let (temp_path, mut temp) = create_temporary_record(parent)?;
    let write_result = (|| {
        temp.write_all(format!("highest = {highest}\n").as_bytes())
            .map_err(|error| StateError::new(format!("write temporary mint record: {error}")))?;
        temp.sync_all()
            .map_err(|error| StateError::new(format!("flush temporary mint record: {error}")))
    })();
    drop(temp);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    match replace_existing(&temp_path, path) {
        Ok(()) => Ok(()),
        Err(error) if target_has_readable_record(path) => {
            let _ = fs::remove_file(&temp_path);
            Err(StateError::new(format!(
                "replace mint record {} with temporary record {}: {error}; the target remains readable",
                path.display(),
                temp_path.display()
            )))
        }
        Err(error) => Err(StateError::new(format!(
            "mint record replacement uncertain: failed to replace {} with temporary record {}: {error}; {} is missing or unreadable after the failed replacement; the durable mint record is in temporary file {}; preserve that file and recover it as {}",
            path.display(),
            temp_path.display(),
            path.display(),
            temp_path.display(),
            path.display()
        ))),
    }
}

fn create_temporary_record(parent: &Path) -> Result<(PathBuf, fs::File), StateError> {
    const ATTEMPTS: usize = 4_096;
    for _ in 0..ATTEMPTS {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(".mint.toml.tmp-{}-{sequence}", std::process::id()));
        match OpenOptions::new().create_new(true).write(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(StateError::new(format!(
                    "create temporary mint record {}: {error}",
                    path.display()
                )))
            }
        }
    }
    Err(StateError::new(
        "create temporary mint record: exhausted unique temporary names",
    ))
}

#[cfg(all(unix, not(windows)))]
fn sync_parent(parent: &Path) -> std::io::Result<()> {
    File::open(parent)?.sync_all()
}

#[cfg(all(not(unix), not(windows)))]
fn sync_parent(_parent: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn target_has_readable_record(path: &Path) -> bool {
    fs::read_to_string(path).is_ok()
}

#[cfg(windows)]
fn replace_existing(temp: &Path, destination: &Path) -> std::io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let temp: Vec<u16> = temp
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: Both path buffers are valid, nul-terminated UTF-16 strings for
    // the duration of this call.
    let replaced = unsafe {
        MoveFileExW(
            temp.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_existing(temp: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(temp, destination)
}
