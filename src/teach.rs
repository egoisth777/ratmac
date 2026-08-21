//! AOP-002: the engine's own next-act table.
//!
//! Every status/step rendering path ends in exactly one truthful `next:`
//! line naming a command the engine dispatches and accepts in the state that
//! rendered it, or deliberately omits the line where nothing can be stood
//! behind - a terminal Run. The lines are generated here from the engine's
//! own facts - the addressed Run's recorded status, the refusal failure
//! kinds, the run roster - never hand-kept prose, so no rendering can invent
//! a command the engine would refuse.

use crate::model::Status;
use crate::scheduler::{Scheduler, StepOutcome};
use crate::state::{StateError, StateStore};
use std::path::Path;

/// The prefix every taught line carries.
pub(crate) const PREFIX: &str = "next: ";

/// The next act for a rendered Run status: the loop's one motion verb while
/// the Run lives, and a deliberate omission once it is terminal - nothing
/// legitimate remains to teach a finished Run.
pub(crate) fn status_next(run_id: &str, status: Status) -> Option<String> {
    match status {
        Status::Passed | Status::Failed => None,
        _ => Some(format!("{PREFIX}rtm step --run {run_id}")),
    }
}

/// The next act for a step refusal, keyed by the first failure's stable kind.
/// Every guard, verdict, and transition refusal keeps `rtm step` the repair:
/// R-020 makes the step idempotent under failure, safe to re-run once the
/// named observed/expected gap is closed. The terminal refusal is the
/// exception - its Run refuses motion forever, so the one act left on it is
/// reading its record.
pub(crate) fn step_refusal_next(run_id: &str, outcome: &StepOutcome) -> Option<String> {
    let StepOutcome::Refused { failures } = outcome else {
        return None;
    };
    match failures.first().map(|failure| failure.kind.as_str()) {
        Some("terminal") => Some(format!("{PREFIX}rtm status --run {run_id}")),
        _ => Some(format!("{PREFIX}rtm step --run {run_id}")),
    }
}

/// The next act for a run-addressing refusal (unknown id, missing or
/// ill-formed `--run`): address a Run the roster actually lists - the first
/// one whose Run Record reads - or mint one when none does. The refused id is
/// never named back.
pub(crate) fn addressing_next(engine_root: &Path) -> String {
    for run_id in Scheduler::run_roster_at(engine_root) {
        if StateStore::for_engine_root(engine_root, &run_id)
            .load()
            .is_ok()
        {
            return format!("{PREFIX}rtm status --run {run_id}");
        }
    }
    format!("{PREFIX}rtm start")
}

/// The next act for a hard error that halted a status/step render, keyed by
/// the stable surface the error names. This table never reads runbook bytes:
/// the `RB*`-coded text it recognizes below is the error surface
/// `MachineClass`'s one parser already produced. A runbook that will not read
/// or parse is diagnosed read-only by `rtm doctor`; a Run
/// Record that will not read (R-027), or a retired Run, admits no engine
/// verb - the record is Engine-owned - so the legitimate act is minting a
/// fresh Run, exactly the repair the retired-run refusal already names in
/// prose.
pub(crate) fn error_next(error: &StateError) -> String {
    let text = error.to_string();
    if text.contains("run.toml") || text.contains("is terminal") {
        format!("{PREFIX}rtm start")
    } else {
        format!("{PREFIX}rtm doctor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_keys_the_next_act_by_status() {
        assert_eq!(
            status_next("run-001", Status::Executing).as_deref(),
            Some("next: rtm step --run run-001")
        );
        assert_eq!(
            status_next("run-001", Status::Planned).as_deref(),
            Some("next: rtm step --run run-001")
        );
        assert_eq!(
            status_next("run-001", Status::Blocked).as_deref(),
            Some("next: rtm step --run run-001")
        );
        assert_eq!(status_next("run-001", Status::Passed), None);
        assert_eq!(status_next("run-001", Status::Failed), None);
    }

    #[test]
    fn the_table_keys_a_step_refusal_by_its_failure_kind() {
        let refused = |kind: &str| StepOutcome::Refused {
            failures: vec![crate::scheduler::guard_failure(
                kind, "run-001", "observed", "expected",
            )],
        };
        assert_eq!(
            step_refusal_next("run-001", &refused("files_exact")).as_deref(),
            Some("next: rtm step --run run-001")
        );
        assert_eq!(
            step_refusal_next("run-001", &refused("verdict")).as_deref(),
            Some("next: rtm step --run run-001")
        );
        assert_eq!(
            step_refusal_next("run-001", &refused("terminal")).as_deref(),
            Some("next: rtm status --run run-001")
        );
    }

    #[test]
    fn the_table_keys_a_hard_error_by_its_stable_surface() {
        assert_eq!(
            error_next(&StateError::new(
                "parse .ratmac/ratmac.toml [RB105]: states table is missing"
            ))
            .as_str(),
            "next: rtm doctor"
        );
        assert_eq!(
            error_next(&StateError::new(
                "invalid run.toml: missing required field status"
            ))
            .as_str(),
            "next: rtm start"
        );
        assert_eq!(
            error_next(&StateError::new(
                "run run-001 is terminal: its admission state is retired"
            ))
            .as_str(),
            "next: rtm start"
        );
    }
}
