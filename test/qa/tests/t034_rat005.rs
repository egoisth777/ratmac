//! PT-034-01: active stale-name audit with an explicit historical allowlist.
//!
//! SVC-008: the walk and the list live in `ratmac_qa::rebrand`, so this lane,
//! the acceptance lane, and the state-vocabulary lane read one list in one run.

use ratmac_qa::rebrand;

#[test]
fn active_reference_audit() {
    let root = rebrand::repo_root();
    let rules = rebrand::load_allowlist(&rebrand::allowlist_path(&root))
        .expect("rebrand audit allowlist must load");
    assert!(!rules.is_empty(), "allowlist must contain active rules");

    let report = rebrand::audit(&root, &rules);
    assert!(
        report.violations.is_empty(),
        "unallowlisted active legacy references:\n{}",
        report.violations.join("\n")
    );
    assert!(
        report.stale.is_empty(),
        "stale allowlist entries:\n{}",
        report.stale.join("\n")
    );

    // These files are historical inputs to the audit, not outputs it may rewrite.
    assert!(
        root.join(".arca/log.md").is_file(),
        "append-only transition log must remain present"
    );
    assert!(
        root.join(".arca/ticket/archive").is_dir(),
        "archived ticket history must remain present"
    );
}
