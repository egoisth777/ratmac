//! The resolving-citation checker (t-106 / RCR-001..RCR-003).
//!
//! P4 landed the spec: `tests/t106_resolving_citations.rs` is the authority
//! on this API's shape and behavior, and this module is its P5 twin.
//!
//! - `read_records` walks the record namespace - `res-*.md` under the
//!   residual root and under its `archive/` folder, two folders counted as
//!   one namespace - answering each record's `residual-id` and its
//!   `implementation-revision` value verbatim. A missing `archive/`
//!   folder is an empty archive, never a refusal; a file that is not a
//!   record (`manifest`, the allowlist) is never one.
//! - `check_records` resolves each `git:` citation against the
//!   repository's refs and refuses naming the record and the hash.
//!   Reachability binds any ref - an ancestor of any branch tip or held
//!   by a tag - never `HEAD` alone and never a reflog: a hash that
//!   answers `git cat-file` only as a dangling object is exactly this
//!   refusal, not a pass. An allowlist row admits only the rot it names;
//!   a row that matches no record fails as stale.
//! - `stamp_step` is the one mechanical re-point: it resolves the landed
//!   tip from the repository at that moment and rewrites the record's
//!   `implementation-revision` and the owning ticket's `landed-commit` in
//!   the same change - never from memory or prediction, and never before
//!   a landed tip exists to resolve.
//! - `archive_record` moves one record active-to-archive, refusing while
//!   the record or its owning ticket cites a commit that does not resolve
//!   and is not covered by the allowlist.
//!
//! The citation shapes are the input surface, and each is judged by its
//! own named rule - never a silent drop. The refusal words are stable so
//! a caller can tell which rule fired:
//!
//! - `the record carries no implementation-revision citation` - the field
//!   is absent or empty.
//! - `the revision cites a non-git scheme, which resolves against no ref`
//!   - a value outside the `git:` citation convention.
//! - `the git: citation names no hash` - `git:` with no hex run after it.
//! - `the cited hash is too short for git to resolve` - fewer hex digits
//!   than git's shortest abbreviation.
//! - `the cited hash resolves to no commit in this repository` - no
//!   object answers the citation; a hash that never existed.
//! - `the cited short hash is ambiguous in this repository` - the
//!   abbreviation names more than one object.
//! - `the cited commit is unreachable from every ref - a dangling object
//!   is not a pass` - the object answers `cat-file` but no ref reaches
//!   it: an amend-orphaned or otherwise rewritten-away commit.
//! - `the allowlist row is stale: no record in the namespace cites this
//!   rot` - a row whose record or rot is gone.
//!
//! Hole-poke notes (P4, held by the tests):
//! - Would the check pass an implementation that stops at the first
//!   refusal? No. RCRV-002 demands the full set - exactly the eleven
//!   rotted citations, no more, no fewer.
//! - Would the check pass an implementation that reads reflogs as refs?
//!   No. The amend-orphaned commit sits in `HEAD`'s reflog and answers
//!   `cat-file`, and RCRV-003 asserts both facts while demanding the
//!   refusal anyway.
//! - Would the check pass `HEAD`-ancestry alone as reachability? No. The
//!   tag-held fixture commit is on no branch and no ancestor of `HEAD`,
//!   and must pass.
//! - Would the allowlist pass an implementation that admits anything?
//!   No. Without it the check must refuse, with it the check must pass,
//!   and a row whose record is gone must fail as stale - a wildcard
//!   admits the rot but cannot name the missing row.
//! - Would the stamp step pass an implementation that re-points only the
//!   record? No. RCRV-004 refuses the archive move while either citation
//!   is rotten, then requires both fields rewritten on disk to the
//!   amended tip one call resolved.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where the enumerated historical allowlist lives, repository-relative
/// (ADR-0019: tracked data, never generated).
pub const ALLOWLIST_REL: &str = ".arca/residual/allowlist.tsv";

/// The residual root, repository-relative: the active folder.
const RESIDUAL_REL: &str = ".arca/residual";

/// The archive folder under the residual root.
const ARCHIVE_REL: &str = "archive";

/// The ticket roots, repository-relative: the active folder and archive.
const TICKET_DIRS: [&str; 2] = [".arca/ticket", ".arca/ticket/archive"];

/// One of the two folders that make the record namespace.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecordFolder {
    /// `.arca/residual/` - the active folder.
    Active,
    /// `.arca/residual/archive/` - the byte-frozen archive.
    Archive,
}

/// One gap record as the checker walks it: its residual id, the folder it
/// was read from, and its raw `implementation-revision` value verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitedRecord {
    /// The record's residual id, e.g. `res-116`.
    pub id: String,
    /// The folder the record was read from - one namespace, two folders.
    pub folder: RecordFolder,
    /// The `implementation-revision` value, e.g. `git:0231def (the t-083
    /// green tree; ...)`. The `git:` citation is parsed from it.
    pub revision: String,
}

/// One citation refusal (RCR-002): the citing id - a record like
/// `res-116`, a ticket like `t-901`, or an allowlist row's record - the
/// hash it cites, and why. The refusal names the record and the hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationRefusal {
    /// The id of the record, ticket, or allowlist row refusing.
    pub record: String,
    /// The cited hash, verbatim.
    pub hash: String,
    /// Why, in plain words.
    pub reason: String,
}

impl fmt::Display for CitationRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "record {} cites {}: {}",
            self.record, self.hash, self.reason
        )
    }
}

/// Why the record namespace itself could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordsRefusal {
    /// The path that could not be read.
    pub path: String,
    /// Why, in plain words.
    pub reason: String,
}

impl fmt::Display for RecordsRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "records at {}: {}", self.path, self.reason)
    }
}

/// One enumerated historical allowlist row (RCR-002): the rotted record,
/// its unresolvable hash, and the reason the row exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowlistRow {
    /// The rotted record's residual id, e.g. `res-116`.
    pub record: String,
    /// The unresolvable hash the record cites.
    pub hash: String,
    /// Why the row exists, in plain words.
    pub reason: String,
}

/// The enumerated historical allowlist (RCR-002): byte-frozen records
/// judged under the pre-rule order ride these rows, never an edit.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Allowlist {
    /// The rows, as tracked.
    pub rows: Vec<AllowlistRow>,
}

/// Why the allowlist could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowlistRefusal {
    /// Why, in plain words.
    pub reason: String,
}

impl fmt::Display for AllowlistRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "allowlist: {}", self.reason)
    }
}

/// The stamp-landing order as the working rules state it (RCR-001): which
/// rule states it, and the two clauses that make the order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StampLandingOrder {
    /// The rule that states the order, by its heading id or section name.
    pub rule: String,
    /// The clause ordering the landings: the green landing merges to
    /// `main` first; the stamp landing follows it.
    pub order_clause: String,
    /// The clause deriving the hash: the commit the landed tip resolves
    /// to at stamp time - never from memory or prediction.
    pub derivation_clause: String,
}

/// Why a working-rules reading refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleRefusal {
    /// Why, in plain words.
    pub reason: String,
}

impl fmt::Display for RuleRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "working rules: {}", self.reason)
    }
}

/// What one stamp step re-derived (RCR-003): the record, its owning
/// ticket, and the one landed tip both citations now name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StampOutcome {
    /// The stamped record's residual id.
    pub record: String,
    /// The owning ticket's id.
    pub ticket: String,
    /// The full hash the landed tip resolved to at stamp time.
    pub landed_commit: String,
    /// The record's new `implementation-revision` value, `git:` plus that
    /// hash.
    pub record_revision: String,
}

/// Why a stamp step refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StampRefusal {
    /// Why, in plain words.
    pub reason: String,
}

impl fmt::Display for StampRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "stamp: {}", self.reason)
    }
}

/// Reads the stamp-landing order from the working rules (RCR-001): the
/// rules must state that the stamp landing follows the merged green
/// landing and that the hash is derived by resolving the landed tip.
///
/// The reader walks the rule sections (heading plus body) and takes the
/// first section whose body names both landings - `stamp landing` and
/// `green landing` - reading its order clause from the body's opening and
/// its derivation clause from the first dash-separated clause that names
/// resolving. A rule text without such a section refuses rather than
/// agrees.
pub fn stamp_landing_order(schema_text: &str) -> Result<StampLandingOrder, RuleRefusal> {
    for (rule, body) in sections(schema_text) {
        let lowered = body.to_lowercase();
        if !lowered.contains("stamp landing") || !lowered.contains("green landing") {
            continue;
        }
        let clauses: Vec<&str> = body.split(" - ").collect();
        let order_clause = clauses[0].trim().to_owned();
        let derivation_clause = clauses
            .iter()
            .map(|clause| clause.trim())
            .find(|clause| clause.to_lowercase().contains("resolv"))
            .unwrap_or_else(|| clauses.last().expect("a split yields a clause").trim())
            .to_owned();
        return Ok(StampLandingOrder {
            rule,
            order_clause,
            derivation_clause,
        });
    }
    Err(RuleRefusal {
        reason: "no rule states the stamp-landing order - a stamp landing following the \
                 merged green landing, its hash derived by resolving the landed tip"
            .to_owned(),
    })
}

/// Why a cited hash did not resolve to one commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unresolved {
    /// No commit answers the citation: a hash that never existed.
    NoSuchCommit,
    /// The abbreviation names more than one object.
    Ambiguous,
}

/// Resolves one cited hash to its full commit id: `git rev-parse --verify`
/// against the commit-peeled object, refusing a hash that names no commit
/// or an ambiguous abbreviation.
fn resolve_commit(repo: &Path, hash: &str) -> Result<String, Unresolved> {
    let peeled = format!("{hash}^{{commit}}");
    let output = git(repo, &["rev-parse", "--verify", &peeled]);
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if output.status.success() && !stdout.is_empty() {
        return Ok(stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    if stderr.contains("ambiguous") {
        Err(Unresolved::Ambiguous)
    } else {
        Err(Unresolved::NoSuchCommit)
    }
}

/// The stated shapes of a self-citing rule (RCR-001): each phrase is an
/// instruction to write the citation inside the landing it names, which no
/// commit can carry - a commit cannot contain its own hash.
const SELF_CITE_INSTRUCTIONS: [&str; 5] = [
    "in the landing it cites",
    "that landing's own hash",
    "its own landing's hash",
    "the hash of the commit it lands inside",
    "in the same commit it cites",
];

/// Scans every live rule text for a rule instructing a record to carry the
/// hash of a commit it lands inside (RCR-001): the answer is the list of
/// offending rule ids, empty when no live rule self-cites.
pub fn self_citing_rules(schema_text: &str) -> Result<Vec<String>, RuleRefusal> {
    let mut offenders = Vec::new();
    for (rule, body) in sections(schema_text) {
        if !body.contains("implementation-revision") {
            continue;
        }
        let lowered = body.to_lowercase();
        if SELF_CITE_INSTRUCTIONS
            .iter()
            .any(|instruction| lowered.contains(instruction))
        {
            offenders.push(rule);
        }
    }
    Ok(offenders)
}

/// Reads the record namespace under `residual_root` - `res-*.md` in the
/// active folder and in `archive/`, one namespace - answering each
/// record's `implementation-revision` (RCR-002).
pub fn read_records(residual_root: &Path) -> Result<Vec<CitedRecord>, RecordsRefusal> {
    let mut records = Vec::new();
    read_folder(residual_root, RecordFolder::Active, &mut records)?;
    read_folder(
        &residual_root.join(ARCHIVE_REL),
        RecordFolder::Archive,
        &mut records,
    )?;
    records.sort_by(|a, b| (&a.id, &a.folder).cmp(&(&b.id, &b.folder)));
    Ok(records)
}

/// Reads the `res-*.md` files of one folder into `records`; a missing
/// folder contributes nothing, and anything that is not a record-shaped
/// file (`manifest`, the allowlist) is never one.
fn read_folder(
    folder: &Path,
    record_folder: RecordFolder,
    records: &mut Vec<CitedRecord>,
) -> Result<(), RecordsRefusal> {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        // A missing archive folder is an empty archive, never a refusal;
        // a missing active folder is a namespace that is not there.
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                && record_folder == RecordFolder::Archive =>
        {
            return Ok(());
        }
        Err(error) => {
            return Err(RecordsRefusal {
                path: folder.display().to_string(),
                reason: format!("the folder cannot be read: {error}"),
            });
        }
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("res-") || !name.ends_with(".md") {
            continue;
        }
        let path = entry.path();
        let text = fs::read_to_string(&path).map_err(|error| RecordsRefusal {
            path: path.display().to_string(),
            reason: format!("the record cannot be read: {error}"),
        })?;
        let id = quoted_field(&text, "residual-id").ok_or_else(|| RecordsRefusal {
            path: path.display().to_string(),
            reason: "the record carries no residual-id field".to_owned(),
        })?;
        // The citation field is judged by `check_records`, so an absent or
        // empty value reads as the empty string - the named refusal is the
        // checker's, never the reader's.
        let revision = quoted_field(&text, "implementation-revision").unwrap_or_default();
        records.push(CitedRecord {
            id,
            folder: record_folder.clone(),
            revision,
        });
    }
    Ok(())
}

/// Reads the enumerated historical allowlist from the repository
/// (RCR-002): each row names its record, its unresolvable hash, and the
/// reason.
pub fn read_allowlist(repo_root: &Path) -> Result<Allowlist, AllowlistRefusal> {
    let path = repo_root.join(ALLOWLIST_REL);
    let text = fs::read_to_string(&path).map_err(|error| AllowlistRefusal {
        reason: format!("{} cannot be read: {error}", path.display()),
    })?;
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        // The house allowlist shape (the fixtures under `test/qa/fixtures/`):
        // `#` lines carry the header and the standing prose; every other
        // non-empty line is one tab-separated row - record, hash, reason.
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let cells: Vec<&str> = line.split('\t').map(str::trim).collect();
        if cells.len() != 3
            || !cells[0].starts_with("res-")
            || cells[1].is_empty()
            || cells[2].is_empty()
        {
            return Err(AllowlistRefusal {
                reason: format!(
                    "line {} is not a row naming a record, a hash, and a reason: {line}",
                    index + 1
                ),
            });
        }
        rows.push(AllowlistRow {
            record: cells[0].to_owned(),
            hash: cells[1].to_owned(),
            reason: cells[2].to_owned(),
        });
    }
    if rows.is_empty() {
        return Err(AllowlistRefusal {
            reason: format!(
                "{} carries no rows: the enumerated allowlist names each rotted record, its \
                 unresolvable hash, and the reason",
                path.display()
            ),
        });
    }
    Ok(Allowlist { rows })
}

/// Resolves every record's `git:` citation against the repository's refs
/// (RCR-002): reachability binds any ref, a dangling-only hash is exactly
/// the refusal, and each refusal names the record and the hash. The
/// allowlist, when given, admits exactly the rot its rows name; a row
/// matching no record fails as stale.
pub fn check_records(
    records: &[CitedRecord],
    repo: &Path,
    allowlist: Option<&Allowlist>,
) -> Result<(), Vec<CitationRefusal>> {
    let mut refusals = Vec::new();
    let mut admitted: Vec<bool> = allowlist
        .map(|list| vec![false; list.rows.len()])
        .unwrap_or_default();
    let reachable = reachable_commits(repo);
    let mut resolved: HashMap<String, Result<String, Unresolved>> = HashMap::new();
    for record in records {
        if let Some((hash, reason)) =
            judge_citation(&record.revision, repo, &reachable, &mut resolved)
        {
            let row = allowlist.and_then(|list| {
                list.rows
                    .iter()
                    .zip(admitted.iter_mut())
                    .position(|(row, _)| row.record == record.id && row.hash == hash)
            });
            match row {
                Some(position) => admitted[position] = true,
                None => refusals.push(CitationRefusal {
                    record: record.id.clone(),
                    hash,
                    reason,
                }),
            }
        }
    }
    if let Some(list) = allowlist {
        for (row, consumed) in list.rows.iter().zip(admitted.iter()) {
            if !consumed {
                refusals.push(CitationRefusal {
                    record: row.record.clone(),
                    hash: row.hash.clone(),
                    reason: "the allowlist row is stale: no record in the namespace cites this rot"
                        .to_owned(),
                });
            }
        }
    }
    if refusals.is_empty() {
        Ok(())
    } else {
        Err(refusals)
    }
}

/// The one stamp step (RCR-003): resolves the landed tip from the
/// repository at that moment and rewrites the record's
/// `implementation-revision` and the owning ticket's `landed-commit` in
/// the same change. Refuses when no landed tip exists to resolve - the
/// hash is derived, never predicted.
pub fn stamp_step(repo_root: &Path, record_id: &str) -> Result<StampOutcome, StampRefusal> {
    let (record_path, record_text) = find_record(repo_root, record_id)?;
    let ticket = owning_ticket(&record_text).ok_or_else(|| StampRefusal {
        reason: format!("record {record_id} names no owning ticket"),
    })?;
    let head = git(repo_root, &["rev-parse", "HEAD"]);
    let landed = if head.status.success() {
        String::from_utf8_lossy(&head.stdout).trim().to_owned()
    } else {
        String::new()
    };
    if landed.is_empty() {
        return Err(StampRefusal {
            reason: format!(
                "no landed commit exists for record {record_id} to cite: HEAD resolves to no \
                 commit, and the hash is derived from the repository, never predicted"
            ),
        });
    }
    let revision = format!("git:{landed}");
    let stamped_record = set_quoted_field(&record_text, "implementation-revision", &revision)
        .ok_or_else(|| StampRefusal {
            reason: format!("record {record_id} carries no implementation-revision field to stamp"),
        })?;
    let ticket_path = find_ticket(repo_root, &ticket).ok_or_else(|| StampRefusal {
        reason: format!("the owning ticket {ticket} does not exist"),
    })?;
    let ticket_text = fs::read_to_string(&ticket_path).map_err(|error| StampRefusal {
        reason: format!("{} cannot be read: {error}", ticket_path.display()),
    })?;
    let stamped_ticket =
        set_quoted_field(&ticket_text, "landed-commit", &landed).ok_or_else(|| StampRefusal {
            reason: format!("ticket {ticket} carries no landed-commit field to stamp"),
        })?;
    fs::write(&record_path, stamped_record).map_err(|error| StampRefusal {
        reason: format!("{} cannot be written: {error}", record_path.display()),
    })?;
    fs::write(&ticket_path, stamped_ticket).map_err(|error| StampRefusal {
        reason: format!("{} cannot be written: {error}", ticket_path.display()),
    })?;
    Ok(StampOutcome {
        record: record_id.to_owned(),
        ticket,
        landed_commit: landed,
        record_revision: revision,
    })
}

/// Moves one record from the active folder to the archive (RCR-003),
/// answering the archive path it moved to. Refuses - moving nothing -
/// while the record or its owning ticket cites a commit that does not
/// resolve and is not covered by the allowlist.
pub fn archive_record(
    repo_root: &Path,
    record_id: &str,
    allowlist: Option<&Allowlist>,
) -> Result<String, CitationRefusal> {
    let residual = repo_root.join(RESIDUAL_REL);
    let active = residual.join(format!("{record_id}.md"));
    let archived = residual.join(ARCHIVE_REL).join(format!("{record_id}.md"));
    if !active.is_file() {
        let reason = if archived.is_file() {
            "the record already lives in the archive folder"
        } else {
            "no such record in the active folder"
        };
        return Err(CitationRefusal {
            record: record_id.to_owned(),
            hash: String::new(),
            reason: reason.to_owned(),
        });
    }
    let record_text = fs::read_to_string(&active).map_err(|error| CitationRefusal {
        record: record_id.to_owned(),
        hash: String::new(),
        reason: format!("{} cannot be read: {error}", active.display()),
    })?;
    let reachable = reachable_commits(repo_root);
    let mut resolved: HashMap<String, Result<String, Unresolved>> = HashMap::new();
    let revision = quoted_field(&record_text, "implementation-revision").unwrap_or_default();
    if let Some((hash, reason)) = judge_citation(&revision, repo_root, &reachable, &mut resolved) {
        if !admitted(allowlist, record_id, &hash) {
            return Err(CitationRefusal {
                record: record_id.to_owned(),
                hash,
                reason,
            });
        }
    }
    // The owning ticket's landed-commit is the second citation the move
    // must see resolve: a pending re-stamp obligation travels with the
    // record, so the move refuses while either name is rotten.
    if let Some(ticket) = owning_ticket(&record_text) {
        let ticket_path = find_ticket(repo_root, &ticket).ok_or_else(|| CitationRefusal {
            record: ticket.clone(),
            hash: String::new(),
            reason: format!("the owning ticket {ticket} does not exist"),
        })?;
        let ticket_text = fs::read_to_string(&ticket_path).map_err(|error| CitationRefusal {
            record: ticket.clone(),
            hash: String::new(),
            reason: format!("{} cannot be read: {error}", ticket_path.display()),
        })?;
        let landed = quoted_field(&ticket_text, "landed-commit").unwrap_or_default();
        if let Some((hash, reason)) = judge_hash(&landed, repo_root, &reachable, &mut resolved) {
            if !admitted(allowlist, &ticket, &hash) {
                return Err(CitationRefusal {
                    record: ticket,
                    hash,
                    reason,
                });
            }
        }
    }
    fs::create_dir_all(residual.join(ARCHIVE_REL)).map_err(|error| CitationRefusal {
        record: record_id.to_owned(),
        hash: String::new(),
        reason: format!("the archive folder cannot be created: {error}"),
    })?;
    // The authorized move keeps the record's bytes, with relative links
    // gaining one `../` level for their new depth.
    let moved = deepen_relative_links(&record_text);
    fs::write(&archived, moved).map_err(|error| CitationRefusal {
        record: record_id.to_owned(),
        hash: String::new(),
        reason: format!("{} cannot be written: {error}", archived.display()),
    })?;
    fs::remove_file(&active).map_err(|error| CitationRefusal {
        record: record_id.to_owned(),
        hash: String::new(),
        reason: format!("{} cannot be removed: {error}", active.display()),
    })?;
    Ok(archived.display().to_string())
}

/// Whether an allowlist row admits exactly this rot: the citing id and
/// the cited hash, verbatim both.
fn admitted(allowlist: Option<&Allowlist>, citing: &str, hash: &str) -> bool {
    allowlist.is_some_and(|list| {
        list.rows
            .iter()
            .any(|row| row.record == citing && row.hash == hash)
    })
}

/// Judges one `implementation-revision` value against the repository:
/// `None` is a pass, and `Some((hash, reason))` is the refusal - the hash
/// verbatim as cited (empty when the value names none), and the named
/// rule that fired.
fn judge_citation(
    revision: &str,
    repo: &Path,
    reachable: &HashSet<String>,
    resolved: &mut HashMap<String, Result<String, Unresolved>>,
) -> Option<(String, String)> {
    let value = revision.trim();
    if value.is_empty() {
        return Some((
            String::new(),
            "the record carries no implementation-revision citation".to_owned(),
        ));
    }
    let Some(cited) = value.strip_prefix("git:") else {
        let scheme = value.split(':').next().unwrap_or(value);
        return Some((
            String::new(),
            format!("the revision cites a non-git scheme, which resolves against no ref: {scheme}"),
        ));
    };
    let hash: String = cited.chars().take_while(char::is_ascii_hexdigit).collect();
    if hash.is_empty() {
        return Some((String::new(), "the git: citation names no hash".to_owned()));
    }
    judge_hash(&hash, repo, reachable, resolved)
}

/// Judges one bare hash - the shape a ticket's `landed-commit` carries -
/// against the repository: `None` is a pass, and `Some((hash, reason))`
/// is the refusal, the hash verbatim and the named rule that fired.
fn judge_hash(
    hash: &str,
    repo: &Path,
    reachable: &HashSet<String>,
    resolved: &mut HashMap<String, Result<String, Unresolved>>,
) -> Option<(String, String)> {
    if hash.is_empty() {
        return Some((String::new(), "the citation names no commit".to_owned()));
    }
    if hash.len() < 4 {
        return Some((
            hash.to_owned(),
            "the cited hash is too short for git to resolve".to_owned(),
        ));
    }
    let full = resolved
        .entry(hash.to_owned())
        .or_insert_with(|| resolve_commit(repo, hash))
        .clone();
    let full = match full {
        Ok(full) => full,
        Err(Unresolved::NoSuchCommit) => {
            return Some((
                hash.to_owned(),
                "the cited hash resolves to no commit in this repository".to_owned(),
            ));
        }
        Err(Unresolved::Ambiguous) => {
            return Some((
                hash.to_owned(),
                "the cited short hash is ambiguous in this repository".to_owned(),
            ));
        }
    };
    if reachable.contains(&full) {
        return None;
    }
    Some((
        hash.to_owned(),
        "the cited commit is unreachable from every ref - a dangling object is not a pass"
            .to_owned(),
    ))
}

/// Every commit id reachable from any ref - branches, tags, and other
/// refs alike (ADR-0019): never a reflog, never `HEAD`'s line alone.
fn reachable_commits(repo: &Path) -> HashSet<String> {
    let output = git(repo, &["rev-list", "--all"]);
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Runs one git command in `repo`, panicking only if git cannot spawn.
fn git(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap_or_else(|error| panic!("git {args:?} must run: {error}"))
}

/// The record file for `record_id`, active or archived, with its bytes.
fn find_record(repo_root: &Path, record_id: &str) -> Result<(PathBuf, String), StampRefusal> {
    for folder in [
        repo_root.join(RESIDUAL_REL),
        repo_root.join(RESIDUAL_REL).join(ARCHIVE_REL),
    ] {
        let path = folder.join(format!("{record_id}.md"));
        if path.is_file() {
            let text = fs::read_to_string(&path).map_err(|error| StampRefusal {
                reason: format!("{} cannot be read: {error}", path.display()),
            })?;
            return Ok((path, text));
        }
    }
    Err(StampRefusal {
        reason: format!("no record {record_id} exists in the residual namespace"),
    })
}

/// The ticket file for `ticket_id`, active or archived.
fn find_ticket(repo_root: &Path, ticket_id: &str) -> Option<PathBuf> {
    TICKET_DIRS
        .iter()
        .map(|dir| repo_root.join(dir).join(format!("{ticket_id}.md")))
        .find(|path| path.is_file())
}

/// The owning ticket a record names in its footer: the id inside
/// ``Owned by ticket `t-106`. ``.
fn owning_ticket(record_text: &str) -> Option<String> {
    let marker = "Owned by ticket `";
    let start = record_text.rfind(marker)? + marker.len();
    let end = record_text[start..].find('`')? + start;
    Some(record_text[start..end].to_owned())
}

/// The first `field: "value"` line's value, verbatim between its quotes -
/// the same reading every citation writer below round-trips.
fn quoted_field(text: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}:");
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with(&prefix))?;
    let value = line.trim().strip_prefix(&prefix)?.trim();
    Some(value.strip_prefix('"')?.strip_suffix('"')?.to_owned())
}

/// Replaces the first `field: "value"` line's value with `new_value`,
/// keeping every other byte of the text - indentation, annotation-free
/// quotes, and line endings included. `None` when the field is absent.
fn set_quoted_field(text: &str, field: &str, new_value: &str) -> Option<String> {
    let prefix = format!("{field}:");
    let mut rebuilt = String::with_capacity(text.len());
    let mut replaced = false;
    for line in text.split_inclusive('\n') {
        if !replaced && line.trim_start().starts_with(&prefix) {
            let indent = line.len() - line.trim_start().len();
            let rest = &line[indent..];
            let open = rest.find('"')?;
            let close = rest.rfind('"')?;
            if close <= open {
                return None;
            }
            rebuilt.push_str(&line[..indent]);
            rebuilt.push_str(&rest[..open]);
            rebuilt.push('"');
            rebuilt.push_str(new_value);
            rebuilt.push('"');
            rebuilt.push_str(&rest[close + 1..]);
            replaced = true;
        } else {
            rebuilt.push_str(line);
        }
    }
    replaced.then_some(rebuilt)
}

/// Relative links gain one `../` level for the archive's depth: a
/// markdown link target that already climbs (`../goal/`) climbs once
/// more, one that does not (`index.md`) climbs for the first time, and
/// anchors, absolute paths, and urls are left alone.
fn deepen_relative_links(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find("](") {
        out.push_str(&rest[..open + 2]);
        rest = &rest[open + 2..];
        let Some(close) = rest.find(')') else {
            out.push_str(rest);
            return out;
        };
        let target = &rest[..close];
        let deepened =
            if target.starts_with('#') || target.starts_with('/') || target.contains("://") {
                target.to_owned()
            } else {
                format!("../{target}")
            };
        out.push_str(&deepened);
        out.push(')');
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

/// The rule sections of a rule text: each heading (a `#` line) with the
/// body that follows it, in order.
fn sections(text: &str) -> Vec<(String, String)> {
    let is_heading = |line: &str| {
        let trimmed = line.trim_start();
        match trimmed.strip_prefix('#') {
            Some(rest) => rest.starts_with('#') || rest.starts_with(' '),
            None => false,
        }
    };
    let mut out: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        if is_heading(line) {
            out.push((
                line.trim().trim_start_matches('#').trim().to_owned(),
                String::new(),
            ));
        } else if let Some((_, body)) = out.last_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    out
}
