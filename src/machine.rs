use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use crate::graph::{Phase, Transition};
use crate::roots::{RootValidationError, WorkflowRoots};

/// A parsed, status-free Machine Class declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineClass {
    roots: WorkflowRoots,
    phases: BTreeMap<String, PhaseDefinition>,
    transitions: Vec<Transition>,
    classes: BTreeMap<String, ChildClass>,
}

/// A Phase declaration: its prompt text and its Exit Guards, in the order the
/// author wrote them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseDefinition {
    phase: Phase,
    prompt: String,
    inputs: Option<Vec<String>>,
    guards: Vec<GuardKind>,
    spawns: Vec<SpawnDeclaration>,
}

/// FDC-009: one inline child Machine Class. A class body is a whole machine
/// under the same rules as the top level, plus its binding declarations, and
/// exactly one level deep - it accepts no `classes` and its Phases no
/// `spawns` (the shape FDC-012 caps).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildClass {
    phases: BTreeMap<String, PhaseDefinition>,
    transitions: Vec<Transition>,
    bindings: BTreeMap<String, BindingDeclaration>,
}

impl ChildClass {
    pub fn phases(&self) -> &BTreeMap<String, PhaseDefinition> {
        &self.phases
    }

    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }

    /// The binding names this class declares, each with its requirement flag.
    pub fn bindings(&self) -> &BTreeMap<String, BindingDeclaration> {
        &self.bindings
    }
}

/// FDC-009: one binding name a child class declares. The spawner must supply
/// every required name; values arrive at spawn time, never in the runbook.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingDeclaration {
    required: bool,
}

impl BindingDeclaration {
    pub fn required(&self) -> bool {
        self.required
    }
}

/// FDC-009: one child Run a Phase may create - a declared class, an
/// instance name unique within the Phase, and the binding names the
/// spawner supplies. The supplied names must cover the class's required
/// set exactly (RB505).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpawnDeclaration {
    class: String,
    name: String,
    bind: Vec<String>,
}

impl SpawnDeclaration {
    pub fn class(&self) -> &str {
        &self.class
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn bind(&self) -> &[String] {
        &self.bind
    }
}

/// TRP-002: the closed guard vocabulary. Each variant carries exactly the
/// fields its kind accepts, so a field foreign to a kind cannot be
/// represented, let alone reach evaluation. The list and the per-kind fields
/// are specified in the runbook specification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GuardKind {
    FilesExact {
        root: Option<String>,
        path: String,
        /// `files` is an accepted alias; both spellings are kept so a runbook
        /// renders back as it was authored, and disagreement is caught where
        /// the entries are compared.
        entries: Option<Vec<String>>,
        files: Option<Vec<String>>,
    },
    FileContains {
        root: Option<String>,
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
        root: Option<String>,
        ticket: String,
    },
    CompletionGate {
        root: Option<String>,
        ticket: String,
    },
    IntakeContract,
    RecordContract,
    /// FDC-009/FDC-011: the composition join. Satisfied only when the spawn
    /// ledger's live children carry Engine-written terminal `passed` facts.
    Join {
        require: String,
        min: Option<i64>,
    },
}

impl GuardKind {
    /// Every kind a runbook may name.
    pub const VOCABULARY: [&'static str; 8] = [
        "files_exact",
        "file_contains",
        "command_exit",
        "sensitivity_receipts",
        "completion_gate",
        "intake_contract",
        "record_contract",
        "join",
    ];

    /// The fields a kind accepts, `kind` itself excluded. `None` for a kind
    /// outside the vocabulary.
    pub fn accepted_fields(kind: &str) -> Option<&'static [&'static str]> {
        Some(match kind {
            "files_exact" => &["root", "path", "entries", "files"],
            "file_contains" => &["root", "path", "contains"],
            "command_exit" => &["program", "args", "expected", "exempt"],
            "sensitivity_receipts" | "completion_gate" => &["root", "ticket"],
            "intake_contract" | "record_contract" => &[],
            "join" => &["require", "min"],
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
            Self::Join { .. } => "join",
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
                root,
                path,
                entries,
                files,
            } => {
                let mut fields = Vec::new();
                if let Some(root) = root {
                    fields.push(("root", string(root)));
                }
                fields.push(("path", string(path)));
                if let Some(entries) = entries {
                    fields.push(("entries", array(entries)));
                }
                if let Some(files) = files {
                    fields.push(("files", array(files)));
                }
                fields
            }
            Self::FileContains {
                root,
                path,
                contains,
            } => {
                let mut fields = Vec::new();
                if let Some(root) = root {
                    fields.push(("root", string(root)));
                }
                fields.push(("path", string(path)));
                fields.push(("contains", string(contains)));
                fields
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
            Self::Join { require, min } => {
                let mut fields = vec![("require", string(require))];
                if let Some(min) = min {
                    fields.push(("min", min.to_string()));
                }
                fields
            }
            Self::SensitivityReceipts { root, .. } | Self::CompletionGate { root, .. } => root
                .as_deref()
                .map(|root| vec![("root", string(root))])
                .unwrap_or_default(),
            Self::IntakeContract | Self::RecordContract => Vec::new(),
        }
    }
}

/// A refusal to parse, carrying the stable diagnostic code that names the
/// defect class (`RB*`, tabled in the runbook specification).
///
/// The code travels with the refusal so `rtm doctor` can report the defect
/// without re-classifying prose: the parser is the runbook's only reader, so
/// it is also the only namer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineClassParseError {
    code: &'static str,
    location: String,
    message: String,
}

impl MachineClassParseError {
    fn at(code: &'static str, location: String, message: String) -> Self {
        Self {
            code,
            location,
            message,
        }
    }

    /// Where the defect lives: `top-level`, `phases`, `phase "build"`,
    /// `phase "build" guard 0`, or the transition that carries it.
    pub fn location(&self) -> &str {
        &self.location
    }

    /// The `RB*` code for this defect class.
    pub fn code(&self) -> &'static str {
        self.code
    }

    /// The refusal text, without the code.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for MachineClassParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for MachineClassParseError {}

impl MachineClass {
    /// The tracked file name a checkout's Machine Class always carries.
    pub const FILE_NAME: &'static str = "ratmac.toml";

    /// Load the reviewed Machine Class from the invoking checkout.
    ///
    /// Loading is deliberately read-only: it only reads
    /// `.ratmac/ratmac.toml` from that checkout and never creates or replaces
    /// a class file.
    pub fn load_from_project_root(
        project_root: impl AsRef<Path>,
    ) -> Result<Self, MachineClassParseError> {
        let roots = crate::root::resolve(project_root);
        let path = roots.machine_class_path();
        let source = std::fs::read_to_string(&path).map_err(|error| {
            MachineClassParseError::at(
                "RB101",
                path.display().to_string(),
                format!("failed to read {}: {error}", path.display()),
            )
        })?;
        Self::from_toml(&source)
    }
    /// Parse a TOML Machine Class while rejecting a status graph dimension.
    ///
    /// Other unknown-key diagnostics are intentionally left to the strict
    /// schema boundary owned by t-005.
    pub fn from_toml(source: &str) -> Result<Self, MachineClassParseError> {
        let document: toml::Value = source.parse().map_err(|error| {
            MachineClassParseError::at(
                "RB102",
                "top-level".to_owned(),
                format!("invalid ratmac.toml: {error}"),
            )
        })?;
        let root = document.as_table().ok_or_else(|| {
            MachineClassParseError::at(
                "RB110",
                "top-level".to_owned(),
                "invalid ratmac.toml: expected a table".to_owned(),
            )
        })?;

        if root.contains_key("status") {
            return Err(Self::status_error("top-level status dimension"));
        }
        Self::reject_unknown_keys(
            root,
            &["roots", "phases", "transitions", "classes"],
            "top-level",
        )?;
        let roots = WorkflowRoots::parse(root.get("roots")).map_err(Self::roots_parse_error)?;

        // FDC-009: inline child classes parse first so spawn validation can
        // see them; absent means a plain single machine, exactly as before.
        let classes = match root.get("classes") {
            None => BTreeMap::new(),
            Some(value) => Self::parse_classes(value)?,
        };
        let (phases, transitions) = Self::parse_machine_body(root, "", true)?;
        Self::validate_spawns(&phases, &classes)?;
        Self::validate_guard_roots(&phases, &classes, &roots)?;

        Ok(Self {
            roots,
            phases,
            transitions,
            classes,
        })
    }

    fn roots_parse_error(error: RootValidationError) -> MachineClassParseError {
        let location = if error.role() == "roots" {
            "roots".to_owned()
        } else {
            format!("roots {:?}", error.role())
        };
        MachineClassParseError::at(error.code(), location, error.message().to_owned())
    }

    fn validate_guard_roots(
        phases: &BTreeMap<String, PhaseDefinition>,
        classes: &BTreeMap<String, ChildClass>,
        roots: &WorkflowRoots,
    ) -> Result<(), MachineClassParseError> {
        for (phase_name, definition) in phases {
            Self::validate_definition_roots(definition, roots, &format!("phase {phase_name:?}"))?;
        }
        for (class_name, class) in classes {
            for (phase_name, definition) in class.phases() {
                Self::validate_definition_roots(
                    definition,
                    roots,
                    &format!("class {class_name:?} phase {phase_name:?}"),
                )?;
            }
        }
        Ok(())
    }

    fn validate_definition_roots(
        definition: &PhaseDefinition,
        roots: &WorkflowRoots,
        phase_location: &str,
    ) -> Result<(), MachineClassParseError> {
        for (index, guard) in definition.guards().iter().enumerate() {
            let location = format!("{phase_location} guard {index}");
            match guard {
                GuardKind::FilesExact {
                    root: Some(role), ..
                }
                | GuardKind::FileContains {
                    root: Some(role), ..
                }
                | GuardKind::SensitivityReceipts {
                    root: Some(role), ..
                }
                | GuardKind::CompletionGate {
                    root: Some(role), ..
                } => Self::require_declared_root(roots, role, guard.name(), &location)?,
                GuardKind::IntakeContract => {
                    for role in ["goal", "issue"] {
                        Self::require_declared_root(roots, role, guard.name(), &location)?;
                    }
                }
                GuardKind::RecordContract => {
                    for role in ["goal", "residual", "ticket"] {
                        Self::require_declared_root(roots, role, guard.name(), &location)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn require_declared_root(
        roots: &WorkflowRoots,
        role: &str,
        kind: &str,
        location: &str,
    ) -> Result<(), MachineClassParseError> {
        if roots.contains(role) {
            Ok(())
        } else {
            Err(MachineClassParseError::at(
                "RB602",
                location.to_owned(),
                format!(
                    "invalid {location}: guard kind {kind:?} names undeclared root role {role:?}"
                ),
            ))
        }
    }

    /// Declared workflow roots. This data is distinct from the Engine-root
    /// resolver in [`crate::root`].
    pub fn roots(&self) -> &WorkflowRoots {
        &self.roots
    }

    /// Validate every declared role once the workspace and Engine root are
    /// known, before a lifecycle entry point can change state.
    pub fn validate_roots(
        &self,
        workspace: &Path,
        engine_root: &Path,
    ) -> Result<(), RootValidationError> {
        self.roots.validate(workspace, engine_root)
    }

    /// Resolve one previously declared role for guard evaluation.
    pub fn resolve_root(
        &self,
        role: &str,
        workspace: &Path,
        engine_root: &Path,
    ) -> Result<PathBuf, RootValidationError> {
        self.roots.resolve(role, workspace, engine_root)
    }
    /// FDC-009: parse the `classes` table - each entry one inline child
    /// machine under the same rules, one level deep - a class body accepts no
    /// `classes` key and its Phases accept no `spawns`.
    fn parse_classes(
        value: &toml::Value,
    ) -> Result<BTreeMap<String, ChildClass>, MachineClassParseError> {
        let table = value.as_table().ok_or_else(|| {
            MachineClassParseError::at(
                "RB501",
                "classes".to_owned(),
                "invalid ratmac.toml: classes must be a table".to_owned(),
            )
        })?;
        if table.is_empty() {
            return Err(MachineClassParseError::at(
                "RB501",
                "classes".to_owned(),
                "invalid ratmac.toml: classes declares no class".to_owned(),
            ));
        }
        let mut classes = BTreeMap::new();
        for (name, body) in table {
            if name.trim().is_empty() {
                return Err(MachineClassParseError::at(
                    "RB501",
                    "classes".to_owned(),
                    "invalid ratmac.toml: class name must not be empty".to_owned(),
                ));
            }
            let body = body.as_table().ok_or_else(|| {
                MachineClassParseError::at(
                    "RB501",
                    format!("class {name:?}"),
                    format!("invalid class {name:?}: expected a table"),
                )
            })?;
            if body.contains_key("status") {
                return Err(Self::status_error(&format!(
                    "class {name:?} status dimension"
                )));
            }
            Self::reject_unknown_keys(
                body,
                &["phases", "transitions", "bindings"],
                &format!("class {name:?}"),
            )?;
            let bindings = Self::parse_bindings(body.get("bindings"), name)?;
            let scope = format!("class {name:?} ");
            let (phases, transitions) = Self::parse_machine_body(body, &scope, false)?;
            classes.insert(
                name.clone(),
                ChildClass {
                    phases,
                    transitions,
                    bindings,
                },
            );
        }
        Ok(classes)
    }

    /// FDC-009: parse one class's binding declarations.
    fn parse_bindings(
        value: Option<&toml::Value>,
        class: &str,
    ) -> Result<BTreeMap<String, BindingDeclaration>, MachineClassParseError> {
        let Some(value) = value else {
            return Ok(BTreeMap::new());
        };
        let location = format!("class {class:?} bindings");
        let table = value.as_table().ok_or_else(|| {
            MachineClassParseError::at(
                "RB502",
                location.clone(),
                format!("invalid {location}: expected a table"),
            )
        })?;
        let mut bindings = BTreeMap::new();
        for (name, body) in table {
            if name.trim().is_empty() {
                return Err(MachineClassParseError::at(
                    "RB502",
                    location.clone(),
                    format!("invalid {location}: binding name must not be empty"),
                ));
            }
            let entry_location = format!("class {class:?} binding {name:?}");
            let body = body.as_table().ok_or_else(|| {
                MachineClassParseError::at(
                    "RB502",
                    entry_location.clone(),
                    format!("invalid {entry_location}: expected a table"),
                )
            })?;
            if body.contains_key("status") {
                return Err(Self::status_error(&format!(
                    "{entry_location} status dimension"
                )));
            }
            Self::reject_unknown_keys(body, &["required"], &entry_location)?;
            let required = match body.get("required") {
                None => false,
                Some(value) => value.as_bool().ok_or_else(|| {
                    MachineClassParseError::at(
                        "RB502",
                        entry_location.clone(),
                        format!("invalid {entry_location}: required must be a boolean"),
                    )
                })?,
            };
            bindings.insert(name.clone(), BindingDeclaration { required });
        }
        Ok(bindings)
    }

    /// FDC-009: parse one Phase's spawn declarations.
    fn parse_spawns(
        value: &toml::Value,
        location: &str,
    ) -> Result<Vec<SpawnDeclaration>, MachineClassParseError> {
        let declared = value.as_array().ok_or_else(|| {
            MachineClassParseError::at(
                "RB503",
                location.to_owned(),
                format!("invalid {location} spawns: expected an array of tables"),
            )
        })?;
        let mut names = BTreeSet::new();
        let mut spawns = Vec::with_capacity(declared.len());
        for (index, entry) in declared.iter().enumerate() {
            let entry_location = format!("{location} spawn {index}");
            let entry = entry.as_table().ok_or_else(|| {
                MachineClassParseError::at(
                    "RB503",
                    entry_location.clone(),
                    format!("invalid {entry_location}: expected a table"),
                )
            })?;
            if entry.contains_key("status") {
                return Err(Self::status_error(&format!(
                    "{entry_location} status dimension"
                )));
            }
            Self::reject_unknown_keys(entry, &["class", "name", "bind"], &entry_location)?;
            let class = entry
                .get("class")
                .and_then(toml::Value::as_str)
                .filter(|class| !class.is_empty())
                .ok_or_else(|| {
                    MachineClassParseError::at(
                        "RB503",
                        entry_location.clone(),
                        format!(
                            "invalid {entry_location}: missing or empty required field \"class\""
                        ),
                    )
                })?;
            let name = entry
                .get("name")
                .and_then(toml::Value::as_str)
                .filter(|name| !name.is_empty())
                .ok_or_else(|| {
                    MachineClassParseError::at(
                        "RB503",
                        entry_location.clone(),
                        format!(
                            "invalid {entry_location}: missing or empty required field \"name\""
                        ),
                    )
                })?;
            if !names.insert(name.to_owned()) {
                return Err(MachineClassParseError::at(
                    "RB503",
                    entry_location.clone(),
                    format!("invalid {entry_location}: spawn name {name:?} is declared twice"),
                ));
            }
            let bind = match entry.get("bind") {
                None => Vec::new(),
                Some(value) => {
                    let declared = value.as_array().ok_or_else(|| {
                        MachineClassParseError::at(
                            "RB503",
                            entry_location.clone(),
                            format!(
                                "invalid {entry_location}: bind must be an array of unique non-empty strings"
                            ),
                        )
                    })?;
                    let mut seen = BTreeSet::new();
                    let mut bind = Vec::with_capacity(declared.len());
                    for value in declared {
                        let binding = value.as_str().filter(|name| !name.is_empty());
                        let Some(binding) = binding else {
                            return Err(MachineClassParseError::at(
                                "RB503",
                                entry_location.clone(),
                                format!(
                                    "invalid {entry_location}: bind must be an array of unique non-empty strings"
                                ),
                            ));
                        };
                        if !seen.insert(binding.to_owned()) {
                            return Err(MachineClassParseError::at(
                                "RB503",
                                entry_location.clone(),
                                format!("invalid {entry_location}: bind names {binding:?} twice"),
                            ));
                        }
                        bind.push(binding.to_owned());
                    }
                    bind
                }
            };
            spawns.push(SpawnDeclaration {
                class: class.to_owned(),
                name: name.to_owned(),
                bind,
            });
        }
        Ok(spawns)
    }

    /// FDC-009: static spawn validation - every spawn names a declared class
    /// (RB504) and its binding names cover the class's required set exactly
    /// while naming nothing the class does not declare (RB505).
    fn validate_spawns(
        phases: &BTreeMap<String, PhaseDefinition>,
        classes: &BTreeMap<String, ChildClass>,
    ) -> Result<(), MachineClassParseError> {
        for (phase_name, definition) in phases {
            for spawn in &definition.spawns {
                let location = format!("phase {phase_name:?} spawn {:?}", spawn.name);
                let Some(class) = classes.get(&spawn.class) else {
                    return Err(MachineClassParseError::at(
                        "RB504",
                        location.clone(),
                        format!(
                            "invalid phase {phase_name:?}: spawn {:?} names undeclared class {:?}",
                            spawn.name, spawn.class
                        ),
                    ));
                };
                for (binding_name, binding) in &class.bindings {
                    if binding.required && !spawn.bind.iter().any(|bound| bound == binding_name) {
                        return Err(MachineClassParseError::at(
                            "RB505",
                            location.clone(),
                            format!(
                                "invalid phase {phase_name:?}: spawn {:?} does not supply required binding {binding_name:?} of class {:?}",
                                spawn.name, spawn.class
                            ),
                        ));
                    }
                }
                for bound in &spawn.bind {
                    if !class.bindings.contains_key(bound) {
                        return Err(MachineClassParseError::at(
                            "RB505",
                            location.clone(),
                            format!(
                                "invalid phase {phase_name:?}: spawn {:?} supplies binding {bound:?} that class {:?} does not declare",
                                spawn.name, spawn.class
                            ),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// One whole machine body - a `phases` table and its `transitions` -
    /// parsed under the same rules at the top level and inside a class body,
    /// every location prefixed with `scope`.
    fn parse_machine_body(
        root: &toml::map::Map<String, toml::Value>,
        scope: &str,
        spawns_allowed: bool,
    ) -> Result<(BTreeMap<String, PhaseDefinition>, Vec<Transition>), MachineClassParseError> {
        let phases_value = root.get("phases").ok_or_else(|| {
            MachineClassParseError::at(
                "RB201",
                format!("{scope}phases"),
                "invalid ratmac.toml: missing phases table".to_owned(),
            )
        })?;
        let phases_table = phases_value.as_table().ok_or_else(|| {
            MachineClassParseError::at(
                "RB110",
                format!("{scope}phases"),
                "invalid ratmac.toml: phases must be a table".to_owned(),
            )
        })?;

        if phases_table.is_empty() {
            return Err(MachineClassParseError::at(
                "RB201",
                format!("{scope}phases"),
                "invalid ratmac.toml: phases declares no phase".to_owned(),
            ));
        }

        let mut phases = BTreeMap::new();
        for (name, value) in phases_table {
            if name.trim().is_empty() {
                return Err(MachineClassParseError::at(
                    "RB105",
                    format!("{scope}phases"),
                    "invalid ratmac.toml: phase name must not be empty".to_owned(),
                ));
            }
            let definition = value.as_table().ok_or_else(|| {
                MachineClassParseError::at(
                    "RB110",
                    format!("{scope}phase {name:?}"),
                    format!("invalid phase {name:?}: expected a table"),
                )
            })?;
            if definition.contains_key("status") {
                return Err(Self::status_error(&format!(
                    "{scope}phase {name:?} status dimension"
                )));
            }
            let allowed: &[&str] = if spawns_allowed {
                &["prompt", "inputs", "guards", "spawns"]
            } else {
                &["prompt", "inputs", "guards"]
            };
            Self::reject_unknown_keys(definition, allowed, &format!("{scope}phase {name:?}"))?;
            let mut guards = Vec::new();
            if let Some(declared) = definition.get("guards") {
                let declared = declared.as_array().ok_or_else(|| {
                    MachineClassParseError::at(
                        "RB110",
                        format!("{scope}phase {name:?}"),
                        format!("invalid phase {name:?} guards: expected an array"),
                    )
                })?;
                for (index, guard) in declared.iter().enumerate() {
                    let location = format!("{scope}phase {name:?} guard {index}");
                    let guard = guard.as_table().ok_or_else(|| {
                        MachineClassParseError::at(
                            "RB110",
                            location.clone(),
                            format!("invalid {location}: expected a table"),
                        )
                    })?;
                    guards.push(Self::parse_guard(guard, &location)?);
                }
            }
            // FDC-009: dormant spawn declarations; only top-level Phases
            // accept them (one level deep).
            let spawns = match definition.get("spawns") {
                None => Vec::new(),
                Some(value) => Self::parse_spawns(value, &format!("{scope}phase {name:?}"))?,
            };
            let inputs = match definition.get("inputs") {
                None => None,
                Some(value) => {
                    let declared = value.as_array().ok_or_else(|| {
                        MachineClassParseError::at(
                            "RB208",
                            format!("{scope}phase {name:?}"),
                            format!(
                                "invalid phase {name:?} inputs: expected a non-empty array of unique strings"
                            ),
                        )
                    })?;
                    if declared.is_empty() {
                        return Err(MachineClassParseError::at(
                            "RB208",
                            format!("{scope}phase {name:?}"),
                            format!("invalid phase {name:?} inputs: list must not be empty"),
                        ));
                    }
                    let mut seen = BTreeSet::new();
                    let mut inputs = Vec::with_capacity(declared.len());
                    for (index, value) in declared.iter().enumerate() {
                        let input = value.as_str().ok_or_else(|| {
                            MachineClassParseError::at(
                                "RB208",
                                format!("{scope}phase {name:?}"),
                                format!(
                                    "invalid phase {name:?} inputs[{index}]: expected a non-empty string"
                                ),
                            )
                        })?;
                        if input.is_empty() || !seen.insert(input.to_owned()) {
                            return Err(MachineClassParseError::at(
                                "RB208",
                                format!("{scope}phase {name:?}"),
                                format!(
                                    "invalid phase {name:?} inputs: values must be non-empty and unique; found {input:?}"
                                ),
                            ));
                        }
                        inputs.push(input.to_owned());
                    }
                    Some(inputs)
                }
            };

            let prompt = definition
                .get("prompt")
                .ok_or_else(|| {
                    MachineClassParseError::at(
                        "RB105",
                        format!("{scope}phase {name:?}"),
                        format!("invalid phase {name:?}: missing required prompt"),
                    )
                })?
                .as_str()
                .ok_or_else(|| {
                    MachineClassParseError::at(
                        "RB110",
                        format!("{scope}phase {name:?}"),
                        format!("invalid phase {name:?}: prompt must be a string"),
                    )
                })?
                .to_owned();
            phases.insert(
                name.clone(),
                PhaseDefinition {
                    phase: Phase::new(name.clone()),
                    prompt,
                    inputs,
                    guards,
                    spawns,
                },
            );
        }

        let transitions = match root.get("transitions") {
            None => Vec::new(),
            Some(value) => value
                .as_array()
                .ok_or_else(|| MachineClassParseError::at("RB110", format!("{scope}transitions"), "invalid ratmac.toml: transitions must be an array".to_owned()))?
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    let table = value.as_table().ok_or_else(|| MachineClassParseError::at("RB110", format!("{scope}transition {index}"), format!("invalid transition {index}: expected a table")))?;
                    if table.contains_key("status") {
                        return Err(Self::status_error(&format!("{scope}transition status dimension")));
                    }
                    Self::reject_unknown_keys(
                        table,
                        &["from", "to", "input", "freeze", "blocked-route"],
                        &format!("{scope}transition {index}"),
                    )?;
                    let from =
                        table
                            .get("from")
                            .and_then(toml::Value::as_str)
                            .ok_or_else(|| MachineClassParseError::at("RB105", format!("{scope}transition {index}"), "invalid transition: missing from phase".to_owned()))?;
                    let to = table
                        .get("to")
                        .and_then(toml::Value::as_str)
                        .ok_or_else(|| MachineClassParseError::at("RB105", format!("{scope}transition {index}"), "invalid transition: missing to phase".to_owned()))?;
                    let mut transition = Transition::new(from, to);
                    if let Some(value) = table.get("input") {
                        let input = value.as_str().ok_or_else(|| {
                            MachineClassParseError::at(
                                "RB110",
                                format!("{scope}transition {index} input"),
                                format!("invalid transition {index} input: expected a string"),
                            )
                        })?;
                        transition = transition.with_input(input);
                    }
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
                        Some(other) => Err(MachineClassParseError::at("RB109", format!("{scope}transition {index}"), format!(
                                "invalid ratmac.toml: transition {index} freeze {other:?} is unknown; the only freeze is \"goal\""
                            ))),
                    }
                })
                .collect::<Result<Vec<_>, MachineClassParseError>>()?,
        };

        for transition in &transitions {
            if transition.from().as_str().is_empty() || transition.to().as_str().is_empty() {
                return Err(MachineClassParseError::at(
                    "RB105",
                    format!(
                        "{scope}transition {:?} -> {:?}",
                        transition.from().as_str(),
                        transition.to().as_str()
                    ),
                    "invalid ratmac.toml: transition endpoints must not be empty".to_owned(),
                ));
            }
            if !phases.contains_key(transition.from().as_str()) {
                return Err(MachineClassParseError::at(
                    "RB108",
                    format!(
                        "{scope}transition {:?} -> {:?}",
                        transition.from().as_str(),
                        transition.to().as_str()
                    ),
                    format!(
                        "invalid ratmac.toml: transition source {:?} is undeclared",
                        transition.from().as_str()
                    ),
                ));
            }
            if !phases.contains_key(transition.to().as_str()) {
                return Err(MachineClassParseError::at(
                    "RB108",
                    format!(
                        "{scope}transition {:?} -> {:?}",
                        transition.from().as_str(),
                        transition.to().as_str()
                    ),
                    format!(
                        "invalid ratmac.toml: transition target {:?} is undeclared",
                        transition.to().as_str()
                    ),
                ));
            }
        }
        if let Some(transition) = transitions
            .iter()
            .find(|transition| transition.is_blocked_route() && transition.input().is_some())
        {
            return Err(MachineClassParseError::at(
                "RB213",
                format!(
                    "{scope}transition {:?} -> {:?}",
                    transition.from().as_str(),
                    transition.to().as_str()
                ),
                "invalid ratmac.toml: a blocked route must not declare input".to_owned(),
            ));
        }

        for (name, definition) in &phases {
            let ordinary = transitions
                .iter()
                .filter(|transition| {
                    !transition.is_blocked_route() && transition.from().as_str() == name
                })
                .collect::<Vec<_>>();
            match ordinary.len() {
                0 | 1 => {
                    if definition.inputs.is_some()
                        || ordinary
                            .first()
                            .is_some_and(|transition| transition.input().is_some())
                    {
                        return Err(MachineClassParseError::at(
                            "RB212",
                            format!("{scope}phase {name:?}"),
                            format!(
                                "invalid phase {name:?}: a terminal or straight-line Phase must not declare inputs or an input-labelled ordinary edge"
                            ),
                        ));
                    }
                }
                _ => {
                    let inputs = definition.inputs.as_ref().ok_or_else(|| {
                        MachineClassParseError::at(
                            "RB209",
                            format!("{scope}phase {name:?}"),
                            format!(
                                "invalid phase {name:?}: a branching Phase must declare its closed inputs list"
                            ),
                        )
                    })?;
                    let labelled = ordinary
                        .iter()
                        .filter(|transition| transition.input().is_some())
                        .copied()
                        .collect::<Vec<_>>();
                    if !labelled.is_empty() && labelled.len() != ordinary.len() {
                        return Err(MachineClassParseError::at(
                            "RB212",
                            format!("{scope}phase {name:?}"),
                            format!(
                                "invalid phase {name:?}: labelled and unlabelled ordinary edges must not be mixed"
                            ),
                        ));
                    }
                    if labelled.is_empty() {
                        return Err(MachineClassParseError::at(
                            "RB210",
                            format!("{scope}phase {name:?}"),
                            format!(
                                "invalid phase {name:?}: no ordinary edge covers the declared inputs"
                            ),
                        ));
                    }
                    let mut covered = BTreeSet::new();
                    for transition in labelled {
                        let input = transition
                            .input()
                            .expect("labelled transitions carry an input");
                        if !covered.insert(input) {
                            return Err(MachineClassParseError::at(
                                "RB211",
                                format!(
                                    "{scope}transition {:?} -> {:?}",
                                    transition.from().as_str(),
                                    transition.to().as_str()
                                ),
                                format!(
                                    "invalid phase {name:?}: transition input {input:?} is covered more than once"
                                ),
                            ));
                        }
                        if !inputs.iter().any(|declared| declared == input) {
                            return Err(MachineClassParseError::at(
                                "RB212",
                                format!("{scope}phase {name:?}"),
                                format!(
                                    "invalid phase {name:?}: transition input {input:?} is outside the closed inputs list"
                                ),
                            ));
                        }
                    }
                    if let Some(missing) = inputs
                        .iter()
                        .find(|input| !covered.contains(input.as_str()))
                    {
                        return Err(MachineClassParseError::at(
                            "RB210",
                            format!("{scope}phase {name:?}"),
                            format!(
                                "invalid phase {name:?}: no ordinary edge covers input {missing:?}"
                            ),
                        ));
                    }
                }
            }
        }

        Ok((phases, transitions))
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
            return Err(MachineClassParseError::at(
                "RB103",
                location.to_owned(),
                format!("invalid ratmac.toml: unknown key {key:?} in {location}"),
            ));
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
            return Err(MachineClassParseError::at(
                "RB105",
                location.to_owned(),
                format!("invalid {location}: missing required field \"kind\""),
            ));
        };
        let Some(kind) = kind_value.as_str() else {
            return Err(MachineClassParseError::at(
                "RB110",
                location.to_owned(),
                format!("invalid {location}: field \"kind\" must be a string"),
            ));
        };
        let Some(accepted) = GuardKind::accepted_fields(kind) else {
            return Err(MachineClassParseError::at(
                "RB106",
                location.to_owned(),
                format!(
                    "invalid {location}: unknown guard kind {kind:?}; the vocabulary is {:?}",
                    GuardKind::VOCABULARY
                ),
            ));
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
            return Err(MachineClassParseError::at("RB107", location.to_owned(), format!(
                    "invalid {location}: guard kind {kind:?} does not accept field {unknown:?}; it accepts {accepted:?}"
                )));
        }

        Ok(match kind {
            "files_exact" => GuardKind::FilesExact {
                root: field.optional_string("root")?,
                path: field.string("path")?,
                entries: field.optional_strings("entries")?,
                files: field.optional_strings("files")?,
            },
            "file_contains" => GuardKind::FileContains {
                root: field.optional_string("root")?,
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
                root: field.optional_string("root")?,
                ticket: field.string("ticket")?,
            },
            "completion_gate" => GuardKind::CompletionGate {
                root: field.optional_string("root")?,
                ticket: field.string("ticket")?,
            },
            "intake_contract" => GuardKind::IntakeContract,
            "record_contract" => GuardKind::RecordContract,
            "join" => {
                let require = field.string("require")?;
                if require != "all_passed" {
                    return Err(MachineClassParseError::at(
                        "RB506",
                        location.to_owned(),
                        format!(
                            "invalid {location}: join require {require:?} is outside the closed vocabulary; the only value is \"all_passed\""
                        ),
                    ));
                }
                let min = field.optional_integer("min")?;
                if let Some(min) = min {
                    if min < 1 {
                        return Err(MachineClassParseError::at(
                            "RB506",
                            location.to_owned(),
                            format!("invalid {location}: join min must be at least 1, got {min}"),
                        ));
                    }
                }
                GuardKind::Join { require, min }
            }
            other => {
                return Err(MachineClassParseError::at(
                    "RB106",
                    location.to_owned(),
                    format!("invalid {location}: unknown guard kind {other:?}"),
                ))
            }
        })
    }

    fn status_error(location: &str) -> MachineClassParseError {
        MachineClassParseError::at(
            "RB104", location.to_owned(),
            format!(
                "invalid ratmac.toml: {location} is forbidden; status is not a Machine Class dimension"
            ),
        )
    }

    pub fn phases(&self) -> &BTreeMap<String, PhaseDefinition> {
        &self.phases
    }

    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }

    /// FDC-009: the inline child classes this runbook declares, by name.
    /// Empty for a plain single machine.
    pub fn classes(&self) -> &BTreeMap<String, ChildClass> {
        &self.classes
    }
}

impl PhaseDefinition {
    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    /// FDC-001: the closed legal values for a branching Phase.
    pub fn inputs(&self) -> Option<&[String]> {
        self.inputs.as_deref()
    }

    /// The Phase's Exit Guards, in declaration order (TRP-004).
    pub fn guards(&self) -> &[GuardKind] {
        &self.guards
    }

    /// FDC-009: the dormant spawn declarations this Phase carries, in
    /// declaration order. Empty for every pre-composition shape.
    pub fn spawns(&self) -> &[SpawnDeclaration] {
        &self.spawns
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
        MachineClassParseError::at(
            "RB105",
            self.location.to_owned(),
            format!(
                "invalid {}: guard kind {:?} is missing required field {key:?}",
                self.location, self.kind
            ),
        )
    }

    fn wrong_type(&self, key: &str, expected: &str) -> MachineClassParseError {
        MachineClassParseError::at(
            "RB110",
            self.location.to_owned(),
            format!(
                "invalid {}: guard kind {:?} field {key:?} must be {expected}",
                self.location, self.kind
            ),
        )
    }

    fn string(&self, key: &str) -> Result<String, MachineClassParseError> {
        let value = self.guard.get(key).ok_or_else(|| self.missing(key))?;
        value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| self.wrong_type(key, "a string"))
    }

    fn optional_string(&self, key: &str) -> Result<Option<String>, MachineClassParseError> {
        match self.guard.get(key) {
            None => Ok(None),
            Some(value) => value
                .as_str()
                .map(|value| Some(value.to_owned()))
                .ok_or_else(|| self.wrong_type(key, "a string")),
        }
    }
    fn integer(&self, key: &str) -> Result<i64, MachineClassParseError> {
        let value = self.guard.get(key).ok_or_else(|| self.missing(key))?;
        value
            .as_integer()
            .ok_or_else(|| self.wrong_type(key, "an integer"))
    }

    fn optional_integer(&self, key: &str) -> Result<Option<i64>, MachineClassParseError> {
        match self.guard.get(key) {
            None => Ok(None),
            Some(value) => value
                .as_integer()
                .map(Some)
                .ok_or_else(|| self.wrong_type(key, "an integer")),
        }
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
