use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use crate::graph::{Phase, Transition};

/// A parsed, status-free Machine Class declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineClass {
    phases: BTreeMap<String, PhaseDefinition>,
    transitions: Vec<Transition>,
}

/// A Phase declaration's prompt text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseDefinition {
    phase: Phase,
    prompt: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineClassParseError {
    message: String,
}

impl fmt::Display for MachineClassParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for MachineClassParseError {}

impl MachineClass {
    /// Load the reviewed Machine Class at the canonical project path.
    ///
    /// Loading is deliberately read-only: this method only reads
    /// `.arca/ratmac.toml` and never creates or replaces a class file.
    pub fn load_from_project_root(
        project_root: impl AsRef<Path>,
    ) -> Result<Self, MachineClassParseError> {
        let path = project_root.as_ref().join(".arca").join("ratmac.toml");
        let source = std::fs::read_to_string(&path).map_err(|error| MachineClassParseError {
            message: format!("failed to read {}: {error}", path.display()),
        })?;
        Self::from_toml(&source)
    }
    /// Parse a TOML Machine Class while rejecting a status graph dimension.
    ///
    /// Other unknown-key diagnostics are intentionally left to the strict
    /// schema boundary owned by t-005.
    pub fn from_toml(source: &str) -> Result<Self, MachineClassParseError> {
        let document: toml::Value = source.parse().map_err(|error| MachineClassParseError {
            message: format!("invalid ratmac.toml: {error}"),
        })?;
        let root = document.as_table().ok_or_else(|| MachineClassParseError {
            message: "invalid ratmac.toml: expected a table".to_owned(),
        })?;

        if root.contains_key("status") {
            return Err(Self::status_error("top-level status dimension"));
        }
        Self::reject_unknown_keys(root, &["phases", "transitions"], "top-level")?;

        let phases_value = root.get("phases").ok_or_else(|| MachineClassParseError {
            message: "invalid ratmac.toml: missing phases table".to_owned(),
        })?;
        let phases_table = phases_value
            .as_table()
            .ok_or_else(|| MachineClassParseError {
                message: "invalid ratmac.toml: phases must be a table".to_owned(),
            })?;

        let mut phases = BTreeMap::new();
        for (name, value) in phases_table {
            if name.trim().is_empty() {
                return Err(MachineClassParseError {
                    message: "invalid ratmac.toml: phase name must not be empty".to_owned(),
                });
            }
            let definition = value.as_table().ok_or_else(|| MachineClassParseError {
                message: format!("invalid phase {name:?}: expected a table"),
            })?;
            if definition.contains_key("status") {
                return Err(Self::status_error(&format!(
                    "phase {name:?} status dimension"
                )));
            }
            Self::reject_unknown_keys(
                definition,
                &["prompt", "guards"],
                &format!("phase {name:?}"),
            )?;
            if let Some(guards) = definition.get("guards") {
                let guards = guards.as_array().ok_or_else(|| MachineClassParseError {
                    message: format!("invalid phase {name:?} guards: expected an array"),
                })?;
                for (index, guard) in guards.iter().enumerate() {
                    let guard = guard.as_table().ok_or_else(|| MachineClassParseError {
                        message: format!("invalid phase {name:?} guard {index}: expected a table"),
                    })?;
                    Self::validate_guard_keys(guard, &format!("phase {name:?} guard {index}"))?;
                }
            }
            let prompt = definition
                .get("prompt")
                .ok_or_else(|| MachineClassParseError {
                    message: format!("invalid phase {name:?}: missing required prompt"),
                })?
                .as_str()
                .ok_or_else(|| MachineClassParseError {
                    message: format!("invalid phase {name:?}: prompt must be a string"),
                })?
                .to_owned();
            phases.insert(
                name.clone(),
                PhaseDefinition {
                    phase: Phase::new(name.clone()),
                    prompt,
                },
            );
        }

        let transitions = match root.get("transitions") {
            None => Vec::new(),
            Some(value) => value
                .as_array()
                .ok_or_else(|| MachineClassParseError {
                    message: "invalid ratmac.toml: transitions must be an array".to_owned(),
                })?
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    let table = value.as_table().ok_or_else(|| MachineClassParseError {
                        message: format!("invalid transition {index}: expected a table"),
                    })?;
                    if table.contains_key("status") {
                        return Err(Self::status_error("transition status dimension"));
                    }
                    Self::reject_unknown_keys(
                        table,
                        &["from", "to"],
                        &format!("transition {index}"),
                    )?;
                    let from =
                        table
                            .get("from")
                            .and_then(toml::Value::as_str)
                            .ok_or_else(|| MachineClassParseError {
                                message: "invalid transition: missing from phase".to_owned(),
                            })?;
                    let to = table
                        .get("to")
                        .and_then(toml::Value::as_str)
                        .ok_or_else(|| MachineClassParseError {
                            message: "invalid transition: missing to phase".to_owned(),
                        })?;
                    Ok(Transition::new(from, to))
                })
                .collect::<Result<Vec<_>, MachineClassParseError>>()?,
        };

        for transition in &transitions {
            if transition.from().as_str().is_empty() || transition.to().as_str().is_empty() {
                return Err(MachineClassParseError {
                    message: "invalid ratmac.toml: transition endpoints must not be empty"
                        .to_owned(),
                });
            }
            if !phases.contains_key(transition.from().as_str()) {
                return Err(MachineClassParseError {
                    message: format!(
                        "invalid ratmac.toml: transition source {:?} is undeclared",
                        transition.from().as_str()
                    ),
                });
            }
            if !phases.contains_key(transition.to().as_str()) {
                return Err(MachineClassParseError {
                    message: format!(
                        "invalid ratmac.toml: transition target {:?} is undeclared",
                        transition.to().as_str()
                    ),
                });
            }
        }
        Ok(Self {
            phases,
            transitions,
        })
    }

    fn reject_unknown_keys(
        table: &toml::map::Map<String, toml::Value>,
        allowed: &[&str],
        location: &str,
    ) -> Result<(), MachineClassParseError> {
        if let Some(key) = table
            .keys()
            .find(|key| !allowed.iter().any(|allowed| allowed == key))
        {
            return Err(MachineClassParseError {
                message: format!("invalid ratmac.toml: unknown key {key:?} in {location}"),
            });
        }
        Ok(())
    }

    fn validate_guard_keys(
        guard: &toml::map::Map<String, toml::Value>,
        location: &str,
    ) -> Result<(), MachineClassParseError> {
        if guard.contains_key("status") {
            return Err(Self::status_error(&format!("{location} status dimension")));
        }
        Self::reject_unknown_keys(
            guard,
            &[
                "kind",
                "path",
                "command",
                "expected",
                "contains",
                "exit_code",
                "files",
                "entries",
                "program",
                "args",
            ],
            location,
        )
    }

    fn status_error(location: &str) -> MachineClassParseError {
        MachineClassParseError {
            message: format!("invalid ratmac.toml: {location} is forbidden; status is not a Machine Class dimension"),
        }
    }

    pub fn phases(&self) -> &BTreeMap<String, PhaseDefinition> {
        &self.phases
    }

    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }
}

impl PhaseDefinition {
    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }
}
