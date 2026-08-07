//! Declared workflow-root data.
//!
//! [`WorkflowRoots`] belongs to a parsed runbook and maps an authored role to
//! a repository-relative path. It is deliberately distinct from
//! [`crate::root::Roots`], which resolves the Engine runtime root for an
//! invocation.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// The runbook's named workflow roots, retained as safe relative paths until
/// a Scheduler has both a workspace and an Engine root to validate against.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkflowRoots {
    paths: BTreeMap<String, PathBuf>,
}

/// A named roots-table defect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootValidationError {
    code: &'static str,
    role: String,
    message: String,
}

impl RootValidationError {
    fn new(code: &'static str, role: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code,
            role: role.into(),
            message: message.into(),
        }
    }

    /// The stable diagnostic code for this defect.
    pub fn code(&self) -> &'static str {
        self.code
    }

    /// The authored role whose declaration or use is defective.
    pub fn role(&self) -> &str {
        &self.role
    }

    /// The actionable refusal prose, without any caller-added context.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for RootValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for RootValidationError {}

impl WorkflowRoots {
    /// Parse the optional top-level roots table. Shape and lexical confinement
    /// are parser concerns because no filesystem context is needed for them.
    pub fn parse(value: Option<&toml::Value>) -> Result<Self, RootValidationError> {
        let Some(value) = value else {
            return Ok(Self::default());
        };
        let table = value.as_table().ok_or_else(|| {
            RootValidationError::new(
                "RB601",
                "roots",
                "invalid roots: expected a table of role names to repository-relative paths",
            )
        })?;

        let mut paths = BTreeMap::new();
        for (role, value) in table {
            if role.trim().is_empty() {
                return Err(RootValidationError::new(
                    "RB601",
                    role,
                    "invalid roots: role name must not be empty",
                ));
            }
            let path = value
                .as_str()
                .filter(|path| !path.trim().is_empty())
                .ok_or_else(|| {
                    RootValidationError::new(
                        "RB601",
                        role,
                        format!("invalid roots role {role:?}: path must be a non-empty string"),
                    )
                })?;
            let relative = Path::new(path);
            if relative.is_absolute()
                || relative.components().any(|component| {
                    matches!(
                        component,
                        Component::ParentDir | Component::RootDir | Component::Prefix(_)
                    )
                })
            {
                return Err(RootValidationError::new(
                    "RB601",
                    role,
                    format!(
                        "invalid roots role {role:?}: path {path:?} must stay repository-relative"
                    ),
                ));
            }
            paths.insert(role.clone(), relative.to_path_buf());
        }
        Ok(Self { paths })
    }

    /// Whether a role is declared by this runbook.
    pub fn contains(&self, role: &str) -> bool {
        self.paths.contains_key(role)
    }

    /// The declared relative path for a role, if any.
    pub fn path(&self, role: &str) -> Option<&Path> {
        self.paths.get(role).map(PathBuf::as_path)
    }

    /// Validate every declaration in this workspace before any lifecycle
    /// operation can mutate Engine state.
    pub fn validate(
        &self,
        workspace: &Path,
        engine_root: &Path,
    ) -> Result<(), RootValidationError> {
        for role in self.paths.keys() {
            self.resolve(role, workspace, engine_root)?;
        }
        Ok(())
    }

    /// Resolve one named root after checking existence, repository confinement,
    /// and non-overlap with the Engine runtime root.
    pub fn resolve(
        &self,
        role: &str,
        workspace: &Path,
        engine_root: &Path,
    ) -> Result<PathBuf, RootValidationError> {
        let relative = self.paths.get(role).ok_or_else(|| {
            RootValidationError::new(
                "RB602",
                role,
                format!("root role {role:?} is not declared in roots"),
            )
        })?;
        let candidate = workspace.join(relative);
        let canonical_workspace = fs::canonicalize(workspace).map_err(|error| {
            RootValidationError::new(
                "RB603",
                role,
                format!(
                    "declared root role {role:?} cannot validate workspace {}: {error}",
                    workspace.display()
                ),
            )
        })?;
        let canonical_candidate = fs::canonicalize(&candidate).map_err(|error| {
            RootValidationError::new(
                "RB603",
                role,
                format!(
                    "declared root role {role:?} path {} does not exist or is unreadable: {error}",
                    candidate.display()
                ),
            )
        })?;
        if !canonical_candidate.starts_with(&canonical_workspace) {
            return Err(RootValidationError::new(
                "RB601",
                role,
                format!(
                    "declared root role {role:?} path {} resolves outside the repository",
                    candidate.display()
                ),
            ));
        }

        let canonical_engine =
            fs::canonicalize(engine_root).unwrap_or_else(|_| absolute(engine_root));
        if canonical_candidate == canonical_engine
            || canonical_candidate.starts_with(&canonical_engine)
            || canonical_engine.starts_with(&canonical_candidate)
        {
            return Err(RootValidationError::new(
                "RB604",
                role,
                format!(
                    "declared root role {role:?} path {} overlaps the Engine root {}",
                    candidate.display(),
                    engine_root.display()
                ),
            ));
        }
        Ok(canonical_candidate)
    }
}

fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}
