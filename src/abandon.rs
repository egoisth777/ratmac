//! PGE-007: safe human-confirmed Run abandonment.
//!
//! A Run can break in ways no Phase transition can fix. The honest exit is not
//! an agent deleting Scheduler-owned files - the schema forbids that - but a
//! human running
//!
//! ```text
//! rtm abandon --run <id> --confirm "abandon <project directory name>"
//! ```
//!
//! The Engine keeps no caller identity (ORS-001); it checks only that the
//! exact phrase was typed at invocation, never read from a file an agent can
//! write. On that phrase, and only then, `rtm` itself:
//!
//! 1. records a terminal abandoned event in the append-only history, naming
//!    the retired Run's Phase, status, and revisions;
//! 2. retires the admission state (`.ratmac/runs/<id>/state.toml`) so a fresh
//!    Run can start;
//! 3. retires the Run-scoped evidence (`.ratmac/runs/<id>/evidence.toml`) so
//!    the next Run records its own baseline and pins rather than inheriting
//!    them;
//! 4. retires the root lock (`.ratmac/locks/root.lock`) - retired through this
//!    path, never bypassed by a flag.
//!
//! Every check runs before the first mutation, so an unconfirmed request leaves
//! state, history, and locks byte-identical. An admitted retirement records
//! its terminal event first, then retires ledger marks and Run artifacts. If a
//! later retirement step fails, history is never rewritten: one compensating
//! append records that the terminal event was written but the Run was not
//! retired. Re-running the confirmed command then finishes the remaining job.
//!
//! Retirement is idempotent: a leftover lock with no admission state is
//! retired without appending a second terminal event, because the lock is
//! transient invocation machinery and its removal is not Run history.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Why an abandonment cannot proceed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbandonRefusal {
    pub reason: String,
}

impl fmt::Display for AbandonRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

fn refusal(reason: impl Into<String>) -> AbandonRefusal {
    AbandonRefusal {
        reason: reason.into(),
    }
}

/// The exact phrase a human must type to retire the addressed Run.
pub fn required_phrase(root: &Path, run: Option<&str>) -> String {
    // FDC-007: abandon-with-run-id demands a phrase naming that run id. Only
    // the unaddressed leftover-lock retirement - which touches no Run - keeps
    // the project-name phrase.
    match run.map(str::trim).filter(|id| !id.is_empty()) {
        Some(id) => format!("abandon {id}"),
        None => format!("abandon {}", project_name(root)),
    }
}

fn project_name(root: &Path) -> String {
    let named = |path: &Path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.is_empty() && name != "." && name != "..")
    };
    named(root)
        .or_else(|| fs::canonicalize(root).ok().as_deref().and_then(named))
        .unwrap_or_else(|| "this project".to_owned())
}

/// What a human asked for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbandonRequest {
    pub confirmation: Option<String>,
    /// The addressed run (FDC-004: `--run <id>`). Required whenever a live
    /// run is being retired; the leftover-lock-only retirement acts on no
    /// existing Run and needs no address.
    pub run: Option<String>,
}

/// The retirement `rtm` will perform, decided before anything is written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbandonPlan {
    /// The terminal event to append, present only when a Run is admitted.
    pub event: Option<String>,
    /// The retired Run's Phase, for the operator-facing report.
    pub phase: Option<String>,
    /// Scheduler-owned paths to retire, in order.
    pub retire: Vec<PathBuf>,
    /// The addressed run, when one is being retired.
    pub run: Option<String>,
    /// Spawn ledgers whose entry for the addressed run gets its abandoned
    /// mark flipped (FDC-011) - only that mark, inside the same transaction.
    pub annotate: Vec<PathBuf>,
    /// A lock pathname observed during planning. Apply claims it normally
    /// before its RAII drop may retire it; this is never a direct unlink.
    root_lock_present: bool,
    run_lock_present: bool,
}

/// Decide whether this project's Run may be retired. Writes nothing.
pub fn plan_abandon(root: &Path, request: &AbandonRequest) -> Result<AbandonPlan, AbandonRefusal> {
    let required = required_phrase(root, request.run.as_deref());
    match request.confirmation.as_deref() {
        None => {
            return Err(refusal(format!(
                "abandonment is unconfirmed: a human must type --confirm {required:?}"
            )))
        }
        Some(phrase) if phrase != required => {
            return Err(refusal(format!(
                "abandonment is unconfirmed: confirmation {phrase:?} does not match the required phrase {required:?}"
            )))
        }
        Some(_) => {}
    }

    // FDC-004: abandon acts on an existing Run through `--run <id>`.
    let engine_root = crate::root::resolve(root).engine_root().to_path_buf();
    let roster = run_roster_at(&engine_root)?;
    let roster_line = if roster.is_empty() {
        "none".to_owned()
    } else {
        roster.join(", ")
    };
    let live: Vec<&String> = roster
        .iter()
        .filter(|id| {
            crate::Scheduler::runs_dir(root)
                .join(id.as_str())
                .join("state.toml")
                .is_file()
        })
        .collect();
    let run_id = match request
        .run
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        Some(id) => {
            if !roster.iter().any(|entry| entry == id) {
                return Err(refusal(format!(
                    "abandon names no run: {id:?} is not on the roster; runs: {roster_line}"
                )));
            }
            Some(id.to_owned())
        }
        None if !live.is_empty() => {
            return Err(refusal(format!(
                "abandon requires --run <id>; runs: {roster_line}"
            )));
        }
        None => None,
    };

    let root_lock_present = lock_path_present(&crate::lock::root_path(&engine_root))?;
    let run_lock_present = match run_id.as_deref() {
        Some(id) => lock_path_present(&crate::lock::run_path(&engine_root, id))?,
        None => false,
    };

    let run_dir = run_id
        .as_deref()
        .map(|id| crate::Scheduler::runs_dir(root).join(id));
    let state_path = run_dir.as_ref().map(|dir| dir.join("state.toml"));
    let evidence_path = run_dir
        .as_ref()
        .map(|dir| dir.join(crate::pin::EVIDENCE_FILE));

    let admitted = state_path.as_ref().is_some_and(|path| path.exists());
    if !admitted {
        // Lock-only retirement is safe only through a normal acquisition:
        // a live owner causes its named bounded wait to refuse, while an
        // unclaimed residue is claimed and retired by that guard's Drop.
        return match run_id.as_deref() {
            Some(_) if run_lock_present => Ok(AbandonPlan {
                event: None,
                phase: None,
                retire: Vec::new(),
                run: run_id,
                annotate: Vec::new(),
                root_lock_present,
                run_lock_present,
            }),
            Some(id) => Err(refusal(format!(
                "run {id} is already terminal: its admission state is retired; nothing to retire"
            ))),
            None if root_lock_present => Ok(AbandonPlan {
                event: None,
                phase: None,
                retire: Vec::new(),
                run: None,
                annotate: Vec::new(),
                root_lock_present,
                run_lock_present: false,
            }),
            None => Err(refusal(format!(
                "nothing to retire in {}: no live run",
                project_name(root)
            ))),
        };
    }

    let mut retire = Vec::new();
    let mut phase = None;
    let mut event = None;
    if admitted {
        let state_path = state_path.expect("admitted implies an addressed run");
        let evidence_path = evidence_path.expect("admitted implies an addressed run");
        let state = crate::state::StateStore::at(state_path.clone())
            .load()
            .map_err(|error| refusal(format!("abandon cannot read the State File: {error}")))?;
        phase = Some(state.phase.clone());
        // FDC-002: the durable terminal event identifies the addressed Run and
        // its last state before any retirement begins.
        event = Some(format!(
            "- Abandoned: Run {} retired at phase {} (status {}, goal revision {}) on an explicit human confirmation; admission state, Run evidence, and lock retired. The Run is terminal: no transition may proceed on it.\n",
            run_id.as_deref().expect("admitted implies an addressed run"),
            state.phase,
            state.status,
            revision_or_none(&state.goal_revision),
        ));
        // Retire evidence before the admission State File. If retirement then
        // fails, the Run remains admitted and the compensating history entry
        // can truthfully say it was not retired.
        if evidence_path.exists() {
            retire.push(evidence_path);
        }
        retire.push(state_path);
        for path in &retire {
            ensure_retirement_file(path)?;
        }
    }

    // FDC-011: a retired child's ledger entry keeps its address; only its
    // abandoned mark flips. A corrupt ledger that names the run in its raw
    // bytes refuses the retirement rather than leaving a live-looking entry
    // behind.
    let mut annotate = Vec::new();
    if admitted {
        let run = run_id
            .as_deref()
            .expect("admitted implies an addressed run");
        for id in run_roster_at(&engine_root)? {
            let path = engine_root.join("runs").join(&id).join("spawn-ledger");
            match crate::ledger::read_entries(&path) {
                Ok(entries) => {
                    if entries
                        .iter()
                        .any(|entry| entry.id == run && !entry.abandoned)
                    {
                        annotate.push(path);
                    }
                }
                Err(error) => {
                    let raw = std::fs::read_to_string(&path).map_err(|read_error| {
                        refusal(format!(
                            "abandon cannot inspect defective spawn ledger {}: {read_error}; parse error: {error}",
                            path.display()
                        ))
                    })?;
                    if raw.contains(&format!("id = {run:?}")) {
                        return Err(refusal(format!(
                            "the spawn ledger recording {run} is defective: {error}"
                        )));
                    }
                }
            }
        }
    }
    Ok(AbandonPlan {
        event,
        phase,
        retire,
        run: run_id,
        annotate,
        root_lock_present,
        run_lock_present,
    })
}
fn lock_path_present(path: &Path) -> Result<bool, AbandonRefusal> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(refusal(format!(
            "abandon cannot inspect lock {}: {error}",
            path.display()
        ))),
    }
}

fn revision_or_none(revision: &str) -> String {
    if revision.trim().is_empty() {
        "none".to_owned()
    } else {
        revision.to_owned()
    }
}

/// Perform a planned retirement. Its terminal history entry is the first
/// durable mutation; later failures are named rather than rolled back through
/// a shared append-only history.
pub fn apply_abandon(root: &Path, plan: &AbandonPlan) -> Result<(), AbandonRefusal> {
    let engine_root = crate::root::resolve(root).engine_root().to_path_buf();
    match plan.run.as_deref() {
        Some(run_id) if plan.event.is_none() => {
            if plan.phase.is_some()
                || !plan.retire.is_empty()
                || !plan.annotate.is_empty()
                || !plan.run_lock_present
            {
                return Err(refusal(
                    "abandon lock-only plan is unsafe or stale; nothing was modified",
                ));
            }
            // An addressed abandonment always records shared history, so it
            // always takes root before the addressed Run. A stale pathname is
            // not an admission condition: normal acquisition either waits for
            // its live holder or claims and retires the residue.
            let (root_lock, run_lock) = crate::lock::acquire_root_then_run(&engine_root, run_id)
                .map_err(|error| refusal(error.to_string()))?;
            root_lock
                .ensure_current()
                .map_err(|error| refusal(error.to_string()))?;
            run_lock
                .ensure_current()
                .map_err(|error| refusal(error.to_string()))?;
            Ok(())
        }
        Some(run_id) => {
            // This pair protects the one terminal transaction: shared history
            // and any ledger mark need root, while the addressed Run needs its
            // motion lock. Keep both through retirement: this is a bounded
            // local file sequence with no guard, subprocess, or lock wait, so
            // releasing root mid-transaction would expose a terminal event
            // beside a still-admitted Run.
            let (root_lock, run_lock) = crate::lock::acquire_root_then_run(&engine_root, run_id)
                .map_err(|error| refusal(error.to_string()))?;
            apply_live_abandon(&engine_root, plan, &root_lock, &run_lock)
        }
        None => {
            if plan.event.is_some()
                || plan.phase.is_some()
                || !plan.retire.is_empty()
                || !plan.annotate.is_empty()
                || !plan.root_lock_present
                || plan.run_lock_present
            {
                return Err(refusal(
                    "abandon plan without a Run may retire only an observed stale root lock; nothing was modified",
                ));
            }
            // Claiming the extant pathname makes it ours before Drop retires
            // it. A live owner causes the bounded refusal naming this path.
            let root_lock = crate::lock::RootLock::acquire(&engine_root)
                .map_err(|error| refusal(error.to_string()))?;
            root_lock
                .ensure_current()
                .map_err(|error| refusal(error.to_string()))?;
            Ok(())
        }
    }
}

fn apply_live_abandon(
    engine_root: &Path,
    plan: &AbandonPlan,
    root_lock: &crate::lock::RootLock,
    run_lock: &crate::lock::RunLock,
) -> Result<(), AbandonRefusal> {
    let run = plan
        .run
        .as_deref()
        .ok_or_else(|| refusal("abandon plan has no addressed Run"))?;
    revalidate_abandon_plan(engine_root, plan)?;
    run_lock
        .ensure_current()
        .map_err(|error| refusal(error.to_string()))?;
    let terminal_event_written = if let Some(entry) = plan.event.as_deref() {
        append_event_once(root_lock, engine_root, entry)?;
        true
    } else {
        false
    };

    // The ledger mark follows the durable terminal event. It is idempotent,
    // so retrying a later failure cannot append a second terminal event.
    for path in &plan.annotate {
        root_lock
            .ensure_current()
            .map_err(|error| refusal(error.to_string()))?;
        if let Err(error) = crate::ledger::annotate_abandoned(path, run) {
            return Err(retirement_failure_after_event(
                engine_root,
                root_lock,
                run,
                terminal_event_written,
                format!("cannot record the ledger mark: {error}"),
            ));
        }
    }

    for path in &plan.retire {
        run_lock
            .ensure_current()
            .map_err(|error| refusal(error.to_string()))?;
        if let Err(error) = fs::remove_file(path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(retirement_failure_after_event(
                    engine_root,
                    root_lock,
                    run,
                    terminal_event_written,
                    format!("cannot retire {}: {error}", path.display()),
                ));
            }
        }
    }
    Ok(())
}

/// Re-check the Run facts that formed an abandonment plan after its final
/// lock acquisition. A caller asked to retire this exact admitted Run, not
/// whatever a concurrent lawful motion may have made it become.
fn revalidate_abandon_plan(engine_root: &Path, plan: &AbandonPlan) -> Result<(), AbandonRefusal> {
    let run = plan
        .run
        .as_deref()
        .ok_or_else(|| refusal("abandon plan has no admitted Run to revalidate"))?;
    let run_dir = engine_root.join("runs").join(run);
    let state_path = run_dir.join("state.toml");
    let evidence_path = run_dir.join(crate::pin::EVIDENCE_FILE);
    let state = crate::state::StateStore::at(state_path.clone())
        .load()
        .map_err(|error| {
            refusal(format!(
                "abandon plan is stale for run {run}: cannot reload its State File: {error}"
            ))
        })?;
    let phase = plan.phase.as_deref().ok_or_else(|| {
        refusal(format!(
            "abandon plan is stale for run {run}: its recorded phase is absent"
        ))
    })?;
    if state.phase != phase {
        return Err(refusal(format!(
            "abandon plan is stale for run {run}: phase changed from {phase:?} to {:?}",
            state.phase
        )));
    }
    let event = format!(
        "- Abandoned: Run {run} retired at phase {} (status {}, goal revision {}) on an explicit human confirmation; admission state, Run evidence, and lock retired. The Run is terminal: no transition may proceed on it.\n",
        state.phase,
        state.status,
        revision_or_none(&state.goal_revision),
    );
    if plan.event.as_deref() != Some(event.as_str()) {
        return Err(refusal(format!(
            "abandon plan is stale for run {run}: its State File status or revision changed"
        )));
    }

    let mut retire = Vec::new();
    if evidence_path.exists() {
        retire.push(evidence_path);
    }
    retire.push(state_path);
    if plan.retire != retire {
        return Err(refusal(format!(
            "abandon plan is stale for run {run}: retirement artifacts changed"
        )));
    }
    for path in &retire {
        ensure_retirement_file(path)?;
    }

    let mut annotate = Vec::new();
    for id in run_roster_at(engine_root)? {
        let path = engine_root.join("runs").join(&id).join("spawn-ledger");
        match crate::ledger::read_entries(&path) {
            Ok(entries) => {
                if entries
                    .iter()
                    .any(|entry| entry.id == run && !entry.abandoned)
                {
                    annotate.push(path);
                }
            }
            Err(error) => {
                let raw = std::fs::read_to_string(&path).map_err(|read_error| {
                    refusal(format!(
                        "abandon cannot inspect defective spawn ledger {}: {read_error}; parse error: {error}",
                        path.display()
                    ))
                })?;
                if raw.contains(&format!("id = {run:?}")) {
                    return Err(refusal(format!(
                        "the spawn ledger recording {run} is defective: {error}"
                    )));
                }
            }
        }
    }
    if plan.annotate != annotate {
        return Err(refusal(format!(
            "abandon plan is stale for run {run}: its ledger annotation changed"
        )));
    }
    Ok(())
}

/// Keep shared history append-only when a terminal event was durable but a
/// later retirement step failed. The Run remains admitted because retirement
/// files are ordered with the State File last.
fn retirement_failure_after_event(
    engine_root: &Path,
    root_lock: &crate::lock::RootLock,
    run: &str,
    terminal_event_written: bool,
    failure: String,
) -> AbandonRefusal {
    if !terminal_event_written {
        return refusal(format!("abandon {failure}"));
    }
    let compensation = format!(
        "- Abandonment compensation: the terminal event was written, retirement then failed for Run {run}; the Run was not retired. {failure}.\n"
    );
    match append_via_scheduler(root_lock, engine_root, &compensation) {
        Ok(()) => refusal(format!(
            "abandon {failure}; the terminal event was written and a compensation was appended: the Run was not retired"
        )),
        Err(append_error) => refusal(format!(
            "abandon {failure}; the terminal event was written, retirement then failed, and the Run was not retired; cannot append compensation: {append_error}"
        )),
    }
}

/// Root serializes both the presence check and the Scheduler-owned append, so
/// a retry cannot duplicate a terminal event after another Engine caller
/// writes history.
fn append_event_once(
    root_lock: &crate::lock::RootLock,
    engine_root: &Path,
    entry: &str,
) -> Result<(), AbandonRefusal> {
    root_lock
        .ensure_current()
        .map_err(|error| refusal(error.to_string()))?;
    let path = crate::Scheduler::transition_log_path(engine_root)
        .map_err(|error| refusal(error.to_string()))?;
    let existing = match fs::read_to_string(&path) {
        Ok(existing) => existing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(refusal(format!(
                "abandon cannot inspect terminal history {}: {error}",
                path.display()
            )))
        }
    };
    if existing.contains(entry) {
        return Ok(());
    }
    append_via_scheduler(root_lock, engine_root, entry)
        .map_err(|error| refusal(format!("abandon cannot record the terminal event: {error}")))
}

fn run_roster_at(engine_root: &Path) -> Result<Vec<String>, AbandonRefusal> {
    let runs = engine_root.join("runs");
    let entries = fs::read_dir(&runs).map_err(|error| {
        refusal(format!(
            "abandon cannot read run roster {}: {error}",
            runs.display()
        ))
    })?;
    let mut ids = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            refusal(format!(
                "abandon cannot read an entry in run roster {}: {error}",
                runs.display()
            ))
        })?;
        let file_type = entry.file_type().map_err(|error| {
            refusal(format!(
                "abandon cannot inspect run roster entry {}: {error}",
                entry.path().display()
            ))
        })?;
        if file_type.is_dir() {
            ids.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    ids.sort();
    Ok(ids)
}
fn ensure_retirement_file(path: &Path) -> Result<(), AbandonRefusal> {
    let metadata = fs::metadata(path).map_err(|error| {
        refusal(format!(
            "abandon cannot inspect {}: {error}",
            path.display()
        ))
    })?;
    if !metadata.is_file() {
        return Err(refusal(format!(
            "abandon cannot retire {}: it is not a regular file",
            path.display()
        )));
    }
    fs::File::open(path)
        .map(|_| ())
        .map_err(|error| refusal(format!("abandon cannot read {}: {error}", path.display())))
}

/// Ask the Scheduler-owned transition-log funnel to append while this
/// terminal transaction's root-domain guard remains current.
fn append_via_scheduler(
    root_lock: &crate::lock::RootLock,
    engine_root: &Path,
    entry: &str,
) -> std::io::Result<()> {
    root_lock
        .ensure_current()
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let mut log = crate::Scheduler::open_transition_log(engine_root, true)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    root_lock
        .ensure_current()
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    match crate::Scheduler::append_transition_log(&mut log, entry.as_bytes()) {
        crate::scheduler::TransitionLogAppend::Complete => Ok(()),
        crate::scheduler::TransitionLogAppend::Failed(failure) => {
            Err(failure.into_operator_error())
        }
    }
}
