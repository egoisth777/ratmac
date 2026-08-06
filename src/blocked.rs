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
//! Anything else refuses before the first write, so Scheduler-owned files stay
//! byte-identical. The apply step is all-or-nothing: every file it touches is
//! restored if any write fails, leaving the Run pre-route rather than half
//! routed.
//!
//! The ticket stays not-passed and its residuals untouched: holding a ticket
//! proves nothing about the work.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract::ISSUE_FILES;

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
    pub from_phase: String,
    pub to_phase: String,
}

/// The exact phrase a human must type to hold `ticket`.
pub fn confirmation_phrase(ticket: &str) -> String {
    format!("hold {ticket}")
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

    let ticket_path = root.join(".arca/ticket").join(format!("{ticket}.md"));
    let source = fs::read_to_string(&ticket_path).map_err(|error| {
        refusal(format!(
            "hold refers to no ticket: .arca/ticket/{ticket}.md is unreadable ({error})"
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
/// All or nothing. Every file touched is snapshotted first and restored if any
/// write fails, so an interrupted hold leaves the Run pre-route.
pub fn apply_hold(root: &Path, plan: &HoldPlan) -> Result<(), HoldRefusal> {
    let engine_root = crate::root::resolve(root).engine_root().to_path_buf();
    let _run_lock = crate::lock::RunLock::acquire(&engine_root, &plan.run_id)
        .map_err(|error| refusal(error.to_string()))?;
    let state_path = crate::Scheduler::runs_dir(root)
        .join(&plan.run_id)
        .join("state.toml");
    let log_path = crate::root::resolve(root).engine_root().join("log.md");
    let touched = [
        state_path.clone(),
        log_path.clone(),
        plan.ticket_path.clone(),
    ];
    // An unreadable file is refused before the first write: treating it as
    // absent would let rollback delete the very file it claims to protect.
    let mut snapshot: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::new();
    for path in &touched {
        match fs::read(path) {
            Ok(bytes) => snapshot.push((path.clone(), Some(bytes))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                snapshot.push((path.clone(), None))
            }
            Err(error) => {
                return Err(refusal(format!(
                    "hold cannot snapshot {} for rollback: {error}",
                    path.display()
                )))
            }
        }
    }

    let restore = |problem: HoldRefusal| -> HoldRefusal {
        let mut unrestored = Vec::new();
        for (path, bytes) in &snapshot {
            let outcome = match bytes {
                Some(bytes) => fs::write(path, bytes),
                None => match fs::remove_file(path) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    other => other,
                },
            };
            if let Err(error) = outcome {
                unrestored.push(format!("{}: {error}", path.display()));
            }
        }
        if unrestored.is_empty() {
            problem
        } else {
            refusal(format!(
                "{}; rollback incomplete, restore by hand: {}",
                problem.reason,
                unrestored.join(", ")
            ))
        }
    };

    let store = crate::state::StateStore::for_run(root, &plan.run_id);
    let mut state = match store.load() {
        Ok(state) => state,
        Err(error) => return Err(restore(refusal(format!("hold cannot read state: {error}")))),
    };
    state.phase = plan.to_phase.clone();
    if let Err(error) = store.write(&state) {
        return Err(restore(refusal(format!(
            "hold cannot write state: {error}"
        ))));
    }

    let entry = format!(
        "- Hold: ticket {} held against {}; Run routed {} -> {} on an explicit human confirmation. The ticket is not passed and its residuals stay unproven.\n",
        plan.ticket, plan.blocker, plan.from_phase, plan.to_phase
    );
    if let Err(error) = append(&log_path, &entry) {
        return Err(restore(refusal(format!(
            "hold cannot append history: {error}"
        ))));
    }

    let source = match fs::read_to_string(&plan.ticket_path) {
        Ok(source) => source,
        Err(error) => {
            return Err(restore(refusal(format!(
                "hold cannot read the ticket: {error}"
            ))))
        }
    };
    let held = hold_ticket(&source, &plan.blocker);
    if let Err(error) = fs::write(&plan.ticket_path, held) {
        return Err(restore(refusal(format!(
            "hold cannot write the ticket: {error}"
        ))));
    }
    Ok(())
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

fn append(path: &Path, entry: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(entry.as_bytes())?;
    file.flush()
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
