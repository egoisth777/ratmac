//! WEB-006 (t-134): the `ratmac-lineage-v1` invocation-lineage record.
//!
//! The lineage is the ordered list of (declared program identity, canonical
//! Engine root) pairs of the programs in one nested call chain, newest last.
//! It travels between Engine processes only as bytes - argv and the
//! environment cannot carry a NUL byte, so both carriers hold the bytes as
//! lowercase hexadecimal. This module is the one codec; nothing else spells
//! the grammar.

/// The record's magic prefix, followed by one NUL byte.
const MAGIC: &[u8] = b"ratmac-lineage-v1";

/// The frozen default maximum lineage depth a declaration may tighten but
/// never loosen.
pub(crate) const DEFAULT_MAX_DEPTH: usize = 16;

/// The grammar an inherited or endpoint-received lineage record must obey,
/// the expected clause a malformed-lineage refusal names.
pub(crate) const GRAMMAR: &str = "ratmac-lineage-v1, one NUL byte, then the entry count as \
     unsigned 64-bit big-endian, then per entry two length-framed UTF-8 strings: the \
     declared program identity and the canonical Engine root, forward slashes";

/// One lineage entry: the declared program identity and the canonical
/// Engine root of one program in the chain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Entry {
    pub(crate) identity: String,
    pub(crate) root: String,
}

impl Entry {
    pub(crate) fn new(identity: impl Into<String>, root: impl Into<String>) -> Self {
        Self {
            identity: identity.into(),
            root: root.into(),
        }
    }
}

fn frame(body: &mut Vec<u8>, field: &[u8]) {
    body.extend_from_slice(&(field.len() as u64).to_be_bytes());
    body.extend_from_slice(field);
}

/// Encode one chain: magic, NUL, the entry count as unsigned 64-bit
/// big-endian, then per entry two length-framed UTF-8 strings.
pub(crate) fn encode(entries: &[Entry]) -> Vec<u8> {
    let mut body = Vec::with_capacity(64);
    body.extend_from_slice(MAGIC);
    body.push(0);
    body.extend_from_slice(&(entries.len() as u64).to_be_bytes());
    for entry in entries {
        frame(&mut body, entry.identity.as_bytes());
        frame(&mut body, entry.root.as_bytes());
    }
    body
}

/// Decode one chain from exact bytes. Trailing bytes, a count below two (the
/// launching parent's entry and the launched program's own must both be
/// present in any inherited or endpoint-received chain), or a frame that
/// stops mid-field is malformed.
pub(crate) fn parse(bytes: &[u8]) -> Result<Vec<Entry>, String> {
    let magic = MAGIC.len() + 1;
    if bytes.len() < magic || &bytes[..MAGIC.len()] != MAGIC || bytes[MAGIC.len()] != 0 {
        return Err(
            "the record does not open with the ratmac-lineage-v1 magic and one NUL byte".to_owned(),
        );
    }
    let mut at = magic;
    let count = match bytes.get(at..at + 8) {
        Some(window) => u64::from_be_bytes(window.try_into().expect("eight bytes")),
        None => return Err("the record stops before its entry count".to_owned()),
    };
    at += 8;
    let count = usize::try_from(count)
        .map_err(|_| "the record's entry count overflows the platform".to_owned())?;
    if count < 2 {
        return Err(format!(
            "the record holds {count} entries; a lineage chain needs at least the launching \
             parent's entry and the launched program's own"
        ));
    }
    // Each entry needs at least its two length frames, so the remaining
    // bytes bound the reservation: a hostile count can never abort on
    // allocation before the frame checks refuse it as malformed.
    let mut entries = Vec::with_capacity(count.min(bytes.len() / 16 + 1));
    for index in 0..count {
        let mut fields = [String::new(), String::new()];
        for field in fields.iter_mut() {
            let length = match bytes.get(at..at + 8) {
                Some(window) => u64::from_be_bytes(window.try_into().expect("eight bytes")),
                None => return Err(format!("the frame for entry {index} stops mid-field")),
            };
            at += 8;
            let length = usize::try_from(length)
                .map_err(|_| "a field length overflows the platform".to_owned())?;
            let window = bytes
                .get(at..at + length)
                .ok_or_else(|| format!("the frame for entry {index} stops mid-field"))?;
            let text = std::str::from_utf8(window)
                .map_err(|_| format!("entry {index} is not valid UTF-8"))?;
            *field = text.to_owned();
            at += length;
        }
        let [identity, root] = fields;
        entries.push(Entry { identity, root });
    }
    if at != bytes.len() {
        return Err("trailing bytes after the final entry".to_owned());
    }
    Ok(entries)
}

/// Decode one carrier: lowercase hexadecimal, two digits per byte and
/// nothing else, then the framed record. An empty, odd-length, or non-hex
/// carrier is malformed before its bytes are parsed.
pub(crate) fn parse_carrier(carrier: &str) -> Result<Vec<Entry>, String> {
    if carrier.is_empty() {
        return Err("the carrier is empty".to_owned());
    }
    if !carrier.len().is_multiple_of(2) {
        return Err("the carrier holds an odd number of hex digits".to_owned());
    }
    if !carrier
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("the carrier holds a character outside 0-9a-f".to_owned());
    }
    let mut bytes = Vec::with_capacity(carrier.len() / 2);
    let (pairs, remainder) = carrier.as_bytes().as_chunks::<2>();
    debug_assert!(remainder.is_empty(), "even length validated above");
    for pair in pairs {
        let digit = |byte: u8| (byte as char).to_digit(16).expect("validated hex");
        bytes.push((digit(pair[0]) * 16 + digit(pair[1])) as u8);
    }
    parse(&bytes)
}

/// Encode one chain as the carrier spelling both sides transport.
pub(crate) fn to_carrier(entries: &[Entry]) -> String {
    encode(entries)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The display form for refusals: entries newest-last, joined by ` > ` as
/// `<identity>@<canonical root>`.
pub(crate) fn display(entries: &[Entry]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}@{}", entry.identity, entry.root))
        .collect::<Vec<_>>()
        .join(" > ")
}

/// The first entry repeating a program identity already in the chain, if any.
pub(crate) fn repeated_identity(entries: &[Entry]) -> Option<&Entry> {
    (0..entries.len())
        .find(|at| {
            entries[..*at]
                .iter()
                .any(|prior| prior.identity == entries[*at].identity)
        })
        .map(|at| &entries[at])
}

/// The first entry repeating a canonical Engine root already in the chain.
pub(crate) fn repeated_root(entries: &[Entry]) -> Option<&Entry> {
    (0..entries.len())
        .find(|at| {
            entries[..*at]
                .iter()
                .any(|prior| prior.root == entries[*at].root)
        })
        .map(|at| &entries[at])
}
