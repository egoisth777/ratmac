//! t-105 / CGD-001, CGD-002: a checker learns a ticket's checks from its
//! tags, never its prose.
//!
//! CGDV-001 `the_checker_reads_tags_and_ignores_prose`
//! CGDV-002 `a_malformed_tag_list_refuses_naming_field_and_entry`
//!
//! The reader under test is `ratmac_qa::ticket_tags::declared_checks`: it
//! takes a ticket's full text and answers either the declared check set -
//! the three front-matter tag lists `focused-tests`, `hidden-lanes`,
//! `quality-commands`, verbatim, order preserved - or a refusal naming the
//! field and the offending entry. `Err` carries no set by type, so a
//! malformed list never yields a partial declaration.
//!
//! The growing fixture is this repository's own tagged sprint (`GPH-003`):
//! the four tickets are read from the worktree at run time, never embedded,
//! so the check keeps reporting as history grows. Expected sets come from
//! the test's own front-matter scan, independent of the reader under test.
//!
//! Hole-poke notes:
//! - Would CGDV-001 pass a reader that regex-greps the whole file, prose
//!   included? No. Every ticket must reproduce its front-matter set three
//!   more times: as a prose-stripped twin (front matter only - a reader
//!   that needs anything below the closing fence answers differently
//!   there), with decoy tag-shaped field blocks appended as prose (a
//!   collector that scans the whole file picks up the decoy ids), and with
//!   `## Merge Gate` renamed (a reader anchored on that heading's prose
//!   changes its answer).
//! - Would CGDV-002 pass a reader that drops malformed entries silently?
//!   No. All three malformed fixtures must answer `Err` naming the field
//!   and the offending entry; a silent drop answers `Ok`, which fails
//!   before the naming is even examined.
//! - Would either test pass a reader that always answers empty lists? No.
//!   Each ticket's expected lists are asserted non-empty first, and the
//!   malformed fixtures must refuse rather than declare anything.

use std::fs;

use ratmac_qa::ticket_tags::declared_checks;

/// This sprint's tagged tickets, resolved from the worktree at run time.
const TAGGED_TICKETS: [&str; 4] = ["t-102", "t-103", "t-104", "t-105"];

/// The three tag lists of one ticket, as the test's own scan reads them.
#[derive(Default)]
struct TagLists {
    focused_tests: Vec<String>,
    hidden_lanes: Vec<String>,
    quality_commands: Vec<String>,
}

/// Which tag field an entry belongs to while scanning the front matter.
#[derive(Clone, Copy)]
enum TagField {
    Focused,
    Hidden,
    Quality,
}

/// The tag field a front-matter line declares, by its `field:` shape.
fn tag_field(line: &str) -> Option<TagField> {
    match line.trim_end() {
        "focused-tests:" => Some(TagField::Focused),
        "hidden-lanes:" => Some(TagField::Hidden),
        "quality-commands:" => Some(TagField::Quality),
        _ => None,
    }
}

/// The line index of the `---` fence closing the front matter; the opening
/// fence on line one is skipped, and `---` inside a longer line (an anchor
/// slug, for instance) never matches.
fn closing_fence(source: &str) -> usize {
    source
        .lines()
        .enumerate()
        .skip(1)
        .find(|(_, line)| line.trim_end() == "---")
        .unwrap_or_else(|| panic!("a tagged ticket closes its front matter"))
        .0
}

/// The test's own oracle: the three tag lists read from the front matter
/// alone by plain line scanning - independent of the reader under test.
/// Only the three declared-check fields are collected, so a leak from any
/// other front-matter list (planned-test-refs, dependencies) shows up as a
/// set difference, not as a coincidence.
fn front_matter_lists(source: &str) -> TagLists {
    let mut lists = TagLists::default();
    let mut field = None;
    for line in source.lines().take(closing_fence(source)) {
        if let Some(next) = tag_field(line) {
            field = Some(next);
            continue;
        }
        let Some(current) = field else { continue };
        if !line.starts_with(' ') {
            field = None; // the list ends at the first unindented line
            continue;
        }
        let Some(id) = quoted_id(line) else {
            continue;
        };
        match current {
            TagField::Focused => lists.focused_tests.push(id),
            TagField::Hidden => lists.hidden_lanes.push(id),
            TagField::Quality => lists.quality_commands.push(id),
        }
    }
    lists
}

/// The entry of one `  - "id"` list line, verbatim between its quotes.
fn quoted_id(line: &str) -> Option<String> {
    let entry = line.trim().strip_prefix("- ")?;
    Some(entry.strip_prefix('"')?.strip_suffix('"')?.to_owned())
}

/// The prose-stripped twin: the same bytes with every markdown section
/// below the closing `---` removed - front matter through its fence, and
/// not one byte of prose after it.
fn prose_stripped_twin(source: &str) -> String {
    let mut twin = source
        .lines()
        .take(closing_fence(source) + 1)
        .collect::<Vec<_>>()
        .join("\n");
    twin.push('\n');
    twin
}

/// A minimal tagged ticket carrying the three given field blocks; the
/// prose below the fence is an ordinary Merge Gate body.
fn tagged_ticket(focused: &str, hidden: &str, quality: &str) -> String {
    format!(
        "---\n\
         ticket-id: \"t-900\"\n\
         {focused}\n\
         {hidden}\n\
         {quality}\n\
         status: \"approved\"\n\
         ---\n\n\
         # Ticket: t-900\n\n\
         ## Merge Gate\n\n\
         - Quality: `cargo test --workspace` passes.\n"
    )
}

/// One malformed fixture's oracle: the answer is `Err` - no declared set
/// exists to inspect - and the refusal names the field and the offending
/// entry, both as data and in its rendered form.
fn refuses_naming(source: &str, field: &str, entry: &str, what: &str) {
    let refusal = declared_checks(source)
        .err()
        .unwrap_or_else(|| panic!("CGDV-002: {what} refuses instead of declaring"));
    assert_eq!(
        refusal.field, field,
        "CGDV-002: {what}: the refusal names the field"
    );
    assert_eq!(
        refusal.entry, entry,
        "CGDV-002: {what}: the refusal names the offending entry"
    );
    let rendered = refusal.to_string();
    assert!(
        rendered.contains(field) && rendered.contains(&format!("\"{entry}\"")),
        "CGDV-002: {what}: the rendered refusal names the field and the quoted entry: {rendered}"
    );
}

/// CGDV-001 (t-105, CGD-001): over this repository's own tagged tickets,
/// the declared set equals the three front-matter lists verbatim, order
/// preserved; a prose-stripped twin yields the identical set; decoy
/// tag-shaped prose and a Merge Gate heading rename change nothing.
#[test]
fn the_checker_reads_tags_and_ignores_prose() {
    let root = ratmac_qa::grown::repo_root();
    for id in TAGGED_TICKETS {
        let source = fs::read_to_string(root.join(".arca/ticket").join(format!("{id}.md")))
            .unwrap_or_else(|error| panic!("read this sprint's tagged ticket {id}: {error}"));

        let expected = front_matter_lists(&source);
        assert!(
            !expected.focused_tests.is_empty()
                && !expected.hidden_lanes.is_empty()
                && !expected.quality_commands.is_empty(),
            "{id}: the expected lists are non-empty, so an always-empty reader cannot pass"
        );

        // The full ticket: exactly the tags, verbatim and in order.
        let declared = declared_checks(&source).unwrap_or_else(|refusal| {
            panic!("{id}: the checker must declare {id}'s checks from its tags: {refusal}")
        });
        assert_eq!(
            declared.focused_tests, expected.focused_tests,
            "{id}: focused-tests verbatim, order preserved"
        );
        assert_eq!(
            declared.hidden_lanes, expected.hidden_lanes,
            "{id}: hidden-lanes verbatim, order preserved"
        );
        assert_eq!(
            declared.quality_commands, expected.quality_commands,
            "{id}: quality-commands verbatim, order preserved"
        );

        // The prose-stripped twin: same tags, no prose, identical set.
        let twin = prose_stripped_twin(&source);
        assert!(
            twin.len() < source.len(),
            "{id}: the twin really drops the prose"
        );
        let stripped = declared_checks(&twin).unwrap_or_else(|refusal| {
            panic!("{id}: the twin still declares {id}'s checks from its tags: {refusal}")
        });
        assert_eq!(
            stripped, declared,
            "{id}: prose below the fence never changes the declared set"
        );

        // Decoy prose: tag-shaped field blocks below the fence never leak.
        let decoy = format!(
            "{source}\n\
             ## Decoy prose, never a declaration\n\n\
             focused-tests:\n  - \"DECOY-{id}-focused\"\n\
             hidden-lanes:\n  - \"DECOY-{id}-hidden\"\n\
             quality-commands:\n  - \"DECOY-{id}-command\"\n"
        );
        let clean = declared_checks(&decoy).unwrap_or_else(|refusal| {
            panic!("{id}: decoy prose must not refuse the real tags: {refusal}")
        });
        assert_eq!(
            clean, declared,
            "{id}: tag-shaped prose below the fence never leaks into the set"
        );

        // Heading rename: the prose's structure is not load-bearing.
        let renamed = source.replace("## Merge Gate", "## A Heading With Another Name");
        assert!(
            !renamed.contains("## Merge Gate"),
            "{id}: the rename took effect"
        );
        let after = declared_checks(&renamed).unwrap_or_else(|refusal| {
            panic!("{id}: a renamed heading must not refuse the tags: {refusal}")
        });
        assert_eq!(
            after, declared,
            "{id}: renaming `## Merge Gate` changes nothing"
        );
    }
}

/// CGDV-002 (t-105, CGD-002): a scalar where a list belongs, an
/// empty-string entry, and a duplicate id across two fields each refuse,
/// naming the field and the offending entry, with no declared set.
#[test]
fn a_malformed_tag_list_refuses_naming_field_and_entry() {
    // A scalar where the focused-tests list belongs: the refusal names the
    // field and the scalar that sat where a list belongs.
    refuses_naming(
        &tagged_ticket(
            "focused-tests: \"x\"",
            "hidden-lanes:\n  - \"HT-900-01\"",
            "quality-commands:\n  - \"cargo test --workspace\"",
        ),
        "focused-tests",
        "x",
        "a scalar where a list belongs",
    );

    // An empty-string entry inside the hidden-lanes list: the offending
    // entry is the empty string, named in the refusal's own words.
    refuses_naming(
        &tagged_ticket(
            "focused-tests:\n  - \"test/qa/tests/t900_example.rs\"",
            "hidden-lanes:\n  - \"\"",
            "quality-commands:\n  - \"cargo test --workspace\"",
        ),
        "hidden-lanes",
        "",
        "an empty-string entry",
    );

    // The same id in two fields: the refusal names the field carrying the
    // repeated occurrence - hidden-lanes, the second declaration in field
    // order - and the duplicated id.
    refuses_naming(
        &tagged_ticket(
            "focused-tests:\n  - \"dup-entry\"",
            "hidden-lanes:\n  - \"dup-entry\"",
            "quality-commands:\n  - \"cargo test --workspace\"",
        ),
        "hidden-lanes",
        "dup-entry",
        "a duplicate id across two fields",
    );
}
