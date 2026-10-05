//! Authorized review transitions (WRA-001).
//!
//! A runbook may declare a review policy - a `[review]` table at the top
//! level or `review` inside a `classes.<name>` body - binding the enrollment
//! facts that govern every transition of the Runs it admits: an addressed
//! enrollment manifest, the verifier executable and allowed-signers file it
//! pins by digest, the reviewer roles, the designated Approver, and the
//! minimum count of distinct reviews. A Run admitted under a declaring
//! class is governed for its whole life: every ordinary step and every hold
//! passes the one verification boundary below before it may write, and the
//! boundary archives exactly one strict-TOML authorization record per
//! accepted transition under the Run's own `authorizations/` directory.
//!
//! The canonical intent is the signed payload: `ratmac-authorization-v1`,
//! one NUL byte, then every varying fact of one motion, each as its
//! unsigned 64-bit big-endian byte length followed by its UTF-8 bytes, with
//! inapplicable fields empty and never omitted. Reviews sign the review
//! intent (review-set digest empty) under `ratmac-review-v1`; the Approver
//! signs the approval intent - the same bytes plus the review-set digest
//! over the exact submitted signature files - under `ratmac-approve-v1`.
//! Both are verified by the pinned verifier itself (`ssh-keygen -Y
//! verify`), judged by exit status alone; signature attribution never
//! trusts a signer field a caller can forge.
//!
//! Production trust is an operator-protected enrollment: a manifest and
//! allowed-signers file owned by the host's administrative principal with
//! the builder's effective token denied write along the path. Nothing a
//! builder can write satisfies it, so no test can accidentally pass it and
//! no governed workflow self-enrolls. The library seam in [`test`] is the
//! only test route past that check.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::machine::MachineClassParseError;
use crate::root::Displayed;

/// The detached-signature namespace of a review signature.
pub const REVIEW_NAMESPACE: &str = "ratmac-review-v1";
/// The detached-signature namespace of an approval signature.
pub const APPROVE_NAMESPACE: &str = "ratmac-approve-v1";
/// WRS-003: the detached-signature namespace of a governed resume's
/// resolution signature.
pub const RESUME_NAMESPACE: &str = "ratmac-resume-v1";

/// The canonical intent's framing prefix.
pub(crate) const INTENT_PREFIX: &[u8] = b"ratmac-authorization-v1";
/// The policy digest's framing prefix.
const POLICY_PREFIX: &[u8] = b"ratmac-policy-v1";
/// The review-set digest's framing prefix.
const REVIEW_SET_PREFIX: &[u8] = b"ratmac-review-set-v1";

/// The governance marker file a governed Run's directory carries from its
/// admission on: `.ratmac/runs/<id>/review.toml`. Its presence is the Run's
/// recorded provenance - governance is fixed by the pinned runbook the Run
/// was admitted under, never re-derived from a live runbook a builder may
/// drift. The recorded value is the declared policy digest.
pub(crate) const GOVERNANCE_FILE: &str = "review.toml";

/// The policy digest a governance marker records, if its bytes parse.
pub(crate) fn marker_policy(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;
    let line = text.lines().next()?;
    let value = line.strip_prefix("policy-sha256 = \"")?.strip_suffix('"')?;
    Some(value.to_owned())
}

/// One declared review policy, parsed and internally consistent (RB70x
/// validates the whole table at runbook-parse time).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewDeclaration {
    manifest: String,
    policy_digest: String,
    verifier_digest: String,
    signers_digest: String,
    minimum_reviews: u64,
    approver: String,
    approver_may_review: bool,
    work: Vec<String>,
    reviewers: Vec<(String, String)>,
}

impl ReviewDeclaration {
    /// The enrollment manifest's addressed path, exactly as declared.
    pub fn manifest(&self) -> &str {
        &self.manifest
    }

    /// The digest over the declared roles and membership.
    pub fn policy_digest(&self) -> &str {
        &self.policy_digest
    }

    /// The declared verifier digest.
    pub fn verifier_digest(&self) -> &str {
        &self.verifier_digest
    }

    /// The declared allowed-signers digest.
    pub fn signers_digest(&self) -> &str {
        &self.signers_digest
    }

    /// The minimum count of distinct accepted reviews.
    pub fn minimum_reviews(&self) -> u64 {
        self.minimum_reviews
    }

    /// The designated Approver's label.
    pub fn approver(&self) -> &str {
        &self.approver
    }

    /// Whether the Approver may also supply one review.
    pub fn approver_may_review(&self) -> bool {
        self.approver_may_review
    }

    /// The declared work paths the current work digest covers.
    pub fn work(&self) -> &[String] {
        &self.work
    }

    /// The declared reviewers, sorted by label.
    pub fn reviewers(&self) -> &[(String, String)] {
        &self.reviewers
    }

    /// The key material (kind and body, comment stripped) of a declared
    /// reviewer label, for identity comparisons.
    pub fn reviewer_material(&self, label: &str) -> Option<String> {
        self.reviewers
            .iter()
            .find(|(name, _)| name == label)
            .map(|(_, key)| key_material(key))
    }

    /// Render the governance marker bytes a governed admission records.
    pub(crate) fn governance_bytes(&self) -> Vec<u8> {
        format!("policy-sha256 = \"{}\"\n", self.policy_digest).into_bytes()
    }
}

fn review_error(code: &'static str, detail: String) -> MachineClassParseError {
    MachineClassParseError::at(
        code,
        "review".to_owned(),
        format!("invalid review declaration: {detail}"),
    )
}

/// The keys a `[review]` table accepts.
const REVIEW_KEYS: [&str; 8] = [
    "manifest",
    "policy-digest",
    "verifier-digest",
    "signers-digest",
    "minimum-reviews",
    "approver",
    "approver-may-review",
    "work",
];

/// Parse one declared review table. `None` means the class declares no
/// review policy. Every defect refuses with its `RB70x` code; unknown keys
/// keep the runbook's own `RB103` family.
pub(crate) fn parse_review_table(
    value: Option<&toml::Value>,
    location: &str,
) -> Result<Option<ReviewDeclaration>, MachineClassParseError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let table = value
        .as_table()
        .ok_or_else(|| review_error("RB701", format!("{location} must be a table")))?;
    if let Some(key) = table
        .keys()
        .find(|key| !REVIEW_KEYS.contains(&key.as_str()) && key.as_str() != "reviewers")
    {
        return Err(MachineClassParseError::at(
            "RB103",
            location.to_owned(),
            format!("invalid ratmac.toml: unknown key {key:?} in {location}"),
        ));
    }
    let string_field = |key: &str| -> Result<String, MachineClassParseError> {
        table
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| review_error("RB701", format!("{key} must be a string in {location}")))
    };
    let digest_field = |key: &str| -> Result<String, MachineClassParseError> {
        let value = string_field(key)?;
        if value.len() == 64
            && value
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        {
            Ok(value)
        } else {
            Err(review_error(
                "RB701",
                format!("{key} must be 64 lowercase hex digits in {location}"),
            ))
        }
    };
    let manifest = string_field("manifest")?;
    if !Path::new(&manifest).is_absolute() {
        return Err(review_error(
            "RB701",
            format!("manifest must be an absolute path in {location}"),
        ));
    }
    let declared_policy_digest = digest_field("policy-digest")?;
    let verifier_digest = digest_field("verifier-digest")?;
    let signers_digest = digest_field("signers-digest")?;
    let minimum_reviews = table
        .get("minimum-reviews")
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| {
            review_error(
                "RB701",
                format!("minimum-reviews must be an integer in {location}"),
            )
        })?;
    if minimum_reviews < 1 {
        return Err(review_error(
            "RB701",
            format!("minimum-reviews must be at least 1 in {location}"),
        ));
    }
    let minimum_reviews = u64::try_from(minimum_reviews).map_err(|_| {
        review_error(
            "RB701",
            format!("minimum-reviews must be at least 1 in {location}"),
        )
    })?;
    // Optional with default false: an absent key means the Approver may
    // not supply one of the reviews. Only a present key that is not a
    // boolean is RB701.
    let approver_may_review = match table.get("approver-may-review") {
        None => false,
        Some(value) => value.as_bool().ok_or_else(|| {
            review_error(
                "RB701",
                format!("approver-may-review must be a boolean in {location}"),
            )
        })?,
    };
    let work = match table.get("work") {
        None => Vec::new(),
        Some(value) => {
            let entries = value.as_array().ok_or_else(|| {
                review_error("RB705", format!("work must be an array in {location}"))
            })?;
            let mut paths = Vec::new();
            for entry in entries {
                let path = entry.as_str().ok_or_else(|| {
                    review_error(
                        "RB705",
                        format!("work entries must be strings in {location}"),
                    )
                })?;
                if path.is_empty() {
                    return Err(review_error(
                        "RB705",
                        format!("work declares an empty path in {location}"),
                    ));
                }
                let relative = Path::new(path);
                if relative.is_absolute()
                    || relative.components().any(|component| {
                        matches!(
                            component,
                            std::path::Component::Prefix(_)
                                | std::path::Component::RootDir
                                | std::path::Component::ParentDir
                        )
                    })
                {
                    return Err(review_error(
                        "RB705",
                        format!("work entry {path:?} escapes the repository in {location}"),
                    ));
                }
                if paths.iter().any(|seen: &String| seen == path) {
                    return Err(review_error(
                        "RB705",
                        format!("work declares {path:?} twice in {location}"),
                    ));
                }
                paths.push(path.to_owned());
            }
            paths
        }
    };

    // RB702: the reviewers list.
    let reviewers_value = table
        .get("reviewers")
        .ok_or_else(|| review_error("RB702", format!("{location} declares no reviewers")))?;
    let rows = reviewers_value.as_array().ok_or_else(|| {
        review_error(
            "RB702",
            format!("{location}.reviewers must be an array of tables"),
        )
    })?;
    let mut reviewers: Vec<(String, String)> = Vec::new();
    for row in rows {
        let row = row.as_table().ok_or_else(|| {
            review_error(
                "RB702",
                format!("{location}.reviewers entries must be tables"),
            )
        })?;
        if row.len() != 2 || !row.contains_key("label") || !row.contains_key("key") {
            return Err(review_error(
                "RB702",
                format!("a {location}.reviewers entry must carry exactly label and key"),
            ));
        }
        let label = row
            .get("label")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| review_error("RB702", "a reviewer label must be a string".to_owned()))?;
        if label.is_empty() {
            return Err(review_error(
                "RB702",
                "a reviewer label must not be empty".to_owned(),
            ));
        }
        if reviewers.iter().any(|(seen, _)| seen == label) {
            return Err(review_error(
                "RB702",
                format!("the label {label:?} is declared twice"),
            ));
        }
        let key = row
            .get("key")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| review_error("RB702", "a reviewer key must be a string".to_owned()))?;
        if !is_ed25519_public_line(key) {
            return Err(review_error(
                "RB702",
                format!("the key declared for reviewer {label:?} is not one ssh-ed25519 line"),
            ));
        }
        let material = key_material(key);
        if let Some((first, _)) = reviewers
            .iter()
            .find(|(_, seen)| key_material(seen) == material)
        {
            return Err(review_error(
                "RB702",
                format!("the same public key appears under labels {first:?} and {label:?}"),
            ));
        }
        reviewers.push((label.to_owned(), key.to_owned()));
    }
    if reviewers.is_empty() {
        return Err(review_error(
            "RB702",
            format!("{location} declares no reviewers"),
        ));
    }
    reviewers.sort_by(|left, right| left.0.cmp(&right.0));

    // RB703: the Approver names a declared reviewer.
    let approver = string_field("approver")?;
    if !reviewers.iter().any(|(label, _)| label == &approver) {
        return Err(review_error(
            "RB703",
            format!("approver {approver:?} names no declared reviewer"),
        ));
    }

    // RB704: the declared roles can supply the minimum.
    let supply = if approver_may_review {
        reviewers.len() as u64
    } else {
        reviewers.len() as u64 - 1
    };
    if minimum_reviews > supply {
        return Err(review_error(
            "RB704",
            format!(
                "minimum-reviews {minimum_reviews} exceeds the {supply} distinct reviews \
                 the declared roles can supply"
            ),
        ));
    }

    Ok(Some(ReviewDeclaration {
        manifest,
        policy_digest: declared_policy_digest,
        verifier_digest,
        signers_digest,
        minimum_reviews,
        approver,
        approver_may_review,
        work,
        reviewers,
    }))
}

/// The contract's policy digest: SHA-256 over `ratmac-policy-v1`, one NUL
/// byte, then the framed Approver label, minimum in canonical decimal,
/// role combination, and each reviewer sorted by label as framed label
/// then key.
pub fn policy_digest(
    approver: &str,
    minimum: u64,
    may_review: bool,
    reviewers: &[(String, String)],
) -> String {
    let mut sorted = reviewers.to_vec();
    sorted.sort_by(|left, right| left.0.cmp(&right.0));
    let mut body = POLICY_PREFIX.to_vec();
    body.push(0);
    frame(&mut body, approver.as_bytes());
    frame(&mut body, minimum.to_string().as_bytes());
    frame(
        &mut body,
        if may_review {
            b"true".as_slice()
        } else {
            b"false".as_slice()
        },
    );
    for (label, key) in &sorted {
        frame(&mut body, label.as_bytes());
        frame(&mut body, key.as_bytes());
    }
    hex(&Sha256::digest(&body))
}

/// The contract's review-set digest: SHA-256 over `ratmac-review-set-v1`,
/// one NUL byte, then for each review sorted by label the framed label and
/// the exact signature-file bytes.
pub fn review_set_digest(reviews: &[(String, Vec<u8>)]) -> String {
    let mut sorted = reviews.to_vec();
    sorted.sort_by(|left, right| left.0.cmp(&right.0));
    let mut body = REVIEW_SET_PREFIX.to_vec();
    body.push(0);
    for (label, signature) in &sorted {
        frame(&mut body, label.as_bytes());
        frame(&mut body, signature);
    }
    hex(&Sha256::digest(&body))
}

/// Length-frame one field: unsigned 64-bit big-endian length, then bytes.
fn frame(body: &mut Vec<u8>, field: &[u8]) {
    body.extend_from_slice(&(field.len() as u64).to_be_bytes());
    body.extend_from_slice(field);
}

/// Lowercase hexadecimal of `bytes`.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The SHA-256 of `bytes` as lowercase hex.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// The key material of one OpenSSH public-key line: kind and base64 body,
/// comment stripped. Key identity compares this, never the comment.
fn key_material(line: &str) -> String {
    let mut parts = line.split_whitespace();
    match (parts.next(), parts.next()) {
        (Some(kind), Some(body)) => format!("{kind} {body}"),
        _ => line.trim().to_owned(),
    }
}

/// Whether `line` is one well-formed `ssh-ed25519` public key: the kind, a
/// base64 body that decodes to an Ed25519 public-key blob, and an optional
/// trailing comment.
fn is_ed25519_public_line(line: &str) -> bool {
    let mut parts = line.split_whitespace();
    let Some(kind) = parts.next() else {
        return false;
    };
    if kind != "ssh-ed25519" {
        return false;
    }
    let Some(body) = parts.next() else {
        return false;
    };
    let Some(blob) = base64_decode(body) else {
        return false;
    };
    is_ed25519_blob(&blob)
}

/// Whether `blob` is the SSH wire form of one Ed25519 public key: a
/// length-prefixed `ssh-ed25519` followed by a length-prefixed 32-byte key,
/// with no trailing bytes.
fn is_ed25519_blob(blob: &[u8]) -> bool {
    ssh_string(blob, 0)
        .map(|(kind, rest)| {
            kind == b"ssh-ed25519"
                && ssh_string(rest, 0).is_some_and(|(key, tail)| key.len() == 32 && tail.is_empty())
        })
        .unwrap_or(false)
}

/// Read one SSH length-prefixed string at `offset`, returning it with the
/// remaining bytes.
fn ssh_string(bytes: &[u8], offset: usize) -> Option<(&[u8], &[u8])> {
    if bytes.len() < offset + 4 {
        return None;
    }
    let length = u32::from_be_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]) as usize;
    let start = offset + 4;
    let end = start.checked_add(length)?;
    if bytes.len() < end {
        return None;
    }
    Some((&bytes[start..end], &bytes[end..]))
}

/// Decode standard-alphabet base64 with padding. Any other character, or
/// malformed padding, refuses.
fn base64_decode(text: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = text.as_bytes();
    if bytes.is_empty() || !bytes.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        let mut indexes = [0u8; 4];
        let mut padding = 0;
        for (slot, byte) in indexes.iter_mut().zip(chunk) {
            if *byte == b'=' {
                padding += 1;
                *slot = 0;
                continue;
            }
            if padding > 0 {
                return None;
            }
            *slot = ALPHABET.iter().position(|candidate| candidate == byte)? as u8;
        }
        let word = (u32::from(indexes[0]) << 18)
            | (u32::from(indexes[1]) << 12)
            | (u32::from(indexes[2]) << 6)
            | u32::from(indexes[3]);
        // The 24 payload bits live in the low half of the word: emit the
        // low three bytes, less any padding.
        let triplet = word.to_be_bytes();
        out.extend_from_slice(&triplet[1..4 - padding as usize]);
    }
    Some(out)
}

/// The public key embedded in one OpenSSH detached signature's armor, as
/// SSH wire bytes - the same bytes the base64 body of a declared key line
/// decodes to. Attribution compares this material, never a signer field.
pub(crate) fn signature_key_material(signature: &[u8]) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(signature).ok()?;
    let mut lines = text.lines();
    let first = lines.next()?;
    if first.trim_end() != "-----BEGIN SSH SIGNATURE-----" {
        return None;
    }
    let mut armor = String::new();
    for line in lines {
        let line = line.trim_end();
        if line == "-----END SSH SIGNATURE-----" {
            break;
        }
        if line.starts_with("-----") || line.contains(':') {
            continue;
        }
        armor.push_str(line.trim());
    }
    let blob = base64_decode(&armor)?;
    // SSHSIG wire: the 6-byte magic, a uint32 version, then the signed
    // public key as an SSH string.
    if blob.len() < 10 || &blob[..6] != b"SSHSIG" {
        return None;
    }
    ssh_string(&blob[10..], 0).map(|(public_key, _)| public_key.to_vec())
}

// --- the enrollment boundary ---------------------------------------------------

/// The enrollment facts a verification or admission consumes.
#[derive(Clone, Debug)]
pub(crate) struct Enrollment {
    /// The SHA-256 of the exact manifest bytes.
    pub manifest_sha256: String,
    /// The bound allowed-signers file.
    pub signers_path: PathBuf,
    /// The bound verifier executable.
    pub verifier_path: PathBuf,
    /// How this enrollment was read: `host-enrolled`, or `test-injected`
    /// under the library seam.
    pub origin: &'static str,
}

/// Read and validate the enrollment the declaration addresses: the trust
/// origin every governed boundary and admission requires. Every refusal
/// detail is one shared string the calling surfaces compose.
pub(crate) fn read_enrollment(
    engine_root: &Path,
    declaration: &ReviewDeclaration,
    project_scope: &str,
) -> Result<Enrollment, String> {
    #[cfg(feature = "test-fault-injection")]
    if let Some(trust) = test::lookup(engine_root) {
        let enrollment = Enrollment {
            manifest_sha256: sha256_hex(&trust.manifest_bytes),
            signers_path: trust.signers_path.clone(),
            verifier_path: trust.verifier_path.clone(),
            origin: "test-injected",
        };
        let manifest = parse_manifest(&trust.manifest_bytes)
            .map_err(|detail| format!("the enrollment manifest is malformed: {detail}"))?;
        agree(&manifest, declaration, project_scope, &enrollment)?;
        verify_digests(&manifest, declaration, &enrollment, &trust.verifier_sha256)?;
        return Ok(enrollment);
    }
    let _ = engine_root;
    let manifest_path = PathBuf::from(&declaration.manifest);
    let manifest_bytes = fs::read(&manifest_path).map_err(|_| {
        format!(
            "the enrollment manifest {} is missing or unreadable",
            declaration.manifest
        )
    })?;
    let actor = builder_actor();
    let manifest = parse_manifest(&manifest_bytes).map_err(|detail| {
        format!(
            "the enrollment manifest {} is malformed: {detail}",
            declaration.manifest
        )
    })?;
    // The ticket's enumeration order: missing or unreadable, then the
    // protection of the manifest and the signers file, then the actor,
    // scope, roles, and keys, then the digests.
    protection::require_protected(&manifest_path, &actor).map_err(|refusal| match refusal {
        protection::ProtectionRefusal::NotProtected => format!(
            "the enrollment manifest {} is not protected against the builder actor {actor}",
            declaration.manifest
        ),
        protection::ProtectionRefusal::Unverifiable(detail) => detail,
    })?;
    protection::require_protected(&manifest.signers_path, &actor).map_err(
        |refusal| match refusal {
            protection::ProtectionRefusal::NotProtected => format!(
                "the allowed-signers file {} is not protected against the builder actor {actor}",
                manifest.signers_path.displayed()
            ),
            protection::ProtectionRefusal::Unverifiable(detail) => detail,
        },
    )?;
    let enrollment = Enrollment {
        manifest_sha256: sha256_hex(&manifest_bytes),
        signers_path: manifest.signers_path.clone(),
        verifier_path: manifest.verifier_path.clone(),
        origin: "host-enrolled",
    };
    agree(&manifest, declaration, project_scope, &enrollment)?;
    verify_digests(
        &manifest,
        declaration,
        &enrollment,
        &manifest.verifier_digest,
    )?;
    Ok(enrollment)
}

/// One parsed enrollment manifest.
pub(crate) struct Manifest {
    pub(crate) project_scope: String,
    pub(crate) policy_digest: String,
    pub(crate) minimum_reviews: u64,
    pub(crate) approver: String,
    pub(crate) approver_may_review: bool,
    pub(crate) verifier_path: PathBuf,
    pub(crate) verifier_digest: String,
    pub(crate) signers_path: PathBuf,
    pub(crate) signers_digest: String,
    pub(crate) builder_actor: String,
    pub(crate) builder_keys: Vec<String>,
    pub(crate) reviewers: Vec<(String, String)>,
}

/// The enrolled key-line material of one label from any reviewers list.
pub(crate) fn reviewer_material_of(reviewers: &[(String, String)], label: &str) -> Option<String> {
    reviewers
        .iter()
        .find(|(name, _)| name == label)
        .map(|(_, material)| material.clone())
}

/// Parse the strict enrollment manifest grammar. Unknown, missing, or
/// mistyped keys refuse.
pub(crate) fn parse_manifest(bytes: &[u8]) -> Result<Manifest, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("the manifest is not UTF-8: {error}"))?;
    let document: toml::Value = text
        .parse()
        .map_err(|error| format!("the manifest is not strict TOML: {error}"))?;
    let root = document
        .as_table()
        .ok_or("the manifest must be a table".to_owned())?;
    const KEYS: [&str; 11] = [
        "project-scope",
        "policy-digest",
        "minimum-reviews",
        "approver",
        "approver-may-review",
        "verifier-path",
        "verifier-digest",
        "signers-path",
        "signers-digest",
        "builder",
        "reviewers",
    ];
    for key in KEYS {
        if !root.contains_key(key) {
            return Err(format!("the manifest is missing {key:?}"));
        }
    }
    for key in root.keys() {
        if !KEYS.contains(&key.as_str()) {
            return Err(format!("the manifest carries unknown key {key:?}"));
        }
    }
    let string_of = |key: &str| -> Result<String, String> {
        root.get(key)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
            .ok_or(format!("{key} must be a string"))
    };
    let digest_of = |key: &str| -> Result<String, String> {
        let value = string_of(key)?;
        if value.len() == 64
            && value
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        {
            Ok(value)
        } else {
            Err(format!("{key} must be 64 lowercase hex digits"))
        }
    };
    let minimum_reviews = root
        .get("minimum-reviews")
        .and_then(toml::Value::as_integer)
        .ok_or("minimum-reviews must be an integer".to_owned())?;
    let minimum_reviews = u64::try_from(minimum_reviews)
        .map_err(|_| "minimum-reviews must be at least 1".to_owned())?;
    if minimum_reviews < 1 {
        return Err("minimum-reviews must be at least 1".to_owned());
    }
    let approver_may_review = root
        .get("approver-may-review")
        .and_then(toml::Value::as_bool)
        .ok_or("approver-may-review must be a boolean".to_owned())?;
    let builder = root
        .get("builder")
        .and_then(toml::Value::as_table)
        .ok_or("builder must be a table".to_owned())?;
    for key in builder.keys() {
        if key != "actor" && key != "keys" {
            return Err(format!("the builder table carries unknown key {key:?}"));
        }
    }
    let builder_actor = builder
        .get("actor")
        .and_then(toml::Value::as_str)
        .ok_or("builder.actor must be a string".to_owned())?
        .to_owned();
    for key in ["project-scope", "verifier-path", "signers-path"] {
        let value = string_of(key)?;
        if !std::path::Path::new(&value).is_absolute() {
            return Err(format!("{key} must be an absolute path"));
        }
    }
    let builder_keys = builder
        .get("keys")
        .and_then(toml::Value::as_array)
        .ok_or("builder.keys must be an array".to_owned())?
        .iter()
        .map(|key| {
            key.as_str()
                .map(str::to_owned)
                .ok_or("builder.keys entries must be strings".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    for key in &builder_keys {
        if !is_ed25519_public_line(key) {
            return Err("builder.keys entries must be ssh-ed25519 public-key lines".to_owned());
        }
    }
    let rows = root
        .get("reviewers")
        .and_then(toml::Value::as_array)
        .ok_or("reviewers must be an array of tables".to_owned())?;
    let mut reviewers = Vec::new();
    for row in rows {
        let row = row
            .as_table()
            .ok_or("reviewers entries must be tables".to_owned())?;
        if row.len() != 2 || !row.contains_key("label") || !row.contains_key("key") {
            return Err("a reviewers entry must carry exactly label and key".to_owned());
        }
        let label = row
            .get("label")
            .and_then(toml::Value::as_str)
            .ok_or("a reviewer label must be a string".to_owned())?
            .to_owned();
        let key = row
            .get("key")
            .and_then(toml::Value::as_str)
            .ok_or("a reviewer key must be a string".to_owned())?
            .to_owned();
        reviewers.push((label, key));
    }
    reviewers.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(Manifest {
        project_scope: string_of("project-scope")?,
        policy_digest: digest_of("policy-digest")?,
        minimum_reviews,
        approver: string_of("approver")?,
        approver_may_review,
        verifier_path: PathBuf::from(string_of("verifier-path")?),
        verifier_digest: digest_of("verifier-digest")?,
        signers_path: PathBuf::from(string_of("signers-path")?),
        signers_digest: digest_of("signers-digest")?,
        builder_actor,
        builder_keys,
        reviewers,
    })
}

/// The declared and enrolled facts must agree, and no enrolled builder key
/// may sit among the reviewers or as the Approver.
fn agree(
    manifest: &Manifest,
    declaration: &ReviewDeclaration,
    project_scope: &str,
    enrollment: &Enrollment,
) -> Result<(), String> {
    let _ = enrollment;
    if manifest.builder_actor != builder_actor() {
        return Err(format!(
            "the enrollment manifest binds the builder actor {:?}; this process is {:?}",
            manifest.builder_actor,
            builder_actor()
        ));
    }
    if manifest.project_scope != project_scope {
        return Err(format!(
            "the enrollment manifest binds the project scope {}; this checkout is {}",
            manifest.project_scope, project_scope
        ));
    }
    if manifest.approver != declaration.approver()
        || manifest.minimum_reviews != declaration.minimum_reviews()
        || manifest.approver_may_review != declaration.approver_may_review()
        || manifest.reviewers != declaration.reviewers()
    {
        return Err(format!(
            "the enrollment manifest {} disagrees with the declared review roles",
            declaration.manifest()
        ));
    }
    // The Approver names the defect first when the builder key sits in
    // both chairs.
    let approver_material = declaration.reviewer_material(declaration.approver());
    if let Some(material) = approver_material {
        if manifest
            .builder_keys
            .iter()
            .any(|builder_key| key_material(builder_key) == material)
        {
            return Err(format!(
                "the approver {:?} carries the builder's enrolled key",
                declaration.approver()
            ));
        }
    }
    for (label, key) in declaration.reviewers() {
        if label == declaration.approver() {
            continue;
        }
        let material = key_material(key);
        if manifest
            .builder_keys
            .iter()
            .any(|builder_key| key_material(builder_key) == material)
        {
            return Err(format!(
                "the reviewer {label:?} carries the builder's enrolled key"
            ));
        }
    }
    let recomputed = policy_digest(
        declaration.approver(),
        declaration.minimum_reviews(),
        declaration.approver_may_review(),
        declaration.reviewers(),
    );
    if recomputed != declaration.policy_digest() {
        return Err(format!(
            "the declared policy digest {} does not match the recomputed {}",
            declaration.policy_digest(),
            recomputed
        ));
    }
    if declaration.policy_digest() != manifest.policy_digest {
        return Err(format!(
            "the declared policy digest {} does not match the enrollment's {}",
            declaration.policy_digest(),
            manifest.policy_digest
        ));
    }
    Ok(())
}

/// The verifier and signers digest checks, in the contract's order: the
/// bound verifier file's actual hash against the binding, then the declared
/// digest against the binding, then the same two for the signers file.
fn verify_digests(
    manifest: &Manifest,
    declaration: &ReviewDeclaration,
    enrollment: &Enrollment,
    verifier_binding: &str,
) -> Result<(), String> {
    crate::observe::operation("target");
    let verifier_real = file_sha256(&enrollment.verifier_path).map_err(|error| {
        format!(
            "the verifier at {} cannot be read: {error}",
            enrollment.verifier_path.displayed()
        )
    })?;
    let verifier_bound = if verifier_binding.is_empty() {
        manifest.verifier_digest.as_str()
    } else {
        verifier_binding
    };
    if verifier_real != verifier_bound {
        return Err(format!(
            "the verifier at {} hashes {}, the enrollment binds {}",
            enrollment.verifier_path.displayed(),
            verifier_real,
            verifier_bound
        ));
    }
    if declaration.verifier_digest() != verifier_bound {
        return Err(format!(
            "the declared verifier digest {} does not match the enrollment's {}",
            declaration.verifier_digest(),
            verifier_bound
        ));
    }
    crate::observe::operation("target");
    let signers_real = file_sha256(&enrollment.signers_path).map_err(|error| {
        format!(
            "the allowed-signers file at {} cannot be read: {error}",
            enrollment.signers_path.displayed()
        )
    })?;
    if signers_real != manifest.signers_digest {
        return Err(format!(
            "the allowed-signers file at {} hashes {}, the enrollment binds {}",
            enrollment.signers_path.displayed(),
            signers_real,
            manifest.signers_digest
        ));
    }
    if declaration.signers_digest() != manifest.signers_digest {
        return Err(format!(
            "the declared signers digest {} does not match the enrollment's {}",
            declaration.signers_digest(),
            manifest.signers_digest
        ));
    }
    crate::observe::operation("target");
    Ok(())
}

pub(crate) fn file_sha256(path: &Path) -> std::io::Result<String> {
    Ok(sha256_hex(&fs::read(path)?))
}

/// The operating-system identity the Engine renders as the builder actor:
/// `DOMAIN\name` on Windows, the account name on Unix.
/// The OS identity of the running process - never an environment variable,
/// which a builder can forge. On Windows the process token's user SID is
/// looked up to its `DOMAIN\\name` account spelling; on Unix the effective
/// uid's account name is read through `getpwuid_r` with no dependency
/// beyond libc.
pub fn builder_actor() -> String {
    #[cfg(windows)]
    return windows_builder_actor();
    #[cfg(not(windows))]
    return unix_builder_actor();
}

/// The token user's `DOMAIN\\name`, or an explicit unknown marker when the
/// OS cannot name it - a refusal then reports honestly instead of trusting
/// caller-controlled state.
#[cfg(windows)]
fn windows_builder_actor() -> String {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, LookupAccountSidW, TokenUser, TOKEN_QUERY, TOKEN_USER,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let unknown = || "an actor the host cannot name".to_owned();
    // SAFETY: GetCurrentProcess returns the current pseudo-handle.
    let process = unsafe { GetCurrentProcess() };
    let mut token = HANDLE::default();
    // SAFETY: `token` is a valid out-pointer for a handle.
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
        return unknown();
    }
    let mut returned = 0u32;
    // SAFETY: a null buffer is the documented size query.
    unsafe { GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut returned) };
    let mut buffer = vec![0u8; returned as usize];
    // SAFETY: `buffer` is the size the query demanded and `token` was
    // opened with TOKEN_QUERY.
    let read = unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr() as *mut core::ffi::c_void,
            returned,
            &mut returned,
        )
    };
    // SAFETY: the handle is no longer needed.
    unsafe { CloseHandle(token) };
    if read == 0 || buffer.len() < std::mem::size_of::<TOKEN_USER>() {
        return unknown();
    }
    // SAFETY: a successful TokenUser query left a TOKEN_USER in `buffer`.
    let user = unsafe { &*(buffer.as_ptr() as *const TOKEN_USER) };
    let mut name = [0u16; 256];
    let mut name_len = name.len() as u32;
    let mut domain = [0u16; 256];
    let mut domain_len = domain.len() as u32;
    let mut account_kind: windows_sys::Win32::Security::SID_NAME_USE = 0;
    // SAFETY: `user.User.Sid` came from the token; the arrays and lengths
    // are valid LookupAccountSidW slots.
    let found = unsafe {
        LookupAccountSidW(
            std::ptr::null(),
            user.User.Sid,
            name.as_mut_ptr(),
            &mut name_len,
            domain.as_mut_ptr(),
            &mut domain_len,
            &mut account_kind,
        )
    };
    if found == 0 {
        return unknown();
    }
    let name = String::from_utf16_lossy(&name[..name_len as usize]);
    let domain = String::from_utf16_lossy(&domain[..domain_len as usize]);
    if domain.is_empty() {
        name
    } else {
        format!("{domain}\\{name}")
    }
}

/// The effective uid's account name via `getpwuid_r`. Only `pw_name`, the
/// first field of `struct passwd` on Linux, macOS, and the BSDs, is read;
/// the oversized 8-byte-aligned buffer absorbs any system layout.
#[cfg(not(windows))]
fn unix_builder_actor() -> String {
    #[repr(C)]
    struct Passwd {
        pw_name: *const core::ffi::c_char,
    }
    extern "C" {
        fn geteuid() -> u32;
        fn getpwuid_r(
            uid: u32,
            passwd: *mut Passwd,
            buffer: *mut core::ffi::c_char,
            buffer_len: usize,
            result: *mut *mut Passwd,
        ) -> i32;
    }
    let unknown = || "an actor the host cannot name".to_owned();
    let mut passwd = Passwd {
        pw_name: std::ptr::null(),
    };
    // 8-byte-aligned and generously oversized: `struct passwd` and its
    // strings must both fit for any of the supported systems.
    let mut buffer = [0u64; 512];
    let mut found: *mut Passwd = std::ptr::null_mut();
    let status = unsafe {
        getpwuid_r(
            geteuid(),
            &mut passwd,
            buffer.as_mut_ptr() as *mut core::ffi::c_char,
            std::mem::size_of_val(&buffer),
            &mut found,
        )
    };
    if status != 0 || found.is_null() || passwd.pw_name.is_null() {
        return unknown();
    }
    // SAFETY: a successful lookup left a NUL-terminated name.
    let name = unsafe { std::ffi::CStr::from_ptr(passwd.pw_name) };
    // The one component-to-text home renders it; on Unix a name holds no
    // separator, so this is the lossy bytes verbatim.
    use std::os::unix::ffi::OsStrExt;
    crate::root::component(std::ffi::OsStr::from_bytes(name.to_bytes()))
}

// --- the canonical intent --------------------------------------------------------

/// The varying facts of one motion an intent binds.
#[derive(Clone, Debug)]
pub(crate) struct IntentFacts {
    pub purpose: String,
    pub run_id: String,
    pub run_dir_path: PathBuf,
    pub engine_root_path: PathBuf,
    pub owning_class: String,
    pub workspace: String,
    pub bindings: Option<BTreeMap<String, String>>,
    pub sequence: u64,
    /// WRS-003: the resume purpose's hold occurrence, canonical decimal;
    /// empty for t-129's own purposes.
    pub hold_occurrence: String,
    /// WRS-003: the resume purpose's held-record digest; empty for
    /// t-129's own purposes.
    pub held_record_digest: String,
    pub from_state: String,
    pub to_state: String,
    pub input: String,
    pub blocker: String,
    pub runbook_sha256: String,
    pub goal_identity: String,
    pub policy_digest: String,
    pub enrollment_sha256: String,
    pub builder_actor: String,
    pub work_digest: String,
    /// WRS-004: the effective amendment head digest the intent binds;
    /// empty while no chain exists for t-129's own purposes.
    pub previous_amendment_digest: String,
    /// WRS-004: the recovery purpose's derived new-authority digest.
    pub proposed_authority_digest: String,
    /// WRS-004: the recovery purpose's derived change-set digest.
    pub proposed_change_digest: String,
    pub review_set_digest: String,
}

impl IntentFacts {
    /// The addressed Run's directory.
    pub(crate) fn run_dir(&self) -> &Path {
        self.run_dir_path.as_path()
    }

    /// The Engine root the Run lives under.
    pub(crate) fn engine_root(&self) -> &Path {
        self.engine_root_path.as_path()
    }

    /// The canonical bytes: the prefix, one NUL, then every field framed in
    /// the fixed order, empty when inapplicable and never omitted.
    pub(crate) fn canonical_bytes(&self, project_scope: &str, engine_identity: &str) -> Vec<u8> {
        let binding_digest = match &self.bindings {
            None => String::new(),
            Some(bindings) => {
                let mut joined: Vec<String> = bindings
                    .iter()
                    .map(|(name, value)| format!("{name}={value}"))
                    .collect();
                joined.sort();
                let mut body = Vec::new();
                for pair in joined {
                    frame(&mut body, pair.as_bytes());
                }
                sha256_hex(&body)
            }
        };
        let mut bytes = INTENT_PREFIX.to_vec();
        bytes.push(0);
        frame(&mut bytes, self.purpose.as_bytes());
        frame(&mut bytes, project_scope.as_bytes());
        frame(&mut bytes, self.run_id.as_bytes());
        frame(&mut bytes, self.owning_class.as_bytes());
        frame(&mut bytes, self.workspace.as_bytes());
        frame(&mut bytes, binding_digest.as_bytes());
        frame(&mut bytes, self.sequence.to_string().as_bytes());
        frame(&mut bytes, self.hold_occurrence.as_bytes());
        frame(&mut bytes, self.held_record_digest.as_bytes());
        frame(&mut bytes, self.from_state.as_bytes());
        frame(&mut bytes, self.to_state.as_bytes());
        frame(&mut bytes, self.input.as_bytes());
        frame(&mut bytes, self.blocker.as_bytes());
        frame(&mut bytes, engine_identity.as_bytes());
        frame(&mut bytes, self.runbook_sha256.as_bytes());
        frame(&mut bytes, self.goal_identity.as_bytes());
        frame(&mut bytes, self.policy_digest.as_bytes());
        frame(&mut bytes, self.enrollment_sha256.as_bytes());
        frame(&mut bytes, self.builder_actor.as_bytes());
        frame(&mut bytes, self.work_digest.as_bytes());
        frame(&mut bytes, self.previous_amendment_digest.as_bytes());
        frame(&mut bytes, self.proposed_authority_digest.as_bytes());
        frame(&mut bytes, self.proposed_change_digest.as_bytes());
        frame(&mut bytes, self.review_set_digest.as_bytes());
        bytes
    }
}

/// The current work digest: the legacy goal-revision calculation over the
/// declared work paths - every file beneath them, collected recursively and
/// sorted by its displayed path relative to its declared root, hashed as
/// relative-path bytes, one LF, the file's bytes, one LF. No declared work
/// hashes the empty input.
pub(crate) fn work_digest(workspace: &Path, work: &[String]) -> Result<String, String> {
    let mut files = Vec::new();
    for declared in work {
        let root = workspace.join(declared);
        // Mirror the goal-freeze collection exactly: recurse directories,
        // read every non-directory entry (links followed), and accept a
        // declared root that is itself a regular file under its declared
        // path. Only a root that is neither file nor directory refuses.
        let metadata = fs::metadata(&root)
            .map_err(|error| format!("cannot read declared work {}: {error}", root.displayed()))?;
        if metadata.is_file() {
            let bytes = fs::read(&root).map_err(|error| {
                format!("cannot read declared work {}: {error}", root.displayed())
            })?;
            files.push((declared.clone(), bytes));
        } else if metadata.is_dir() {
            gather_files(&root, &root, &mut files)?;
        } else {
            return Err(format!(
                "cannot inspect declared work {}: it is neither a file nor a directory",
                root.displayed()
            ));
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hasher = Sha256::new();
    for (relative, bytes) in &files {
        hasher.update(relative.as_bytes());
        hasher.update(b"\n");
        hasher.update(bytes);
        hasher.update(b"\n");
    }
    Ok(hex(&hasher.finalize()))
}

fn gather_files(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|error| format!("cannot read declared work {}: {error}", dir.displayed()))?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "cannot enumerate declared work {}: {error}",
                dir.displayed()
            )
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            format!("cannot inspect declared work {}: {error}", path.displayed())
        })?;
        if file_type.is_dir() {
            gather_files(base, &path, out)?;
            continue;
        }
        let relative = path.strip_prefix(base).unwrap_or(&path);
        out.push((
            relative.displayed(),
            fs::read(&path).map_err(|error| {
                format!("cannot read declared work {}: {error}", path.displayed())
            })?,
        ));
    }
    Ok(())
}

// --- the verification boundary --------------------------------------------------

/// The authorization options `rtm step`/`rtm hold` carry: one signature
/// file per review label, and the Approver's approval signature file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AuthorizationOptions {
    pub reviews: Vec<(String, String)>,
    pub approve: Option<String>,
    /// How many --approve options the invocation carried: the boundary
    /// refuses a repeated approval option.
    pub approval_count: usize,
}

impl AuthorizationOptions {
    pub(crate) fn is_empty(&self) -> bool {
        self.reviews.is_empty() && self.approve.is_none()
    }
}

/// The Engine identity a canonical intent binds: the running executable's
/// SHA-256. In-process library invocations hash the caller's own image,
/// exactly as the shipped binary hashes its.
pub(crate) fn engine_identity_sha256() -> Result<String, String> {
    let path = std::env::current_exe()
        .map_err(|error| format!("cannot locate the running executable: {error}"))?;
    file_sha256(&path).map_err(|error| format!("hash the running executable: {error}"))
}

/// One accepted authorization, ready to archive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerifiedAuthorization {
    pub sequence: u64,
    pub intent_sha256: String,
    pub policy_sha256: String,
    pub approval_signature_sha256: String,
    /// Each accepted review as (label, signature-file digest), in the
    /// order the reviews were accepted.
    pub reviews: Vec<(String, String)>,
    pub origin: &'static str,
    pub enrollment_sha256: String,
}

/// One structured refusal from the verification boundary: the kind,
/// observed fact, and expected fact, composed by the calling surfaces.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AuthorizationRefusal {
    /// The refusal's kind, already carrying its `authorization: ` surface
    /// prefix, so a GuardFailure renders `authorization: <kind>: observed
    /// ...; expected ...`.
    pub kind: String,
    pub observed: String,
    pub expected: String,
}

fn refuse(
    kind: impl Into<String>,
    observed: impl Into<String>,
    expected: impl Into<String>,
) -> AuthorizationRefusal {
    AuthorizationRefusal {
        kind: kind.into(),
        observed: observed.into(),
        expected: expected.into(),
    }
}

/// The one verification boundary. `facts` carries the motion's canonical
/// fields with `review_set_digest` still empty; both intents are derived
/// inside. Returns the structured refusal on refusal.
pub(crate) fn verify(
    declaration: &ReviewDeclaration,
    enrollment: &Enrollment,
    project_scope: &str,
    facts: &IntentFacts,
    options: &AuthorizationOptions,
) -> Result<VerifiedAuthorization, AuthorizationRefusal> {
    let run_id = facts.run_id.as_str();
    let minimum = declaration.minimum_reviews();
    let expected_line = format!(
        "{minimum} distinct review signature(s) and one approval signature over the \
         canonical intent (rtm intent --run {run_id})"
    );

    // The option-presence refusals come first: a governed Run offering no
    // signatures at all never reaches the per-label checks.
    if options.approve.is_none() || options.reviews.is_empty() {
        return Err(refuse("signatures", "none", expected_line));
    }
    // One approval is the contract: a repeated --approve option refuses
    // before any signature is read.
    if options.approval_count > 1 {
        return Err(refuse(
            "signatures",
            format!("--approve given {} times", options.approval_count),
            "exactly one --approve option naming the approver's signature",
        ));
    }
    let approval_path = options.approve.clone().expect("presence checked above");

    // A duplicate label refuses before any signature is verified.
    let mut seen: Vec<&str> = Vec::new();
    for (label, _) in &options.reviews {
        if seen.contains(&label.as_str()) {
            return Err(refuse(
                "reviews",
                format!("label {label:?} given twice"),
                "each enrolled reviewer at most once",
            ));
        }
        seen.push(label);
    }
    let approval_bytes = fs::read(&approval_path).map_err(|error| {
        refuse(
            "approval",
            format!(
                "an unreadable signature at {}: {error}",
                PathBuf::from(&approval_path).displayed()
            ),
            "the approver's ratmac-approve-v1 signature over the canonical intent",
        )
    })?;
    let approval_sha = sha256_hex(&approval_bytes);
    let run_dir = facts.run_dir().to_path_buf();
    let consumed = archived_approval_at(&run_dir, &approval_sha)
        .map_err(|detail| refuse("replay", detail, "a readable authorization archive"))?;
    if let Some(consumed) = consumed {
        return Err(refuse(
            "replay",
            format!(
                "the approval already consumed as sequence {consumed} under \
                 {}/runs/{}/authorizations/",
                facts.engine_root().displayed(),
                run_id
            ),
            format!("a fresh authorization at sequence {}", consumed + 1),
        ));
    }

    // Reviews verify under their enrolled key over the recomputed review
    // intent, in submission order.
    let engine_identity = engine_identity_sha256()
        .map_err(|detail| refuse("engine", detail, "a readable running executable"))?;
    let review_intent = facts.canonical_bytes(project_scope, &engine_identity);
    let mut accepted: Vec<(String, String, Vec<u8>)> = Vec::new();
    for (label, signature) in &options.reviews {
        let signature_path = PathBuf::from(signature);
        let bytes = fs::read(&signature_path).map_err(|error| {
            refuse(
                "review",
                format!(
                    "an unreadable signature at {}: {error}",
                    signature_path.displayed()
                ),
                "a ratmac-review-v1 signature by an enrolled reviewer",
            )
        })?;
        // Ticket lines 268-275: each review verifies under its submitted
        // label's enrolled key, judged by exit status over the recomputed
        // review intent.
        if !ssh_verify(
            &enrollment.verifier_path,
            &enrollment.signers_path,
            label,
            REVIEW_NAMESPACE,
            &review_intent,
            &bytes,
        ) {
            return Err(refuse(
                "review",
                format!(
                    "a signature at label {label:?} that verifies under no \
                     enrolled reviewer key"
                ),
                "a ratmac-review-v1 signature by an enrolled reviewer",
            ));
        }
        accepted.push((label.clone(), sha256_hex(&bytes), bytes));
    }
    let distinct = accepted.len() as u64;
    if distinct < minimum {
        return Err(refuse(
            "reviews",
            format!("{distinct} distinct accepted review(s)"),
            format!("at least {minimum}"),
        ));
    }
    if !declaration.approver_may_review()
        && accepted
            .iter()
            .any(|(label, _, _)| label == declaration.approver())
    {
        return Err(refuse(
            "reviews",
            format!("a review by the approver {:?}", declaration.approver()),
            "reviews by members other than the approver",
        ));
    }

    // The approval intent binds the review-set digest of the exact files.
    let mut approval_facts = facts.clone();
    approval_facts.review_set_digest = review_set_digest(
        &accepted
            .iter()
            .map(|(label, _, bytes)| (label.clone(), bytes.clone()))
            .collect::<Vec<_>>(),
    );
    let approval_intent = approval_facts.canonical_bytes(project_scope, &engine_identity);

    // Approval attribution: verify under the designated Approver first,
    // then under each other enrolled member; both judged by exit status
    // over the recomputed approval intent. When neither verifies, the
    // signature's embedded key material attributes it: the Approver's own
    // key means a stale or wrong-payload approval; any other key means no
    // enrolled identity.
    if !ssh_verify(
        &enrollment.verifier_path,
        &enrollment.signers_path,
        declaration.approver(),
        APPROVE_NAMESPACE,
        &approval_intent,
        &approval_bytes,
    ) {
        let mut member: Option<&str> = None;
        for (label, _) in declaration.reviewers() {
            if label == declaration.approver() {
                continue;
            }
            if ssh_verify(
                &enrollment.verifier_path,
                &enrollment.signers_path,
                label,
                APPROVE_NAMESPACE,
                &approval_intent,
                &approval_bytes,
            ) {
                member = Some(label);
                break;
            }
        }
        if let Some(member) = member {
            return Err(refuse(
                "approval",
                format!("a signature by member {member:?}"),
                format!("the designated approver {:?}", declaration.approver()),
            ));
        }
        let embedded = signature_key_material(&approval_bytes);
        let approver_material = declaration.reviewer_material(declaration.approver());
        if let (Some(embedded), Some(approver_material)) = (embedded, approver_material) {
            if embedded_key_body(&embedded).as_deref() == Some(approver_material.as_str()) {
                return Err(refuse(
                    "approval",
                    format!(
                        "a signature that does not verify against the \
                         recomputed canonical intent sha256 {}",
                        sha256_hex(&approval_intent)
                    ),
                    format!("a fresh approval of that exact intent (rtm intent --run {run_id})"),
                ));
            }
        }
        return Err(refuse(
            "approval",
            "a signature by no enrolled identity",
            format!("the designated approver {:?}", declaration.approver()),
        ));
    }

    Ok(VerifiedAuthorization {
        sequence: facts.sequence,
        intent_sha256: sha256_hex(&approval_intent),
        policy_sha256: declaration.policy_digest().to_owned(),
        approval_signature_sha256: approval_sha,
        reviews: accepted
            .into_iter()
            .map(|(label, sha, _)| (label, sha))
            .collect(),
        origin: enrollment.origin,
        enrollment_sha256: enrollment.manifest_sha256.clone(),
    })
}

/// The declared key line's material, for comparing with an embedded
/// signature key: kind plus the decoded wire bytes.
pub(crate) fn embedded_key_body(embedded: &[u8]) -> Option<String> {
    let (kind, rest) = ssh_string(embedded, 0)?;
    let kind = std::str::from_utf8(kind).ok()?;
    if kind != "ssh-ed25519" {
        return None;
    }
    let (key, tail) = ssh_string(rest, 0)?;
    if key.len() != 32 || !tail.is_empty() {
        return None;
    }
    Some(format!("ssh-ed25519 {}", base64_encode(embedded)))
}

/// Verify one detached signature with the pinned verifier, judged by exit
/// status alone. The payload is fed on stdin; the verifier's own output is
/// discarded.
pub(crate) fn ssh_verify(
    verifier: &Path,
    signers: &Path,
    label: &str,
    namespace: &str,
    payload: &[u8],
    signature: &[u8],
) -> bool {
    let temporary = tempfile_path("ratmac-verify-sig");
    if fs::write(&temporary, signature).is_err() {
        return false;
    }
    let spawned = std::process::Command::new(verifier)
        .arg("-Y")
        .arg("verify")
        .arg("-f")
        .arg(signers)
        .arg("-I")
        .arg(label)
        .arg("-n")
        .arg(namespace)
        .arg("-s")
        .arg(&temporary)
        // t-128: the pinned verifier child never inherits the Engine's
        // trace channel.
        .env_remove(crate::diag::TRACE_VARIABLE)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    let verified = match spawned {
        Ok(mut child) => {
            if let Some(stdin) = child.stdin.as_mut() {
                if stdin.write_all(payload).is_err() {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = fs::remove_file(&temporary);
                    return false;
                }
            }
            drop(child.stdin.take());
            matches!(child.wait_with_output(), Ok(output) if output.status.success())
        }
        Err(_) => false,
    };
    let _ = fs::remove_file(&temporary);
    verified
}

/// One scratch path beside the Engine's own runtime artifacts. Signature
/// files are caller-supplied paths read as bytes; the verifier needs them
/// on disk, so the verification stages them under the system temporary
/// directory.
fn tempfile_path(prefix: &str) -> PathBuf {
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("{prefix}-{}-{sequence}.sig", std::process::id()))
}

/// Standard-alphabet base64 encoding of `bytes`.
fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let mut word = 0u32;
        for (shift, byte) in chunk.iter().enumerate() {
            word |= u32::from(*byte) << (16 - 8 * shift);
        }
        let quad = [
            ALPHABET[(word >> 18) as usize & 63],
            ALPHABET[(word >> 12) as usize & 63],
            ALPHABET[(word >> 6) as usize & 63],
            ALPHABET[word as usize & 63],
        ];
        let kept = match chunk.len() {
            1 => &quad[..2],
            2 => &quad[..3],
            _ => &quad[..4],
        };
        out.push_str(std::str::from_utf8(kept).expect("base64 is ASCII"));
        for _ in chunk.len()..3 {
            out.push('=');
        }
    }
    out
}

/// WRS-003: one accepted governed-resume resolution, ready to archive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerifiedResolution {
    pub sequence: u64,
    pub intent_sha256: String,
    pub policy_sha256: String,
    pub enrollment_sha256: String,
    pub resolution_signature_sha256: String,
    pub occurrence: u64,
    pub origin: &'static str,
}

/// The governed resume's one-signature verification boundary: a single
/// detached `ratmac-resume-v1` signature by the designated Approver over
/// the recomputed canonical resolution intent, never replayed. Attribution
/// runs first under the designated Approver and on failure under every
/// other enrolled label, exactly as the approval boundary attributes.
pub(crate) fn verify_resolution(
    declaration: &ReviewDeclaration,
    enrollment: &Enrollment,
    project_scope: &str,
    facts: &IntentFacts,
    occurrence: u64,
    resolution: &str,
) -> Result<VerifiedResolution, AuthorizationRefusal> {
    let run_id = facts.run_id.as_str();
    let approver = declaration.approver();
    let expected_line = format!(
        "one ratmac-resume-v1 signature by the designated approver over the canonical intent \
         (rtm intent --run {run_id} --resume)"
    );
    let signature_path = PathBuf::from(resolution);
    let signature_bytes = fs::read(&signature_path).map_err(|error| {
        refuse(
            "resolution",
            format!(
                "an unreadable signature at {}: {error}",
                signature_path.displayed()
            ),
            expected_line.clone(),
        )
    })?;
    let signature_sha = sha256_hex(&signature_bytes);
    let run_dir = facts.run_dir().to_path_buf();
    let consumed = archived_resolution_at(&run_dir, &signature_sha)
        .map_err(|detail| refuse("replay", detail, "a readable authorization archive"))?;
    if let Some(consumed) = consumed {
        return Err(refuse(
            "replay",
            format!(
                "the resolution already consumed as sequence {consumed} under \
                 {}/runs/{}/authorizations/",
                facts.engine_root().displayed(),
                run_id
            ),
            format!("a fresh authorization at sequence {}", consumed + 1),
        ));
    }

    let engine_identity = engine_identity_sha256()
        .map_err(|detail| refuse("engine", detail, "a readable running executable"))?;
    let intent = facts.canonical_bytes(project_scope, &engine_identity);
    if !ssh_verify(
        &enrollment.verifier_path,
        &enrollment.signers_path,
        approver,
        RESUME_NAMESPACE,
        &intent,
        &signature_bytes,
    ) {
        let mut member: Option<&str> = None;
        for (label, _) in declaration.reviewers() {
            if label == approver {
                continue;
            }
            if ssh_verify(
                &enrollment.verifier_path,
                &enrollment.signers_path,
                label,
                RESUME_NAMESPACE,
                &intent,
                &signature_bytes,
            ) {
                member = Some(label);
                break;
            }
        }
        if let Some(member) = member {
            return Err(refuse(
                "resolution",
                format!("a signature by member {member:?}"),
                format!("the designated approver {approver:?}"),
            ));
        }
        let embedded = signature_key_material(&signature_bytes);
        let approver_material = declaration.reviewer_material(approver);
        if let (Some(embedded), Some(approver_material)) = (embedded, approver_material) {
            if embedded_key_body(&embedded).as_deref() == Some(approver_material.as_str()) {
                return Err(refuse(
                    "resolution",
                    format!(
                        "a signature that does not verify against the recomputed canonical \
                         intent sha256 {}",
                        sha256_hex(&intent)
                    ),
                    format!(
                        "a fresh ratmac-resume-v1 resolution of that exact intent (rtm intent \
                         --run {run_id} --resume)"
                    ),
                ));
            }
        }
        return Err(refuse(
            "resolution",
            "a signature by no enrolled identity",
            format!("the designated approver {approver:?}"),
        ));
    }

    Ok(VerifiedResolution {
        sequence: facts.sequence,
        intent_sha256: sha256_hex(&intent),
        policy_sha256: declaration.policy_digest().to_owned(),
        enrollment_sha256: enrollment.manifest_sha256.clone(),
        resolution_signature_sha256: signature_sha,
        occurrence,
        origin: enrollment.origin,
    })
}

// --- the authorization archive ---------------------------------------------------

/// The next authorization sequence: one past the highest existing index.
pub(crate) fn next_sequence(run_dir: &Path) -> Result<u64, String> {
    let mut highest = 0u64;
    for (index, _, _) in archived_records(run_dir)? {
        highest = highest.max(index);
    }
    Ok(highest + 1)
}

/// One archived record's kind, by its signature digest key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ArchivedKind {
    Approval,
    Resolution,
    /// WRS-004: a recovery decision record.
    Amendment,
}

/// Every archived record as (sequence, its signature digest, and its
/// kind), sorted by sequence. A record that will not parse refuses.
fn archived_records(run_dir: &Path) -> Result<Vec<(u64, String, ArchivedKind)>, String> {
    crate::observe::operation("record");
    let dir = archive_dir(run_dir);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "read the authorization archive {}: {error}",
                dir.displayed()
            ))
        }
    };
    let mut records = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("read an authorization archive entry: {error}"))?;
        let name = crate::root::component(entry.file_name());
        let Some(stem) = name.strip_suffix(".toml") else {
            return Err(format!(
                "the authorization archive holds unrecognized entry {name:?}"
            ));
        };
        let index = stem.parse::<u64>().map_err(|_| {
            format!("the authorization archive entry {name:?} does not name a sequence")
        })?;
        let text = fs::read_to_string(entry.path()).map_err(|error| {
            format!(
                "read the authorization record {}: {error}",
                entry.path().displayed()
            )
        })?;
        let value: toml::Value = text.parse::<toml::Value>().map_err(|error| {
            // The detail stays one line: a refusal line never carries the
            // parser's multi-line caret diagram.
            format!(
                "the authorization record {} will not parse: {}",
                entry.path().displayed(),
                error.to_string().lines().next().unwrap_or_default()
            )
        })?;
        let approval = value
            .get("approval_signature_sha256")
            .and_then(toml::Value::as_str);
        let resolution = value
            .get("resolution_signature_sha256")
            .and_then(toml::Value::as_str);
        let amendment = value
            .get("decision_signature_sha256")
            .and_then(toml::Value::as_str);
        let (digest, kind) = match (approval, resolution, amendment) {
            (Some(approval), None, None) => (approval.to_owned(), ArchivedKind::Approval),
            (None, Some(resolution), None) => (resolution.to_owned(), ArchivedKind::Resolution),
            (None, None, Some(amendment)) => (amendment.to_owned(), ArchivedKind::Amendment),
            (None, None, None) => {
                // The landed approval/resolution wording stays; only the
                // amendment kind extends the no-digest case.
                if value.get("purpose").and_then(toml::Value::as_str) == Some("ratmac-recover-v1")
                    || value.get("amendment_sequence").is_some()
                {
                    return Err(format!(
                        "the authorization record {} carries no signature digest",
                        entry.path().displayed()
                    ));
                }
                return Err(format!(
                    "the authorization record {} carries no approval signature digest",
                    entry.path().displayed()
                ));
            }
            _ => {
                if amendment.is_some() {
                    return Err(format!(
                        "the authorization record {} carries more than one signature digest",
                        entry.path().displayed()
                    ));
                }
                return Err(format!(
                    "the authorization record {} carries both an approval and a resolution \
                     signature digest",
                    entry.path().displayed()
                ));
            }
        };
        records.push((index, digest, kind));
    }
    records.sort_by_key(|(index, _, _)| *index);
    Ok(records)
}

/// The lowest sequence whose archived approval digest matches an approval
/// record - a record whose digest is an `approval_signature_sha256` - if
/// any.
fn archived_approval_at(run_dir: &Path, approval_sha: &str) -> Result<Option<u64>, String> {
    archived_decision_in(run_dir, approval_sha, ArchivedKind::Approval)
}

/// WRS-004: the lowest sequence whose archived decision digest matches a
/// recovery decision record, if any.
pub(crate) fn archived_decision_at(
    run_dir: &Path,
    decision_sha: &str,
) -> Result<Option<u64>, String> {
    archived_decision_in(run_dir, decision_sha, ArchivedKind::Amendment)
}

/// The lowest sequence whose archived digest of one kind matches, if any.
fn archived_decision_in(
    run_dir: &Path,
    digest_of_interest: &str,
    kind: ArchivedKind,
) -> Result<Option<u64>, String> {
    Ok(archived_records(run_dir)?
        .into_iter()
        .find(|(_, digest, record_kind)| *record_kind == kind && digest == digest_of_interest)
        .map(|(index, _, _)| index))
}

/// WRS-003: the lowest sequence whose archived resolution digest matches a
/// resolution record - a record whose digest is a
/// `resolution_signature_sha256` - if any.
fn archived_resolution_at(run_dir: &Path, resolution_sha: &str) -> Result<Option<u64>, String> {
    archived_decision_in(run_dir, resolution_sha, ArchivedKind::Resolution)
}

fn archive_dir(run_dir: &Path) -> PathBuf {
    run_dir.join("authorizations")
}

/// Render one motion authorization record's exact bytes.
pub(crate) fn authorization_record_text(
    verified: &VerifiedAuthorization,
    purpose: &str,
    from_state: &str,
    to_state: &str,
) -> String {
    let mut text = String::new();
    text.push_str(&format!("sequence = {}\n", verified.sequence));
    text.push_str(&format!("purpose = {}\n", toml_string(purpose)));
    text.push_str(&format!(
        "intent_sha256 = {}\n",
        toml_string(&verified.intent_sha256)
    ));
    text.push_str(&format!(
        "policy_sha256 = {}\n",
        toml_string(&verified.policy_sha256)
    ));
    text.push_str(&format!(
        "enrollment_sha256 = {}\n",
        toml_string(&verified.enrollment_sha256)
    ));
    text.push_str(&format!("from_state = {}\n", toml_string(from_state)));
    text.push_str(&format!("to_state = {}\n", toml_string(to_state)));
    text.push_str(&format!(
        "approval_signature_sha256 = {}\n",
        toml_string(&verified.approval_signature_sha256)
    ));
    text.push_str(&format!("origin = {}\n", toml_string(verified.origin)));
    for (label, digest) in &verified.reviews {
        text.push_str("\n[[reviews]]\n");
        text.push_str(&format!("label = {}\n", toml_string(label)));
        text.push_str(&format!("signature_sha256 = {}\n", toml_string(digest)));
    }
    text
}

/// WRS-003: render one governed resume's resolution record's exact bytes.
pub(crate) fn resolution_record_text(
    resolution: &VerifiedResolution,
    from_state: &str,
    to_state: &str,
) -> String {
    let mut text = String::new();
    text.push_str(&format!("sequence = {}\n", resolution.sequence));
    text.push_str(&format!("purpose = {}\n", toml_string(RESUME_NAMESPACE)));
    text.push_str(&format!(
        "intent_sha256 = {}\n",
        toml_string(&resolution.intent_sha256)
    ));
    text.push_str(&format!(
        "policy_sha256 = {}\n",
        toml_string(&resolution.policy_sha256)
    ));
    text.push_str(&format!(
        "enrollment_sha256 = {}\n",
        toml_string(&resolution.enrollment_sha256)
    ));
    text.push_str(&format!(
        "resolution_signature_sha256 = {}\n",
        toml_string(&resolution.resolution_signature_sha256)
    ));
    text.push_str(&format!("from_state = {}\n", toml_string(from_state)));
    text.push_str(&format!("to_state = {}\n", toml_string(to_state)));
    text.push_str(&format!("occurrence = {}\n", resolution.occurrence));
    text.push_str(&format!("origin = {}\n", toml_string(resolution.origin)));
    text
}

/// Write one authorization record whole in one step: a same-directory
/// temporary, then a rename. Returns the record's path and whether the
/// archive directory was created by this write.
pub(crate) fn write_archive(
    run_dir: &Path,
    verified: &VerifiedAuthorization,
    purpose: &str,
    from_state: &str,
    to_state: &str,
) -> Result<(PathBuf, bool), String> {
    let dir = archive_dir(run_dir);
    let dir_was_absent = match fs::symlink_metadata(&dir) {
        Ok(_) => false,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(error) => {
            return Err(format!(
                "inspect the authorization archive {}: {error}",
                dir.displayed()
            ))
        }
    };
    if dir_was_absent {
        fs::create_dir(&dir).map_err(|error| {
            format!(
                "create the authorization archive {}: {error}",
                dir.displayed()
            )
        })?;
    }
    let sequence = verified.sequence;
    let text = authorization_record_text(verified, purpose, from_state, to_state);
    let path = dir.join(format!("{sequence:06}.toml"));
    let temporary = dir.join(format!(".{sequence:06}.toml.tmp-{}", std::process::id()));
    fs::write(&temporary, text.as_bytes()).map_err(|error| {
        format!(
            "write the authorization record {}: {error}",
            temporary.displayed()
        )
    })?;
    fs::rename(&temporary, &path).map_err(|error| {
        format!(
            "publish the authorization record {}: {error}",
            path.displayed()
        )
    })?;
    crate::observe::operation("record");
    Ok((path, dir_was_absent))
}

/// Remove one authorization record this invocation just wrote, restoring an
/// absent archive directory when it created it.
pub(crate) fn remove_archive(
    run_dir: &Path,
    path: &Path,
    dir_was_absent: bool,
) -> Result<(), String> {
    fs::remove_file(path).map_err(|error| {
        format!(
            "remove the authorization record {}: {error}",
            path.displayed()
        )
    })?;
    if dir_was_absent {
        if let Err(error) = fs::remove_dir(archive_dir(run_dir)) {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(format!(
                    "remove the emptied authorization archive {}: {error}",
                    archive_dir(run_dir).displayed()
                ));
            }
        }
    }
    Ok(())
}

fn toml_string(value: &str) -> String {
    format!("{value:?}")
}

// --- protection ------------------------------------------------------------------

/// The production trust origin's protection check. A builder-writable
/// manifest or signers file is not an enrollment; a path whose protection
/// cannot be verified is reported as exactly that.
pub(crate) mod protection;

/// Test-only trust injection, compiled only with `test-fault-injection`.
///
/// The shipped command never consults it and no environment variable
/// reaches it: the map is keyed by the canonicalized Engine root an
/// in-process invocation resolves, and its guard uninstalls it on drop.
#[cfg(feature = "test-fault-injection")]
pub mod test {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};
    use std::sync::{LazyLock, Mutex};

    /// Fixture trust facts that stand in for an operator-protected
    /// enrollment: the manifest's exact bytes, and the real host paths and
    /// verifier digest that substitute for the manifest's `signers-path`,
    /// `verifier-path`, and `verifier-digest` bindings. Every other
    /// manifest field binds as in production; only the protection check is
    /// skipped.
    pub struct TestTrust {
        pub manifest_bytes: Vec<u8>,
        pub signers_path: PathBuf,
        pub verifier_path: PathBuf,
        pub verifier_sha256: String,
    }

    /// Uninstalls its trust reader when dropped.
    pub struct TrustGuard {
        engine_root: PathBuf,
    }

    impl Drop for TrustGuard {
        fn drop(&mut self) {
            if let Ok(mut map) = trust_map().lock() {
                if let Some((_, count)) = map.get_mut(&self.engine_root) {
                    *count -= 1;
                    if *count == 0 {
                        map.remove(&self.engine_root);
                    }
                }
            }
        }
    }

    fn trust_map() -> &'static Mutex<BTreeMap<PathBuf, (TestTrust, usize)>> {
        static MAP: LazyLock<Mutex<BTreeMap<PathBuf, (TestTrust, usize)>>> =
            LazyLock::new(|| Mutex::new(BTreeMap::new()));
        &MAP
    }

    /// Resolve one Engine-root spelling to the same directory every other
    /// spelling resolves to: the platform's canonicalized path, with
    /// forward slashes normalized to backslashes first (a `//?/C:/...`
    /// spelling parses as UNC, not verbatim, and canonicalize refuses it),
    /// falling back to the nearest existing ancestor plus the remaining
    /// components for a directory the fixture has not created yet.
    fn resolved(path: &Path) -> PathBuf {
        // `crate::root` owns the one path spelling; a separator-only
        // rewrite through it keeps this normalize inside the rule.
        #[cfg(windows)]
        let candidate = PathBuf::from(
            path.components()
                .map(|c| crate::root::component(c.as_os_str()))
                .collect::<Vec<_>>()
                .join("\\"),
        );
        #[cfg(not(windows))]
        let candidate = path.to_path_buf();
        if let Ok(canonical) = std::fs::canonicalize(&candidate) {
            return canonical;
        }
        for ancestor in candidate.ancestors().skip(1) {
            if let Ok(canonical) = std::fs::canonicalize(ancestor) {
                let remainder = candidate.strip_prefix(ancestor).unwrap_or(&candidate);
                return canonical.join(remainder);
            }
        }
        candidate
    }

    /// Install a trust reader for one Engine root, resolved to the same
    /// directory whatever spelling the caller gives. Concurrent tests with
    /// distinct Engine roots compose; a nested install for the same root
    /// shares the entry, so the trust lives until the last guard drops.
    #[must_use]
    pub fn install(engine_root: &Path, trust: TestTrust) -> TrustGuard {
        // Nested installs for one Engine root count: an inner guard's drop
        // must never retire an outer guard's still-live trust.
        let key = resolved(engine_root);
        if let Ok(mut map) = trust_map().lock() {
            let entry = map.entry(key.clone()).or_insert((trust, 0));
            entry.1 += 1;
        }
        TrustGuard { engine_root: key }
    }

    /// The installed trust for one Engine root, matched as the same
    /// directory whatever its spelling.
    pub(crate) fn lookup(engine_root: &Path) -> Option<TestTrust> {
        let key = resolved(engine_root);
        let Ok(map) = trust_map().lock() else {
            return None;
        };
        map.iter()
            .find(|(installed, _)| resolved(installed) == key)
            .map(|(_, (trust, _))| TestTrust {
                manifest_bytes: trust.manifest_bytes.clone(),
                signers_path: trust.signers_path.clone(),
                verifier_path: trust.verifier_path.clone(),
                verifier_sha256: trust.verifier_sha256.clone(),
            })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn builder_actor_names_the_process_identity() {
        let actor = crate::review::builder_actor();
        assert!(
            !actor.is_empty() && actor != "an actor the host cannot name",
            "the OS must name the running process's identity, got {actor:?}"
        );
        #[cfg(windows)]
        assert!(
            actor.contains('\\'),
            "on Windows the identity is the token user's DOMAIN\\name, got {actor:?}"
        );
    }
}
