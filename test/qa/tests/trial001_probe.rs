//! TRIAL-001 throwaway probe — not for merge.
//! Question: does the PGE-001 intake gate catch an issue folder that is
//! nothing but unfilled template blanks?

use std::path::PathBuf;

#[test]
fn blank_issue_against_intake_gate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root")
        .to_path_buf();

    let probe = root.join(".arca/issue/i-011-issue-authoring-scaffold");
    assert!(probe.is_dir(), "probe issue folder must exist at {probe:?}");

    // Facts about the probe, stated so the report is self-contained.
    let index = std::fs::read_to_string(probe.join("index.md")).unwrap();
    let mut blanks = 0usize;
    for file in [
        "index.md",
        "ubi-lang.md",
        "spec.md",
        "design.md",
        "test-plan.md",
    ] {
        let text = std::fs::read_to_string(probe.join(file)).unwrap();
        blanks += text.matches("{{").count();
    }

    println!("--- probe facts ---");
    println!("unfilled {{{{...}}}} placeholders : {blanks}");
    println!(
        "issue-id field              : {}",
        index
            .lines()
            .find(|l| l.starts_with("issue-id:"))
            .unwrap_or("<absent>")
    );
    println!(
        "provenance field            : {}",
        index
            .lines()
            .find(|l| l.starts_with("provenance:"))
            .unwrap_or("<absent>")
    );
    println!("folder name                 : i-011-issue-authoring-scaffold");

    let verdict = ratmac::contract::gate_intake(&root);
    println!("--- gate_intake verdict ---");
    match &verdict {
        Ok(()) => println!("PASS - the gate accepted this folder"),
        Err(defects) => {
            println!("REFUSED with {} defect(s):", defects.len());
            for d in defects {
                println!("  {d}");
            }
        }
    }

    assert!(blanks > 0, "probe must still contain unfilled blanks");
}
