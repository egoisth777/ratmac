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
//! 2. retires the admission state (`.arca/runs/<id>/state.toml`) so a fresh
//!    Run can start;
//! 3. retires the Run-scoped evidence (`.arca/runs/<id>/evidence.toml`) so
//!    the next Run records its own baseline and pins rather than inheriting
//!    them;
//! 4. retires the invocation lock (`.arca/rtm.lock`) - retired through this
//!    path, never bypassed by a flag.
//!
//! Every check runs before the first write, so an unconfirmed request leaves
//! state, history, and lock byte-identical. The apply step is all-or-nothing:
//! if any step fails, every file it touched is restored, leaving the Run
//! active rather than half retired. Re-running the confirmed command then
//! finishes the job.
//!
//! Retirement is idempotent: a leftover lock with no admission state is
//! retired without appending a second terminal event, because the lock is
//! transient invocation machinery and its removal is not Run history.

use std::fmt;
use std::fs;
use std::io::Write;
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

    let arca = root.join(".arca");
    let lock_path = arca.join("rtm.lock");

    // FDC-004: abandon acts on an existing Run through `--run <id>`. Only the
    // leftover-lock retirement — no live run anywhere on the roster — may
    // proceed unaddressed, because it retires transient invocation machinery,
    // not a Run.
    let roster = crate::Scheduler::run_roster(root);
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

    let run_dir = run_id
        .as_deref()
        .map(|id| crate::Scheduler::runs_dir(root).join(id));
    let state_path = run_dir.as_ref().map(|dir| dir.join("state.toml"));
    let evidence_path = run_dir
        .as_ref()
        .map(|dir| dir.join(crate::pin::EVIDENCE_FILE));

    let admitted = state_path.as_ref().is_some_and(|path| path.exists());
    if !admitted && !lock_path.exists() {
        // FDC-006: runs are plural, so the refusal speaks about the addressed
        // run — or the empty project — never about "the" project-wide Run.
        return Err(match run_id.as_deref() {
            Some(id) => refusal(format!(
                "run {id} is already terminal: its admission state is retired; nothing to retire"
            )),
            None => refusal(format!(
                "nothing to retire in {}: no live run and no leftover lock",
                project_name(root)
            )),
        });
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
        retire.push(state_path);
        if evidence_path.exists() {
            retire.push(evidence_path);
        }
    }
    if lock_path.exists() {
        retire.push(lock_path);
    }

    Ok(AbandonPlan {
        event,
        phase,
        retire,
    })
}

fn revision_or_none(revision: &str) -> String {
    if revision.trim().is_empty() {
        "none".to_owned()
    } else {
        revision.to_owned()
    }
}

/// Perform the planned retirement, all of it or none of it.
pub fn apply_abandon(root: &Path, plan: &AbandonPlan) -> Result<(), AbandonRefusal> {
    let log_path = root.join(".arca/log.md");
    let lock_path = root.join(".arca/rtm.lock");

    // Snapshot every file whose bytes rollback must be able to put back. The
    // lock is excluded deliberately: it is transient invocation machinery,
    // retired last, with no content worth restoring. An unreadable Run file is
    // refused here, before the first write, rather than silently treated as
    // absent - restoring "absent" would delete the very file it claims to
    // protect.
    let mut snapshot: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::new();
    for path in std::iter::once(&log_path).chain(plan.retire.iter().filter(|p| **p != lock_path)) {
        match fs::read(path) {
            Ok(bytes) => snapshot.push((path.clone(), Some(bytes))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                snapshot.push((path.clone(), None))
            }
            Err(error) => {
                return Err(refusal(format!(
                    "abandon cannot snapshot {} for rollback: {error}",
                    path.display()
                )))
            }
        }
    }

    let restore = |problem: AbandonRefusal| -> AbandonRefusal {
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
            // Never report a clean refusal over a tree we could not put back.
            refusal(format!(
                "{}; rollback incomplete, restore by hand: {}",
                problem.reason,
                unrestored.join(", ")
            ))
        }
    };

    // The terminal event is recorded before anything is retired, so history
    // can never lose a Run that the filesystem has already forgotten.
    if let Some(entry) = plan.event.as_deref() {
        if let Err(error) = append(&log_path, entry) {
            return Err(restore(refusal(format!(
                "abandon cannot record the terminal event: {error}"
            ))));
        }
    }

    for path in &plan.retire {
        if let Err(error) = fs::remove_file(path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(restore(refusal(format!(
                    "abandon cannot retire {}: {error}",
                    path.display()
                ))));
            }
        }
    }
    Ok(())
}

fn append(path: &Path, entry: &str) -> std::io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(entry.as_bytes())?;
    file.sync_all()
}
