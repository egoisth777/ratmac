//! PGE-006: the human-confirmed blocked route.
//!
//! A ticket blocked for an out-of-scope reason gets an honest exit instead of
//! a dishonest pass. A human runs
//!
//! ```text
//! rtm hold <ticket-id> --blocker <ref> --confirm "hold <ticket-id>"
//! ```
//!
//! and the Engine, only when every condition below holds, marks the ticket
//! `held`, records what blocks it, and routes the Run along the Runbook's
//! declared blocked route:
//!
//! - the confirmation phrase matches `hold <ticket-id>` exactly: it is typed
//!   at invocation by the human who decides to hold, never read from a file an
//!   agent can write. The Engine keeps no caller identity; it checks only that
//!   the phrase was typed (ORS-001);
//! - the ticket exists and is not already passed;
//! - the blocker reference resolves to a complete five-file issue folder or a
//!   named residual record;
//! - the current Phase declares a blocked route.
//!
//! An admission refusal before the route write leaves Scheduler-owned files
//! byte-identical. A ticket replacement failure before it reaches its
//! destination restores the addressed Run's pre-route state. If replacement
//! reached the ticket but its parent directory cannot be synced, the Engine
//! warns and keeps the agreeing ticket and Run state. Once that route and
//! ticket are durable, a history append failure never rewrites either artifact;
//! it is reported as an honest error.
//!
//! The ticket stays not-passed and its residuals untouched: holding a ticket
//! proves nothing about the work.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "test-fault-injection")]
use std::thread;
#[cfg(feature = "test-fault-injection")]
use std::time::{Duration, Instant};

use crate::contract::ISSUE_FILES;
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
fn ticket_replace_barrier_timeout() -> Result<Duration, HoldRefusal> {
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

/// Feature-gated QA seam after the final ticket comparison and before its
/// replacement. The caller keeps its root and Run claims live while waiting,
/// so the marker proves the shared-ticket mutation boundary is occupied.
#[cfg(feature = "test-fault-injection")]
fn wait_before_ticket_replace_if_requested() -> Result<(), HoldRefusal> {
    if std::env::var("RATMAC_TEST_HOLD_BARRIER").ok().as_deref() != Some("before-ticket-replace") {
        return Ok(());
    }
    let marker = std::env::var_os("RATMAC_TEST_HOLD_BARRIER_MARKER")
        .map(PathBuf::from)
        .ok_or_else(|| refusal("hold test barrier needs RATMAC_TEST_HOLD_BARRIER_MARKER"))?;
    let release = std::env::var_os("RATMAC_TEST_HOLD_BARRIER_RELEASE")
        .map(PathBuf::from)
        .ok_or_else(|| refusal("hold test barrier needs RATMAC_TEST_HOLD_BARRIER_RELEASE"))?;
    if let Some(parent) = marker.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            refusal(format!(
                "hold test barrier cannot create marker directory {}: {error}",
                parent.display()
            ))
        })?;
    }
    fs::write(&marker, "holding before ticket replacement\n").map_err(|error| {
        refusal(format!(
            "hold test barrier cannot write marker {}: {error}",
            marker.display()
        ))
    })?;

    let deadline = Instant::now() + ticket_replace_barrier_timeout()?;
    loop {
        if release.is_file() {
            return Ok(());
        }
        let now = Instant::now();
        if now >= deadline {
            return Err(refusal(format!(
                "hold test barrier expired before ticket replacement waiting for {}",
                release.display()
            )));
        }
        thread::sleep(Duration::from_millis(10).min(deadline.saturating_duration_since(now)));
    }
}

#[cfg(not(feature = "test-fault-injection"))]
fn wait_before_ticket_replace_if_requested() -> Result<(), HoldRefusal> {
    Ok(())
}

/// What a human asked for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoldRequest {
    pub ticket: String,
    pub blocker: Option<String>,
    pub confirmation: Option<String>,
    /// The addressed run (FDC-004: `--run <id>`, always required).
    pub run: Option<String>,
}

/// A verified hold, ready to apply.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoldPlan {
    pub ticket: String,
    pub ticket_path: PathBuf,
    pub blocker: String,
    pub run_id: String,
    /// reloads and compares both fields after taking the root-then-addressed
    /// Run lock pair.
    pub from_phase: String,
    pub from_status: crate::model::Status,
    pub to_phase: String,
}

/// The exact phrase a human must type to hold `ticket`.
pub fn confirmation_phrase(ticket: &str) -> String {
    format!("hold {ticket}")
}

fn declared_ticket_root(workspace: &Path) -> Result<PathBuf, HoldRefusal> {
    let engine = crate::root::resolve(workspace);
    let class = crate::machine::MachineClass::load_from_project_root(workspace)
        .map_err(|error| refusal(format!("{}: {}", error.code(), error.message())))?;
    class
        .validate_roots(engine.invoking_checkout_root(), engine.engine_root())
        .map_err(|error| refusal(error.to_string()))?;
    class
        .resolve_root(
            "ticket",
            engine.invoking_checkout_root(),
            engine.engine_root(),
        )
        .map_err(|error| refusal(error.to_string()))
}

/// The `p5-blocked` route predicate: verify a hold without writing anything.
pub fn plan_hold(root: &Path, request: &HoldRequest) -> Result<HoldPlan, HoldRefusal> {
    let ticket = request.ticket.trim();
    if ticket.is_empty() {
        return Err(refusal("hold names no ticket"));
    }
    let required = confirmation_phrase(ticket);
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
            "hold has no blocker link: pass --blocker <issue folder or residual record>",
        ));
    };
    if blocker.is_empty() {
        return Err(refusal(
            "hold has no blocker link: pass --blocker <issue folder or residual record>",
        ));
    }

    let ticket_root = declared_ticket_root(root)?;
    let ticket_path = ticket_root.join(format!("{ticket}.md"));
    let source = fs::read_to_string(&ticket_path).map_err(|error| {
        refusal(format!(
            "hold refers to no ticket: {} is unreadable ({error})",
            ticket_path.display()
        ))
    })?;
    let status = field(&source, "status").unwrap_or_default();
    if status == "passed" {
        return Err(refusal(format!(
            "ticket {ticket} is already passed; a passed ticket has nothing to hold"
        )));
    }
    if status == "held" {
        return Err(refusal(format!("ticket {ticket} is already held")));
    }

    verify_blocker(root, blocker)?;

    // FDC-004/FDC-005: hold is an existing-Run operation. Resolve one exact
    // canonical roster member through Scheduler::open_run so flat residue and
    // the recorded runbook pin are checked before this plan can permit a
    // mutation.
    let roster = crate::Scheduler::run_roster(root);
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
    let scheduler =
        crate::Scheduler::open_run(root, run_id).map_err(|error| refusal(error.to_string()))?;
    let state = scheduler
        .load_state()
        .map_err(|error| refusal(format!("hold requires an active Run: {error}")))?;
    let from_phase = state.phase.clone();
    // FDC-002: a passed Run admits no further transition — not even the
    // human-confirmed blocked route. The refusal precedes any route lookup.
    if state.status == crate::model::Status::Passed {
        return Err(refusal(format!(
            "run {run_id} is terminal (status passed): a blocked route may not move it"
        )));
    }
    let Some(route) = scheduler.machine().blocked_route_for(&from_phase) else {
        return Err(refusal(format!(
            "Phase {from_phase:?} declares no blocked route; add a transition with blocked-route = true"
        )));
    };

    Ok(HoldPlan {
        ticket: ticket.to_owned(),
        ticket_path,
        blocker: blocker.to_owned(),
        run_id: run_id.to_owned(),
        from_phase,
        from_status: state.status,
        to_phase: route.to().as_str().to_owned(),
    })
}

/// A blocker record is a complete five-file issue folder or a named residual.
fn verify_blocker(root: &Path, blocker: &str) -> Result<(), HoldRefusal> {
    let path = root.join(blocker);
    if path.is_dir() {
        let mut missing: Vec<&str> = Vec::new();
        for required in ISSUE_FILES {
            if !path.join(required).is_file() {
                missing.push(required);
            }
        }
        if missing.is_empty() {
            return Ok(());
        }
        return Err(refusal(format!(
            "blocker {blocker} is not a complete five-file issue record: missing {}",
            missing.join(", ")
        )));
    }
    if path.is_file() {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if name.starts_with("res-") && name.ends_with(".md") {
            return Ok(());
        }
        return Err(refusal(format!(
            "blocker {blocker} is neither a five-file issue folder nor a named residual record"
        )));
    }
    Err(refusal(format!(
        "blocker {blocker} does not resolve to any artifact"
    )))
}

/// Apply a verified hold: ticket state, routed Phase, and one history entry.
///
/// The plan's cheap non-ticket checks happen before the lock pair. The exact
/// planned state is compared after acquisition. The shared ticket is a
/// root-domain read-modify-write: root is acquired before the addressed Run and
/// remains held from its source read through the final compare and replacement,
/// so ordinary lifecycle callers cannot write between them. Root releases
/// before the append-only history record. A portable rename still cannot detect
/// an out-of-band edit that races its final replacement window.
pub fn apply_hold(root: &Path, plan: &HoldPlan) -> Result<(), HoldRefusal> {
    // Planning and application are separate public boundaries. Recheck the
    // current declared roots before this path can mutate a ticket or State.
    let _ = declared_ticket_root(root)?;

    let roots = crate::root::resolve(root);
    let engine_root = roots.engine_root().to_path_buf();
    let state_path = engine_root
        .join("runs")
        .join(&plan.run_id)
        .join("state.toml");

    // The ticket is shared across Runs, while the State File is not. Acquire
    // the pair in the global root-before-Run order before reading the shared
    // ticket, so the entire read-modify-write has mutual exclusion.
    let (root_lock, run_lock) = crate::lock::acquire_root_then_run(&engine_root, &plan.run_id)
        .map_err(|error| refusal(error.to_string()))?;
    root_lock
        .ensure_current()
        .map_err(|error| refusal(error.to_string()))?;
    run_lock
        .ensure_current()
        .map_err(|error| refusal(error.to_string()))?;
    let source = fs::read_to_string(&plan.ticket_path)
        .map_err(|error| refusal(format!("hold cannot read the ticket: {error}")))?;
    match field(&source, "status").unwrap_or_default().as_str() {
        "passed" => {
            return Err(refusal(format!(
                "ticket {} is already passed; a passed ticket has nothing to hold",
                plan.ticket
            )));
        }
        "held" => return Err(refusal(format!("ticket {} is already held", plan.ticket))),
        _ => {}
    }
    let held = hold_ticket(&source, &plan.blocker);

    // Snapshot only after owning the root-then-Run pair. If ticket writing
    // later fails, restoration returns these exact locked bytes, never bytes a
    // concurrent lawful motion left before we acquired the Run lock.
    let old_state_bytes = fs::read(&state_path)
        .map_err(|error| refusal(format!("hold cannot read state: {error}")))?;
    let store = crate::state::StateStore::for_engine_root(&engine_root, &plan.run_id);
    let mut state = store
        .load()
        .map_err(|error| refusal(format!("hold cannot read state: {error}")))?;
    // A plan is only an admission proof. Another lawful motion may have
    // changed this Run between planning and acquisition, so compare the exact
    // state that chose the blocked route before writing anything.
    if state.phase != plan.from_phase || state.status != plan.from_status {
        return Err(refusal(format!(
            "hold plan is stale for run {}: expected phase {:?} with status {:?}, found phase {:?} with status {:?}; nothing was modified",
            plan.run_id, plan.from_phase, plan.from_status, state.phase, state.status
        )));
    }
    let current_ticket = fs::read_to_string(&plan.ticket_path).map_err(|error| {
        refusal(format!(
            "hold cannot reread ticket {}: {error}",
            plan.ticket_path.display()
        ))
    })?;
    if current_ticket != source {
        return Err(refusal(format!(
            "hold ticket {} changed while the hold was being prepared; nothing was written",
            plan.ticket_path.display()
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

    state.phase = plan.to_phase.clone();
    run_lock
        .ensure_current()
        .map_err(|error| refusal(error.to_string()))?;
    match store
        .write(&state)
        .map_err(|error| refusal(format!("hold cannot write state: {error}")))?
    {
        crate::state::StateWriteOutcome::Durable => {}
        crate::state::StateWriteOutcome::ReplacedWithParentSyncWarning(error) => {
            eprintln!(
                "warning: hold State File {} was replaced but its parent directory could not be synced: {error}; continuing because the State File is committed and the hold will append history",
                state_path.display()
            );
        }
    }

    // Check again immediately before replacing the ticket. This catches an
    // edit that arrives before the comparison; an edit inside the portable
    // replacement window cannot be detected.
    let current_ticket = fs::read_to_string(&plan.ticket_path);
    match current_ticket {
        Ok(current) if current == source => {}
        Ok(_) => {
            let restore = restore_state_bytes(&run_lock, &state_path, &old_state_bytes);
            return match restore {
                Ok(()) => Err(refusal(format!(
                    "hold ticket {} changed while the hold was being prepared; nothing was written",
                    plan.ticket_path.display()
                ))),
                Err(restore_error) => Err(refusal(format!(
                    "hold ticket {} changed while the hold was being prepared; state rollback incomplete: {restore_error}",
                    plan.ticket_path.display()
                ))),
            };
        }
        Err(error) => {
            let restore = restore_state_bytes(&run_lock, &state_path, &old_state_bytes);
            return match restore {
                Ok(()) => Err(refusal(format!(
                    "hold cannot reread ticket {}: {error}; nothing was written",
                    plan.ticket_path.display()
                ))),
                Err(restore_error) => Err(refusal(format!(
                    "hold cannot reread ticket {}: {error}; state rollback incomplete: {restore_error}",
                    plan.ticket_path.display()
                ))),
            };
        }
    }

    // The shared ticket comparison and replacement run under the root claim.
    // Recheck both live claims before publishing the QA marker and again after
    // its release; a barrier error restores the state written above.
    if let Err(error) = root_lock
        .ensure_current()
        .and_then(|_| run_lock.ensure_current())
    {
        let restore = restore_state_bytes(&run_lock, &state_path, &old_state_bytes);
        return match restore {
            Ok(()) => Err(refusal(format!(
                "hold cannot verify the ticket mutation lock: {error}; nothing was written"
            ))),
            Err(restore_error) => Err(refusal(format!(
                "hold cannot verify the ticket mutation lock: {error}; state rollback incomplete: {restore_error}"
            ))),
        };
    }
    if let Err(error) = wait_before_ticket_replace_if_requested() {
        let restore = restore_state_bytes(&run_lock, &state_path, &old_state_bytes);
        return match restore {
            Ok(()) => Err(refusal(format!("{}; nothing was written", error.reason))),
            Err(restore_error) => Err(refusal(format!(
                "{}; state rollback incomplete: {restore_error}",
                error.reason
            ))),
        };
    }
    if let Err(error) = root_lock
        .ensure_current()
        .and_then(|_| run_lock.ensure_current())
    {
        let restore = restore_state_bytes(&run_lock, &state_path, &old_state_bytes);
        return match restore {
            Ok(()) => Err(refusal(format!(
                "hold cannot verify the ticket mutation lock: {error}; nothing was written"
            ))),
            Err(restore_error) => Err(refusal(format!(
                "hold cannot verify the ticket mutation lock: {error}; state rollback incomplete: {restore_error}"
            ))),
        };
    }
    let parent_sync_warning = match replace_file_atomically(&plan.ticket_path, held.as_bytes()) {
        Ok(ReplaceFileOutcome::Durable) => None,
        Ok(ReplaceFileOutcome::ReplacedWithParentSyncWarning(error)) => Some(error),
        Err(error) => {
            let restore = restore_state_bytes(&run_lock, &state_path, &old_state_bytes);
            return match restore {
                Ok(()) => Err(refusal(format!("hold cannot write the ticket: {error}"))),
                Err(restore_error) => Err(refusal(format!(
                    "hold cannot write the ticket: {error}; state rollback incomplete: {restore_error}"
                ))),
            };
        }
    };
    // The shared ticket is now durable. Release root before diagnostics or the
    // append-only history record can take any further time.
    drop(root_lock);
    if let Some(error) = parent_sync_warning {
        eprintln!(
            "warning: hold ticket {} was replaced but its parent directory could not be synced: {error}; continuing because the ticket and Run state now agree",
            plan.ticket_path.display()
        );
    }

    // Keep the addressed Run lock through its own route and append-only record,
    // so an unrelated root holder cannot delay a completed hold.
    let entry = format!(
        "- Hold: ticket {} held against {}; Run {} routed {} -> {} on an explicit human confirmation. The ticket is not passed and its residuals stay unproven.\n",
        plan.ticket, plan.blocker, plan.run_id, plan.from_phase, plan.to_phase
    );
    match crate::Scheduler::append_transition_log(&mut log, entry.as_bytes()) {
        crate::scheduler::TransitionLogAppend::Complete => Ok(()),
        crate::scheduler::TransitionLogAppend::Failed(failure) => Err(refusal(format!(
            "hold cannot append history; the ticket and Run state were updated and no history rewrite was attempted: {}",
            failure.into_operator_error()
        ))),
    }
}

/// Rewrite a ticket's front matter: `status: "held"` plus its blocker link.
fn hold_ticket(source: &str, blocker: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut wrote_status = false;
    let mut wrote_blocker = false;
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("status:") && !wrote_status {
            lines.push("status: \"held\"".to_owned());
            wrote_status = true;
            continue;
        }
        if trimmed.starts_with("blocker-ref:") && !wrote_blocker {
            lines.push(format!("blocker-ref: \"{blocker}\""));
            wrote_blocker = true;
            continue;
        }
        lines.push(line.to_owned());
    }
    if !wrote_blocker {
        // Place the link beside the status it explains.
        if let Some(index) = lines
            .iter()
            .position(|line| line.trim_start().starts_with("status:"))
        {
            lines.insert(index + 1, format!("blocker-ref: \"{blocker}\""));
        } else {
            lines.push(format!("blocker-ref: \"{blocker}\""));
        }
    }
    let mut text = lines.join("\n");
    if source.ends_with('\n') {
        text.push('\n');
    }
    text
}

/// Restore exact addressed-Run bytes only while the caller still owns its
/// motion lock. The replacement is atomic, so a failed restore never truncates
/// a State File it cannot finish restoring.
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
                "warning: restored pre-hold State File {} but could not sync its parent directory: {error}",
                state_path.display()
            );
            Ok(())
        }
        Err(error) => Err(crate::state::StateError::new(format!(
            "restore pre-hold State File {}: {error}",
            state_path.display()
        ))),
    }
}

/// Replace an existing artifact from a same-directory temporary file.
///
/// A failed temporary write leaves the destination intact. The only replace
/// step is atomic on supported platforms, matching StateStore's durable-write
/// discipline for the human ticket and exact State rollback. Once the rename
/// succeeds, a parent-sync failure is reported as a warning rather than as an
/// untouched destination: the replacement has already happened.
fn replace_file_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<ReplaceFileOutcome> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} has no parent directory", path.display()),
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

/// Read `key: value` from a record's front matter.
fn field(source: &str, key: &str) -> Option<String> {
    source.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(key)?.strip_prefix(':')?;
        Some(rest.trim().trim_matches('"').to_owned())
    })
}

/// Whether a ticket is currently held, and against what.
pub fn held_against(ticket_source: &str) -> Option<String> {
    if field(ticket_source, "status").as_deref() != Some("held") {
        return None;
    }
    Some(field(ticket_source, "blocker-ref").unwrap_or_default())
}
