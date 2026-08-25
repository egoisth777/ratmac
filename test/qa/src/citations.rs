//! The resolving-citation checker (t-106 / RCR-001..RCR-003) - red stub.
//!
//! P4 landed the spec: `tests/t106_resolving_citations.rs` is the authority
//! on this API's shape and behavior. The checker itself is P5 work; until
//! it lands, every function below answers with one explicit refusal for
//! every input. It parses nothing, walks nothing, and never panics, so the
//! ticket's four tests run red for the designed reason - no citation
//! checker exists yet - rather than failing to compile.
//!
//! The contract the tests hold (P5 implements against it):
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
//! Hole-poke notes (P4):
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

use std::fmt;
use std::path::Path;

/// Where the enumerated historical allowlist lives, repository-relative
/// (ADR-0019: tracked data, never generated).
pub const ALLOWLIST_REL: &str = ".arca/residual/allowlist.md";

/// One of the two folders that make the record namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
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
pub fn stamp_landing_order(_schema_text: &str) -> Result<StampLandingOrder, RuleRefusal> {
    Err(RuleRefusal {
        reason: "unimplemented: the working rules carry no stamp-landing-order reader yet"
            .to_owned(),
    })
}

/// Scans every live rule text for a rule instructing a record to carry the
/// hash of a commit it lands inside (RCR-001): the answer is the list of
/// offending rule ids, empty when no live rule self-cites.
pub fn self_citing_rules(_schema_text: &str) -> Result<Vec<String>, RuleRefusal> {
    Err(RuleRefusal {
        reason: "unimplemented: no live-rule self-citation scan exists yet".to_owned(),
    })
}

/// Reads the record namespace under `residual_root` - `res-*.md` in the
/// active folder and in `archive/`, one namespace - answering each
/// record's `implementation-revision` (RCR-002).
pub fn read_records(_residual_root: &Path) -> Result<Vec<CitedRecord>, RecordsRefusal> {
    Err(RecordsRefusal {
        path: String::new(),
        reason: "unimplemented: the record namespace has no reader yet".to_owned(),
    })
}

/// Reads the enumerated historical allowlist from the repository
/// (RCR-002): each row names its record, its unresolvable hash, and the
/// reason.
pub fn read_allowlist(_repo_root: &Path) -> Result<Allowlist, AllowlistRefusal> {
    Err(AllowlistRefusal {
        reason: "unimplemented: the historical allowlist has no reader yet".to_owned(),
    })
}

/// Resolves every record's `git:` citation against the repository's refs
/// (RCR-002): reachability binds any ref, a dangling-only hash is exactly
/// the refusal, and each refusal names the record and the hash. The
/// allowlist, when given, admits exactly the rot its rows name; a row
/// matching no record fails as stale.
pub fn check_records(
    _records: &[CitedRecord],
    _repo: &Path,
    _allowlist: Option<&Allowlist>,
) -> Result<(), Vec<CitationRefusal>> {
    Err(vec![CitationRefusal {
        record: String::new(),
        hash: String::new(),
        reason: "unimplemented: no citation checker exists yet".to_owned(),
    }])
}

/// The one stamp step (RCR-003): resolves the landed tip from the
/// repository at that moment and rewrites the record's
/// `implementation-revision` and the owning ticket's `landed-commit` in
/// the same change. Refuses when no landed tip exists to resolve - the
/// hash is derived, never predicted.
pub fn stamp_step(_repo_root: &Path, _record_id: &str) -> Result<StampOutcome, StampRefusal> {
    Err(StampRefusal {
        reason: "unimplemented: no stamp step exists yet".to_owned(),
    })
}

/// Moves one record from the active folder to the archive (RCR-003),
/// answering the archive path it moved to. Refuses - moving nothing -
/// while the record or its owning ticket cites a commit that does not
/// resolve and is not covered by the allowlist.
pub fn archive_record(
    _repo_root: &Path,
    _record_id: &str,
    _allowlist: Option<&Allowlist>,
) -> Result<String, CitationRefusal> {
    Err(CitationRefusal {
        record: String::new(),
        hash: String::new(),
        reason: "unimplemented: the archive move carries no citation check yet".to_owned(),
    })
}
