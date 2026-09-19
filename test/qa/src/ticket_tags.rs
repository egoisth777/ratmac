//! The ticket tag reader (t-105 / CGD-001, CGD-002): a workflow-side checker
//! that learns a ticket's checks from its three front-matter tag lists and
//! never from its prose.
//!
//! Since the completion-declaration cutover (i-032 / CGD-003) the parsing is
//! production's: `ratmac::declaration::read_selected_string_lists` is the one
//! parser, and this module is a thin adapter that keeps the QA surface -
//! `DeclaredTagSet` answered for the shop's three fixed fields - without
//! carrying a second grammar. Field names, entry shape, and refusal words
//! are the shared reader's, so a ticket reads identically from either side.
//!
//! Everything below the closing `---` fence is invisible to the reader, so
//! prose sections, decoy tag-shaped blocks, and heading renames cannot move
//! its answer. The three lists are read verbatim, order preserved, each
//! entry an opaque check id.
//!
//! A malformed list refuses naming the field and the offending entry, never
//! silently dropping one, and the refusal words are stable so a caller can
//! tell which rule fired:
//!
//! - `a scalar where a list belongs` - the field line carries inline text.
//! - `an empty entry` - a list item that is the empty string.
//! - `an entry that is not a quoted string` - a list item outside the
//!   format's `"id"` shape: a bare word, a number, or a bare dash.
//! - `a duplicate id across the tag fields` - the id is already declared
//!   among the three lists; the field named is the one carrying the repeat.
//! - `missing` - the front matter declares the other tag fields but not
//!   this one. CGD-002 words only the present-but-malformed case; the tag
//!   format's blank carries all three fields, so a gap is a shape break
//!   refused by the format, naming the absent field with entry `""`.
//! - `truncated front matter: the closing --- fence never comes` - the
//!   source ends inside the front matter; nothing is declared from it.
//!
//! A ticket that declares none of the three fields is answered, not refused:
//! `not cut to the tag format`, naming no field. Tickets cut before the tag
//! integration are judged by the rules they were cut under (CGD-001), and
//! that standing is stated distinctly from every refusal above.

use ratmac::declaration::read_selected_string_lists;

/// A malformed tag list's refusal (CGD-002): the field, the offending
/// entry, and why - never a silent drop, never a partial set. The shared
/// reader's defect is that refusal; it is re-exported under the QA name so
/// the two surfaces cannot drift apart.
pub type TagRefusal = ratmac::declaration::StringListDefect;

/// The checks a ticket declares as its three front-matter tag lists
/// (CGD-001): `focused-tests`, `hidden-lanes`, `quality-commands`, each
/// entry an opaque check id taken verbatim, order preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredTagSet {
    pub focused_tests: Vec<String>,
    pub hidden_lanes: Vec<String>,
    pub quality_commands: Vec<String>,
}

/// The three tag fields, in the field order the duplicate rule walks.
const FIELDS: [&str; 3] = ["focused-tests", "hidden-lanes", "quality-commands"];

/// Reads a ticket's declared checks from its three front-matter tag lists -
/// never from its prose (CGD-001). A malformed list refuses naming the
/// field and the offending entry (CGD-002). A ticket cut before the tag
/// integration - no tag field declared at all - is answered with
/// [`not_cut`]'s standing, which is this adapter's alone: the shared reader
/// says only that no declaration exists.
pub fn declared_checks(source: &str) -> Result<DeclaredTagSet, TagRefusal> {
    match read_selected_string_lists(source, &FIELDS) {
        Err(refusal) => Err(refusal),
        Ok(None) => Err(not_cut()),
        Ok(Some(lists)) => {
            let mut lists = lists.into_iter();
            Ok(DeclaredTagSet {
                focused_tests: lists.next().unwrap_or_default(),
                hidden_lanes: lists.next().unwrap_or_default(),
                quality_commands: lists.next().unwrap_or_default(),
            })
        }
    }
}

/// The standing of a ticket cut before the tag integration: not refused -
/// it is judged by the rules it was cut under (CGD-001) - and stated as
/// such, naming no field and no entry.
fn not_cut() -> TagRefusal {
    TagRefusal {
        field: String::new(),
        entry: String::new(),
        reason: "not cut to the tag format: no tag field is declared".to_owned(),
    }
}
