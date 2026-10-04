//! WRS-003: pause occurrences, the operation journal, and recovery.
//!
//! Hold and resume - and nothing else - publish through the operation
//! journal `<Engine root>/runs/<id>/journal/`. Both verbs prepare every
//! output before publishing, publish one whole committed entry as their
//! commit decision, then finish the prepared outputs idempotently and
//! remove the entry. Any later mutating command on the Run reconciles a
//! committed entry first, under the Run lock, so a killed or gracefully
//! failed publication is finished exactly once. Readers - `rtm status`,
//! `rtm doctor` - report a pending reconciliation and repair nothing.
//!
//! This module owns the two Engine-owned record shapes of the pause
//! family: the strict version-1 pause record `pauses/<n>.toml`, whose
//! `held-context-sha256` recomputes from the contract's framed context,
//! and the strict version-1 committed entry
//! `journal/<action>-<n>.committed`, whose bytes are sufficient to finish
//! the action without recomputing any decision.

use std::fs;
use std::path::{Path, PathBuf};

use crate::root::Displayed;

/// The pause record's digest prefix: the held context is
/// `ratmac-held-context-v1`, one NUL byte, then each field framed.
const HELD_CONTEXT_PREFIX: &[u8] = b"ratmac-held-context-v1";

/// The committed entry's action for a hold.
pub(crate) const HOLD_ACTION: &str = "hold";
/// The committed entry's action for a resume.
pub(crate) const RESUME_ACTION: &str = "resume";

/// The ten fields of a strict version-1 pause record, in file order.
const PAUSE_FIELDS: [&str; 10] = [
    "version",
    "sequence",
    "run",
    "state",
    "blocker",
    "class",
    "workspace",
    "runbook-sha256",
    "goal-identity",
    "held-context-sha256",
];

/// One pause record: the durable fact of one committed hold.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PauseRecord {
    pub sequence: u64,
    pub run: String,
    pub state: String,
    pub blocker: String,
    pub class: String,
    pub workspace: String,
    pub runbook_sha256: String,
    pub goal_identity: String,
    pub held_context_sha256: String,
}

/// Frame one field: its byte length as unsigned 64-bit big endian, then
/// its bytes.
fn frame(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value);
}

/// The held-record digest: SHA-256 over the framed held context.
pub(crate) fn held_context_digest(record: &PauseRecord) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(HELD_CONTEXT_PREFIX);
    bytes.push(0);
    frame(&mut bytes, record.run.as_bytes());
    frame(&mut bytes, record.state.as_bytes());
    frame(&mut bytes, record.blocker.as_bytes());
    frame(&mut bytes, record.class.as_bytes());
    frame(&mut bytes, record.workspace.as_bytes());
    frame(&mut bytes, record.runbook_sha256.as_bytes());
    frame(&mut bytes, record.goal_identity.as_bytes());
    crate::review::sha256_hex(&bytes)
}

/// Render one pause record's exact bytes.
pub(crate) fn render_pause_record(record: &PauseRecord) -> Vec<u8> {
    let mut text = String::new();
    text.push_str("version = 1\n");
    text.push_str(&format!("sequence = {}\n", record.sequence));
    text.push_str(&format!("run = {}\n", toml_string(&record.run)));
    text.push_str(&format!("state = {}\n", toml_string(&record.state)));
    text.push_str(&format!("blocker = {}\n", toml_string(&record.blocker)));
    text.push_str(&format!("class = {}\n", toml_string(&record.class)));
    text.push_str(&format!("workspace = {}\n", toml_string(&record.workspace)));
    text.push_str(&format!(
        "runbook-sha256 = {}\n",
        toml_string(&record.runbook_sha256)
    ));
    text.push_str(&format!(
        "goal-identity = {}\n",
        toml_string(&record.goal_identity)
    ));
    text.push_str(&format!(
        "held-context-sha256 = {}\n",
        toml_string(&record.held_context_sha256)
    ));
    text.into_bytes()
}

/// The Engine-owned facts a pause record carries beyond the motion itself,
/// read from the Engine's own records at hold time.
pub(crate) struct PauseContext {
    /// `runbook` for a top-level Run, the recorded child class name for a
    /// ledger-recorded child.
    pub class: String,
    /// The recorded workspace, canonical, forward slashes.
    pub workspace: String,
    /// The pinned runbook hash.
    pub runbook_sha256: String,
    /// The frozen goal revision once frozen, else the baseline, else empty.
    pub goal_identity: String,
}

/// Build and render one hold's pause record at `occurrence`, sealing its
/// held-context digest.
pub(crate) fn pause_record_bytes(
    run_id: &str,
    held_state: &str,
    blocker: &str,
    context: &PauseContext,
    occurrence: u64,
) -> Vec<u8> {
    let mut record = PauseRecord {
        sequence: occurrence,
        run: run_id.to_owned(),
        state: held_state.to_owned(),
        blocker: blocker.to_owned(),
        class: context.class.clone(),
        workspace: context.workspace.clone(),
        runbook_sha256: context.runbook_sha256.clone(),
        goal_identity: context.goal_identity.clone(),
        held_context_sha256: String::new(),
    };
    record.held_context_sha256 = held_context_digest(&record);
    render_pause_record(&record)
}

/// The pause-record family's refusal line: `pause record <path> is not a
/// strict version-1 record: <detail>`.
fn record_refusal(path: &Path, detail: &str) -> String {
    format!(
        "pause record {} is not a strict version-1 record: {detail}",
        path.displayed()
    )
}

/// Parse and strictly validate the pause record at `path`. The record
/// holds exactly the declared fields; its `sequence` equals its filename;
/// its digest matches the recomputed context.
pub(crate) fn parse_pause_record(path: &Path) -> Result<PauseRecord, String> {
    parse_pause_record_with(path, true)
}

/// Parse the pause record with or without the recomputed-context check:
/// the strict form serves the motions, which refuse a record whose
/// recorded facts do not hold; the lenient form serves the resume
/// intent's read-only render, which binds the digest exactly as
/// recorded.
fn parse_pause_record_with(path: &Path, require_digest_match: bool) -> Result<PauseRecord, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| record_refusal(path, &format!("cannot read: {error}")))?;
    let value: toml::Value = text
        .parse()
        .map_err(|error: toml::de::Error| record_refusal(path, &error.to_string()))?;
    let Some(table) = value.as_table() else {
        return Err(record_refusal(path, "the document is not a table"));
    };
    for field in PAUSE_FIELDS {
        if !table.contains_key(field) {
            return Err(record_refusal(path, &format!("missing field {field:?}")));
        }
    }
    for key in table.keys() {
        if !PAUSE_FIELDS.contains(&key.as_str()) {
            return Err(record_refusal(path, &format!("unknown field {key:?}")));
        }
    }
    if value.get("version").and_then(toml::Value::as_integer) != Some(1) {
        return Err(record_refusal(path, "version is not 1"));
    }
    let Some(sequence) = value.get("sequence").and_then(toml::Value::as_integer) else {
        return Err(record_refusal(path, "sequence is not an integer"));
    };
    if sequence <= 0 || sequence as u64 > i64::MAX as u64 {
        return Err(record_refusal(
            path,
            "sequence is not a positive 64-bit integer",
        ));
    }
    let sequence = sequence as u64;
    let name = crate::root::component(path.file_name().unwrap_or_default());
    let Some(stem) = name.strip_suffix(".toml") else {
        return Err(record_refusal(path, "the file name is not an occurrence"));
    };
    if stem != sequence.to_string() {
        return Err(record_refusal(
            path,
            "sequence does not equal its file name",
        ));
    }
    let string_field = |field: &str| -> Result<String, String> {
        value
            .get(field)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| record_refusal(path, &format!("{field} is not a string")))
    };
    let record = PauseRecord {
        sequence,
        run: string_field("run")?,
        state: string_field("state")?,
        blocker: string_field("blocker")?,
        class: string_field("class")?,
        workspace: string_field("workspace")?,
        runbook_sha256: string_field("runbook-sha256")?,
        goal_identity: string_field("goal-identity")?,
        held_context_sha256: string_field("held-context-sha256")?,
    };
    if require_digest_match && record.held_context_sha256 != held_context_digest(&record) {
        return Err(record_refusal(
            path,
            "the held-context digest does not match the recomputed context",
        ));
    }
    Ok(record)
}

/// The pauses directory of one Run.
fn pauses_dir(run_dir: &Path) -> PathBuf {
    run_dir.join("pauses")
}

/// One pause record's path at its occurrence.
pub(crate) fn pauses_path(run_dir: &Path, occurrence: u64) -> PathBuf {
    pauses_dir(run_dir).join(format!("{occurrence}.toml"))
}

/// The occurrence-refusal text for one noncanonical member name.
fn noncanonical_occurrence(name: &str) -> String {
    format!("pause occurrence {name} is not a positive decimal integer without leading zeroes")
}

/// A canonical occurrence spelling: positive, decimal, no leading zeroes.
pub(crate) fn canonical_occurrence_of(stem: &str) -> Option<u64> {
    if stem.len() > 1 && stem.starts_with('0') {
        return None;
    }
    let occurrence = stem.parse::<u64>().ok()?;
    (occurrence > 0).then_some(occurrence)
}

/// Every canonical occurrence on disk, ascending. A member whose name is
/// not a positive decimal integer without leading zeroes makes the
/// occurrences untrusted.
fn canonical_occurrences(run_dir: &Path) -> Result<Vec<u64>, String> {
    let entries = match fs::read_dir(pauses_dir(run_dir)) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("read the pause records: {error}")),
    };
    let mut occurrences = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("read a pause record entry: {error}"))?;
        if entry.path().is_dir() {
            return Err(noncanonical_occurrence(&crate::root::component(
                entry.file_name(),
            )));
        }
        let name = crate::root::component(entry.file_name());
        let Some(stem) = name.strip_suffix(".toml") else {
            return Err(noncanonical_occurrence(&name));
        };
        let Some(occurrence) = canonical_occurrence_of(stem) else {
            return Err(noncanonical_occurrence(stem));
        };
        occurrences.push(occurrence);
    }
    occurrences.sort_unstable();
    Ok(occurrences)
}

/// The next hold occurrence: one past the highest canonical member, or 1
/// when the directory is absent or empty. An exhausted sequence refuses
/// rather than wrapping.
pub(crate) fn next_occurrence(run_dir: &Path, run_id: &str) -> Result<u64, String> {
    let occurrences = canonical_occurrences(run_dir)?;
    match occurrences.last() {
        None => Ok(1),
        Some(&u64::MAX) => Err(format!(
            "the next pause occurrence for run {run_id} is unavailable; refusing rather than \
             wrapping"
        )),
        Some(&highest) => Ok(highest + 1),
    }
}

/// The live occurrence: the highest canonical member, or `None` when the
/// Run holds no canonical pause record.
pub(crate) fn live_occurrence(run_dir: &Path) -> Result<Option<u64>, String> {
    Ok(canonical_occurrences(run_dir)?.last().copied())
}

/// The live occurrence's strict record, with its occurrence. `Err("")`
/// answers when the Run holds no canonical pause record.
pub(crate) fn load_live_record(run_dir: &Path) -> Result<(u64, PauseRecord), String> {
    let Some(occurrence) = live_occurrence(run_dir)? else {
        return Err(String::new());
    };
    let path = pauses_dir(run_dir).join(format!("{occurrence}.toml"));
    Ok((occurrence, parse_pause_record(&path)?))
}

/// The live record parsed leniently: the resume intent's render binds the
/// recorded held-context digest without re-validating it, so a forged
/// digest still renders and the motion's own strict check still refuses.
pub(crate) fn load_live_record_lenient(run_dir: &Path) -> Result<(u64, PauseRecord), String> {
    let Some(occurrence) = live_occurrence(run_dir)? else {
        return Err(String::new());
    };
    let path = pauses_dir(run_dir).join(format!("{occurrence}.toml"));
    Ok((occurrence, parse_pause_record_with(&path, false)?))
}

/// One committed journal entry: every byte needed to finish the action
/// without recomputing any decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct JournalEntry {
    /// `hold` or `resume`.
    pub action: String,
    pub run: String,
    pub occurrence: u64,
    pub blocker: String,
    /// The motion's from-State.
    pub state: String,
    /// The motion's to-State: the routed destination for a hold, the held
    /// State again for a resume.
    pub to_state: String,
    /// The complete pre-motion Run Record bytes.
    pub old_record: Vec<u8>,
    /// The complete post-motion Run Record bytes.
    pub new_record: Vec<u8>,
    /// The prepared pause record bytes (hold only).
    pub pause_record: Option<Vec<u8>>,
    /// The prepared authorization record bytes and its sequence (governed
    /// only).
    pub authorization_record: Option<(u64, Vec<u8>)>,
    /// The one history line the action appends.
    pub history_line: String,
}

/// The journal directory of one Run.
pub(crate) fn journal_dir(run_dir: &Path) -> PathBuf {
    run_dir.join("journal")
}

/// The committed entry's file name for one action and occurrence.
pub(crate) fn entry_name(action: &str, occurrence: u64) -> String {
    format!("{action}-{occurrence}.committed")
}

/// Render one committed entry's exact bytes.
pub(crate) fn render_journal_entry(entry: &JournalEntry) -> Vec<u8> {
    let embedded = |raw: &[u8]| -> String { toml_string(&String::from_utf8_lossy(raw)) };
    let mut text = String::new();
    text.push_str("version = 1\n");
    text.push_str(&format!("action = {}\n", toml_string(&entry.action)));
    text.push_str(&format!("run = {}\n", toml_string(&entry.run)));
    text.push_str(&format!("occurrence = {}\n", entry.occurrence));
    text.push_str(&format!("blocker = {}\n", toml_string(&entry.blocker)));
    text.push_str(&format!("state = {}\n", toml_string(&entry.state)));
    text.push_str(&format!("to-state = {}\n", toml_string(&entry.to_state)));
    text.push_str(&format!("old-record = {}\n", embedded(&entry.old_record)));
    text.push_str(&format!("new-record = {}\n", embedded(&entry.new_record)));
    if let Some(pause_record) = &entry.pause_record {
        text.push_str(&format!("pause-record = {}\n", embedded(pause_record)));
    }
    if let Some((sequence, authorization)) = &entry.authorization_record {
        text.push_str(&format!("authorization-sequence = {sequence}\n"));
        text.push_str(&format!(
            "authorization-record = {}\n",
            embedded(authorization)
        ));
    }
    text.push_str(&format!(
        "history-line = {}\n",
        toml_string(&entry.history_line)
    ));
    text.into_bytes()
}

/// Publish one committed entry whole in one step, creating the journal
/// directory when absent.
pub(crate) fn publish_entry(run_dir: &Path, entry: &JournalEntry) -> Result<PathBuf, String> {
    let dir = journal_dir(run_dir);
    fs::create_dir_all(&dir)
        .map_err(|error| format!("create the operation journal {}: {error}", dir.displayed()))?;
    crate::observe::operation("record");
    let path = dir.join(entry_name(&entry.action, entry.occurrence));
    crate::blocked::replace_file_atomically(&path, &render_journal_entry(entry))
        .map(|outcome| match outcome {
            crate::blocked::ReplaceFileOutcome::Durable => (),
            crate::blocked::ReplaceFileOutcome::ReplacedWithParentSyncWarning(error) => {
                crate::diag::diagnostic(format_args!(
                    "warning: published the journal entry {} but could not sync its parent \
                     directory: {error}",
                    path.displayed()
                ));
            }
        })
        .map_err(|error| format!("publish the journal entry {}: {error}", path.displayed()))?;
    Ok(path)
}

/// The pending committed entry's path, when the journal holds one.
pub(crate) fn pending_entry_path(run_dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(journal_dir(run_dir)).ok()?;
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|entry| crate::root::component(entry.file_name()))
        .filter(|name| name.ends_with(".committed"))
        .collect();
    names.sort();
    names.first().map(|name| journal_dir(run_dir).join(name))
}

/// Whether the Run is pending reconciliation: readers report this, never
/// repair it. Presence alone decides; readers never parse an entry.
pub(crate) fn has_pending_entry(run_dir: &Path) -> bool {
    pending_entry_path(run_dir).is_some()
}

/// The lock-free planning form of the pending check: a journal directory
/// that will not read counts as pending, so the locked reconcile reports
/// the failure honestly; an absent directory answers not pending. Used to
/// gate a plan-time lock, so an ordinary Run takes no extra lock at all
/// - exactly today's acquisition sequence.
pub(crate) fn planning_pending_entry(run_dir: &Path) -> bool {
    match fs::read_dir(journal_dir(run_dir)) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .any(|entry| crate::root::component(entry.file_name()).ends_with(".committed")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => true,
    }
}

/// Parse one committed entry strictly. The refusal line names the entry,
/// states it does not parse, and that nothing was modified.
fn parse_entry(path: &Path) -> Result<JournalEntry, String> {
    let unparseable = || {
        format!(
            "the pause journal entry {} does not parse; nothing was modified",
            path.displayed()
        )
    };
    let text = fs::read_to_string(path).map_err(|_error| unparseable())?;
    let value: toml::Value = text.parse().map_err(|_error| unparseable())?;
    let integer = |name: &str| -> Result<i64, String> {
        value
            .get(name)
            .and_then(toml::Value::as_integer)
            .ok_or_else(&unparseable)
    };
    let text_field = |name: &str| -> Result<String, String> {
        value
            .get(name)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(&unparseable)
    };
    let bytes_field =
        |name: &str| -> Result<Vec<u8>, String> { text_field(name).map(String::into_bytes) };
    if integer("version")? != 1 {
        return Err(unparseable());
    }
    let occurrence = integer("occurrence")?;
    if occurrence <= 0 {
        return Err(unparseable());
    }
    let authorization_sequence = match value.get("authorization-sequence") {
        None => None,
        Some(_) => Some(integer("authorization-sequence")?),
    };
    let authorization_record = match value.get("authorization-record") {
        None => None,
        Some(_) => Some(bytes_field("authorization-record")?),
    };
    if authorization_sequence.is_some() != authorization_record.is_some() {
        return Err(unparseable());
    }
    let authorization_record = authorization_record
        .map(|bytes| (authorization_sequence.expect("checked above") as u64, bytes));
    let action = text_field("action")?;
    if action != HOLD_ACTION && action != RESUME_ACTION {
        return Err(unparseable());
    }
    let entry = JournalEntry {
        action,
        run: text_field("run")?,
        occurrence: occurrence as u64,
        blocker: text_field("blocker")?,
        state: text_field("state")?,
        to_state: text_field("to-state")?,
        old_record: bytes_field("old-record")?,
        new_record: bytes_field("new-record")?,
        pause_record: match value.get("pause-record") {
            None => None,
            Some(_) => Some(bytes_field("pause-record")?),
        },
        authorization_record,
        history_line: text_field("history-line")?,
    };
    if entry.action == HOLD_ACTION && entry.pause_record.is_none() {
        return Err(unparseable());
    }
    Ok(entry)
}

/// Why a reconciliation could not complete.
pub(crate) struct ReconcileFailure {
    /// The refusal's detail.
    pub detail: String,
    /// Whether a committed action is waiting: a finishing failure renders
    /// the committed-with-recovery-pending line, an unparseable entry the
    /// nothing-was-modified line.
    pub committed: bool,
}

/// Reconcile the addressed Run under its held Run lock: finish a pending
/// committed entry's outputs exactly once, then remove it. `Ok(None)`
/// answers when no entry is pending. The finishing path belongs to the
/// command that published the entry, so reconciliation never fires the
/// hold seam's test barriers.
pub(crate) fn reconcile(run_dir: &Path) -> Result<Option<JournalEntry>, ReconcileFailure> {
    let Some(path) = pending_entry_path(run_dir) else {
        return Ok(None);
    };
    // WRS-003: recovery trusts the pauses directory only when every member
    // is canonical: a noncanonical member makes every later hold, resume,
    // or recovery refuse before the committed action is finished, writing
    // nothing beyond the entry it left alone.
    canonical_occurrences(run_dir).map_err(|detail| ReconcileFailure {
        detail,
        committed: false,
    })?;
    let entry = parse_entry(&path).map_err(|detail| ReconcileFailure {
        detail,
        committed: false,
    })?;
    finish(run_dir, &entry, false).map_err(|detail| ReconcileFailure {
        detail,
        committed: true,
    })?;
    Ok(Some(entry))
}

/// Finish a published entry's outputs idempotently and in the contract's
/// order: the authorization archive (governed only), the pause record
/// (hold), the Run Record replace, the history append - then the entry's
/// removal. An output whose target already equals the prepared bytes, or
/// a history line already sitting at the log's end, is skipped, never
/// rewritten. `own_boundaries` fires the hold seam's in-finish test
/// barriers for the invocation that published the entry; a reconciliation
/// passes `false`.
pub(crate) fn finish(
    run_dir: &Path,
    entry: &JournalEntry,
    own_boundaries: bool,
) -> Result<(), String> {
    let engine_root = run_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            format!(
                "the run directory {} has no Engine root",
                run_dir.displayed()
            )
        })?;
    if let Some((sequence, bytes)) = &entry.authorization_record {
        if own_boundaries {
            crate::blocked::hold_barrier_at("before-authorization-archive")?;
        }
        write_authorization_record(run_dir, *sequence, bytes)?;
    }
    if let Some(pause_record) = &entry.pause_record {
        fs::create_dir_all(pauses_dir(run_dir)).map_err(|error| {
            format!(
                "create the pause records {}: {error}",
                pauses_dir(run_dir).displayed()
            )
        })?;
        let path = pauses_dir(run_dir).join(format!("{}.toml", entry.occurrence));
        write_if_changed(&path, pause_record)?;
    }
    write_if_changed(&run_dir.join("run.toml"), &entry.new_record)?;
    if own_boundaries {
        crate::blocked::hold_barrier_at("before-history-append")?;
    }
    let log_path =
        crate::Scheduler::transition_log_path(&engine_root).map_err(|error| error.to_string())?;
    let existing =
        fs::read(&log_path).map_err(|error| format!("read {}: {error}", log_path.displayed()))?;
    // History is append-only and this action's append is its last output
    // under the lock, so the prepared line sitting at the log's end is the
    // action already finished; a line merely present earlier in the log is
    // some earlier motion's and never skips this append.
    if !existing.ends_with(entry.history_line.as_bytes()) {
        let mut log = crate::Scheduler::open_transition_log(&engine_root, false)
            .map_err(|error| error.to_string())?;
        match crate::Scheduler::append_transition_log(&mut log, entry.history_line.as_bytes()) {
            crate::scheduler::TransitionLogAppend::Complete => {}
            crate::scheduler::TransitionLogAppend::Failed(failure) => {
                return Err(failure.into_operator_error().to_string());
            }
        }
    }
    let entry_path = journal_dir(run_dir).join(entry_name(&entry.action, entry.occurrence));
    fs::remove_file(&entry_path)
        .map_err(|error| format!("remove {}: {error}", entry_path.displayed()))?;
    Ok(())
}

/// Write one authorization record at its sequence, skipping a target that
/// already equals the prepared bytes.
fn write_authorization_record(run_dir: &Path, sequence: u64, bytes: &[u8]) -> Result<(), String> {
    let dir = run_dir.join("authorizations");
    fs::create_dir_all(&dir).map_err(|error| {
        format!(
            "create the authorization archive {}: {error}",
            dir.displayed()
        )
    })?;
    let path = dir.join(format!("{sequence:06}.toml"));
    write_if_changed(&path, bytes)
}

/// Replace `path` with `bytes` unless it already equals them. The
/// replacement is atomic; its temporary names never contain `.tmp`.
fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if fs::read(path).is_ok_and(|existing| existing == bytes) {
        crate::observe::operation("record");
        return Ok(());
    }
    crate::observe::operation("record");
    crate::blocked::replace_file_atomically(path, bytes)
        .map_err(|error| format!("write {}: {error}", path.displayed()))?;
    Ok(())
}

/// A TOML basic string for one value, escaping exactly the characters the
/// grammar reserves.
fn toml_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if (control as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}
