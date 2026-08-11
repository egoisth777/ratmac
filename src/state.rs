use std::fmt;
use std::fs::{self, OpenOptions};

#[cfg(unix)]
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::model::RunState;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const REQUIRED_FIELDS: [&str; 7] = [
    "state",
    "status",
    "goal_revision",
    "input_revision",
    "output_revision",
    "active_refs",
    "blocker",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateError {
    message: String,
    /// DRD-007: when a refusal is the documented format defect, it names its
    /// own code and the doctor relays it instead of re-classifying it.
    code: Option<&'static str>,
}

impl StateError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: None,
        }
    }

    /// A refusal that already knows the documented code it reports under.
    pub(crate) fn coded(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: Some(code),
        }
    }

    pub(crate) fn code(&self) -> Option<&'static str> {
        self.code
    }
}

impl fmt::Display for StateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for StateError {}

/// Whether a Run Record replacement is durable, or reached its destination
/// but could not confirm the parent directory's durability.
#[must_use]
#[derive(Debug)]
pub(crate) enum StateWriteOutcome {
    Durable,
    ReplacedWithParentSyncWarning(std::io::Error),
}

/// Read access to an addressed run's Run Record and Scheduler-mediated writes.
///
/// The Run Record resides inside the run's directory under the resolved Engine
/// root's `.ratmac/runs/<id>/` path; no checkout-local flat Run Record is
/// written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateStore {
    path: PathBuf,
}

impl StateStore {
    pub(crate) fn for_run(root: &Path, run_id: &str) -> Self {
        let engine_root = crate::root::resolve(root).engine_root().to_path_buf();
        Self::for_engine_root(&engine_root, run_id)
    }

    pub(crate) fn for_engine_root(engine_root: &Path, run_id: &str) -> Self {
        Self::at(engine_root.join("runs").join(run_id).join("run.toml"))
    }

    pub(crate) fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// Parse a Run Record strictly, requiring exactly the seven R-025 fields.
    pub fn parse(source: impl AsRef<str>) -> Result<RunState, StateError> {
        let source = source.as_ref();
        let document: toml::Value = source
            .parse()
            .map_err(|error| StateError::new(format!("invalid run.toml: {error}")))?;
        let table = document
            .as_table()
            .ok_or_else(|| StateError::new("invalid run.toml: expected a table"))?;

        for field in REQUIRED_FIELDS {
            if !table.contains_key(field) {
                return Err(StateError::new(format!(
                    "invalid run.toml: missing required field {field}"
                )));
            }
        }
        for key in table.keys() {
            if !REQUIRED_FIELDS.contains(&key.as_str()) {
                return Err(StateError::new(format!(
                    "invalid run.toml: unknown field {key}"
                )));
            }
        }

        toml::from_str(source)
            .map_err(|error| StateError::new(format!("invalid run.toml: {error}")))
    }

    pub fn load(&self) -> Result<RunState, StateError> {
        let source = fs::read_to_string(&self.path)
            .map_err(|error| StateError::new(format!("read run.toml: {error}")))?;
        Self::parse(source)
    }

    pub(crate) fn serialize(state: &RunState) -> Result<Vec<u8>, StateError> {
        toml::to_string(state)
            .map(String::into_bytes)
            .map_err(|error| StateError::new(format!("serialize run.toml: {error}")))
    }

    /// Replace this Run Record. A parent-sync warning means the replacement
    /// happened and callers must not treat the prior State as still current.
    pub(crate) fn write(&self, state: &RunState) -> Result<StateWriteOutcome, StateError> {
        let source = Self::serialize(state)?;
        let parent = self
            .path
            .parent()
            .ok_or_else(|| StateError::new("run.toml has no parent directory"))?;
        fs::create_dir_all(parent)
            .map_err(|error| StateError::new(format!("create state parent: {error}")))?;

        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp_path = parent.join(format!(".run.toml.tmp-{}-{sequence}", std::process::id()));
        let result: Result<StateWriteOutcome, StateError> = (|| {
            let mut temp = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp_path)
                .map_err(|error| StateError::new(format!("create temporary state: {error}")))?;
            temp.write_all(&source)
                .map_err(|error| StateError::new(format!("write temporary state: {error}")))?;
            temp.sync_all()
                .map_err(|error| StateError::new(format!("flush temporary state: {error}")))?;
            drop(temp);

            match fs::rename(&temp_path, &self.path) {
                Ok(()) => {}
                Err(_) if self.path.exists() => {
                    replace_existing(&temp_path, &self.path).map_err(|replace_error| {
                        StateError::new(format!("replace run.toml: {replace_error}"))
                    })?;
                }
                Err(error) => return Err(StateError::new(format!("replace run.toml: {error}"))),
            }
            match sync_parent(parent) {
                Ok(()) => Ok(StateWriteOutcome::Durable),
                Err(error) => Ok(StateWriteOutcome::ReplacedWithParentSyncWarning(error)),
            }
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }
        result
    }
}

#[cfg(unix)]
fn sync_parent(parent: &Path) -> std::io::Result<()> {
    File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent(_parent: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn replace_existing(temp: &Path, destination: &Path) -> std::io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

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
    let replaced = unsafe {
        ReplaceFileW(
            destination.as_ptr(),
            temp.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            std::ptr::null(),
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

/// The rendered State Prompt shown by status.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatePrompt {
    text: String,
}

impl StatePrompt {
    pub(crate) fn new(text: String) -> Self {
        Self { text }
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for StatePrompt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Read-only result rendered by status; it carries state and current guard labels.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusReport {
    pub state: RunState,
    pub pending_guards: Vec<String>,
    pub(crate) state_prompt: StatePrompt,
}

impl StatusReport {
    pub fn state_prompt(&self) -> &StatePrompt {
        &self.state_prompt
    }
}

impl fmt::Display for StatusReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "State: {}", self.state.state)?;
        writeln!(formatter, "Status: {}", self.state.status)?;
        writeln!(formatter, "Goal revision: {}", self.state.goal_revision)?;
        writeln!(formatter, "Input revision: {}", self.state.input_revision)?;
        writeln!(formatter, "Output revision: {}", self.state.output_revision)?;
        writeln!(
            formatter,
            "Active refs: {}",
            self.state.active_refs.join(", ")
        )?;
        writeln!(formatter, "Blocker: {}", self.state.blocker)?;
        for guard in &self.pending_guards {
            writeln!(formatter, "pending guard: {guard}")?;
        }
        Ok(())
    }
}
