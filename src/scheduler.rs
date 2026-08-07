use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::graph::{MachineGraph, Phase};
use crate::ledger::LedgerEntry;
use crate::lock::{RootLock, RunLock};
use crate::machine::{GuardKind, MachineClass, PhaseDefinition};
use crate::model::{Run, RunState, Status};
use crate::state::{PhasePrompt, StateError, StateStore, StatusReport};

/// What a human asked for when superseding a Run (FDC-007/FDC-006).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RespawnRequest {
    /// The run to supersede: `--run <id>`, always required.
    pub run: Option<String>,
    /// The typed confirmation phrase, `respawn <id>`, naming that run id.
    pub confirmation: Option<String>,
}

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
/// Runs reside under the resolved Engine root's plural
/// `.ratmac/runs/<id>/` path. Commands that act on an existing Run address it
/// explicitly: the Scheduler binds to one run via [`Scheduler::open_run`] or
/// by minting one in [`Scheduler::start`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scheduler {
    machine: MachineGraph,
    /// The addressed Run's workspace. For a top-level Run this is the
    /// invoking checkout; for a child it is the durable ledger binding.
    /// Guards, goal reads, and gate-program resolution use this root.
    root: Option<PathBuf>,
    /// The checkout that invoked this Scheduler. Machine Class and runbook-pin
    /// reads remain invocation inputs so linked-worktree pin drift stays
    /// observable under ENS-002.
    invoking_root: Option<PathBuf>,
    /// The resolved runtime root, shared by linked worktrees.
    engine_root: Option<PathBuf>,
    run_id: Option<String>,
    /// The exact Machine Class recorded for an addressed child. Its phase
    /// names can overlap sibling classes, so phase lookup never guesses.
    child_class: Option<String>,
    store: Option<StateStore>,
}

/// Exact artifacts created for a freshly minted Run. A later compensating
/// rollback may remove the directory only after these bytes and its direct
/// entries still match.
#[derive(Clone, Debug)]
struct MintedRunSnapshot {
    state_bytes: Vec<u8>,
    evidence_bytes: Vec<u8>,
    spawn_ledger_bytes: Vec<u8>,
}

/// A fresh Run remains motion-locked while a caller completes the transaction
/// that made it addressable (notably spawn's parent-ledger entry).
#[derive(Debug)]
struct MintedRun {
    id: String,
    run_lock: RunLock,
    snapshot: MintedRunSnapshot,
}

/// Exact bytes a guard read from one spawn ledger. `None` records the
/// semantically empty, absent ledger so a later creation is also a change.
#[derive(Debug)]
struct LedgerSnapshot {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
}

/// A failed append is resolved from the ledger bytes that actually remain,
/// rather than assuming an I/O error means no bytes reached the file.
#[derive(Debug)]
enum LedgerAppendAftermath {
    Appended,
    RecordedAfterError(String),
    AbsentAfterError(String),
    Indeterminate(String),
}

impl Scheduler {
    /// Construct the in-memory scheduler used by the entry-prerequisite model.
    pub fn new(machine: MachineGraph) -> Self {
        Self {
            machine,
            root: None,
            invoking_root: None,
            engine_root: None,
            run_id: None,
            child_class: None,
            store: None,
        }
    }

    /// Open a project without creating or modifying any scheduler-owned file.
    ///
    /// No run is addressed yet: `start` mints one, and `open_run` binds to an
    /// existing one. State operations refuse until a run is addressed.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, StateError> {
        let roots = crate::root::resolve(root);
        let root = roots.invoking_checkout_root().to_path_buf();
        let engine_root = roots.engine_root().to_path_buf();
        let machine = Self::graph_of(&Self::load_class(&root)?);
        Self::refuse_flat_residue(&root)?;
        Ok(Self {
            machine,
            run_id: None,
            child_class: None,
            store: None,
            root: Some(root.clone()),
            invoking_root: Some(root),
            engine_root: Some(engine_root),
        })
    }

    /// Open a project addressed at one canonical, minted roster member under
    /// `.ratmac/runs/<run_id>/`.
    pub fn open_run(root: impl AsRef<Path>, run_id: impl AsRef<str>) -> Result<Self, StateError> {
        let run_id = run_id.as_ref();
        let roots = crate::root::resolve(root);
        let invoking_root = roots.invoking_checkout_root().to_path_buf();
        let engine_root = roots.engine_root().to_path_buf();
        // FDC-004: caller input is proved to be one canonical direct-child
        // name on the roster before it participates in any path join.
        Self::validate_run_address_at(&engine_root, run_id)?;
        // ENS-006: a child never derives its workspace or class scope from
        // this caller. A top-level Run has no ledger entry and therefore
        // keeps that caller's checkout as its workspace.
        let (workspace, child_class) = match Self::ledger_record_of_at(&engine_root, run_id)? {
            Some((ledger_path, entry)) => (
                Self::workspace_from_ledger_entry(&engine_root, &ledger_path, &entry)?,
                Some(entry.class),
            ),
            None => (invoking_root.clone(), None),
        };
        let machine = Self::graph_of(&Self::load_class(&invoking_root)?);
        Self::refuse_flat_residue(&invoking_root)?;
        let run_dir = Self::runs_dir_at(&engine_root).join(run_id);
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
        Self::verify_runbook_pin(&invoking_root, &run_dir)?;
        Ok(Self {
            machine,
            run_id: Some(run_id.to_owned()),
            child_class,
            store: Some(StateStore::for_run(&workspace, run_id)),
            root: Some(workspace),
            invoking_root: Some(invoking_root),
            engine_root: Some(engine_root),
        })
    }

    /// FDC-005: a pre-plural flat `.ratmac/state.toml` or pre-split flat
    /// `.arca/state.toml` is residue, never adopted. Meeting one refuses,
    /// names the observed fact and the repair, and modifies nothing — the
    /// legacy-lock precedent, never an auto-migration. The check runs at open
    /// and again at the top of `start`, before any run is minted: `start` on a
    /// residue-carrying project names the residue and mints nothing.
    fn refuse_flat_residue(root: &Path) -> Result<(), StateError> {
        let engine_root = crate::root::resolve(root).engine_root().to_path_buf();
        Self::refuse_flat_residue_at(root, &engine_root)
    }

    fn refuse_flat_residue_at(root: &Path, engine_root: &Path) -> Result<(), StateError> {
        for flat in [
            engine_root.join("state.toml"),
            root.join(".arca").join("state.toml"),
        ] {
            if fs::symlink_metadata(&flat).is_ok() {
                return Err(StateError::new(format!(
                    "refusing to run: flat-layout residue {} exists; runs reside under \
                     .ratmac/runs/<id>/ — explicitly migrate that file into its run's directory \
                     or remove it, then retry; it was not modified",
                    flat.display()
                )));
            }
        }
        Ok(())
    }

    /// FDC-005: SHA-256 of the canonical invoking-checkout runbook, lowercase
    /// hex. The runbook pin is this hash and nothing more; no code path copies
    /// the runbook.
    fn runbook_sha256(root: &Path) -> Result<String, StateError> {
        let path = root.join(".ratmac").join("ratmac.toml");
        crate::pin::sha256_file(&path).map_err(|error| {
            StateError::new(format!(
                "hash .ratmac/ratmac.toml: {error} ({})",
                path.display()
            ))
        })
    }

    /// FDC-005: every Scheduler read of the class compares the on-disk
    /// runbook against the run's recorded pin; a mismatch refuses naming
    /// observed and expected identity, and writes nothing. A run whose
    /// evidence records no runbook pin predates the pin and is not checked.
    fn verify_runbook_pin(root: &Path, run_dir: &Path) -> Result<(), StateError> {
        let observed = Self::runbook_sha256(root)?;
        Self::verify_runbook_pin_hash(run_dir, &observed)
    }

    /// Compare a Run's recorded pin to a hash already computed before a root
    /// mutation lock is taken.
    fn verify_runbook_pin_hash(run_dir: &Path, observed: &str) -> Result<(), StateError> {
        let Some(expected) = crate::pin::Evidence::load(run_dir).runbook_sha256 else {
            return Ok(());
        };
        if observed != expected {
            return Err(StateError::new(format!(
                "runbook pin mismatch: .ratmac/ratmac.toml drifted since rtm start — \
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

    /// The plural runs directory for an invoking checkout.
    pub fn runs_dir(root: impl AsRef<Path>) -> PathBuf {
        let engine_root = crate::root::resolve(root).engine_root().to_path_buf();
        Self::runs_dir_at(&engine_root)
    }

    fn runs_dir_at(engine_root: &Path) -> PathBuf {
        engine_root.join("runs")
    }

    /// Listing the resolved `.ratmac/runs/` is the roster: direct
    /// run-directory artifacts, sorted. Symlinks are not Run directories and
    /// cannot put a roster member outside the plural residency path.
    pub fn run_roster(root: impl AsRef<Path>) -> Vec<String> {
        let engine_root = crate::root::resolve(root).engine_root().to_path_buf();
        Self::run_roster_at(&engine_root)
    }

    fn run_roster_at(engine_root: &Path) -> Vec<String> {
        let Ok(entries) = fs::read_dir(Self::runs_dir_at(engine_root)) else {
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
        let engine_root = crate::root::resolve(root).engine_root().to_path_buf();
        Self::validate_run_address_at(&engine_root, run_id)
    }

    fn validate_run_address_at(engine_root: &Path, run_id: &str) -> Result<(), StateError> {
        let roster = Self::run_roster_at(engine_root);
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

    fn engine_root(&self) -> Result<&Path, StateError> {
        self.engine_root
            .as_deref()
            .ok_or_else(|| StateError::new("operation requires Scheduler::open"))
    }

    fn invoking_root(&self) -> Result<&Path, StateError> {
        self.invoking_root
            .as_deref()
            .ok_or_else(|| StateError::new("operation requires Scheduler::open"))
    }

    fn run_dir(&self) -> Result<PathBuf, StateError> {
        let engine_root = self.engine_root()?;
        let run_id = self.run_id.as_deref().ok_or_else(|| {
            StateError::new(
                "no run addressed: open one with Scheduler::open_run or mint one with start",
            )
        })?;
        Ok(Self::runs_dir_at(engine_root).join(run_id))
    }

    /// TRP-001, TRP-005: the one reader. An absent or unreadable runbook is a
    /// refusal that names the path, never an empty machine.
    fn load_class(root: &Path) -> Result<MachineClass, StateError> {
        let path = root.join(".ratmac").join("ratmac.toml");
        let source = fs::read_to_string(&path).map_err(|error| {
            StateError::new(format!(
                "read .ratmac/ratmac.toml: {error} ({})",
                path.display()
            ))
        })?;
        MachineClass::from_toml(&source)
            .map_err(|error| StateError::new(format!("parse .ratmac/ratmac.toml: {error}")))
    }

    fn graph_of(class: &MachineClass) -> MachineGraph {
        let phases = class.phases().keys().map(Phase::new).collect::<Vec<_>>();
        MachineGraph::new(phases, class.transitions().to_vec())
    }
    pub fn machine(&self) -> &MachineGraph {
        &self.machine
    }

    /// The invoking checkout's Git revision at spawn; `"none"` when it has
    /// no readable Git HEAD. This is ledger provenance, distinct from the
    /// content-hashed goal baseline and from the later byte snapshots used to
    /// revalidate a motion.
    fn revision_at(root: &Path) -> String {
        std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "none".to_owned())
    }

    /// Find one child ledger entry strictly. An absent ledger is empty by
    /// contract, but any unreadable or malformed ledger is a named refusal:
    /// guessing that a child is top-level would lose its durable workspace.
    /// An abandoned mark does not erase that fact: a partially failed
    /// retirement can leave its State File admitted after the mark landed.
    fn ledger_record_of_at(
        engine_root: &Path,
        run_id: &str,
    ) -> Result<Option<(PathBuf, LedgerEntry)>, StateError> {
        for candidate in Self::run_roster_at(engine_root) {
            let path = Self::runs_dir_at(engine_root)
                .join(&candidate)
                .join("spawn-ledger");
            let entries = crate::ledger::read_entries(&path).map_err(|error| {
                StateError::new(format!(
                    "cannot read spawn ledger {} while resolving run {run_id}: {error}",
                    path.display()
                ))
            })?;
            if let Some(entry) = entries.into_iter().find(|entry| entry.id == run_id) {
                return Ok(Some((path, entry)));
            }
        }
        Ok(None)
    }

    fn workspace_from_ledger_entry(
        engine_root: &Path,
        ledger_path: &Path,
        entry: &LedgerEntry,
    ) -> Result<PathBuf, StateError> {
        let recorded = entry.workspace.as_deref().ok_or_else(|| {
            StateError::new(format!(
                "run {} is a child recorded in {} but has no workspace binding; \
                 refusing to fall back to the caller's directory",
                entry.id,
                ledger_path.display()
            ))
        })?;
        let workspace = PathBuf::from(recorded);
        if !workspace.is_absolute() {
            return Err(StateError::new(format!(
                "run {} has a non-absolute workspace binding {:?} in {}; \
                 refusing to fall back to the caller's directory",
                entry.id,
                recorded,
                ledger_path.display()
            )));
        }
        let canonical = fs::canonicalize(&workspace).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                StateError::new(format!(
                    "run {} has a recorded workspace path {} in {} that no longer exists; \
                     refusing to fall back to the caller's directory",
                    entry.id,
                    workspace.display(),
                    ledger_path.display()
                ))
            } else {
                StateError::new(format!(
                    "run {} has an unusable workspace binding {:?} in {}: {error}; \
                     refusing to fall back to the caller's directory",
                    entry.id,
                    recorded,
                    ledger_path.display()
                ))
            }
        })?;
        // A binding identifies this canonical pathname: deleting and
        // recreating a directory at exactly this path keeps the same binding,
        // and later guards judge its contents. A changed resolution is not
        // that path and must never redirect the child through a symlink or
        // junction replacement.
        if canonical != workspace {
            return Err(StateError::new(format!(
                "run {} has a recorded workspace path {} in {} that now resolves to {}; \
                 refusing to fall back to the caller's directory",
                entry.id,
                workspace.display(),
                ledger_path.display(),
                canonical.display()
            )));
        }
        if !canonical.is_dir() {
            return Err(StateError::new(format!(
                "run {} has a workspace binding {} in {} that is not a directory; \
                 refusing to fall back to the caller's directory",
                entry.id,
                canonical.display(),
                ledger_path.display()
            )));
        }
        Self::ensure_workspace_in_repository(engine_root, &canonical)?;
        Ok(canonical)
    }

    /// Canonicalize a caller-supplied spawn workspace before the mint
    /// transaction. Relative spellings are interpreted from the invocation
    /// checkout, not from a parent Run's stored workspace.
    fn canonical_spawn_workspace(
        invoking_root: &Path,
        engine_root: &Path,
        workspace: &Path,
    ) -> Result<PathBuf, StateError> {
        let spelling = workspace.to_string_lossy().into_owned();
        let candidate = if workspace.is_absolute() {
            workspace.to_path_buf()
        } else {
            invoking_root.join(workspace)
        };
        let metadata = fs::metadata(&candidate).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                StateError::new(format!("workspace {spelling:?} does not exist"))
            } else {
                StateError::new(format!(
                    "workspace {spelling:?} cannot be inspected: {error}"
                ))
            }
        })?;
        if !metadata.is_dir() {
            return Err(StateError::new(format!(
                "workspace {spelling:?} is not a directory"
            )));
        }
        let canonical = fs::canonicalize(&candidate).map_err(|error| {
            StateError::new(format!(
                "workspace {spelling:?} cannot be canonicalized: {error}"
            ))
        })?;
        Self::ensure_workspace_in_repository(engine_root, &canonical)?;
        Ok(canonical)
    }

    /// Repository confinement follows the Engine root rather than a lexical
    /// parent-directory prefix, so linked worktrees remain legitimate
    /// workspaces while a symlink or traversal that resolves elsewhere does
    /// not escape the runtime namespace.
    fn ensure_workspace_in_repository(
        engine_root: &Path,
        workspace: &Path,
    ) -> Result<(), StateError> {
        let workspace_engine_root = crate::root::resolve(workspace).engine_root().to_path_buf();
        let engine_root =
            fs::canonicalize(engine_root).unwrap_or_else(|_| engine_root.to_path_buf());
        let workspace_engine_root =
            fs::canonicalize(&workspace_engine_root).unwrap_or(workspace_engine_root);
        if workspace_engine_root == engine_root {
            Ok(())
        } else {
            Err(StateError::new(format!(
                "workspace {} is outside this repository",
                workspace.display()
            )))
        }
    }

    fn snapshot_ledger(path: &Path) -> Result<LedgerSnapshot, StateError> {
        match fs::read(path) {
            Ok(bytes) => Ok(LedgerSnapshot {
                path: path.to_path_buf(),
                bytes: Some(bytes),
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(LedgerSnapshot {
                path: path.to_path_buf(),
                bytes: None,
            }),
            Err(error) => Err(StateError::new(format!(
                "read spawn ledger {}: {error}",
                path.display()
            ))),
        }
    }

    /// A ledger write can report an error after bytes reached the file. The
    /// exact before/after bytes and strict parser classify only the cases in
    /// which deleting the just-minted child is demonstrably safe.
    fn append_ledger_entry(path: &Path, entry: &LedgerEntry) -> LedgerAppendAftermath {
        let before = match Self::snapshot_ledger(path) {
            Ok(snapshot) => snapshot.bytes,
            Err(error) => {
                return LedgerAppendAftermath::Indeterminate(format!(
                    "cannot snapshot the ledger before append: {error}"
                ))
            }
        };
        match crate::ledger::append_entry(path, entry) {
            Ok(()) => LedgerAppendAftermath::Appended,
            Err(append_error) => {
                let after = match Self::snapshot_ledger(path) {
                    Ok(snapshot) => snapshot.bytes,
                    Err(read_error) => {
                        return LedgerAppendAftermath::Indeterminate(format!(
                            "append reported {append_error}; cannot reread the ledger: {read_error}"
                        ))
                    }
                };
                match crate::ledger::read_entries(path) {
                    Ok(entries) if entries.iter().any(|recorded| recorded == entry) => {
                        LedgerAppendAftermath::RecordedAfterError(append_error.to_string())
                    }
                    Ok(entries)
                        if after == before
                            && entries.iter().all(|recorded| recorded.id != entry.id) =>
                    {
                        LedgerAppendAftermath::AbsentAfterError(append_error.to_string())
                    }
                    Ok(_) => LedgerAppendAftermath::Indeterminate(format!(
                        "append reported {append_error}; the ledger bytes changed without a complete matching entry"
                    )),
                    Err(read_error) => LedgerAppendAftermath::Indeterminate(format!(
                        "append reported {append_error}; the ledger is partial or unreadable: {read_error}"
                    )),
                }
            }
        }
    }

    /// Revalidate every ledger queried by a guard immediately before the
    /// durable motion. Root-domain writers remain independent; this catches a
    /// verdict that was computed from bytes no longer present.
    fn ensure_ledger_snapshots_current(
        run_id: &str,
        snapshots: &[LedgerSnapshot],
    ) -> Result<(), StateError> {
        for snapshot in snapshots {
            let current = Self::snapshot_ledger(&snapshot.path).map_err(|error| {
                StateError::new(format!(
                    "state mutation refused for run {run_id}: a ledger the guards read changed while the step was being decided: {} could not be reread ({error}); reload it before retrying",
                    snapshot.path.display()
                ))
            })?;
            if current.bytes != snapshot.bytes {
                return Err(StateError::new(format!(
                    "state mutation refused for run {run_id}: a ledger the guards read changed while the step was being decided: {}; reload it before retrying",
                    snapshot.path.display()
                )));
            }
        }
        Ok(())
    }

    /// Instantiate a Run from the canonical, human-authored Machine Class.
    ///
    /// FDC-004: start mints a run id in the single namespace and creates
    /// `.ratmac/runs/<id>/` with its durable State File and Run evidence.
    /// FDC-003: the `verdict.toml` live slot is absent when empty; the
    /// `spawn-ledger` path remains reserved by name only for machine
    /// composition. No flat `.ratmac/state.toml` is written.
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
        let engine_root = self.engine_root()?.to_path_buf();
        // Do the content reads before taking the mutation domain. The cheap
        // flat-residue fact is checked again after acquisition below.
        Self::refuse_flat_residue(&root)?;
        self.machine = Self::graph_of(&Self::load_class(&root)?);
        let phase = self.initial_phase()?;
        let runbook_pin = Self::runbook_sha256(&root)?;
        let goal_baseline = crate::goal::revision(&root);
        // FDC-002: a Run beginning in a terminal Phase — no ordinary outgoing
        // edge — is complete from its first State File. The Engine writes the
        // terminal fact; no agent claim participates.
        let initial_status = if self.machine.has_ordinary_outgoing(phase.as_str()) {
            Status::Planned
        } else {
            Status::Passed
        };
        let engine_identity = crate::pin::engine_identity();
        let root_lock = RootLock::acquire(&engine_root)?;
        // Recheck this inexpensive existence fact after the final mutation
        // lock, before minting changes the roster.
        Self::refuse_flat_residue_at(&root, &engine_root)?;
        // FDC-006/ENS-004: no active-Run cap. Every allocation advances the
        // durable high-water record while this root lock is held, then creates
        // an independently addressed member of .ratmac/runs/.
        let minted = Self::mint_run(
            &root_lock,
            &engine_root,
            phase.as_str(),
            initial_status,
            &runbook_pin,
            &goal_baseline,
            &engine_identity,
        )?;
        let run_id = minted.id.clone();
        let store = StateStore::for_engine_root(&engine_root, &run_id);
        self.store = Some(store);
        self.run_id = Some(run_id.clone());
        self.child_class = None;
        Ok(Run::new(phase, initial_status).with_artifacts(&root, &run_id))
    }

    /// Reserve the next durable id, then create its directory, State File,
    /// evidence, and reserved spawn-ledger path. Used by `start` for the
    /// project machine and by `spawn`/`respawn` for children and successors.
    /// A later creation failure removes the half-made directory but
    /// deliberately leaves its reserved ordinal.
    fn mint_run(
        root_lock: &RootLock,
        engine_root: &Path,
        phase: &str,
        status: Status,
        runbook_pin: &str,
        goal_baseline: &Option<String>,
        engine_identity: &Option<crate::pin::Identity>,
    ) -> Result<MintedRun, StateError> {
        // A new Engine root receives its empty shared history under the root
        // domain. This is deliberately not restored on a later mint failure:
        // it contains no Run claim and another caller may legitimately use it.
        root_lock.ensure_current()?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(engine_root.join("log.md"))
            .map_err(|error| StateError::new(format!("open log.md: {error}")))?;
        root_lock.ensure_current()?;
        let run_id = crate::mint::next(engine_root)?;
        let runs_dir = Self::runs_dir_at(engine_root);
        root_lock.ensure_current()?;
        fs::create_dir_all(&runs_dir)
            .map_err(|error| StateError::new(format!("create .ratmac/runs: {error}")))?;
        let run_dir = runs_dir.join(&run_id);
        root_lock.ensure_current()?;
        fs::create_dir(&run_dir)
            .map_err(|error| StateError::new(format!("create run directory {run_id}: {error}")))?;

        // The directory is not addressable until State File creation below.
        // Claim its Run lock now, while root is still held, and retain it past
        // minting for a caller's ledger transaction. A new id should have no
        // ordinary contender; do not wait on the unlikely foreign claim while
        // holding root.
        let run_lock = match RunLock::try_acquire(engine_root, &run_id) {
            Ok(Some(lock)) => lock,
            Ok(None) => {
                let error = StateError::new(format!(
                    "mint cannot claim fresh Run lock {} without waiting while root is held",
                    crate::lock::run_path(engine_root, &run_id).display()
                ));
                return Err(Self::cleanup_incomplete_mint(
                    root_lock, None, &run_dir, error,
                ));
            }
            Err(error) => {
                return Err(Self::cleanup_incomplete_mint(
                    root_lock, None, &run_dir, error,
                ));
            }
        };

        let state = RunState {
            phase: phase.to_string(),
            status,
            goal_revision: String::new(),
            input_revision: String::new(),
            output_revision: String::new(),
            active_refs: Vec::new(),
            blocker: String::new(),
        };
        let store = StateStore::for_engine_root(engine_root, &run_id);
        let create_result = (|| -> Result<MintedRunSnapshot, StateError> {
            root_lock.ensure_current()?;
            run_lock.ensure_current()?;
            store.write(&state)?;

            // ETB-001: the identity was hashed before the root mutation
            // domain; only writing it is performed under the short lock.
            let mut evidence = crate::pin::Evidence::load(&run_dir);
            if let Some(identity) = engine_identity {
                evidence.set_engine(identity.clone());
            }
            evidence.goal_baseline = goal_baseline.clone();
            evidence.goal_frozen = None;
            evidence.runbook_sha256 = Some(runbook_pin.to_owned());
            root_lock.ensure_current()?;
            run_lock.ensure_current()?;
            evidence
                .write(&run_dir)
                .map_err(|error| StateError::new(format!("write evidence.toml: {error}")))?;

            // FDC-003/FDC-004: an empty Verdict slot is absence. Only the
            // per-run spawn-ledger path remains reserved by name here.
            root_lock.ensure_current()?;
            run_lock.ensure_current()?;
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(run_dir.join("spawn-ledger"))
                .map_err(|error| {
                    StateError::new(format!("reserve spawn-ledger under {run_id}: {error}"))
                })?;
            root_lock.ensure_current()?;
            run_lock.ensure_current()?;
            Self::snapshot_minted_run(&run_dir)
        })();
        match create_result {
            Ok(snapshot) => Ok(MintedRun {
                id: run_id,
                run_lock,
                snapshot,
            }),
            Err(error) => Err(Self::cleanup_incomplete_mint(
                root_lock,
                Some(&run_lock),
                &run_dir,
                error,
            )),
        }
    }

    fn cleanup_incomplete_mint(
        root_lock: &RootLock,
        run_lock: Option<&RunLock>,
        run_dir: &Path,
        error: StateError,
    ) -> StateError {
        // A minted ordinal remains durable, but an incomplete Run must not
        // remain on the roster. Once a Run lock exists, require it still names
        // this mutation before deleting anything.
        let cleanup = (|| {
            root_lock.ensure_current()?;
            if let Some(run_lock) = run_lock {
                run_lock.ensure_current()?;
            }
            fs::remove_dir_all(run_dir).map_err(|cleanup_error| {
                StateError::new(format!(
                    "remove incomplete run directory {}: {cleanup_error}",
                    run_dir.display()
                ))
            })
        })();
        match cleanup {
            Ok(()) => error,
            Err(cleanup_error) => {
                StateError::new(format!("{error}; cleanup incomplete: {cleanup_error}"))
            }
        }
    }

    fn snapshot_minted_run(run_dir: &Path) -> Result<MintedRunSnapshot, StateError> {
        let state_bytes = fs::read(run_dir.join("state.toml"))
            .map_err(|error| StateError::new(format!("snapshot minted State File: {error}")))?;
        let evidence_bytes = fs::read(run_dir.join(crate::pin::EVIDENCE_FILE))
            .map_err(|error| StateError::new(format!("snapshot minted evidence.toml: {error}")))?;
        let spawn_ledger_bytes = fs::read(run_dir.join("spawn-ledger"))
            .map_err(|error| StateError::new(format!("snapshot minted spawn-ledger: {error}")))?;
        Ok(MintedRunSnapshot {
            state_bytes,
            evidence_bytes,
            spawn_ledger_bytes,
        })
    }

    fn ensure_minted_run_matches(
        run_dir: &Path,
        snapshot: &MintedRunSnapshot,
        operation: &str,
    ) -> Result<(), StateError> {
        let metadata = fs::symlink_metadata(run_dir).map_err(|error| {
            StateError::new(format!(
                "{operation} refused: cannot inspect minted Run directory {}: {error}",
                run_dir.display()
            ))
        })?;
        if !metadata.file_type().is_dir() {
            return Err(StateError::new(format!(
                "{operation} refused: minted Run directory {} was replaced",
                run_dir.display()
            )));
        }

        let mut seen = BTreeMap::new();
        let entries = fs::read_dir(run_dir).map_err(|error| {
            StateError::new(format!(
                "{operation} refused: cannot inspect minted Run directory {}: {error}",
                run_dir.display()
            ))
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                StateError::new(format!(
                    "{operation} refused: cannot inspect a minted Run entry: {error}"
                ))
            })?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let expected = match name.as_str() {
                "state.toml" => snapshot.state_bytes.as_slice(),
                crate::pin::EVIDENCE_FILE => snapshot.evidence_bytes.as_slice(),
                "spawn-ledger" => snapshot.spawn_ledger_bytes.as_slice(),
                _ => {
                    return Err(StateError::new(format!(
                        "{operation} refused: minted Run directory {} changed (unexpected entry {name:?})",
                        run_dir.display()
                    )))
                }
            };
            let file_type = entry.file_type().map_err(|error| {
                StateError::new(format!(
                    "{operation} refused: cannot inspect minted Run entry {}: {error}",
                    entry.path().display()
                ))
            })?;
            if !file_type.is_file() {
                return Err(StateError::new(format!(
                    "{operation} refused: minted Run entry {} is no longer a regular file",
                    entry.path().display()
                )));
            }
            let observed = fs::read(entry.path()).map_err(|error| {
                StateError::new(format!(
                    "{operation} refused: cannot read minted Run entry {}: {error}",
                    entry.path().display()
                ))
            })?;
            if observed != expected {
                return Err(StateError::new(format!(
                    "{operation} refused: minted Run entry {} changed; it was not removed",
                    entry.path().display()
                )));
            }
            seen.insert(name, ());
        }
        for name in ["state.toml", crate::pin::EVIDENCE_FILE, "spawn-ledger"] {
            if !seen.contains_key(name) {
                return Err(StateError::new(format!(
                    "{operation} refused: minted Run entry {} is missing; it was not removed",
                    run_dir.join(name).display()
                )));
            }
        }
        Ok(())
    }

    fn rollback_minted_run_while_locked(
        root_lock: &RootLock,
        run_lock: &RunLock,
        engine_root: &Path,
        run_id: &str,
        snapshot: &MintedRunSnapshot,
        operation: &str,
    ) -> Result<(), StateError> {
        let run_dir = Self::runs_dir_at(engine_root).join(run_id);
        root_lock.ensure_current()?;
        run_lock.ensure_current()?;
        Self::ensure_minted_run_matches(&run_dir, snapshot, operation)?;
        // Recheck both claims and exact bytes immediately before deletion.
        root_lock.ensure_current()?;
        run_lock.ensure_current()?;
        Self::ensure_minted_run_matches(&run_dir, snapshot, operation)?;
        fs::remove_dir_all(&run_dir).map_err(|error| {
            StateError::new(format!(
                "{operation} cannot remove minted Run directory {}: {error}",
                run_dir.display()
            ))
        })
    }

    /// FDC-007: `rtm spawn` is ordinary checked motion - no confirmation
    /// phrase. Legal only while the addressed parent occupies the spawning
    /// Phase and only for a spawn that Phase declares. The child is minted as
    /// an ordinary flat top-level Run in the single run-id namespace - same
    /// State File, evidence, terminal facts, and reserved spawn-ledger path;
    /// the ledger's written entry is FDC-011, a later increment.
    /// The addressed spawn boundary (FDC-012 ordering). A malformed or
    /// never-recorded id refuses for the ordinary addressing reason; an id
    /// recorded as a child in any spawn ledger refuses naming the one-level
    /// cap - checked before the retired-run admission check, so an abandoned
    /// child and a superseded record refuse by the cap's name, exactly like
    /// a live child. Only then is the parent opened and the spawn attempted.
    pub fn spawn_to(
        root: impl AsRef<Path>,
        parent_id: &str,
        spawn_name: &str,
        bindings: &BTreeMap<String, String>,
    ) -> Result<String, StateError> {
        Self::spawn_to_with_workspace(root, parent_id, spawn_name, bindings, None)
    }

    /// Spawn with an optional workspace spelling from the invocation. Keeping
    /// the original `spawn_to` entry point preserves callers that inherit the
    /// parent workspace by default.
    pub fn spawn_to_with_workspace(
        root: impl AsRef<Path>,
        parent_id: &str,
        spawn_name: &str,
        bindings: &BTreeMap<String, String>,
        workspace: Option<&Path>,
    ) -> Result<String, StateError> {
        let root = root.as_ref();
        Self::validate_run_address(root, parent_id)?;
        if crate::ledger::is_recorded_child(&Self::runs_dir(root), parent_id)
            .map_err(|error| StateError::new(format!("spawn cap check refused: {error}")))?
        {
            return Err(StateError::new(format!(
                "spawn refused: run {parent_id} is a ledger-recorded child; \
composition is capped at one level (FDC-012)"
            )));
        }
        let mut scheduler = Self::open_run(root, parent_id)?;
        scheduler.spawn_with_bindings_at_workspace(spawn_name, bindings, workspace)
    }

    pub fn spawn(&mut self, spawn_name: &str) -> Result<String, StateError> {
        self.spawn_with_bindings(spawn_name, &BTreeMap::new())
    }

    /// FDC-011: spawn records what it makes. The ledger entry - child id,
    /// class, binding values, revision at spawn, and canonical workspace -
    /// lands in the same turn that mints the child. A failed append rolls that
    /// child back only when a strict reread proves no entry landed; uncertain
    /// bytes leave both paths intact for recovery rather than destroying
    /// recorded state.
    pub fn spawn_with_bindings(
        &mut self,
        spawn_name: &str,
        bindings: &BTreeMap<String, String>,
    ) -> Result<String, StateError> {
        self.spawn_with_bindings_at_workspace(spawn_name, bindings, None)
    }

    fn spawn_with_bindings_at_workspace(
        &mut self,
        spawn_name: &str,
        bindings: &BTreeMap<String, String>,
        workspace: Option<&Path>,
    ) -> Result<String, StateError> {
        let parent_workspace = self
            .root
            .as_ref()
            .ok_or_else(|| StateError::new("spawn requires Scheduler::open_run"))?
            .clone();
        let invoking_root = self.invoking_root()?.to_path_buf();
        let engine_root = self.engine_root()?.to_path_buf();
        let child_workspace = match workspace {
            Some(workspace) => {
                Self::canonical_spawn_workspace(&invoking_root, &engine_root, workspace)?
            }
            None => {
                Self::canonical_spawn_workspace(&parent_workspace, &engine_root, &parent_workspace)?
            }
        };
        let parent_id = self.run_id.clone().ok_or_else(|| {
            StateError::new("spawn requires an addressed parent: open one with Scheduler::open_run")
        })?;
        let run_dir = self.run_dir()?;
        let state_path = run_dir.join("state.toml");

        // Slow content and Git reads never belong to the shared mutation
        // scope. The final pair only rechecks the facts this plan relies on.
        let class = Self::load_class(&invoking_root)?;
        let runbook_pin = Self::runbook_sha256(&invoking_root)?;
        let goal_baseline = crate::goal::revision(&parent_workspace);
        let spawned_at = Self::revision_at(&invoking_root);
        let engine_identity = crate::pin::engine_identity();
        self.machine = Self::graph_of(&class);

        // This is a read-only spawn plan. Do not take the parent Run lock
        // here: the later mint/ledger transaction needs both domains and must
        // acquire root before Run. Its final pair revalidates these facts.
        let (parent_state_bytes, child_class_name, child_phase, child_status) = {
            if crate::ledger::is_recorded_child(&Self::runs_dir_at(&engine_root), &parent_id)
                .map_err(|error| StateError::new(format!("spawn cap check refused: {error}")))?
            {
                return Err(StateError::new(format!(
                    "spawn refused: run {parent_id} is a ledger-recorded child; \
composition is capped at one level (FDC-012)"
                )));
            }
            let parent_state_bytes = fs::read(&state_path)
                .map_err(|error| StateError::new(format!("read State File: {error}")))?;
            let state = self.load_state_unlocked()?;
            if state.status == Status::Passed {
                return Err(StateError::new(format!(
                    "spawn refused: run {parent_id} is terminal (status passed): no motion may proceed"
                )));
            }
            if state.status == Status::Blocked {
                return Err(StateError::new(format!(
                    "spawn refused: run {parent_id} is held (status blocked): the blocked route admits no spawn"
                )));
            }
            let definition = class.phases().get(&state.phase).ok_or_else(|| {
                StateError::new(format!(
                    "State File phase {:?} is undeclared in ratmac.toml",
                    state.phase
                ))
            })?;
            let declared = definition.spawns();
            if declared.is_empty() {
                return Err(StateError::new(format!(
                    "spawn refused: phase {:?} declares no spawns; run {parent_id} is outside a spawning Phase",
                    state.phase
                )));
            }
            let declaration = declared
                .iter()
                .find(|spawn| spawn.name() == spawn_name)
                .ok_or_else(|| {
                    let names = declared
                        .iter()
                        .map(|spawn| spawn.name())
                        .collect::<Vec<_>>()
                        .join(", ");
                    StateError::new(format!(
                        "spawn refused: {spawn_name:?} is not declared in phase {:?}; declared spawns: {names}",
                        state.phase
                    ))
                })?;
            for name in bindings.keys() {
                if !declaration.bind().iter().any(|declared| declared == name) {
                    let declared = declaration.bind().join(", ");
                    return Err(StateError::new(format!(
                        "spawn refused: binding {name:?} is not declared for spawn {spawn_name:?}; declared bindings: {declared}"
                    )));
                }
            }
            let child_class = class.classes().get(declaration.class()).ok_or_else(|| {
                StateError::new(format!(
                    "spawn refused: class {:?} is not declared in ratmac.toml",
                    declaration.class()
                ))
            })?;
            let child_machine = MachineGraph::new(
                child_class
                    .phases()
                    .keys()
                    .map(Phase::new)
                    .collect::<Vec<_>>(),
                child_class.transitions().to_vec(),
            );
            let child_phase = Self::initial_phase_of(&child_machine)?;
            let child_status = if child_machine.has_ordinary_outgoing(child_phase.as_str()) {
                Status::Planned
            } else {
                Status::Passed
            };
            (
                parent_state_bytes,
                declaration.class().to_owned(),
                child_phase,
                child_status,
            )
        };

        // Root is acquired first only for the short mint/ledger transaction.
        let (root_lock, run_lock) = crate::lock::acquire_root_then_run(&engine_root, &parent_id)?;
        run_lock.ensure_current()?;
        Self::validate_run_address_at(&engine_root, &parent_id)?;
        Self::verify_runbook_pin_hash(&run_dir, &runbook_pin)?;
        let current_state_bytes = fs::read(&state_path)
            .map_err(|error| StateError::new(format!("read State File before spawn: {error}")))?;
        if current_state_bytes != parent_state_bytes {
            return Err(StateError::new(format!(
                "spawn refused: run {parent_id} changed while the spawn plan was prepared; reload it before retrying"
            )));
        }
        if crate::ledger::is_recorded_child(&Self::runs_dir_at(&engine_root), &parent_id)
            .map_err(|error| StateError::new(format!("spawn cap check refused: {error}")))?
        {
            return Err(StateError::new(format!(
                "spawn refused: run {parent_id} is a ledger-recorded child; \
composition is capped at one level (FDC-012)"
            )));
        }

        let MintedRun {
            id: child,
            run_lock: child_run_lock,
            snapshot: child_snapshot,
        } = Self::mint_run(
            &root_lock,
            &engine_root,
            child_phase.as_str(),
            child_status,
            &runbook_pin,
            &goal_baseline,
            &engine_identity,
        )?;
        let entry = LedgerEntry {
            id: child.clone(),
            class: child_class_name,
            bind: bindings.clone(),
            spawned_at,
            workspace: Some(child_workspace.to_string_lossy().into_owned()),
            abandoned: false,
            supersedes: None,
        };
        run_lock.ensure_current()?;
        root_lock.ensure_current()?;
        child_run_lock.ensure_current()?;
        let ledger_path = run_dir.join("spawn-ledger");
        match Self::append_ledger_entry(&ledger_path, &entry) {
            LedgerAppendAftermath::Appended => {}
            LedgerAppendAftermath::RecordedAfterError(error) => {
                eprintln!(
                    "warning: spawn ledger append for {child} reported {error}, but {} records the complete entry; treating the child as committed",
                    ledger_path.display()
                );
            }
            LedgerAppendAftermath::AbsentAfterError(error) => {
                let cleanup = Self::rollback_minted_run_while_locked(
                    &root_lock,
                    &child_run_lock,
                    &engine_root,
                    &child,
                    &child_snapshot,
                    &format!("spawn rollback for unrecorded child {child}"),
                );
                return match cleanup {
                    Ok(()) => Err(StateError::new(format!(
                        "spawn cannot record the ledger entry for {child}: {error}; the child mint was rolled back"
                    ))),
                    Err(cleanup_error) => Err(StateError::new(format!(
                        "spawn cannot record the ledger entry for {child}: {error}; \
the child mint cleanup is incomplete: {cleanup_error}"
                    ))),
                };
            }
            LedgerAppendAftermath::Indeterminate(detail) => {
                return Err(StateError::new(format!(
                    "spawn cannot determine whether its ledger entry committed for {child}: {detail}; \
the ledger {} and minted child {} were left in place; inspect both paths before removing anything",
                    ledger_path.display(),
                    Self::runs_dir_at(&engine_root).join(&child).display()
                )));
            }
        }
        Ok(child)
    }

    /// FDC-007/FDC-006: human-confirmed supersession. Refuses without a
    /// confirmation phrase naming the superseded run id - typed at
    /// invocation, never read from a file. With the exact phrase it mints a
    /// fresh successor id (never overwriting: the superseded record keeps its
    /// address) and retires the superseded Run by the abandon path. The
    /// successor is start-shaped this increment; class-faithful
    /// re-instantiation needs the ledger's recorded class and bindings
    /// (FDC-011, a later increment).
    pub fn respawn(root: impl AsRef<Path>, request: &RespawnRequest) -> Result<String, StateError> {
        let roots = crate::root::resolve(root);
        let root = roots.invoking_checkout_root().to_path_buf();
        let engine_root = roots.engine_root().to_path_buf();
        let roster = Self::run_roster_at(&engine_root);
        let roster_line = if roster.is_empty() {
            "none".to_owned()
        } else {
            roster.join(", ")
        };
        let superseded = match request
            .run
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        {
            Some(id) => id.to_owned(),
            None => {
                return Err(StateError::new(format!(
                    "respawn requires --run <id>; runs: {roster_line}"
                )))
            }
        };
        let required = format!("respawn {superseded}");
        match request.confirmation.as_deref() {
            None => {
                return Err(StateError::new(format!(
                    "respawn is unconfirmed: a human must type --confirm {required:?}"
                )))
            }
            Some(phrase) if phrase != required => {
                return Err(StateError::new(format!(
                    "respawn is unconfirmed: confirmation {phrase:?} does not match the required phrase {required:?}"
                )))
            }
            Some(_) => {}
        }
        if !roster.iter().any(|entry| entry == &superseded) {
            return Err(StateError::new(format!(
                "respawn names no run: {superseded:?} is not on the roster; runs: {roster_line}"
            )));
        }
        if !Self::runs_dir_at(&engine_root)
            .join(&superseded)
            .join("state.toml")
            .is_file()
        {
            return Err(StateError::new(format!(
                "run {superseded} is already terminal: its admission state is retired; nothing to supersede"
            )));
        }
        let recorded = Self::ledger_record_of_at(&engine_root, &superseded)?;
        if let Some((ledger_path, entry)) = recorded.as_ref() {
            // A successor cannot repair or replace a missing legacy binding:
            // it inherits exactly the durable workspace of the child it
            // supersedes.
            let _ = Self::workspace_from_ledger_entry(&engine_root, ledger_path, entry)?;
        }
        let class = Self::load_class(&root)?;
        let runbook_pin = Self::runbook_sha256(&root)?;
        let goal_baseline = crate::goal::revision(&root);
        let successor_spawned_at = Self::revision_at(&root);
        let engine_identity = crate::pin::engine_identity();
        let (phase, status) = match recorded.as_ref() {
            Some((_, entry)) => {
                let child_class = class.classes().get(&entry.class).ok_or_else(|| {
                    StateError::new(format!(
                        "respawn refused: recorded class {:?} is not declared in ratmac.toml",
                        entry.class
                    ))
                })?;
                let child_machine = MachineGraph::new(
                    child_class
                        .phases()
                        .keys()
                        .map(Phase::new)
                        .collect::<Vec<_>>(),
                    child_class.transitions().to_vec(),
                );
                let phase = Self::initial_phase_of(&child_machine)?;
                let status = if child_machine.has_ordinary_outgoing(phase.as_str()) {
                    Status::Planned
                } else {
                    Status::Passed
                };
                (phase, status)
            }
            None => {
                let machine = Self::graph_of(&class);
                let phase = Self::initial_phase_of(&machine)?;
                let status = if machine.has_ordinary_outgoing(phase.as_str()) {
                    Status::Planned
                } else {
                    Status::Passed
                };
                (phase, status)
            }
        };

        // Minting is root-first and brief. It intentionally precedes terminal
        // retirement so a failure leaves the existing admitted Run untouched.
        let minted_successor = {
            let root_lock = RootLock::acquire(&engine_root)?;
            Self::refuse_flat_residue_at(&root, &engine_root)?;
            Self::mint_run(
                &root_lock,
                &engine_root,
                phase.as_str(),
                status,
                &runbook_pin,
                &goal_baseline,
                &engine_identity,
            )?
        };
        let MintedRun {
            id: successor,
            run_lock: successor_run_lock,
            snapshot: successor_snapshot,
        } = minted_successor;
        // The successor is no longer part of the short mint domain. Any
        // compensating delete later reacquires root then this Run and proves
        // these exact minted artifacts first.
        drop(successor_run_lock);

        let abandon_request = crate::abandon::AbandonRequest {
            confirmation: Some(crate::abandon::required_phrase(&root, Some(&superseded))),
            run: Some(superseded.clone()),
        };
        let retired = crate::abandon::plan_abandon(&root, &abandon_request)
            .and_then(|plan| crate::abandon::apply_abandon(&root, &plan));
        if let Err(refusal) = retired {
            return match Self::rollback_minted_successor(
                &engine_root,
                &successor,
                &successor_snapshot,
            ) {
                Ok(()) => Err(StateError::new(format!(
                    "respawn interrupted retiring {superseded}: {refusal}; the successor mint was rolled back"
                ))),
                Err(rollback) => Err(StateError::new(format!(
                    "respawn interrupted retiring {superseded}: {refusal}; \
the successor mint rollback is incomplete: {rollback}"
                ))),
            };
        }
        if let Some((ledger_path, entry)) = recorded {
            let successor_entry = LedgerEntry {
                id: successor.clone(),
                class: entry.class.clone(),
                bind: entry.bind.clone(),
                spawned_at: successor_spawned_at,
                workspace: entry.workspace.clone(),
                abandoned: false,
                supersedes: Some(superseded.clone()),
            };
            // Keep the root claim that classified a failed append through any
            // compensating delete. Releasing it between "no entry landed" and
            // the delete would let another root-domain writer record this
            // successor in that ledger first.
            let (root_lock, successor_run_lock) =
                crate::lock::acquire_root_then_run(&engine_root, &successor)?;
            root_lock.ensure_current()?;
            successor_run_lock.ensure_current()?;
            let aftermath = Self::append_ledger_entry(&ledger_path, &successor_entry);
            match aftermath {
                LedgerAppendAftermath::Appended => {}
                LedgerAppendAftermath::RecordedAfterError(error) => {
                    eprintln!(
                        "warning: successor ledger append for {successor} reported {error}, but {} records the complete entry; treating the successor as committed",
                        ledger_path.display()
                    );
                }
                LedgerAppendAftermath::AbsentAfterError(error) => {
                    let cleanup = Self::rollback_minted_run_while_locked(
                        &root_lock,
                        &successor_run_lock,
                        &engine_root,
                        &successor,
                        &successor_snapshot,
                        &format!("respawn rollback for unrecorded successor {successor}"),
                    );
                    return match cleanup {
                        Ok(()) => Err(StateError::new(format!(
                            "respawn cannot record the successor entry for {successor}: {error}; the successor mint was rolled back"
                        ))),
                        Err(rollback) => Err(StateError::new(format!(
                            "respawn cannot record the successor entry for {successor}: {error}; \
the successor mint rollback is incomplete: {rollback}"
                        ))),
                    };
                }
                LedgerAppendAftermath::Indeterminate(detail) => {
                    return Err(StateError::new(format!(
                        "respawn cannot determine whether its successor entry committed for {successor}: {detail}; \
the ledger {} and minted successor {} were left in place; inspect both paths before removing anything",
                        ledger_path.display(),
                        Self::runs_dir_at(&engine_root).join(&successor).display()
                    )));
                }
            }
        }
        Ok(successor)
    }

    /// Roll back only this invocation's freshly minted successor. It proves
    /// every direct artifact still matches the bytes minted above before the
    /// ordered root-then-Run pair may remove the directory.
    fn rollback_minted_successor(
        engine_root: &Path,
        successor: &str,
        snapshot: &MintedRunSnapshot,
    ) -> Result<(), StateError> {
        let (root_lock, run_lock) = crate::lock::acquire_root_then_run(engine_root, successor)?;
        Self::rollback_minted_run_while_locked(
            &root_lock,
            &run_lock,
            engine_root,
            successor,
            snapshot,
            &format!("respawn rollback for successor {successor}"),
        )
    }

    /// Evaluate supported guards and apply a transition only after every
    /// guard passes. One addressed Run lock serializes the entire motion,
    /// including its append-only transition record; `step` never takes the
    /// root lock.
    pub fn step(&mut self, request: StepRequest) -> Result<StepOutcome, StateError> {
        let _claim = request.claim;
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| StateError::new("step requires Scheduler::open"))?
            .clone();
        let invoking_root = self.invoking_root()?.to_path_buf();
        let engine_root = self.engine_root()?.to_path_buf();
        let run_id = self
            .run_id
            .clone()
            .ok_or_else(|| StateError::new("step requires an addressed Run"))?;
        let run_dir = self.run_dir()?;
        let state_path = run_dir.join("state.toml");

        // Hash and parse before either lock. A final pin comparison happens
        // under the Run lock, but neither comparison can invoke Git or root
        // resolution while a lock is held.
        let class = Self::load_class(&invoking_root)?;
        let runbook_pin = Self::runbook_sha256(&invoking_root)?;
        self.machine = Self::graph_of(&class);

        // ENS-005: one Run's long guard evaluation is serialized only by its
        // own lock. No root-domain lock is acquired during this motion.
        let run_lock = RunLock::acquire(&engine_root, &run_id)?;
        run_lock.ensure_current()?;
        let (
            guarded_state_bytes,
            next,
            from,
            to,
            consumes_verdict,
            frozen_evidence,
            guarded_ledgers,
        ) = {
            Self::verify_runbook_pin_hash(&run_dir, &runbook_pin)?;
            let guarded_state_bytes = fs::read(&state_path)
                .map_err(|error| StateError::new(format!("read State File: {error}")))?;
            let state = self.load_state_unlocked()?;
            let state_phase = state.phase.clone();
            let (definition, scope_machine) =
                Self::resolve_phase_scope(&class, &state_phase, self.child_class.as_deref())?;
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

            let (mut failures, guarded_ledgers) = self.guard_failures(definition)?;
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
            let Some(transition) =
                Self::route_for(&scope_machine, &from, transition_input.as_deref())
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
            let to = transition.to().clone();
            let frozen_revision =
                if transition.freezes_goal() {
                    Some(crate::goal::revision(&root).ok_or_else(|| {
                        StateError::new("cannot freeze goal: .arca/goal/ is absent")
                    })?)
                } else {
                    None
                };

            // All read/open prerequisites happen before a verdict can be
            // archived. Opening project history remains under this Run lock:
            // it is not a root-domain roster or ledger mutation.
            let mut next = state;
            next.phase = to.to_string();
            if !scope_machine.has_ordinary_outgoing(to.as_str()) {
                next.status = Status::Passed;
            }
            let frozen_evidence = frozen_revision.map(|frozen| {
                let mut evidence = evidence;
                evidence.goal_frozen = Some(frozen.clone());
                next.goal_revision = frozen;
                evidence
            });
            (
                guarded_state_bytes,
                next,
                from,
                to,
                consumes_verdict,
                frozen_evidence,
                guarded_ledgers,
            )
        };

        // The same Run guard remains held from guard evaluation through the
        // durable motion. Compare the exact State File bytes guards observed
        // as a defense against an out-of-band writer.
        run_lock.ensure_current()?;
        let current_state_bytes = fs::read(&state_path)
            .map_err(|error| StateError::new(format!("read State File before commit: {error}")))?;
        if current_state_bytes != guarded_state_bytes {
            return Err(StateError::new(format!(
                "state mutation refused for run {run_id}: State File changed while guards ran; reload it before retrying"
            )));
        }
        Self::verify_runbook_pin_hash(&run_dir, &runbook_pin)?;

        // Open the pre-existing append target before committing state: an
        // unusable or missing history file refuses without a half-motion.
        let mut log = OpenOptions::new()
            .append(true)
            .open(engine_root.join("log.md"))
            .map_err(|error| StateError::new(format!("open log.md: {error}")))?;
        Self::ensure_ledger_snapshots_current(&run_id, &guarded_ledgers)?;
        if consumes_verdict {
            inject_step_fault("before-verdict-archive")?;
            run_lock.ensure_current()?;
            crate::verdict::archive_live(&run_dir)?;
            inject_step_fault("before-state-replace")?;
        }
        if let Some(frozen_evidence) = frozen_evidence {
            run_lock.ensure_current()?;
            frozen_evidence.write(&run_dir).map_err(|error| {
                StateError::new(format!(
                    "freeze goal revision: write evidence.toml: {error}"
                ))
            })?;
        }
        run_lock.ensure_current()?;
        self.store()?.write(&next)?;
        if consumes_verdict {
            inject_step_fault("after-state-replace")?;
        }

        // The addressed Run lock serializes this transition through its
        // append-only history record. Project history is not a root-domain
        // roster or ledger mutation, so unrelated root work cannot delay it.
        let entry = format!("\n- Transition: {from} -> {to}\n");
        if let Err(append_error) = append_history_once(&mut log, entry.as_bytes()) {
            let durable = if consumes_verdict {
                format!(
                    "the verdict was consumed and the Run advanced {from} -> {to}; \
                     no history rewrite was attempted"
                )
            } else {
                format!("the Run advanced {from} -> {to}; no history rewrite was attempted")
            };
            return Err(StateError::new(format!(
                "append log.md failed: {append_error}; {durable}"
            )));
        }
        Ok(StepOutcome::Advanced { from, to })
    }

    /// TRP-001, TRP-004: evaluate the Phase's retained guards, in declaration
    /// order, from the typed class - no second walk over runbook TOML.
    fn guard_failures(
        &self,
        definition: &PhaseDefinition,
    ) -> Result<(Vec<GuardFailure>, Vec<LedgerSnapshot>), StateError> {
        let root = match self.root.as_ref() {
            Some(root) => root,
            None => {
                return Ok((
                    vec![guard_failure(
                        "scheduler",
                        "",
                        "step requires Scheduler::open",
                        "opened project",
                    )],
                    Vec::new(),
                ))
            }
        };
        let engine_root = self.engine_root()?;

        let mut failures = Vec::new();
        let mut ledger_snapshots = Vec::new();
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
                GuardKind::Join { min, .. } => self.evaluate_join(*min, &mut ledger_snapshots),
                GuardKind::RecordContract => self.evaluate_contract(
                    "record_contract",
                    crate::contract::gate_records(
                        root,
                        engine_root,
                        self.run_id.as_deref().unwrap_or_default(),
                    ),
                    "one residual per requirement citing the frozen revision, evidence behind every satisfied, one owning ticket per gap, acyclic dependencies, complete tickets",
                ),
            };
            if let Err(failure) = result {
                failures.push(failure);
            }
        }
        Ok((failures, ledger_snapshots))
    }

    /// FDC-009: the composition join. Until a spawn ledger records children
    /// (FDC-011, t-066), the honest verdict is that zero children are passed,
    /// so a join guard cannot be satisfied.
    fn evaluate_join(
        &self,
        min: Option<i64>,
        ledger_snapshots: &mut Vec<LedgerSnapshot>,
    ) -> Result<(), GuardFailure> {
        let required = min.unwrap_or(1);
        let expected = "every ledger-recorded live child passed, at least the declared min";
        let refuse = |observed: String| guard_failure("join", "spawn ledger", observed, expected);
        let run_dir = self
            .run_dir()
            .map_err(|error| refuse(format!("the addressed run is unresolved: {error}")))?;
        let engine_root = self
            .engine_root()
            .map_err(|error| refuse(format!("the Engine root is unresolved: {error}")))?;
        let ledger_path = run_dir.join("spawn-ledger");
        // Parse the same bytes retained for commit-time revalidation. A second
        // path read here could make the guard decide from one ledger version
        // while retaining another.
        let snapshot = Self::snapshot_ledger(&ledger_path)
            .map_err(|error| refuse(format!("cannot snapshot spawn ledger: {error}")))?;
        let entries = match snapshot.bytes.as_deref() {
            Some(bytes) => crate::ledger::parse_entries_bytes(&ledger_path, bytes),
            None => Ok(Vec::new()),
        }
        .map_err(|error| refuse(error.to_string()))?;
        ledger_snapshots.push(snapshot);
        let live: Vec<&crate::ledger::LedgerEntry> =
            entries.iter().filter(|entry| !entry.abandoned).collect();
        if live.is_empty() {
            return Err(refuse(format!(
                "no spawn ledger records a child Run; 0 of {required} required children are passed"
            )));
        }
        let mut missing = Vec::new();
        let mut unfinished = Vec::new();
        let mut passed: i64 = 0;
        for entry in &live {
            let state_path = Self::runs_dir_at(engine_root)
                .join(&entry.id)
                .join("state.toml");
            if !state_path.is_file() {
                missing.push(entry.id.as_str());
                continue;
            }
            let state = crate::state::StateStore::at(state_path)
                .load()
                .map_err(|error| {
                    refuse(format!(
                        "ledger child {} has an unreadable State File: {error}",
                        entry.id
                    ))
                })?;
            if state.status == Status::Passed {
                passed += 1;
            } else {
                unfinished.push(format!(
                    "{} is {} at {}",
                    entry.id, state.status, state.phase
                ));
            }
        }
        if !missing.is_empty() {
            return Err(refuse(format!(
                "ledger children missing on disk: {}; the expected set never silently shrinks",
                missing.join(", ")
            )));
        }
        if !unfinished.is_empty() {
            return Err(refuse(format!(
                "{passed} of {required} required children are passed; unfinished: {}",
                unfinished.join(", ")
            )));
        }
        if passed < required {
            return Err(refuse(format!(
                "{passed} of {required} required children are passed"
            )));
        }
        Ok(())
    }

    /// PGE-003: the P4 gate. Every planned test the ticket declares must
    /// resolve to a sensitivity receipt under the addressed Run's
    /// `.ratmac/evidence/<run-id>/`; prose, filenames, and status fields
    /// satisfy nothing.
    fn evaluate_sensitivity_receipts(&self, root: &Path, ticket: &str) -> Result<(), GuardFailure> {
        let engine_root = self.engine_root().map_err(|error| {
            guard_failure(
                "sensitivity_receipts",
                ticket,
                error.to_string(),
                "an addressed Run with a resolved Engine root",
            )
        })?;
        let run_id = self.run_id().ok_or_else(|| {
            guard_failure(
                "sensitivity_receipts",
                ticket,
                "no addressed Run",
                "an addressed Run",
            )
        })?;
        crate::receipt::gate_sensitivity(root, engine_root, run_id, ticket).map_err(|defects| {
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
        let engine_root = self.engine_root().map_err(|error| {
            guard_failure(
                "completion_gate",
                ticket,
                error.to_string(),
                "an addressed Run with a resolved Engine root",
            )
        })?;
        let run_id = self.run_id().ok_or_else(|| {
            guard_failure(
                "completion_gate",
                ticket,
                "no addressed Run",
                "an addressed Run",
            )
        })?;
        crate::completion::gate_completion(root, engine_root, run_id, ticket).map_err(|defects| {
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
        Self::initial_phase_of(&self.machine)
    }

    /// The unique Phase no ordinary transition enters. Shared by `start` for
    /// the project machine and by `spawn`/`respawn` for child machines.
    fn initial_phase_of(machine: &MachineGraph) -> Result<Phase, StateError> {
        let candidates = machine
            .phases()
            .filter(|phase| {
                // A blocked route points backwards by design; it never makes
                // its destination a non-initial Phase.
                !machine
                    .transitions()
                    .any(|transition| transition.to() == *phase && !transition.is_blocked_route())
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

    fn run_lock(&self) -> Result<RunLock, StateError> {
        let engine_root = self.engine_root()?;
        let run_id = self
            .run_id
            .as_deref()
            .ok_or_else(|| StateError::new("state mutation requires an addressed Run"))?;
        RunLock::acquire(engine_root, run_id)
    }

    fn store(&self) -> Result<&StateStore, StateError> {
        self.store.as_ref().ok_or_else(|| {
            StateError::new(
                "state operations require an addressed run: Scheduler::open_run or start",
            )
        })
    }

    /// Caller-provided state is a compare-and-write proposal, never an
    /// authority to overwrite a newer motion that completed before this Run
    /// lock was acquired.
    fn confirm_current_state(&self, proposed: &RunState) -> Result<(), StateError> {
        let current = self.store()?.load()?;
        if current != *proposed {
            let run_id = self.run_id.as_deref().unwrap_or("<unaddressed>");
            return Err(StateError::new(format!(
                "state mutation refused for run {run_id}: caller-provided state is stale; reload it before retrying"
            )));
        }
        Ok(())
    }

    /// Scheduler-owned initialization of a complete State File.
    pub fn initialize_state(&mut self, state: RunState) -> Result<(), StateError> {
        let run_lock = self.run_lock()?;
        self.confirm_current_state(&state)?;
        run_lock.ensure_current()?;
        self.store()?.write(&state)
    }

    /// Record the only state transition that may enter `blocked`.
    pub fn record_missing_prerequisite(
        &mut self,
        mut state: RunState,
        prerequisite: impl AsRef<str>,
    ) -> Result<(), StateError> {
        let run_lock = self.run_lock()?;
        self.confirm_current_state(&state)?;
        let prerequisite = prerequisite.as_ref();

        state.status = Status::Blocked;
        state.blocker = format!("missing entry prerequisite: {prerequisite}");
        run_lock.ensure_current()?;
        self.store()?.write(&state)
    }

    /// Read the addressed State File without serializing on either lock.
    pub fn load_state(&self) -> Result<RunState, StateError> {
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
        let invoking_root = self.invoking_root()?;
        // ENS-005 leaves status outside both new lock domains, but the
        // explicitly refused pre-split lock remains a global entry preflight.
        crate::lock::refuse_legacy(self.engine_root()?)?;
        let class = Self::load_class(invoking_root)?;
        // FDC-005: status reads the class too; an addressed run's recorded
        // pin is compared on every such read.
        if self.run_id.is_some() {
            Self::verify_runbook_pin(invoking_root, &self.run_dir()?)?;
        }
        let state = self.load_state_unlocked()?;
        // FDC-011/FDC-012: a child Run is reported from its own class's view.
        let (definition, _scope_machine) =
            Self::resolve_phase_scope(&class, &state.phase, self.child_class.as_deref())?;
        let pending_guards = Self::pending_guard_labels(definition);
        let phase_prompt = Self::render_phase_prompt(definition)?;
        Ok(StatusReport {
            state,
            pending_guards,
            phase_prompt,
        })
    }

    /// FDC-011/FDC-012: resolve the owning scope of a State File phase. A
    /// child is addressed by its durable ledger class, not by the first class
    /// whose phase happens to share the same name with a sibling.
    fn resolve_phase_scope<'a>(
        class: &'a MachineClass,
        state_phase: &str,
        child_class_name: Option<&str>,
    ) -> Result<(&'a PhaseDefinition, MachineGraph), StateError> {
        if let Some(child_class_name) = child_class_name {
            let child_class = class.classes().get(child_class_name).ok_or_else(|| {
                StateError::new(format!(
                    "recorded child class {child_class_name:?} is not declared in ratmac.toml"
                ))
            })?;
            let definition = child_class.phases().get(state_phase).ok_or_else(|| {
                StateError::new(format!(
                    "State File phase {state_phase:?} is undeclared in recorded child class \
                     {child_class_name:?}"
                ))
            })?;
            let machine = MachineGraph::new(
                child_class
                    .phases()
                    .keys()
                    .map(Phase::new)
                    .collect::<Vec<_>>(),
                child_class.transitions().to_vec(),
            );
            return Ok((definition, machine));
        }

        if let Some(definition) = class.phases().get(state_phase) {
            return Ok((definition, Self::graph_of(class)));
        }
        if let Some(child_class) = class
            .classes()
            .values()
            .find(|child| child.phases().contains_key(state_phase))
        {
            let definition = child_class
                .phases()
                .get(state_phase)
                .expect("the owning child class carries the phase definition");
            let machine = MachineGraph::new(
                child_class
                    .phases()
                    .keys()
                    .map(Phase::new)
                    .collect::<Vec<_>>(),
                child_class.transitions().to_vec(),
            );
            return Ok((definition, machine));
        }
        Err(StateError::new(format!(
            "State File phase {state_phase:?} is undeclared in ratmac.toml"
        )))
    }

    /// R-028: the Phase Prompt is the authored prose plus the generated list of
    /// this Phase's Exit Guards - rendered from the typed guards, so what the
    /// agent reads is what the Scheduler will evaluate.
    fn render_phase_prompt(definition: &PhaseDefinition) -> Result<PhasePrompt, StateError> {
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

    fn pending_guard_labels(definition: &PhaseDefinition) -> Vec<String> {
        definition
            .guards()
            .iter()
            .map(|guard| guard.name().to_owned())
            .collect()
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

/// Write one complete append-only history record without seeking, truncating,
/// or retrying a partial write. Retrying after a short append could duplicate
/// bytes once another writer has appended, so that case is an honest error.
fn append_history_once(file: &mut fs::File, entry: &[u8]) -> std::io::Result<()> {
    let written = file.write(entry)?;
    if written != entry.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::WriteZero,
            format!(
                "short append wrote {written} of {} history bytes without retry",
                entry.len()
            ),
        ));
    }
    file.sync_all()
}
