//! t-106 / RCR-001, RCR-002, RCR-003: every gap-record citation resolves
//! against the repository, and rot is named.
//!
//! RCRV-001 `the_working_rules_state_the_stamp_landing_order_and_no_live_rule_self_cites`
//! RCRV-002 `the_check_refuses_exactly_the_rotted_citations_and_the_allowlist_matches`
//! RCRV-003 `reachability_binds_any_ref_and_dangling_is_a_refusal`
//! RCRV-004 `a_repoint_is_one_stamp_step_and_the_archive_move_refuses_until_it_resolves`
//!
//! The surface under test is `ratmac_qa::citations`: the checker resolves
//! every `git:` citation in every gap record's `implementation-revision` -
//! active folder and archive counted as one namespace - against the
//! repository's refs, refusing a record whose cited commit is unreachable.
//! Reachability binds any ref, so a tag-held edition commit passes, while a
//! hash that answers `git cat-file` only as a dangling object is exactly
//! the refusal. Byte-frozen rot rides the enumerated allowlist, whose rows
//! name record, hash, and reason, and a row that matches nothing fails as
//! stale. The mechanical re-point is one stamp step re-deriving the
//! record's citation and the owning ticket's `landed-commit` from the
//! repository in the same change, and the archive move refuses until both
//! resolve.
//!
//! Fixtures follow `GPH-001` and `GPH-003`: RCRV-002 runs against this
//! repository's own rotted archive - the eleven citations are the history
//! the check walks, read-only - and RCRV-003/RCRV-004 build throwaway git
//! repositories with `tempgit`, never this repository's real history.
//! Expected hashes are read from the records by the test's own scanner,
//! independent of the checker under test.
//!
//! Hole-poke notes:
//! - Would RCRV-002 pass a checker that stops at the first refusal? No.
//!   The without-allowlist verdict must name exactly the eleven rotted
//!   citations - set equality, no more, no fewer - so a checker that
//!   answers one refusal, or eleven vague ones, fails.
//! - Would RCRV-002 pass a checker that admits the allowlist as a
//!   wildcard? No. Without it the check must refuse and with it the check
//!   must pass over the same records - an always-pass fails the first
//!   half, an always-refuse the second - and deleting a matched row's
//!   record must fail exactly that row as stale, named by record and
//!   hash. A row count short of the rotted set fails the row-per-citation
//!   equality.
//! - Would RCRV-003 pass a checker that equates existence with
//!   reachability (`git cat-file -e` alone)? No. The amend-orphaned commit
//!   still answers `cat-file` - the test asserts that itself before
//!   demanding the refusal - so only ref-based reachability can refuse it.
//! - Would RCRV-003 pass a checker that reads reflogs as refs, or binds
//!   reachability to `HEAD`'s ancestry alone? No. The orphaned commit sits
//!   in `HEAD`'s reflog and must still refuse; the tag-held commit is on
//!   no branch and no ancestor of `HEAD` and must still pass.
//! - Would RCRV-004 pass a stamp step that re-points only the record, or
//!   predicts the hash from memory? No. With the record clean and the
//!   ticket's `landed-commit` still rotten the archive move must refuse
//!   naming the ticket; after the stamp both on-disk fields must equal the
//!   amended tip one call resolved; and a stamp over a repository with no
//!   landed commit at all must refuse rather than invent a hash.
//! - Would RCRV-001 pass a scan content with the word "stamp"? No. Both
//!   clauses are pinned - the stamp landing following the merged green
//!   landing, and the hash derived by resolving the landed tip - a
//!   control text without the rule refuses, and a control rule that
//!   instructs a record to carry its own landing's hash is detected by
//!   name.

use std::fs;

use ratmac_qa::citations::{
    archive_record, check_records, read_allowlist, read_records, self_citing_rules,
    stamp_landing_order, stamp_step, RecordFolder,
};
use ratmac_qa::grown::repo_root;
use ratmac_qa::tempgit::TempRepo;

/// The eleven rotted citations found at filing (RCRV-002): res-116/117,
/// res-124/125, res-147..153. The hashes are read from the records at run
/// time by this test's own scanner, so the enumeration stays the filing
/// contract while the bytes stay the records'.
const ROTTED_IDS: [&str; 11] = [
    "res-116", "res-117", "res-124", "res-125", "res-147", "res-148", "res-149", "res-150",
    "res-151", "res-152", "res-153",
];

/// A schema-shaped control text that states the stamp-landing order in the
/// requirement's own words: both halves - the landing order, and the
/// repository-derived hash.
const ORDER_STATED: &str = "\
## Units and git

One commit = one landing = one log.md line.

### RCR-900 - the stamp landing follows the green landing

A gap record's implementation-revision is written only in a landing that
follows the green landing it cites: the green landing merges to `main`
first, then a stamp landing flips the record `satisfied` and derives the
hash from the repository at that moment - the commit the landed tip
resolves to - never from memory or prediction.
";

/// The same shape with the stamp-landing rule absent: nothing states the
/// order, so the reader must refuse rather than agree.
const ORDER_ABSENT: &str = "\
## Units and git

One commit = one landing = one log.md line.
";

/// A schema-shaped control text whose live rule instructs a record to
/// carry the hash of the commit it lands inside - the self-cite the scan
/// exists to catch, by name.
const SELF_CITE_RULE: &str = "\
## Units and git

One commit = one landing = one log.md line.

### XXX-900 - the citation rides its own landing

A gap record's implementation-revision is written in the landing it cites,
carrying that landing's own hash in that same commit.
";

/// A minimal gap record in the house format: the yaml fence carries the
/// `implementation-revision` citation, the footer names the owning ticket.
fn fixture_record(id: &str, revision: &str, owner: &str) -> String {
    format!(
        "# Residual Record\n\n\
         ```yaml\n\
         residual-id: \"{id}\"\n\
         goal-requirement-ref: \"RCR-002\"\n\
         frozen-goal-bundle-revision: \"git:0000000+goal-sha256:0000\"\n\
         implementation-revision: \"{revision}\"\n\
         classification-rationale: \"fixture: grown in a throwaway repository\"\n\
         status: \"satisfied\"\n\
         required-test-refs:\n\
           - \"RCRV-003\"\n\
         ```\n\n\
         Owned by ticket `{owner}`.\n"
    )
}

/// A minimal ticket whose front matter carries the `landed-commit` the
/// stamp step re-derives.
fn fixture_ticket(id: &str, landed: &str) -> String {
    format!(
        "---\n\
         ticket-id: \"{id}\"\n\
         status: \"approved\"\n\
         landed-commit: \"{landed}\"\n\
         ---\n\n\
         # Ticket: {id}\n\n\
         ## Merge Gate\n\n\
         - Quality: `cargo test --workspace` passes.\n"
    )
}

/// The test's own field reader: the first `field: "value"` line's value,
/// verbatim between its quotes - independent of the checker under test.
fn field_value(text: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}:");
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with(&prefix))?;
    let value = line.trim().strip_prefix(&prefix)?.trim();
    Some(value.strip_prefix('"')?.strip_suffix('"')?.to_owned())
}

/// The `git:` hash of one record's `implementation-revision`, as the
/// test's own scanner reads it: the hex run after `git:`.
fn cited_hash(record_text: &str) -> String {
    let value = field_value(record_text, "implementation-revision")
        .unwrap_or_else(|| panic!("the record carries an implementation-revision"));
    let hash: String = value
        .strip_prefix("git:")
        .unwrap_or_else(|| panic!("the citation is a git: citation: {value}"))
        .chars()
        .take_while(char::is_ascii_hexdigit)
        .collect();
    assert!(!hash.is_empty(), "the citation names a hash: {value}");
    hash
}

/// One check's refusal, demanded rather than excused.
fn demanded_err(
    checked: Result<(), Vec<ratmac_qa::citations::CitationRefusal>>,
    what: &str,
) -> Vec<ratmac_qa::citations::CitationRefusal> {
    match checked {
        Err(refusals) => refusals,
        Ok(()) => panic!("RCRV: {what} must refuse, not pass"),
    }
}

/// RCRV-001 (t-106, RCR-001): the working rules state the stamp-landing
/// order - the stamp landing follows the merged green landing; the hash is
/// derived by resolving the landed tip - and no live rule instructs a
/// record to carry the hash of a commit it lands inside.
#[test]
fn the_working_rules_state_the_stamp_landing_order_and_no_live_rule_self_cites() {
    let root = repo_root();
    let schema = fs::read_to_string(root.join(".arca/schema.md"))
        .unwrap_or_else(|error| panic!("read this repository's working rules: {error}"));

    // The real working rules state the order, both halves of it.
    let order = stamp_landing_order(&schema).unwrap_or_else(|refusal| {
        panic!("RCRV-001: the working rules must state the stamp-landing order: {refusal}")
    });
    let order_words = format!("{} {}", order.order_clause, order.derivation_clause).to_lowercase();
    assert!(
        order_words.contains("stamp landing") && order_words.contains("green landing"),
        "RCRV-001: the stated order names the stamp landing and the green landing it follows: {order:?}"
    );
    assert!(
        order.derivation_clause.to_lowercase().contains("resolv"),
        "RCRV-001: the hash is derived by resolving the landed tip: {order:?}"
    );

    // Control texts: a rule stated in plain words reads back; the same
    // shape without the rule refuses rather than agrees.
    let stated = stamp_landing_order(ORDER_STATED).unwrap_or_else(|refusal| {
        panic!("RCRV-001: a text stating the order is read as stating it: {refusal}")
    });
    let stated_words =
        format!("{} {}", stated.order_clause, stated.derivation_clause).to_lowercase();
    assert!(
        stated_words.contains("stamp landing")
            && stated_words.contains("green landing")
            && stated_words.contains("resolv"),
        "RCRV-001: the control text reads back with the order clause and the derivation clause: {stated:?}"
    );
    assert!(
        stamp_landing_order(ORDER_ABSENT).is_err(),
        "RCRV-001: with no rule stating the order, the reader refuses"
    );

    // The scan over every live rule text: nothing instructs a record to
    // carry the hash of a commit it lands inside.
    let offenders = self_citing_rules(&schema).unwrap_or_else(|refusal| {
        panic!("RCRV-001: the self-citation scan must run over the live rules: {refusal}")
    });
    assert!(
        offenders.is_empty(),
        "RCRV-001: no live rule may instruct a record to carry the hash of a commit it lands inside: {offenders:?}"
    );
    let detected = self_citing_rules(SELF_CITE_RULE).unwrap_or_else(|refusal| {
        panic!("RCRV-001: the scan must run over a control schema too: {refusal}")
    });
    assert!(
        !detected.is_empty(),
        "RCRV-001: a live rule instructing a record to carry its own landing's hash is detected and named"
    );
}

/// RCRV-002 (t-106, RCR-002): on this repository, before the allowlist
/// the check refuses naming exactly the eleven rotted citations; with the
/// enumerated allowlist it passes; every allowlist row matches a real
/// record, and deleting a matched row's record fails that row as stale.
#[test]
fn the_check_refuses_exactly_the_rotted_citations_and_the_allowlist_matches() {
    let root = repo_root();
    let residual = root.join(".arca/residual");

    // The namespace: active folder and archive counted as one.
    let records = read_records(&residual).unwrap_or_else(|refusal| {
        panic!("RCRV-002: the record namespace must read as one namespace: {refusal}")
    });
    // At a clean close the active folder may be empty - the sweep moves
    // every satisfied record to the archive - so presence, not folder, is
    // the pinned fact here; the res-116 assertion below pins the archive
    // side of the same namespace.
    assert!(
        records.iter().any(|record| record.id == "res-158"),
        "RCRV-002: the namespace reads res-158 from whichever folder holds it"
    );
    assert!(
        records
            .iter()
            .any(|record| record.id == "res-116" && record.folder == RecordFolder::Archive),
        "RCRV-002: the archive feeds the same namespace"
    );

    // The expected rot, read by this test's own scanner: the eleven
    // records found at filing and the hash each cites today.
    let mut expected: Vec<(String, String)> = ROTTED_IDS
        .iter()
        .map(|id| {
            let text = fs::read_to_string(residual.join("archive").join(format!("{id}.md")))
                .unwrap_or_else(|error| panic!("read the rotted record {id}: {error}"));
            ((*id).to_owned(), cited_hash(&text))
        })
        .collect();
    expected.sort();

    // Before the allowlist: the refusal names exactly the rotted
    // citations, each naming its record and its hash.
    let refusal = demanded_err(
        check_records(&records, &root, None),
        "the check without the allowlist, over the rotted archive",
    );
    let mut named: Vec<(String, String)> = refusal
        .iter()
        .map(|one| (one.record.clone(), one.hash.clone()))
        .collect();
    named.sort();
    assert_eq!(
        named, expected,
        "RCRV-002: exactly the eleven rotted citations, no more and no fewer"
    );
    for one in &refusal {
        let rendered = one.to_string();
        assert!(
            rendered.contains(&one.record) && rendered.contains(&one.hash),
            "RCRV-002: the rendered refusal names the record and the hash: {rendered}"
        );
    }

    // With the enumerated allowlist: the check passes, and every row
    // matches a real record's rotted citation - one row per citation.
    let allowlist = read_allowlist(&root).unwrap_or_else(|refusal| {
        panic!("RCRV-002: the historical allowlist must load: {refusal}")
    });
    assert_eq!(
        allowlist.rows.len(),
        expected.len(),
        "RCRV-002: one row per rotted citation, no spare rows"
    );
    for row in &allowlist.rows {
        assert!(
            expected.contains(&(row.record.clone(), row.hash.clone())),
            "RCRV-002: the row matches a real record's rotted citation: {:?}",
            row
        );
    }
    if let Err(passed) = check_records(&records, &root, Some(&allowlist)) {
        panic!("RCRV-002: with the allowlist the check passes over this repository: {passed:?}");
    }

    // Deleting a matched row's record fails that row as stale - and only
    // that row: the namespace shrinks by one, the refusal names the
    // orphaned row by record and hash.
    let row = &allowlist.rows[0];
    let reduced: Vec<_> = records
        .iter()
        .filter(|record| record.id != row.record)
        .cloned()
        .collect();
    assert_eq!(
        reduced.len() + 1,
        records.len(),
        "exactly one record deleted"
    );
    let stale = demanded_err(
        check_records(&reduced, &root, Some(&allowlist)),
        "the check over a namespace missing an allowlisted record",
    );
    assert_eq!(stale.len(), 1, "RCRV-002: only the orphaned row fails");
    assert_eq!(
        (stale[0].record.clone(), stale[0].hash.clone()),
        (row.record.clone(), row.hash.clone()),
        "RCRV-002: the failure names the stale row's record and hash"
    );
    assert!(
        stale[0].reason.to_lowercase().contains("stale"),
        "RCRV-002: the failure is judged as staleness: {}",
        stale[0]
    );
}

/// RCRV-003 (t-106, RCR-002): reachability binds any ref, and a dangling
/// hash is a refusal. A record citing a commit a later amend orphans
/// refuses naming the record and the hash; the same record citing the
/// amended (reachable) commit passes; a hash that never existed refuses; a
/// commit held only by a tag passes.
#[test]
fn reachability_binds_any_ref_and_dangling_is_a_refusal() {
    // One fixture repository whose landing is amended after the record
    // cites it.
    let repo = TempRepo::new("t106-amend-rot");
    repo.write("work.md", "the green landing\n");
    repo.commit_all("t-900: green landing");
    let landing = repo.head();
    repo.write(
        ".arca/residual/res-900.md",
        &fixture_record(
            "res-900",
            &format!("git:{landing} (the green landing)"),
            "t-900",
        ),
    );

    // The amend orphans the cited commit: it still exists as an object -
    // `git cat-file` answers - but no ref reaches it anymore.
    repo.write("work.md", "the green landing, amended\n");
    repo.stage("work.md");
    let amend = repo.git(&["commit", "--amend", "-m", "t-900: green landing"]);
    assert!(amend.status.success(), "the fixture amend must succeed");
    let amended = repo.head();
    assert_ne!(landing, amended, "the amend really rewrote the landing");
    let exists = repo.git(&["cat-file", "-e", &format!("{landing}^{{commit}}")]);
    assert!(
        exists.status.success(),
        "RCRV-003 fixture: the orphaned commit still answers cat-file, so the refusal below is about reachability"
    );

    let records = read_records(&repo.root().join(".arca/residual"))
        .unwrap_or_else(|refusal| panic!("RCRV-003: the fixture namespace must read: {refusal}"));
    assert_eq!(records.len(), 1, "one record in the fixture");

    let refusal = demanded_err(
        check_records(&records, repo.root(), None),
        "the record citing the amend-orphaned commit",
    );
    assert_eq!(refusal.len(), 1, "one rotted citation in the fixture");
    assert_eq!(refusal[0].record, "res-900", "the refusal names the record");
    assert_eq!(refusal[0].hash, landing, "the refusal names the hash");
    let rendered = refusal[0].to_string();
    assert!(
        rendered.contains("res-900") && rendered.contains(&landing),
        "RCRV-003: the rendered refusal names the record and the hash: {rendered}"
    );

    // The same record citing the amended - reachable - commit passes.
    repo.write(
        ".arca/residual/res-900.md",
        &fixture_record(
            "res-900",
            &format!("git:{amended} (the green landing)"),
            "t-900",
        ),
    );
    let repointed = read_records(&repo.root().join(".arca/residual"))
        .unwrap_or_else(|refusal| panic!("RCRV-003: the repointed namespace must read: {refusal}"));
    if let Err(passed) = check_records(&repointed, repo.root(), None) {
        panic!("RCRV-003: the amended commit is reachable and passes: {passed:?}");
    }

    // A hash that never existed refuses - planted in the archive folder,
    // so the namespace walk must cross both folders to meet it.
    let never = "1".repeat(40);
    repo.write(
        ".arca/residual/archive/res-901.md",
        &fixture_record(
            "res-901",
            &format!("git:{never} (nothing ever held this)"),
            "t-900",
        ),
    );
    let with_never = read_records(&repo.root().join(".arca/residual")).unwrap_or_else(|refusal| {
        panic!("RCRV-003: the two-folder namespace must read: {refusal}")
    });
    assert_eq!(with_never.len(), 2, "active and archive are one namespace");
    let refusal = demanded_err(
        check_records(&with_never, repo.root(), None),
        "the record citing a hash that never existed",
    );
    assert_eq!(refusal.len(), 1, "only the never-existing citation refuses");
    assert_eq!(
        (refusal[0].record.clone(), refusal[0].hash.clone()),
        ("res-901".to_owned(), never),
        "the refusal names the record and the hash that never existed"
    );

    // A commit held only by a tag passes: reachability binds any ref, not
    // HEAD's line alone.
    let tagged = TempRepo::new("t106-tag-held");
    tagged.write("base.md", "the base landing\n");
    tagged.commit_all("base landing");
    tagged.git(&["branch", "side"]);
    let checkout = tagged.git(&["checkout", "side"]);
    assert!(
        checkout.status.success(),
        "the fixture branch checkout must succeed"
    );
    tagged.write("edition.md", "the tagged edition content\n");
    tagged.commit_all("side landing");
    let edition = tagged.head();
    tagged.git(&["tag", "edition-fixture"]);
    tagged.git(&["checkout", "main"]);
    tagged.git(&["branch", "-D", "side"]);
    let branches = tagged.git(&["for-each-ref", "--contains", &edition, "refs/heads"]);
    assert!(
        String::from_utf8_lossy(&branches.stdout).trim().is_empty(),
        "RCRV-003 fixture: no branch holds the edition commit"
    );
    let tags = tagged.git(&["for-each-ref", "--contains", &edition, "refs/tags"]);
    assert!(
        !String::from_utf8_lossy(&tags.stdout).trim().is_empty(),
        "RCRV-003 fixture: the tag holds the edition commit"
    );
    tagged.write(
        ".arca/residual/res-902.md",
        &fixture_record(
            "res-902",
            &format!("git:{edition} (the tagged edition)"),
            "t-900",
        ),
    );
    let tag_records =
        read_records(&tagged.root().join(".arca/residual")).unwrap_or_else(|refusal| {
            panic!("RCRV-003: the tag fixture namespace must read: {refusal}")
        });
    if let Err(passed) = check_records(&tag_records, tagged.root(), None) {
        panic!(
            "RCRV-003: a commit held only by a tag is reachable from a ref and passes: {passed:?}"
        );
    }
}

/// RCRV-004 (t-106, RCR-003): a fixture amend after stamping. One stamp
/// step re-derives the record's `implementation-revision` and the ticket's
/// `landed-commit` in the same change; the residual archive move refuses
/// while either cites an unresolvable commit outside the allowlist and
/// passes once re-pointed; and a stamp written before its landing commit
/// exists is refused, never predicted.
#[test]
fn a_repoint_is_one_stamp_step_and_the_archive_move_refuses_until_it_resolves() {
    let repo = TempRepo::new("t106-restamp");
    repo.write("work.md", "the green landing\n");
    repo.commit_all("t-901: green landing");
    let landing = repo.head();
    repo.write(
        ".arca/residual/res-903.md",
        &fixture_record(
            "res-903",
            &format!("git:{landing} (the green landing)"),
            "t-901",
        ),
    );
    repo.write(".arca/ticket/t-901.md", &fixture_ticket("t-901", &landing));

    // The landing is amended after stamping: both citations now name a
    // commit no ref reaches.
    repo.write("work.md", "the green landing, amended\n");
    repo.stage("work.md");
    let amend = repo.git(&["commit", "--amend", "-m", "t-901: green landing"]);
    assert!(amend.status.success(), "the fixture amend must succeed");
    let amended = repo.head();
    assert_ne!(landing, amended, "the amend really rewrote the landing");

    // The archive move refuses while either citation is unresolvable, and
    // moves nothing.
    let refusal = match archive_record(repo.root(), "res-903", None) {
        Err(refusal) => refusal,
        Ok(moved) => panic!(
            "RCRV-004: the archive move refuses while the citations name an unresolvable commit: moved {moved}"
        ),
    };
    assert_eq!(
        refusal.hash, landing,
        "RCRV-004: the refusal names the orphaned hash: {refusal}"
    );
    assert!(
        refusal.record == "res-903" || refusal.record == "t-901",
        "RCRV-004: the refusal names the record or the ticket: {}",
        refusal
    );
    assert!(
        repo.root().join(".arca/residual/res-903.md").is_file()
            && !repo
                .root()
                .join(".arca/residual/archive/res-903.md")
                .exists(),
        "RCRV-004: the refusing move moved nothing"
    );

    // Either: the record re-pointed by hand, the ticket's landed-commit
    // still rotten - the move still refuses, now naming the ticket.
    repo.write(
        ".arca/residual/res-903.md",
        &fixture_record(
            "res-903",
            &format!("git:{amended} (the hand repoint)"),
            "t-901",
        ),
    );
    let either = match archive_record(repo.root(), "res-903", None) {
        Err(refusal) => refusal,
        Ok(moved) => panic!(
            "RCRV-004: the ticket's rotten landed-commit alone still refuses the move: moved {moved}"
        ),
    };
    assert_eq!(
        (either.record.clone(), either.hash.clone()),
        ("t-901".to_owned(), landing.clone()),
        "RCRV-004: the refusal names the ticket and its rotten hash: {either}"
    );

    // Back to the stamped-then-orphaned state for the stamp-step scenario.
    repo.write(
        ".arca/residual/res-903.md",
        &fixture_record(
            "res-903",
            &format!("git:{landing} (the green landing)"),
            "t-901",
        ),
    );

    // One stamp step re-derives both citations in the same change, from
    // the repository: the landed tip it resolves to is the amended one.
    let outcome = stamp_step(repo.root(), "res-903").unwrap_or_else(|refusal| {
        panic!("RCRV-004: one stamp step re-points the record and its ticket: {refusal}")
    });
    assert_eq!(outcome.record, "res-903", "the outcome names the record");
    assert_eq!(
        outcome.ticket, "t-901",
        "the outcome names the owning ticket"
    );
    assert_eq!(
        outcome.landed_commit, amended,
        "the hash is the landed tip resolved now"
    );
    assert_eq!(
        outcome.record_revision,
        format!("git:{amended}"),
        "the record's citation is re-derived"
    );

    let record_text = fs::read_to_string(repo.root().join(".arca/residual/res-903.md"))
        .expect("the stamped record is still on disk");
    assert_eq!(
        field_value(&record_text, "implementation-revision"),
        Some(format!("git:{amended}")),
        "RCRV-004: the record on disk carries the re-derived citation"
    );
    let ticket_text = fs::read_to_string(repo.root().join(".arca/ticket/t-901.md"))
        .expect("the stamped ticket is still on disk");
    assert_eq!(
        field_value(&ticket_text, "landed-commit"),
        Some(amended.clone()),
        "RCRV-004: the ticket on disk carries the re-derived landed-commit"
    );

    // The archive move passes once re-pointed, and really moves the
    // record to the archive folder.
    let moved = archive_record(repo.root(), "res-903", None).unwrap_or_else(|refusal| {
        panic!("RCRV-004: the archive move passes once both citations resolve: {refusal}")
    });
    assert!(
        moved.ends_with("res-903.md") && moved.contains("archive"),
        "RCRV-004: the move answers the archive path: {moved}"
    );
    assert!(
        repo.root()
            .join(".arca/residual/archive/res-903.md")
            .is_file()
            && !repo.root().join(".arca/residual/res-903.md").exists(),
        "RCRV-004: the record physically moved active-to-archive"
    );

    // The recording order (RCR-001): a stamp written before its landing
    // commit exists is refused - the hash is derived from the repository,
    // never predicted from memory.
    let unborn = TempRepo::new("t106-unborn");
    unborn.write(
        ".arca/residual/res-904.md",
        &fixture_record("res-904", "", "t-902"),
    );
    unborn.write(".arca/ticket/t-902.md", &fixture_ticket("t-902", ""));
    match stamp_step(unborn.root(), "res-904") {
        Err(refusal) => assert!(
            !refusal.reason.is_empty(),
            "RCRV-004: the pre-landing refusal says why: {refusal}"
        ),
        Ok(stamped) => panic!(
            "RCRV-004: a stamp before its landing commit exists is refused, never predicted: {stamped:?}"
        ),
    }
}
