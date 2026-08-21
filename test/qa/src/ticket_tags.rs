//! The ticket tag reader (t-105 / CGD-001, CGD-002): a workflow-side checker
//! that learns a ticket's checks from its three front-matter tag lists and
//! never from its prose.
//!
//! Everything below the closing `---` fence is invisible to the reader, so
//! prose sections, decoy tag-shaped blocks, and heading renames cannot move
//! its answer. The three lists are read verbatim, order preserved, each
//! entry an opaque check id. The parsing is hand-rolled line scanning in the
//! house style (`src/receipt.rs`, `src/contract.rs`): the format owns three
//! fields of `  - "id"` lines, and no YAML dependency is carried for it.
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

/// The three tag fields, in the field order the duplicate rule walks.
const FIELDS: [&str; 3] = ["focused-tests", "hidden-lanes", "quality-commands"];

/// Reads a ticket's declared checks from its three front-matter tag lists -
/// never from its prose (CGD-001). A malformed list refuses naming the
/// field and the offending entry (CGD-002).
pub fn declared_checks(source: &str) -> Result<DeclaredTagSet, TagRefusal> {
    let mut lines = source.lines();

    // The front matter opens with a `---` line on line one; a file without
    // one cannot declare tags, and the reader never hunts for a fence deeper
    // in the file.
    if !matches!(lines.next(), Some(first) if first.trim_end() == "---") {
        return Err(not_cut());
    }

    // One slot per field: `None` until the field is declared.
    let mut lists: [Option<Vec<String>>; 3] = [None, None, None];
    // Every id collected so far, for the cross-field duplicate rule.
    let mut seen: Vec<&str> = Vec::new();
    // The tag field whose list is currently open.
    let mut open: Option<usize> = None;
    let mut closed = false;

    for line in lines {
        if line.trim_end() == "---" {
            closed = true;
            break; // the closing fence: not one line past it is read
        }
        let text = line.trim_end();
        if !text.starts_with(' ') {
            // Top level: a tag field opens its list (or refuses a scalar);
            // any other line - another key, a comment, a blank - closes the
            // open one.
            match tag_field(text) {
                Some((index, None)) => {
                    lists[index].get_or_insert_with(Vec::new);
                    open = Some(index);
                }
                Some((index, Some(scalar))) => {
                    let entry = quoted(scalar).unwrap_or(scalar);
                    return Err(refusal(
                        FIELDS[index],
                        entry,
                        "a scalar where a list belongs",
                    ));
                }
                None => open = None,
            }
            continue;
        }

        // Indented: the list body. An entry of a non-tag field is not the
        // reader's business; a comment or blank line is not an entry.
        let Some(index) = open else { continue };
        let item = text.trim();
        if item.is_empty() || item.starts_with('#') {
            continue;
        }
        let content = if let Some(content) = item.strip_prefix("- ") {
            content.trim()
        } else if item == "-" {
            ""
        } else {
            // Inside a tag list but not a list item: not the declared shape.
            return Err(refusal(
                FIELDS[index],
                item,
                "an entry that is not a quoted string",
            ));
        };
        let id = match quoted(content) {
            Some("") => {
                return Err(refusal(FIELDS[index], "", "an empty entry"));
            }
            Some(id) => id,
            None if content.is_empty() => {
                return Err(refusal(FIELDS[index], "", "an empty entry"));
            }
            None => {
                return Err(refusal(
                    FIELDS[index],
                    content,
                    "an entry that is not a quoted string",
                ));
            }
        };
        if seen.contains(&id) {
            return Err(refusal(
                FIELDS[index],
                id,
                "a duplicate id across the tag fields",
            ));
        }
        seen.push(id);
        lists[index]
            .as_mut()
            .expect("an open list is a declared field")
            .push(id.to_owned());
    }

    if !closed {
        // The front matter never closes: refuse naming the tag field whose
        // list was still being read when the source ended, and declare
        // nothing - never a half-parse.
        let field = open.map(|index| FIELDS[index]).unwrap_or("");
        return Err(refusal(
            field,
            "",
            "truncated front matter: the closing --- fence never comes",
        ));
    }
    if lists.iter().all(|list| list.is_none()) {
        return Err(not_cut());
    }
    for (index, list) in lists.iter().enumerate() {
        if list.is_none() {
            return Err(refusal(FIELDS[index], "", "missing"));
        }
    }

    let [focused, hidden, quality] = lists;
    Ok(DeclaredTagSet {
        focused_tests: focused.unwrap_or_default(),
        hidden_lanes: hidden.unwrap_or_default(),
        quality_commands: quality.unwrap_or_default(),
    })
}

/// A top-level tag-field line: `(index, None)` for `field:` opening its
/// list, `(index, Some(text))` for `field: text` where the list belongs.
fn tag_field(text: &str) -> Option<(usize, Option<&str>)> {
    for (index, field) in FIELDS.iter().enumerate() {
        let Some(rest) = text.strip_prefix(field) else {
            continue;
        };
        let Some(rest) = rest.strip_prefix(':') else {
            continue;
        };
        let scalar = rest.trim();
        return Some((
            index,
            if scalar.is_empty() {
                None
            } else {
                Some(scalar)
            },
        ));
    }
    None
}

/// The body of a `"..."`-quoted string, or `None` when not that shape.
fn quoted(content: &str) -> Option<&str> {
    content.strip_prefix('"')?.strip_suffix('"')
}

/// One refusal, in the reader's stable words.
fn refusal(field: &str, entry: &str, reason: &str) -> TagRefusal {
    TagRefusal {
        field: field.to_owned(),
        entry: entry.to_owned(),
        reason: reason.to_owned(),
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
