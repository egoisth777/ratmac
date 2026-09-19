//! t-113 / PCR-003: a reopened gap keeps landed history and requires a
//! current owner.
//!
//! PT-113-06 `reopened_gap_keeps_history_landed_and_requires_current_owner`
//!
//! A reopened gap record is partial again while an archived historical
//! ticket still cites it. Current ownership is the direct-root ticket that
//! cites the gap now: the record gate accepts exactly that one owner, and
//! the classifier keeps the historical ticket landed while the current
//! owner stays open. Removing the current owner makes both readers report
//! the gap as currently ownerless by name - the archived citation satisfies
//! neither reader - and the classifier says so through its orphan
//! diagnosis, never an unrelated error. A proven-history anchor and a
//! duplicate-owner control rule out fake progress: landed history still
//! lands on its own proven merits, and the gate's earlier acceptance was
//! the exactly-one-current-owner boundary, not an unchecked pass.
//!
//! Every identifier here is a non-project literal (res-777, res-778,
//! t-770, t-771, t-772, t-773, run-771, DEMO-001, DEMO-002); the fixture
//! shares no code with the t-091 suite or the private lanes.

use ratmac::contract::{gate_records_at, work_items_at, ContractDefect, WorkItem, WorkItemState};
use std::fs;
use std::path::PathBuf;

/// One temporary repository: declared roots, a frozen goal, a reopened
/// partial gap with an archived and a current citation, and one proven
/// historical gap. Removed on drop, so the check leaves nothing behind.
struct Tree {
    root: PathBuf,
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// The frozen goal revision this fixture's live records cite. A literal of
/// sixty-four sevens: nothing in this repository ever froze it.
const FROZEN: &str = "7777777777777777777777777777777777777777777777777777777777777777";

/// The addressed run whose evidence carries the frozen revision.
const RUN: &str = "run-771";

impl Tree {
    fn create(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t113-reopened-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in [
            ".arca/goal",
            ".arca/issue",
            ".arca/residual",
            ".arca/ticket/archive",
            ".ratmac",
        ] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        fs::create_dir_all(root.join(format!(".ratmac/runs/{RUN}"))).expect("create run directory");
        let tree = Tree { root };
        tree.write_goal();
        tree.write_runbook();
        tree.write_evidence();
        tree
    }

    fn goal_root(&self) -> PathBuf {
        self.root.join(".arca/goal")
    }

    fn residual_root(&self) -> PathBuf {
        self.root.join(".arca/residual")
    }

    fn ticket_root(&self) -> PathBuf {
        self.root.join(".arca/ticket")
    }

    fn engine_root(&self) -> PathBuf {
        self.root.join(".ratmac")
    }

    /// The record gate over the resolved fixed-role roots, so the check
    /// names the reader it proves something about.
    fn records(&self) -> Result<(), Vec<ContractDefect>> {
        gate_records_at(
            &self.root,
            &self.goal_root(),
            &self.residual_root(),
            &self.ticket_root(),
            &self.engine_root(),
            RUN,
        )
    }

    /// The classifier's items, so a failure shows the reader's whole
    /// answer; the comparison borrows each `id` as `&str`, never clones.
    fn classified(&self) -> Result<Vec<WorkItem>, Vec<ContractDefect>> {
        work_items_at(&self.ticket_root(), &self.residual_root())
    }

    /// Two goal rows, each carried by exactly one gap record.
    fn write_goal(&self) {
        fs::write(
            self.goal_root().join("spec.md"),
            "# Demo goal\n\n\
             | Req ID | Requirement | Source |\n\
             |---|---|---|\n\
             | DEMO-001 | The demo behaves. | authored |\n\
             | DEMO-002 | The demo is proven. | authored |\n",
        )
        .expect("write goal spec");
    }

    /// A Machine Class that declares the three mechanized gate kinds, so a
    /// satisfied record in this fixture is mechanized, never vacuous. The
    /// sensitivity guard names a ticket file literally; nothing in these
    /// checks evaluates that guard, so it survives the file's removal.
    fn write_runbook(&self) {
        fs::write(
            self.engine_root().join("ratmac.toml"),
            "[roots]\n\
             goal = \".arca/goal\"\n\
             issue = \".arca/issue\"\n\
             residual = \".arca/residual\"\n\
             ticket = \".arca/ticket\"\n\
             \n\
             [states.intake]\n\
             prompt = \"Intake.\"\n\
             guards = [{ kind = \"intake_contract\" }]\n\
             \n\
             [states.gaps]\n\
             prompt = \"Find gaps.\"\n\
             guards = [{ kind = \"record_contract\" }]\n\
             \n\
             [states.build]\n\
             prompt = \"Build.\"\n\
             guards = [{ kind = \"sensitivity_receipts\", root = \"ticket\", ticket = \"t-772.md\" }]\n\
             \n\
             [[transitions]]\n\
             from = \"intake\"\n\
             to = \"gaps\"\n\
             \n\
             [[transitions]]\n\
             from = \"gaps\"\n\
             to = \"build\"\n",
        )
        .expect("write machine class");
    }

    /// Run evidence carrying the frozen revision the live records cite.
    fn write_evidence(&self) {
        fs::write(
            self.engine_root().join(format!("runs/{RUN}/evidence.toml")),
            format!("[goal]\nbaseline = \"{FROZEN}\"\nfrozen = \"{FROZEN}\"\n"),
        )
        .expect("write run evidence");
    }

    fn write_residual(&self, id: &str, requirement: &str, status: &str, evidence: &[&str]) {
        let refs = if evidence.is_empty() {
            String::new()
        } else {
            evidence
                .iter()
                .map(|entry| format!("  - \"{entry}\"\n"))
                .collect()
        };
        fs::write(
            self.residual_root().join(format!("{id}.md")),
            format!(
                "# Residual Record\n\n```yaml\n\
                 residual-id: \"{id}\"\n\
                 goal-requirement-ref: \"{requirement}\"\n\
                 frozen-goal-bundle-revision: \"goal-sha256:{FROZEN}\"\n\
                 concrete-evidence-refs:\n{refs}\
                 status: \"{status}\"\n```\n"
            ),
        )
        .expect("write gap record");
    }

    /// A complete ticket, in the working root when `current`, in the
    /// archive folder when it already took the archive move.
    fn write_ticket(&self, id: &str, gap: &str, current: bool) {
        let lanes = [
            "Regression",
            "Input/Routing",
            "Lifecycle/Model",
            "Durability/Recovery",
            "Output/Filesystem",
            "Cross-Feature",
        ]
        .iter()
        .map(|lane| format!("| `{lane}` | `covered` | Reason. | `none` |\n"))
        .collect::<String>();
        let folder = if current {
            self.ticket_root()
        } else {
            self.ticket_root().join("archive")
        };
        fs::write(
            folder.join(format!("{id}.md")),
            format!(
                "---\nticket-id: {id}\nresidual-ids:\n  - \"{gap}\"\n\
                 dependencies:\nstatus: \"approved\"\n---\n\n\
                 # Ticket: {id}\n\n## Vertical Outcome\n\nOutcome.\n\n\
                 ## Worktree Scope\n\nScope.\n\n\
                 ## P4 Apparent Test Plan\n\n| Apparent Test ID |\n|---|\n| `PT-772-01` |\n\n\
                 ## P5 Hidden Test Public Coverage Manifest\n\n\
                 | Lane | Assessment | Rationale | Hidden IDs |\n|---|---|---|---|\n{lanes}\
                 ## Merge Gate\n\n- Ticket tests pass.\n"
            ),
        )
        .expect("write ticket");
    }
}

/// Every defect on one line, so a failure shows the reader's whole answer.
fn rendered(defects: &[ContractDefect]) -> String {
    defects
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// PT-113-06: a reopened gap keeps its landed history and demands a
/// current owner. The record gate accepts sole current ownership; the
/// classifier returns the archived historical ticket as landed and the
/// current owner as open. Removing the current owner makes both readers
/// report current ownerlessness by gap name - history cannot satisfy the
/// orphan check - and the classifier emits its orphan diagnosis
/// explicitly, never an unrelated error.
#[test]
fn reopened_gap_keeps_history_landed_and_requires_current_owner() {
    let tree = Tree::create("proof");
    // The cast:
    // - res-777: the reopened gap, partial again while history cites it.
    // - t-771: the archived historical ticket citing res-777.
    // - t-772: the sole direct-root ticket - the current owner.
    // - res-778 cited by archived t-770: the proven-history anchor, landed
    //   on its own satisfied merits, needing no reopen.
    tree.write_residual("res-777", "DEMO-001", "partial", &[]);
    tree.write_ticket("t-771", "res-777", false);
    tree.write_ticket("t-772", "res-777", true);
    tree.write_residual("res-778", "DEMO-002", "satisfied", &["src/demo.rs"]);
    tree.write_ticket("t-770", "res-778", false);

    // The record gate accepts sole current ownership: it counts the
    // direct-root ticket, never the archived citation.
    tree.records().unwrap_or_else(|defects| {
        panic!(
            "the gate accepts sole current ownership: {}",
            rendered(&defects)
        )
    });

    // The classifier keeps landed history landed and reads the reopened
    // gap as the current owner's open work. This valid-reopen assertion is
    // the one the current classifier fails: it reads the archived citation
    // as current ownership and refuses the archive move instead.
    let items = tree.classified().unwrap_or_else(|defects| {
        panic!(
            "a valid reopen classifies without defect: {}",
            rendered(&defects)
        )
    });
    assert_eq!(
        items
            .iter()
            .map(|item| (item.id.as_str(), item.state))
            .collect::<Vec<_>>(),
        vec![
            ("t-770", WorkItemState::Landed),
            ("t-771", WorkItemState::Landed),
            ("t-772", WorkItemState::Open),
        ],
        "history stays landed, the current owner carries the open gap, and \
         the proven anchor is untouched"
    );

    // Remove the current owner. The reopened gap is partial on disk with
    // only an archived citation left.
    fs::remove_file(tree.ticket_root().join("t-772.md")).expect("remove the current owner");

    // The record gate reports the reopened gap as currently ownerless by
    // name, and by nothing else: the archived citation never counts.
    let gate = tree
        .records()
        .expect_err("an ownerless reopened gap refuses the record gate");
    assert_eq!(
        gate.len(),
        1,
        "the refusal is exactly the ownerless gap: {}",
        rendered(&gate)
    );
    assert!(
        gate[0].artifact.contains("res-777"),
        "the refusal names the gap: {}",
        rendered(&gate)
    );
    assert!(
        gate[0].reason.contains("owned by no ticket"),
        "the refusal is the ownership diagnosis: {}",
        rendered(&gate)
    );

    // The classifier reports the same ownerlessness through its orphan
    // diagnosis naming the gap - not through an unrelated error - while
    // the archived ticket's unproven citation stays refused.
    let defects = work_items_at(&tree.ticket_root(), &tree.residual_root())
        .expect_err("current ownerlessness refuses the classifier");
    let orphan = defects
        .iter()
        .find(|defect| {
            defect.artifact.contains("res-777")
                && defect.reason.contains("no work item on disk owns it")
        })
        .unwrap_or_else(|| panic!("the orphan diagnosis names the gap: {}", rendered(&defects)));
    assert!(
        !orphan.artifact.contains("t-771"),
        "the orphan diagnosis addresses the gap, not the historical ticket: {orphan}"
    );
    assert!(
        defects.iter().any(|defect| {
            defect.artifact.contains("t-771")
                && defect.reason.contains("archive")
                && defect.reason.contains("unproven")
        }),
        "the archived ticket's unproven citation stays refused: {}",
        rendered(&defects)
    );
    assert!(
        defects.iter().all(|defect| {
            !defect.artifact.contains("res-778") && !defect.artifact.contains("t-770")
        }),
        "the proven-history anchor is not part of the refusal: {}",
        rendered(&defects)
    );

    // Negative control: the gate's earlier acceptance was the
    // exactly-one-current-owner boundary, not an unchecked pass. Two
    // direct-root owners refuse; the refusal names both current tickets,
    // and the archived citation still counts for neither side.
    tree.write_ticket("t-772", "res-777", true);
    tree.write_ticket("t-773", "res-777", true);
    let duplicate = tree
        .records()
        .expect_err("two current owners refuse the record gate");
    assert_eq!(
        duplicate.len(),
        1,
        "the refusal is exactly the ownership conflict: {}",
        rendered(&duplicate)
    );
    let conflict = &duplicate[0];
    assert!(
        conflict.artifact.contains("res-777"),
        "the conflict names the gap: {conflict}"
    );
    assert!(
        conflict.reason.contains("2 tickets")
            && conflict.reason.contains("t-772")
            && conflict.reason.contains("t-773"),
        "the conflict names both current owners: {conflict}"
    );
    assert!(
        !conflict.reason.contains("t-771"),
        "an archived citation is never a current owner: {conflict}"
    );
}
