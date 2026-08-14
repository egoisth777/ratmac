//! The aged-fixture builder (t-097 / ARF-003, res-140).
//!
//! Every contract-gate fixture before i-029 was born at the current freeze,
//! which is exactly why an unpassable gate looked green for months. This
//! builder advances a fixture repository through freezes: write records at
//! freeze A, archive them, move to freeze B - so a gate can be exercised on
//! the kind of history the run-002 defect class hides in. `t-098` composes it
//! for the remaining gates.

use std::fs;
use std::path::{Path, PathBuf};

/// A fixture repository whose archive carries records judged under earlier
/// freezes. Age is a property of the path (`.arca/residual/archive/`), never
/// of whether a citation happens to match.
pub struct AgedTree {
    pub root: PathBuf,
    /// Freezes this tree has lived through, oldest first; the last entry is
    /// the current one live records must cite.
    freezes: Vec<String>,
}

impl AgedTree {
    /// A fresh repository at its first freeze: the standard workflow roots,
    /// a one-row goal, and no history yet.
    pub fn new(label: &str, first_freeze: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-aged-{label}-{}-{}",
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
            ".arca/residual/archive",
            ".arca/ticket",
            ".ratmac",
        ] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        fs::create_dir_all(root.join(".arca/issue/i-100-demo")).expect("create demo issue");
        fs::write(
            root.join(".arca/issue/i-100-demo/index.md"),
            "# Issue i-100-demo\n\n```yaml\nissue-id: \"i-100-demo\"\nstatus: \"integrated\"\n```\n\n\
             See [goal spec](../../goal/spec.md).\n",
        )
        .expect("write demo issue index");
        fs::write(
            root.join(".arca/issue/i-100-demo/spec.md"),
            "# Issue specification\n\n## Requirement Records\n\n\
             | Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |\n\
             | :--- | :--- | :--- | :--- | :--- |\n\
             | `DEMO-001` | The demo behaves. | accepted | Demo. | [goal spec](../../goal/spec.md) |\n",
        )
        .expect("write demo issue spec");
        for leaf in ["design.md", "test-plan.md", "ubi-lang.md"] {
            fs::write(
                root.join(".arca/issue/i-100-demo").join(leaf),
                "# Demo\n\nDemo.\n",
            )
            .expect("write demo issue leaf");
        }
        fs::write(
            root.join(".arca/goal/spec.md"),
            "# Goal spec\n\n\
             | Req ID | Requirement | Source |\n\
             |---|---|---|\n\
             | DEMO-001 | The demo behaves. | [issue DEMO-001](../issue/i-100-demo/spec.md#requirement-records) |\n",
        )
        .expect("write goal spec");
        Self {
            root,
            freezes: vec![first_freeze.to_owned()],
        }
    }

    /// The freeze live records must cite now.
    pub fn current_freeze(&self) -> &str {
        self.freezes.last().expect("a tree always has a freeze")
    }

    /// How many freezes deep the tree's history goes (zero for a newborn).
    pub fn age(&self) -> usize {
        self.freezes.len() - 1
    }

    /// Advance to the next freeze: every live record takes the authorized
    /// archive move first, so the archive is what carries the past.
    pub fn advance_to(&mut self, next_freeze: &str) {
        let active = self.root.join(".arca/residual");
        let archive = active.join("archive");
        for entry in fs::read_dir(&active)
            .expect("read active records")
            .flatten()
        {
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "md") {
                fs::rename(&path, archive.join(entry.file_name())).expect("archive move");
            }
        }
        self.freezes.push(next_freeze.to_owned());
    }

    /// Add a goal row so a second requirement can carry records.
    pub fn add_requirement(&self, id: &str) {
        let path = self.root.join(".arca/goal/spec.md");
        let mut text = fs::read_to_string(&path).expect("read goal spec");
        text.push_str(&format!(
            "| {id} | The {id} behavior holds. | [issue {id}](../issue/i-100-demo/spec.md#requirement-records) |\n"
        ));
        fs::write(&path, text).expect("widen goal spec");
    }

    /// A live record judged at the current freeze.
    pub fn write_record(&self, id: &str, requirement: &str, status: &str, evidence: &[&str]) {
        let refs = if evidence.is_empty() {
            String::new()
        } else {
            evidence
                .iter()
                .map(|entry| format!("  - \"{entry}\"\n"))
                .collect()
        };
        fs::write(
            self.root.join(".arca/residual").join(format!("{id}.md")),
            format!(
                "# Residual Record\n\n```yaml\n\
                 residual-id: \"{id}\"\n\
                 goal-requirement-ref: \"{requirement}\"\n\
                 frozen-goal-bundle-revision: \"goal-sha256:{}\"\n\
                 concrete-evidence-refs:\n{refs}\
                 status: \"{status}\"\n```\n",
                self.current_freeze()
            ),
        )
        .expect("write live record");
    }

    /// An owning ticket for cited gap records.
    pub fn write_ticket(&self, id: &str, residuals: &[&str]) {
        let residual_lines: String = residuals
            .iter()
            .map(|entry| format!("  - \"{entry}\"\n"))
            .collect();
        fs::write(
            self.root.join(".arca/ticket").join(format!("{id}.md")),
            format!(
                "---\nticket-id: {id}\nresidual-ids:\n{residual_lines}\
                 dependencies:\nstatus: \"approved\"\n---\n\n# Ticket: {id}\n\n\
                 ## Vertical Outcome\n\nOutcome.\n\n\
                 ## Worktree Scope\n\nScope.\n\n\
                 ## P4 Apparent Test Plan\n\n| Apparent Test ID |\n|---|\n| `PT-100-01` |\n\n\
                 ## P5 Hidden Test Public Coverage Manifest\n\n\
                 | Lane | Assessment | Rationale | Hidden IDs |\n|---|---|---|---|\n\
                 | `Regression` | `covered` | Reason. | `none` |\n\
                 | `Input/Routing` | `covered` | Reason. | `none` |\n\
                 | `Lifecycle/Model` | `covered` | Reason. | `none` |\n\
                 | `Durability/Recovery` | `covered` | Reason. | `none` |\n\
                 | `Output/Filesystem` | `covered` | Reason. | `none` |\n\
                 | `Cross-Feature` | `covered` | Reason. | `none` |\n\n\
                 ## Merge Gate\n\n- `cargo test`\n"
            ),
        )
        .expect("write ticket");
    }

    pub fn goal_root(&self) -> PathBuf {
        self.root.join(".arca/goal")
    }

    pub fn residual_root(&self) -> PathBuf {
        self.root.join(".arca/residual")
    }

    pub fn ticket_root(&self) -> PathBuf {
        self.root.join(".arca/ticket")
    }

    /// A synthetic Engine root whose one Run froze the current freeze, so the
    /// record gate can be exercised without machine-local Run state.
    pub fn engine_with_freeze(&self) -> (PathBuf, &'static str) {
        let run_dir = self.root.join(".ratmac/runs/run-000");
        fs::create_dir_all(&run_dir).expect("create synthetic run");
        fs::write(
            run_dir.join("evidence.toml"),
            format!("[goal]\nfrozen = \"{}\"\n", self.current_freeze()),
        )
        .expect("write synthetic freeze");
        (self.root.join(".ratmac"), "run-000")
    }
}

impl Drop for AgedTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn snapshot_dir(dir: &Path, into: &mut Vec<(String, Vec<u8>)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            snapshot_dir(&path, into);
        } else {
            into.push((
                path.to_string_lossy().into_owned(),
                fs::read(&path).unwrap_or_default(),
            ));
        }
    }
}

/// Byte-exact snapshot of a tree, for write-nothing oracles.
pub fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut entries = Vec::new();
    snapshot_dir(root, &mut entries);
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries
}
