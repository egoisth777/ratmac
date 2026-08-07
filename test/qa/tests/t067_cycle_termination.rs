//! t-067 / FDC-008: cycle termination as a static check.
//!
//! PT-067-01 `guarded_cycle_passes_termination`
//! PT-067-02 `unguarded_cycle_fails_naming_phases_and_class`
//!
//! Termination is guard-kind membership, never execution: every Phase on a
//! cycle must carry at least one out-edge guarded by a receipt-class
//! (`sensitivity_receipts`, `completion_gate`) or contract-class
//! (`intake_contract`, `record_contract`) guard. The doctor reports the
//! defect as data with a stable code; blocked routes satisfy nothing.

use std::fs;
use std::path::PathBuf;

use ratmac::doctor::{self, Severity};

/// The stable finding code this ticket introduces.
const TERMINATION_CODE: &str = "RB214";

/// A throwaway directory holding runbooks to diagnose.
struct Bench {
    root: PathBuf,
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Bench {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t067-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create bench");
        Self { root }
    }

    fn runbook(&self, name: &str, source: &str) -> PathBuf {
        let path = self.root.join(format!("{name}.toml"));
        fs::write(&path, source).expect("write runbook");
        path
    }
}

/// A cycle `plan <-> review` entered from `intake`, exiting to `done`.
/// Both cycle Phases carry a guard of the given kinds.
fn cycle_runbook(plan_guard: &str, review_guard: &str) -> String {
    format!(
        r#"[roots]
goal = "workflow/goal"
issue = "workflow/issue"
residual = "workflow/residual"
ticket = "workflow/ticket"

[phases.intake]
prompt = "Intake."
[phases.plan]
prompt = "Plan."
{plan_guard}

[phases.review]
prompt = "Review."
inputs = ["revise", "approve"]
{review_guard}

[phases.done]
prompt = "Done."

[[transitions]]
from = "intake"
to = "plan"

[[transitions]]
from = "plan"
to = "review"

[[transitions]]
from = "review"
to = "plan"
input = "revise"

[[transitions]]
from = "review"
to = "done"
input = "approve"
"#
    )
}

fn termination_findings(path: &std::path::Path) -> Vec<doctor::Finding> {
    doctor::diagnose(path)
        .into_iter()
        .filter(|f| f.code() == TERMINATION_CODE)
        .collect()
}

/// PT-067-01: a cycle whose every Phase carries a receipt-class guarded
/// out-edge passes with zero termination findings; the contract-class twin
/// passes identically.
#[test]
fn guarded_cycle_passes_termination() {
    let bench = Bench::new("guarded");

    let receipt = bench.runbook(
        "receipt",
        &cycle_runbook(
            r#"guards = [{ kind = "sensitivity_receipts", ticket = "t-067" }]"#,
            r#"guards = [{ kind = "completion_gate", ticket = "t-067" }]"#,
        ),
    );
    let all = doctor::diagnose(&receipt);
    assert!(
        !all.iter().any(|f| f.severity() == Severity::Error),
        "receipt-guarded cycle must be error-free, got: {all:?}"
    );
    assert!(
        termination_findings(&receipt).is_empty(),
        "receipt-class guards on every cycle Phase satisfy termination"
    );

    let contract = bench.runbook(
        "contract",
        &cycle_runbook(
            r#"guards = [{ kind = "intake_contract" }]"#,
            r#"guards = [{ kind = "record_contract" }]"#,
        ),
    );
    let all = doctor::diagnose(&contract);
    assert!(
        !all.iter().any(|f| f.severity() == Severity::Error),
        "contract-guarded cycle must be error-free, got: {all:?}"
    );
    assert!(
        termination_findings(&contract).is_empty(),
        "contract-class guards on every cycle Phase satisfy termination"
    );
}

/// PT-067-02: stripping the guarded out-edge from one cycle Phase fails the
/// pass with the stable code, naming the cycle's Phases, the offending
/// Phase, and the missing guard-kind classes. A cycle-free runbook is never
/// named.
#[test]
fn unguarded_cycle_fails_naming_phases_and_class() {
    let bench = Bench::new("unguarded");

    let broken = bench.runbook(
        "broken",
        &cycle_runbook(
            "",
            r#"guards = [{ kind = "completion_gate", ticket = "t-067" }]"#,
        ),
    );
    let found = termination_findings(&broken);
    assert_eq!(
        found.len(),
        1,
        "one unguarded cycle yields exactly one termination finding, got: {found:?}"
    );
    let finding = &found[0];
    assert_eq!(
        finding.severity(),
        Severity::Error,
        "termination is an error"
    );
    let text = format!("{} {}", finding.location(), finding.message());
    for needle in ["plan", "review", "receipt", "contract"] {
        assert!(
            text.contains(needle),
            "the finding names the cycle's Phases and the missing guard-kind \
             classes; missing {needle:?} in: {text}"
        );
    }
    assert!(
        !text.contains("intake") && !text.contains("done"),
        "off-cycle Phases are not named: {text}"
    );

    let straight = bench.runbook(
        "straight",
        r#"[phases.plan]
prompt = "Plan."

[phases.done]
prompt = "Done."

[[transitions]]
from = "plan"
to = "done"
"#,
    );
    assert!(
        termination_findings(&straight).is_empty(),
        "a cycle-free runbook carries no termination finding"
    );
}
