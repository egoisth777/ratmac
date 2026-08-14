//! t-046 / PGE-001, PGE-002: intake and record contract gates.
//!
//! PT-046-01 `intake_contract_verified`
//! PT-046-02 `record_contract_verified`
//! PT-046-03 `no_vacuous_satisfaction`
//! PT-046-04 `active_and_archived_residuals_form_one_namespace`
//! HT-046-01 `dependency_cycle_is_named`
//! HT-046-02 `broken_five_file_shape_refuses`
//! HT-046-03 `stale_frozen_revision_refuses`
//!
//! A status edit must not be able to route the loop: the gates read the
//! records themselves - issue shape, requirement IDs, links, residual
//! evidence, ticket ownership, and dependency order.

use ratmac::contract::{gate_intake, gate_records, unproven_mechanization, ContractDefect};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

struct Tree {
    root: PathBuf,
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const FROZEN: &str = "1111111111111111111111111111111111111111111111111111111111111111";

/// The addressed run whose evidence carries the frozen revision (FDC-004).
const RUN: &str = "run-001";

impl Tree {
    /// A correct batch: one integrated issue, one residual, one ticket.
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t048-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in [
            ".arca/goal",
            ".arca/issue/i-100-demo",
            ".arca/residual",
            ".arca/ticket",
            ".ratmac",
        ] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        let tree = Tree { root };
        tree.write_goal();
        tree.write_issue("i-100-demo", "integrated", "DEMO-001");
        tree.write_residual("res-100", "DEMO-001", "missing", FROZEN, &[]);
        tree.write_ticket("t-100", &["res-100"], &[]);
        tree.write_evidence(FROZEN);
        tree.write_runbook();
        tree
    }

    fn write_goal(&self) {
        fs::write(
            self.root.join(".arca/goal/spec.md"),
            "# Goal spec\n\n\
             | Req ID | Requirement | Source |\n\
             |---|---|---|\n\
             | DEMO-001 | The demo behaves. | [issue DEMO-001](../issue/i-100-demo/spec.md#requirement-records) |\n",
        )
        .expect("write goal spec");
    }

    fn write_issue(&self, folder: &str, status: &str, requirement: &str) {
        self.write_issue_rows_at("", folder, status, &[(requirement, "accepted")]);
    }

    fn write_issue_rows_at(&self, bucket: &str, folder: &str, status: &str, rows: &[(&str, &str)]) {
        let issue_root = self.root.join(".arca/issue");
        let dir = if bucket.is_empty() {
            issue_root.join(folder)
        } else {
            issue_root.join(bucket).join(folder)
        };
        fs::create_dir_all(&dir).expect("create issue folder");
        let goal_target = if bucket.is_empty() {
            "../../goal/spec.md"
        } else {
            "../../../goal/spec.md"
        };
        fs::write(
            dir.join("index.md"),
            format!(
                "# Issue {folder}\n\n\
                 ```yaml\nissue-id: \"{folder}\"\nstatus: \"{status}\"\n```\n\n\
                 See [goal spec]({goal_target}).\n"
            ),
        )
        .expect("write issue index");
        let records: String = rows
            .iter()
            .map(|(requirement, disposition)| {
                format!("| `{requirement}` | The demo behaves. | {disposition} |\n")
            })
            .collect();
        fs::write(
            dir.join("spec.md"),
            format!(
                "# Requirement records\n\n\
                 | Req ID | Requirement | Status |\n|---|---|---|\n\
                 {records}"
            ),
        )
        .expect("write issue spec");
        for name in ["design.md", "test-plan.md", "ubi-lang.md"] {
            fs::write(dir.join(name), format!("# {name}\n")).expect("write issue file");
        }
    }

    fn write_residual(
        &self,
        id: &str,
        requirement: &str,
        status: &str,
        frozen: &str,
        evidence: &[&str],
    ) {
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
                 frozen-goal-bundle-revision: \"goal-sha256:{frozen}\"\n\
                 concrete-evidence-refs:\n{refs}\
                 status: \"{status}\"\n```\n"
            ),
        )
        .expect("write residual");
    }

    /// ARF-001: an archived residual is frozen provenance from an earlier
    /// freeze. Written under `archive/` with an older revision citation.
    fn write_archived_residual(&self, id: &str, requirement: &str, frozen: &str) {
        let dir = self.root.join(".arca/residual/archive");
        fs::create_dir_all(&dir).expect("create residual archive");
        fs::write(
            dir.join(format!("{id}.md")),
            format!(
                "# Residual Record\n\n```yaml\n\
                 residual-id: \"{id}\"\n\
                 goal-requirement-ref: \"{requirement}\"\n\
                 frozen-goal-bundle-revision: \"goal-sha256:{frozen}\"\n\
                 concrete-evidence-refs:\n  - \"src/demo.rs\"\n\
                 status: \"satisfied\"\n```\n"
            ),
        )
        .expect("write archived residual");
    }

    fn write_ticket(&self, id: &str, residuals: &[&str], dependencies: &[&str]) {
        let residual_lines: String = residuals
            .iter()
            .map(|entry| format!("  - \"{entry}\"\n"))
            .collect();
        let dependency_lines: String = dependencies
            .iter()
            .map(|entry| format!("  - \"{entry}\"\n"))
            .collect();
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
        fs::write(
            self.root.join(".arca/ticket").join(format!("{id}.md")),
            format!(
                "---\nticket-id: {id}\nresidual-ids:\n{residual_lines}\
                 dependencies:\n{dependency_lines}status: \"approved\"\n---\n\n\
                 # Ticket: {id}\n\n## Vertical Outcome\n\nOutcome.\n\n\
                 ## Worktree Scope\n\nScope.\n\n\
                 ## P4 Apparent Test Plan\n\n| Apparent Test ID |\n|---|\n| `PT-100-01` |\n\n\
                 ## P5 Hidden Test Public Coverage Manifest\n\n\
                 | Lane | Assessment | Rationale | Hidden IDs |\n|---|---|---|---|\n{lanes}\n\
                 ## Merge Gate\n\n- Ticket tests pass.\n"
            ),
        )
        .expect("write ticket");
    }

    fn write_evidence(&self, frozen: &str) {
        // FDC-004: Run evidence resides in the Engine's addressed run directory.
        let run_dir = self.root.join(".ratmac/runs").join(RUN);
        fs::create_dir_all(&run_dir).expect("create run directory");
        fs::write(
            run_dir.join("evidence.toml"),
            format!("[goal]\nbaseline = \"{frozen}\"\nfrozen = \"{frozen}\"\n"),
        )
        .expect("write evidence");
    }

    /// A Machine Class that declares the mechanized gates.
    fn write_runbook(&self) {
        fs::write(
            self.root.join(".ratmac/ratmac.toml"),
            "[roots]\n\
             goal = \".arca/goal\"\n\
             issue = \".arca/issue\"\n\
             residual = \".arca/residual\"\n\
             ticket = \".arca/ticket\"\n\
             \n\
             [states.intake]\n\
             prompt = \"Integrate.\"\n\
             guards = [{ kind = \"intake_contract\" }]\n\
             \n\
             [states.gaps]\n\
             prompt = \"Find gaps.\"\n\
             guards = [{ kind = \"record_contract\" }]\n\
             \n\
             [states.build]\n\
             prompt = \"Build.\"\n\
             guards = [{ kind = \"sensitivity_receipts\", root = \"ticket\", ticket = \"t-100.md\" }]\n\
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

    /// Drive the real `rtm` CLI over the fixture, so the gate is exercised
    /// where it actually runs: inside the pinned boundary.
    fn rtm(&self, args: &[&str]) -> String {
        let output = Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm");
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }

    fn issue_dir(&self, folder: &str) -> PathBuf {
        self.root.join(".arca/issue").join(folder)
    }

    fn engine_root(&self) -> PathBuf {
        self.root.join(".ratmac")
    }
}

fn reasons(defects: &[ContractDefect]) -> String {
    defects
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

fn records(tree: &Tree) -> Result<(), Vec<ContractDefect>> {
    gate_records(&tree.root, &tree.engine_root(), RUN)
}

/// PT-046-01: integration is verified, not claimed.
#[test]
fn intake_contract_verified() {
    let tree = Tree::new("intake");
    gate_intake(&tree.root).unwrap_or_else(|defects| {
        panic!(
            "a correct batch must pass the intake gate: {}",
            reasons(&defects)
        )
    });

    // `integrated` while the accepted requirement is absent from the goal.
    let tree = Tree::new("missing-requirement");
    tree.write_issue("i-100-demo", "integrated", "DEMO-404");
    let defects = gate_intake(&tree.root).expect_err("an unintegrated requirement must refuse");
    let text = reasons(&defects);
    assert!(
        text.contains("DEMO-404") && text.contains("i-100-demo"),
        "the refusal names the offending artifact: {text}"
    );

    // A dangling reverse link from the goal to an issue that does not exist.
    let tree = Tree::new("dangling");
    fs::write(
        tree.root.join(".arca/goal/spec.md"),
        "# Goal spec\n\n\
         | Req ID | Requirement | Source |\n|---|---|---|\n\
         | DEMO-001 | The demo behaves. | [issue DEMO-001](../issue/i-999-gone/spec.md#requirement-records) |\n",
    )
    .expect("write goal with dangling link");
    let defects = gate_intake(&tree.root).expect_err("a dangling link must refuse");
    let text = reasons(&defects);
    assert!(
        text.contains("i-999-gone"),
        "the refusal names the dangling target: {text}"
    );

    // A pending issue folder is not integration.
    let tree = Tree::new("pending");
    tree.write_issue("i-100-demo", "pending", "DEMO-001");
    let defects = gate_intake(&tree.root).expect_err("a pending issue must refuse");
    assert!(
        reasons(&defects).contains("pending"),
        "the refusal names the status it found: {}",
        reasons(&defects)
    );

    // The same predicate must run inside the pinned boundary: the Engine's own
    // gate refuses the step and leaves the State where it was.
    tree.rtm(&["start"]);
    // FDC-004: address the live run — the Engine roster entry carries its State File.
    let live = fs::read_dir(tree.root.join(".ratmac/runs"))
        .expect("list the runs roster")
        .map(|entry| entry.expect("roster entry is readable"))
        .find(|entry| entry.path().join("run.toml").is_file())
        .expect("the started run appears on the roster")
        .file_name()
        .to_string_lossy()
        .into_owned();
    let refusal = tree.rtm(&["step", "--run", &live]);
    assert!(
        refusal.contains("i-100-demo") && refusal.contains("pending"),
        "the engine's refusal names the offending artifact: {refusal}"
    );
    let status = tree.rtm(&["status", "--run", &live]);
    assert!(
        status.contains("intake"),
        "a refused gate leaves the State unchanged: {status}"
    );
}

/// PGE-001 extension: dispositions determine the physical issue carrier.
#[test]
fn deferred_issue_location_and_dispositions_verified() {
    let tree = Tree::new("deferred-valid");
    tree.write_issue_rows_at(
        "deferred",
        "i-101-wait",
        "deferred",
        &[("WAI-001", "deferred")],
    );
    gate_intake(&tree.root).unwrap_or_else(|defects| {
        panic!(
            "a deferred issue in the waiting buffer must pass: {}",
            reasons(&defects)
        )
    });

    let tree = Tree::new("deferred-mixed");
    tree.write_issue_rows_at(
        "deferred",
        "i-101-wait",
        "deferred",
        &[("DEMO-001", "accepted"), ("WAI-001", "deferred")],
    );
    gate_intake(&tree.root).unwrap_or_else(|defects| {
        panic!(
            "accepted plus deferred asks stay live and must pass: {}",
            reasons(&defects)
        )
    });

    let tree = Tree::new("deferred-in-intake");
    tree.write_issue_rows_at("", "i-101-wait", "integrated", &[("WAI-001", "deferred")]);
    let text = reasons(&gate_intake(&tree.root).expect_err("a deferred ask in intake must refuse"));
    assert!(
        text.contains("i-101-wait") && text.contains(".arca/issue/deferred"),
        "refusal names the required carrier: {text}"
    );

    let tree = Tree::new("deferred-without-row");
    tree.write_issue_rows_at(
        "deferred",
        "i-101-wait",
        "deferred",
        &[("DEMO-001", "accepted")],
    );
    let text = reasons(
        &gate_intake(&tree.root).expect_err("deferred status without a deferred ask must refuse"),
    );
    assert!(text.contains("no deferred ask"), "{text}");

    let tree = Tree::new("deferred-in-archive");
    tree.write_issue_rows_at(
        "archive",
        "i-101-wait",
        "integrated",
        &[("WAI-001", "deferred")],
    );
    let text = reasons(&gate_intake(&tree.root).expect_err("an archived deferred ask must refuse"));
    assert!(
        text.contains("i-101-wait") && text.contains("restore"),
        "{text}"
    );

    let tree = Tree::new("integrated-empty");
    tree.write_issue_rows_at("", "i-100-demo", "integrated", &[("DEMO-001", "rejected")]);
    let text = reasons(
        &gate_intake(&tree.root).expect_err("integrated without accepted or duplicate must refuse"),
    );
    assert!(text.contains("accepted or duplicate"), "{text}");

    let tree = Tree::new("integrated-duplicate");
    tree.write_issue_rows_at("", "i-100-demo", "integrated", &[("SPC-001", "duplicate")]);
    gate_intake(&tree.root).unwrap_or_else(|defects| {
        panic!(
            "a duplicate is already represented in the goal and is a valid integration: {}",
            reasons(&defects)
        )
    });

    let tree = Tree::new("duplicate-issue-id");
    tree.write_issue_rows_at(
        "deferred",
        "i-100-demo",
        "deferred",
        &[("WAI-001", "deferred")],
    );
    let text = reasons(
        &gate_intake(&tree.root).expect_err("duplicate issue ids across buckets must refuse"),
    );
    assert!(
        text.contains("duplicated") && text.contains("i-100-demo"),
        "{text}"
    );
}

#[test]
fn archived_links_are_frozen_but_deferred_links_are_live() {
    let tree = Tree::new("historical-link");
    tree.write_issue_rows_at(
        "archive",
        "i-101-history",
        "integrated",
        &[("HIS-001", "duplicate")],
    );
    fs::write(
        tree.issue_dir("archive/i-101-history").join("design.md"),
        "# Design\n\n[historical target](../i-999-gone/design.md)\n",
    )
    .expect("write frozen historical link");
    gate_intake(&tree.root).unwrap_or_else(|defects| {
        panic!(
            "archived links are frozen provenance: {}",
            reasons(&defects)
        )
    });

    let tree = Tree::new("live-deferred-link");
    tree.write_issue_rows_at(
        "deferred",
        "i-101-wait",
        "deferred",
        &[("WAI-001", "deferred")],
    );
    fs::write(
        tree.issue_dir("deferred/i-101-wait").join("design.md"),
        "# Design\n\n[live target](../i-999-gone/design.md)\n",
    )
    .expect("write dangling live link");
    let text = reasons(&gate_intake(&tree.root).expect_err("a dangling deferred link must refuse"));
    assert!(text.contains("i-999-gone"), "{text}");
}

/// PT-046-02: record contracts are validated, not trusted.
#[test]
fn record_contract_verified() {
    let tree = Tree::new("records");
    records(&tree)
        .unwrap_or_else(|defects| panic!("complete records must pass: {}", reasons(&defects)));

    // `satisfied` with no concrete evidence references.
    let tree = Tree::new("evidence-free");
    tree.write_residual("res-100", "DEMO-001", "satisfied", FROZEN, &[]);
    let defects = records(&tree).expect_err("satisfied without evidence must refuse");
    let text = reasons(&defects);
    assert!(
        text.contains("res-100") && text.contains("evidence"),
        "the refusal names the record: {text}"
    );

    // A gap owned by no ticket.
    let tree = Tree::new("unowned");
    fs::remove_file(tree.root.join(".arca/ticket/t-100.md")).expect("remove the owning ticket");
    let defects = records(&tree).expect_err("an unowned gap must refuse");
    assert!(
        reasons(&defects).contains("res-100"),
        "the refusal names the unowned residual: {}",
        reasons(&defects)
    );

    // A gap owned by two tickets.
    let tree = Tree::new("double-owned");
    tree.write_ticket("t-101", &["res-100"], &[]);
    let defects = records(&tree).expect_err("a doubly owned gap must refuse");
    let text = reasons(&defects);
    assert!(
        text.contains("res-100") && text.contains("t-100") && text.contains("t-101"),
        "the refusal names the gap and both owners: {text}"
    );

    // A ticket missing its hidden-lane assessments.
    let tree = Tree::new("no-lanes");
    let path = tree.root.join(".arca/ticket/t-100.md");
    let source = fs::read_to_string(&path).expect("read ticket");
    let trimmed = source
        .split("## P5 Hidden Test Public Coverage Manifest")
        .next()
        .expect("ticket has a P5 section")
        .to_owned();
    fs::write(&path, trimmed).expect("truncate the ticket");
    let defects = records(&tree).expect_err("an incomplete ticket must refuse");
    let text = reasons(&defects);
    assert!(
        text.contains("t-100") && text.to_ascii_lowercase().contains("hidden"),
        "the refusal names the ticket and the missing section: {text}"
    );
}
/// PT-046-04: active and archived residuals form one complete namespace.
#[test]
fn active_and_archived_residuals_form_one_namespace() {
    let tree = Tree::new("archived-only");
    tree.write_residual("res-100", "DEMO-001", "satisfied", FROZEN, &["src/demo.rs"]);
    let archive = tree.root.join(".arca/residual/archive");
    fs::create_dir_all(&archive).expect("create residual archive");
    fs::rename(
        tree.root.join(".arca/residual/res-100.md"),
        archive.join("res-100.md"),
    )
    .expect("archive residual");
    records(&tree)
        .unwrap_or_else(|defects| panic!("an archived mapping must count: {}", reasons(&defects)));

    let tree = Tree::new("missing-mapping");
    fs::write(
        tree.root.join(".arca/goal/spec.md"),
        "# Goal spec\n\n\
         | Req ID | Requirement | Source |\n\
         |---|---|---|\n\
         | DEMO-001 | The demo behaves. | Source. |\n\
         | DEMO-002 | The second behavior exists. | Source. |\n",
    )
    .expect("write goal with an unmapped requirement");
    let defects = records(&tree).expect_err("every frozen requirement needs a residual");
    let text = reasons(&defects);
    assert!(
        text.contains("DEMO-002")
            && text.contains(".arca/residual")
            && text.contains(".arca/residual/archive"),
        "the refusal names the requirement and both residual locations: {text}"
    );

    let tree = Tree::new("duplicate-across-archive");
    tree.write_residual("res-101", "DEMO-001", "satisfied", FROZEN, &["src/demo.rs"]);
    let archive = tree.root.join(".arca/residual/archive");
    fs::create_dir_all(&archive).expect("create residual archive");
    fs::rename(
        tree.root.join(".arca/residual/res-101.md"),
        archive.join("res-101.md"),
    )
    .expect("archive duplicate residual");
    let defects =
        records(&tree).expect_err("duplicate mappings across active and archive must refuse");
    let text = reasons(&defects);
    assert!(
        text.contains("DEMO-001")
            && text.contains(".arca/residual/res-100.md")
            && text.contains(".arca/residual/archive/res-101.md"),
        "the refusal names the requirement and both records: {text}"
    );
}

/// PT-046-03: nothing is satisfied by the absence of a gate.
#[test]
fn no_vacuous_satisfaction() {
    let tree = Tree::new("unmechanized");
    // The tree claims everything is done, but declares no contract gates.
    fs::write(
        tree.root.join(".ratmac/ratmac.toml"),
        "[roots]\n\
         goal = \".arca/goal\"\n\
         issue = \".arca/issue\"\n\
         residual = \".arca/residual\"\n\
         ticket = \".arca/ticket\"\n\n\
         [states.build]\nprompt = \"Build.\"\n\n[states.done]\nprompt = \"Done.\"\n\n\
         [[transitions]]\nfrom = \"build\"\nto = \"done\"\n",
    )
    .expect("write gateless machine class");
    // A goal that states the mechanization requirement, and a record that
    // claims it is done.
    fs::write(
        tree.root.join(".arca/goal/spec.md"),
        "# Goal spec\n\n\
         | Req ID | Requirement | Source |\n|---|---|---|\n\
         | PGE-001 | The intake gate verifies integration. | [issue PGE-001](../issue/i-100-demo/spec.md#requirement-records) |\n",
    )
    .expect("write goal stating PGE-001");
    tree.write_residual(
        "res-100",
        "PGE-001",
        "satisfied",
        FROZEN,
        &["we followed the loop carefully"],
    );

    let unproven = unproven_mechanization(&tree.root);
    let text = reasons(&unproven);
    for requirement in ["PGE-001", "PGE-002", "PGE-003"] {
        assert!(
            text.contains(requirement),
            "{requirement} must be classified missing without its gate: {text}"
        );
    }
    assert!(
        text.contains("intake_contract")
            && text.contains("record_contract")
            && text.contains("sensitivity_receipts"),
        "the classification names the absent gate kinds: {text}"
    );

    // And the record gate refuses a satisfied claim that rests on that absence.
    let defects = records(&tree).expect_err("satisfied cannot rest on an unmechanized loop");
    assert!(
        reasons(&defects).contains("res-100"),
        "the refusal names the record making the claim: {}",
        reasons(&defects)
    );

    // With the gates declared, the same records are judged on their contents.
    tree.write_runbook();
    assert!(
        unproven_mechanization(&tree.root).is_empty(),
        "a Runbook declaring every gate leaves nothing unmechanized"
    );
}

/// HT-046-01 (Input/Routing): a dependency cycle is named, not ordered.
#[test]
fn dependency_cycle_is_named() {
    let tree = Tree::new("cycle");
    tree.write_residual("res-101", "DEMO-001", "missing", FROZEN, &[]);
    tree.write_residual("res-102", "DEMO-001", "missing", FROZEN, &[]);
    tree.write_ticket("t-100", &["res-100"], &["t-102"]);
    tree.write_ticket("t-101", &["res-101"], &["t-100"]);
    tree.write_ticket("t-102", &["res-102"], &["t-101"]);

    let defects = records(&tree).expect_err("a dependency cycle must refuse");
    let text = reasons(&defects);
    assert!(
        text.to_ascii_lowercase().contains("cycle"),
        "the refusal says it found a cycle: {text}"
    );
    for ticket in ["t-100", "t-101", "t-102"] {
        assert!(
            text.contains(ticket),
            "the refusal names every ticket in the cycle: {text}"
        );
    }

    // Malformed YAML front matter refuses instead of crashing.
    let tree = Tree::new("malformed");
    fs::write(
        tree.root.join(".arca/residual/res-100.md"),
        "# Residual Record\n\n```yaml\nresidual-id \"res-100\"\nstatus:\n```\n",
    )
    .expect("write malformed residual");
    let defects = records(&tree).expect_err("a malformed record must refuse");
    assert!(
        reasons(&defects).contains("res-100"),
        "the refusal names the unreadable record: {}",
        reasons(&defects)
    );
}

/// HT-046-02 (Output/Filesystem): the five-file shape is a contract.
#[test]
fn broken_five_file_shape_refuses() {
    let tree = Tree::new("shape");
    let dir = tree.issue_dir("i-100-demo");
    fs::rename(dir.join("design.md"), dir.join("designs.md")).expect("rename an issue file");

    let defects = gate_intake(&tree.root).expect_err("a broken five-file shape must refuse");
    let text = reasons(&defects);
    assert!(
        text.contains("i-100-demo") && text.contains("design.md"),
        "the refusal names the folder and the missing file: {text}"
    );

    // An extra file is a shape break too: the folder is a fixed five-file form.
    let tree = Tree::new("extra");
    fs::write(tree.issue_dir("i-100-demo").join("notes.md"), "notes\n")
        .expect("write an extra file");
    let defects = gate_intake(&tree.root).expect_err("an extra file must refuse");
    assert!(
        reasons(&defects).contains("notes.md"),
        "the refusal names the extra file: {}",
        reasons(&defects)
    );
}

/// HT-046-03 (Cross-Feature): the record gate consumes the frozen revision the
/// freeze boundary produced.
#[test]
fn stale_frozen_revision_refuses() {
    let stale = "2222222222222222222222222222222222222222222222222222222222222222";
    let tree = Tree::new("stale");
    tree.write_residual("res-100", "DEMO-001", "missing", stale, &[]);

    let defects = records(&tree).expect_err("a stale frozen revision must refuse");
    let text = reasons(&defects);
    assert!(
        text.contains("res-100") && text.contains(stale) && text.contains(FROZEN),
        "the refusal names the residual and both revisions: {text}"
    );

    // With no freeze at all there is nothing to cite: the gate says so.
    let tree = Tree::new("unfrozen");
    // FDC-004: Run evidence resides in the Engine's addressed run directory.
    fs::remove_file(
        tree.root
            .join(".ratmac/runs")
            .join(RUN)
            .join("evidence.toml"),
    )
    .expect("remove evidence");
    let defects = records(&tree).expect_err("an unfrozen goal must refuse");
    assert!(
        reasons(&defects).to_ascii_lowercase().contains("frozen"),
        "the refusal says the goal is not frozen: {}",
        reasons(&defects)
    );
}

/// ARF-001 / ARFV-001: a fixture with a past. An archived record citing an
/// older freeze passes; a live record citing the same older freeze refuses;
/// an archived record with no parseable citation refuses. The fixture that
/// could not exist before i-029: every earlier tree was born at the current
/// freeze, which is exactly why an unpassable gate looked green.
#[test]
fn an_archived_record_cites_its_own_freeze_and_a_live_one_cites_todays() {
    const OLDER: &str = "2222222222222222222222222222222222222222222222222222222222222222";

    // Second requirement so the archived record is not a duplicate.
    let tree = Tree::new("aged");
    fs::write(
        tree.root.join(".arca/goal/spec.md"),
        "# Goal spec\n\n\
         | Req ID | Requirement | Source |\n\
         |---|---|---|\n\
         | DEMO-001 | The demo behaves. | [issue DEMO-001](../issue/i-100-demo/spec.md#requirement-records) |\n\
         | DEMO-002 | The past behaves. | [issue DEMO-002](../issue/i-100-demo/spec.md#requirement-records) |\n",
    )
    .expect("widen goal spec");
    tree.write_archived_residual("res-101", "DEMO-002", OLDER);

    records(&tree)
        .expect("an archived record citing the freeze it was judged under is not a defect");

    // The same older citation on a live record still refuses, same wording.
    tree.write_residual("res-102", "DEMO-002", "missing", OLDER, &[]);
    tree.write_ticket("t-101", &["res-102"], &[]);
    let defects = records(&tree).expect_err("a live record must cite today's freeze");
    assert!(
        defects.iter().any(|defect| {
            defect.artifact.contains("res-102")
                && defect.reason.contains("but the frozen revision is")
        }),
        "the live record is the defect, with the original wording: {defects:?}"
    );
    assert!(
        !defects
            .iter()
            .any(|defect| defect.artifact.contains("res-101")),
        "the archived record is still not the defect: {defects:?}"
    );
    fs::remove_file(tree.root.join(".arca/residual/res-102.md")).expect("drop live record");
    fs::remove_file(tree.root.join(".arca/ticket/t-101.md")).expect("drop ticket");

    // Age is never a free pass: an archived record with no parseable citation
    // refuses wherever it lives.
    fs::write(
        tree.root.join(".arca/residual/archive/res-101.md"),
        "# Residual Record\n\n```yaml\n\
         residual-id: \"res-101\"\n\
         goal-requirement-ref: \"DEMO-002\"\n\
         frozen-goal-bundle-revision: \"pending\"\n\
         concrete-evidence-refs:\n  - \"src/demo.rs\"\n\
         status: \"satisfied\"\n```\n",
    )
    .expect("corrupt the citation");
    let defects = records(&tree).expect_err("a citation must exist and parse in the archive");
    assert!(
        defects.iter().any(|defect| {
            defect.artifact.contains("res-101")
                && defect
                    .reason
                    .contains("no parseable frozen-goal-bundle-revision")
        }),
        "the archived record without a citation is the defect: {defects:?}"
    );
}

/// GPH intake fold surfaced two record-gate gaps. First: PCR-008 lets a
/// requirement live as a working-authority heading, so a gap record citing
/// such a requirement is a legal citation, not "absent from the goal
/// authority". The one-record-per-requirement demand still binds goal rows
/// only, because working-authority requirements deliberately mint no gap row.
#[test]
fn a_record_may_cite_a_working_authority_requirement() {
    let tree = Tree::new("authority-citation");
    tree.write_goal();
    fs::write(
        tree.root.join(".arca/schema.md"),
        "# Working rules\n\n### WKA-001 - a working-authority requirement\n\nBinds contributors.\n",
    )
    .expect("write working authority");
    let runbook = fs::read_to_string(tree.root.join(".ratmac/ratmac.toml")).expect("read runbook");
    fs::write(
        tree.root.join(".ratmac/ratmac.toml"),
        runbook.replace("[roots]\n", "[roots]\nauthority = \".arca\"\n"),
    )
    .expect("declare the authority root");
    tree.write_residual("res-100", "DEMO-001", "satisfied", FROZEN, &["src/lib.rs"]);
    tree.write_residual("res-101", "WKA-001", "satisfied", FROZEN, &["schema.md"]);
    let verdict = records(&tree);
    let text = match &verdict {
        Ok(()) => String::new(),
        Err(defects) => reasons(defects),
    };
    assert!(
        verdict.is_ok(),
        "a citation to a working-authority heading is legal: {text}"
    );
}

/// Second: a runbook may mechanize the per-ticket gates inside a child class
/// (the cycle runbook does exactly that), so the mechanization scan must read
/// class states too, not only the top-level States.
#[test]
fn gates_declared_in_a_child_class_count_as_mechanized() {
    let tree = Tree::new("class-mechanization");
    tree.write_goal();
    let runbook = fs::read_to_string(tree.root.join(".ratmac/ratmac.toml")).expect("read runbook");
    // Strip the per-ticket gate from the top level and move it into a class.
    let runbook = runbook.replace(
        "guards = [{ kind = \"sensitivity_receipts\", root = \"ticket\", ticket = \"t-100.md\" }]",
        "guards = []",
    );
    let runbook = format!(
        "{runbook}\n[classes.ticket]\n\n[classes.ticket.states.work]\nprompt = \"Work.\"\nguards = [{{ kind = \"sensitivity_receipts\", ticket-binding = \"item\", root = \"ticket\" }}, {{ kind = \"completion_gate\", ticket-binding = \"item\", root = \"ticket\" }}]\n"
    );
    fs::write(tree.root.join(".ratmac/ratmac.toml"), runbook).expect("write class runbook");
    let unproven = ratmac::contract::unproven_mechanization(&tree.root);
    let text = unproven
        .iter()
        .map(|defect| format!("{defect}"))
        .collect::<Vec<_>>()
        .join("; ");
    assert!(
        !text.contains("sensitivity_receipts") && !text.contains("completion_gate"),
        "gates declared in a child class are mechanized: {text}"
    );
}

/// ARFV-003 / ARF-002: a re-judgment reaches into the archive. While a live
/// `missing` record stands against a requirement whose archived record still
/// says `satisfied`, the gate refuses naming both; once the archived record is
/// moved back to the active folder (the re-judgment replacing it) with an
/// owning ticket, the gate passes. A `satisfied` claim resting on a
/// requirement no gate mechanizes still refuses on the aged fixture.
#[test]
fn a_rejudged_archived_record_refuses_until_it_moves_back() {
    const OLD_FROZEN: &str = "2222222222222222222222222222222222222222222222222222222222222222";
    let tree = Tree::new("rejudged-archive");
    // Second requirement beside the seeded DEMO-001.
    fs::write(
        tree.root.join(".arca/goal/spec.md"),
        "# Goal spec\n\n\
         | Req ID | Requirement | Source |\n\
         |---|---|---|\n\
         | DEMO-001 | The demo behaves. | [issue DEMO-001](../issue/i-100-demo/spec.md#requirement-records) |\n\
         | DEMO-002 | The past behaves. | [issue DEMO-002](../issue/i-100-demo/spec.md#requirement-records) |\n",
    )
    .expect("widen goal spec");
    // The aged half: an archived record from freeze A, satisfied.
    tree.write_archived_residual("res-150", "DEMO-002", OLD_FROZEN);
    // The re-judgment: a live missing record against the same requirement.
    tree.write_residual("res-151", "DEMO-002", "missing", FROZEN, &[]);
    tree.write_ticket("t-150", &["res-151"], &[]);
    let refused = records(&tree)
        .expect_err("a live re-judgment beside an archived satisfied record must refuse");
    let text = reasons(&refused);
    assert!(
        text.contains("res-150") && text.contains("res-151"),
        "the refusal names both records: {text}"
    );
    // The move back: the re-judgment replaces the archived record in the
    // active folder; one record per requirement again, owned.
    fs::remove_file(tree.root.join(".arca/residual/archive/res-150.md")).expect("archive move");
    let verdict = records(&tree);
    let text = match &verdict {
        Ok(()) => String::new(),
        Err(defects) => reasons(defects),
    };
    assert!(verdict.is_ok(), "the moved-back record passes: {text}");
}

/// ARFV-004 / ARF-001, ARF-003: the check that could not run before this
/// issue. The record gate's expected verdict on this repository as it stands
/// is green, and it keeps passing as the archive grows, because an archived
/// record answers to its own freeze.
#[test]
fn the_record_gate_passes_on_this_repository_as_it_stands() {
    let root = ratmac_qa::baseline::repo_root();
    // The freeze is computed from the tracked goal bundle, exactly as the
    // Engine computes it, so this check needs no machine-local Run state and
    // keeps running on a fresh clone as the archive grows.
    let frozen = ratmac::goal::revision(&root.join(".arca/goal"))
        .expect("read the goal bundle")
        .expect("this repository has a goal bundle");
    let engine = std::env::temp_dir().join(format!(
        "ratmac-t048-selfcheck-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    let run_dir = engine.join("runs/run-000");
    fs::create_dir_all(&run_dir).expect("create synthetic run");
    fs::write(
        run_dir.join("evidence.toml"),
        format!("[goal]\nfrozen = \"{frozen}\"\n"),
    )
    .expect("write synthetic freeze");
    let verdict = gate_records(&root, &engine, "run-000");
    let _ = fs::remove_dir_all(&engine);
    let text = match &verdict {
        Ok(()) => String::new(),
        Err(defects) => reasons(defects),
    };
    assert!(
        verdict.is_ok(),
        "this repository as it stands passes its own record gate: {text}"
    );
}

/// GPHV-001 (t-098): the intake contract on a tree whose archive carries an
/// issue bundle and a ticket from an earlier freeze. Age is legitimate here,
/// so the gate's stated verdict is a pass.
#[test]
fn the_intake_gate_passes_on_a_tree_with_a_past() {
    let mut tree = ratmac_qa::aged::AgedTree::new(
        "intake-past",
        "1111111111111111111111111111111111111111111111111111111111111111",
    );
    write_aged_runbook(&tree.root);
    write_archived_bundle(&tree.root, "i-101-old");
    write_archived_ticket(&tree.root, "t-050", "res-050");
    tree.advance_to("3333333333333333333333333333333333333333333333333333333333333333");
    assert!(tree.age() >= 1, "the fixture carries a past");
    let verdict = gate_intake(&tree.root);
    assert!(
        verdict.is_ok(),
        "intake passes on a tree with a past: {:?}",
        verdict.err()
    );
}

/// GPHV-002 (t-098): age is never a free pass. A corrupted archived bundle
/// still refuses by name; history is checked, never waved through.
#[test]
fn age_is_never_a_free_pass() {
    let mut tree = ratmac_qa::aged::AgedTree::new(
        "intake-corrupt",
        "1111111111111111111111111111111111111111111111111111111111111111",
    );
    write_aged_runbook(&tree.root);
    write_archived_bundle(&tree.root, "i-101-old");
    tree.advance_to("3333333333333333333333333333333333333333333333333333333333333333");
    std::fs::remove_file(tree.root.join(".arca/issue/archive/i-101-old/design.md"))
        .expect("corrupt the archived bundle");
    let defects = gate_intake(&tree.root).expect_err("a corrupted archived bundle refuses");
    let text = format!("{defects:?}");
    assert!(
        text.contains("i-101-old"),
        "the refusal names the corrupted artifact: {text}"
    );
}

/// The minimal runbook the intake gate demands at the fixture's Engine path.
fn write_aged_runbook(root: &std::path::Path) {
    std::fs::write(
        root.join(".ratmac/ratmac.toml"),
        "[roots]\n\
         goal = \".arca/goal\"\n\
         issue = \".arca/issue\"\n\
         residual = \".arca/residual\"\n\
         ticket = \".arca/ticket\"\n\
         \n\
         [states.intake]\n\
         prompt = \"Integrate.\"\n\
         guards = [{ kind = \"intake_contract\" }]\n\
         \n\
         [states.rest]\n\
         prompt = \"Rest.\"\n",
    )
    .expect("write the aged runbook");
}

/// An archived five-file issue bundle from an earlier point in the tree's story.
fn write_archived_bundle(root: &std::path::Path, id: &str) {
    let dir = root.join(".arca/issue/archive").join(id);
    std::fs::create_dir_all(&dir).expect("create archived bundle");
    std::fs::write(
        dir.join("index.md"),
        format!(
            "# Issue {id}\n\n```yaml\nissue-id: \"{id}\"\nstatus: \"integrated\"\n```\n\n\
             See [goal spec](../../../goal/spec.md).\n"
        ),
    )
    .expect("write archived index");
    std::fs::write(
        dir.join("spec.md"),
        "# Issue specification\n\n## Requirement Records\n\n\
         | Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |\n\
         | :--- | :--- | :--- | :--- | :--- |\n\
         | `DEMO-001` | The demo behaves. | accepted | Demo. | [goal spec](../../../goal/spec.md) |\n",
    )
    .expect("write archived spec");
    for leaf in ["design.md", "test-plan.md", "ubi-lang.md"] {
        std::fs::write(dir.join(leaf), "# Archived\n\nArchived.\n").expect("write archived leaf");
    }
}

/// An archived ticket whose residual took the archive move with it.
fn write_archived_ticket(root: &std::path::Path, id: &str, residual: &str) {
    let dir = root.join(".arca/ticket/archive");
    std::fs::create_dir_all(&dir).expect("create ticket archive");
    std::fs::write(
        dir.join(format!("{id}.md")),
        format!("---\nticket-id: \"{id}\"\nresidual-ids:\n  - \"{residual}\"\nstatus: \"passed\"\n---\n\n# Ticket: {id}\n"),
    )
    .expect("write archived ticket");
}
