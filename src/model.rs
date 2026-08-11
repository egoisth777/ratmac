use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::graph::State;

/// Lifecycle metadata recorded alongside a Run's machine State.
#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Planned,
    Executing,
    Blocked,
    Passed,
    Failed,
}

impl Status {
    pub const ALL: [Self; 5] = [
        Self::Planned,
        Self::Executing,
        Self::Blocked,
        Self::Passed,
        Self::Failed,
    ];
}

impl fmt::Display for Status {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Planned => "planned",
            Self::Executing => "executing",
            Self::Blocked => "blocked",
            Self::Passed => "passed",
            Self::Failed => "failed",
        };
        formatter.write_str(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusParseError {
    value: String,
}

impl fmt::Display for StatusParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid status {:?}; expected one of planned, executing, blocked, passed, failed",
            self.value
        )
    }
}

impl std::error::Error for StatusParseError {}

impl FromStr for Status {
    type Err = StatusParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "planned" => Ok(Self::Planned),
            "executing" => Ok(Self::Executing),
            "blocked" => Ok(Self::Blocked),
            "passed" => Ok(Self::Passed),
            "failed" => Ok(Self::Failed),
            _ => Err(StatusParseError {
                value: value.to_owned(),
            }),
        }
    }
}

/// Paths owned by a started Run under the resolved Engine root's
/// `.ratmac/runs/<id>/` directory and Run-scoped lock namespace.
///
/// The Run Record and Run evidence reside in the run directory; the history
/// log stays at the Engine root, while this Run's transient motion lock stays
/// under `.ratmac/locks/runs/`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunArtifacts {
    run_dir: PathBuf,
    record_path: PathBuf,
    log_path: PathBuf,
    lock_path: PathBuf,
}

impl RunArtifacts {
    fn for_run(root: &Path, run_id: &str) -> Self {
        let engine_root = crate::root::resolve(root).engine_root().to_path_buf();
        let run_dir = engine_root.join("runs").join(run_id);
        Self {
            record_path: run_dir.join("run.toml"),
            run_dir,
            log_path: engine_root.join("log.md"),
            lock_path: crate::lock::run_path(&engine_root, run_id),
        }
    }

    pub fn run_dir(&self) -> &Path {
        &self.run_dir
    }

    pub fn record_path(&self) -> &Path {
        &self.record_path
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }
}

/// A Run's state-local lifecycle record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Run {
    state: State,
    status: Status,
    blocker: Option<String>,
    id: Option<String>,
    artifacts: Option<RunArtifacts>,
}

impl Run {
    pub fn new(state: impl Into<State>, status: Status) -> Self {
        Self {
            state: state.into(),
            status,
            blocker: None,
            id: None,
            artifacts: None,
        }
    }

    /// The minted run id, present once the Run resides on the roster.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn blocker(&self) -> Option<&str> {
        self.blocker.as_deref()
    }

    pub fn artifacts(&self) -> Option<&RunArtifacts> {
        self.artifacts.as_ref()
    }

    pub fn state_path(&self) -> Option<&Path> {
        self.artifacts.as_ref().map(RunArtifacts::record_path)
    }

    pub fn log_path(&self) -> Option<&Path> {
        self.artifacts.as_ref().map(RunArtifacts::log_path)
    }

    pub fn lock_path(&self) -> Option<&Path> {
        self.artifacts.as_ref().map(RunArtifacts::lock_path)
    }

    pub(crate) fn with_artifacts(mut self, root: &Path, run_id: &str) -> Self {
        self.artifacts = Some(RunArtifacts::for_run(root, run_id));
        self.id = Some(run_id.to_owned());
        self
    }

    pub(crate) fn block_for(&mut self, prerequisite: impl Into<String>) {
        self.status = Status::Blocked;
        self.blocker = Some(prerequisite.into());
    }
}

/// A plural collection of independent Runs; it has no singleton or identity policy.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Runs(Vec<Run>);

impl Runs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, run: Run) {
        self.0.push(run);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Run> {
        self.0.iter()
    }
}

/// Scheduler-owned persisted state.  All seven fields are required on disk.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RunState {
    pub state: String,
    pub status: Status,
    pub goal_revision: String,
    pub input_revision: String,
    pub output_revision: String,
    pub active_refs: Vec<String>,
    pub blocker: String,
}
