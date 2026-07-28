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

/// A Phase declaration: its prompt text and its Exit Guards, in the order the
/// author wrote them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseDefinition {
    phase: Phase,
    prompt: String,
    guards: Vec<GuardKind>,
}

/// TRP-002: the closed guard vocabulary. Each variant carries exactly the
/// fields its kind accepts, so a field foreign to a kind cannot be
/// represented, let alone reach evaluation. The list and the per-kind fields
/// are specified in `.arca/runbook-spec.md`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GuardKind {
    FilesExact {
        path: String,
        /// `files` is an accepted alias; both spellings are kept so a runbook
        /// renders back as it was authored, and disagreement is caught where
        /// the entries are compared.
        entries: Option<Vec<String>>,
        files: Option<Vec<String>>,
    },
    FileContains {
        path: String,
        contains: String,
    },
    CommandExit {
        program: String,
        args: Vec<String>,
        expected: i64,
        exempt: bool,
    },
    SensitivityReceipts {
        ticket: String,
    },
    CompletionGate {
        ticket: String,
    },
    IntakeContract,
    RecordContract,
}

impl GuardKind {
    /// Every kind a runbook may name.
    pub const VOCABULARY: [&'static str; 7] = [
        "files_exact",
        "file_contains",
        "command_exit",
        "sensitivity_receipts",
        "completion_gate",
        "intake_contract",
        "record_contract",
    ];

    /// The fields a kind accepts, `kind` itself excluded. `None` for a kind
    /// outside the vocabulary.
    pub fn accepted_fields(kind: &str) -> Option<&'static [&'static str]> {
        Some(match kind {
            "files_exact" => &["path", "entries", "files"],
            "file_contains" => &["path", "contains"],
            "command_exit" => &["program", "args", "expected", "exempt"],
            "sensitivity_receipts" | "completion_gate" => &["ticket"],
            "intake_contract" | "record_contract" => &[],
            _ => return None,
        })
    }

    /// The fields a kind cannot carry: the vocabulary's whole field set minus
    /// its own. Stating the complement is what makes a wrong-field-for-kind
    /// defect nameable.
    pub fn forbidden_fields(kind: &str) -> Option<Vec<&'static str>> {
        let accepted = Self::accepted_fields(kind)?;
        let mut every = Self::VOCABULARY
            .iter()
            .filter_map(|kind| Self::accepted_fields(kind))
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        every.sort_unstable();
        every.dedup();
        Some(
            every
                .into_iter()
                .filter(|field| !accepted.contains(field))
                .collect(),
        )
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::FilesExact { .. } => "files_exact",
            Self::FileContains { .. } => "file_contains",
            Self::CommandExit { .. } => "command_exit",
            Self::SensitivityReceipts { .. } => "sensitivity_receipts",
            Self::CompletionGate { .. } => "completion_gate",
            Self::IntakeContract => "intake_contract",
            Self::RecordContract => "record_contract",
        }
    }

    /// ETB-001: whether this guard is the declared toolchain probe that needs
    /// no gate pin.
    pub fn is_exempt(&self) -> bool {
        matches!(self, Self::CommandExit { exempt: true, .. })
    }

    /// The guard's authored fields, rendered as TOML, in the order the Phase
    /// Prompt lists them (R-028).
    pub fn rendered_fields(&self) -> Vec<(&'static str, String)> {
        let string = |value: &str| toml::Value::String(value.to_owned()).to_string();
        let array = |values: &[String]| {
            toml::Value::Array(
                values
                    .iter()
                    .map(|value| toml::Value::String(value.clone()))
                    .collect(),
            )
            .to_string()
        };
        match self {
            Self::FilesExact {
                path,
                entries,
                files,
            } => {
                let mut fields = vec![("path", string(path))];
                if let Some(entries) = entries {
                    fields.push(("entries", array(entries)));
                }
                if let Some(files) = files {
                    fields.push(("files", array(files)));
                }
                fields
            }
            Self::FileContains { path, contains } => {
                vec![("path", string(path)), ("contains", string(contains))]
            }
            Self::CommandExit {
                program,
                args,
                expected,
                ..
            } => {
                let mut fields = vec![("program", string(program))];
                if !args.is_empty() {
                    fields.push(("args", array(args)));
                }
                fields.push(("expected", expected.to_string()));
                fields
            }
            Self::SensitivityReceipts { .. }
            | Self::CompletionGate { .. }
            | Self::IntakeContract
            | Self::RecordContract => Vec::new(),
        }
    }
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

        if phases_table.is_empty() {
            return Err(MachineClassParseError {
                message: "invalid ratmac.toml: phases declares no phase".to_owned(),
            });
        }

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
            let mut guards = Vec::new();
            if let Some(declared) = definition.get("guards") {
                let declared = declared.as_array().ok_or_else(|| MachineClassParseError {
                    message: format!("invalid phase {name:?} guards: expected an array"),
                })?;
                for (index, guard) in declared.iter().enumerate() {
                    let location = format!("phase {name:?} guard {index}");
                    let guard = guard.as_table().ok_or_else(|| MachineClassParseError {
                        message: format!("invalid {location}: expected a table"),
                    })?;
                    guards.push(Self::parse_guard(guard, &location)?);
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
                    guards,
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
                        &["from", "to", "freeze", "blocked-route"],
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
                    let mut transition = Transition::new(from, to);
                    // PGE-006: an escape a human confirms, never an edge the
                    // Scheduler may take on its own.
                    if table
                        .get("blocked-route")
                        .and_then(toml::Value::as_bool)
                        .unwrap_or(false)
                    {
                        transition = transition.blocked_route();
                    }
                    match table.get("freeze").and_then(toml::Value::as_str) {
                        None => Ok(transition),
                        Some("goal") => Ok(transition.freezing_goal()),
                        Some(other) => Err(MachineClassParseError {
                            message: format!(
                                "invalid ratmac.toml: transition {index} freeze {other:?} is unknown; the only freeze is \"goal\""
                            ),
                        }),
                    }
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

    /// TRP-002, TRP-003: parse one guard into its typed kind, rejecting an
    /// unknown kind and any field the kind does not accept, where they are
    /// written.
    fn parse_guard(
        guard: &toml::map::Map<String, toml::Value>,
        location: &str,
    ) -> Result<GuardKind, MachineClassParseError> {
        if guard.contains_key("status") {
            return Err(Self::status_error(&format!("{location} status dimension")));
        }
        let Some(kind_value) = guard.get("kind") else {
            return Err(MachineClassParseError {
                message: format!("invalid {location}: missing required field \"kind\""),
            });
        };
        let Some(kind) = kind_value.as_str() else {
            return Err(MachineClassParseError {
                message: format!("invalid {location}: field \"kind\" must be a string"),
            });
        };
        let Some(accepted) = GuardKind::accepted_fields(kind) else {
            return Err(MachineClassParseError {
                message: format!(
                    "invalid {location}: unknown guard kind {kind:?}; the vocabulary is {:?}",
                    GuardKind::VOCABULARY
                ),
            });
        };
        let field = Field {
            guard,
            location,
            kind,
        };
        if let Some(unknown) = guard
            .keys()
            .find(|key| key.as_str() != "kind" && !accepted.contains(&key.as_str()))
        {
            return Err(MachineClassParseError {
                message: format!(
                    "invalid {location}: guard kind {kind:?} does not accept field {unknown:?}; it accepts {accepted:?}"
                ),
            });
        }

        Ok(match kind {
            "files_exact" => GuardKind::FilesExact {
                path: field.string("path")?,
                entries: field.optional_strings("entries")?,
                files: field.optional_strings("files")?,
            },
            "file_contains" => GuardKind::FileContains {
                path: field.string("path")?,
                contains: field.string("contains")?,
            },
            "command_exit" => GuardKind::CommandExit {
                program: field.string("program")?,
                args: field.optional_strings("args")?.unwrap_or_default(),
                expected: field.integer("expected")?,
                exempt: field.optional_bool("exempt")?.unwrap_or(false),
            },
            "sensitivity_receipts" => GuardKind::SensitivityReceipts {
                ticket: field.string("ticket")?,
            },
            "completion_gate" => GuardKind::CompletionGate {
                ticket: field.string("ticket")?,
            },
            "intake_contract" => GuardKind::IntakeContract,
            "record_contract" => GuardKind::RecordContract,
            other => {
                return Err(MachineClassParseError {
                    message: format!("invalid {location}: unknown guard kind {other:?}"),
                })
            }
        })
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

    /// The Phase's Exit Guards, in declaration order (TRP-004).
    pub fn guards(&self) -> &[GuardKind] {
        &self.guards
    }
}

/// Reads one guard field, refusing by kind, field, and location rather than
/// defaulting a wrong type into silence.
struct Field<'a> {
    guard: &'a toml::map::Map<String, toml::Value>,
    location: &'a str,
    kind: &'a str,
}

impl Field<'_> {
    fn missing(&self, key: &str) -> MachineClassParseError {
        MachineClassParseError {
            message: format!(
                "invalid {}: guard kind {:?} is missing required field {key:?}",
                self.location, self.kind
            ),
        }
    }

    fn wrong_type(&self, key: &str, expected: &str) -> MachineClassParseError {
        MachineClassParseError {
            message: format!(
                "invalid {}: guard kind {:?} field {key:?} must be {expected}",
                self.location, self.kind
            ),
        }
    }

    fn string(&self, key: &str) -> Result<String, MachineClassParseError> {
        let value = self.guard.get(key).ok_or_else(|| self.missing(key))?;
        value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| self.wrong_type(key, "a string"))
    }

    fn integer(&self, key: &str) -> Result<i64, MachineClassParseError> {
        let value = self.guard.get(key).ok_or_else(|| self.missing(key))?;
        value
            .as_integer()
            .ok_or_else(|| self.wrong_type(key, "an integer"))
    }

    fn optional_bool(&self, key: &str) -> Result<Option<bool>, MachineClassParseError> {
        match self.guard.get(key) {
            None => Ok(None),
            Some(value) => value
                .as_bool()
                .map(Some)
                .ok_or_else(|| self.wrong_type(key, "a boolean")),
        }
    }

    fn optional_strings(&self, key: &str) -> Result<Option<Vec<String>>, MachineClassParseError> {
        let Some(value) = self.guard.get(key) else {
            return Ok(None);
        };
        let array = value
            .as_array()
            .ok_or_else(|| self.wrong_type(key, "an array of strings"))?;
        array
            .iter()
            .map(|entry| {
                entry
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| self.wrong_type(key, "an array of strings"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }
}
