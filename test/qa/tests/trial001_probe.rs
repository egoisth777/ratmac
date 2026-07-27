//! TRIAL-001 throwaway probe — not for merge.

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root")
        .to_path_buf()
}

/// The issue authored end-to-end through the intended path, status `pending`.
#[test]
fn fully_authored_pending_issue() {
    let root = root();
    let issue = root.join(".arca/issue/i-011-issue-authoring-scaffold");

    let mut blanks = 0usize;
    for file in [
        "index.md",
        "ubi-lang.md",
        "spec.md",
        "design.md",
        "test-plan.md",
    ] {
        blanks += std::fs::read_to_string(issue.join(file))
            .unwrap()
            .matches("{{")
            .count();
    }
    println!("--- fully authored issue ---");
    println!("unfilled placeholders remaining : {blanks}");
    assert_eq!(blanks, 0, "the issue is authored, nothing left blank");

    println!("--- gate_intake verdict on the finished, pending issue ---");
    match ratmac::contract::gate_intake(&root) {
        Ok(()) => println!("PASS"),
        Err(defects) => {
            println!("REFUSED with {} defect(s):", defects.len());
            for d in &defects {
                println!("  {d}");
            }
        }
    }
}

/// IAS-005: disposition is read from the whole row, not the column.
#[test]
fn disposition_is_read_from_the_whole_row() {
    let spec = "\
| Requirement ID | Requirement | Disposition | Rationale | Refs |
| :--- | :--- | :--- | :--- | :--- |
| `ZZZ-001` | some ask | rejected | we have not accepted this, and will not | none |
";
    // Mirror of src/contract.rs::accepted_requirements.
    let mut counted = Vec::new();
    for line in spec.lines() {
        let t = line.trim();
        if !t.starts_with('|') || !t.to_ascii_lowercase().contains("accepted") {
            continue;
        }
        let first = t
            .trim_start_matches('|')
            .split('|')
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches('`')
            .to_owned();
        counted.push(first);
    }

    println!("--- IAS-005 probe ---");
    println!("row disposition column : rejected");
    println!("rationale contains     : \"not accepted\"");
    println!("parser counted as accepted : {counted:?}");
    assert_eq!(
        counted,
        vec!["ZZZ-001".to_string()],
        "demonstrates the defect: a rejected row is counted accepted"
    );
}
