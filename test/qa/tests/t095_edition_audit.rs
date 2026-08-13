//! t-095 / EDN-001: every edition is annotated, documents the bar, and reachable.
//!
//! `EDNV-004` `every_edition_is_annotated_documented_and_reachable`
//!
//! The closing guard `t-094` landed proves that a tag named `edition-*` points at
//! the commit being left. It cannot tell an annotated tag from a lightweight one
//! and cannot read the message - `EDN-002` states that limit itself. This is the
//! check that reads them, so an edition cut wrong is found by a machine instead
//! of by a person following a citation that stopped meaning anything.
//!
//! The audit itself lives in `ratmac_qa::edition` so the private lanes judge the
//! same code rather than a second copy of the rule.

use ratmac_qa::edition::{
    audit_editions, audit_sequence, edition_tags, repo_root, report, AuditFinding,
    EXAMPLE_BAR_MESSAGE,
};
use ratmac_qa::tempgit::TempRepo;

/// A repository with one commit on `main` and nothing else.
fn base(label: &str) -> TempRepo {
    let repo = TempRepo::new(label);
    repo.write("src/lib.rs", "pub fn fixture() {}\n");
    repo.commit_all("fixture base");
    repo
}

/// `EDNV-004`: this repository's editions are all annotated, all record the bar,
/// and all sit on the trunk - and each planted defect is reported by name.
#[test]
fn every_edition_is_annotated_documented_and_reachable() {
    let root = ratmac_qa::edition::repo_root();
    let findings = audit_editions(&root).expect("audit this repository's editions");
    assert!(
        findings.is_empty(),
        "every edition of this repository is annotated, documented, and reachable:\n{}",
        report(&findings)
    );
    assert!(
        !edition_tags(&root)
            .expect("list this repository's editions")
            .is_empty(),
        "this repository has at least one edition, so the audit above proved something"
    );

    let lightweight = base("t095-lightweight");
    lightweight.git(&["tag", "edition-001"]);
    assert_eq!(
        audit_editions(lightweight.root()).expect("audit the lightweight edition"),
        vec![AuditFinding {
            tag: "edition-001".to_owned(),
            property: "annotated".to_owned(),
            detail: "the tag object is a commit, so nothing records what was proven".to_owned(),
        }],
        "a lightweight edition is reported as not annotated, and only as that"
    );

    let blank = base("t095-blank");
    blank.git(&["tag", "-a", "edition-001", "-m", ""]);
    let findings = audit_editions(blank.root()).expect("audit the blank edition");
    assert_eq!(findings.len(), 1, "one finding:\n{}", report(&findings));
    assert_eq!(findings[0].property, "documented");
    assert!(
        findings[0].detail.contains("the message is empty"),
        "an empty message is named as empty rather than as a list of missing phrases: {}",
        findings[0].detail
    );

    let undocumented = base("t095-undocumented");
    undocumented.git(&["tag", "-a", "edition-001", "-m", "cut it"]);
    let findings = audit_editions(undocumented.root()).expect("audit the undocumented edition");
    assert_eq!(findings.len(), 1, "one finding:\n{}", report(&findings));
    assert_eq!(findings[0].property, "documented");
    assert!(
        findings[0].detail.contains("cargo test")
            && findings[0].detail.contains("rtm doctor")
            && findings[0].detail.contains("Proven at this commit:"),
        "the report names the phrases the message is missing: {}",
        findings[0].detail
    );

    let unreachable = base("t095-unreachable");
    unreachable.git(&["checkout", "--quiet", "-b", "sidetrack"]);
    unreachable.write("src/lib.rs", "pub fn aside() {}\n");
    unreachable.commit_all("work that never merged");
    unreachable.git(&["tag", "-a", "edition-001", "-m", EXAMPLE_BAR_MESSAGE]);
    unreachable.git(&["checkout", "--quiet", "main"]);
    let findings = audit_editions(unreachable.root()).expect("audit the unreachable edition");
    assert_eq!(findings.len(), 1, "one finding:\n{}", report(&findings));
    assert_eq!(findings[0].property, "reachable");
    assert!(
        findings[0].detail.contains("not an ancestor of main"),
        "the report says what the commit is not reachable from: {}",
        findings[0].detail
    );

    let good = base("t095-good");
    good.git(&["tag", "-a", "edition-001", "-m", EXAMPLE_BAR_MESSAGE]);
    assert!(
        audit_editions(good.root())
            .expect("audit a well-formed edition")
            .is_empty(),
        "an annotated edition recording the bar passes"
    );
}

/// Write the tracked record of what each edition marks.
fn ledger(repo: &TempRepo, rows: &[(&str, &str)]) {
    let mut text = String::from(
        "# Editions\n\n| Edition | Commit | What it marks |\n| :--- | :--- | :--- |\n",
    );
    for (tag, commit) in rows {
        text.push_str(&format!("| `{tag}` | `{commit}` | fixture |\n"));
    }
    repo.write(".arca/editions.md", &text);
    repo.commit_all("record the edition");
}

/// `EDNV-004`, sequence half: the numbers run from `001` with no hole and no
/// duplicate, and every edition still marks the commit the tracked ledger
/// records. Version control cannot refuse a moved tag, so the committed record
/// is the only thing a move can disagree with.
#[test]
fn the_sequence_has_no_holes_and_no_edition_has_moved() {
    let findings = audit_sequence(&repo_root()).expect("audit this repository's edition sequence");
    assert_eq!(
        findings,
        Vec::new(),
        "this repository's editions are sequential, unique, and unmoved"
    );

    // A hole: the sequence reaches 002 with no 001.
    let hole = base("t096-hole");
    ledger(&hole, &[("edition-002", &hole.head())]);
    hole.git(&["tag", "-a", "edition-002", "-m", EXAMPLE_BAR_MESSAGE]);
    let findings = audit_sequence(hole.root()).expect("audit a sequence with a hole");
    let hole_finding = findings
        .iter()
        .find(|finding| finding.tag == "edition-001" && finding.property == "sequence")
        .unwrap_or_else(|| panic!("a missing number is reported by itself: {findings:?}"));
    assert!(
        hole_finding.detail.contains("002") && hole_finding.detail.contains("001"),
        "the report names the number reached and the number missing: {}",
        hole_finding.detail
    );

    // A move: the tag no longer marks the commit the ledger records.
    let moved = base("t096-moved");
    let first = moved.head();
    ledger(&moved, &[("edition-001", &first)]);
    moved.git(&["tag", "-a", "edition-001", "-m", EXAMPLE_BAR_MESSAGE]);
    moved.write("src/lib.rs", "pub fn later() {}\n");
    moved.commit_all("later work");
    moved.git(&["tag", "-f", "-a", "edition-001", "-m", EXAMPLE_BAR_MESSAGE]);
    let moved_head = moved.head();
    let findings = audit_sequence(moved.root()).expect("audit a moved edition");
    let move_finding = findings
        .iter()
        .find(|finding| finding.tag == "edition-001" && finding.property == "immutable")
        .unwrap_or_else(|| panic!("a moved edition is reported: {findings:?}"));
    assert!(
        move_finding.detail.contains(&first) && move_finding.detail.contains(&moved_head),
        "the report names the recorded commit and the one the tag now marks: {}",
        move_finding.detail
    );

    // An unrecorded edition: nothing to disagree with is itself the defect.
    let unrecorded = base("t096-unrecorded");
    ledger(&unrecorded, &[("edition-001", &unrecorded.head())]);
    unrecorded.git(&["tag", "-a", "edition-001", "-m", EXAMPLE_BAR_MESSAGE]);
    unrecorded.write("src/lib.rs", "pub fn later() {}\n");
    unrecorded.commit_all("later work");
    unrecorded.git(&["tag", "-a", "edition-002", "-m", EXAMPLE_BAR_MESSAGE]);
    let findings = audit_sequence(unrecorded.root()).expect("audit an unrecorded edition");
    assert!(
        findings
            .iter()
            .any(|finding| finding.tag == "edition-002" && finding.property == "immutable"),
        "an edition the ledger never recorded is reported: {findings:?}"
    );

    // A malformed number is not an alternative spelling.
    let malformed = base("t096-malformed");
    ledger(&malformed, &[("edition-001", &malformed.head())]);
    malformed.git(&["tag", "-a", "edition-001", "-m", EXAMPLE_BAR_MESSAGE]);
    malformed.git(&["tag", "-a", "edition-0002", "-m", EXAMPLE_BAR_MESSAGE]);
    let findings = audit_sequence(malformed.root()).expect("audit a malformed edition name");
    assert!(
        findings
            .iter()
            .any(|finding| finding.tag == "edition-0002" && finding.property == "named"),
        "edition-0002 is malformed, not edition two: {findings:?}"
    );

    // A missing ledger refuses; absence never reads as agreement.
    let absent = base("t096-no-ledger");
    absent.git(&["tag", "-a", "edition-001", "-m", EXAMPLE_BAR_MESSAGE]);
    let refusal = audit_sequence(absent.root()).expect_err("a missing ledger refuses");
    assert!(
        refusal.contains(".arca/editions.md"),
        "the refusal names the record that is missing: {refusal}"
    );
}
