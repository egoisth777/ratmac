//! WRS-004 (t-132): rule fix recovery.
//!
//! A rule fix is an *amendment*: an explicit, independently signed decision
//! that moves a Run's authority head forward over corrected rule bytes while
//! the Run keeps its address, State, class, bindings, workspace, and
//! original pins. The Engine owns an append-only amendment chain under
//! `<Engine root>/runs/<id>/amendments/`; every reader validates the whole
//! chain and reports, never repairs. `rtm recover` is the one verb that
//! appends to it, and a repeated `rtm recover` is the one way an interrupted
//! publication is reconciled - exactly once.
//!
//! The durable order of a committed recovery:
//!
//! 1. write the complete amendment record to `amendments/<n>.prepare`;
//! 2. rename it to `amendments/<n>.toml` - the commit point of the new head;
//! 3. append the authorization archive record;
//! 4. append the one history event to `log.md`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::root::{Displayed, Roots};
use crate::state::StateError;

/// The directory an Engine root owns for one Run's amendment chain.
pub const AMENDMENTS_DIR: &str = "amendments";

/// The signature namespace one recovery decision signs under.
pub const RECOVER_NAMESPACE: &str = "ratmac-recover-v1";

/// The confirmation phrase a human types for the Run `run_id`.
pub fn confirmation_phrase(run_id: &str) -> String {
    format!("recover {run_id}")
}

/// The exact history event one committed recovery records.
pub(crate) fn event_line(run_id: &str, sequence: u64) -> String {
    format!(
        "- Recover: run {run_id} adopted amendment {sequence} on an explicit signed recovery \
         decision."
    )
}

/// The authority fields an amendment enumerates, in canonical order.
const AUTHORITY_FIELDS: [&str; 6] = [
    "engine",
    "enrollment",
    "gate-artifact",
    "goal",
    "policy",
    "runbook",
];

/// The exact key set of one strict version-1 amendment member.
const RECORD_KEYS: [&str; 11] = [
    "affected-proof-boundary",
    "changes",
    "context",
    "new-authorities",
    "old-authorities",
    "previous-amendment-digest",
    "run",
    "sequence",
    "signature-identities",
    "signed-intent",
    "version",
];

const CONTEXT_KEYS: [&str; 4] = ["bindings", "class", "state", "workspace"];

// --- the recovery request -------------------------------------------------------

/// What a human asked for. Mirrors `blocked::HoldRequest`'s shape.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecoverRequest {
    /// The externally enrolled authority manifest, when the Run carries no
    /// previously enrolled recovery trust of its own.
    pub enrollment: Option<PathBuf>,
    /// The signed recovery decision (`ratmac-recover-v1`).
    pub decision: Option<PathBuf>,
    pub confirmation: Option<String>,
    /// The addressed run (FDC-004: `--run <id>`, always required).
    pub run: Option<String>,
}

/// One recovery refusal. A committed refusal reports the durable state the
/// interruption left behind, never a false unchanged refusal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverFailure {
    detail: String,
    committed: bool,
    hard: bool,
}

impl RecoverFailure {
    fn refusal(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
            committed: false,
            hard: false,
        }
    }

    fn now_committed(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
            committed: true,
            hard: false,
        }
    }

    /// A hard operational failure that keeps today's `<verb>: ...` family:
    /// lock acquisition and lock-wait failures refuse the hard way.
    fn hard(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
            committed: false,
            hard: true,
        }
    }

    /// Whether this failure renders through the hard-error family.
    pub fn hard_failure(&self) -> bool {
        self.hard
    }

    /// The one refusal line, without the command's own prefix.
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// Whether the failure left a committed amendment behind.
    pub fn committed(&self) -> bool {
        self.committed
    }
}

/// One completed (or reconciled) recovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Recovered {
    run_id: String,
    sequence: u64,
}

impl Recovered {
    /// The amended Run.
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// The amendment sequence that became effective.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
}

// --- framing and digests --------------------------------------------------------

fn frame(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    out.extend_from_slice(bytes);
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn is_digest(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Lowercase hexadecimal: the one spelling `signed-intent` stores the exact
/// canonical authorized bytes in, because the framed bytes need not be UTF-8.
fn hex_encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .concat()
}

/// Strict lowercase-hex decode: even length, ASCII lowercase digits only.
fn hex_decode(text: &str) -> Option<Vec<u8>> {
    if text.is_empty() || !text.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(text.len() / 2);
    let digits = text.as_bytes();
    for pair in digits.chunks(2) {
        let high = (pair[0] as char).to_digit(16)?;
        let low = (pair[1] as char).to_digit(16)?;
        if pair[0].is_ascii_uppercase() || pair[1].is_ascii_uppercase() {
            return None;
        }
        bytes.push((high * 16 + low) as u8);
    }
    Some(bytes)
}

/// Split the canonical intent bytes into their length-framed fields: the
/// `ratmac-authorization-v1` prefix, one NUL byte, then u64 big-endian
/// length-framed field bytes in `IntentFacts::canonical_bytes` order.
fn parse_intent_frames(bytes: &[u8]) -> Option<Vec<Vec<u8>>> {
    let mut rest = bytes
        .strip_prefix(crate::review::INTENT_PREFIX)?
        .strip_prefix(b"\0")?;
    let mut fields = Vec::new();
    while !rest.is_empty() {
        if rest.len() < 8 {
            return None;
        }
        let length = u64::from_be_bytes(rest[..8].try_into().ok()?) as usize;
        rest = &rest[8..];
        if rest.len() < length {
            return None;
        }
        fields.push(rest[..length].to_vec());
        rest = &rest[length..];
    }
    Some(fields)
}

/// The digest of the Run's original pin record, exactly as the recorded
/// evidence text frames it: `ratmac-pin-record-v1`, one NUL, then the
/// framed engine, runbook, goal baseline, and goal frozen identities, each
/// `absent` when the evidence records none.
pub(crate) fn pin_record_digest(evidence_text: &str) -> String {
    let field = |table: &str, name: &str| -> String {
        evidence_text
            .parse::<toml::Value>()
            .ok()
            .and_then(|value| {
                value
                    .get(table)
                    .and_then(|entry| entry.get(name))
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned)
            })
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "absent".to_owned())
    };
    let engine = field("engine", "sha256");
    let runbook = field("runbook", "sha256");
    let baseline = field("goal", "baseline");
    let frozen = field("goal", "frozen");
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"ratmac-pin-record-v1");
    bytes.push(0);
    frame(&mut bytes, engine.as_bytes());
    frame(&mut bytes, runbook.as_bytes());
    frame(&mut bytes, baseline.as_bytes());
    frame(&mut bytes, frozen.as_bytes());
    sha256_hex(&bytes)
}

/// One authority table: every field's identity string, `absent` when the
/// authority records none.
pub(crate) type Authorities = BTreeMap<String, String>;

fn gate_artifact_identity(pairs: &[(String, String)]) -> String {
    if pairs.is_empty() {
        return "absent".to_owned();
    }
    let mut sorted: Vec<&(String, String)> = pairs.iter().collect();
    sorted.sort_by(|left, right| left.0.cmp(&right.0));
    sorted
        .iter()
        .map(|(program, digest)| format!("{program}={digest}"))
        .collect::<Vec<_>>()
        .join(";")
}

/// The digest order the ticket fixes: `engine`, `runbook`, `goal`,
/// `gate-artifact`, `policy`, `enrollment`. `AUTHORITY_FIELDS` keeps the
/// alphabetical key order tables parse under; only the digest frames this
/// order.
const AUTHORITY_DIGEST_ORDER: [&str; 6] = [
    "engine",
    "runbook",
    "goal",
    "gate-artifact",
    "policy",
    "enrollment",
];

/// The digest of one authority table: `ratmac-authority-v1`, one NUL,
/// then every field framed in the ticket's fixed order. The gate-artifact
/// identity frames as its decimal pair count then each `program=sha256`
/// pair framed in byte order of program name - the landed key order, not
/// whole-pair text order, which differs when one program name prefixes
/// another - so the table's `;`-joined spelling is split apart and sorted
/// again here by the program part before the first `=`; a historically
/// absent identity - gate-artifact included - frames as the literal
/// `absent`.
pub(crate) fn authority_digest(authorities: &Authorities) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"ratmac-authority-v1");
    bytes.push(0);
    for field in AUTHORITY_DIGEST_ORDER {
        let value = authorities
            .get(field)
            .map(String::as_str)
            .unwrap_or("absent");
        if field != "gate-artifact" || value == "absent" {
            frame(&mut bytes, value.as_bytes());
            continue;
        }
        if value.is_empty() {
            frame(&mut bytes, b"0");
            continue;
        }
        let mut pairs: Vec<&str> = value.split(';').collect();
        pairs.sort_by(|left, right| {
            left.split('=')
                .next()
                .unwrap_or(left)
                .cmp(right.split('=').next().unwrap_or(right))
        });
        frame(&mut bytes, pairs.len().to_string().as_bytes());
        for pair in pairs {
            frame(&mut bytes, pair.as_bytes());
        }
    }
    sha256_hex(&bytes)
}

/// The digest of one enumerated change set: `ratmac-change-set-v1`, one
/// NUL, the count and framed sorted field names, then the framed old- and
/// new-authority digests.
pub(crate) fn change_set_digest(changes: &[String], old: &str, new: &str) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"ratmac-change-set-v1");
    bytes.push(0);
    frame(&mut bytes, changes.len().to_string().as_bytes());
    for change in changes {
        frame(&mut bytes, change.as_bytes());
    }
    frame(&mut bytes, old.as_bytes());
    frame(&mut bytes, new.as_bytes());
    sha256_hex(&bytes)
}

// --- the strict chain reader ----------------------------------------------------

/// One validated chain member's facts.
#[derive(Clone, Debug)]
pub(crate) struct ChainMember {
    pub sequence: u64,
    pub file_sha256: String,
    pub previous_digest: String,
    pub authorities: Authorities,
    pub old_authorities: Authorities,
    pub changes: Vec<String>,
    pub signed_intent: String,
    pub state: String,
    pub class: String,
    pub workspace: String,
    pub bindings: Vec<String>,
}

/// The validated effective head of one Run's amendment chain.
#[derive(Clone, Debug)]
pub(crate) struct ChainHead {
    pub sequence: u64,
    pub file_sha256: String,
    pub authorities: Authorities,
}

fn amendments_dir_of(run_dir: &Path) -> PathBuf {
    run_dir.join(AMENDMENTS_DIR)
}

fn member_path(run_dir: &Path, sequence: u64) -> PathBuf {
    amendments_dir_of(run_dir).join(format!("{sequence}.toml"))
}

fn run_id_of(run_dir: &Path) -> String {
    run_dir
        .file_name()
        .map(crate::root::component)
        .unwrap_or_default()
}

fn strict_detail(path: &Path, detail: impl std::fmt::Display) -> StateError {
    StateError::new(format!(
        "amendment record {} is not a strict version-1 record: {detail}",
        path.displayed()
    ))
}

/// Whether one member filename spells its sequence canonically: a positive
/// decimal integer with no leading zeroes.
fn canonical_sequence_name(name: &str) -> Option<u64> {
    let stem = name.strip_suffix(".toml")?;
    if stem.is_empty()
        || !stem.chars().all(|character| character.is_ascii_digit())
        || stem.starts_with('0')
    {
        return None;
    }
    stem.parse().ok()
}

/// Parse one amendment member's bytes strictly, proving it carries exactly
/// the version-1 grammar and that its own tables hash to the digests its
/// signed intent binds.
fn parse_member(
    run_dir: &Path,
    run_id: &str,
    sequence: u64,
    bytes: &[u8],
) -> Result<ChainMember, StateError> {
    let path = member_path(run_dir, sequence);
    let text = std::str::from_utf8(bytes)
        .map_err(|error| strict_detail(&path, format!("the record is not UTF-8: {error}")))?;
    let document: toml::Value = text
        .parse::<toml::Value>()
        .map_err(|error: toml::de::Error| {
            strict_detail(&path, error.to_string().lines().next().unwrap_or_default())
        })?;
    let root = document
        .as_table()
        .ok_or_else(|| strict_detail(&path, "the record must be a table"))?;
    for key in RECORD_KEYS {
        if !root.contains_key(key) {
            return Err(strict_detail(
                &path,
                format!("the record is missing {key:?}"),
            ));
        }
    }
    for key in root.keys() {
        if !RECORD_KEYS.contains(&key.as_str()) {
            return Err(strict_detail(
                &path,
                format!("the record carries unknown key {key:?}"),
            ));
        }
    }
    if root.get("version").and_then(toml::Value::as_integer) != Some(1) {
        return Err(strict_detail(&path, "version must be the integer 1"));
    }
    if root.get("sequence").and_then(toml::Value::as_integer) != Some(sequence as i64) {
        return Err(strict_detail(
            &path,
            format!("sequence must be the integer {sequence}"),
        ));
    }
    if root.get("run").and_then(toml::Value::as_str) != Some(run_id) {
        return Err(StateError::new(format!(
            "amendment record {} names run {}; this chain belongs to run {run_id}",
            path.displayed(),
            root.get("run")
                .and_then(toml::Value::as_str)
                .unwrap_or_default()
        )));
    }
    let previous = root
        .get("previous-amendment-digest")
        .and_then(toml::Value::as_str)
        .unwrap_or_default();
    if !is_digest(previous) {
        return Err(strict_detail(
            &path,
            "previous-amendment-digest must be 64 lowercase hex digits",
        ));
    }
    let changes: Vec<String> = match root.get("changes").and_then(toml::Value::as_array) {
        None => Vec::new(),
        Some(entries) => {
            let mut parsed = Vec::new();
            for entry in entries {
                let Some(name) = entry.as_str() else {
                    return Err(strict_detail(
                        &path,
                        "changes must list authority field names as strings",
                    ));
                };
                parsed.push(name.to_owned());
            }
            parsed
        }
    };
    if changes
        .iter()
        .any(|change| !AUTHORITY_FIELDS.contains(&change.as_str()))
        || changes.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(strict_detail(
            &path,
            "changes must list distinct authority fields in sorted order",
        ));
    }
    let _ = root
        .get("affected-proof-boundary")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| strict_detail(&path, "affected-proof-boundary must be a string"))?;
    let signed_intent = root
        .get("signed-intent")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| strict_detail(&path, "signed-intent must be a string"))?
        .to_owned();
    if hex_decode(&signed_intent).is_none() {
        return Err(strict_detail(
            &path,
            "signed-intent must be the exact canonical bytes spelled as lowercase \
             hexadecimal",
        ));
    }
    let context = root
        .get("context")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| strict_detail(&path, "context must be a table"))?;
    if context.len() != CONTEXT_KEYS.len()
        || context
            .keys()
            .any(|key| !CONTEXT_KEYS.contains(&key.as_str()))
    {
        return Err(strict_detail(
            &path,
            "context must hold exactly state, class, bindings, and workspace",
        ));
    }
    for key in ["state", "class", "workspace"] {
        if context.get(key).and_then(toml::Value::as_str).is_none() {
            return Err(strict_detail(
                &path,
                format!("context.{key} must be a string"),
            ));
        }
    }
    let bindings = context
        .get("bindings")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| strict_detail(&path, "context.bindings must be an array of strings"))?;
    if bindings.iter().any(|binding| binding.as_str().is_none()) {
        return Err(strict_detail(
            &path,
            "context.bindings must be an array of strings",
        ));
    }
    let member_bindings: Vec<String> = bindings
        .iter()
        .map(|binding| binding.as_str().unwrap_or_default().to_owned())
        .collect();
    let member_class = context
        .get("class")
        .and_then(toml::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let member_workspace = context
        .get("workspace")
        .and_then(toml::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let parse_authorities = |name: &str| -> Result<Authorities, StateError> {
        let table = root
            .get(name)
            .and_then(toml::Value::as_table)
            .ok_or_else(|| strict_detail(&path, format!("{name} must be a table")))?;
        if table.len() != AUTHORITY_FIELDS.len()
            || table
                .keys()
                .any(|key| !AUTHORITY_FIELDS.contains(&key.as_str()))
        {
            return Err(strict_detail(
                &path,
                format!("{name} must hold exactly the six authority fields"),
            ));
        }
        let mut authorities = Authorities::new();
        for field in AUTHORITY_FIELDS {
            let value = table
                .get(field)
                .and_then(toml::Value::as_str)
                .ok_or_else(|| strict_detail(&path, format!("{name}.{field} must be a string")))?;
            let well_formed = value == "absent"
                || is_digest(value)
                // The goal identity is the current goal revision, whose
                // engine-canonical spelling is `v2:<sha>` while the
                // recorded pin is frozen at v2 (src/goal.rs).
                || (field == "goal"
                    && value
                        .strip_prefix("v2:")
                        .is_some_and(is_digest))
                || (field == "gate-artifact"
                    && value.split(';').all(|pair| {
                        pair.split_once('=')
                            .is_some_and(|(_, digest)| is_digest(digest))
                    }));
            if !well_formed {
                return Err(strict_detail(
                    &path,
                    format!("{name}.{field} is no authority identity"),
                ));
            }
            authorities.insert(field.to_owned(), value.to_owned());
        }
        Ok(authorities)
    };
    let old = parse_authorities("old-authorities")?;
    let new = parse_authorities("new-authorities")?;
    let signers = root
        .get("signature-identities")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| strict_detail(&path, "signature-identities must be an array of tables"))?;
    if signers.is_empty() {
        return Err(strict_detail(
            &path,
            "signature-identities must carry at least one signer row",
        ));
    }
    for row in signers {
        let row = row
            .as_table()
            .ok_or_else(|| strict_detail(&path, "a signature-identities row must be a table"))?;
        if row.len() != 3
            || !row.contains_key("label")
            || !row.contains_key("key")
            || !row.contains_key("signature-sha256")
        {
            return Err(strict_detail(
                &path,
                "a signature-identities row must carry exactly label, key, and \
                 signature-sha256",
            ));
        }
        if row.get("label").and_then(toml::Value::as_str).is_none() {
            return Err(strict_detail(
                &path,
                "a signature-identities label must be a string",
            ));
        }
        if row
            .get("key")
            .and_then(toml::Value::as_str)
            .unwrap_or_default()
            .is_empty()
        {
            return Err(strict_detail(
                &path,
                "a signer row's key must be the enrolled key",
            ));
        }
        let digest = row
            .get("signature-sha256")
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        if !is_digest(digest) {
            return Err(strict_detail(
                &path,
                "a signature-sha256 must be 64 lowercase hex digits",
            ));
        }
    }
    // The enumerated change set must be exactly the recorded difference.
    for field in AUTHORITY_FIELDS {
        let old_value = old.get(field).map(String::as_str).unwrap_or("absent");
        let new_value = new.get(field).map(String::as_str).unwrap_or("absent");
        if old_value != new_value && !changes.iter().any(|change| change == field) {
            return Err(StateError::new(format!(
                "amendment {sequence} changes {field} beyond its enumerated change set"
            )));
        }
        if old_value == new_value && changes.iter().any(|change| change == field) {
            return Err(StateError::new(format!(
                "amendment {sequence} enumerates a change to {field} whose old and new \
                 identities agree"
            )));
        }
    }
    let state = context
        .get("state")
        .and_then(toml::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    Ok(ChainMember {
        sequence,
        file_sha256: sha256_hex(bytes),
        previous_digest: previous.to_owned(),
        authorities: new,
        old_authorities: old,
        changes,
        signed_intent,
        state,
        class: member_class,
        workspace: member_workspace,
        bindings: member_bindings,
    })
}

/// The signed intent must bind its record's own derivation: the canonical
/// prefix and purpose, the Run, the kept State, the recorded predecessor,
/// and the recomputed authority and change-set digests. Runs in
/// `validate_chain` after the predecessor comparison, so a record whose
/// predecessor is wrong refuses naming that, never a downstream field.
fn verify_member_intent(run_id: &str, member: &ChainMember) -> Result<(), StateError> {
    let disagree = |what: &str| {
        StateError::new(format!(
            "amendment {} binds a signed intent that disagrees with its record \
             fields: it does not carry {what}",
            member.sequence
        ))
    };
    let Some(bytes) = hex_decode(&member.signed_intent) else {
        return Err(disagree("the canonical intent prefix"));
    };
    let Some(fields) = parse_intent_frames(&bytes) else {
        return Err(disagree("the canonical intent prefix"));
    };
    let field = |index: usize| -> Option<&[u8]> { fields.get(index).map(Vec::as_slice) };
    let old_digest = authority_digest(&member.old_authorities);
    let new_digest = authority_digest(&member.authorities);
    let recomputed = change_set_digest(&member.changes, &old_digest, &new_digest);
    let binding_digest = member_binding_digest(member);
    // The frame order is `IntentFacts::canonical_bytes`: purpose, project
    // scope, run, class, workspace, bindings, then the motion's own facts.
    // Every field the record itself names is compared exactly, byte for
    // byte, against the record's own spelling - never by substring.
    let expected: [(usize, Vec<u8>, &str); 10] = [
        (0, b"recover".to_vec(), "the recover purpose"),
        (2, run_id.as_bytes().to_vec(), "the amended run"),
        (3, member.class.as_bytes().to_vec(), "the owning class"),
        (
            4,
            member.workspace.as_bytes().to_vec(),
            "the recorded workspace",
        ),
        (5, binding_digest.into_bytes(), "the recorded bindings"),
        (9, member.state.as_bytes().to_vec(), "the kept State"),
        (10, member.state.as_bytes().to_vec(), "the kept State"),
        (
            20,
            member.previous_digest.as_bytes().to_vec(),
            "the recorded previous-amendment-digest",
        ),
        (
            21,
            new_digest.into_bytes(),
            "the recomputed new-authority digest",
        ),
        (
            22,
            recomputed.into_bytes(),
            "the recomputed change-set digest",
        ),
    ];
    for (index, value, what) in expected {
        if field(index) != Some(value.as_slice()) {
            return Err(disagree(what));
        }
    }
    Ok(())
}

/// The binding digest a member's own context table hashes to: sorted
/// `name=value` pairs framed once, exactly as the canonical intent frames
/// them. An empty binding list frames the empty input.
fn member_binding_digest(member: &ChainMember) -> String {
    if member.bindings.is_empty() {
        // The render writes `bindings = []` for a Run with no bindings,
        // whose canonical intent frames the empty string, not the digest
        // of empty bytes.
        return String::new();
    }
    let mut joined = member.bindings.clone();
    joined.sort();
    let mut body = Vec::new();
    for pair in joined {
        frame(&mut body, pair.as_bytes());
    }
    sha256_hex(&body)
}

/// Validate one Run's complete amendment chain and answer its effective
/// head. `None` while no member exists. A preparation file is ignored,
/// never a member.
pub(crate) fn validate_chain(
    run_dir: &Path,
    run_id: &str,
) -> Result<Option<ChainHead>, StateError> {
    let dir = amendments_dir_of(run_dir);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(StateError::new(format!(
                "read the amendment chain {}: {error}",
                dir.displayed()
            )))
        }
    };
    let mut members: Vec<(u64, String)> = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|error| StateError::new(format!("read an amendment entry: {error}")))?;
        let name = crate::root::component(entry.file_name());
        // A preparation file is never a member: readers ignore it
        // completely, exactly as the pause journal's pending entry is
        // ignored until the one command that may finish it runs.
        if name.ends_with(".prepare") {
            continue;
        }
        match canonical_sequence_name(&name) {
            Some(sequence) => members.push((sequence, name)),
            None => {
                return Err(StateError::new(format!(
                    "amendment sequence {} is not a positive decimal integer without \
                     leading zeroes",
                    name.strip_suffix(".toml").unwrap_or(&name)
                )))
            }
        }
    }
    if members.is_empty() {
        return Ok(None);
    }
    members.sort_by_key(|left| left.0);
    for (index, (sequence, _)) in members.iter().enumerate() {
        if *sequence != index as u64 + 1 {
            return Err(StateError::new(format!(
                "the amendment chain for run {run_id} is not contiguous: expected sequence \
                 {}, found {sequence}",
                index as u64 + 1
            )));
        }
    }
    crate::observe::operation("record");
    let evidence_text =
        std::fs::read_to_string(crate::pin::evidence_path(run_dir)).unwrap_or_default();
    let original = pin_record_digest(&evidence_text);
    let mut head: Option<ChainMember> = None;
    for (sequence, _) in &members {
        let path = member_path(run_dir, *sequence);
        let bytes = std::fs::read(&path).map_err(|error| {
            StateError::new(format!(
                "read the amendment record {}: {error}",
                path.displayed()
            ))
        })?;
        crate::observe::operation("record");
        let member = parse_member(run_dir, run_id, *sequence, &bytes)?;
        let (named, actual) = match &head {
            None => ("the original pin record".to_owned(), original.clone()),
            Some(previous) => (
                format!("sequence {}", previous.sequence),
                previous.file_sha256.clone(),
            ),
        };
        if member.previous_digest != actual {
            return Err(StateError::new(format!(
                "amendment {sequence} records previous-amendment-digest {}, but {named} hashes \
                 {actual}",
                member.previous_digest
            )));
        }
        verify_member_intent(run_id, &member)?;
        head = Some(member);
    }
    Ok(head.map(|member| ChainHead {
        sequence: member.sequence,
        file_sha256: member.file_sha256.clone(),
        authorities: member.authorities,
    }))
}

/// The pending-amendment guard every mutating verb holds. A head member
/// committed with its history event missing refuses further mutation until
/// a repeated `rtm recover` reconciles it.
pub(crate) fn require_complete_history(engine_root: &Path, run_id: &str) -> Result<(), StateError> {
    let run_dir = engine_root.join("runs").join(run_id);
    let Some(head) = validate_chain(&run_dir, run_id)? else {
        return Ok(());
    };
    if !event_present(engine_root, run_id, head.sequence)? {
        return Err(StateError::new(format!(
            "amendment history is incomplete for run {run_id}: sequence {} is committed with \
             no history event; repeat rtm recover --run {run_id} to reconcile it",
            head.sequence
        )));
    }
    Ok(())
}

/// Whether one Run's head amendment is committed with its history event
/// missing, and that amendment's sequence - the report readers and doctor
/// carry, never a repair.
pub(crate) fn pending_amendment(engine_root: &Path, run_dir: &Path) -> Option<(String, u64)> {
    let run_id = run_id_of(run_dir);
    let head = validate_chain(run_dir, &run_id).ok()??;
    if event_present(engine_root, &run_id, head.sequence).ok()? {
        return None;
    }
    Some((run_id, head.sequence))
}

/// The amendment history event one sequence records, if the shared log
/// already carries it.
fn event_present(engine_root: &Path, run_id: &str, sequence: u64) -> Result<bool, StateError> {
    let text = std::fs::read_to_string(engine_root.join("log.md")).unwrap_or_default();
    Ok(text
        .lines()
        .any(|line| line == event_line(run_id, sequence)))
}

// --- the plan -------------------------------------------------------------------

/// The trust a recovery decision verifies under: the prior policy's facts,
/// read from the recorded head and marker while the Run is governed, or
/// from an independently enrolled external authority when it is not.
pub(crate) struct RecoveryTrust {
    pub manifest_sha256: String,
    pub policy_digest: String,
    pub approver: String,
    pub reviewers: Vec<(String, String)>,
    pub verifier_path: PathBuf,
    pub signers_path: PathBuf,
    pub origin: &'static str,
    pub work: Vec<String>,
}

/// The derived authority movement one recovery binds.
pub(crate) struct Derivation {
    pub old: Authorities,
    pub new: Authorities,
    pub changes: Vec<String>,
    pub new_digest: String,
    pub change_digest: String,
}

/// A verified recovery, ready to apply.
#[derive(Clone, Debug)]
pub struct RecoverPlan {
    run_id: String,
    engine_root: PathBuf,
    run_dir: PathBuf,
    state: String,
    head_before: String,
    sequence: u64,
    /// The reconciliation this invocation owes instead of a fresh mint.
    reconcile: bool,
    record_text: String,
    archive_text: String,
    archive_sequence: u64,
    /// The plan's proof of derivation, re-derived under apply's
    /// re-acquired lock: a drift here is a stale plan.
    governed: bool,
    prior_policy: String,
    manifest_sha256: String,
    change_digest: String,
}

impl RecoverPlan {
    /// The addressed Run.
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// The amendment sequence this plan commits or reconciles.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
}

/// The public path-taking library entries (WEB-005), re-exported from
/// `root::entries` like every verb's own module re-exports them.
pub use crate::root::entries::{apply_recover, plan_recover};

pub(crate) fn plan_recover_in(
    roots: &Roots,
    request: &RecoverRequest,
) -> Result<RecoverPlan, RecoverFailure> {
    check_pause_companions()?;
    crate::Scheduler::refuse_flat_residue_with_roots(roots)
        .map_err(|error| RecoverFailure::refusal(error.to_string()))?;

    let engine_root = roots.engine_root().to_path_buf();
    let roster = crate::Scheduler::run_roster_with_roots(roots)
        .map_err(|error| RecoverFailure::refusal(error.to_string()))?;
    let run_id = request
        .run
        .as_deref()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| {
            RecoverFailure::refusal(format!(
                "recover requires --run <id>; runs: {}{}",
                crate::roster::summary(&engine_root, &roster, "none"),
                crate::roster::rows(&engine_root, &roster)
            ))
        })?
        .to_owned();
    if !roster.iter().any(|member| member == &run_id) {
        return Err(RecoverFailure::refusal(format!(
            "run id {run_id:?} is not an exact roster member; runs: {}{}",
            crate::roster::summary(&engine_root, &roster, "none"),
            crate::roster::rows(&engine_root, &roster)
        )));
    }

    let required = confirmation_phrase(&run_id);
    match request.confirmation.as_deref().map(str::trim) {
        None => {
            return Err(RecoverFailure::refusal(format!(
                "recover is unconfirmed: a human must type --confirm {required:?}"
            )))
        }
        Some(phrase) if phrase != required => {
            return Err(RecoverFailure::refusal(format!(
                "recover is unconfirmed: confirmation {phrase:?} does not match the required \
                 phrase {required:?}"
            )))
        }
        Some(_) => {}
    }

    // The open-run family, minus the pin-mismatch refusal: a recovery is
    // the one verb whose business is a drifted runbook.
    let scheduler = crate::Scheduler::open_run_for_recover_with_roots(roots, &run_id)
        .map_err(|error| RecoverFailure::refusal(error.to_string()))?;
    let run_lock = crate::lock::RunLock::acquire(&engine_root, &run_id)
        .map_err(|error| RecoverFailure::hard(error.to_string()))?;
    run_lock
        .ensure_current()
        .map_err(|error| RecoverFailure::hard(error.to_string()))?;
    let run_dir = engine_root.join("runs").join(&run_id);
    if !run_dir.join("run.toml").is_file() {
        return Err(RecoverFailure::refusal(format!(
            "run {run_id} is terminal: its admission state is retired and no transition may \
             proceed on it; address a live run or mint a fresh one with rtm start"
        )));
    }
    let state = scheduler
        .load_state()
        .map_err(|error| RecoverFailure::refusal(error.to_string()))?;
    if state.status == crate::model::Status::Passed {
        return Err(RecoverFailure::refusal(format!(
            "run {run_id} is terminal (status passed): recovery may not amend a terminal Run"
        )));
    }
    let evidence_text =
        std::fs::read_to_string(crate::pin::evidence_path(&run_dir)).map_err(|error| {
            RecoverFailure::refusal(format!(
                "the recorded evidence for run {run_id} does not parse: {error}"
            ))
        })?;
    crate::pin::Evidence::parse(&evidence_text).map_err(|error| {
        RecoverFailure::refusal(format!(
            "the recorded evidence for run {run_id} does not parse: {error}"
        ))
    })?;
    crate::observe::operation("record");
    let evidence = crate::pin::Evidence::load(&run_dir);

    // The compatibility rule: the corrected runbook keeps the Run's class,
    // its State, and its workspace - in that order.
    let snapshot_sha256 = scheduler.runbook_pin_of().to_owned();
    let class = scheduler
        .runbook_class()
        .map_err(|error| RecoverFailure::refusal(error.to_string()))?;
    let child_class = scheduler.child_class().map(str::to_owned);
    let class_name = child_class.clone().unwrap_or_else(|| "runbook".to_owned());
    let states = match child_class.as_deref() {
        Some(child) => class.classes().get(child).map(|child| child.states()),
        None => Some(class.states()),
    };
    let Some(states) = states else {
        return Err(RecoverFailure::refusal(format!(
            "the corrected runbook declares no class {class_name}; recovery keeps the owning \
             class"
        )));
    };
    if states.get(state.state.as_str()).is_none() {
        return Err(RecoverFailure::refusal(format!(
            "the corrected runbook declares no State {} in class {class_name}; recovery \
             refuses a removed State",
            state.state
        )));
    }
    let workspace = scheduler
        .workspace_root()
        .map_err(|error| RecoverFailure::refusal(error.to_string()))?
        .to_path_buf();

    // The chain validates before anything derives from it.
    let head = validate_chain(&run_dir, &run_id)
        .map_err(|error| RecoverFailure::refusal(error.to_string()))?;

    // The recorded-workspace rule: the resolved workspace must equal the
    // recorded workspace - the effective head's `context.workspace` when an
    // amendment exists, otherwise a held Run's pause-record workspace,
    // otherwise a child's spawn-ledger workspace. The invoking checkout is
    // what a top-level Run resolves to, so no comparison can differ there.
    let resolved_workspace = std::fs::canonicalize(&workspace)
        .unwrap_or(workspace.clone())
        .displayed()
        .to_string();
    let recorded_workspace: Option<String> = if let Some(head) = &head {
        read_member(&run_dir, &run_id, head.sequence)
            .ok()
            .map(|member| member.workspace)
    } else if state.status == crate::model::Status::Blocked {
        crate::pause::load_live_record_lenient(&run_dir)
            .ok()
            .map(|(_, record)| record.workspace)
    } else {
        crate::Scheduler::recorded_child_workspace_at(&engine_root, &run_id)
            .map_err(|error| RecoverFailure::refusal(error.to_string()))?
            .map(|workspace| workspace.displayed().to_string())
    };
    if let Some(recorded) = recorded_workspace {
        if recorded != resolved_workspace {
            return Err(RecoverFailure::refusal(format!(
                "the recorded workspace {recorded} differs from the resolved workspace \
                 {resolved_workspace}; recovery keeps the workspace"
            )));
        }
    }

    // The reconciliation branch: a committed head with a missing history
    // event is finished by this very invocation, before any authorization
    // is examined.
    if let Some(head) = &head {
        if !event_present(&engine_root, &run_id, head.sequence)
            .map_err(|error| RecoverFailure::refusal(error.to_string()))?
        {
            let member = read_member(&run_dir, &run_id, head.sequence)
                .map_err(|error| RecoverFailure::refusal(error.to_string()))?;
            drop(run_lock);
            return reconcile_plan(&engine_root, &run_dir, &run_id, &member);
        }
    }

    // The eligibility boundary: governed Runs carry their prior trusted
    // policy from the head or the recorded marker; a legacy Run requires an
    // independently enrolled external authority.
    let governed = run_dir.join(crate::review::GOVERNANCE_FILE).is_file();
    let prior_policy = match &head {
        Some(head) => head
            .authorities
            .get("policy")
            .cloned()
            .unwrap_or_else(|| "absent".to_owned()),
        None => {
            if governed {
                crate::review::marker_policy(
                    &std::fs::read(run_dir.join(crate::review::GOVERNANCE_FILE))
                        .unwrap_or_default(),
                )
                .unwrap_or_default()
            } else {
                "absent".to_owned()
            }
        }
    };
    let trust = if governed {
        read_governed_trust(&engine_root, &run_dir, &run_id, &prior_policy, &scheduler)
            .map_err(RecoverFailure::refusal)?
    } else {
        let Some(path) = request.enrollment.as_deref() else {
            return Err(RecoverFailure::refusal(format!(
                "run {run_id} lacks previously enrolled recovery trust; an independently \
                 enrolled external authority must supply it (--enrollment <path>): the builder \
                 may neither generate it nor substitute task authorization for it"
            )));
        };
        read_external_trust(&engine_root, path).map_err(RecoverFailure::refusal)?
    };

    // The signer row names the approver's enrolled key from the reviewers,
    // which RB703 guarantees; a trust that somehow omits it refuses before
    // any preparation write - an amendment may never name an empty key.
    if signer_key_of(&trust).is_none() {
        return Err(RecoverFailure::refusal(format!(
            "the recovery trust names no enrolled key for the designated approver {:?}; \
             the builder may neither mint one nor proceed without it",
            trust.approver
        )));
    }

    // The derived authorities: old from the effective head (the original
    // pins while no chain exists), new from the current on-disk artifacts.
    let derivation = derive_authorities(
        &scheduler,
        &run_dir,
        &run_id,
        &state,
        &evidence,
        governed,
        &trust,
        head.as_ref(),
        &prior_policy,
        &snapshot_sha256,
    )
    .map_err(RecoverFailure::refusal)?;
    // The canonical recover intent and its verification. The decision
    // boundary answers before the derived movement: a Run with no decision
    // observes none even when nothing differs yet.
    let facts = scheduler
        .recover_facts(
            &engine_root,
            &run_id,
            &trust,
            &derivation,
            &state,
            head.as_ref(),
        )
        .map_err(RecoverFailure::refusal)?;
    let engine_identity =
        crate::review::engine_identity_sha256().map_err(RecoverFailure::refusal)?;
    let project_scope = crate::Scheduler::project_scope(&engine_root);
    let intent = facts.canonical_bytes(&project_scope, &engine_identity);
    let declared_materials: Vec<String> = scheduler
        .declared_review_of(&run_dir, &run_id)
        .map_err(|error| RecoverFailure::refusal(error.to_string()))?
        .map(|declaration| {
            declaration
                .reviewers()
                .iter()
                .map(|(_, material)| material.clone())
                .collect()
        })
        .unwrap_or_default();
    verify_decision(
        &run_dir,
        &run_id,
        &trust,
        &facts,
        &intent,
        request.decision.as_deref(),
        head.as_ref(),
        &declared_materials,
    )?;
    if derivation.changes.is_empty() {
        return Err(RecoverFailure::refusal(
            "authorization: changes: observed no differing authority; expected at least one",
        ));
    }

    let Some(sequence) = head
        .as_ref()
        .map(|head| head.sequence.checked_add(1))
        .unwrap_or(Some(1))
    else {
        return Err(RecoverFailure::refusal(format!(
            "the next amendment sequence for run {run_id} is unavailable; refusing rather \
             than wrapping"
        )));
    };
    let archive_sequence =
        crate::review::next_sequence(&run_dir).map_err(RecoverFailure::refusal)?;
    let decision_sha = request
        .decision
        .as_deref()
        .map(|path| sha256_hex(&std::fs::read(path).unwrap_or_default()))
        .unwrap_or_default();
    let record_text = render_record(
        sequence,
        &derivation,
        &intent,
        &facts,
        &trust,
        &decision_sha,
    );
    let archive_text = render_archive(
        archive_sequence,
        sequence,
        &facts,
        &trust,
        &intent,
        request.decision.as_deref(),
    );
    Ok(RecoverPlan {
        run_id,
        engine_root,
        run_dir,
        state: state.state.clone(),
        head_before: head
            .as_ref()
            .map(|head| head.file_sha256.clone())
            .unwrap_or_default(),
        sequence,
        reconcile: false,
        record_text,
        archive_text,
        archive_sequence,
        governed,
        prior_policy,
        manifest_sha256: trust.manifest_sha256.clone(),
        change_digest: derivation.change_digest.clone(),
    })
}

/// Apply a verified recovery plan through the durable order. The repeated
/// command of an interrupted publication reconciles exactly once here.
pub(crate) fn apply_recover_in(
    roots: &Roots,
    plan: &RecoverPlan,
) -> Result<Recovered, RecoverFailure> {
    let engine_root = plan.engine_root.clone();
    let run_dir = plan.run_dir.clone();
    let run_id = plan.run_id.clone();
    let run_lock = crate::lock::RunLock::acquire(&engine_root, &run_id)
        .map_err(|error| RecoverFailure::hard(error.to_string()))?;
    run_lock
        .ensure_current()
        .map_err(|error| RecoverFailure::hard(error.to_string()))?;

    // The stale re-check under the re-acquired lock: the plan's proof must
    // still describe this Run - its Record, its effective head, and the
    // authorities it derived. A drift on any of the three is a stale plan.
    let head = validate_chain(&run_dir, &run_id)
        .map_err(|error| RecoverFailure::refusal(error.to_string()))?;
    let observed_head = head
        .as_ref()
        .map(|head| head.file_sha256.clone())
        .unwrap_or_default();
    let state = crate::state::StateStore::for_engine_root(&engine_root, &run_id)
        .load()
        .map_err(|error| RecoverFailure::refusal(error.to_string()))?;
    if state.state != plan.state || observed_head != plan.head_before {
        return Err(RecoverFailure::refusal(format!(
            "recover plan is stale for run {run_id}: expected state {} at head {}, found state \
             {} at head {observed_head}; nothing was modified",
            plan.state, plan.head_before, state.state
        )));
    }
    if !plan.reconcile {
        let scheduler = crate::Scheduler::open_run_for_recover_with_roots(roots, &run_id)
            .map_err(|error| RecoverFailure::refusal(error.to_string()))?;
        let observed_state = scheduler
            .load_state()
            .map_err(|error| RecoverFailure::refusal(error.to_string()))?;
        let evidence = crate::pin::Evidence::load(&run_dir);
        let snapshot_sha256 = scheduler.runbook_pin_of().to_owned();
        let trust_mirror = RecoveryTrust {
            manifest_sha256: plan.manifest_sha256.clone(),
            policy_digest: plan.prior_policy.clone(),
            approver: String::new(),
            reviewers: Vec::new(),
            verifier_path: PathBuf::new(),
            signers_path: PathBuf::new(),
            origin: "plan",
            work: Vec::new(),
        };
        let derivation = derive_authorities(
            &scheduler,
            &run_dir,
            &run_id,
            &observed_state,
            &evidence,
            plan.governed,
            &trust_mirror,
            head.as_ref(),
            &plan.prior_policy,
            &snapshot_sha256,
        )
        .map_err(RecoverFailure::refusal)?;
        if derivation.change_digest != plan.change_digest {
            return Err(RecoverFailure::refusal(format!(
                "recover plan is stale for run {run_id}: the derived authority movement \
                 changed since the plan was verified; nothing was modified"
            )));
        }
    }

    // The reconciliation branch keeps the Run lock held: its archive and
    // history writes are as much one Run-locked decision as the mint's.
    if plan.reconcile {
        let _held = &run_lock;
        return reconcile_apply(&engine_root, &run_dir, &run_id, plan.sequence);
    }

    let amendments = amendments_dir_of(&run_dir);
    std::fs::create_dir_all(&amendments).map_err(|error| {
        RecoverFailure::refusal(format!(
            "cannot create the amendment chain {}: {error}",
            amendments.displayed()
        ))
    })?;
    let prepare = amendments.join(format!("{}.prepare", plan.sequence));
    write_whole_atomically(&prepare, plan.record_text.as_bytes()).map_err(|error| {
        RecoverFailure::refusal(format!(
            "cannot write the preparation file {}: {error}",
            prepare.displayed()
        ))
    })?;
    wait_at_recover_pause("before-rename")?;
    if inject_recover_fault("before-rename")? {
        return Err(RecoverFailure::refusal(
            "injected recover fault at before-rename",
        ));
    }
    let member = amendments.join(format!("{}.toml", plan.sequence));
    std::fs::rename(&prepare, &member).map_err(|error| {
        RecoverFailure::refusal(format!(
            "cannot publish the amendment {}: {error}",
            member.displayed()
        ))
    })?;
    wait_at_recover_pause("after-rename")?;
    if inject_recover_fault("after-rename")? {
        return Err(committed_pending(
            plan.sequence,
            "injected recover fault at after-rename",
        ));
    }
    let authorizations = run_dir.join("authorizations");
    std::fs::create_dir_all(&authorizations).map_err(|error| {
        committed_pending(
            plan.sequence,
            format!(
                "cannot create the authorization archive {}: {error}",
                authorizations.displayed()
            ),
        )
    })?;
    let archive = authorizations.join(format!("{:06}.toml", plan.archive_sequence));
    write_whole_atomically(&archive, plan.archive_text.as_bytes()).map_err(|error| {
        committed_pending(
            plan.sequence,
            format!(
                "cannot write the authorization record {}: {error}",
                archive.displayed()
            ),
        )
    })?;
    crate::observe::operation("record");
    if inject_recover_fault("history-append")? {
        return Err(committed_pending(
            plan.sequence,
            "injected recover fault at history-append",
        ));
    }
    append_event(&engine_root, &run_id, plan.sequence)
        .map_err(|failure| committed_pending(plan.sequence, failure.detail()))?;
    Ok(Recovered {
        run_id,
        sequence: plan.sequence,
    })
}

/// A failure after the amendment's rename: the head is committed and only
/// its history may still be pending, so the refusal family never claims the
/// amendment was refused.
fn committed_pending(sequence: u64, detail: impl std::fmt::Display) -> RecoverFailure {
    RecoverFailure::now_committed(format!(
        "committed amendment {sequence} with history pending: {detail}; repeat this command to reconcile it exactly once"
    ))
}

/// The landed atomic-replace discipline: the whole record lands in one
/// rename, so a torn write can never leave a half-record behind.
fn write_whole_atomically(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    crate::blocked::replace_file_atomically(path, bytes).map(|_| ())
}

// --- reconciliation -------------------------------------------------------------

/// Read one committed member's parsed facts.
fn read_member(run_dir: &Path, run_id: &str, sequence: u64) -> Result<ChainMember, StateError> {
    let path = member_path(run_dir, sequence);
    let bytes = std::fs::read(&path).map_err(|error| {
        StateError::new(format!(
            "read the amendment record {}: {error}",
            path.displayed()
        ))
    })?;
    parse_member(run_dir, run_id, sequence, &bytes)
}

fn reconcile_plan(
    engine_root: &Path,
    run_dir: &Path,
    run_id: &str,
    member: &ChainMember,
) -> Result<RecoverPlan, RecoverFailure> {
    let archive_sequence =
        crate::review::next_sequence(run_dir).map_err(RecoverFailure::refusal)?;
    let archive = run_dir
        .join("authorizations")
        .join(format!("{:06}.toml", archive_sequence));
    let archive_needed = !archive.is_file();
    let archive_text = if archive_needed {
        // The digests come from the committed member's own rows, never from
        // any file this invocation was handed.
        let policy = member
            .authorities
            .get("policy")
            .cloned()
            .unwrap_or_else(|| "absent".to_owned());
        let enrollment = member
            .authorities
            .get("enrollment")
            .cloned()
            .unwrap_or_else(|| "absent".to_owned());
        let signature = signature_digest_of(run_dir, member.sequence).unwrap_or_default();
        render_archive_fields(
            archive_sequence,
            member.sequence,
            &member.state,
            &member.state,
            &policy,
            &enrollment,
            member_intent_sha256(member).as_str(),
            &signature,
            reconciliation_origin(engine_root),
        )
    } else {
        String::new()
    };
    Ok(RecoverPlan {
        run_id: run_id.to_owned(),
        engine_root: engine_root.to_path_buf(),
        run_dir: run_dir.to_path_buf(),
        state: member.state.clone(),
        head_before: member.file_sha256.clone(),
        sequence: member.sequence,
        reconcile: true,
        record_text: String::new(),
        archive_text,
        archive_sequence,
        governed: false,
        prior_policy: String::new(),
        manifest_sha256: String::new(),
        change_digest: String::new(),
    })
}

/// The origin a reconciliation records: the injected reader when the trust
/// seam answers, the host otherwise.
/// Whether an archived authorization record already names this amendment
/// sequence as its ratmac-recover-v1 decision: a reconciliation that finds
/// it appends nothing to the archive, never a duplicate.
fn archive_names_amendment(run_dir: &Path, sequence: u64) -> Result<bool, String> {
    let dir = run_dir.join("authorizations");
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("read the authorization archive: {error}")),
    };
    for entry in entries {
        let entry = entry.map_err(|error| format!("read an archive entry: {error}"))?;
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        let Ok(value) = text.parse::<toml::Value>() else {
            continue;
        };
        if value.get("purpose").and_then(toml::Value::as_str) == Some("ratmac-recover-v1")
            && value
                .get("amendment_sequence")
                .and_then(toml::Value::as_integer)
                == Some(sequence as i64)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn reconciliation_origin(engine_root: &Path) -> &'static str {
    let _ = engine_root;
    #[cfg(feature = "test-fault-injection")]
    if crate::review::test::lookup(engine_root).is_some() {
        return "test-injected";
    }
    "host-enrolled"
}

fn signature_digest_of(run_dir: &Path, sequence: u64) -> Option<String> {
    let text = std::fs::read_to_string(member_path(run_dir, sequence)).ok()?;
    let value: toml::Value = text.parse().ok()?;
    value
        .get("signature-identities")
        .and_then(toml::Value::as_array)?
        .first()?
        .get("signature-sha256")
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
}

fn reconcile_apply(
    engine_root: &Path,
    run_dir: &Path,
    run_id: &str,
    sequence: u64,
) -> Result<Recovered, RecoverFailure> {
    // The history must be a proper prefix of the expected events: every
    // present recovery event for this Run is exactly the event its
    // sequence names, and no event beyond the head exists.
    let log_bytes = std::fs::read(engine_root.join("log.md")).unwrap_or_default();
    let ended = newline_ended(&log_bytes);
    let lines: Vec<&[u8]> = log_bytes.split(|byte| *byte == b'\n').collect();
    for (index, line_bytes) in lines.iter().enumerate() {
        let line = String::from_utf8_lossy(line_bytes).into_owned();
        if let Some(rest) =
            line.strip_prefix(&format!("- Recover: run {run_id} adopted amendment "))
        {
            if let Some(present) = rest
                .split(' ')
                .next()
                .and_then(|number| number.parse::<u64>().ok())
            {
                if present <= sequence && *line_bytes != event_line(run_id, present).as_bytes() {
                    // A torn tail - the log's last, unterminated line that is
                    // a proper prefix of the expected event, even torn past
                    // the sequence number - is completed by the suffix
                    // append below, never refused here.
                    let torn_tail = index == lines.len() - 1 && !ended;
                    if !(torn_tail
                        && event_line(run_id, present)
                            .as_bytes()
                            .starts_with(line_bytes))
                    {
                        return Err(RecoverFailure::refusal(format!(
                            "the amendment history for run {run_id} holds a fragment that \
                             is no prefix of the expected event for sequence {sequence}; \
                             repair it outside the Engine"
                        )));
                    }
                }
            }
        }
    }
    // The missing archive record, when this reconciliation owes one: a
    // record already archived for this member satisfies it, never a
    // duplicate beside it.
    let member = read_member(run_dir, run_id, sequence)
        .map_err(|error| RecoverFailure::refusal(error.to_string()))?;
    let already_archived =
        archive_names_amendment(run_dir, sequence).map_err(RecoverFailure::refusal)?;
    let archive_sequence =
        crate::review::next_sequence(run_dir).map_err(RecoverFailure::refusal)?;
    let archive = run_dir
        .join("authorizations")
        .join(format!("{:06}.toml", archive_sequence));
    if !already_archived {
        let policy = member
            .authorities
            .get("policy")
            .cloned()
            .unwrap_or_else(|| "absent".to_owned());
        let enrollment = member
            .authorities
            .get("enrollment")
            .cloned()
            .unwrap_or_else(|| "absent".to_owned());
        let signature = signature_digest_of(run_dir, sequence).unwrap_or_default();
        let text = render_archive_fields(
            archive_sequence,
            sequence,
            &member.state,
            &member.state,
            &policy,
            &enrollment,
            member_intent_sha256(&member).as_str(),
            &signature,
            reconciliation_origin(engine_root),
        );
        std::fs::create_dir_all(archive.parent().unwrap_or(run_dir)).map_err(|error| {
            committed_pending(
                sequence,
                format!("cannot create the authorization archive: {error}"),
            )
        })?;
        write_whole_atomically(&archive, text.as_bytes()).map_err(|error| {
            committed_pending(
                sequence,
                format!(
                    "cannot write the authorization record {}: {error}",
                    archive.displayed()
                ),
            )
        })?;
        crate::observe::operation("record");
    }
    // The missing event(s): every committed sequence without its exact
    // line. A log tail that is a proper prefix of the expected event bytes
    // is completed by appending the missing suffix, never by truncating
    // history; a tail that is no prefix of the expected event refuses.
    for pending in 1..=sequence {
        // Each completed sequence appends bytes, so the tail is re-read
        // before every prefix test: a later sequence must judge the tail
        // as it exists on disk now, not as this loop first found it.
        let log_bytes = std::fs::read(engine_root.join("log.md")).unwrap_or_default();
        let expected = event_line(run_id, pending);
        if log_bytes
            .split(|byte| *byte == b'\n')
            .any(|line| line == expected.as_bytes())
        {
            continue;
        }
        let tail_start = log_bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map(|at| at + 1)
            .unwrap_or(0);
        let tail: &[u8] = if newline_ended(&log_bytes) {
            &log_bytes[log_bytes.len()..]
        } else {
            &log_bytes[tail_start..]
        };
        if !tail.is_empty() {
            if expected.as_bytes().starts_with(tail) {
                let suffix = &expected.as_bytes()[tail.len()..];
                let mut completion = suffix.to_vec();
                completion.push(b'\n');
                append_event_bytes(engine_root, &completion)
                    .map_err(|failure| committed_pending(sequence, failure.detail()))?;
                continue;
            }
            return Err(RecoverFailure::refusal(format!(
                "the amendment history for run {run_id} holds a fragment that is no prefix \
                 of the expected event for sequence {pending}; repair it outside the Engine"
            )));
        }
        append_event(engine_root, run_id, pending)
            .map_err(|failure| committed_pending(sequence, failure.detail()))?;
    }
    Ok(Recovered {
        run_id: run_id.to_owned(),
        sequence,
    })
}

/// The digest of a member's exact canonical authorized bytes - the bytes
/// its `signed-intent` field spells as lowercase hexadecimal.
fn member_intent_sha256(member: &ChainMember) -> String {
    sha256_hex(&hex_decode(&member.signed_intent).unwrap_or_default())
}

/// Whether the shared log ends on a complete line.
fn newline_ended(bytes: &[u8]) -> bool {
    bytes.last() == Some(&b'\n')
}

/// Append raw event bytes through the one shared-log write boundary.
fn append_event_bytes(engine_root: &Path, completion: &[u8]) -> Result<(), RecoverFailure> {
    let mut log = crate::Scheduler::open_transition_log(engine_root, false)
        .map_err(|error| RecoverFailure::refusal(format!("open log.md: {error}")))?;
    match crate::Scheduler::complete_transition_log_line(&mut log, completion) {
        crate::scheduler::TransitionLogAppend::Complete => Ok(()),
        crate::scheduler::TransitionLogAppend::Failed(failure) => Err(RecoverFailure::refusal(
            failure.into_operator_error().to_string(),
        )),
    }
}

fn append_event(engine_root: &Path, run_id: &str, sequence: u64) -> Result<(), RecoverFailure> {
    let mut log = crate::Scheduler::open_transition_log(engine_root, false)
        .map_err(|error| RecoverFailure::refusal(format!("open log.md: {error}")))?;
    let entry = format!("\n{}\n", event_line(run_id, sequence));
    match crate::Scheduler::append_transition_log(&mut log, entry.as_bytes()) {
        crate::scheduler::TransitionLogAppend::Complete => Ok(()),
        crate::scheduler::TransitionLogAppend::Failed(failure) => Err(RecoverFailure::refusal(
            failure.into_operator_error().to_string(),
        )),
    }
}

// --- the enrollment reads -------------------------------------------------------

/// The governed read: the prior trusted policy's enrollment. The injected
/// trust reader answers first, exactly as `read_enrollment` consults it;
/// the host path requires the runbook still to declare the head's policy.
fn read_governed_trust(
    engine_root: &Path,
    run_dir: &Path,
    run_id: &str,
    prior_policy: &str,
    scheduler: &crate::Scheduler,
) -> Result<RecoveryTrust, String> {
    let project_scope = crate::Scheduler::project_scope(engine_root);
    #[cfg(feature = "test-fault-injection")]
    if let Some(trust) = crate::review::test::lookup(engine_root) {
        // The prior policy's work list still comes from the runbook in
        // force: work is the scope the intent digests, never the manifest's.
        let work = scheduler
            .declared_review_of(run_dir, run_id)
            .ok()
            .flatten()
            .map(|declaration| declaration.work().to_vec())
            .unwrap_or_default();
        return trust_facts(
            &trust.manifest_bytes,
            &trust.signers_path,
            &trust.verifier_path,
            &project_scope,
            Some(prior_policy),
            &trust.verifier_sha256,
            "test-injected",
            work,
        );
    }
    // The host path: the declared table of the runbook in force, when it
    // still declares the head's policy, is the manifest to read.
    let declaration = scheduler
        .declared_review_of(run_dir, run_id)
        .map_err(|error| error.to_string())?;
    if let Some(declaration) = declaration {
        if declaration.policy_digest() == prior_policy {
            let enrollment =
                crate::review::read_enrollment(engine_root, &declaration, &project_scope)?;
            return Ok(RecoveryTrust {
                manifest_sha256: enrollment.manifest_sha256,
                policy_digest: declaration.policy_digest().to_owned(),
                approver: declaration.approver().to_owned(),
                reviewers: declaration.reviewers().to_vec(),
                verifier_path: enrollment.verifier_path,
                signers_path: enrollment.signers_path,
                origin: enrollment.origin,
                work: declaration.work().to_vec(),
            });
        }
    }
    Err(format!(
        "run {run_id} is governed by policy-sha256 {prior_policy} that the runbook no longer \
         declares; no recovery may proceed until the prior trusted policy is independently \
         enrolled again"
    ))
}

/// The external read of a legacy Run: seam first, then the caller-named
/// path through the landed protection boundary - the one route that does
/// not consult a declaration at all.
fn read_external_trust(engine_root: &Path, path: &Path) -> Result<RecoveryTrust, String> {
    let project_scope = crate::Scheduler::project_scope(engine_root);
    #[cfg(feature = "test-fault-injection")]
    if let Some(trust) = crate::review::test::lookup(engine_root) {
        return trust_facts(
            &trust.manifest_bytes,
            &trust.signers_path,
            &trust.verifier_path,
            &project_scope,
            None,
            &trust.verifier_sha256,
            "test-injected",
            Vec::new(),
        );
    }
    let manifest_bytes = std::fs::read(path).map_err(|_| {
        format!(
            "the enrollment manifest {} is missing or unreadable",
            path.displayed()
        )
    })?;
    let actor = crate::review::builder_actor();
    let manifest = crate::review::parse_manifest(&manifest_bytes).map_err(|detail| {
        format!(
            "the enrollment manifest {} is malformed: {detail}",
            path.displayed()
        )
    })?;
    crate::review::protection::require_protected(path, &actor).map_err(
        |refusal| match refusal {
            crate::review::protection::ProtectionRefusal::NotProtected => format!(
                "the enrollment manifest {} is not protected against the builder actor {actor}",
                path.displayed()
            ),
            crate::review::protection::ProtectionRefusal::Unverifiable(detail) => detail,
        },
    )?;
    crate::review::protection::require_protected(&manifest.signers_path, &actor).map_err(
        |refusal| match refusal {
            crate::review::protection::ProtectionRefusal::NotProtected => format!(
                "the allowed-signers file {} is not protected against the builder actor \
                 {actor}",
                manifest.signers_path.displayed()
            ),
            crate::review::protection::ProtectionRefusal::Unverifiable(detail) => detail,
        },
    )?;
    trust_facts(
        &manifest_bytes,
        &manifest.signers_path,
        &manifest.verifier_path,
        &project_scope,
        None,
        &manifest.verifier_digest,
        "host-enrolled",
        Vec::new(),
    )
}

/// Validate one manifest's facts into the recovery trust. `prior_policy`
/// pins the digest a governed read must reproduce; the external read trusts
/// the manifest's own policy.
#[allow(clippy::too_many_arguments)]
fn trust_facts(
    manifest_bytes: &[u8],
    signers_path: &Path,
    verifier_path: &Path,
    project_scope: &str,
    prior_policy: Option<&str>,
    verifier_binding: &str,
    origin: &'static str,
    work: Vec<String>,
) -> Result<RecoveryTrust, String> {
    let manifest = crate::review::parse_manifest(manifest_bytes)
        .map_err(|detail| format!("the enrollment manifest is malformed: {detail}"))?;
    if manifest.builder_actor != crate::review::builder_actor() {
        return Err(format!(
            "the enrollment manifest binds the builder actor {:?}; this process is {:?}",
            manifest.builder_actor,
            crate::review::builder_actor()
        ));
    }
    if manifest.project_scope != project_scope {
        return Err(format!(
            "the enrollment manifest binds the project scope {}; this checkout is {}",
            manifest.project_scope, project_scope
        ));
    }
    if let Some(prior) = prior_policy {
        if manifest.policy_digest != prior {
            return Err(format!(
                "the enrollment manifest binds policy-sha256 {}; the prior trusted policy \
                 is {prior}",
                manifest.policy_digest
            ));
        }
    }
    // The enrollment, verifier, and signers validation boundary is a
    // target operation, exactly as the landed t-129 placement renders it.
    crate::observe::operation("target");
    let verifier_real = crate::review::file_sha256(verifier_path).map_err(|error| {
        format!(
            "the verifier at {} cannot be read: {error}",
            verifier_path.displayed()
        )
    })?;
    if verifier_real != verifier_binding {
        return Err(format!(
            "the verifier at {} hashes {}, the enrollment binds {}",
            verifier_path.displayed(),
            verifier_real,
            verifier_binding
        ));
    }
    let signers_real = crate::review::file_sha256(signers_path).map_err(|error| {
        format!(
            "the allowed-signers file at {} cannot be read: {error}",
            signers_path.displayed()
        )
    })?;
    if signers_real != manifest.signers_digest {
        return Err(format!(
            "the allowed-signers file at {} hashes {}, the enrollment binds {}",
            signers_path.displayed(),
            signers_real,
            manifest.signers_digest
        ));
    }
    Ok(RecoveryTrust {
        manifest_sha256: sha256_hex(manifest_bytes),
        policy_digest: manifest.policy_digest,
        approver: manifest.approver,
        reviewers: manifest.reviewers,
        verifier_path: verifier_path.to_path_buf(),
        signers_path: signers_path.to_path_buf(),
        origin,
        work,
    })
}

// --- the derivation -------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn derive_authorities(
    scheduler: &crate::Scheduler,
    run_dir: &Path,
    run_id: &str,
    state: &crate::model::RunState,
    evidence: &crate::pin::Evidence,
    governed: bool,
    trust: &RecoveryTrust,
    head: Option<&ChainHead>,
    prior_policy: &str,
    snapshot_sha256: &str,
) -> Result<Derivation, String> {
    let workspace = scheduler
        .workspace_root()
        .map_err(|error| error.to_string())?
        .to_path_buf();
    let absent = "absent".to_owned();
    let head_authority = |field: &str| -> Option<String> {
        head.and_then(|head| head.authorities.get(field).cloned())
    };
    let mut old = Authorities::new();
    old.insert(
        "engine".to_owned(),
        head_authority("engine").unwrap_or_else(|| {
            evidence
                .engine
                .as_ref()
                .map(|identity| identity.sha256.clone())
                .unwrap_or_else(|| absent.clone())
        }),
    );
    old.insert(
        "runbook".to_owned(),
        head_authority("runbook")
            .or_else(|| evidence.runbook_sha256.clone())
            .unwrap_or_else(|| absent.clone()),
    );
    old.insert(
        "goal".to_owned(),
        head_authority("goal").unwrap_or_else(|| {
            evidence
                .goal_frozen
                .clone()
                .or_else(|| evidence.goal_baseline.clone())
                .unwrap_or_else(|| absent.clone())
        }),
    );
    old.insert(
        "policy".to_owned(),
        head_authority("policy").unwrap_or_else(|| {
            if governed {
                prior_policy.to_owned()
            } else {
                absent.clone()
            }
        }),
    );
    old.insert(
        "enrollment".to_owned(),
        head_authority("enrollment").unwrap_or_else(|| {
            if governed {
                trust.manifest_sha256.clone()
            } else {
                absent.clone()
            }
        }),
    );
    let old_gates: Vec<(String, String)> = evidence
        .gates
        .iter()
        .map(|pin| (pin.program.clone(), pin.identity.sha256.clone()))
        .collect();
    old.insert(
        "gate-artifact".to_owned(),
        head_authority("gate-artifact").unwrap_or_else(|| gate_artifact_identity(&old_gates)),
    );

    let mut new = Authorities::new();
    new.insert(
        "engine".to_owned(),
        crate::review::engine_identity_sha256()?,
    );
    new.insert("runbook".to_owned(), snapshot_sha256.to_owned());
    let version = crate::goal::Version::of_baseline(evidence.goal_baseline.as_deref())
        .unwrap_or(crate::goal::Version::Legacy);
    new.insert(
        "goal".to_owned(),
        scheduler
            .goal_revision_of(&workspace, version)
            .map_err(|error| error.to_string())?
            .unwrap_or_else(|| absent.clone()),
    );
    let declared_policy = scheduler
        .declared_review_of(run_dir, run_id)
        .map_err(|error| error.to_string())?
        .map(|declaration| declaration.policy_digest().to_owned())
        .unwrap_or_else(|| absent.clone());
    new.insert("policy".to_owned(), declared_policy);
    new.insert("enrollment".to_owned(), trust.manifest_sha256.clone());
    let new_gates = scheduler
        .state_gate_artifacts(&workspace, state.state.as_str())
        .unwrap_or_default();
    new.insert(
        "gate-artifact".to_owned(),
        gate_artifact_identity(&new_gates),
    );

    let mut changes = Vec::new();
    for field in AUTHORITY_FIELDS {
        let old_value = old.get(field).map(String::as_str).unwrap_or("absent");
        let new_value = new.get(field).map(String::as_str).unwrap_or("absent");
        if old_value != new_value {
            changes.push(field.to_owned());
        }
    }
    let old_digest = authority_digest(&old);
    let new_digest = authority_digest(&new);
    let change_digest = change_set_digest(&changes, &old_digest, &new_digest);
    Ok(Derivation {
        old,
        new,
        changes,
        new_digest,
        change_digest,
    })
}

// --- the decision boundary ------------------------------------------------------

fn refusal_line(
    kind: &str,
    observed: impl std::fmt::Display,
    expected: impl std::fmt::Display,
) -> RecoverFailure {
    RecoverFailure::refusal(format!(
        "authorization: {kind}: observed {observed}; expected {expected}"
    ))
}

#[allow(clippy::too_many_arguments)]
fn verify_decision(
    run_dir: &Path,
    run_id: &str,
    trust: &RecoveryTrust,
    facts: &crate::review::IntentFacts,
    intent: &[u8],
    decision: Option<&Path>,
    head: Option<&ChainHead>,
    declared_materials: &[String],
) -> Result<(), RecoverFailure> {
    let expected_line = format!(
        "one {RECOVER_NAMESPACE} signature by the designated approver over the canonical intent \
         (rtm intent --run {run_id} --recover)"
    );
    let Some(decision_path) = decision else {
        return Err(refusal_line("decision", "none", expected_line));
    };
    let signature_bytes = match std::fs::read(decision_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Err(refusal_line(
                "decision",
                format!(
                    "an unreadable signature at {}: {error}",
                    decision_path.displayed()
                ),
                expected_line,
            ))
        }
    };
    let signature_sha = sha256_hex(&signature_bytes);
    let consumed = crate::review::archived_decision_at(run_dir, &signature_sha)
        .map_err(|detail| refusal_line("replay", detail, "a readable authorization archive"))?;
    if let Some(consumed) = consumed {
        return Err(refusal_line(
            "replay",
            format!(
                "the decision already consumed as sequence {consumed} under \
                 {}/runs/{}/authorizations/",
                facts.engine_root().displayed(),
                run_id
            ),
            format!("a fresh authorization at sequence {}", consumed + 1),
        ));
    }
    if crate::review::ssh_verify(
        &trust.verifier_path,
        &trust.signers_path,
        &trust.approver,
        RECOVER_NAMESPACE,
        intent,
        &signature_bytes,
    ) {
        return Ok(());
    }
    // A stale chain tip: the decision may bind an older effective head.
    if let Some(head) = head {
        if let Some(older) = stale_head_bound(run_dir, head, trust, facts, &signature_bytes) {
            return Err(refusal_line(
                "chain",
                format!("a decision bound to the head that hashes {older}"),
                format!("the effective head {}", head.file_sha256),
            ));
        }
    }
    // Attribution runs under every other enrolled label: the manifest's
    // reviewers first, then the labels the allowed-signers file itself
    // enrolls.
    let mut member_labels: Vec<String> = trust
        .reviewers
        .iter()
        .map(|(label, _)| label.clone())
        .filter(|label| label != &trust.approver)
        .collect();
    if let Ok(signers) = std::fs::read_to_string(&trust.signers_path) {
        for line in signers.lines() {
            if line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split_whitespace().collect();
            if let Some(label) = fields.first() {
                if *label != trust.approver && !member_labels.iter().any(|known| known == label) {
                    member_labels.push((*label).to_owned());
                }
            }
        }
    }
    for label in &member_labels {
        if crate::review::ssh_verify(
            &trust.verifier_path,
            &trust.signers_path,
            label,
            RECOVER_NAMESPACE,
            intent,
            &signature_bytes,
        ) {
            return Err(refusal_line(
                "decision",
                format!("a signature by member {label:?}"),
                format!("the designated approver {:?}", trust.approver),
            ));
        }
    }
    let embedded = crate::review::signature_key_material(&signature_bytes);
    let mut enrolled_materials: Vec<String> = trust
        .reviewers
        .iter()
        .map(|(_, material)| material.clone())
        .collect();
    if let Some(approver_material) =
        crate::review::reviewer_material_of(&trust.reviewers, &trust.approver)
    {
        enrolled_materials.push(approver_material);
    }
    // Every identity the prior trust's allowed-signers file enrolls carries
    // the same authority as the declared reviewers: a signature by one of
    // them that does not verify is the does-not-verify family.
    if let Ok(signers) = std::fs::read_to_string(&trust.signers_path) {
        for line in signers.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 3 && !line.starts_with('#') {
                enrolled_materials.push(format!("{} {}", fields[1], fields[2]));
            }
        }
    }
    enrolled_materials.extend(declared_materials.iter().cloned());
    let enrolled_body = embedded.as_ref().and_then(|embedded| {
        crate::review::embedded_key_body(embedded)
            .filter(|body| enrolled_materials.iter().any(|material| material == body))
            .map(|_| ())
    });
    if enrolled_body.is_some() {
        return Err(refusal_line(
            "decision",
            format!(
                "a signature that does not verify against the recomputed canonical intent \
                 sha256 {}",
                sha256_hex(intent)
            ),
            format!(
                "a fresh {RECOVER_NAMESPACE} decision of that exact intent (rtm intent \
                 --run {run_id} --recover)"
            ),
        ));
    }
    Err(refusal_line(
        "decision",
        "a signature by no enrolled identity",
        format!("the designated approver {:?}", trust.approver),
    ))
}

/// Whether a signature verifies against the same facts bound to an older
/// effective head: the digest the head replaced.
fn stale_head_bound(
    run_dir: &Path,
    head: &ChainHead,
    trust: &RecoveryTrust,
    facts: &crate::review::IntentFacts,
    signature_bytes: &[u8],
) -> Option<String> {
    let engine_identity = crate::review::engine_identity_sha256().ok()?;
    let project_scope = crate::Scheduler::project_scope(facts.engine_root());
    // Each prior head k, paired with the amendment sequence the Run would
    // have minted while that head was effective: a decision signed against
    // head k carries both its digest and its next sequence, so both fields
    // are restored together when the older intent is reconstructed.
    let mut prior: Vec<(String, u64)> = Vec::new();
    for sequence in 1..head.sequence {
        let bytes = std::fs::read(member_path(run_dir, sequence)).ok()?;
        prior.push((sha256_hex(&bytes), sequence + 1));
    }
    let evidence_text =
        std::fs::read_to_string(crate::pin::evidence_path(run_dir)).unwrap_or_default();
    prior.push((pin_record_digest(&evidence_text), 1));
    let mut candidate = facts.clone();
    for (digest, sequence) in prior {
        if digest == head.file_sha256 {
            continue;
        }
        candidate.previous_amendment_digest = digest.clone();
        candidate.sequence = sequence;
        let bytes = candidate.canonical_bytes(&project_scope, &engine_identity);
        if crate::review::ssh_verify(
            &trust.verifier_path,
            &trust.signers_path,
            &trust.approver,
            RECOVER_NAMESPACE,
            &bytes,
            signature_bytes,
        ) {
            return Some(digest);
        }
    }
    None
}

// --- the records ----------------------------------------------------------------

fn toml_basic_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            character if (character as u32) < 0x20 || character as u32 == 0x7f => {
                out.push_str(&format!("\\u{:04X}", character as u32))
            }
            character => out.push(character),
        }
    }
    out.push('"');
    out
}

/// The approver's enrolled key material. RB703 guarantees the declared
/// reviewers carry it; a trust that somehow omits it refuses before any
/// preparation write - an amendment never names an empty key.
fn signer_key_of(trust: &RecoveryTrust) -> Option<String> {
    crate::review::reviewer_material_of(&trust.reviewers, &trust.approver)
}

fn render_record(
    sequence: u64,
    derivation: &Derivation,
    intent: &[u8],
    facts: &crate::review::IntentFacts,
    trust: &RecoveryTrust,
    decision_sha: &str,
) -> String {
    // The exact canonical authorized bytes, spelled as lowercase
    // hexadecimal: the framed bytes need not be UTF-8.
    let signed_intent = hex_encode(intent);
    let mut text = String::new();
    text.push_str("version = 1\n");
    text.push_str(&format!("sequence = {sequence}\n"));
    text.push_str(&format!(
        "previous-amendment-digest = {}\n",
        toml_basic_string(&facts.previous_amendment_digest)
    ));
    text.push_str(&format!("run = {}\n", toml_basic_string(&facts.run_id)));
    text.push_str(&format!(
        "changes = [{}]\n",
        derivation
            .changes
            .iter()
            .map(|change| toml_basic_string(change))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    text.push_str(&format!(
        "affected-proof-boundary = {}\n",
        toml_basic_string(&derivation.changes.join(","))
    ));
    text.push_str(&format!(
        "signed-intent = {}\n",
        toml_basic_string(&signed_intent)
    ));
    text.push_str("\n[context]\n");
    text.push_str(&format!(
        "state = {}\n",
        toml_basic_string(&facts.from_state)
    ));
    text.push_str(&format!(
        "class = {}\n",
        toml_basic_string(&facts.owning_class)
    ));
    let bindings: Vec<String> = facts
        .bindings
        .as_ref()
        .map(|bindings| {
            let mut pairs: Vec<String> = bindings
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect();
            pairs.sort();
            pairs
        })
        .unwrap_or_default();
    text.push_str(&format!(
        "bindings = [{}]\n",
        bindings
            .iter()
            .map(|binding| toml_basic_string(binding))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    text.push_str(&format!(
        "workspace = {}\n",
        toml_basic_string(&facts.workspace)
    ));
    for (name, authorities) in [
        ("old-authorities", &derivation.old),
        ("new-authorities", &derivation.new),
    ] {
        text.push_str(&format!("\n[{name}]\n"));
        for field in AUTHORITY_FIELDS {
            text.push_str(&format!(
                "{field} = {}\n",
                toml_basic_string(
                    authorities
                        .get(field)
                        .map(String::as_str)
                        .unwrap_or("absent")
                )
            ));
        }
    }
    text.push_str("\n[[signature-identities]]\n");
    text.push_str(&format!("label = {}\n", toml_basic_string(&trust.approver)));
    text.push_str(&format!(
        "key = {}\n",
        toml_basic_string(&signer_key_of(trust).unwrap_or_default())
    ));
    text.push_str(&format!(
        "signature-sha256 = {}\n",
        toml_basic_string(decision_sha)
    ));
    text
}

fn render_archive(
    sequence: u64,
    amendment_sequence: u64,
    facts: &crate::review::IntentFacts,
    trust: &RecoveryTrust,
    intent: &[u8],
    decision: Option<&Path>,
) -> String {
    let signature = decision
        .map(|path| sha256_hex(&std::fs::read(path).unwrap_or_default()))
        .unwrap_or_default();
    render_archive_fields(
        sequence,
        amendment_sequence,
        &facts.from_state,
        &facts.to_state,
        &trust.policy_digest,
        &trust.manifest_sha256,
        &sha256_hex(intent),
        &signature,
        trust.origin,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_archive_fields(
    sequence: u64,
    amendment_sequence: u64,
    from_state: &str,
    to_state: &str,
    policy: &str,
    enrollment: &str,
    intent_sha: &str,
    signature_sha: &str,
    origin: &str,
) -> String {
    let mut text = String::new();
    text.push_str(&format!("amendment_sequence = {amendment_sequence}\n"));
    text.push_str(&format!(
        "decision_signature_sha256 = {}\n",
        toml_basic_string(signature_sha)
    ));
    text.push_str(&format!(
        "enrollment_sha256 = {}\n",
        toml_basic_string(enrollment)
    ));
    text.push_str(&format!("from_state = {}\n", toml_basic_string(from_state)));
    text.push_str(&format!(
        "intent_sha256 = {}\n",
        toml_basic_string(intent_sha)
    ));
    text.push_str(&format!("origin = {}\n", toml_basic_string(origin)));
    text.push_str(&format!("policy_sha256 = {}\n", toml_basic_string(policy)));
    text.push_str(&format!(
        "purpose = {}\n",
        toml_basic_string(RECOVER_NAMESPACE)
    ));
    text.push_str(&format!("sequence = {sequence}\n"));
    text.push_str(&format!("to_state = {}\n", toml_basic_string(to_state)));
    text
}

/// The recover intent render: the exact facts a recovery decision signs,
/// derived without a confirmation, a decision, or any write. Mirrors the
/// plan's resolution order so the rendered bytes are the bytes a later
/// `rtm recover` verifies.
pub(crate) fn recover_intent_in(
    roots: &Roots,
    run_id: &str,
    enrollment: Option<&Path>,
) -> Result<crate::review::IntentFacts, String> {
    let engine_root = roots.engine_root().to_path_buf();
    let scheduler = crate::Scheduler::open_run_for_recover_with_roots(roots, run_id)
        .map_err(|error| error.to_string())?;
    let run_dir = engine_root.join("runs").join(run_id);
    let state = crate::state::StateStore::for_engine_root(&engine_root, run_id)
        .load()
        .map_err(|error| error.to_string())?;
    if state.status == crate::model::Status::Passed {
        return Err(format!(
            "run {run_id} is terminal (status passed): recovery may not amend a terminal Run"
        ));
    }
    let evidence = crate::pin::Evidence::load(&run_dir);
    let snapshot_sha256 = scheduler.runbook_pin_of().to_owned();
    let class = scheduler
        .runbook_class()
        .map_err(|error| error.to_string())?;
    let child_class = scheduler.child_class().map(str::to_owned);
    let class_name = child_class.clone().unwrap_or_else(|| "runbook".to_owned());
    let states = match child_class.as_deref() {
        Some(child) => class.classes().get(child).map(|child| child.states()),
        None => Some(class.states()),
    };
    let Some(states) = states else {
        return Err(format!(
            "the corrected runbook declares no class {class_name}; recovery keeps the owning \
             class"
        ));
    };
    if states.get(state.state.as_str()).is_none() {
        return Err(format!(
            "the corrected runbook declares no State {} in class {class_name}; recovery \
             refuses a removed State",
            state.state
        ));
    }
    let head = validate_chain(&run_dir, run_id).map_err(|error| error.to_string())?;
    let governed = run_dir.join(crate::review::GOVERNANCE_FILE).is_file();
    let prior_policy = match &head {
        Some(head) => head
            .authorities
            .get("policy")
            .cloned()
            .unwrap_or_else(|| "absent".to_owned()),
        None => {
            if governed {
                crate::review::marker_policy(
                    &std::fs::read(run_dir.join(crate::review::GOVERNANCE_FILE))
                        .unwrap_or_default(),
                )
                .unwrap_or_default()
            } else {
                "absent".to_owned()
            }
        }
    };
    let trust = if governed {
        read_governed_trust(&engine_root, &run_dir, run_id, &prior_policy, &scheduler)
    } else {
        match enrollment {
            Some(path) => read_external_trust(&engine_root, path),
            None => Err(format!(
                "run {run_id} lacks previously enrolled recovery trust; an independently \
                 enrolled external authority must supply it (--enrollment <path>): the builder \
                 may neither generate it nor substitute task authorization for it"
            )),
        }
    }?;
    let derivation = derive_authorities(
        &scheduler,
        &run_dir,
        run_id,
        &state,
        &evidence,
        governed,
        &trust,
        head.as_ref(),
        &prior_policy,
        &snapshot_sha256,
    )?;
    scheduler.recover_facts(
        &engine_root,
        run_id,
        &trust,
        &derivation,
        &state,
        head.as_ref(),
    )
}

// --- the seams -------------------------------------------------------------------

#[cfg(feature = "test-fault-injection")]
fn inject_recover_fault(boundary: &str) -> Result<bool, RecoverFailure> {
    Ok(std::env::var("RATMAC_TEST_RECOVER_FAULT").as_deref() == Ok(boundary))
}

#[cfg(not(feature = "test-fault-injection"))]
fn inject_recover_fault(_boundary: &str) -> Result<bool, RecoverFailure> {
    Ok(false)
}

#[cfg(feature = "test-fault-injection")]
fn wait_at_recover_pause(boundary: &str) -> Result<(), RecoverFailure> {
    if std::env::var("RATMAC_TEST_RECOVER_PAUSE").as_deref() != Ok(boundary) {
        return Ok(());
    }
    let marker = std::env::var_os("RATMAC_TEST_RECOVER_PAUSE_MARKER")
        .map(PathBuf::from)
        .unwrap_or_default();
    let release = std::env::var_os("RATMAC_TEST_RECOVER_PAUSE_RELEASE")
        .map(PathBuf::from)
        .unwrap_or_default();
    let timeout = std::env::var("RATMAC_TEST_RECOVER_PAUSE_TIMEOUT_MILLIS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(60000);
    if !marker.as_os_str().is_empty() {
        if let Some(parent) = marker.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    RecoverFailure::refusal(format!(
                        "the recover pause seam cannot create the marker directory {}: {error}",
                        parent.displayed()
                    ))
                })?;
            }
        }
        std::fs::write(&marker, format!("{boundary}\n")).map_err(|error| {
            RecoverFailure::refusal(format!(
                "the recover pause seam cannot write the marker {}: {error}",
                marker.displayed()
            ))
        })?;
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout);
    loop {
        if release.is_file() {
            return Ok(());
        }
        let now = std::time::Instant::now();
        if now >= deadline {
            return Err(RecoverFailure::refusal(format!(
                "injected recover pause at {boundary} expired"
            )));
        }
        std::thread::sleep(
            std::time::Duration::from_millis(10).min(deadline.saturating_duration_since(now)),
        );
    }
}

#[cfg(not(feature = "test-fault-injection"))]
fn wait_at_recover_pause(_boundary: &str) -> Result<(), RecoverFailure> {
    Ok(())
}

/// The pause seam's companions are validated when recover starts, before
/// any lock or write.
#[cfg(feature = "test-fault-injection")]
fn check_pause_companions() -> Result<(), RecoverFailure> {
    let Ok(boundary) = std::env::var("RATMAC_TEST_RECOVER_PAUSE") else {
        return Ok(());
    };
    // Only a recognized boundary arms the seam; an unset, empty, or
    // unrecognized value changes nothing, even with no marker present.
    if !matches!(boundary.as_str(), "before-rename" | "after-rename") {
        return Ok(());
    }
    for companion in [
        "RATMAC_TEST_RECOVER_PAUSE_MARKER",
        "RATMAC_TEST_RECOVER_PAUSE_RELEASE",
    ] {
        let missing = std::env::var_os(companion)
            .map(|value| value.is_empty())
            .unwrap_or(true);
        if missing {
            return Err(RecoverFailure::refusal(format!(
                "the recover pause seam names no {companion} companion; the pause test seam \
                 requires one before any lock or write"
            )));
        }
    }
    let timeout_valid = std::env::var("RATMAC_TEST_RECOVER_PAUSE_TIMEOUT_MILLIS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|millis| millis > 0);
    if !timeout_valid {
        return Err(RecoverFailure::refusal(
            "the recover pause seam names no positive integer \
             RATMAC_TEST_RECOVER_PAUSE_TIMEOUT_MILLIS companion; the pause test seam requires \
             one before any lock or write",
        ));
    }
    Ok(())
}

#[cfg(not(feature = "test-fault-injection"))]
fn check_pause_companions() -> Result<(), RecoverFailure> {
    Ok(())
}
