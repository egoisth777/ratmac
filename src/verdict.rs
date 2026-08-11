//! Durable transition-input delivery for one addressed Run (FDC-003).

use std::fs;
use std::path::{Path, PathBuf};

use crate::state::StateError;

const LIVE_FILE: &str = "verdict.toml";
const ARCHIVE_DIR: &str = "verdicts";
const ARCHIVE_DIGITS: usize = 6;
const MAX_ARCHIVE_SEQUENCE: u64 = 999_999;
const REQUIRED_FIELDS: [&str; 3] = ["state", "input", "rationale"];

/// One external reviewer's strict transition-input record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerdictRecord {
    input: String,
}

impl VerdictRecord {
    pub(crate) fn input(&self) -> &str {
        &self.input
    }
}

/// A live-record defect rendered through the Scheduler's ordinary refusal
/// surface. Validation never changes or consumes the live record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerdictRefusal {
    observed: String,
    expected: String,
}

impl VerdictRefusal {
    fn new(observed: impl Into<String>, expected: impl Into<String>) -> Self {
        Self {
            observed: observed.into(),
            expected: expected.into(),
        }
    }

    pub(crate) fn observed(&self) -> &str {
        &self.observed
    }

    pub(crate) fn expected(&self) -> &str {
        &self.expected
    }
}

pub(crate) fn live_path(run_dir: &Path) -> PathBuf {
    run_dir.join(LIVE_FILE)
}

pub(crate) fn archive_dir(run_dir: &Path) -> PathBuf {
    run_dir.join(ARCHIVE_DIR)
}

/// Whether anything occupies the live slot. Straight-line movement uses only
/// this metadata check and never reads verdict bytes.
pub(crate) fn live_slot_is_occupied(run_dir: &Path) -> Result<bool, StateError> {
    match fs::symlink_metadata(live_path(run_dir)) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(StateError::new(format!(
            "inspect live verdict slot: {error}"
        ))),
    }
}

/// Strictly read and validate the current live record without changing it.
pub(crate) fn load_live(
    run_dir: &Path,
    current_state: &str,
    legal_inputs: &[String],
) -> Result<VerdictRecord, VerdictRefusal> {
    let path = live_path(run_dir);
    let metadata = fs::symlink_metadata(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            VerdictRefusal::new(
                "no live verdict record",
                format!(
                    "verdict.toml with state {current_state:?}, one legal input, and non-empty rationale"
                ),
            )
        } else {
            VerdictRefusal::new(
                format!("cannot inspect verdict.toml: {error}"),
                "a readable regular verdict.toml",
            )
        }
    })?;
    if !metadata.file_type().is_file() {
        return Err(VerdictRefusal::new(
            "verdict.toml is not a regular file",
            "a regular live verdict record",
        ));
    }

    let source = fs::read_to_string(&path).map_err(|error| {
        VerdictRefusal::new(
            format!("cannot read verdict.toml: {error}"),
            "UTF-8 strict TOML",
        )
    })?;
    parse_and_validate(&source, current_state, legal_inputs)
}

fn parse_and_validate(
    source: &str,
    current_state: &str,
    legal_inputs: &[String],
) -> Result<VerdictRecord, VerdictRefusal> {
    let document = source.parse::<toml::Value>().map_err(|error| {
        VerdictRefusal::new(format!("malformed verdict.toml: {error}"), "strict TOML")
    })?;
    let table = document.as_table().ok_or_else(|| {
        VerdictRefusal::new(
            "verdict.toml is not a top-level table",
            "exactly state, input, and rationale string fields",
        )
    })?;

    for field in REQUIRED_FIELDS {
        if !table.contains_key(field) {
            return Err(VerdictRefusal::new(
                format!("verdict.toml is missing field {field}"),
                "exactly state, input, and rationale string fields",
            ));
        }
    }
    for key in table.keys() {
        if !REQUIRED_FIELDS.contains(&key.as_str()) {
            return Err(VerdictRefusal::new(
                format!("verdict.toml has unknown field {key}"),
                "exactly state, input, and rationale string fields",
            ));
        }
    }

    let string_field = |name: &str| {
        table
            .get(name)
            .and_then(toml::Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| {
                VerdictRefusal::new(
                    format!("verdict.toml field {name} is not a non-empty string"),
                    format!("non-empty string field {name}"),
                )
            })
    };
    let state = string_field("state")?;
    let input = string_field("input")?;
    let rationale = string_field("rationale")?;
    if rationale.trim().is_empty() {
        return Err(VerdictRefusal::new(
            "verdict.toml rationale is blank",
            "a non-whitespace rationale",
        ));
    }
    if state != current_state {
        return Err(VerdictRefusal::new(
            format!("verdict.toml state {state:?} does not match current State {current_state:?}"),
            format!("state = {current_state:?}"),
        ));
    }
    if !legal_inputs.iter().any(|legal| legal == &input) {
        return Err(VerdictRefusal::new(
            format!("verdict.toml input {input:?} is not legal for State {current_state:?}"),
            format!("one of [{}]", legal_inputs.join(", ")),
        ));
    }

    Ok(VerdictRecord { input })
}

/// Atomically consume the already-validated live record by renaming its exact
/// bytes into the next immutable Run-local evidence name.
pub(crate) fn archive_live(run_dir: &Path) -> Result<PathBuf, StateError> {
    let directory = archive_dir(run_dir);
    let mut created = false;
    match fs::symlink_metadata(&directory) {
        Ok(metadata) if metadata.file_type().is_dir() => {}
        Ok(_) => {
            return Err(StateError::new(
                "archive verdict: verdicts is not a real directory",
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(&directory)
                .map_err(|error| StateError::new(format!("create verdicts directory: {error}")))?;
            created = true;
        }
        Err(error) => {
            return Err(StateError::new(format!(
                "inspect verdicts directory: {error}"
            )))
        }
    }

    let result = (|| {
        let sequence = next_sequence(&directory)?;
        let destination = directory.join(format!("{sequence:06}.toml"));
        fs::rename(live_path(run_dir), &destination)
            .map_err(|error| StateError::new(format!("archive verdict.toml: {error}")))?;
        sync_after_rename(run_dir, &directory)
            .map_err(|error| StateError::new(format!("sync archived verdict: {error}")))?;
        Ok(destination)
    })();

    if result.is_err() && created {
        let _ = fs::remove_dir(&directory);
    }
    result
}

fn next_sequence(directory: &Path) -> Result<u64, StateError> {
    let entries = fs::read_dir(directory)
        .map_err(|error| StateError::new(format!("read verdicts directory: {error}")))?;
    let mut maximum = 0_u64;
    for entry in entries {
        let entry = entry
            .map_err(|error| StateError::new(format!("read verdict archive entry: {error}")))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(digits) = name.strip_suffix(".toml") else {
            continue;
        };
        if digits.len() != ARCHIVE_DIGITS || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let Ok(value) = digits.parse::<u64>() else {
            continue;
        };
        if value > 0 {
            maximum = maximum.max(value);
        }
    }
    if maximum >= MAX_ARCHIVE_SEQUENCE {
        return Err(StateError::new(
            "archive verdict: six-digit verdict sequence is exhausted",
        ));
    }
    Ok(maximum + 1)
}

#[cfg(unix)]
fn sync_after_rename(run_dir: &Path, archive_dir: &Path) -> std::io::Result<()> {
    use std::fs::File;

    File::open(archive_dir)?.sync_all()?;
    File::open(run_dir)?.sync_all()
}

#[cfg(not(unix))]
fn sync_after_rename(_run_dir: &Path, _archive_dir: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_record_keeps_exact_values() {
        let legal = vec!["approve".to_owned(), "rework".to_owned()];
        let record = parse_and_validate(
            "state = \"review\"\ninput = \"approve\"\nrationale = \"checked\"\n",
            "review",
            &legal,
        )
        .expect("valid record");
        assert_eq!(record.input(), "approve");
    }

    #[test]
    fn strict_record_rejects_extra_and_blank_fields() {
        let legal = vec!["approve".to_owned()];
        assert!(parse_and_validate(
            "state = \"review\"\ninput = \"approve\"\nrationale = \"ok\"\nextra = \"no\"\n",
            "review",
            &legal,
        )
        .is_err());
        assert!(parse_and_validate(
            "state = \"review\"\ninput = \"approve\"\nrationale = \" \\t\"\n",
            "review",
            &legal,
        )
        .is_err());
    }
}
