//! t-088 / PCR-008: the intake gate accepts a working-authority requirement.
//!
//! PCRV-006 `intake_resolves_a_requirement_in_either_authority`
//!
//! A requirement can live in two places: a row in the goal's specification, or
//! a requirement-ID heading in the working authority - the rules that bind the
//! contributor and deliberately mint no goal row. An accepted ask resolving to
//! either one passes; one resolving to neither refuses, and the refusal names
//! the ask and both places that were searched.

use ratmac::contract::gate_intake;
use std::fs;
use std::path::PathBuf;

struct Tree {
    root: PathBuf,
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Tree {
    /// A tree whose goal carries one requirement row and whose working
    /// authority carries one requirement-ID heading.
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t088-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in [".arca/goal", ".arca/authority", ".arca/issue", ".ratmac"] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        let tree = Tree { root };
        tree.write_goal();
        tree.write_authority();
        tree.write_runbook();
        tree
    }

    fn write_goal(&self) {
        fs::write(
            self.root.join(".arca/goal/spec.md"),
            "# Goal spec\n\n\
             | Req ID | Requirement | Source |\n\
             |---|---|---|\n\
             | `DEMO-001` | The demo behaves. | [i-100-demo](../issue/i-100-demo/index.md) |\n",
        )
        .expect("write goal spec");
    }

    /// The working authority: requirements that bind the contributor, carried
    /// as requirement-ID headings and never as goal rows.
    fn write_authority(&self) {
        fs::write(
            self.root.join(".arca/authority/schema.md"),
            "# Working rules\n\n\
             ## AUTH-001 - the contributor commits before damage\n\n\
             A contributor commits a checkpoint before any deliberate damage.\n\n\
             ## Not a requirement heading\n\n\
             Prose mentioning DEMO-001 and AUTH-001 resolves nothing.\n",
        )
        .expect("write working authority");
    }

    fn write_runbook(&self) {
        fs::write(
            self.root.join(".ratmac/ratmac.toml"),
            "[roots]\n\
             goal = \".arca/goal\"\n\
             issue = \".arca/issue\"\n\
             authority = \".arca/authority\"\n\
             \n\
             [states.intake]\n\
             prompt = \"Integrate.\"\n\
             guards = [{ kind = \"intake_contract\" }]\n",
        )
        .expect("write machine class");
    }

    /// One integrated bundle whose single accepted ask cites `requirement`.
    fn write_issue(&self, folder: &str, requirement: &str) {
        let dir = self.root.join(".arca/issue").join(folder);
        fs::create_dir_all(&dir).expect("create issue folder");
        fs::write(
            dir.join("index.md"),
            format!(
                "# Issue {folder}\n\n\
                 ```yaml\nissue-id: \"{folder}\"\nstatus: \"integrated\"\n```\n\n\
                 See [goal spec](../../goal/spec.md).\n"
            ),
        )
        .expect("write issue index");
        fs::write(
            dir.join("spec.md"),
            format!(
                "# Requirement records\n\n\
                 | Req ID | Requirement | Status |\n|---|---|---|\n\
                 | `{requirement}` | The demo behaves. | accepted |\n"
            ),
        )
        .expect("write issue spec");
        for name in ["design.md", "test-plan.md", "ubi-lang.md"] {
            fs::write(dir.join(name), format!("# {name}\n")).expect("write issue file");
        }
    }

    fn remove_authority_root(&self) {
        fs::remove_dir_all(self.root.join(".arca/authority")).expect("remove working authority");
        fs::write(
            self.root.join(".ratmac/ratmac.toml"),
            "[roots]\n\
             goal = \".arca/goal\"\n\
             issue = \".arca/issue\"\n\
             \n\
             [states.intake]\n\
             prompt = \"Integrate.\"\n\
             guards = [{ kind = \"intake_contract\" }]\n",
        )
        .expect("rewrite machine class");
    }
}

/// PCRV-006: either authority resolves an accepted ask; neither refuses, and
/// the refusal names the ask and both searched places.
#[test]
fn intake_resolves_a_requirement_in_either_authority() {
    let tree = Tree::new("either");
    tree.write_issue("i-100-demo", "DEMO-001");
    tree.write_issue("i-101-authority", "AUTH-001");

    gate_intake(&tree.root).expect("an ask resolving to a goal row or an authority heading passes");

    tree.write_issue("i-102-nowhere", "NOPE-001");
    let defects = gate_intake(&tree.root).expect_err("an ask resolving to neither place refuses");
    let reported: Vec<String> = defects.iter().map(|defect| defect.to_string()).collect();
    let joined = reported.join("\n");

    assert_eq!(
        defects.len(),
        1,
        "exactly the unresolvable ask is reported; got:\n{joined}"
    );
    assert!(
        joined.contains("NOPE-001"),
        "the refusal names the ask; got:\n{joined}"
    );
    assert!(
        joined.contains("spec.md"),
        "the refusal names the goal authority it searched; got:\n{joined}"
    );
    assert!(
        joined.contains("authority"),
        "the refusal names the working authority it searched; got:\n{joined}"
    );
}

/// A tree that declares no working authority keeps today's behaviour: the goal
/// is the only place a requirement can live, and an absent authority never
/// turns an unresolvable ask into a pass.
#[test]
fn an_undeclared_authority_resolves_nothing() {
    let tree = Tree::new("undeclared");
    tree.write_issue("i-100-demo", "DEMO-001");
    tree.remove_authority_root();

    gate_intake(&tree.root).expect("a goal-row ask still passes with no authority declared");

    tree.write_issue("i-101-authority", "AUTH-001");
    let defects = gate_intake(&tree.root)
        .expect_err("with no authority declared, its heading resolves nothing");
    let joined = defects
        .iter()
        .map(|defect| defect.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("AUTH-001"),
        "the refusal names the unresolvable ask; got:\n{joined}"
    );
}
