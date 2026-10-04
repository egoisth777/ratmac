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

pub use crate::root::entries::{apply_hold, plan_hold};
pub use crate::root::entries::{apply_resume, plan_resume};
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

static HOLD_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "test-fault-injection")]
const MAX_TEST_HOLD_BARRIER_TIMEOUT: Duration = Duration::from_secs(30);

/// Whether an atomic replacement reached its destination durably, or reached
/// it but could not confirm the parent directory's durability.
#[derive(Debug)]
pub(crate) enum ReplaceFileOutcome {
    Durable,
    ReplacedWithParentSyncWarning(std::io::Error),
}

/// Why a hold cannot be applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoldRefusal {
    pub reason: String,
    /// WRS-003: the operation already committed through the journal; the
    /// failure renders as recovery-pending, never as an unchanged refusal.
    pub committed: bool,
}

impl fmt::Display for HoldRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

fn refusal(reason: impl Into<String>) -> HoldRefusal {
    HoldRefusal {
        reason: reason.into(),
        committed: false,
    }
}

/// WRS-003: a failure after the journal published the action's commit
/// decision.
fn committed_refusal(detail: impl Into<String>) -> HoldRefusal {
    HoldRefusal {
        reason: detail.into(),
        committed: true,
    }
}

/// WRS-003: fold one reconciliation failure into the addressed command's
/// refusal family. An unparseable entry never modified anything; a
/// finishing failure leaves a committed action awaiting reconciliation.
///
/// WRS-003: whether one pending committed entry is this hold's own commit
/// (the same Run and blocker); the typed phrase was already checked before
/// the Run lock was taken.
fn adopted_is_hold(entry: &crate::pause::JournalEntry, run_id: &str, blocker: &str) -> bool {
    entry.action == crate::pause::HOLD_ACTION && entry.run == run_id && entry.blocker == blocker
}

fn pause_failure(failure: crate::pause::ReconcileFailure) -> HoldRefusal {
    HoldRefusal {
        reason: failure.detail,
        committed: failure.committed,
    }
}

/// The hold seam's one boundary wait, surfaced for the pause journal's
/// finishing order (WRS-003): the same marker, release, and timeout
/// discipline under both verbs, because the seam is hold's.
pub(crate) fn hold_barrier_at(boundary: &str) -> Result<(), String> {
    wait_before_hold_boundary_if_requested(boundary).map_err(|failure| failure.reason)
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
    // The seam is armed by its explicit timeout: an invocation that names a
    // boundary without one moves through it untouched, so a left-over
    // boundary name never slows an ordinary later command.
    if std::env::var("RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS").is_err() {
        return Ok(());
    }
    // The marker and its release are observability companions, not
    // preconditions: an invocation that sets only the boundary still waits
    // out its timeout at that boundary, reporting the release it waited
    // for (empty when none was named).
    let marker = std::env::var_os("RATMAC_TEST_HOLD_BARRIER_MARKER")
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    let release = std::env::var_os("RATMAC_TEST_HOLD_BARRIER_RELEASE")
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    if let Some(parent) = marker.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| {
                refusal(format!(
                    "hold test barrier cannot create marker directory {}: {error}",
                    parent.displayed()
                ))
            })?;
        }
    }
    // WRA-001: the authorization-archive barrier's marker lands whole in
    // one step, so an observer never sees it half-written.
    let marker_bytes = format!("holding before {boundary}\n");
    if marker.as_os_str().is_empty() {
        // No marker was named: the boundary still holds and waits.
    } else if boundary == "before-authorization-archive" {
        let temporary = marker.with_extension(format!(
            "tmp-{}-{}",
            std::process::id(),
            HOLD_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&temporary, marker_bytes)
            .and_then(|()| fs::rename(&temporary, &marker))
            .map_err(|error| {
                refusal(format!(
                    "hold test barrier cannot write marker {}: {error}",
                    marker.displayed()
                ))
            })?;
    } else {
        fs::write(&marker, marker_bytes).map_err(|error| {
            refusal(format!(
                "hold test barrier cannot write marker {}: {error}",
                marker.displayed()
            ))
        })?;
    }

    let deadline = Instant::now() + hold_barrier_timeout()?;
    loop {
        if release.is_file() {
            return Ok(());
        }
        let now = Instant::now();
        if now >= deadline {
            // The marker is the wait's own observability companion: an
            // expired wait leaves it behind, so the graceful refusal
            // leaves every durable byte untouched.
            if !marker.as_os_str().is_empty() {
                let _ = fs::remove_file(&marker);
            }
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
    /// WRA-001: the verified authorization a governed hold archives when it
    /// applies. `None` plans an ungoverned hold, which writes today's bytes
    /// and nothing more.
    authorization: Option<crate::review::VerifiedAuthorization>,
    /// The submitted options, re-verified under the Run lock at apply.
    options: crate::review::AuthorizationOptions,
    /// WRS-003: this invocation retried a hold whose committed journal
    /// entry was still pending, finished it under the plan's Run lock, and
    /// now owes no further apply: the action is complete exactly once.
    adopted: bool,
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

    pub(crate) fn options(&self) -> &crate::review::AuthorizationOptions {
        &self.options
    }
}

/// The exact phrase a human must type to hold the Run `run_id`.
pub fn confirmation_phrase(run_id: &str) -> String {
    format!("hold {run_id}")
}

/// The `p5-blocked` route predicate: verify a hold without writing anything.
/// The path-taking `plan_hold` lives in `src/root.rs`.
pub(crate) fn plan_hold_in(
    roots: &crate::root::Roots,
    request: &HoldRequest,
) -> Result<HoldPlan, HoldRefusal> {
    crate::Scheduler::refuse_flat_residue_with_roots(roots)
        .map_err(|error| refusal(error.to_string()))?;

    // FDC-004/FDC-005: hold is an existing-Run operation. Resolve one exact
    // canonical roster member through Scheduler::open_run so flat residue and
    // the recorded runbook pin are checked before this plan can permit a
    // mutation.
    let roster = crate::Scheduler::run_roster_with_roots(roots)
        .map_err(|error| refusal(error.to_string()))?;
    let Some(run_id) = request.run.as_deref().filter(|id| !id.is_empty()) else {
        let engine_root = roots.engine_root();
        return Err(refusal(format!(
            "hold requires --run <id>; runs: {}{}",
            crate::roster::summary(engine_root, &roster, "none"),
            crate::roster::rows(engine_root, &roster)
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

    let scheduler = crate::Scheduler::open_run_for_motion_with_roots(roots, run_id)
        .map_err(|error| refusal(error.to_string()))?;
    // WRS-001: the plan waits on the addressed Run's own claim before it
    // decides the Run terminal, so a peer completing a successor
    // publication that owns this claim is waited on exactly as a live peer
    // is. The plan's proof is still read-only; apply re-acquires and
    // revalidates everything below before any write.
    let engine_root = roots.engine_root().to_path_buf();
    let run_lock = crate::lock::RunLock::acquire(&engine_root, run_id)
        .map_err(|error| refusal(error.to_string()))?;
    run_lock
        .ensure_current()
        .map_err(|error| refusal(error.to_string()))?;
    // FDC-006: a roster entry without a Run Record is a retired run -
    // terminal, never resurrected - decided here under the held claim.
    let run_dir = engine_root.join("runs").join(run_id);
    if !run_dir.join("run.toml").is_file() {
        return Err(refusal(format!(
            "run {run_id} is terminal: its admission state is retired and no \
             transition may proceed on it; address a live run or mint a fresh \
             one with rtm start"
        )));
    }
    // WRS-003: the journal discipline. Under this lock a pending committed
    // entry from an interrupted hold or resume is finished exactly once
    // before any later mutation of the Run proceeds. A retried hold whose
    // own entry is still pending - the same Run, blocker, and typed phrase,
    // the phrase already checked above - adopts it: finishing the entry is
    // the whole action, so the plan answers success and apply writes
    // nothing.
    let mut adopted: Option<crate::pause::JournalEntry> = None;
    match crate::pause::reconcile(&run_dir) {
        Ok(Some(entry)) => {
            if adopted_is_hold(&entry, run_id, blocker) {
                adopted = Some(entry);
            }
        }
        Ok(None) => {}
        Err(failure) => return Err(pause_failure(failure)),
    }
    if let Some(entry) = &adopted {
        return Ok(HoldPlan {
            blocker: blocker.to_owned(),
            run_id: run_id.to_owned(),
            from_state: entry.state.clone(),
            from_status: crate::model::Status::Blocked,
            to_state: entry.to_state.clone(),
            authorization: None,
            options: crate::review::AuthorizationOptions::default(),
            adopted: true,
        });
    }
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
    let Some(to_state) = scheduler
        .blocked_destination(&from_state)
        .map_err(|error| refusal(error.to_string()))?
    else {
        return Err(refusal(format!(
            "State {from_state:?} declares no blocked route; add a transition with blocked-route = true"
        )));
    };

    Ok(HoldPlan {
        blocker: blocker.to_owned(),
        run_id: run_id.to_owned(),
        from_state,
        from_status: state.status,
        to_state,
        authorization: None,
        options: crate::review::AuthorizationOptions::default(),
        adopted: false,
    })
}

/// WRA-001: plan a governed hold - [`plan_hold_in`]'s whole proof, then the
/// authorization boundary over the planned blocked route. The authorization
/// rides the plan; `apply` re-verifies it under the Run lock.
pub(crate) fn plan_authorized_hold_in(
    roots: &crate::root::Roots,
    request: &HoldRequest,
    authorization: &crate::review::AuthorizationOptions,
) -> Result<HoldPlan, HoldRefusal> {
    let mut plan = plan_hold_in(roots, request)?;
    // A retried hold that adopted its pending committed entry is already
    // complete: its authorization was archived by the reconciliation, so
    // no second verification runs.
    if plan.adopted {
        return Ok(plan);
    }
    // The full verification boundary of `rtm step` runs here, after the
    // plan's own checks; `apply` re-verifies under the Run lock before the
    // first write. Both verifications re-read the archive and evidence the
    // plan already observed, so each runs in a record-quiet scope: the
    // hold's one extra record mark is the landing authorization archive.
    let _quiet = crate::observe::quiet_records();
    let scheduler = crate::Scheduler::open_run_for_motion_with_roots(roots, plan.run_id())
        .map_err(|error| refusal(error.to_string()))?;
    let engine_root = roots.engine_root().to_path_buf();
    let run_dir = engine_root.join("runs").join(plan.run_id());
    let governed = scheduler
        .governed_review_of(&run_dir, plan.run_id())
        .map_err(|error| refusal(error.to_string()))?;
    let Some(declaration) = governed else {
        return Ok(plan);
    };
    if authorization.is_empty() {
        let minimum = declaration.minimum_reviews();
        return Err(refusal(format!(
            "authorization: signatures: observed none; expected {minimum} distinct review \
             signature(s) and one approval signature over the canonical intent (rtm intent \
             --run {})",
            plan.run_id()
        )));
    }
    let verified = scheduler
        .authorize_motion(
            &engine_root,
            &run_dir,
            plan.run_id(),
            &declaration,
            "hold",
            &plan.from_state,
            &plan.to_state,
            "",
            plan.blocker(),
            scheduler.runbook_pin_of(),
            authorization,
        )
        .map_err(|failure| refusal(failure.to_string()))?;
    drop(_quiet);
    plan.authorization = Some(verified);
    plan.options = authorization.clone();
    Ok(plan)
}

/// NRR-001: the blocker is an opaque reference. The Engine checks that it
/// exists and that it resolves beneath a root the runbook declares - never
/// what kind of record it is, which is the shop's rule and not the Engine's.
fn verify_blocker(scheduler: &crate::Scheduler, blocker: &str) -> Result<(), HoldRefusal> {
    // WEBV-014: looking up a hold's blocker reference is a blocker
    // operation.
    crate::observe::operation("blocker");
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
/// Apply a verified hold through the operation journal (WRS-003): every
/// output is prepared before publication, one committed entry published
/// whole in a single atomic step is the action's commit decision, and the
/// outputs are then finished idempotently and in the contract's order. A
/// graceful failure before publication leaves every persisted byte
/// unchanged; after publication it never rolls back and reports the
/// committed action as awaiting reconciliation.
///
/// Everything the hold changes lives under the Engine root and belongs to one
/// Run, so the addressed Run lock alone serializes it - there is no shared
/// document to read-modify-write any more (NRR-001). The exact planned state
/// is compared after acquisition.
/// The path-taking `apply_hold` lives in `src/root.rs`.
pub(crate) fn apply_hold_in(
    roots: &crate::root::Roots,
    plan: &HoldPlan,
) -> Result<(), HoldRefusal> {
    // A retried hold whose plan adopted the pending committed entry is
    // already finished exactly once.
    if plan.adopted {
        return Ok(());
    }
    // Planning and application are separate public boundaries. Resolve from
    // the addressed Run's workspace again before this path can mutate state.
    let scheduler = crate::Scheduler::open_run_with_roots(roots, &plan.run_id)
        .map_err(|error| refusal(error.to_string()))?;
    verify_blocker(&scheduler, &plan.blocker)?;

    let engine_root = roots.engine_root().to_path_buf();
    let run_dir = engine_root.join("runs").join(&plan.run_id);
    let state_path = run_dir.join("run.toml");

    let run_lock = crate::lock::RunLock::acquire(&engine_root, &plan.run_id)
        .map_err(|error| refusal(error.to_string()))?;
    run_lock
        .ensure_current()
        .map_err(|error| refusal(error.to_string()))?;

    // WRS-003: reconcile any pending committed entry under this lock before
    // this hold's own checks. A matching pending entry - the same Run and
    // blocker, the phrase already carried by this plan - is this hold's own
    // commit: finish it and report the ordinary success.
    let mut adopted = false;
    match crate::pause::reconcile(&run_dir) {
        Ok(Some(entry)) => {
            adopted = entry.action == crate::pause::HOLD_ACTION
                && entry.run == plan.run_id
                && entry.blocker == plan.blocker;
        }
        Ok(None) => {}
        Err(failure) => return Err(pause_failure(failure)),
    }
    if adopted {
        return Ok(());
    }

    crate::observe::operation("record");
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
    // the destination in a caller-supplied HoldPlan.
    let current_scheduler = crate::Scheduler::open_run_with_roots(roots, &plan.run_id)
        .map_err(|error| refusal(error.to_string()))?;
    let Some(to_state) = current_scheduler
        .blocked_destination(&state.state)
        .map_err(|error| refusal(error.to_string()))?
    else {
        return Err(refusal(format!(
            "hold route changed: state {:?} no longer declares a blocked route; re-plan the hold",
            state.state
        )));
    };
    if to_state != plan.to_state {
        return Err(refusal(format!(
            "hold route changed: planned state {:?} -> {:?}, but the declared blocked route is {:?} -> {:?}; re-plan the hold",
            plan.from_state,
            plan.to_state,
            state.state,
            to_state
        )));
    }

    // WRA-001: a governed hold re-verifies its whole authorization under
    // the Run lock this apply owns. The archive record is prepared here and
    // written only inside the journal's finishing order, after publication.
    let mut authorization: Option<(u64, Vec<u8>)> = None;
    if let Some(declaration) = current_scheduler
        .governed_review_of(&run_dir, &plan.run_id)
        .map_err(|error| refusal(error.to_string()))?
    {
        let verified = {
            let _verify_quiet = crate::observe::quiet_records();
            current_scheduler
                .authorize_motion(
                    &engine_root,
                    &run_dir,
                    &plan.run_id,
                    &declaration,
                    "hold",
                    &plan.from_state,
                    &plan.to_state,
                    "",
                    plan.blocker(),
                    current_scheduler.runbook_pin_of(),
                    plan.options(),
                )
                .map_err(|failure| refusal(failure.to_string()))?
        };
        let record = crate::review::authorization_record_text(
            &verified,
            "hold",
            &plan.from_state,
            &plan.to_state,
        );
        authorization = Some((verified.sequence, record.into_bytes()));
    }

    // Snapshot the pre-existing append target through the Scheduler before
    // committing state: an unusable or missing log refuses without a
    // half-motion.
    let _log = crate::Scheduler::open_transition_log(&engine_root, false).map_err(|error| {
        refusal(format!(
            "hold cannot open history: {error}; nothing was modified"
        ))
    })?;

    // WRS-003: the durable facts a committed hold leaves. The occurrence is
    // decided under this lock before publication; the record seals its own
    // held-context digest.
    let occurrence = crate::pause::next_occurrence(&run_dir, &plan.run_id).map_err(refusal)?;
    let context = current_scheduler
        .pause_context()
        .map_err(|error| refusal(error.to_string()))?;
    let pause_record = crate::pause::pause_record_bytes(
        &plan.run_id,
        &plan.to_state,
        plan.blocker(),
        &context,
        occurrence,
    );

    state.state = plan.to_state.clone();
    // The whole held fact, in Engine-owned state: the paused mark and the
    // opaque reference the human named (NRR-001).
    state.status = crate::model::Status::Blocked;
    state.blocker = plan.blocker.clone();
    let new_state_bytes = crate::state::StateStore::serialize(&state)
        .map_err(|error| refusal(format!("hold cannot write state: {error}")))?;
    let history_line = format!(
        "- Hold: run {} paused against {}; routed {} -> {} on an explicit human confirmation.\n",
        plan.run_id, plan.blocker, plan.from_state, plan.to_state
    );
    let entry = crate::pause::JournalEntry {
        action: crate::pause::HOLD_ACTION.to_owned(),
        run: plan.run_id.clone(),
        occurrence,
        blocker: plan.blocker.clone(),
        state: plan.from_state.clone(),
        to_state: plan.to_state.clone(),
        old_record: old_state_bytes,
        new_record: new_state_bytes,
        pause_record: Some(pause_record),
        authorization_record: authorization,
        history_line,
    };

    run_lock
        .ensure_current()
        .map_err(|error| refusal(error.to_string()))?;
    wait_before_hold_boundary_if_requested("before-state-write")?;
    if let Err(error) = current_scheduler.verify_open_runbook_snapshot() {
        return Err(refusal(error.to_string()));
    }
    wait_before_hold_boundary_if_requested("before-journal-publication")?;

    // The commit decision: one whole committed entry, published atomically.
    // Every failure past this point reports the committed action honestly.
    crate::pause::publish_entry(&run_dir, &entry).map_err(refusal)?;
    wait_before_hold_boundary_if_requested("after-journal-publication")
        .map_err(|error| committed_refusal(error.reason))?;
    crate::pause::finish(&run_dir, &entry, true).map_err(committed_refusal)?;
    Ok(())
}

// --- WRS-003: `rtm resume` ------------------------------------------------------

/// Why a resume cannot be applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeRefusal {
    pub reason: String,
    /// WRS-003: the operation already committed through the journal; the
    /// failure renders as recovery-pending, never as an unchanged refusal.
    pub committed: bool,
}

impl fmt::Display for ResumeRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

fn resume_refusal(reason: impl Into<String>) -> ResumeRefusal {
    ResumeRefusal {
        reason: reason.into(),
        committed: false,
    }
}

/// Fold one reconciliation failure into the resume family's refusals.
fn resume_pause_failure(failure: crate::pause::ReconcileFailure) -> ResumeRefusal {
    ResumeRefusal {
        reason: failure.detail,
        committed: failure.committed,
    }
}

/// What a human asked for when resuming.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeRequest {
    /// The recorded blocker, given verbatim: the Engine compares it and
    /// never consults the referenced path on disk.
    pub blocker: Option<String>,
    pub confirmation: Option<String>,
    /// The addressed run (FDC-004: `--run <id>`, always required).
    pub run: Option<String>,
    /// WRS-003: one detached `ratmac-resume-v1` signature file, only under
    /// a declared review policy.
    pub resolution: Option<String>,
}

/// A verified resume, ready to apply.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumePlan {
    run_id: String,
    blocker: String,
    held_state: String,
    occurrence: u64,
    resolution: Option<String>,
    /// WRS-003: this invocation retried a resume whose committed journal
    /// entry was still pending, finished it under the plan's Run lock, and
    /// now owes no further apply.
    adopted: bool,
}

impl ResumePlan {
    pub(crate) fn run_id(&self) -> &str {
        &self.run_id
    }

    pub(crate) fn blocker(&self) -> &str {
        &self.blocker
    }

    pub(crate) fn occurrence(&self) -> u64 {
        self.occurrence
    }
}

/// The exact phrase a human must type to resume the Run `run_id`.
pub fn resume_confirmation_phrase(run_id: &str) -> String {
    format!("resume {run_id}")
}

/// The governed resume's one-signature boundary rendered as one refusal
/// line: `authorization: <sub-boundary>: observed <observed>; expected
/// <expected>`.
fn resolution_refusal_text(refusal: &crate::review::AuthorizationRefusal) -> String {
    format!(
        "authorization: {}: observed {}; expected {}",
        refusal.kind, refusal.observed, refusal.expected
    )
}

/// Verify one governed resume's resolution signature under the Run lock:
/// the canonical resolution intent recomputed from the Engine's own
/// records, one `ratmac-resume-v1` signature by the designated Approver.
#[allow(clippy::too_many_arguments)]
fn verify_resolution_of(
    scheduler: &crate::Scheduler,
    engine_root: &Path,
    _run_dir: &Path,
    run_id: &str,
    declaration: &crate::review::ReviewDeclaration,
    held_state: &str,
    blocker: &str,
    occurrence: u64,
    held_record_digest: &str,
    resolution: &str,
) -> Result<crate::review::VerifiedResolution, ResumeRefusal> {
    let _quiet = crate::observe::quiet_records();
    let enrollment = crate::review::read_enrollment(
        engine_root,
        declaration,
        &crate::Scheduler::project_scope(engine_root),
    )
    .map_err(resume_refusal)?;
    let facts = scheduler
        .resolution_facts(
            engine_root,
            run_id,
            declaration,
            &enrollment,
            held_state,
            blocker,
            occurrence,
            held_record_digest,
        )
        .map_err(resume_refusal)?;
    crate::review::verify_resolution(
        declaration,
        &enrollment,
        &crate::Scheduler::project_scope(engine_root),
        &facts,
        occurrence,
        resolution,
    )
    .map_err(|failure| resume_refusal(resolution_refusal_text(&failure)))
}

/// WRS-003: verify a resume without writing anything. The path-taking
/// `plan_resume` lives in `src/root.rs`.
pub(crate) fn plan_resume_in(
    roots: &crate::root::Roots,
    request: &ResumeRequest,
) -> Result<ResumePlan, ResumeRefusal> {
    crate::Scheduler::refuse_flat_residue_with_roots(roots)
        .map_err(|error| resume_refusal(error.to_string()))?;

    let roster = crate::Scheduler::run_roster_with_roots(roots)
        .map_err(|error| resume_refusal(error.to_string()))?;
    let Some(run_id) = request.run.as_deref().filter(|id| !id.is_empty()) else {
        let engine_root = roots.engine_root();
        return Err(resume_refusal(format!(
            "resume requires --run <id>; runs: {}{}",
            crate::roster::summary(engine_root, &roster, "none"),
            crate::roster::rows(engine_root, &roster)
        )));
    };

    let required = resume_confirmation_phrase(run_id);
    match request.confirmation.as_deref().map(str::trim) {
        None => {
            return Err(resume_refusal(format!(
                "resume is unconfirmed: a human must type --confirm {required:?}"
            )))
        }
        Some(phrase) if phrase != required => {
            return Err(resume_refusal(format!(
                "resume is unconfirmed: confirmation {phrase:?} does not match the required phrase {required:?}"
            )))
        }
        Some(_) => {}
    }

    let Some(blocker) = request.blocker.as_deref().map(str::trim) else {
        return Err(resume_refusal(
            "resume has no blocker link: pass --blocker <reference beneath a declared root>",
        ));
    };
    if blocker.is_empty() {
        return Err(resume_refusal(
            "resume has no blocker link: pass --blocker <reference beneath a declared root>",
        ));
    }

    // The open-run family - runbook parse and pin, class resolution in the
    // Run's recorded owning class, record parse - with today's texts.
    let scheduler = crate::Scheduler::open_run_for_motion_with_roots(roots, run_id)
        .map_err(|error| resume_refusal(error.to_string()))?;
    let engine_root = roots.engine_root().to_path_buf();
    let run_dir = engine_root.join("runs").join(run_id);
    // The family's tail resolves the recorded State in the Run's recorded
    // owning class by the t-114 rule, so an undeclared recorded State
    // refuses here - before the Run lock and before any pause fact is
    // read. The route itself is hold's question; resume only borrows the
    // class-resolution refusal.
    let open_state = scheduler
        .load_state()
        .map_err(|error| resume_refusal(format!("resume requires an active Run: {error}")))?;
    scheduler
        .blocked_destination(&open_state.state)
        .map_err(|error| resume_refusal(error.to_string()))?;

    // The addressed Run lock: the only lock resume ever takes, acquired
    // immediately after the open-run family, exactly where today's hold
    // plan acquires it. Every later check, including the governed
    // boundary, runs under it.
    let run_lock = crate::lock::RunLock::acquire(&engine_root, run_id)
        .map_err(|error| resume_refusal(error.to_string()))?;
    run_lock
        .ensure_current()
        .map_err(|error| resume_refusal(error.to_string()))?;
    // FDC-006: a roster entry without a Run Record is a retired run.
    if !run_dir.join("run.toml").is_file() {
        return Err(resume_refusal(format!(
            "run {run_id} is terminal: its admission state is retired and no \
             transition may proceed on it; address a live run or mint a fresh \
             one with rtm start"
        )));
    }

    // WRS-003: the journal discipline, exactly as the hold plan keeps it. A
    // retried resume whose own entry is still pending - the same Run,
    // blocker, and occurrence - adopts it; any other pending entry is
    // finished first.
    match crate::pause::reconcile(&run_dir) {
        Ok(Some(entry))
            if entry.action == crate::pause::RESUME_ACTION
                && entry.run == run_id
                && entry.blocker == blocker
                && crate::pause::live_occurrence(&run_dir) == Ok(Some(entry.occurrence)) =>
        {
            return Ok(ResumePlan {
                run_id: run_id.to_owned(),
                blocker: entry.blocker.clone(),
                held_state: entry.state.clone(),
                occurrence: entry.occurrence,
                resolution: None,
                adopted: true,
            });
        }
        Ok(_) => {}
        Err(failure) => return Err(resume_pause_failure(failure)),
    }

    let state = scheduler
        .load_state()
        .map_err(|error| resume_refusal(format!("resume requires an active Run: {error}")))?;
    if state.status == crate::model::Status::Passed {
        return Err(resume_refusal(format!(
            "run {run_id} is terminal (status passed): nothing to resume"
        )));
    }
    if state.status != crate::model::Status::Blocked {
        return Err(resume_refusal(format!(
            "run {run_id} is not held (status {}); nothing to resume",
            state.status
        )));
    }
    if crate::pause::live_occurrence(&run_dir)
        .map_err(resume_refusal)?
        .is_none()
    {
        return Err(resume_refusal(format!(
            "run {run_id} is held with no recorded pause occurrence; a hold from before \
             occurrences were recorded admits no resume"
        )));
    }
    if blocker != state.blocker {
        return Err(resume_refusal(format!(
            "resume blocker {blocker} does not match the blocker recorded for run {run_id}: {}",
            state.blocker
        )));
    }
    let (occurrence, record) = crate::pause::load_live_record(&run_dir).map_err(resume_refusal)?;
    if record.run != run_id || record.state != state.state || record.blocker != state.blocker {
        let path = crate::pause::pauses_path(&run_dir, occurrence);
        return Err(resume_refusal(format!(
            "pause record {} is not a strict version-1 record: its recorded facts differ \
             from the live Run Record",
            path.displayed()
        )));
    }
    let held_record_digest = record.held_context_sha256.clone();

    // WRS-003: the governed resume runs t-129's enrollment boundary with
    // its shared detail strings, then verifies one resolution signature.
    if let Some(declaration) = scheduler
        .governed_review_of(&run_dir, run_id)
        .map_err(|error| resume_refusal(error.to_string()))?
    {
        let Some(resolution) = request.resolution.as_deref() else {
            return Err(resume_refusal(format!(
                "authorization: resolution: observed none; expected one \
                 ratmac-resume-v1 signature by the designated approver over the canonical \
                 intent (rtm intent --run {run_id} --resume)"
            )));
        };
        verify_resolution_of(
            &scheduler,
            &engine_root,
            &run_dir,
            run_id,
            &declaration,
            &state.state,
            &state.blocker,
            occurrence,
            &held_record_digest,
            resolution,
        )?;
    }

    Ok(ResumePlan {
        run_id: run_id.to_owned(),
        blocker: state.blocker.clone(),
        held_state: state.state.clone(),
        occurrence,
        resolution: request.resolution.clone(),
        adopted: false,
    })
}

/// WRS-003: apply a verified resume through the operation journal, exactly
/// as a hold applies. The path-taking `apply_resume` lives in `src/root.rs`.
pub(crate) fn apply_resume_in(
    roots: &crate::root::Roots,
    plan: &ResumePlan,
) -> Result<(), ResumeRefusal> {
    // A retried resume whose plan adopted the pending committed entry is
    // already finished exactly once.
    if plan.adopted {
        return Ok(());
    }
    let scheduler = crate::Scheduler::open_run_with_roots(roots, &plan.run_id)
        .map_err(|error| resume_refusal(error.to_string()))?;

    let engine_root = roots.engine_root().to_path_buf();
    let run_dir = engine_root.join("runs").join(&plan.run_id);
    let state_path = run_dir.join("run.toml");

    let run_lock = crate::lock::RunLock::acquire(&engine_root, &plan.run_id)
        .map_err(|error| resume_refusal(error.to_string()))?;
    run_lock
        .ensure_current()
        .map_err(|error| resume_refusal(error.to_string()))?;

    // WRS-003: reconcile under this lock first; a matching pending entry is
    // this resume's own commit.
    let mut adopted = false;
    match crate::pause::reconcile(&run_dir) {
        Ok(Some(entry)) => {
            adopted = entry.action == crate::pause::RESUME_ACTION
                && entry.run == plan.run_id
                && entry.blocker == plan.blocker
                && entry.occurrence == plan.occurrence;
        }
        Ok(None) => {}
        Err(failure) => return Err(resume_pause_failure(failure)),
    }
    if adopted {
        return Ok(());
    }

    crate::observe::operation("record");
    let old_state_bytes = fs::read(&state_path)
        .map_err(|error| resume_refusal(format!("resume cannot read state: {error}")))?;
    let store = crate::state::StateStore::for_engine_root(&engine_root, &plan.run_id);
    let mut state = store
        .load()
        .map_err(|error| resume_refusal(format!("resume cannot read state: {error}")))?;
    // The apply-time comparison: the Run Record's State, status, blocker,
    // and live occurrence must still match what was verified.
    let live = crate::pause::live_occurrence(&run_dir).map_err(resume_refusal)?;
    if state.state != plan.held_state
        || state.status != crate::model::Status::Blocked
        || state.blocker != plan.blocker
        || live != Some(plan.occurrence)
    {
        let observed_blocker = if state.blocker.is_empty() {
            "an unnamed blocker"
        } else {
            state.blocker.as_str()
        };
        let observed_occurrence = live
            .map(|occurrence| occurrence.to_string())
            .unwrap_or_else(|| "none".to_owned());
        return Err(resume_refusal(format!(
            "resume plan is stale for run {}: expected state {:?} with status blocked against {} \
             at occurrence {}, found state {:?} with status {} against {} at occurrence {}; \
             nothing was modified",
            plan.run_id,
            plan.held_state,
            plan.blocker,
            plan.occurrence,
            state.state,
            state.status,
            observed_blocker,
            observed_occurrence
        )));
    }
    let (occurrence, record) = crate::pause::load_live_record(&run_dir).map_err(resume_refusal)?;
    let held_record_digest = record.held_context_sha256.clone();

    // WRA-001: the governed boundary re-verifies under the Run lock this
    // apply owns; the resolution record is prepared and archived only
    // inside the journal's finishing order.
    let mut authorization: Option<(u64, Vec<u8>)> = None;
    if let Some(declaration) = scheduler
        .governed_review_of(&run_dir, &plan.run_id)
        .map_err(|error| resume_refusal(error.to_string()))?
    {
        let Some(resolution) = plan.resolution.as_deref() else {
            return Err(resume_refusal(format!(
                "authorization: resolution: observed none; expected one \
                 ratmac-resume-v1 signature by the designated approver over the canonical \
                 intent (rtm intent --run {run_id} --resume)",
                run_id = plan.run_id
            )));
        };
        let verified = verify_resolution_of(
            &scheduler,
            &engine_root,
            &run_dir,
            &plan.run_id,
            &declaration,
            &plan.held_state,
            &plan.blocker,
            plan.occurrence,
            &held_record_digest,
            resolution,
        )?;
        let record =
            crate::review::resolution_record_text(&verified, &plan.held_state, &plan.held_state);
        authorization = Some((verified.sequence, record.into_bytes()));
    }

    // The history preflight keeps its position before publication.
    let _log = crate::Scheduler::open_transition_log(&engine_root, false).map_err(|error| {
        resume_refusal(format!(
            "resume cannot open history: {error}; nothing was modified"
        ))
    })?;

    state.status = crate::model::Status::Executing;
    state.blocker = String::new();
    let new_state_bytes = crate::state::StateStore::serialize(&state)
        .map_err(|error| resume_refusal(format!("resume cannot write state: {error}")))?;
    let history_line = format!(
        "- Resume: run {} resumed against {}; occurrence {} cleared on an explicit human \
         resolution authorization.\n",
        plan.run_id, plan.blocker, occurrence
    );
    let entry = crate::pause::JournalEntry {
        action: crate::pause::RESUME_ACTION.to_owned(),
        run: plan.run_id.clone(),
        occurrence: plan.occurrence,
        blocker: plan.blocker.clone(),
        state: plan.held_state.clone(),
        to_state: plan.held_state.clone(),
        old_record: old_state_bytes,
        new_record: new_state_bytes,
        pause_record: None,
        authorization_record: authorization,
        history_line,
    };

    run_lock
        .ensure_current()
        .map_err(|error| resume_refusal(error.to_string()))?;
    // WRS-003: the boundary set serves resume identically: the runbook
    // snapshot re-verifies under this lock after `before-state-write`, so a
    // runbook swapped across it refuses having written nothing and a
    // release still completes the resume.
    wait_before_hold_boundary_if_requested("before-state-write")
        .map_err(|error| resume_refusal(error.reason))?;
    if let Err(error) = scheduler.verify_open_runbook_snapshot() {
        return Err(resume_refusal(error.to_string()));
    }
    wait_before_hold_boundary_if_requested("before-journal-publication")
        .map_err(|error| resume_refusal(error.reason))?;

    // The commit decision.
    crate::pause::publish_entry(&run_dir, &entry).map_err(resume_refusal)?;
    wait_before_hold_boundary_if_requested("after-journal-publication").map_err(|error| {
        ResumeRefusal {
            reason: error.reason,
            committed: true,
        }
    })?;
    crate::pause::finish(&run_dir, &entry, true).map_err(|detail| ResumeRefusal {
        reason: detail,
        committed: true,
    })?;
    Ok(())
}

/// discipline for exact Run Record rollback. Once the rename
/// succeeds, a parent-sync failure is reported as a warning rather than as an
/// untouched destination: the replacement has already happened.
pub(crate) fn replace_file_atomically(
    path: &Path,
    bytes: &[u8],
) -> std::io::Result<ReplaceFileOutcome> {
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
