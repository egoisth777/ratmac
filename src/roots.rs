//! Declared workflow-root data.
//!
//! [`WorkflowRoots`] belongs to a parsed runbook and maps an authored role to
//! a repository-relative path. It is deliberately distinct from
//! [`crate::root::Roots`], which resolves the Engine runtime root for an
//! invocation.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

/// The runbook's named workflow roots, retained as safe relative paths until
/// a Scheduler has both a workspace and an Engine root to validate against.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkflowRoots {
    paths: BTreeMap<String, PathBuf>,
}

/// The canonical workflow-root mapping validated for one addressed workspace.
///
/// Unlike [`WorkflowRoots`], this value contains no authored relative paths:
/// every role has already been resolved, checked for directory type, confined
/// to the workspace, and checked against the Engine root. Lifecycle operations
/// retain this mapping rather than resolving an authored role again at use
/// time.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ValidatedWorkflowRoots {
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

impl ValidatedWorkflowRoots {
    /// Whether this validated mapping contains a role.
    pub fn contains(&self, role: &str) -> bool {
        self.paths.contains_key(role)
    }

    /// Return the canonical directory mapped to `role`.
    pub fn resolve(&self, role: &str) -> Result<PathBuf, RootValidationError> {
        self.paths.get(role).cloned().ok_or_else(|| {
            RootValidationError::new(
                "RB602",
                role,
                format!("root role {role:?} is not declared in roots"),
            )
        })
    }
    /// The canonical directory for a role, if it was validated.
    pub fn path(&self, role: &str) -> Option<&Path> {
        self.paths.get(role).map(PathBuf::as_path)
    }

    /// Iterate over the validated role-to-directory mapping.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Path)> {
        self.paths
            .iter()
            .map(|(role, path)| (role.as_str(), path.as_path()))
    }
}

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

    /// Validate every declaration and return the canonical mapping that was
    /// checked. Callers performing a lifecycle operation must retain this
    /// result and resolve roles from it instead of recalculating paths later.
    pub fn validate(
        &self,
        workspace: &Path,
        engine_root: &Path,
    ) -> Result<ValidatedWorkflowRoots, RootValidationError> {
        if self.paths.is_empty() {
            return Ok(ValidatedWorkflowRoots::default());
        }

        let canonical_workspace = canonical_workspace(workspace, "roots")?;
        let canonical_engine = canonical_engine_root(engine_root).map_err(|error| {
            RootValidationError::new(
                "RB603",
                "roots",
                format!(
                    "declared roots cannot validate Engine root {}: {error}",
                    engine_root.display()
                ),
            )
        })?;
        let mut paths = BTreeMap::new();
        for (role, relative) in &self.paths {
            let resolved = resolve_declared(
                role,
                relative,
                workspace,
                &canonical_workspace,
                engine_root,
                &canonical_engine,
            )?;
            paths.insert(role.clone(), resolved);
        }
        Ok(ValidatedWorkflowRoots { paths })
    }

    /// Resolve one named root after checking existence, repository confinement,
    /// directory type, and non-overlap with the Engine runtime root.
    ///
    /// Lifecycle code should use [`ValidatedWorkflowRoots::resolve`] after one
    /// complete [`Self::validate`] call. This compatibility helper is retained
    /// for read-only callers that need one ad-hoc role.
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
        let canonical_workspace = canonical_workspace(workspace, role)?;
        let canonical_engine = canonical_engine_root(engine_root).map_err(|error| {
            RootValidationError::new(
                "RB603",
                role,
                format!(
                    "declared root role {role:?} cannot validate Engine root {}: {error}",
                    engine_root.display()
                ),
            )
        })?;
        resolve_declared(
            role,
            relative,
            workspace,
            &canonical_workspace,
            engine_root,
            &canonical_engine,
        )
    }
}

fn canonical_workspace(workspace: &Path, role: &str) -> Result<PathBuf, RootValidationError> {
    fs::canonicalize(workspace).map_err(|error| {
        RootValidationError::new(
            "RB603",
            role,
            format!(
                "declared root role {role:?} cannot validate workspace {}: {error}",
                workspace.display()
            ),
        )
    })
}

fn resolve_declared(
    role: &str,
    relative: &Path,
    workspace: &Path,
    canonical_workspace: &Path,
    engine_root: &Path,
    canonical_engine: &Path,
) -> Result<PathBuf, RootValidationError> {
    let candidate = workspace.join(relative);
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
    if !canonical_candidate.starts_with(canonical_workspace) {
        return Err(RootValidationError::new(
            "RB601",
            role,
            format!(
                "declared root role {role:?} path {} resolves outside the repository",
                candidate.display()
            ),
        ));
    }
    let metadata = fs::metadata(&canonical_candidate).map_err(|error| {
        RootValidationError::new(
            "RB603",
            role,
            format!(
                "declared root role {role:?} path {} cannot be inspected: {error}",
                candidate.display()
            ),
        )
    })?;
    if !metadata.is_dir() {
        let found = if metadata.is_file() {
            "a file"
        } else {
            "a non-directory filesystem object"
        };
        return Err(RootValidationError::new(
            "RB603",
            role,
            format!(
                "declared root role {role:?} path {} is {found}; expected a directory",
                candidate.display()
            ),
        ));
    }
    if canonical_candidate == canonical_engine
        || canonical_candidate.starts_with(canonical_engine)
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

/// Canonicalize an Engine path that may not exist yet. Missing final
/// components are normal for a fresh checkout; all other canonicalization
/// failures are meaningful and must remain refusals.
fn canonical_engine_root(path: &Path) -> io::Result<PathBuf> {
    let mut cursor = path.to_path_buf();
    let mut missing = Vec::new();
    loop {
        match fs::canonicalize(&cursor) {
            Ok(mut canonical) => {
                for component in missing.iter().rev() {
                    canonical.push(component);
                }
                return Ok(canonical);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let Some(name) = cursor.file_name() else {
                    return Err(error);
                };
                missing.push(name.to_os_string());
                let Some(parent) = cursor.parent() else {
                    return Err(error);
                };
                cursor = parent.to_path_buf();
            }
            Err(error) => return Err(error),
        }
    }
}
