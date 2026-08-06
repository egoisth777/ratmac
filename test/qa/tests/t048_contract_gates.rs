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
            "[phases.intake]\n\
             prompt = \"Integrate.\"\n\
             guards = [{ kind = \"intake_contract\" }]\n\
             \n\
             [phases.gaps]\n\
             prompt = \"Find gaps.\"\n\
             guards = [{ kind = \"record_contract\" }]\n\
             \n\
             [phases.build]\n\
             prompt = \"Build.\"\n\
             guards = [{ kind = \"sensitivity_receipts\", ticket = \".arca/ticket/t-100.md\" }]\n\
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
        let output = Command::new(env!("CARGO_BIN_EXE_rtm"))
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
    // gate refuses the step and leaves the Phase where it was.
    tree.rtm(&["start"]);
    // FDC-004: address the live run — the Engine roster entry carries its State File.
    let live = fs::read_dir(tree.root.join(".ratmac/runs"))
        .expect("list the runs roster")
        .map(|entry| entry.expect("roster entry is readable"))
        .find(|entry| entry.path().join("state.toml").is_file())
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
        "a refused gate leaves the Phase unchanged: {status}"
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
        "[phases.build]\nprompt = \"Build.\"\n\n[phases.done]\nprompt = \"Done.\"\n\n\
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
