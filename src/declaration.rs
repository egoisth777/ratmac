//! CGD-003 (ADR-0022): the generic selected-string-list declaration reader.
//!
//! One reader owns the narrow `front-matter-string-lists` subset the runbook
//! specification's Completion declaration mapping documents. The workflow's
//! QA checker supplies its local field names; the Engine supplies the names
//! its runbook mapping parsed. Neither carries a second parser, and this
//! module holds no project field-name constant: every field it reads is
//! caller-selected, every entry opaque.
//!
//! The data region opens only when line one is `---` and ends at the first
//! later `---` line. Only an exact selected name at column one, immediately
//! followed by `:`, opens a list: double-quoted block entries, or an
//! explicit `[]` for an empty list. Unselected fields and their entries,
//! and every byte after the closing fence, are invisible to this reader. A
//! source declaring none of the selected fields carries no declaration at
//! all - `None`, not a refusal.
//!
//! A malformed declaration refuses naming the field and the offending
//! entry, never silently dropping one, and the refusal words are stable so
//! a caller can tell which rule fired:
//!
//! - `a scalar where a list belongs` - a selected field line carries inline
//!   text other than the explicit empty list `[]`.
//! - `an empty entry` - a list item that is the empty string.
//! - `an entry that is not a quoted string` - a list item outside the
//!   format's `"id"` shape: a bare word, a number, or a bare dash.
//! - `a duplicate id across the tag fields` - the id is already declared
//!   within one list or across the selected lists; the field named is the
//!   one carrying the repeat. The wording is the extracted workflow
//!   reader's, kept verbatim so its landed semantics do not move.
//! - `an entry with no open list` - entry-shaped data attached to a
//!   selected field whose list is complete: past an explicit `[]`, or past
//!   the column-one line that ended the block. Never a silent drop.
//! - `an entry indented with a tab, not ASCII spaces` - the grammar's
//!   entries begin with ASCII spaces; tab-indented entry-shaped data
//!   attached to a selected field refuses instead of vanishing.
//! - `missing` - some selected fields are declared but not this one.
//! - `truncated front matter: the closing --- fence never comes` - the
//!   source ends inside the front matter; nothing is declared from it.
//!
//! `[]` is this format's own extension over the extracted workflow reader,
//! which accepted only empty block lists; it is documented here first, not
//! inherited provenance (ADR-0022).

use std::collections::HashSet;
use std::fmt;

/// The documented format of a completion declaration (ADR-0022): one
/// spelling, one production owner. A runbook's `declaration-format` value
/// parses through [`DeclarationFormat::parse`] and renders through
/// [`DeclarationFormat::as_str`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclarationFormat {
    /// The `front-matter-string-lists` subset: selected column-one fields
    /// inside the first `---` region, double-quoted block entries or an
    /// explicit `[]`.
    FrontMatterStringLists,
}

impl DeclarationFormat {
    /// The format's authored spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FrontMatterStringLists => "front-matter-string-lists",
        }
    }

    /// The format an authored `declaration-format` value names, or `None`
    /// when that value supports no declaration reading.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "front-matter-string-lists" => Some(Self::FrontMatterStringLists),
            _ => None,
        }
    }
}

/// One `completion_gate`'s typed declaration mapping (ADR-0022): the
/// documented format and the three front-matter field names its receipt
/// kinds select - focused, hidden-lane, then quality.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionDeclaration {
    pub format: DeclarationFormat,
    pub focused_field: String,
    pub hidden_lane_field: String,
    pub quality_field: String,
}

impl CompletionDeclaration {
    /// The authored mapping keys, in the order the duplicate rule walks.
    const MAPPING_KEYS: [&str; 3] = ["focused-field", "hidden-lane-field", "quality-field"];

    /// Rejects a mapping whose field names are blank or reused for two
    /// receipt kinds (`RB113`): the refusal names the authored mapping key
    /// and the offending value, so the runbook reader and every direct
    /// completion caller fail closed on the same defect.
    pub fn validate(&self) -> Result<(), StringListDefect> {
        let names = [
            (Self::MAPPING_KEYS[0], &self.focused_field),
            (Self::MAPPING_KEYS[1], &self.hidden_lane_field),
            (Self::MAPPING_KEYS[2], &self.quality_field),
        ];
        let mut seen: HashSet<&str> = HashSet::new();
        for (key, name) in names {
            if name.trim().is_empty() {
                return Err(defect(key, name, "an empty mapped field name"));
            }
            if !seen.insert(name) {
                return Err(defect(
                    key,
                    name,
                    "a field name mapped to more than one receipt kind",
                ));
            }
        }
        Ok(())
    }
}

/// A malformed selected-list declaration's refusal: the field, the
/// offending entry, and why - never a silent drop, never a partial set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StringListDefect {
    /// The selected field the refusal names; the authored mapping key when
    /// the defect is about the mapping itself.
    pub field: String,
    /// The offending entry, verbatim; the empty string where no entry
    /// exists.
    pub entry: String,
    /// Why, in plain words.
    pub reason: String,
}

impl fmt::Display for StringListDefect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "field {} entry {:?}: {}",
            self.field, self.entry, self.reason
        )
    }
}

/// What the indented lines of the data region are currently attached to.
#[derive(Clone, Copy)]
enum Territory {
    /// A selected field whose block accepts entries.
    Open(usize),
    /// A selected field whose list is complete - an explicit `[]`, or a
    /// column-one line ended its block. Entry-shaped data still belongs to
    /// it and refuses rather than silently dropping.
    Sealed(usize),
    /// The region's start, or an unselected column-one key's territory:
    /// every indented or tab-indented line here is invisible, malformed or
    /// not. Unselected data never participates.
    Foreign,
}

/// Reads the selected top-level string-list fields from `source`'s front
/// matter - the first `---` / closing `---` region only (CGD-003).
///
/// Returns `Ok(None)` when no selected field is declared. Otherwise,
/// `Ok(Some(lists))` contains one complete list per name in `fields`, in
/// that order. Malformed, truncated, duplicated, or partially declared
/// selections return a [`StringListDefect`]. The names are opaque: this
/// reader owns no project vocabulary.
pub fn read_selected_string_lists(
    source: &str,
    fields: &[&str],
) -> Result<Option<Vec<Vec<String>>>, StringListDefect> {
    let mut lines = source.lines();

    // The data region opens only when line one is `---`; a file without
    // that fence declares none of the selected fields, and the reader
    // never hunts for a fence deeper in the file.
    if !matches!(lines.next(), Some(first) if first.trim_end() == "---") {
        return Ok(None);
    }

    // One slot per field: `None` until the field is declared.
    let mut lists: Vec<Option<Vec<String>>> = vec![None; fields.len()];
    // Every entry collected so far, for the within- and across-list
    // duplicate rule.
    let mut seen: HashSet<&str> = HashSet::new();
    // What indented lines currently belong to.
    let mut territory = Territory::Foreign;
    let mut closed = false;

    for line in lines {
        if line.trim_end() == "---" {
            closed = true;
            break; // the closing fence: not one line past it is read
        }
        let text = line.trim_end();
        if text.is_empty() {
            // A blank line is neither an entry nor a block-ending field:
            // the grammar ignores it, so no declared entry is lost.
            continue;
        }
        if text.starts_with('\t') {
            // The grammar's entries begin with ASCII spaces, so a
            // tab-indented line is neither an entry nor a column-one
            // field, and it never ends an open block. Entry-shaped data
            // attached to a selected field refuses instead of silently
            // vanishing; comments and unselected territory stay invisible.
            let item = text.trim();
            if !item.starts_with('#') && is_entry_shaped(item) {
                if let Territory::Open(index) | Territory::Sealed(index) = territory {
                    return Err(defect(
                        fields[index],
                        entry_text(item),
                        "an entry indented with a tab, not ASCII spaces",
                    ));
                }
            }
            continue;
        }
        if !text.starts_with(' ') {
            // Top level. A selected field opens its list (or refuses a
            // scalar); a real unselected key takes over the territory; any
            // other column-one line - a comment, prose - ends the open
            // block while the selected field keeps its attachment.
            match selected_field(text, fields) {
                Some((index, None)) => {
                    lists[index].get_or_insert_with(Vec::new);
                    territory = Territory::Open(index);
                }
                Some((index, Some("[]"))) => {
                    // `[]` is the only inline list form: an explicit empty
                    // list, complete in itself - no block opens for it.
                    lists[index].get_or_insert_with(Vec::new);
                    territory = Territory::Sealed(index);
                }
                Some((index, Some(scalar))) => {
                    let entry = quoted(scalar).unwrap_or(scalar);
                    return Err(defect(
                        fields[index],
                        entry,
                        "a scalar where a list belongs",
                    ));
                }
                None => {
                    territory = if is_key(text) {
                        Territory::Foreign
                    } else if let Territory::Open(index) | Territory::Sealed(index) = territory {
                        Territory::Sealed(index)
                    } else {
                        Territory::Foreign
                    };
                }
            }
            continue;
        }

        // Indented with ASCII spaces. A comment is not an entry, and
        // unselected territory is invisible - malformed or not.
        let item = text.trim();
        if item.starts_with('#') {
            continue;
        }
        let index = match territory {
            Territory::Foreign => continue,
            Territory::Sealed(index) => {
                // The selected field's list is complete: visibly declared
                // entry data refuses, never silently drops.
                if is_entry_shaped(item) {
                    return Err(defect(
                        fields[index],
                        entry_text(item),
                        "an entry with no open list",
                    ));
                }
                continue;
            }
            Territory::Open(index) => index,
        };

        let content = if let Some(content) = item.strip_prefix("- ") {
            content.trim()
        } else if item == "-" {
            ""
        } else {
            // Inside a selected list but not a list item: not the declared
            // shape.
            return Err(defect(
                fields[index],
                item,
                "an entry that is not a quoted string",
            ));
        };
        let id = match quoted(content) {
            Some("") => return Err(defect(fields[index], "", "an empty entry")),
            Some(id) => id,
            None if content.is_empty() => {
                return Err(defect(fields[index], "", "an empty entry"));
            }
            None => {
                return Err(defect(
                    fields[index],
                    content,
                    "an entry that is not a quoted string",
                ));
            }
        };
        if !seen.insert(id) {
            return Err(defect(
                fields[index],
                id,
                "a duplicate id across the tag fields",
            ));
        }
        lists[index]
            .as_mut()
            .expect("an open list is a declared field")
            .push(id.to_owned());
    }

    if !closed {
        // The front matter never closes: refuse naming the selected field
        // whose list was still being read when the source ended, and
        // declare nothing - never a half-parse.
        let field = if let Territory::Open(index) = territory {
            fields[index]
        } else {
            ""
        };
        return Err(defect(
            field,
            "",
            "truncated front matter: the closing --- fence never comes",
        ));
    }
    if lists.iter().all(|list| list.is_none()) {
        return Ok(None);
    }
    for (index, list) in lists.iter().enumerate() {
        if list.is_none() {
            return Err(defect(fields[index], "", "missing"));
        }
    }

    Ok(Some(
        lists
            .into_iter()
            .map(|list| list.unwrap_or_default())
            .collect(),
    ))
}

/// A top-level selected-field line: `(index, None)` for `field:` opening
/// its list, `(index, Some(text))` for `field: text` where the list
/// belongs. Only an exact name immediately followed by `:` selects.
fn selected_field<'a>(text: &'a str, fields: &[&str]) -> Option<(usize, Option<&'a str>)> {
    for (index, field) in fields.iter().enumerate() {
        let Some(rest) = text.strip_prefix(*field) else {
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

/// A column-one key line - `name:` or `name: value` with a bare name - as
/// opposed to a comment or stray prose. Only a real unselected key takes
/// the territory away from a selected field; a comment merely ends the
/// open block.
fn is_key(text: &str) -> bool {
    let name = text.split(':').next().unwrap_or("");
    !name.is_empty() && !name.contains(' ') && !text.starts_with('#')
}

/// A list item's shape: `- entry` or a bare `-`.
fn is_entry_shaped(item: &str) -> bool {
    item == "-" || item.starts_with("- ")
}

/// The offending entry an entry-shaped item carries, for the refusals
/// about data no open list accepts: the `- ` prefix dropped, outer quotes
/// removed when the shape is the quoted form.
fn entry_text(item: &str) -> &str {
    let content = item.strip_prefix("- ").map(str::trim).unwrap_or("");
    quoted(content).unwrap_or(content)
}

/// One refusal, in the reader's stable words.
fn defect(field: &str, entry: &str, reason: &str) -> StringListDefect {
    StringListDefect {
        field: field.to_owned(),
        entry: entry.to_owned(),
        reason: reason.to_owned(),
    }
}
