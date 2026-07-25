use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread;

use crate::graph::{MachineGraph, Phase};
use crate::machine::MachineClass;
use crate::model::{Run, RunState, Status};
use crate::state::{PhasePrompt, StateError, StateStore, StatusReport};

/// Entry facts needed before a Run may proceed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryPrerequisites {
    input_revision: PathBuf,
}

impl EntryPrerequisites {
    pub fn new(input_revision: impl Into<PathBuf>) -> Self {
        Self {
            input_revision: input_revision.into(),
        }
    }

    fn is_complete(&self) -> bool {
        self.input_revision.is_file()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepRequest {
    pub claim: String,
}

impl StepRequest {
    pub fn new(claim: impl Into<String>) -> Self {
        Self {
            claim: claim.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuardFailure {
    pub kind: String,
    pub path: String,
    pub observed: String,
    pub expected: String,
    name: String,
}

impl GuardFailure {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn observed(&self) -> &str {
        &self.observed
    }

    pub fn expected(&self) -> &str {
        &self.expected
    }
}

impl fmt::Display for GuardFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: observed {}; expected {}",
            self.name, self.observed, self.expected
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepOutcome {
    Advanced { from: Phase, to: Phase },
    Refused { failures: Vec<GuardFailure> },
}

impl fmt::Display for StepOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Advanced { from, to } => write!(formatter, "advanced: {from} -> {to}"),
            Self::Refused { failures } => {
                write!(formatter, "step refused")?;
                for failure in failures {
                    write!(formatter, "; {failure}")?;
                }
                Ok(())
            }
        }
    }
}

/// Scheduler-owned machine, lifecycle, and optional project state access.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scheduler {
    machine: MachineGraph,
    root: Option<PathBuf>,
    store: Option<StateStore>,
}

struct InvocationLock {
    path: PathBuf,
}

impl InvocationLock {
    fn legacy_lock_path(path: &Path) -> Option<PathBuf> {
        path.parent().map(|parent| parent.join("schd.lock"))
    }

    fn refuse_legacy(path: &Path) -> Result<(), StateError> {
        let Some(legacy_path) = Self::legacy_lock_path(path) else {
            return Ok(());
        };
        match fs::symlink_metadata(&legacy_path) {
            Ok(_) => Err(StateError::new(format!(
                "refusing to run: legacy lock {} exists; explicitly migrate or remove that lock, then retry; it was not modified",
                legacy_path.display()
            ))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(StateError::new(format!(
                "inspect legacy lock {}: {error}",
                legacy_path.display()
            ))),
        }
    }

    fn try_acquire(path: &Path) -> std::io::Result<Self> {
        OpenOptions::new().write(true).create_new(true).open(path)?;
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    fn acquire_with_retry(path: &Path) -> Result<Self, StateError> {
        const MAX_ATTEMPTS: usize = 4_096;
        Self::refuse_legacy(path)?;
        for attempt in 0..MAX_ATTEMPTS {
            match Self::try_acquire(path) {
                Ok(lock) => {
                    if let Err(error) = Self::refuse_legacy(path) {
                        drop(lock);
                        return Err(error);
                    }
                    return Ok(lock);
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::PermissionDenied
                    ) =>
                {
                    Self::refuse_legacy(path)?;
                    if attempt + 1 < MAX_ATTEMPTS {
                        thread::yield_now();
                    } else {
                        return Err(StateError::new(format!(
                            "acquire rtm.lock: lock remained held after {MAX_ATTEMPTS} attempts"
                        )));
                    }
                }
                Err(error) => {
                    return Err(StateError::new(format!("create rtm.lock: {error}")));
                }
            }
        }
        unreachable!("lock retry loop returns on every attempt");
    }
}
impl Drop for InvocationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

impl Scheduler {
    /// Construct the in-memory scheduler used by the entry-prerequisite model.
    pub fn new(machine: MachineGraph) -> Self {
        Self {
            machine,
            root: None,
            store: None,
        }
    }

    /// Open a project without creating or modifying any scheduler-owned file.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, StateError> {
        let root = root.as_ref().to_path_buf();
        let machine = Self::load_machine(&root)?;
        Ok(Self {
            machine,
            store: Some(StateStore::new(&root)),
            root: Some(root),
        })
    }

    fn read_machine_source(root: &Path) -> Result<String, StateError> {
        fs::read_to_string(root.join(".arca/ratmac.toml"))
            .map_err(|error| StateError::new(format!("read ratmac.toml: {error}")))
    }

    fn machine_from_source(source: &str) -> Result<MachineGraph, StateError> {
        let class = MachineClass::from_toml(source)
            .map_err(|error| StateError::new(format!("parse ratmac.toml: {error}")))?;
        let phases = class.phases().keys().map(Phase::new).collect::<Vec<_>>();
        Ok(MachineGraph::new(phases, class.transitions().to_vec()))
    }

    fn load_machine(root: &Path) -> Result<MachineGraph, StateError> {
        let class_path = root.join(".arca/ratmac.toml");
        match fs::read_to_string(class_path) {
            Ok(source) => Self::machine_from_source(&source),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(MachineGraph::default())
            }
            Err(error) => Err(StateError::new(format!("read ratmac.toml: {error}"))),
        }
    }

    pub fn machine(&self) -> &MachineGraph {
        &self.machine
    }

    /// Instantiate a Run from the canonical, human-authored Machine Class.
    ///
    /// State and log persist after return. The lock is held only for this
    /// invocation and is released by the RAII guard before the Run is observed.
    pub fn start(&mut self) -> Result<Run, StateError> {
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| StateError::new("start requires Scheduler::open"))?
            .clone();
        let source = Self::read_machine_source(&root)?;
        self.machine = Self::machine_from_source(&source)?;
        let phase = self.initial_phase()?;
        let arca = root.join(".arca");
        fs::create_dir_all(&arca)
            .map_err(|error| StateError::new(format!("create .arca: {error}")))?;
        let lock_path = arca.join("rtm.lock");
        let _lock = InvocationLock::acquire_with_retry(&lock_path)?;
        // A canonical State File is the durable marker for an active v1 Run.
        // The invocation lock above is transient and is not used for admission.
        if arca.join("state.toml").exists() {
            self.store()?.load()?;
            return Err(StateError::new(
                "cannot start: an active Run already exists for this project",
            ));
        }

        let state = RunState {
            phase: phase.to_string(),
            status: Status::Planned,
            goal_revision: String::new(),
            input_revision: String::new(),
            output_revision: String::new(),
            active_refs: Vec::new(),
            blocker: String::new(),
        };
        let log_path = arca.join("log.md");
        let log_existed = log_path.exists();
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&log_path)
            .map_err(|error| StateError::new(format!("open log.md: {error}")))?;
        let old_log_len = log
            .metadata()
            .map_err(|error| StateError::new(format!("stat log.md: {error}")))?
            .len();
        if let Err(state_error) = self.store()?.write(&state) {
            drop(log);
            if !log_existed {
                let _ = fs::remove_file(&log_path);
            } else {
                let _ = OpenOptions::new()
                    .write(true)
                    .open(&log_path)
                    .and_then(|file| file.set_len(old_log_len));
            }
            return Err(state_error);
        }
        drop(log);

        // ETB-001: Run evidence carries the Stable Engine pin from Run start,
        // so every later gate pin is recorded beside a known Engine identity.
        let mut evidence = crate::pin::Evidence::load(&root);
        if let Some(identity) = crate::pin::engine_identity() {
            evidence.set_engine(identity);
        }
        // ETB-003: Run start records the baseline goal revision. The freeze
        // happens later, at the intake-completion boundary.
        evidence.goal_baseline = crate::goal::revision(&root);
        evidence.goal_frozen = None;
        evidence
            .write(&root)
            .map_err(|error| StateError::new(format!("write evidence.toml: {error}")))?;

        Ok(Run::new(phase, Status::Planned).with_artifact_root(&root))
    }

    /// Evaluate the supported `files_exact` guards and apply a transition only
    /// after every guard passes.  The claim is metadata and never evidence.
    pub fn step(&mut self, request: StepRequest) -> Result<StepOutcome, StateError> {
        let _claim = request.claim;
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| StateError::new("step requires Scheduler::open"))?
            .clone();
        let lock_path = root.join(".arca/rtm.lock");
        let _lock = InvocationLock::acquire_with_retry(&lock_path)?;
        let source = Self::read_machine_source(&root)?;
        self.machine = Self::machine_from_source(&source)?;
        let state = self.load_state_unlocked()?;
        let state_phase = state.phase.clone();
        if !self
            .machine
            .phases()
            .any(|phase| phase.as_str() == state_phase)
        {
            return Err(StateError::new(format!(
                "State File phase {state_phase:?} is undeclared in ratmac.toml"
            )));
        }
        let mut failures = self.files_exact_failures(&state.phase, &source)?;
        // ETB-003: between the freeze and batch closure, the goal is fixed.
        // The drift check is appended rather than short-circuited so a guard
        // refusal and a drift refusal are reported in the same reply.
        let evidence = crate::pin::Evidence::load(&root);
        if let Some(frozen) = evidence.goal_frozen.as_deref() {
            let observed = crate::goal::revision(&root).unwrap_or_else(|| "absent".to_owned());
            if observed != frozen {
                failures.push(guard_failure(
                    "goal drift",
                    crate::goal::GOAL_DIR,
                    observed,
                    frozen,
                ));
            }
        }
        if !failures.is_empty() {
            return Ok(StepOutcome::Refused { failures });
        }

        let from = Phase::new(state.phase.clone());
        let freezes_goal = self
            .machine
            .transition_for(from.as_str())
            .is_some_and(crate::graph::Transition::freezes_goal);
        let Some(to) = self.machine.next_phase(&from).cloned() else {
            return Ok(StepOutcome::Refused {
                failures: vec![guard_failure(
                    "transition",
                    state.phase,
                    "no outgoing transition",
                    "declared next Phase",
                )],
            });
        };
        let prior = state.clone();
        let log_path = root.join(".arca/log.md");
        let mut log = OpenOptions::new()
            .append(true)
            .read(true)
            .open(&log_path)
            .map_err(|error| StateError::new(format!("open log.md: {error}")))?;
        let old_log_len = log
            .metadata()
            .map_err(|error| StateError::new(format!("stat log.md: {error}")))?
            .len();
        let needs_separator = if old_log_len == 0 {
            false
        } else {
            log.seek(SeekFrom::End(-1))
                .map_err(|error| StateError::new(format!("seek log.md: {error}")))?;
            let mut last = [0_u8; 1];
            log.read_exact(&mut last)
                .map_err(|error| StateError::new(format!("read log.md: {error}")))?;
            last[0] != b'\n'
        };

        let mut next = state;
        next.phase = to.to_string();
        if freezes_goal {
            // The frozen revision is the post-integration content: it is
            // computed here, at the boundary that closes intake. Evidence is
            // written before the State File, so an interrupted freeze leaves
            // the Run unchanged rather than half-frozen.
            let frozen = crate::goal::revision(&root)
                .ok_or_else(|| StateError::new("cannot freeze goal: .arca/current/ is absent"))?;
            let mut frozen_evidence = crate::pin::Evidence::load(&root);
            frozen_evidence.goal_frozen = Some(frozen.clone());
            if let Err(error) = frozen_evidence.write(&root) {
                drop(log);
                return Err(StateError::new(format!(
                    "freeze goal revision: write evidence.toml: {error}"
                )));
            }
            next.goal_revision = frozen;
        }
        if let Err(state_error) = self.store()?.write(&next) {
            drop(log);
            return Err(state_error);
        }

        let append_result = (|| {
            if needs_separator {
                log.write_all(b"\n")?;
            }
            writeln!(log, "- Transition: {from} -> {to}")?;
            log.flush()?;
            log.sync_all()
        })();
        if let Err(append_error) = append_result {
            let mut rollback_errors = Vec::new();
            if let Err(error) = log.set_len(old_log_len) {
                rollback_errors.push(format!("truncate log.md: {error}"));
            }
            if let Err(error) = log.sync_all() {
                rollback_errors.push(format!("sync log.md rollback: {error}"));
            }
            drop(log);
            if let Err(error) = self.store()?.write(&prior) {
                rollback_errors.push(format!("restore state.toml: {error}"));
            }
            let message = if rollback_errors.is_empty() {
                format!("append log.md failed: {append_error}")
            } else {
                format!(
                    "append log.md failed: {append_error}; rollback failed: {}",
                    rollback_errors.join("; ")
                )
            };
            return Err(StateError::new(message));
        }
        Ok(StepOutcome::Advanced { from, to })
    }

    fn files_exact_failures(
        &self,
        phase: &str,
        source: &str,
    ) -> Result<Vec<GuardFailure>, StateError> {
        let root = match self.root.as_ref() {
            Some(root) => root,
            None => {
                return Ok(vec![guard_failure(
                    "scheduler",
                    "",
                    "step requires Scheduler::open",
                    "opened project",
                )])
            }
        };
        let document: toml::Value = match source.parse() {
            Ok(document) => document,
            Err(error) => {
                return Ok(vec![guard_failure(
                    "ratmac",
                    ".arca/ratmac.toml",
                    error.to_string(),
                    "valid TOML",
                )])
            }
        };
        let Some(phases) = document.get("phases").and_then(toml::Value::as_table) else {
            return Ok(vec![guard_failure(
                "ratmac",
                "phases",
                "missing phases table",
                "phase guard declarations",
            )]);
        };
        let Some(definition) = phases.get(phase).and_then(toml::Value::as_table) else {
            return Ok(vec![guard_failure(
                "ratmac",
                phase,
                "missing phase definition",
                "current Phase definition",
            )]);
        };
        let Some(guards) = definition.get("guards") else {
            return Ok(Vec::new());
        };
        let Some(guards) = guards.as_array() else {
            return Ok(vec![guard_failure(
                "malformed",
                phase,
                "guards is not an array",
                "array of guard tables",
            )]);
        };

        let mut failures = Vec::new();
        for guard in guards {
            let Some(table) = guard.as_table() else {
                failures.push(guard_failure(
                    "malformed",
                    phase,
                    "guard is not a table",
                    "guard table with kind",
                ));
                continue;
            };
            let Some(kind) = table.get("kind").and_then(toml::Value::as_str) else {
                failures.push(guard_failure(
                    "malformed",
                    phase,
                    "missing guard kind",
                    "files_exact, file_contains, command_exit, sensitivity_receipts, completion_gate, intake_contract, or record_contract",
                ));
                continue;
            };
            let result = match kind {
                "files_exact" => self.evaluate_files_exact(root, table),
                "file_contains" => self.evaluate_file_contains(root, table),
                "command_exit" => self.evaluate_command_exit(root, table),
                "sensitivity_receipts" => self.evaluate_sensitivity_receipts(root, table),
                "completion_gate" => self.evaluate_completion_gate(root, table),
                "intake_contract" => self.evaluate_contract(
                    "intake_contract",
                    crate::contract::gate_intake(root),
                    "every issue integrated or rejected, five-file shape intact, requirement IDs in the goal, links resolving",
                ),
                "record_contract" => self.evaluate_contract(
                    "record_contract",
                    crate::contract::gate_records(root),
                    "one residual per requirement citing the frozen revision, evidence behind every satisfied, one owning ticket per gap, acyclic dependencies, complete tickets",
                ),
                unsupported => Err(guard_failure(
                    unsupported,
                    table
                        .get("path")
                        .and_then(toml::Value::as_str)
                        .unwrap_or(phase),
                    "unsupported guard kind",
                    "files_exact, file_contains, command_exit, sensitivity_receipts, completion_gate, intake_contract, or record_contract",
                )),
            };
            if let Err(failure) = result {
                failures.push(failure);
            }
        }
        Ok(failures)
    }

    /// PGE-003: the P4 gate. Every planned test the ticket declares must
    /// resolve to a sensitivity receipt under `.arca/evidence/`; prose,
    /// filenames, and status fields satisfy nothing. The predicate runs
    /// in-process, inside the pinned gate boundary, so no external program is
    /// trusted to decide whether the work was done.
    fn evaluate_sensitivity_receipts(
        &self,
        root: &Path,
        table: &toml::map::Map<String, toml::Value>,
    ) -> Result<(), GuardFailure> {
        let ticket = table
            .get("ticket")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| {
                guard_failure(
                    "sensitivity_receipts",
                    "ticket",
                    "missing ticket path",
                    "ticket = \"<path to the ticket file>\"",
                )
            })?;
        crate::receipt::gate_sensitivity(root, ticket).map_err(|defects| {
            let observed = defects
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            guard_failure(
                "sensitivity_receipts",
                ticket,
                observed,
                "one sensitivity receipt per planned test",
            )
        })
    }

    /// PGE-005: the P5 gate. Every check the executing ticket declares must
    /// carry a green, fresh, self-consistent completion receipt. The gate
    /// verifies receipts rather than running the checks, because ETB-001
    /// forbids rebuilding project source at evaluation time.
    fn evaluate_completion_gate(
        &self,
        root: &Path,
        table: &toml::map::Map<String, toml::Value>,
    ) -> Result<(), GuardFailure> {
        let ticket = table
            .get("ticket")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| {
                guard_failure(
                    "completion_gate",
                    "ticket",
                    "missing ticket path",
                    "ticket = \"<path to the ticket file>\"",
                )
            })?;
        crate::completion::gate_completion(root, ticket).map_err(|defects| {
            let observed = defects
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            guard_failure(
                "completion_gate",
                ticket,
                observed,
                "one green, fresh completion receipt per declared check",
            )
        })
    }

    /// PGE-001, PGE-002: render a contract-gate result as a refusal that names
    /// every offending record.
    fn evaluate_contract(
        &self,
        kind: &str,
        result: Result<(), Vec<crate::contract::ContractDefect>>,
        expected: &str,
    ) -> Result<(), GuardFailure> {
        result.map_err(|defects| {
            let observed = defects
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            guard_failure(kind, ".arca", observed, expected)
        })
    }

    fn evaluate_files_exact(
        &self,
        root: &Path,
        table: &toml::map::Map<String, toml::Value>,
    ) -> Result<(), GuardFailure> {
        let path = required_string(table, "path", "files_exact")?;
        reject_guard_keys(table, &["kind", "path", "entries", "files"], path)?;
        let entries_value = table.get("entries");
        let files_value = table.get("files");
        let entries = match (entries_value, files_value) {
            (Some(entries), Some(files)) if entries != files => {
                return Err(guard_failure(
                    "files_exact",
                    path,
                    "entries and files disagree",
                    "matching entries/files aliases",
                ))
            }
            (Some(entries), _) | (_, Some(entries)) => Some(entries),
            (None, None) => None,
        };
        let target = guarded_target(root, path, "files_exact")?;
        let Some(entries) = entries else {
            return if target.exists() {
                Ok(())
            } else {
                Err(guard_failure("files_exact", path, "missing", "path exists"))
            };
        };
        let Some(entries) = entries.as_array() else {
            return Err(guard_failure(
                "files_exact",
                path,
                "entries/files is not an array",
                "array of string entry names",
            ));
        };
        if entries.iter().any(|entry| !entry.is_str()) {
            return Err(guard_failure(
                "files_exact",
                path,
                "entries/files contains a non-string",
                "array of string entry names",
            ));
        }
        let expected = entries
            .iter()
            .filter_map(toml::Value::as_str)
            .map(str::to_owned)
            .collect::<std::collections::BTreeSet<_>>();
        if !target.is_dir() {
            return Err(guard_failure(
                "files_exact",
                path,
                if target.exists() {
                    "target is not a directory"
                } else {
                    "directory is missing"
                },
                "existing directory",
            ));
        }
        let actual = fs::read_dir(&target)
            .map_err(|error| {
                guard_failure("files_exact", path, error.to_string(), "readable directory")
            })?
            .map(|entry| {
                entry
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .map_err(|error| {
                        guard_failure("files_exact", path, error.to_string(), "readable entry")
                    })
            })
            .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
        if actual == expected {
            Ok(())
        } else {
            Err(guard_failure(
                "files_exact",
                path,
                format!("{actual:?}"),
                format!("{expected:?}"),
            ))
        }
    }

    fn evaluate_file_contains(
        &self,
        root: &Path,
        table: &toml::map::Map<String, toml::Value>,
    ) -> Result<(), GuardFailure> {
        let path = required_string(table, "path", "file_contains")?;
        reject_guard_keys(table, &["kind", "path", "contains"], path)?;
        let contains = table
            .get("contains")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| {
                guard_failure(
                    "file_contains",
                    path,
                    "missing or non-string contains",
                    "string content",
                )
            })?;
        let target = guarded_target(root, path, "file_contains")?;
        let source = fs::read_to_string(&target).map_err(|error| {
            guard_failure("file_contains", path, error.to_string(), "readable file")
        })?;
        if source.contains(contains) {
            Ok(())
        } else {
            Err(guard_failure(
                "file_contains",
                path,
                format!("content {source:?}"),
                format!("content containing {contains:?}"),
            ))
        }
    }

    fn evaluate_command_exit(
        &self,
        root: &Path,
        table: &toml::map::Map<String, toml::Value>,
    ) -> Result<(), GuardFailure> {
        let program = required_string(table, "program", "command_exit")?;
        reject_guard_keys(
            table,
            &["kind", "program", "args", "expected", "exempt"],
            program,
        )?;
        let args = match table.get("args") {
            Some(value) => {
                let Some(args) = value.as_array() else {
                    return Err(guard_failure(
                        "command_exit",
                        program,
                        "args is not an array",
                        "array of strings",
                    ));
                };
                let Some(args) = args
                    .iter()
                    .map(toml::Value::as_str)
                    .collect::<Option<Vec<_>>>()
                else {
                    return Err(guard_failure(
                        "command_exit",
                        program,
                        "args contains a non-string",
                        "array of strings",
                    ));
                };
                args
            }
            None => Vec::new(),
        };
        let Some(expected) = table.get("expected").and_then(toml::Value::as_integer) else {
            return Err(guard_failure(
                "command_exit",
                program,
                "missing or non-integer expected",
                "integer exit code",
            ));
        };
        // ETB-001: a probe that reads no project state must say so; every
        // other command guard runs a pinned gate artifact.
        let exempt = table
            .get("exempt")
            .and_then(toml::Value::as_bool)
            .unwrap_or(false);
        if let Some(reason) = crate::pin::build_invocation_reason(program, &args) {
            return Err(guard_failure(
                "command_exit",
                program,
                format!("{reason}; diagnostic: gate not executed: build-at-evaluation guard"),
                "pinned or exempt gate command that compiles nothing",
            ));
        }
        if !exempt {
            self.verify_gate_pin(root, program)?;
        }

        // ETB-002: capture the child's stderr so a refusal can name the
        // artifact to repair, bounded so a runaway guard cannot flood output.
        let output = std::process::Command::new(program)
            .args(args)
            .current_dir(root)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
            .map_err(|error| {
                guard_failure(
                    "command_exit",
                    program,
                    error.to_string(),
                    format!("exit {expected}"),
                )
            })?;
        let observed = output.status.code().map(i64::from).unwrap_or(-1);
        if observed == expected {
            Ok(())
        } else {
            Err(guard_failure(
                "command_exit",
                program,
                format!("exit {observed}; {}", bounded_diagnostic(&output.stderr)),
                format!("exit {expected}"),
            ))
        }
    }

    /// ETB-001: resolve, hash, and verify the gate artifact for `program`,
    /// recording the pin in Run evidence on first use.
    fn verify_gate_pin(&self, root: &Path, program: &str) -> Result<(), GuardFailure> {
        let resolved = crate::pin::resolve_program(root, program).map_err(|reason| {
            guard_failure(
                "command_exit",
                program,
                format!("{reason}; diagnostic: gate artifact not executed: unpinnable path"),
                "a regular executable file with a stable identity",
            )
        })?;
        let sha256 = crate::pin::sha256_file(&resolved).map_err(|error| {
            guard_failure(
                "command_exit",
                program,
                format!(
                    "gate artifact is unreadable: {error}; \
                     diagnostic: gate artifact not executed: unreadable"
                ),
                "a readable executable file",
            )
        })?;
        let observed = crate::pin::Identity {
            resolved: resolved.to_string_lossy().replace('\\', "/"),
            sha256,
        };

        let mut evidence = crate::pin::Evidence::load(root);
        match evidence.gate(program) {
            Some(pinned) if *pinned == observed => Ok(()),
            Some(pinned) => Err(guard_failure(
                "command_exit",
                program,
                format!("{observed}; diagnostic: gate artifact not executed: pin mismatch"),
                pinned.to_string(),
            )),
            None => {
                evidence.record_gate(program, observed);
                evidence.write(root).map_err(|error| {
                    guard_failure(
                        "command_exit",
                        program,
                        format!(
                            "cannot record gate pin: {error}; \
                             diagnostic: gate artifact not executed: unrecordable pin"
                        ),
                        "writable Run evidence",
                    )
                })
            }
        }
    }

    fn initial_phase(&self) -> Result<Phase, StateError> {
        let candidates =
            self.machine
                .phases()
                .filter(|phase| {
                    // A blocked route points backwards by design; it never makes
                    // its destination a non-initial Phase.
                    !self.machine.transitions().any(|transition| {
                        transition.to() == *phase && !transition.is_blocked_route()
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
        match candidates.as_slice() {
            [phase] => Ok(phase.clone()),
            [] => Err(StateError::new(
                "cannot start: Machine Class has no unique initial Phase",
            )),
            _ => Err(StateError::new(
                "cannot start: Machine Class has multiple initial Phases",
            )),
        }
    }

    /// Apply only entry-prerequisite semantics. Exit Guards and persistence are
    /// deliberately outside this boundary.
    pub fn evaluate_entry_prerequisites(
        &mut self,
        mut run: Run,
        prerequisites: EntryPrerequisites,
    ) -> Run {
        if !prerequisites.is_complete() {
            run.block_for("input_revision");
        }
        run
    }

    fn invocation_lock_with_retry(&self) -> Result<InvocationLock, StateError> {
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| StateError::new("operation requires Scheduler::open"))?;
        let arca = root.join(".arca");
        InvocationLock::acquire_with_retry(&arca.join("rtm.lock"))
    }

    fn store(&self) -> Result<&StateStore, StateError> {
        self.store
            .as_ref()
            .ok_or_else(|| StateError::new("state operations require Scheduler::open"))
    }

    /// Scheduler-owned initialization of a complete State File.
    pub fn initialize_state(&mut self, state: RunState) -> Result<(), StateError> {
        let _lock = self.invocation_lock_with_retry()?;
        self.store()?.write(&state)
    }

    /// Record the only state transition that may enter `blocked`.
    pub fn record_missing_prerequisite(
        &mut self,
        mut state: RunState,
        prerequisite: impl AsRef<str>,
    ) -> Result<(), StateError> {
        let _lock = self.invocation_lock_with_retry()?;
        let prerequisite = prerequisite.as_ref();
        state.status = Status::Blocked;
        state.blocker = format!("missing entry prerequisite: {prerequisite}");
        self.store()?.write(&state)
    }

    pub fn load_state(&self) -> Result<RunState, StateError> {
        let _lock = self.invocation_lock_with_retry()?;
        self.load_state_unlocked()
    }

    fn load_state_unlocked(&self) -> Result<RunState, StateError> {
        self.store()?.load()
    }

    /// Read-only status; it loads state and reports labels from the current class.
    pub fn status(&self) -> Result<StatusReport, StateError> {
        let _lock = self.invocation_lock_with_retry()?;
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| StateError::new("status requires Scheduler::open"))?;
        let source = Self::read_machine_source(root)?;
        let machine = Self::machine_from_source(&source)?;
        let state = self.load_state_unlocked()?;
        if !machine.phases().any(|phase| phase.as_str() == state.phase) {
            return Err(StateError::new(format!(
                "State File phase {:?} is undeclared in ratmac.toml",
                state.phase
            )));
        }
        let pending_guards = self.pending_guard_labels(&state.phase, &source)?;
        let phase_prompt = self.render_phase_prompt(&state.phase, &source)?;
        Ok(StatusReport {
            state,
            pending_guards,
            phase_prompt,
        })
    }

    fn render_phase_prompt(&self, phase: &str, source: &str) -> Result<PhasePrompt, StateError> {
        let document: toml::Value = source
            .parse()
            .map_err(|error| StateError::new(format!("invalid ratmac.toml: {error}")))?;
        let definition = document
            .get("phases")
            .and_then(toml::Value::as_table)
            .and_then(|phases| phases.get(phase))
            .and_then(toml::Value::as_table)
            .ok_or_else(|| StateError::new(format!("missing phase definition: {phase}")))?;
        let prose = definition
            .get("prompt")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| StateError::new(format!("missing string prompt for phase: {phase}")))?;
        let guards = definition.get("guards").and_then(toml::Value::as_array);
        let mut rendered = prose.to_owned();
        if let Some(guards) = guards.filter(|guards| !guards.is_empty()) {
            rendered.push_str("\n\nExit Guards:\n");
            for guard in guards {
                let table = guard.as_table().ok_or_else(|| {
                    StateError::new(format!("invalid guard for phase {phase}: expected a table"))
                })?;
                let kind = table
                    .get("kind")
                    .and_then(toml::Value::as_str)
                    .unwrap_or("unknown");
                rendered.push_str("- ");
                rendered.push_str(kind);
                for key in [
                    "path", "entries", "files", "contains", "program", "args", "expected",
                ] {
                    if let Some(value) = table.get(key) {
                        rendered.push(' ');
                        rendered.push_str(key);
                        rendered.push('=');
                        rendered.push_str(&value.to_string());
                    }
                }
                rendered.push('\n');
            }
            rendered.pop();
        }
        Ok(PhasePrompt::new(rendered))
    }

    fn pending_guard_labels(&self, phase: &str, source: &str) -> Result<Vec<String>, StateError> {
        let document: toml::Value = source
            .parse()
            .map_err(|error| StateError::new(format!("invalid ratmac.toml: {error}")))?;
        let guards = document
            .get("phases")
            .and_then(toml::Value::as_table)
            .and_then(|phases| phases.get(phase))
            .and_then(toml::Value::as_table)
            .and_then(|definition| definition.get("guards"))
            .and_then(toml::Value::as_array);

        Ok(guards
            .into_iter()
            .flatten()
            .filter_map(|guard| guard.get("kind").and_then(toml::Value::as_str))
            .map(str::to_owned)
            .collect())
    }
}

fn guarded_target(root: &Path, path: &str, kind: &str) -> Result<PathBuf, GuardFailure> {
    let relative = Path::new(path);
    if relative.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        return Err(guard_failure(
            kind,
            path,
            "path escapes project root",
            "relative path within project root",
        ));
    }
    let target = root.join(relative);
    if target.exists() {
        let canonical_root = fs::canonicalize(root).map_err(|error| {
            guard_failure(kind, path, error.to_string(), "canonical project root")
        })?;
        let canonical_target = fs::canonicalize(&target).map_err(|error| {
            guard_failure(kind, path, error.to_string(), "canonical guarded path")
        })?;
        if !canonical_target.starts_with(canonical_root) {
            return Err(guard_failure(
                kind,
                path,
                "path escapes project root through symlink",
                "path contained within project root",
            ));
        }
    }
    Ok(target)
}

/// ETB-002: the deterministic diagnostic bound. A refusal carries at most the
/// last [`DIAGNOSTIC_BOUND`] bytes of the guard's stderr.
pub const DIAGNOSTIC_BOUND: usize = 4096;

/// Fixed wording when the guard says nothing: never an omitted field.
pub const NO_DIAGNOSTIC: &str = "no diagnostic emitted";

/// Explicit overflow marker prefixed to a truncated diagnostic.
pub const TRUNCATION_MARKER: &str = "\u{2026}truncated";

/// Render a guard's stderr as a bounded, lossy, single-line diagnostic.
///
/// The retained window is the *last* [`DIAGNOSTIC_BOUND`] bytes, because a
/// failing command states why it failed at the end of its output.
fn bounded_diagnostic(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return format!("diagnostic: {NO_DIAGNOSTIC}");
    }
    let flattened = trimmed.replace(['\r', '\n'], " | ").replace('\0', "\\0");
    let mut retained: String = flattened;
    if retained.len() > DIAGNOSTIC_BOUND {
        // Keep the last DIAGNOSTIC_BOUND bytes, snapped forward to a char
        // boundary so the retained window is always valid UTF-8.
        let target = retained.len() - DIAGNOSTIC_BOUND;
        let start = (target..retained.len())
            .find(|index| retained.is_char_boundary(*index))
            .unwrap_or(retained.len());
        retained = format!("{TRUNCATION_MARKER}{}", &retained[start..]);
    }
    format!("diagnostic: {retained}")
}

fn guard_failure(
    kind: impl Into<String>,
    path: impl Into<String>,
    observed: impl Into<String>,
    expected: impl Into<String>,
) -> GuardFailure {
    let kind = kind.into();
    let path = path.into();
    let observed = observed.into();
    let expected = expected.into();
    let name = if path.is_empty() {
        kind.clone()
    } else {
        format!("{kind}: {path}")
    };
    GuardFailure {
        kind,
        path,
        observed,
        expected,
        name,
    }
}

fn required_string<'a>(
    table: &'a toml::map::Map<String, toml::Value>,
    key: &str,
    kind: &str,
) -> Result<&'a str, GuardFailure> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .ok_or_else(|| guard_failure(kind, key, format!("missing or non-string {key}"), "string"))
}

fn reject_guard_keys(
    table: &toml::map::Map<String, toml::Value>,
    allowed: &[&str],
    path: &str,
) -> Result<(), GuardFailure> {
    if let Some(unknown) = table
        .keys()
        .find(|key| !allowed.iter().any(|allowed| allowed == key))
    {
        return Err(guard_failure(
            table
                .get("kind")
                .and_then(toml::Value::as_str)
                .unwrap_or("malformed"),
            path,
            format!("unsupported guard key {unknown}"),
            format!("keys {allowed:?}"),
        ));
    }
    Ok(())
}
