//! t-091 / PCR-003: open work items are decided by machine.
//!
//! PCRV-003 `open_items_are_computed_from_the_tree`
//!
//! Whether work remains open is computed from what is on disk - where the
//! item sits and what its gap records say - never from prose a contributor
//! wrote. Seeding one unproven gap record flips exactly one item to open and
//! moves nothing else.

use ratmac::contract::{work_items, WorkItemState};
use std::fs;
use std::path::{Path, PathBuf};

/// A tree with a declared ticket root: two items that took the archive move,
/// one item still being worked, and one that is proven but not yet archived.
struct Tree {
    root: PathBuf,
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Tree {
    fn create(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t091-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in [
            ".arca/goal",
            ".arca/residual",
            ".arca/ticket/archive",
            ".ratmac",
        ] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        let tree = Tree { root };
        tree.write_runbook();
        tree
    }

    /// The declared roots the predicate reads. Nothing else in the class
    /// matters to it.
    fn write_runbook(&self) {
        fs::write(
            self.root.join(".ratmac/ratmac.toml"),
            "[roots]\n\
             goal = \".arca/goal\"\n\
             residual = \".arca/residual\"\n\
             ticket = \".arca/ticket\"\n\n\
             [states.build]\nprompt = \"Build.\"\n\n\
             [states.done]\nprompt = \"Done.\"\n\n\
             [[transitions]]\nfrom = \"build\"\nto = \"done\"\n",
        )
        .expect("write machine class");
    }

    fn write_gap(&self, id: &str, status: &str) {
        fs::write(
            self.root.join(".arca/residual").join(format!("{id}.md")),
            format!(
                "# Residual Record\n\n```yaml\n\
                 residual-id: \"{id}\"\n\
                 goal-requirement-ref: \"DEMO-001\"\n\
                 status: \"{status}\"\n```\n"
            ),
        )
        .expect("write gap record");
    }

    fn write_item(&self, id: &str, gaps: &[&str], archived: bool) {
        let lines: String = gaps
            .iter()
            .map(|entry| format!("  - \"{entry}\"\n"))
            .collect();
        let dir = if archived {
            self.root.join(".arca/ticket/archive")
        } else {
            self.root.join(".arca/ticket")
        };
        fs::write(
            dir.join(format!("{id}.md")),
            format!("---\nticket-id: \"{id}\"\nresidual-ids:\n{lines}---\n\n# Ticket: {id}\n"),
        )
        .expect("write work item");
    }
}

/// The computed answer as `id -> state`, so a failure names the item that
/// moved rather than reporting that some vector differs.
fn states(root: &Path) -> Vec<(String, WorkItemState)> {
    work_items(root)
        .unwrap_or_else(|defects| {
            panic!(
                "the tree classifies without defect: {}",
                defects
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })
        .into_iter()
        .map(|item| (item.id, item.state))
        .collect()
}

/// PCRV-003: the predicate names exactly the open item from the tree alone,
/// and one further unproven gap record flips exactly one further item.
#[test]
fn open_items_are_computed_from_the_tree() {
    let tree = Tree::create("computed");
    // Two items that took the authorized archive move, both fully proven.
    tree.write_gap("res-001", "satisfied");
    tree.write_gap("res-002", "satisfied");
    tree.write_item("t-001", &["res-001"], true);
    tree.write_item("t-002", &["res-002"], true);
    // One item still being worked: its gap is not proven.
    tree.write_gap("res-003", "missing");
    tree.write_item("t-003", &["res-003"], false);
    // One item proven but not yet archived - the state every landing passes
    // through until the cycle-end archive move.
    tree.write_gap("res-004", "satisfied");
    tree.write_item("t-004", &["res-004"], false);

    let before = states(&tree.root);
    assert_eq!(
        before,
        vec![
            ("t-001".to_owned(), WorkItemState::Landed),
            ("t-002".to_owned(), WorkItemState::Landed),
            ("t-003".to_owned(), WorkItemState::Open),
            ("t-004".to_owned(), WorkItemState::AwaitingArchive),
        ],
        "the predicate reads location and gap records, and nothing else"
    );

    // Exactly one seed: a further unproven gap record owned by the item that
    // was proven. No document's prose changes.
    tree.write_gap("res-005", "missing");
    tree.write_item("t-004", &["res-004", "res-005"], false);

    let after = states(&tree.root);
    assert_eq!(
        after,
        vec![
            ("t-001".to_owned(), WorkItemState::Landed),
            ("t-002".to_owned(), WorkItemState::Landed),
            ("t-003".to_owned(), WorkItemState::Open),
            ("t-004".to_owned(), WorkItemState::Open),
        ],
        "one unproven gap record flips exactly one item to open"
    );

    let open_before = before
        .iter()
        .filter(|(_, state)| *state == WorkItemState::Open)
        .count();
    let open_after = after
        .iter()
        .filter(|(_, state)| *state == WorkItemState::Open)
        .count();
    assert_eq!(
        (open_before, open_after),
        (1, 2),
        "exactly one item is open before the seed and exactly one more after"
    );
}
