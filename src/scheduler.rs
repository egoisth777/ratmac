use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread;

use crate::graph::{MachineGraph, Phase};
use crate::machine::{GuardKind, MachineClass};
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

#[cfg(feature = "test-fault-injection")]
fn inject_step_fault(boundary: &str) -> Result<(), StateError> {
    if std::env::var("RATMAC_TEST_STEP_FAULT").ok().as_deref() == Some(boundary) {
        return Err(StateError::new(format!(
            "injected step fault at {boundary}"
        )));
    }
    Ok(())
}

#[cfg(not(feature = "test-fault-injection"))]
fn inject_step_fault(_boundary: &str) -> Result<(), StateError> {
    Ok(())
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
///
/// Runs reside under the plural `.arca/runs/<id>/` path (FDC-004). Commands
/// that act on an existing Run address it explicitly: the Scheduler binds to
/// one run via [`Scheduler::open_run`] or by minting one in
/// [`Scheduler::start`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scheduler {
    machine: MachineGraph,
    root: Option<PathBuf>,
    run_id: Option<String>,
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
            run_id: None,
            store: None,
        }
    }

    /// Open a project without creating or modifying any scheduler-owned file.
    ///
    /// No run is addressed yet: `start` mints one, and `open_run` binds to an
    /// existing one. State operations refuse until a run is addressed.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, StateError> {
        let root = root.as_ref().to_path_buf();
        let machine = Self::graph_of(&Self::load_class(&root)?);
        Self::refuse_flat_residue(&root)?;
        Ok(Self {
            machine,
            run_id: None,
            store: None,
            root: Some(root),
        })
    }

    /// Open a project addressed at one canonical, minted roster member under
    /// `.arca/runs/<run_id>/`.
    pub fn open_run(root: impl AsRef<Path>, run_id: impl AsRef<str>) -> Result<Self, StateError> {
        let run_id = run_id.as_ref();
        let root = root.as_ref().to_path_buf();
        // FDC-004: caller input is proved to be one canonical direct-child
        // name on the roster before it participates in any path join.
        Self::validate_run_address(&root, run_id)?;
        let machine = Self::graph_of(&Self::load_class(&root)?);
        Self::refuse_flat_residue(&root)?;
        let run_dir = Self::runs_dir(&root).join(run_id);
        // FDC-006: a roster entry without a State File is a retired run —
        // terminal, never resurrected. The refusal names the run as terminal,
        // distinct from an unknown id (refused with the roster listing).
        if !run_dir.join("state.toml").is_file() {
            return Err(StateError::new(format!(
                "run {run_id} is terminal: its admission state is retired and no \
                 transition may proceed on it; address a live run or mint a fresh \
                 one with rtm start"
            )));
        }
        Self::verify_runbook_pin(&root, &run_dir)?;
        Ok(Self {
            machine,
            run_id: Some(run_id.to_owned()),
            store: Some(StateStore::for_run(&root, run_id)),
            root: Some(root),
        })
    }

    /// FDC-005: a pre-plural flat `.arca/state.toml` is residue, never
    /// adopted. Meeting one refuses, names the observed fact and the repair,
    /// and modifies nothing — the legacy-lock precedent, never an
    /// auto-migration. The check runs at open and again at the top of
    /// `start`, before any run is minted: `start` on a residue-carrying
    /// project names the residue and mints nothing.
    fn refuse_flat_residue(root: &Path) -> Result<(), StateError> {
        let flat = root.join(".arca").join("state.toml");
        if fs::symlink_metadata(&flat).is_ok() {
            return Err(StateError::new(format!(
                "refusing to run: flat-layout residue {} exists; runs reside under \
                 .arca/runs/<id>/ — explicitly migrate that file into its run's directory \
                 or remove it, then retry; it was not modified",
                flat.display()
            )));
        }
        Ok(())
    }

    /// FDC-005: SHA-256 of the canonical runbook, lowercase hex. The runbook
    /// pin is this hash and nothing more; no code path copies the runbook.
    fn runbook_sha256(root: &Path) -> Result<String, StateError> {
        let path = root.join(".arca/ratmac.toml");
        crate::pin::sha256_file(&path).map_err(|error| {
            StateError::new(format!(
                "hash .arca/ratmac.toml: {error} ({})",
                path.display()
            ))
        })
    }

    /// FDC-005: every Scheduler read of the class compares the on-disk
    /// runbook against the run's recorded pin; a mismatch refuses naming
    /// observed and expected identity, and writes nothing. A run whose
    /// evidence records no runbook pin predates the pin and is not checked.
    fn verify_runbook_pin(root: &Path, run_dir: &Path) -> Result<(), StateError> {
        let Some(expected) = crate::pin::Evidence::load(run_dir).runbook_sha256 else {
            return Ok(());
        };
        let observed = Self::runbook_sha256(root)?;
        if observed != expected {
            return Err(StateError::new(format!(
                "runbook pin mismatch: .arca/ratmac.toml drifted since rtm start — \
                 observed sha256={observed}; expected sha256={expected}; restore the \
                 pinned runbook bytes or retire the run (rtm abandon); nothing was modified"
            )));
        }
        Ok(())
    }

    /// The addressed run's id, present after `open_run` or a successful `start`.
    pub fn run_id(&self) -> Option<&str> {
        self.run_id.as_deref()
    }

    /// The plural runs directory for a project root.
    pub fn runs_dir(root: impl AsRef<Path>) -> PathBuf {
        root.as_ref().join(".arca").join("runs")
    }

    /// Listing `.arca/runs/` IS the roster: direct run-directory artifacts,
    /// sorted. Symlinks are not Run directories and cannot put a roster member
    /// outside the plural residency path.
    pub fn run_roster(root: impl AsRef<Path>) -> Vec<String> {
        let Ok(entries) = fs::read_dir(Self::runs_dir(root)) else {
            return Vec::new();
        };
        let mut ids: Vec<String> = entries
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_type()
                    .map(|file_type| file_type.is_dir())
                    .unwrap_or(false)
            })
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        ids.sort();
        ids
    }

    /// Validate caller-supplied run identity without joining it to a path.
    ///
    /// A usable address is exactly the canonical spelling minted by `start`
    /// and exactly equals one direct roster member. Every refusal carries the
    /// roster so command surfaces can report it without probing a candidate.
    pub(crate) fn validate_run_address(root: &Path, run_id: &str) -> Result<(), StateError> {
        let roster = Self::run_roster(root);
        let roster_line = if roster.is_empty() {
            "none".to_owned()
        } else {
            roster.join(", ")
        };
        if Self::canonical_run_ordinal(run_id).is_none() {
            return Err(StateError::new(format!(
                "run id {run_id:?} is not one canonical minted path segment; runs: {roster_line}"
            )));
        }
        if !roster.iter().any(|entry| entry == run_id) {
            return Err(StateError::new(format!(
                "run id {run_id:?} is not an exact roster member; runs: {roster_line}"
            )));
        }
        Ok(())
    }

    fn canonical_run_ordinal(run_id: &str) -> Option<u64> {
        let digits = run_id.strip_prefix("run-")?;
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let ordinal = digits.parse::<u64>().ok()?;
        if ordinal == 0 || format!("run-{ordinal:03}") != run_id {
            return None;
        }
        Some(ordinal)
    }

    /// Mint the next run id in the single id namespace: one more than the
    /// highest canonical ordinal on the roster — live or retired — so an
    /// abandoned run's id is never reissued (FDC-006).
    fn mint_run_id(root: &Path) -> String {
        let next = Self::run_roster(root)
            .iter()
            .filter_map(|id| Self::canonical_run_ordinal(id))
            .max()
            .unwrap_or(0)
            + 1;
        format!("run-{next:03}")
    }

    fn run_dir(&self) -> Result<PathBuf, StateError> {
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| StateError::new("run addressing requires Scheduler::open"))?;
        let run_id = self.run_id.as_deref().ok_or_else(|| {
            StateError::new(
                "no run addressed: open one with Scheduler::open_run or mint one with start",
            )
        })?;
        Ok(Self::runs_dir(root).join(run_id))
    }

    /// TRP-001, TRP-005: the one reader. An absent or unreadable runbook is a
    /// refusal that names the path, never an empty machine.
    fn load_class(root: &Path) -> Result<MachineClass, StateError> {
        let path = root.join(".arca/ratmac.toml");
        let source = fs::read_to_string(&path).map_err(|error| {
            StateError::new(format!(
                "read .arca/ratmac.toml: {error} ({})",
                path.display()
            ))
        })?;
        MachineClass::from_toml(&source)
            .map_err(|error| StateError::new(format!("parse .arca/ratmac.toml: {error}")))
    }

    fn graph_of(class: &MachineClass) -> MachineGraph {
        let phases = class.phases().keys().map(Phase::new).collect::<Vec<_>>();
        MachineGraph::new(phases, class.transitions().to_vec())
    }

    pub fn machine(&self) -> &MachineGraph {
        &self.machine
    }

    /// Instantiate a Run from the canonical, human-authored Machine Class.
    ///
    /// FDC-004: start mints a run id in the single namespace and creates
    /// `.arca/runs/<id>/` with its durable State File and Run evidence.
    /// FDC-003: the `verdict.toml` live slot is absent when empty; the
    /// `spawn-ledger` path remains reserved by name only for machine
    /// composition. No flat `.arca/state.toml` is written.
    ///
    /// State and log persist after return. The lock is held only for this
    /// invocation and is released by the RAII guard before the Run is observed.
    /// An interrupted start removes the half-made run directory, so the roster
    /// never lists a run that was not fully created.
    pub fn start(&mut self) -> Result<Run, StateError> {
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| StateError::new("start requires Scheduler::open"))?
            .clone();
        self.machine = Self::graph_of(&Self::load_class(&root)?);
        let phase = self.initial_phase()?;
        // FDC-002: a Run beginning in a terminal Phase — no ordinary outgoing
        // edge — is complete from its first State File. The Engine writes the
        // terminal fact; no agent claim participates.
        let initial_status = if self.machine.has_ordinary_outgoing(phase.as_str()) {
            Status::Planned
        } else {
            Status::Passed
        };
        let arca = root.join(".arca");
        fs::create_dir_all(&arca)
            .map_err(|error| StateError::new(format!("create .arca: {error}")))?;
        let lock_path = arca.join("rtm.lock");
        let _lock = InvocationLock::acquire_with_retry(&lock_path)?;
        // FDC-005: flat-layout residue refuses before any run id is minted,
        // so the refusal names the residue and no run directory is created.
        Self::refuse_flat_residue(&root)?;
        // FDC-005: the runbook pin recorded in the run's evidence is the
        // SHA-256 of the canonical runbook — a hash and nothing more.
        let runbook_pin = Self::runbook_sha256(&root)?;
        // FDC-006: no active-Run cap. Any number of runs coexist under
        // .arca/runs/, each addressed by its own id; start only mints the
        // next id over the unfiltered roster — live or retired.
        let run_id = Self::mint_run_id(&root);
        let runs_dir = Self::runs_dir(&root);
        fs::create_dir_all(&runs_dir)
            .map_err(|error| StateError::new(format!("create .arca/runs: {error}")))?;
        let run_dir = runs_dir.join(&run_id);
        fs::create_dir(&run_dir)
            .map_err(|error| StateError::new(format!("create run directory {run_id}: {error}")))?;

        let state = RunState {
            phase: phase.to_string(),
            status: initial_status,
            goal_revision: String::new(),
            input_revision: String::new(),
            output_revision: String::new(),
            active_refs: Vec::new(),
            blocker: String::new(),
        };
        let log_path = arca.join("log.md");
        let log_existed = log_path.exists();
        let store = StateStore::for_run(&root, &run_id);
        let create_run = || -> Result<(), StateError> {
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
            if let Err(state_error) = store.write(&state) {
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

            // ETB-001: Run evidence carries the Stable Engine pin from Run
            // start, so every later gate pin is recorded beside a known Engine
            // identity.
            let mut evidence = crate::pin::Evidence::load(&run_dir);
            if let Some(identity) = crate::pin::engine_identity() {
                evidence.set_engine(identity);
            }
            // ETB-003: Run start records the baseline goal revision. The
            // freeze happens later, at the intake-completion boundary.
            evidence.goal_baseline = crate::goal::revision(&root);
            evidence.goal_frozen = None;
            // FDC-005: record the runbook pin — hash only, never a copy.
            evidence.runbook_sha256 = Some(runbook_pin.clone());
            evidence
                .write(&run_dir)
                .map_err(|error| StateError::new(format!("write evidence.toml: {error}")))?;

            // FDC-003/FDC-004: an empty Verdict slot is absence. Only the
            // per-run spawn-ledger path remains reserved by name here.
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(run_dir.join("spawn-ledger"))
                .map_err(|error| {
                    StateError::new(format!("reserve spawn-ledger under {run_id}: {error}"))
                })?;
            Ok(())
        };
        if let Err(error) = create_run() {
            // No half-made run directory may remain on the roster.
            let _ = fs::remove_dir_all(&run_dir);
            return Err(error);
        }
        self.store = Some(store);
        self.run_id = Some(run_id.clone());

        Ok(Run::new(phase, initial_status).with_artifacts(&root, &run_id))
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
        let run_dir = self.run_dir()?;
        let lock_path = root.join(".arca/rtm.lock");
        let _lock = InvocationLock::acquire_with_retry(&lock_path)?;
        let class = Self::load_class(&root)?;
        // FDC-005: no transition may proceed on a drifted class — the pin
        // check refuses before any guard of the drifted class is evaluated.
        Self::verify_runbook_pin(&root, &run_dir)?;
        self.machine = Self::graph_of(&class);
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
        // FDC-002: a passed Run admits no further transition. The refusal
        // precedes guard and verdict work and mutates nothing.
        if state.status == Status::Passed {
            return Ok(StepOutcome::Refused {
                failures: vec![guard_failure(
                    "terminal",
                    state_phase,
                    "run is terminal (status passed): no transition may proceed",
                    "a live, non-terminal Run",
                )],
            });
        }
        let mut failures = self.guard_failures(&class, &state.phase)?;
        // ETB-003: between the freeze and batch closure, the goal is fixed.
        // The drift check is appended rather than short-circuited so a guard
        // refusal and a drift refusal are reported in the same reply.
        let evidence = crate::pin::Evidence::load(&run_dir);
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
        let definition = class
            .phases()
            .get(&state.phase)
            .ok_or_else(|| StateError::new("current Phase definition disappeared"))?;
        // FDC-003/FDC-001: every readiness guard above finishes before the
        // live slot is inspected. Branches validate one external record;
        // straight lines read no record and reject any occupied slot.
        let transition_input = if let Some(inputs) = definition.inputs() {
            match crate::verdict::load_live(&run_dir, &state.phase, inputs) {
                Ok(record) => Some(record.input().to_owned()),
                Err(refusal) => {
                    return Ok(StepOutcome::Refused {
                        failures: vec![guard_failure(
                            "verdict",
                            "verdict.toml",
                            refusal.observed(),
                            refusal.expected(),
                        )],
                    })
                }
            }
        } else {
            if crate::verdict::live_slot_is_occupied(&run_dir)? {
                return Ok(StepOutcome::Refused {
                    failures: vec![guard_failure(
                        "verdict",
                        "verdict.toml",
                        "live record presented to a straight-line Phase",
                        "absent live verdict slot",
                    )],
                });
            }
            None
        };
        let Some(transition) = Self::route_for(&self.machine, &from, transition_input.as_deref())
        else {
            return Ok(StepOutcome::Refused {
                failures: vec![guard_failure(
                    "transition",
                    state.phase,
                    "no matching outgoing transition",
                    "one transition selected by the current Phase and input",
                )],
            });
        };
        let consumes_verdict = transition_input.is_some();
        let freezes_goal = transition.freezes_goal();
        let to = transition.to().clone();
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

        // Resolve every ordinary fallible prerequisite before the irreversible
        // Verdict rename. Freeze evidence is still written only after
        // consumption and before the successor State File.
        let frozen_revision = if freezes_goal {
            Some(
                crate::goal::revision(&root)
                    .ok_or_else(|| StateError::new("cannot freeze goal: .arca/goal/ is absent"))?,
            )
        } else {
            None
        };
        if consumes_verdict {
            inject_step_fault("before-verdict-archive")?;
            crate::verdict::archive_live(&run_dir)?;
            inject_step_fault("before-state-replace")?;
        }

        let mut next = state;
        next.phase = to.to_string();
        // FDC-002: arrival at a Phase with no ordinary outgoing edge completes
        // ordinary execution. The terminal fact lands in the same atomic State
        // File replacement that records the position.
        if !self.machine.has_ordinary_outgoing(to.as_str()) {
            next.status = Status::Passed;
        }
        if let Some(frozen) = frozen_revision {
            // ETB-003 and FDC-003: freeze evidence follows Verdict consumption
            // but still precedes the successor State File. Failure leaves the
            // old Phase with an archived, non-replayable verdict.
            let mut frozen_evidence = crate::pin::Evidence::load(&run_dir);
            frozen_evidence.goal_frozen = Some(frozen.clone());
            if let Err(error) = frozen_evidence.write(&run_dir) {
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
        if consumes_verdict {
            inject_step_fault("after-state-replace")?;
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
            // FDC-003: after Verdict consumption and successor replacement,
            // advancing is durable even when the later history append fails.
            // Restoring the old Phase here would make the archived judgment
            // look replayable while its live slot is already gone.
            if !consumes_verdict {
                if let Err(error) = self.store()?.write(&prior) {
                    rollback_errors.push(format!("restore state.toml: {error}"));
                }
            }
            let message = if consumes_verdict {
                let durable = format!(
                    "the verdict was consumed and the Run advanced {from} -> {to}; \
                     the transition history line is missing and must be appended after restoring log.md"
                );
                if rollback_errors.is_empty() {
                    format!("append log.md failed: {append_error}; {durable}")
                } else {
                    format!(
                        "append log.md failed: {append_error}; {durable}; log cleanup failed: {}",
                        rollback_errors.join("; ")
                    )
                }
            } else if rollback_errors.is_empty() {
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

    /// TRP-001, TRP-004: evaluate the Phase's retained guards, in declaration
    /// order, from the typed class - no second walk over runbook TOML.
    fn guard_failures(
        &self,
        class: &MachineClass,
        phase: &str,
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
        let Some(definition) = class.phases().get(phase) else {
            return Ok(vec![guard_failure(
                "ratmac",
                phase,
                "missing phase definition",
                "current Phase definition",
            )]);
        };

        let mut failures = Vec::new();
        for guard in definition.guards() {
            let result = match guard {
                GuardKind::FilesExact {
                    path,
                    entries,
                    files,
                } => self.evaluate_files_exact(root, path, entries.as_deref(), files.as_deref()),
                GuardKind::FileContains { path, contains } => {
                    self.evaluate_file_contains(root, path, contains)
                }
                GuardKind::CommandExit {
                    program,
                    args,
                    expected,
                    exempt,
                } => self.evaluate_command_exit(root, program, args, *expected, *exempt),
                GuardKind::SensitivityReceipts { ticket } => {
                    self.evaluate_sensitivity_receipts(root, ticket)
                }
                GuardKind::CompletionGate { ticket } => self.evaluate_completion_gate(root, ticket),
                GuardKind::IntakeContract => self.evaluate_contract(
                    "intake_contract",
                    crate::contract::gate_intake(root),
                    "issue dispositions, status, and location agree across intake/deferred/archive; five-file shape intact; accepted IDs in the goal; live links resolving",
                ),
                GuardKind::Join { min, .. } => self.evaluate_join(*min),
                GuardKind::RecordContract => self.evaluate_contract(
                    "record_contract",
                    crate::contract::gate_records(root, self.run_id.as_deref().unwrap_or_default()),
                    "one residual per requirement citing the frozen revision, evidence behind every satisfied, one owning ticket per gap, acyclic dependencies, complete tickets",
                ),
            };
            if let Err(failure) = result {
                failures.push(failure);
            }
        }
        Ok(failures)
    }

    /// FDC-009: the composition join. Until a spawn ledger records children
    /// (FDC-011, t-066), the honest verdict is that zero children are passed,
    /// so a join guard cannot be satisfied.
    fn evaluate_join(&self, min: Option<i64>) -> Result<(), GuardFailure> {
        let required = min.unwrap_or(1);
        Err(guard_failure(
            "join",
            "spawn ledger",
            format!(
                "no spawn ledger records a child Run; 0 of {required} required children are passed"
            ),
            "every ledger-recorded live child passed, at least the declared min",
        ))
    }

    /// PGE-003: the P4 gate. Every planned test the ticket declares must
    /// resolve to a sensitivity receipt under `.arca/evidence/`; prose,
    /// filenames, and status fields satisfy nothing. The predicate runs
    /// in-process, inside the pinned gate boundary, so no external program is
    /// trusted to decide whether the work was done.
    fn evaluate_sensitivity_receipts(&self, root: &Path, ticket: &str) -> Result<(), GuardFailure> {
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
    fn evaluate_completion_gate(&self, root: &Path, ticket: &str) -> Result<(), GuardFailure> {
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
        path: &str,
        entries: Option<&[String]>,
        files: Option<&[String]>,
    ) -> Result<(), GuardFailure> {
        let entries = match (entries, files) {
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
        let expected = entries
            .iter()
            .cloned()
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
        path: &str,
        contains: &str,
    ) -> Result<(), GuardFailure> {
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
        program: &str,
        args: &[String],
        expected: i64,
        exempt: bool,
    ) -> Result<(), GuardFailure> {
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
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

        let run_dir = self.run_dir().map_err(|error| {
            guard_failure(
                "command_exit",
                program,
                error.to_string(),
                "an addressed run carrying Run evidence",
            )
        })?;
        let mut evidence = crate::pin::Evidence::load(&run_dir);
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
                evidence.write(&run_dir).map_err(|error| {
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
        self.store.as_ref().ok_or_else(|| {
            StateError::new(
                "state operations require an addressed run: Scheduler::open_run or start",
            )
        })
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

    /// FDC-001: the routing function depends only on current Phase and the
    /// exact validated input. Declaration order and guards do not select.
    fn route_for<'a>(
        machine: &'a MachineGraph,
        phase: &Phase,
        input: Option<&str>,
    ) -> Option<&'a crate::graph::Transition> {
        machine.transition_for_input(phase, input)
    }

    /// Read-only status; it loads state and reports labels from the current class.
    pub fn status(&self) -> Result<StatusReport, StateError> {
        let _lock = self.invocation_lock_with_retry()?;
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| StateError::new("status requires Scheduler::open"))?;
        let class = Self::load_class(root)?;
        // FDC-005: status reads the class too; an addressed run's recorded
        // pin is compared on every such read.
        if self.run_id.is_some() {
            Self::verify_runbook_pin(root, &self.run_dir()?)?;
        }
        let machine = Self::graph_of(&class);
        let state = self.load_state_unlocked()?;
        if !machine.phases().any(|phase| phase.as_str() == state.phase) {
            return Err(StateError::new(format!(
                "State File phase {:?} is undeclared in ratmac.toml",
                state.phase
            )));
        }
        let pending_guards = Self::pending_guard_labels(&class, &state.phase);
        let phase_prompt = Self::render_phase_prompt(&class, &state.phase)?;
        Ok(StatusReport {
            state,
            pending_guards,
            phase_prompt,
        })
    }

    /// R-028: the Phase Prompt is the authored prose plus the generated list of
    /// this Phase's Exit Guards - rendered from the typed guards, so what the
    /// agent reads is what the Scheduler will evaluate.
    fn render_phase_prompt(class: &MachineClass, phase: &str) -> Result<PhasePrompt, StateError> {
        let definition = class
            .phases()
            .get(phase)
            .ok_or_else(|| StateError::new(format!("missing phase definition: {phase}")))?;
        let mut rendered = definition.prompt().to_owned();
        let guards = definition.guards();
        if !guards.is_empty() {
            rendered.push_str("\n\nExit Guards:\n");
            for guard in guards {
                rendered.push_str("- ");
                rendered.push_str(guard.name());
                for (key, value) in guard.rendered_fields() {
                    rendered.push(' ');
                    rendered.push_str(key);
                    rendered.push('=');
                    rendered.push_str(&value);
                }
                rendered.push('\n');
            }
            rendered.pop();
        }
        if let Some(inputs) = definition.inputs() {
            rendered.push_str("\n\nLegal transition inputs:\n");
            for input in inputs {
                rendered.push_str("- ");
                rendered.push_str(input);
                rendered.push('\n');
            }
            rendered.pop();
        }
        Ok(PhasePrompt::new(rendered))
    }

    fn pending_guard_labels(class: &MachineClass, phase: &str) -> Vec<String> {
        class
            .phases()
            .get(phase)
            .map_or_else(Vec::new, |definition| {
                definition
                    .guards()
                    .iter()
                    .map(|guard| guard.name().to_owned())
                    .collect()
            })
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
