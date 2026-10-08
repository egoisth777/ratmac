//! WEB-006 (t-134): the nested-invocation boundary's shared machinery.
//!
//! This module owns the three mechanical layers the parent guard and the
//! child endpoint share: the strict `ratmac-receipt-v1` envelope codec, the
//! parent-owned nested ledger (append/annotate-only, mechanically identical
//! to the spawn ledger), and the dedicated receipt pipe. It defines no
//! policy; the ticket's boundary steps live with their callers.

use crate::root::Displayed;
use std::fs;
use std::io::Write;
use std::path::Path;

/// The receipt envelope's magic prefix, followed by one NUL byte.
const ENVELOPE_MAGIC: &[u8] = b"ratmac-receipt-v1";

/// The number of length-framed UTF-8 fields one envelope carries.
const ENVELOPE_FIELDS: usize = 12;

/// One terminal receipt, in the frozen field order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReceiptEnvelope {
    pub(crate) invocation: String,
    pub(crate) program: String,
    pub(crate) root: String,
    pub(crate) run: String,
    pub(crate) class_pin: String,
    pub(crate) engine: String,
    pub(crate) terminal_sequence: String,
    pub(crate) outcome: String,
    pub(crate) governance: String,
    pub(crate) policy_digest: String,
    pub(crate) enrollment_digest: String,
    pub(crate) authorization_digest: String,
}

impl ReceiptEnvelope {
    /// The fields in envelope order.
    fn fields(&self) -> [&str; ENVELOPE_FIELDS] {
        [
            &self.invocation,
            &self.program,
            &self.root,
            &self.run,
            &self.class_pin,
            &self.engine,
            &self.terminal_sequence,
            &self.outcome,
            &self.governance,
            &self.policy_digest,
            &self.enrollment_digest,
            &self.authorization_digest,
        ]
    }

    /// Encode the exact bytes the child writes to the pipe in one write.
    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(256);
        body.extend_from_slice(ENVELOPE_MAGIC);
        body.push(0);
        for field in self.fields() {
            body.extend_from_slice(&(field.len() as u64).to_be_bytes());
            body.extend_from_slice(field.as_bytes());
        }
        body
    }

    /// Decode one whole envelope. Trailing bytes, a missing magic, or a
    /// frame that stops mid-field is malformed.
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, String> {
        let (envelope, consumed) = Self::parse_prefix(bytes)?;
        if consumed != bytes.len() {
            return Err("the record carries trailing bytes".to_owned());
        }
        Ok(envelope)
    }

    /// Decode one envelope from the front, returning it and the bytes it
    /// consumed; the remainder is the caller's to judge.
    pub(crate) fn parse_prefix(bytes: &[u8]) -> Result<(Self, usize), String> {
        let magic = ENVELOPE_MAGIC.len() + 1;
        if bytes.len() < magic
            || &bytes[..ENVELOPE_MAGIC.len()] != ENVELOPE_MAGIC
            || bytes[ENVELOPE_MAGIC.len()] != 0
        {
            return Err(
                "the record does not open with the ratmac-receipt-v1 magic and one NUL byte"
                    .to_owned(),
            );
        }
        let mut at = magic;
        let mut fields = Vec::with_capacity(ENVELOPE_FIELDS);
        // Exactly twelve fields: anything after them is the caller's
        // remainder, not more envelope.
        while fields.len() < ENVELOPE_FIELDS && at < bytes.len() {
            let length = match bytes.get(at..at + 8) {
                Some(window) => u64::from_be_bytes(window.try_into().expect("eight bytes")),
                None => return Err("the frame stops mid-field".to_owned()),
            };
            at += 8;
            let length = usize::try_from(length)
                .map_err(|_| "a field length overflows the platform".to_owned())?;
            let window = bytes
                .get(at..at + length)
                .ok_or_else(|| "the frame stops mid-field".to_owned())?;
            let text = std::str::from_utf8(window).map_err(|_| "a field is not valid UTF-8")?;
            fields.push(text.to_owned());
            at += length;
        }
        if fields.len() != ENVELOPE_FIELDS {
            return Err(format!(
                "the record holds {} fields; the envelope carries exactly {ENVELOPE_FIELDS}",
                fields.len()
            ));
        }
        let consumed = at;
        let mut fields = fields.into_iter();
        let mut next = || fields.next().expect("count checked above");
        Ok((
            Self {
                invocation: next(),
                program: next(),
                root: next(),
                run: next(),
                class_pin: next(),
                engine: next(),
                terminal_sequence: next(),
                outcome: next(),
                governance: next(),
                policy_digest: next(),
                enrollment_digest: next(),
                authorization_digest: next(),
            },
            consumed,
        ))
    }
}

/// A nested-ledger defect, named. Never a guess, never a smaller set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NestedLedgerError {
    message: String,
}

impl NestedLedgerError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NestedLedgerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for NestedLedgerError {}

/// One recorded nested invocation, fields in the ledger's frozen order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NestedLedgerEntry {
    pub(crate) invocation: String,
    pub(crate) program: String,
    pub(crate) root: String,
    pub(crate) canonical_root: String,
    pub(crate) from_state: String,
    pub(crate) parent_sequence: String,
    pub(crate) engine_pin: String,
    pub(crate) class_pin: String,
    pub(crate) receipt_sha256: String,
    pub(crate) child_run: String,
    pub(crate) consumed: bool,
}

const LEDGER_FIELDS: [&str; 11] = [
    "invocation",
    "program",
    "root",
    "canonical_root",
    "from_state",
    "parent_sequence",
    "engine_pin",
    "class_pin",
    "receipt_sha256",
    "child_run",
    "consumed",
];

/// Strictly read every `[[invocations]]` entry. An absent or empty file is
/// an empty ledger; anything unreadable or malformed refuses by name.
pub(crate) fn read_entries(path: &Path) -> Result<Vec<NestedLedgerEntry>, NestedLedgerError> {
    // WEBV-014: reading a nested ledger is a ledger operation.
    crate::observe::operation("ledger");
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(NestedLedgerError::new(format!(
                "nested ledger {} is unreadable: {error}",
                path.displayed()
            )))
        }
    };
    parse_entries(path, &bytes)
}

fn parse_entries(path: &Path, bytes: &[u8]) -> Result<Vec<NestedLedgerEntry>, NestedLedgerError> {
    let source = std::str::from_utf8(bytes).map_err(|error| {
        NestedLedgerError::new(format!(
            "nested ledger {} is unreadable: {error}",
            path.displayed()
        ))
    })?;
    if source.trim().is_empty() {
        return Ok(Vec::new());
    }
    let value: toml::Value = source.parse().map_err(|error| {
        NestedLedgerError::new(format!(
            "nested ledger {} is not valid TOML: {error}",
            path.displayed()
        ))
    })?;
    if let Some(table) = value.as_table() {
        for key in table.keys() {
            if key != "invocations" {
                return Err(NestedLedgerError::new(format!(
                    "nested ledger {} declares an unknown top-level key {key:?}; only \"invocations\" is recorded",
                    path.displayed()
                )));
            }
        }
    }
    let invocations = value
        .get("invocations")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| {
            NestedLedgerError::new(format!(
                "nested ledger {} \"invocations\" must be an array of tables",
                path.displayed()
            ))
        })?;
    let mut entries = Vec::with_capacity(invocations.len());
    for (index, entry) in invocations.iter().enumerate() {
        let table = entry.as_table().ok_or_else(|| {
            NestedLedgerError::new(format!(
                "nested ledger {} entry {index} must be a table",
                path.displayed()
            ))
        })?;
        let mut strings: Vec<String> = Vec::with_capacity(LEDGER_FIELDS.len() - 1);
        let mut consumed: Option<bool> = None;
        for field in LEDGER_FIELDS {
            let value = table.get(field).ok_or_else(|| {
                NestedLedgerError::new(format!(
                    "nested ledger {} entry {index} is missing {field:?}",
                    path.displayed()
                ))
            })?;
            if field == "consumed" {
                consumed = Some(value.as_bool().ok_or_else(|| {
                    NestedLedgerError::new(format!(
                        "nested ledger {} entry {index}: \"consumed\" must be a boolean",
                        path.displayed()
                    ))
                })?);
                continue;
            }
            let text = value.as_str().ok_or_else(|| {
                NestedLedgerError::new(format!(
                    "nested ledger {} entry {index}: {field:?} must be a string",
                    path.displayed()
                ))
            })?;
            if text.is_empty() && !matches!(field, "class_pin" | "receipt_sha256" | "child_run") {
                return Err(NestedLedgerError::new(format!(
                    "nested ledger {} entry {index}: {field:?} must be non-empty",
                    path.displayed()
                )));
            }
            strings.push(text.to_owned());
        }
        for key in table.keys() {
            if !LEDGER_FIELDS.contains(&key.as_str()) {
                return Err(NestedLedgerError::new(format!(
                    "nested ledger {} entry {index} declares an unknown key {key:?}",
                    path.displayed()
                )));
            }
        }
        let mut strings = strings.into_iter();
        let mut next = || strings.next().expect("ten string fields collected");
        let invocation = next();
        if entries
            .iter()
            .any(|recorded: &NestedLedgerEntry| recorded.invocation == invocation)
        {
            return Err(NestedLedgerError::new(format!(
                "nested ledger {} records {invocation:?} twice; invocation addresses are unique per ledger",
                path.displayed()
            )));
        }
        entries.push(NestedLedgerEntry {
            program: next(),
            root: next(),
            canonical_root: next(),
            from_state: next(),
            parent_sequence: next(),
            engine_pin: next(),
            class_pin: next(),
            receipt_sha256: next(),
            child_run: next(),
            consumed: consumed.expect("the consumed field is required above"),
            invocation,
        });
    }
    Ok(entries)
}

/// Render one entry as the TOML block `append` writes: one `key = value`
/// line per field in the frozen order, one blank line closing the block.
pub(crate) fn render_entry(entry: &NestedLedgerEntry) -> String {
    let mut block = String::from("[[invocations]]\n");
    block.push_str(&format!(
        "invocation = {}\n",
        crate::ledger::toml_string(&entry.invocation)
    ));
    block.push_str(&format!(
        "program = {}\n",
        crate::ledger::toml_string(&entry.program)
    ));
    block.push_str(&format!(
        "root = {}\n",
        crate::ledger::toml_string(&entry.root)
    ));
    block.push_str(&format!(
        "canonical_root = {}\n",
        crate::ledger::toml_string(&entry.canonical_root)
    ));
    block.push_str(&format!(
        "from_state = {}\n",
        crate::ledger::toml_string(&entry.from_state)
    ));
    block.push_str(&format!(
        "parent_sequence = {}\n",
        crate::ledger::toml_string(&entry.parent_sequence)
    ));
    block.push_str(&format!(
        "engine_pin = {}\n",
        crate::ledger::toml_string(&entry.engine_pin)
    ));
    block.push_str(&format!(
        "class_pin = {}\n",
        crate::ledger::toml_string(&entry.class_pin)
    ));
    block.push_str(&format!(
        "receipt_sha256 = {}\n",
        crate::ledger::toml_string(&entry.receipt_sha256)
    ));
    block.push_str(&format!(
        "child_run = {}\n",
        crate::ledger::toml_string(&entry.child_run)
    ));
    block.push_str(&format!("consumed = {}\n\n", entry.consumed));
    block
}

/// Prove the ledger bytes are exactly the Engine's own canonical render of
/// their strict parse before any byte edit (WEB-006). A strictly parseable
/// but non-canonical ledger - a multi-line basic string smuggling lines
/// that read like another entry's header or fields - refuses through the
/// byte-edit family's strict wording, never gets edited. After the check,
/// every `invocation = "..."` at a line start is a real entry's own first
/// field, so the locator's line-start match stands. Appends and reads are
/// unchanged.
fn require_canonical_bytes(
    path: &Path,
    entries: &[NestedLedgerEntry],
    bytes: &[u8],
) -> Result<(), NestedLedgerError> {
    let mut canonical = String::new();
    for entry in entries {
        canonical.push_str(&render_entry(entry));
    }
    if bytes != canonical.as_bytes() {
        return Err(NestedLedgerError::new(format!(
            "nested ledger {} is not the canonical render of its strict parse; \
             refusing a byte edit on unrecognized shape",
            path.displayed()
        )));
    }
    Ok(())
}

/// Append one entry after the preserved prior bytes. The strict read runs
/// first: a corrupt ledger is never extended.
pub(crate) fn append_entry(
    path: &Path,
    entry: &NestedLedgerEntry,
) -> Result<(), NestedLedgerError> {
    let prior = read_entries(path)?;
    if prior
        .iter()
        .any(|recorded| recorded.invocation == entry.invocation)
    {
        return Err(NestedLedgerError::new(format!(
            "nested ledger {} already records {:?}; invocation addresses are unique per ledger",
            path.displayed(),
            entry.invocation
        )));
    }
    // WEBV-014: appending a nested-ledger entry is a ledger operation,
    // the spawn ledger's own mutation mark.
    crate::observe::operation("ledger");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| {
            NestedLedgerError::new(format!(
                "nested ledger {} cannot open for append: {error}",
                path.displayed()
            ))
        })?;
    file.write_all(render_entry(entry).as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|error| {
            NestedLedgerError::new(format!(
                "nested ledger {} cannot append durably: {error}",
                path.displayed()
            ))
        })
}

/// Rewrite exactly one field of exactly one entry in place, leaving every
/// other byte untouched. Writing the value already there is a no-op.
pub(crate) fn annotate(
    path: &Path,
    invocation: &str,
    key: &str,
    value: &str,
) -> Result<(), NestedLedgerError> {
    let entries = read_entries(path)?;
    let entry = entries
        .iter()
        .find(|entry| entry.invocation == invocation)
        .ok_or_else(|| {
            NestedLedgerError::new(format!(
                "nested ledger {} names no invocation {invocation:?}",
                path.displayed()
            ))
        })?;
    // The bytes must be the canonical render of this strict parse before
    // any edit lands: a parseable but crafted ledger refuses, exactly as
    // an unrecognizable one does.
    let bytes = fs::read(path).map_err(|error| {
        NestedLedgerError::new(format!(
            "nested ledger {} is unreadable: {error}",
            path.displayed()
        ))
    })?;
    require_canonical_bytes(path, &entries, &bytes)?;
    let current = match key {
        "class_pin" => entry.class_pin.as_str(),
        "receipt_sha256" => entry.receipt_sha256.as_str(),
        "child_run" => entry.child_run.as_str(),
        other => {
            return Err(NestedLedgerError::new(format!(
                "nested ledger {} entry {invocation:?} cannot be annotated at {other:?}",
                path.displayed()
            )))
        }
    };
    if current == value {
        return Ok(());
    }
    crate::observe::operation("ledger");
    rewrite_field(path, invocation, key, &crate::ledger::toml_string(value))
}

/// One entry's `consumed` flip, as the consuming transition must remember
/// it: the ledger's pre-flip bytes and the exact flipped bytes the flip
/// commits. Both are computed in memory before anything is written, so the
/// caller registers its rollback from these facts alone, with no disk
/// read - and nothing fallible - between a successful flip and its
/// registration.
pub(crate) struct ConsumedFlip {
    pub(crate) before: Vec<u8>,
    pub(crate) flipped: Vec<u8>,
}

/// Flip one entry's `consumed` mark from `false` to `true`, returning the
/// pre-flip and flipped bytes so a graceful rollback can restore the mark
/// exactly. An already-flipped entry is left alone.
pub(crate) fn consume(
    path: &Path,
    invocation: &str,
) -> Result<Option<ConsumedFlip>, NestedLedgerError> {
    let entries = read_entries(path)?;
    let entry = entries
        .iter()
        .find(|entry| entry.invocation == invocation)
        .ok_or_else(|| {
            NestedLedgerError::new(format!(
                "nested ledger {} names no invocation {invocation:?}",
                path.displayed()
            ))
        })?;
    if entry.consumed {
        return Ok(None);
    }
    crate::observe::operation("ledger");
    let before = fs::read(path).map_err(|error| {
        NestedLedgerError::new(format!(
            "nested ledger {} is unreadable: {error}",
            path.displayed()
        ))
    })?;
    require_canonical_bytes(path, &entries, &before)?;
    let source = std::str::from_utf8(&before).map_err(|_| {
        NestedLedgerError::new(format!(
            "nested ledger {} is not valid UTF-8; refusing a byte edit on unrecognized shape",
            path.displayed()
        ))
    })?;
    let (field_at, line_end) = locate_field(source, invocation, "consumed", path)?;
    let mut flipped = Vec::with_capacity(before.len());
    flipped.extend_from_slice(&before[..field_at]);
    flipped.extend_from_slice(b"consumed = true");
    flipped.extend_from_slice(&before[line_end..]);
    // A read-only ledger is a real-I/O refusal on both platforms: Unix
    // rename would replace a read-only target where the old in-place write
    // refused, so the target is opened for writing first and its own
    // denial wording - the same real-I/O text the in-place write produced -
    // surfaces, leaving the ledger untouched.
    if let Err(error) = fs::OpenOptions::new().write(true).open(path) {
        return Err(NestedLedgerError::new(format!(
            "nested ledger {} cannot commit its consumed flip: {error}",
            path.displayed()
        )));
    }
    // The commit is all-or-nothing: the whole flipped content lands through
    // one same-directory temporary file and one atomic rename, so a write
    // or rename failure leaves the ledger wholly pre-flip - no flip to
    // register, nothing to restore. Only a death between the temp write
    // and the rename can strand the temporary file, and a death may leave
    // today's deliberate limits.
    let temp = path.with_extension("consumed-flip");
    let commit = fs::write(&temp, &flipped).and_then(|()| fs::rename(&temp, path));
    if let Err(error) = commit {
        let _ = fs::remove_file(&temp);
        return Err(NestedLedgerError::new(format!(
            "nested ledger {} cannot commit its consumed flip: {error}",
            path.displayed()
        )));
    }
    Ok(Some(ConsumedFlip { before, flipped }))
}

/// Restore one entry's `consumed` mark to its pre-flip bytes' value,
/// comparing the current bytes against `flipped` first: a ledger another
/// writer changed is never rewritten.
pub(crate) fn restore_consumed(
    path: &Path,
    invocation: &str,
    flipped: &[u8],
    before: &[u8],
) -> Result<(), NestedLedgerError> {
    let _ = invocation;
    let current = fs::read(path).map_err(|error| {
        NestedLedgerError::new(format!(
            "nested ledger {} is unreadable: {error}",
            path.displayed()
        ))
    })?;
    if current != flipped {
        return Err(NestedLedgerError::new(format!(
            "nested ledger {} changed while the motion was prepared; the consumed mark was not restored",
            path.displayed()
        )));
    }
    fs::write(path, before).map_err(|error| {
        NestedLedgerError::new(format!(
            "nested ledger {} cannot restore the consumed mark: {error}",
            path.displayed()
        ))
    })
}

/// Locate the one `key = ` line the byte edits touch: inside the entry's
/// block, found by its invocation line, matched only at a line start - a
/// program identity or root spelling that happens to contain the field's
/// `key = ` bytes inside it can never redirect the edit. Returns the byte
/// offsets of the field start and the end of its line.
fn locate_field(
    source: &str,
    invocation: &str,
    key: &str,
    path: &Path,
) -> Result<(usize, usize), NestedLedgerError> {
    let needle = format!("invocation = {}", crate::ledger::toml_string(invocation));
    // The invocation line is matched only at a line start, with the same
    // line discipline as the field match below: a parseable TOML comment
    // quoting another entry's `invocation = "..."` inside an earlier block
    // can never steer the edit into the wrong entry.
    let mut found = None;
    let mut line_start = 0;
    while line_start < source.len() {
        let line_end = source[line_start..]
            .find('\n')
            .map_or(source.len(), |offset| line_start + offset);
        if source[line_start..line_end].starts_with(&needle) {
            found = Some(line_start);
            break;
        }
        line_start = line_end + 1;
    }
    let at = found.ok_or_else(|| {
        NestedLedgerError::new(format!(
            "nested ledger {} names {invocation:?} but the entry's invocation line is not found \
             verbatim; refusing a byte edit on unrecognized shape",
            path.displayed()
        ))
    })?;
    let block_start = source[..at].rfind("[[invocations]]").unwrap_or(0);
    let block_end = source[at..]
        .find("[[invocations]]")
        .map_or(source.len(), |offset| at + offset);
    let wanted = format!("{key} = ");
    let mut field_at = None;
    let mut line_start = block_start;
    while line_start < block_end {
        let line_end = source[line_start..block_end]
            .find('\n')
            .map_or(block_end, |offset| line_start + offset);
        if source[line_start..line_end].starts_with(&wanted) {
            field_at = Some(line_start);
            break;
        }
        line_start = line_end + 1;
    }
    let field_at = field_at.ok_or_else(|| {
        NestedLedgerError::new(format!(
            "nested ledger {} entry {invocation:?} carries no {wanted}mark to rewrite",
            path.displayed()
        ))
    })?;
    let line_end = source[field_at..]
        .find('\n')
        .map(|offset| field_at + offset)
        .unwrap_or(source.len());
    Ok((field_at, line_end))
}

/// The one byte edit annotate, consume, and the rollback reduce to: find
/// the entry's block by its invocation line, then replace that one
/// `key = <rendered value>` line inside the block.
fn rewrite_field(
    path: &Path,
    invocation: &str,
    key: &str,
    rendered_value: &str,
) -> Result<(), NestedLedgerError> {
    let source = fs::read_to_string(path).map_err(|error| {
        NestedLedgerError::new(format!(
            "nested ledger {} is unreadable: {error}",
            path.displayed()
        ))
    })?;
    let (field_at, line_end) = locate_field(&source, invocation, key, path)?;
    let mut rewritten = String::with_capacity(source.len() + rendered_value.len());
    rewritten.push_str(&source[..field_at]);
    rewritten.push_str(key);
    rewritten.push_str(" = ");
    rewritten.push_str(rendered_value);
    rewritten.push_str(&source[line_end..]);
    fs::write(path, rewritten).map_err(|error| {
        NestedLedgerError::new(format!(
            "nested ledger {} cannot record the {key} field: {error}",
            path.displayed()
        ))
    })
}

// --- the dedicated receipt pipe ------------------------------------------------

/// The dedicated receipt pipe: an OS pipe the parent creates for one
/// invocation. The parent owns the read end; only the write end - as the
/// raw platform value, unsigned decimal - reaches the endpoint process.
///
/// The FFI below is deliberately minimal and hand-declared, following the
/// `src/diag.rs` precedent: `pipe(2)` on Unix, `CreatePipe` on Windows, and
/// nothing else. Standard values carry no special meaning.
pub(crate) struct ReceiptPipe {
    read: std::fs::File,
    /// The write end's raw platform value in unsigned decimal.
    pub(crate) descriptor: String,
    write: RawWriteEnd,
}

#[cfg(windows)]
struct RawWriteEnd {
    handle: isize,
}

#[cfg(unix)]
struct RawWriteEnd {
    fd: i32,
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreatePipe(read: *mut isize, write: *mut isize, attributes: *const u8, size: u32) -> i32;
    fn SetHandleInformation(handle: isize, mask: u32, flags: u32) -> i32;
    fn CloseHandle(handle: isize) -> i32;
    fn WriteFile(
        handle: isize,
        bytes: *const u8,
        length: u32,
        written: *mut u32,
        overlapped: *const u8,
    ) -> i32;
}

#[cfg(windows)]
const HANDLE_FLAG_INHERIT: u32 = 0x0000_0001;

#[cfg(unix)]
#[link(name = "c")]
unsafe extern "C" {
    fn pipe(fds: *mut i32) -> i32;
    fn write(fd: i32, bytes: *const u8, count: usize) -> isize;
    fn close(fd: i32) -> i32;
    fn fcntl(fd: i32, command: i32, argument: i32) -> i32;
}

#[cfg(unix)]
const F_SETFD: i32 = 2;
#[cfg(unix)]
const FD_CLOEXEC: i32 = 1;

impl ReceiptPipe {
    /// Create the pipe and prepare the write end for exactly one launch:
    /// inheritable for this launch alone on Windows, without close-on-exec
    /// on Unix. The read end stays private to this process.
    pub(crate) fn create() -> std::io::Result<Self> {
        #[cfg(windows)]
        {
            let mut read = 0isize;
            let mut write = 0isize;
            // SAFETY: both out-pointers are live locals; the returned
            // handles are owned here and released below or by the endpoint.
            if unsafe { CreatePipe(&mut read, &mut write, std::ptr::null(), 0) } == 0 {
                return Err(std::io::Error::last_os_error());
            }
            // SAFETY: the handle is the one CreatePipe just returned.
            if unsafe { SetHandleInformation(write, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) } == 0
            {
                let error = std::io::Error::last_os_error();
                unsafe { CloseHandle(read) };
                unsafe { CloseHandle(write) };
                return Err(error);
            }
            // SAFETY: the read handle came from CreatePipe and transfers to
            // the File, which closes it on drop.
            let file = unsafe { std::os::windows::io::FromRawHandle::from_raw_handle(read as _) };
            Ok(Self {
                read: file,
                descriptor: write.to_string(),
                write: RawWriteEnd { handle: write },
            })
        }
        #[cfg(unix)]
        {
            let mut fds = [0i32; 2];
            // SAFETY: the out-pointer covers both descriptors; both are
            // owned here and released below or passed to the endpoint.
            if unsafe { pipe(fds.as_mut_ptr()) } != 0 {
                return Err(std::io::Error::last_os_error());
            }
            let (read, write) = (fds[0], fds[1]);
            // The parent's read end must not survive into any later child.
            // SAFETY: a live descriptor and plain integer flags.
            if unsafe { fcntl(read, F_SETFD, FD_CLOEXEC) } != 0 {
                let error = std::io::Error::last_os_error();
                unsafe { close(read) };
                unsafe { close(write) };
                return Err(error);
            }
            // SAFETY: the descriptor came from pipe(2) and transfers to the
            // File, which closes it on drop.
            let file = unsafe { std::os::unix::io::FromRawFd::from_raw_fd(read) };
            Ok(Self {
                read: file,
                descriptor: write.to_string(),
                write: RawWriteEnd { fd: write },
            })
        }
    }

    /// The parent drops its own write end right after the spawn, so the
    /// read reaches EOF once the endpoint finishes.
    pub(crate) fn drop_write_end(&mut self) {
        #[cfg(windows)]
        {
            if self.write.handle != 0 {
                // SAFETY: the handle is owned here and released once.
                unsafe { CloseHandle(self.write.handle) };
                self.write.handle = 0;
            }
        }
        #[cfg(unix)]
        {
            if self.write.fd >= 0 {
                // SAFETY: the descriptor is owned here and released once.
                unsafe { close(self.write.fd) };
                self.write.fd = -1;
            }
        }
    }

    /// Read the receipt bytes to EOF.
    pub(crate) fn read_to_end(mut self) -> std::io::Result<Vec<u8>> {
        use std::io::Read;
        let mut bytes = Vec::new();
        self.read.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

/// Make the endpoint's receipt descriptor non-inheritable (close-on-exec)
/// before it launches anything, so no driver receives it.
pub(crate) fn seclude_receipt_descriptor(descriptor: u64) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let handle = descriptor as isize;
        // SAFETY: the value is the descriptor this process was handed; the
        // flags change touches inheritance only.
        if unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(unix)]
    {
        let fd = i32::try_from(descriptor).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "descriptor overflow")
        })?;
        // SAFETY: a live descriptor and plain integer flags.
        if unsafe { fcntl(fd, F_SETFD, FD_CLOEXEC) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}

/// Write the whole envelope to the receipt descriptor in one write, then
/// close it.
pub(crate) fn write_receipt(descriptor: u64, bytes: &[u8]) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let handle = descriptor as isize;
        let mut written = 0u32;
        let mut at = 0usize;
        while at < bytes.len() {
            let length = u32::try_from(bytes.len() - at).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "oversized write")
            })?;
            // SAFETY: the handle is this process's receipt descriptor; the
            // buffer is live for the call.
            if unsafe {
                WriteFile(
                    handle,
                    bytes[at..].as_ptr(),
                    length,
                    &mut written,
                    std::ptr::null(),
                )
            } == 0
            {
                let error = std::io::Error::last_os_error();
                // SAFETY: owned here.
                unsafe { CloseHandle(handle) };
                return Err(error);
            }
            at += written as usize;
        }
        // SAFETY: owned here; the endpoint owes nothing else on it.
        unsafe { CloseHandle(handle) };
        Ok(())
    }
    #[cfg(unix)]
    {
        let fd = i32::try_from(descriptor).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "descriptor overflow")
        })?;
        let mut at = 0usize;
        while at < bytes.len() {
            // SAFETY: the descriptor and buffer are live for the call.
            let count = unsafe { write(fd, bytes[at..].as_ptr(), bytes.len() - at) };
            if count < 0 {
                let error = std::io::Error::last_os_error();
                // SAFETY: owned here.
                unsafe { close(fd) };
                return Err(error);
            }
            at += count as usize;
        }
        // SAFETY: owned here; the endpoint owes nothing else on it.
        unsafe { close(fd) };
        Ok(())
    }
}

// --- QA-only pause companions --------------------------------------------------

/// One injected-pause seam: the selector variable, its companion variables,
/// and the refusal prefix each companion defect renders under. The fields
/// are read only in the fault-injection build that honors the seams.
#[cfg_attr(not(feature = "test-fault-injection"), allow(dead_code))]
pub(crate) struct PauseSpec {
    pub(crate) selector: &'static str,
    pub(crate) marker: &'static str,
    pub(crate) release: &'static str,
    pub(crate) timeout: &'static str,
    /// The seam's name as a refusal names it.
    pub(crate) name: &'static str,
    /// The seam's word in an expired-pause refusal (`injected <word> pause
    /// at <boundary> expired`).
    pub(crate) word: &'static str,
    /// The boundary names this seam holds. A selector value outside this
    /// list changes nothing, so an unset, empty, or unrecognized value
    /// never fires the companions contract.
    pub(crate) boundaries: &'static [&'static str],
}

/// The parent-side nested pause seam.
pub(crate) const NESTED_PAUSE: PauseSpec = PauseSpec {
    selector: "RATMAC_TEST_NESTED_PAUSE",
    marker: "RATMAC_TEST_NESTED_PAUSE_MARKER",
    release: "RATMAC_TEST_NESTED_PAUSE_RELEASE",
    timeout: "RATMAC_TEST_NESTED_PAUSE_TIMEOUT_MILLIS",
    name: "RATMAC_TEST_NESTED_PAUSE",
    word: "nested",
    boundaries: &["after-record", "after-receipt", "after-consume"],
};

/// The endpoint-side pause seam.
pub(crate) const ENDPOINT_PAUSE: PauseSpec = PauseSpec {
    selector: "RATMAC_TEST_ENDPOINT_PAUSE",
    marker: "RATMAC_TEST_ENDPOINT_PAUSE_MARKER",
    release: "RATMAC_TEST_ENDPOINT_PAUSE_RELEASE",
    timeout: "RATMAC_TEST_ENDPOINT_PAUSE_TIMEOUT_MILLIS",
    name: "RATMAC_TEST_ENDPOINT_PAUSE",
    word: "endpoint",
    boundaries: &["after-admit", "after-terminal", "after-receipt"],
};

#[cfg(feature = "test-fault-injection")]
const MAX_PAUSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

#[cfg(feature = "test-fault-injection")]
fn pause_timeout(spec: &PauseSpec) -> Result<std::time::Duration, String> {
    // The timeout is a required companion: unset, empty, or unparsable
    // values are a missing or invalid companion, never a silent default.
    let milliseconds = std::env::var(spec.timeout)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|milliseconds| *milliseconds > 0)
        .ok_or_else(|| spec.timeout.to_owned())?;
    Ok(std::time::Duration::from_millis(milliseconds).min(MAX_PAUSE_TIMEOUT))
}

#[cfg(feature = "test-fault-injection")]
/// Validate one pause seam's companions: a marker path, a release path, and
/// a positive-integer timeout when the selector is set. Compiled only with
/// `test-fault-injection`; production builds expose none of these seams.
pub(crate) fn validate_pause_companions(spec: &PauseSpec) -> Result<(), String> {
    let Ok(value) = std::env::var(spec.selector) else {
        return Ok(());
    };
    // An empty or unrecognized value changes nothing: the companions
    // contract fires only for a boundary this seam actually holds.
    if !spec.boundaries.contains(&value.as_str()) {
        return Ok(());
    }
    if std::env::var_os(spec.marker).is_none() {
        return Err(companion_defect(spec, spec.marker));
    }
    if std::env::var_os(spec.release).is_none() {
        return Err(companion_defect(spec, spec.release));
    }
    if pause_timeout(spec).is_err() {
        return Err(companion_defect(spec, spec.timeout));
    }
    Ok(())
}

/// One missing or invalid companion, named by its full variable name.
#[cfg_attr(not(feature = "test-fault-injection"), allow(dead_code))]
fn companion_defect(spec: &PauseSpec, name: &str) -> String {
    format!("{} companion {name} is missing or invalid", spec.name)
}

#[cfg(not(feature = "test-fault-injection"))]
pub(crate) fn validate_pause_companions(_spec: &PauseSpec) -> Result<(), String> {
    Ok(())
}

#[cfg(feature = "test-fault-injection")]
/// Pause at `boundary` when the seam names it: write the boundary name and
/// a newline to the marker path in one step, then wait for the release
/// path to exist. Leaves exactly the boundary's disk state behind.
pub(crate) fn hold_pause_if_requested(spec: &PauseSpec, boundary: &str) -> Result<(), String> {
    if std::env::var(spec.selector).ok().as_deref() != Some(boundary) {
        return Ok(());
    }
    let marker = std::env::var_os(spec.marker)
        .map(std::path::PathBuf::from)
        .ok_or_else(|| companion_defect(spec, spec.marker))?;
    let release = std::env::var_os(spec.release)
        .map(std::path::PathBuf::from)
        .ok_or_else(|| companion_defect(spec, spec.release))?;
    fs::write(&marker, format!("{boundary}\n"))
        .map_err(|error| format!("write pause marker {}: {error}", marker.displayed()))?;
    let deadline = std::time::Instant::now()
        + pause_timeout(spec).map_err(|_| companion_defect(spec, spec.timeout))?;
    loop {
        if release.is_file() {
            return Ok(());
        }
        let now = std::time::Instant::now();
        if now >= deadline {
            return Err(format!(
                "injected {} pause at {boundary} expired",
                spec.word
            ));
        }
        std::thread::sleep(
            std::time::Duration::from_millis(10).min(deadline.saturating_duration_since(now)),
        );
    }
}

#[cfg(not(feature = "test-fault-injection"))]
pub(crate) fn hold_pause_if_requested(_spec: &PauseSpec, _boundary: &str) -> Result<(), String> {
    Ok(())
}
