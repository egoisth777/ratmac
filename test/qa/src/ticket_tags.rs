//! The ticket tag reader (t-105 / CGD-001, CGD-002) - red stub.
//!
//! P4 landed the spec: `tests/t105_ticket_check_tags.rs` is the authority on
//! the API's shape and behavior. The reader itself is P5 work; until it
//! lands, `declared_checks` answers with one explicit refusal for every
//! input. It parses nothing and never panics, so the ticket's two tests run
//! red for the designed reason - the checker does not read tags yet - rather
//! than failing to compile.
//!
//! Hole-poke notes (P4):
//! - Would CGDV-001 pass a reader that regex-greps the whole file, prose
//!   included? No. Each sprint ticket's expected set is derived from its
//!   front matter alone, and the identical set is then required from a
//!   prose-stripped twin (front matter only - a reader that needs anything
//!   below the closing fence answers differently there), from the same
//!   ticket with decoy tag-shaped field blocks appended as prose (a reader
//!   that collects field blocks from anywhere in the file picks up the
//!   decoy ids), and from the same ticket with `## Merge Gate` renamed (a
//!   reader that anchors on that heading's prose loses or changes its
//!   answer).
//! - Would CGDV-002 pass a reader that drops malformed entries silently?
//!   No. Every malformed fixture must answer `Err` naming the field and the
//!   offending entry - a silent drop answers `Ok`, which the test rejects
//!   before it even looks at the names.

use std::fmt;

/// The checks a ticket declares as its three front-matter tag lists
/// (CGD-001): `focused-tests`, `hidden-lanes`, `quality-commands`, each
/// entry an opaque check id taken verbatim, order preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredTagSet {
    pub focused_tests: Vec<String>,
    pub hidden_lanes: Vec<String>,
    pub quality_commands: Vec<String>,
}

/// A malformed tag list's refusal (CGD-002): the field, the offending
/// entry, and why - never a silent drop, never a partial set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRefusal {
    /// The front-matter field the refusal names.
    pub field: String,
    /// The offending entry, verbatim; the empty string for an empty entry.
    pub entry: String,
    /// Why, in plain words.
    pub reason: String,
}

impl fmt::Display for TagRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "field {} entry {:?}: {}",
            self.field, self.entry, self.reason
        )
    }
}

/// Reads a ticket's declared checks from its three front-matter tag lists -
/// never from its prose (CGD-001). A malformed list refuses naming the
/// field and the offending entry (CGD-002).
pub fn declared_checks(_source: &str) -> Result<DeclaredTagSet, TagRefusal> {
    Err(TagRefusal {
        field: String::new(),
        entry: String::new(),
        reason: "unimplemented: the checker does not read tags yet".to_owned(),
    })
}
