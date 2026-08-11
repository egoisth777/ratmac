//! PGE-006 / NRR-001: the human-confirmed blocked route.
//!
//! `rtm` is a generic state-machine runner, and some runbooks have no notion
//! of a work item at all, so this module knows about Runs and nothing else. A
//! human runs
//!
//! ```text
//! rtm hold --run <run-id> --blocker <ref> --confirm "hold <run-id>"
//! ```
//!
//! and the Engine, only when every condition below holds, records the pause in
//! the Run Record, routes the Run along the Runbook's declared blocked route,
//! and appends one history entry:
//!
//! - the confirmation phrase matches `hold <run-id>` exactly: it is typed at
//!   invocation by the human who decides to hold, never read from a file an
//!   agent can write. The Engine keeps no caller identity; it checks only that
//!   the phrase was typed (ORS-001);
//! - the blocker reference exists and resolves beneath a declared runbook
//!   root. What kind of record it names is the shop's rule, never the
//!   Engine's;
//! - the addressed Run is not terminal;
//! - the current State declares a blocked route.
//!
//! An admission refusal before the route write leaves Scheduler-owned files
//! byte-identical. Once the Run Record is durable, a history append failure
//! never rewrites it; it is reported as an honest error.
//!
//! The Engine writes no file under any workflow root here (NRR-001). A shop
//! that also marks its own documents does that itself, in its own landing.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "test-fault-injection")]
use std::thread;
#[cfg(feature = "test-fault-injection")]
use std::time::{Duration, Instant};

use crate::root::Displayed;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

static HOLD_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "test-fault-injection")]
const MAX_TEST_HOLD_BARRIER_TIMEOUT: Duration = Duration::from_secs(30);

/// Whether an atomic replacement reached its destination durably, or reached
/// it but could not confirm the parent directory's durability.
#[derive(Debug)]
enum ReplaceFileOutcome {
    Durable,
    ReplacedWithParentSyncWarning(std::io::Error),
}

/// Why a hold cannot be applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoldRefusal {
    pub reason: String,
}

impl fmt::Display for HoldRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

fn refusal(reason: impl Into<String>) -> HoldRefusal {
    HoldRefusal {
        reason: reason.into(),
    }
}

#[cfg(feature = "test-fault-injection")]
fn hold_barrier_timeout() -> Result<Duration, HoldRefusal> {
    let Some(value) = std::env::var("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS").ok() else {
        return Ok(crate::lock::WAIT_TIMEOUT);
    };
    let milliseconds = value.parse::<u64>().map_err(|error| {
        refusal(format!(
            "RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS must be a positive integer: {error}"
        ))
    })?;
    if milliseconds == 0 {
        return Err(refusal(
            "RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS must be a positive integer",
        ));
    }
    Ok(Duration::from_millis(milliseconds).min(MAX_TEST_HOLD_BARRIER_TIMEOUT))
}

/// Feature-gated QA seam at a named hold mutation boundary. The caller keeps
/// its root and Run claims live while waiting, so the marker proves that
/// boundary is occupied.
#[cfg(feature = "test-fault-injection")]
fn wait_before_hold_boundary_if_requested(boundary: &str) -> Result<(), HoldRefusal> {
    if std::env::var("RATMAC_TEST_HOLD_BARRIER").ok().as_deref() != Some(boundary) {
        return Ok(());
    }
    let marker = std::env::var_os("RATMAC_TEST_HOLD_BARRIER_MARKER")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| refusal("hold test barrier needs RATMAC_TEST_HOLD_BARRIER_MARKER"))?;
    let release = std::env::var_os("RATMAC_TEST_HOLD_BARRIER_RELEASE")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| refusal("hold test barrier needs RATMAC_TEST_HOLD_BARRIER_RELEASE"))?;
    if let Some(parent) = marker.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            refusal(format!(
                "hold test barrier cannot create marker directory {}: {error}",
                parent.displayed()
            ))
        })?;
    }
    fs::write(&marker, format!("holding before {boundary}\n")).map_err(|error| {
        refusal(format!(
            "hold test barrier cannot write marker {}: {error}",
            marker.displayed()
        ))
    })?;

    let deadline = Instant::now() + hold_barrier_timeout()?;
    loop {
        if release.is_file() {
            return Ok(());
        }
        let now = Instant::now();
        if now >= deadline {
            return Err(refusal(format!(
                "hold test barrier expired before {boundary} waiting for {}",
                release.displayed()
            )));
        }
        thread::sleep(Duration::from_millis(10).min(deadline.saturating_duration_since(now)));
    }
}

#[cfg(not(feature = "test-fault-injection"))]
fn wait_before_hold_boundary_if_requested(_boundary: &str) -> Result<(), HoldRefusal> {
    Ok(())
}

/// What a human asked for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoldRequest {
    /// An opaque reference to whatever the shop says blocks this Run.
    pub blocker: Option<String>,
    pub confirmation: Option<String>,
    /// The addressed run (FDC-004: `--run <id>`, always required).
    pub run: Option<String>,
}

/// A verified hold, ready to apply.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoldPlan {
    blocker: String,
    run_id: String,
    /// reloads and compares both fields after taking the addressed Run lock.
    from_state: String,
    from_status: crate::model::Status,
    to_state: String,
}

impl HoldPlan {
    pub(crate) fn run_id(&self) -> &str {
        &self.run_id
    }

    pub(crate) fn blocker(&self) -> &str {
        &self.blocker
    }

    pub(crate) fn source_state(&self) -> &str {
        &self.from_state
    }

    pub(crate) fn to_state(&self) -> &str {
        &self.to_state
    }
}

/// The exact phrase a human must type to hold the Run `run_id`.
pub fn confirmation_phrase(run_id: &str) -> String {
    format!("hold {run_id}")
}

/// The `p5-blocked` route predicate: verify a hold without writing anything.
pub fn plan_hold(root: &Path, request: &HoldRequest) -> Result<HoldPlan, HoldRefusal> {
    crate::Scheduler::refuse_flat_residue(root).map_err(|error| refusal(error.to_string()))?;

    // FDC-004/FDC-005: hold is an existing-Run operation. Resolve one exact
    // canonical roster member through Scheduler::open_run so flat residue and
    // the recorded runbook pin are checked before this plan can permit a
    // mutation.
    let roster = crate::Scheduler::run_roster(root).map_err(|error| refusal(error.to_string()))?;
    let roster_line = if roster.is_empty() {
        "none".to_owned()
    } else {
        roster.join(", ")
    };
    let Some(run_id) = request.run.as_deref().filter(|id| !id.is_empty()) else {
        return Err(refusal(format!(
            "hold requires --run <id>; runs: {roster_line}"
        )));
    };

    let required = confirmation_phrase(run_id);
    match request.confirmation.as_deref().map(str::trim) {
        None => {
            return Err(refusal(format!(
                "hold is unconfirmed: a human must type --confirm {required:?}"
            )))
        }
        Some(phrase) if phrase != required => {
            return Err(refusal(format!(
                "hold is unconfirmed: confirmation {phrase:?} does not match the required phrase {required:?}"
            )))
        }
        Some(_) => {}
    }

    let Some(blocker) = request.blocker.as_deref().map(str::trim) else {
        return Err(refusal(
            "hold has no blocker link: pass --blocker <reference beneath a declared root>",
        ));
    };
    if blocker.is_empty() {
        return Err(refusal(
            "hold has no blocker link: pass --blocker <reference beneath a declared root>",
        ));
    }

    let scheduler =
        crate::Scheduler::open_run(root, run_id).map_err(|error| refusal(error.to_string()))?;
    verify_blocker(&scheduler, blocker)?;

    let state = scheduler
        .load_state()
        .map_err(|error| refusal(format!("hold requires an active Run: {error}")))?;
    let from_state = state.state.clone();
    // FDC-002: a passed Run admits no further transition — not even the
    // human-confirmed blocked route. The refusal precedes any route lookup.
    if state.status == crate::model::Status::Passed {
        return Err(refusal(format!(
            "run {run_id} is terminal (status passed): a blocked route may not move it"
        )));
    }
    if state.status == crate::model::Status::Blocked {
        return Err(refusal(format!(
            "run {run_id} is already blocked against {}",
            if state.blocker.is_empty() {
                "an unnamed blocker"
            } else {
                state.blocker.as_str()
            }
        )));
    }
    let Some(route) = scheduler.machine().blocked_route_for(&from_state) else {
        return Err(refusal(format!(
            "State {from_state:?} declares no blocked route; add a transition with blocked-route = true"
        )));
    };

    Ok(HoldPlan {
        blocker: blocker.to_owned(),
        run_id: run_id.to_owned(),
        from_state,
        from_status: state.status,
        to_state: route.to().as_str().to_owned(),
    })
}

/// NRR-001: the blocker is an opaque reference. The Engine checks that it
/// exists and that it resolves beneath a root the runbook declares - never
/// what kind of record it is, which is the shop's rule and not the Engine's.
fn verify_blocker(scheduler: &crate::Scheduler, blocker: &str) -> Result<(), HoldRefusal> {
    // The reference belongs to the addressed Run's workspace: that is the
    // tree its declared roots resolve in, whether it is the invoking checkout
    // or a linked one bound at spawn.
    let root = scheduler
        .workspace_root()
        .map_err(|error| refusal(error.to_string()))?;
    let relative = Path::new(blocker);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                std::path::Component::Prefix(_)
                    | std::path::Component::RootDir
                    | std::path::Component::ParentDir
            )
        })
    {
        return Err(refusal(format!(
            "blocker {blocker:?} must stay beneath a declared root"
        )));
    }
    let canonical_root = fs::canonicalize(root).map_err(|error| {
        refusal(format!(
            "cannot resolve the Run workspace {} while checking blocker {blocker}: {error}",
            root.displayed()
        ))
    })?;
    let candidate = canonical_root.join(relative);
    let path = fs::canonicalize(&candidate).map_err(|error| {
        refusal(format!(
            "blocker {blocker} does not resolve beneath a declared root: {error}"
        ))
    })?;

    let mut declared: Vec<String> = Vec::new();
    for (role, directory) in scheduler.workflow_roots() {
        declared.push(role.to_owned());
        let canonical_directory = match fs::canonicalize(directory) {
            Ok(resolved) => resolved,
            Err(_) => continue,
        };
        if path.starts_with(&canonical_directory) {
            return Ok(());
        }
    }
    declared.sort();
    let roots_line = if declared.is_empty() {
        "the runbook declares no [roots]".to_owned()
    } else {
        format!("declared roots: {}", declared.join(", "))
    };
    Err(refusal(format!(
        "blocker {blocker} resolves outside every declared root; {roots_line}"
    )))
}

/// Apply a verified hold: the paused mark in the Run Record, the routed
/// State, and one history entry.
///
/// Everything the hold changes lives under the Engine root and belongs to one
/// Run, so the addressed Run lock alone serializes it - there is no shared
/// document to read-modify-write any more (NRR-001). The exact planned state
/// is compared after acquisition, and a failure after the record is durable is
/// reported honestly rather than rewritten.
pub fn apply_hold(root: &Path, plan: &HoldPlan) -> Result<(), HoldRefusal> {
    // Planning and application are separate public boundaries. Resolve from
    // the addressed Run's workspace again before this path can mutate state.
    let scheduler = crate::Scheduler::open_run(root, &plan.run_id)
        .map_err(|error| refusal(error.to_string()))?;
    verify_blocker(&scheduler, &plan.blocker)?;

    let roots = crate::root::resolve(root);
    let engine_root = roots.engine_root().to_path_buf();
    let state_path = engine_root.join("runs").join(&plan.run_id).join("run.toml");

    let run_lock = crate::lock::RunLock::acquire(&engine_root, &plan.run_id)
        .map_err(|error| refusal(error.to_string()))?;
    run_lock
        .ensure_current()
        .map_err(|error| refusal(error.to_string()))?;

    let old_state_bytes = fs::read(&state_path)
        .map_err(|error| refusal(format!("hold cannot read state: {error}")))?;
    let store = crate::state::StateStore::for_engine_root(&engine_root, &plan.run_id);
    let mut state = store
        .load()
        .map_err(|error| refusal(format!("hold cannot read state: {error}")))?;
    // A plan is only an admission proof. Another lawful motion may have
    // changed this Run between planning and acquisition, so compare the exact
    // state that chose the blocked route before writing anything.
    if state.state != plan.from_state || state.status != plan.from_status {
        return Err(refusal(format!(
            "hold plan is stale for run {}: expected state {:?} with status {:?}, found state {:?} with status {:?}; nothing was modified",
            plan.run_id, plan.from_state, plan.from_status, state.state, state.status
        )));
    }
    // Reopen while holding the mutation lock. This binds the route to the same
    // freshly pinned class that permits this write, rather than trusting
    // public fields in a caller-supplied HoldPlan.
    let current_scheduler = crate::Scheduler::open_run(root, &plan.run_id)
        .map_err(|error| refusal(error.to_string()))?;
    let Some(route) = current_scheduler.machine().blocked_route_for(&state.state) else {
        return Err(refusal(format!(
            "hold route changed: state {:?} no longer declares a blocked route; re-plan the hold",
            state.state
        )));
    };
    if route.to().as_str() != plan.to_state {
        return Err(refusal(format!(
            "hold route changed: planned state {:?} -> {:?}, but the declared blocked route is {:?} -> {:?}; re-plan the hold",
            plan.from_state,
            plan.to_state,
            state.state,
            route.to().as_str()
        )));
    }

    // Snapshot the pre-existing append target through the Scheduler before
    // committing state: an unusable or missing log refuses without a
    // half-motion.
    let mut log = crate::Scheduler::open_transition_log(&engine_root, false).map_err(|error| {
        refusal(format!(
            "hold cannot open history: {error}; nothing was modified"
        ))
    })?;

    state.state = plan.to_state.clone();
    // The whole held fact, in Engine-owned state: the paused mark and the
    // opaque reference the human named (NRR-001).
    state.status = crate::model::Status::Blocked;
    state.blocker = plan.blocker.clone();
    run_lock
        .ensure_current()
        .map_err(|error| refusal(error.to_string()))?;
    wait_before_hold_boundary_if_requested("before-state-write")?;
    current_scheduler
        .verify_open_runbook_snapshot()
        .map_err(|error| refusal(error.to_string()))?;
    match store
        .write(&state)
        .map_err(|error| refusal(format!("hold cannot write state: {error}")))?
    {
        crate::state::StateWriteOutcome::Durable => {}
        crate::state::StateWriteOutcome::ReplacedWithParentSyncWarning(error) => {
            eprintln!(
                "warning: hold Run Record {} was replaced but its parent directory could not be synced: {error}; continuing because the Run Record is committed and the hold will append history",
                state_path.displayed()
            );
        }
    }

    // The QA seam that used to sit before the shared-document replacement now
    // sits between the durable record and its history entry: the last point at
    // which an interruption can be observed mid-hold.
    if let Err(error) = wait_before_hold_boundary_if_requested("before-history-append") {
        let restore = restore_state_bytes(&run_lock, &state_path, &old_state_bytes);
        return match restore {
            Ok(()) => Err(refusal(format!("{}; nothing was written", error.reason))),
            Err(restore_error) => Err(refusal(format!(
                "{}; state rollback incomplete: {restore_error}",
                error.reason
            ))),
        };
    }
    if let Err(error) = run_lock.ensure_current() {
        let restore = restore_state_bytes(&run_lock, &state_path, &old_state_bytes);
        return match restore {
            Ok(()) => Err(refusal(format!(
                "hold cannot verify the Run mutation lock: {error}; nothing was written"
            ))),
            Err(restore_error) => Err(refusal(format!(
                "hold cannot verify the Run mutation lock: {error}; state rollback incomplete: {restore_error}"
            ))),
        };
    }

    // Keep the addressed Run lock through its own route and append-only record.
    let entry = format!(
        "- Hold: run {} paused against {}; routed {} -> {} on an explicit human confirmation.\n",
        plan.run_id, plan.blocker, plan.from_state, plan.to_state
    );
    match crate::Scheduler::append_transition_log(&mut log, entry.as_bytes()) {
        crate::scheduler::TransitionLogAppend::Complete => Ok(()),
        crate::scheduler::TransitionLogAppend::Failed(failure) => Err(refusal(format!(
            "hold cannot append history; the Run Record was updated and no history rewrite was attempted: {}",
            failure.into_operator_error()
        ))),
    }
}

/// Restore exact addressed-Run bytes only while the caller still owns its
/// motion lock. The replacement is atomic, so a failed restore never truncates
/// a Run Record it cannot finish restoring.
fn restore_state_bytes(
    run_lock: &crate::lock::RunLock,
    state_path: &Path,
    bytes: &[u8],
) -> Result<(), crate::state::StateError> {
    run_lock.ensure_current()?;
    match replace_file_atomically(state_path, bytes) {
        Ok(ReplaceFileOutcome::Durable) => Ok(()),
        Ok(ReplaceFileOutcome::ReplacedWithParentSyncWarning(error)) => {
            eprintln!(
                "warning: restored pre-hold Run Record {} but could not sync its parent directory: {error}",
                state_path.displayed()
            );
            Ok(())
        }
        Err(error) => Err(crate::state::StateError::new(format!(
            "restore pre-hold Run Record {}: {error}",
            state_path.displayed()
        ))),
    }
}

/// Replace an existing artifact from a same-directory temporary file.
///
/// A failed temporary write leaves the destination intact. The only replace
/// step is atomic on supported platforms, matching StateStore's durable-write
/// discipline for exact Run Record rollback. Once the rename
/// succeeds, a parent-sync failure is reported as a warning rather than as an
/// untouched destination: the replacement has already happened.
fn replace_file_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<ReplaceFileOutcome> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} has no parent directory", path.displayed()),
        )
    })?;
    let sequence = HOLD_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let temporary = parent.join(format!(
        ".{name}.hold-tmp-{}-{sequence}",
        std::process::id()
    ));
    let result: std::io::Result<ReplaceFileOutcome> = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        match fs::rename(&temporary, path) {
            Ok(()) => {}
            Err(_) if path.exists() => replace_existing_file(&temporary, path)?,
            Err(error) => return Err(error),
        }
        match sync_parent(parent) {
            Ok(()) => Ok(ReplaceFileOutcome::Durable),
            Err(error) => Ok(ReplaceFileOutcome::ReplacedWithParentSyncWarning(error)),
        }
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(unix)]
fn sync_parent(parent: &Path) -> std::io::Result<()> {
    std::fs::File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent(_parent: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn replace_existing_file(temporary: &Path, destination: &Path) -> std::io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let temporary: Vec<u16> = temporary
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: both NUL-terminated paths remain live for this synchronous API
    if unsafe {
        ReplaceFileW(
            destination.as_ptr(),
            temporary.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    } == 0
    {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_existing_file(temporary: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(temporary, destination)
}
