//! t-123 / WEB-001: portable goal fingerprints.
//!
//! WEBV-001 `webv_001` - portable text: two isolated trees with identical
//! names and LF versus CRLF text yield exactly the same versioned revision
//! (nested paths, mixed CRLF/LF content), and the contract's four fixed
//! serialized-byte and revision vectors hold, computed here from the
//! contract's grammar with `sha2` alone, plus the pinned literal vectors.
//! WEBV-002 `webv_002` - meaning remains visible: rename, add, remove, edit
//! a character, add a final newline, alter a binary byte, and change a lone
//! CR each change the digest; file enumeration order does not; length
//! framing distinguishes adversarial path/content boundary combinations.
//! WEBV-003 `webv_003` - forward compatibility: an old fixture verifies only
//! with the old algorithm and its archived stamp stays byte-identical and
//! parseable; a new fixture carries exactly `v2:` plus 64 lowercase hex
//! digits and is accepted inside the compound `goal-sha256:v2:<digest>`
//! citation; unknown versions, malformed markers, wrong digest lengths,
//! uppercase new digests, and invalid path encodings refuse without writes.
//! WEBV-004 `webv_004` - drift boundary: start/freeze/step fixtures cover
//! both versions, a legacy baseline first frozen after the upgrade stays
//! legacy, a semantic edit after freeze refuses on both, and
//! line-ending-only edits affect only the legacy fixture.
//!
//! The fixed vectors are a pure byte grammar, so the in-test computation
//! proves them on any platform; the engine fixtures here run on Windows.

use ratmac::contract::{gate_records, ContractDefect};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The contract's four fixed vectors (t-123 `Observable Output`): the
/// serialized bytes and the revision each fixture must hash to.
const EMPTY_BYTES: &str = "7261746d61632d676f616c2d7632000000000000000000";
const EMPTY_REVISION: &str = "v2:985cf08ca7fbde22915e18c240e90faec480d42cdcbb9c6a2574ab7e8aa88528";
const TEXT_BYTES: &str = "7261746d61632d676f616c2d76320000000000000000010000000000000005612e747874000000000000000668656c6c6f0a";
const TEXT_REVISION: &str = "v2:f971951e85b5db5335843e6d326699f7f1fd0583f9698c7e520631e308ffb22f";
const BINARY_BYTES: &str = "7261746d61632d676f616c2d7632000000000000000001000000000000000762696e2e6461740000000000000004000d0aff";
const BINARY_REVISION: &str = "v2:391218dbef39b1bdd53dd95d6c43ce1fca611af864da532fa7ee541a111c29f6";
const TWO_FILES_BYTES: &str = "7261746d61632d676f616c2d7632000000000000000002000000000000000161000000000000000000000000000000016200000000000000010a";
const TWO_FILES_REVISION: &str =
    "v2:476aa6a2b832fd23a0f717bbbb70c7a9e2efae43a887282c93b2d3097a588718";

/// An archived stamp: an older legacy freeze, provenance only.
const ARCHIVED_STAMP: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// Every engine invocation is bounded: the child is polled and killed at the
/// deadline, so a stuck engine fails the test instead of hanging it.
const RTM_DEADLINE: Duration = Duration::from_secs(120);

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos()
}

/// One bounded engine invocation's observable bytes and exit code.
struct Proc {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
}

impl Proc {
    fn out(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    fn err(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

fn run_rtm(root: &Path, args: &[&str], env: &[(&str, &str)]) -> Proc {
    let mut command = Command::new(ratmac_qa::engine_bin!());
    command.args(args).current_dir(root);
    for (name, value) in env {
        command.env(name, value);
    }
    let deadline = Instant::now() + RTM_DEADLINE;
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the engine");
    loop {
        match child.try_wait().expect("poll the engine") {
            Some(status) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut pipe) = child.stdout.take() {
                    std::io::Read::read_to_end(&mut pipe, &mut stdout).expect("drain stdout");
                }
                if let Some(mut pipe) = child.stderr.take() {
                    std::io::Read::read_to_end(&mut pipe, &mut stderr).expect("drain stderr");
                }
                return Proc {
                    stdout,
                    stderr,
                    code: status.code(),
                };
            }
            None if Instant::now() > deadline => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("engine did not exit within {RTM_DEADLINE:?}: {args:?}");
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    }
}

fn to_hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn from_hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|index| u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).expect("hex digit"))
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// A file is text exactly when its bytes are valid UTF-8 without a NUL byte;
/// text CRLF pairs become LF and nothing else changes (lone CR, byte-order
/// marks, and final-newline presence are preserved); every other file
/// contributes its raw bytes unchanged.
fn canonical_content(bytes: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    match std::str::from_utf8(bytes) {
        Ok(text) if !text.contains('\0') => {
            if text.contains("\r\n") {
                std::borrow::Cow::Owned(text.replace("\r\n", "\n").into_bytes())
            } else {
                std::borrow::Cow::Borrowed(bytes)
            }
        }
        _ => std::borrow::Cow::Borrowed(bytes),
    }
}

/// The v2 serialization from the contract's grammar alone: `ratmac-goal-v2`,
/// one NUL, the u64 big-endian file count, then per file in ascending
/// bytewise path order the u64 big-endian path length, the forward-slash
/// path bytes, the u64 big-endian canonical-content length, and the bytes.
fn canonical_goal_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut ordered = files.to_vec();
    // `str` ordering is bytewise, which is the contract's path order.
    ordered.sort_by(|left, right| left.0.cmp(right.0));
    let mut bytes = b"ratmac-goal-v2".to_vec();
    bytes.push(0);
    bytes.extend_from_slice(&(ordered.len() as u64).to_be_bytes());
    for (path, content) in &ordered {
        bytes.extend_from_slice(&(path.len() as u64).to_be_bytes());
        bytes.extend_from_slice(path.as_bytes());
        let canonical = canonical_content(content);
        bytes.extend_from_slice(&(canonical.len() as u64).to_be_bytes());
        bytes.extend_from_slice(&canonical);
    }
    bytes
}

/// The v2 revision: `v2:` plus 64 lowercase hexadecimal digits.
fn v2_revision(files: &[(&str, &[u8])]) -> String {
    format!("v2:{}", sha256_hex(&canonical_goal_bytes(files)))
}

/// Today's legacy calculation (`src/goal.rs`): sort by relative path, hash
/// `path bytes, LF, content bytes, LF` per file, render lowercase hex.
fn legacy_revision(files: &[(&str, &[u8])]) -> String {
    let mut ordered = files.to_vec();
    // `str` ordering is bytewise, matching today's producer.
    ordered.sort_by(|left, right| left.0.cmp(right.0));
    let mut hasher = Sha256::new();
    for (path, content) in &ordered {
        hasher.update(path.as_bytes());
        hasher.update(b"\n");
        hasher.update(content);
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

/// Exactly `v2:` followed by 64 lowercase hexadecimal digits.
fn is_v2_revision(text: &str) -> bool {
    let Some(digest) = text.strip_prefix("v2:") else {
        return false;
    };
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// The one displayed spelling: forward slashes.
fn displayed(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// The goal root as the Engine names it in refusal lines: the resolved
/// declared root, displayed.
fn goal_root_displayed(root: &Path) -> String {
    let resolved = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    displayed(&resolved.join(".arca/goal"))
}

/// Read one `key = "value"` string out of a TOML section body.
fn field(section: &str, key: &str) -> Option<String> {
    section.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name.trim() == key).then(|| value.trim().trim_matches('"').to_owned())
    })
}

/// The `[goal]` table of Run evidence.
fn goal_table(evidence: &str) -> String {
    evidence
        .split("[goal]")
        .nth(1)
        .map(|rest| {
            rest.split("\n[")
                .next()
                .unwrap_or_default()
                .trim()
                .to_owned()
        })
        .unwrap_or_default()
}

fn reasons(defects: &[ContractDefect]) -> String {
    defects
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join("; ")
}

/// A project whose `intake -> gaps` transition closes intake integration by
/// freezing the goal; fixtures build their own goal trees beneath
/// `.arca/goal`.
struct Fixture {
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t123-{label}-{}-{}",
            std::process::id(),
            nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".arca/goal")).expect("create the goal root");
        fs::create_dir_all(root.join(".ratmac")).expect("create the Engine root");
        let class = "[roots]\n\
                     goal = \".arca/goal\"\n\
                     \n\
                     [states.intake]\n\
                     prompt = \"Integrate the issues.\"\n\
                     [states.gaps]\n\
                     prompt = \"Find the gaps.\"\n\
                     \n\
                     [states.tickets]\n\
                     prompt = \"Cut the tickets.\"\n\
                     \n\
                     [[transitions]]\n\
                     from = \"intake\"\n\
                     to = \"gaps\"\n\
                     freeze = \"goal\"\n\
                     \n\
                     [[transitions]]\n\
                     from = \"gaps\"\n\
                     to = \"tickets\"\n";
        fs::write(root.join(".ratmac/ratmac.toml"), class).expect("write the machine class");
        Fixture { root }
    }

    fn goal_path(&self, name: &str) -> PathBuf {
        self.root.join(".arca/goal").join(name)
    }

    fn write_goal_bytes(&self, name: &str, bytes: &[u8]) {
        let path = self.goal_path(name);
        fs::create_dir_all(path.parent().expect("goal file has a parent"))
            .expect("create the goal folder");
        fs::write(&path, bytes).expect("write the goal file");
    }

    fn rtm(&self, args: &[&str]) -> Proc {
        run_rtm(&self.root, args, &[])
    }

    /// The live run's id, read off the roster.
    fn run_id(&self) -> String {
        let mut ids: Vec<String> = fs::read_dir(self.root.join(".ratmac/runs"))
            .expect("list the runs roster")
            .map(|entry| entry.expect("roster entry is readable"))
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        ids.sort();
        ids.pop().expect("a run appears on the roster")
    }

    fn run_dir(&self) -> PathBuf {
        self.root.join(".ratmac/runs").join(self.run_id())
    }

    fn step(&self) -> Proc {
        let id = self.run_id();
        self.rtm(&["step", "--run", &id])
    }

    fn status(&self) -> Proc {
        let id = self.run_id();
        self.rtm(&["status", "--run", &id])
    }

    fn evidence_bytes(&self) -> Vec<u8> {
        fs::read(self.run_dir().join("evidence.toml")).unwrap_or_default()
    }

    fn state_bytes(&self) -> Vec<u8> {
        fs::read(self.run_dir().join("run.toml")).unwrap_or_default()
    }

    fn log_bytes(&self) -> Vec<u8> {
        fs::read(self.root.join(".ratmac/log.md")).unwrap_or_default()
    }

    fn goal_field(&self, key: &str) -> String {
        let evidence = String::from_utf8_lossy(&self.evidence_bytes()).into_owned();
        field(&goal_table(&evidence), key).unwrap_or_default()
    }

    fn baseline(&self) -> String {
        self.goal_field("baseline")
    }

    fn frozen(&self) -> String {
        self.goal_field("frozen")
    }

    /// Rewrite one `[goal]` string field in place, keeping the shape the
    /// Engine's strict evidence parser reads.
    fn set_evidence_goal_field(&self, key: &str, value: &str) {
        let path = self.run_dir().join("evidence.toml");
        let source = fs::read_to_string(&path).expect("read evidence");
        let mut rendered = String::new();
        let mut in_goal = false;
        for line in source.lines() {
            if line.starts_with('[') {
                in_goal = line == "[goal]";
            }
            if in_goal && line.starts_with(key) && line[key.len()..].starts_with(" = ") {
                rendered.push_str(&format!("{key} = \"{value}\"\n"));
            } else {
                rendered.push_str(line);
                rendered.push('\n');
            }
        }
        fs::write(&path, rendered).expect("patch evidence");
    }
}

/// Plant a goal tree and start a Run over it; the positive control (a clean
/// start that records a baseline) is asserted before anything else.
fn baseline_of(label: &str, files: &[(&str, &[u8])]) -> String {
    let fixture = Fixture::new(label);
    for (name, bytes) in files {
        fixture.write_goal_bytes(name, bytes);
    }
    let started = fixture.rtm(&["start"]);
    assert_eq!(
        started.code,
        Some(0),
        "positive control: {label} starts: {}{}",
        started.out(),
        started.err()
    );
    let baseline = fixture.baseline();
    assert!(
        !baseline.is_empty(),
        "start records a goal baseline: {label}"
    );
    baseline
}

/// Create a real goal entry whose relative path is not valid UTF-8; `None`
/// when the host refuses the name (the fault-injection seam covers it then).
#[cfg(windows)]
fn write_non_utf8_goal_entry(goal_root: &Path) -> Option<OsString> {
    use std::os::windows::ffi::OsStringExt;
    let name = OsString::from_wide(&[0xd800]);
    fs::write(goal_root.join(&name), b"payload\n").ok()?;
    name.to_str().is_none().then_some(name)
}

#[cfg(unix)]
fn write_non_utf8_goal_entry(goal_root: &Path) -> Option<OsString> {
    use std::os::unix::ffi::OsStringExt;
    let name = OsString::from_vec(vec![0xff, 0xfe]);
    fs::write(goal_root.join(&name), b"payload\n").ok()?;
    name.to_str().is_none().then_some(name)
}

/// The drift refusal's exact observable bytes: both stdout lines with the
/// observed and expected revisions in the Run's own version, exit 0, empty
/// stderr, and Run Record, log, and evidence byte-identical.
fn expect_drift(fixture: &Fixture, observed: &str, frozen: &str) {
    let id = fixture.run_id();
    let state_before = fixture.state_bytes();
    let log_before = fixture.log_bytes();
    let evidence_before = fixture.evidence_bytes();
    let refused = fixture.step();
    assert_eq!(refused.code, Some(0), "a drift refusal exits 0");
    assert_eq!(
        refused.out(),
        format!(
            "rtm: step refused; goal drift: {}: observed {observed}; expected {frozen}\n\
             next: rtm step --run {id}\n",
            goal_root_displayed(&fixture.root)
        ),
        "today's exact drift bytes"
    );
    assert!(refused.stderr.is_empty(), "stderr stays empty");
    assert_eq!(
        state_before,
        fixture.state_bytes(),
        "run.toml is byte-identical"
    );
    assert_eq!(log_before, fixture.log_bytes(), "log.md is byte-identical");
    assert_eq!(
        evidence_before,
        fixture.evidence_bytes(),
        "evidence is byte-identical"
    );
}

/// WEBV-001: two isolated trees with identical names and LF versus CRLF text
/// yield exactly the same versioned revision, including nested paths and
/// mixed CRLF/LF content; the fixed serialized-byte and revision vectors are
/// asserted independently of the production serializer.
#[test]
fn webv_001() {
    // The in-test serializer (the contract's grammar plus `sha2`, never the
    // production code) reproduces every pinned vector, and each pinned byte
    // string hashes to its pinned revision, so the literals agree.
    let text_file: &[(&str, &[u8])] = &[("a.txt", b"hello\n")];
    let binary_file: &[(&str, &[u8])] = &[("bin.dat", &[0x00, 0x0d, 0x0a, 0xff])];
    let two_files: &[(&str, &[u8])] = &[("a", b""), ("b", b"\n")];
    for (files, pinned_bytes, pinned_revision) in [
        (&[][..], EMPTY_BYTES, EMPTY_REVISION),
        (text_file, TEXT_BYTES, TEXT_REVISION),
        (binary_file, BINARY_BYTES, BINARY_REVISION),
        (two_files, TWO_FILES_BYTES, TWO_FILES_REVISION),
    ] {
        assert_eq!(to_hex(&canonical_goal_bytes(files)), pinned_bytes);
        assert_eq!(v2_revision(files), pinned_revision);
        assert_eq!(
            sha256_hex(&from_hex(pinned_bytes)),
            pinned_revision.trim_start_matches("v2:")
        );
    }
    // The text vector also applies when its input is CRLF text.
    assert_eq!(v2_revision(&[("a.txt", b"hello\r\n")]), TEXT_REVISION);
    // The two-file vector from an unordered list, and the binary file keeps
    // its NUL, CRLF, and invalid UTF-8 byte unchanged.
    assert_eq!(v2_revision(&[("b", b"\n"), ("a", b"")]), TWO_FILES_REVISION);

    // Two isolated trees: identical names, LF versus CRLF, nested paths, and
    // mixed line endings within one file.
    let trees: &[(&str, &[u8])] = &[
        ("spec.md", b"alpha\nbeta\n"),
        ("mixed.md", b"a\nb\nc\n"),
        ("notes/top.md", b"plain\n"),
        ("notes/sub/deep.md", b"x\ny\n"),
    ];
    let lf = Fixture::new("webv001-lf");
    for (name, bytes) in trees {
        lf.write_goal_bytes(name, bytes);
    }
    let crlf = Fixture::new("webv001-crlf");
    for (name, bytes) in trees {
        let mut styled = Vec::with_capacity(bytes.len());
        for &byte in *bytes {
            if byte == b'\n' {
                styled.push(b'\r');
            }
            styled.push(byte);
        }
        crlf.write_goal_bytes(name, &styled);
    }
    let lf_started = lf.rtm(&["start"]);
    assert_eq!(
        lf_started.code,
        Some(0),
        "positive control: the LF tree starts: {}{}",
        lf_started.out(),
        lf_started.err()
    );
    let crlf_started = crlf.rtm(&["start"]);
    assert_eq!(
        crlf_started.code,
        Some(0),
        "positive control: the CRLF tree starts: {}{}",
        crlf_started.out(),
        crlf_started.err()
    );
    let id = lf.run_id();
    assert_eq!(
        lf_started.out(),
        format!("rtm: started run {id} at .ratmac/runs/{id}/\n"),
        "today's one stdout line"
    );
    let lf_baseline = lf.baseline();
    let crlf_baseline = crlf.baseline();
    assert!(
        is_v2_revision(&lf_baseline),
        "a newly started Run records a v2 baseline: {lf_baseline}"
    );
    assert_eq!(
        lf_baseline, crlf_baseline,
        "LF and CRLF trees with identical names yield the same revision"
    );
    assert_eq!(
        lf_baseline,
        v2_revision(trees),
        "the engine's revision equals the independent v2 computation"
    );

    // The pinned fixtures through the engine, including the two-file vector
    // with `b` created before `a`.
    assert_eq!(baseline_of("webv001-text", text_file), TEXT_REVISION);
    assert_eq!(baseline_of("webv001-binary", binary_file), BINARY_REVISION);
    assert_eq!(baseline_of("webv001-empty", &[]), EMPTY_REVISION);
    assert_eq!(
        baseline_of("webv001-order-a", two_files),
        TWO_FILES_REVISION
    );
    assert_eq!(
        baseline_of("webv001-order-b", &[("b", b"\n"), ("a", b"")]),
        TWO_FILES_REVISION,
        "the two-file vector matches even if b is created before a"
    );
}

/// The binary goal file both eras hash raw: NUL, CRLF, and an invalid
/// UTF-8 byte together make the file binary.
const BINARY: &[u8] = &[0x00, 0x0d, 0x0a, 0xff];

/// One mutation twin: its label and its whole goal tree.
type Twin = (&'static str, &'static [(&'static str, &'static [u8])]);

/// WEBV-002: rename/add/remove files, edit a character, add a final newline,
/// alter a binary byte, and change a lone CR; each changes the digest. File
/// enumeration order does not. Length framing distinguishes adversarial
/// path/content boundary combinations.
#[test]
fn webv_002() {
    let base: &[(&str, &[u8])] = &[
        ("spec.md", b"one\ntwo\n"),
        ("data.bin", BINARY),
        ("notes/a.md", b"x\r\ny\rz\n"),
        ("notes/b.md", b""),
    ];
    let base_revision = baseline_of("webv002-base", base);

    let twins: &[Twin] = &[
        (
            "renaming a file changes the digest",
            &[
                ("spec2.md", b"one\ntwo\n"),
                ("data.bin", BINARY),
                ("notes/a.md", b"x\r\ny\rz\n"),
                ("notes/b.md", b""),
            ],
        ),
        (
            "adding a file changes the digest",
            &[
                ("spec.md", b"one\ntwo\n"),
                ("data.bin", BINARY),
                ("notes/a.md", b"x\r\ny\rz\n"),
                ("notes/b.md", b""),
                ("extra.md", b"e\n"),
            ],
        ),
        (
            "removing a file changes the digest",
            &[
                ("spec.md", b"one\ntwo\n"),
                ("data.bin", BINARY),
                ("notes/a.md", b"x\r\ny\rz\n"),
            ],
        ),
        (
            "editing a character changes the digest",
            &[
                ("spec.md", b"one\ntwp\n"),
                ("data.bin", BINARY),
                ("notes/a.md", b"x\r\ny\rz\n"),
                ("notes/b.md", b""),
            ],
        ),
        (
            "adding a final newline changes the digest",
            &[
                ("spec.md", b"one\ntwo"),
                ("data.bin", BINARY),
                ("notes/a.md", b"x\r\ny\rz\n"),
                ("notes/b.md", b""),
            ],
        ),
        (
            "altering a binary byte changes the digest",
            &[
                ("spec.md", b"one\ntwo\n"),
                ("data.bin", &[0x00, 0x0d, 0x0a, 0xfe]),
                ("notes/a.md", b"x\r\ny\rz\n"),
                ("notes/b.md", b""),
            ],
        ),
        (
            "changing a lone CR changes the digest",
            &[
                ("spec.md", b"one\ntwo\n"),
                ("data.bin", BINARY),
                ("notes/a.md", b"x\r\nyz\n"),
                ("notes/b.md", b""),
            ],
        ),
    ];
    for (label, files) in twins {
        let twin_revision = baseline_of("webv002-twin", files);
        assert_ne!(twin_revision, base_revision, "{label}");
        assert_ne!(
            v2_revision(files),
            v2_revision(base),
            "{label}, in the independent computation"
        );
    }

    // Names that differ are never equated: case is preserved.
    assert_ne!(
        baseline_of("webv002-lower", &[("a.md", b"x\n")]),
        baseline_of("webv002-upper", &[("A.md", b"x\n")]),
        "a case-only rename changes the digest"
    );

    // File enumeration order does not change the digest.
    let ordered: &[(&str, &[u8])] = &[
        ("a.txt", b"first\n"),
        ("m/one.md", b"1\n"),
        ("m/two.md", b"2\n"),
        ("z.txt", b"last\n"),
    ];
    let reversed: &[(&str, &[u8])] = &[
        ("z.txt", b"last\n"),
        ("m/two.md", b"2\n"),
        ("m/one.md", b"1\n"),
        ("a.txt", b"first\n"),
    ];
    assert_eq!(
        baseline_of("webv002-order-forward", ordered),
        baseline_of("webv002-order-reverse", reversed),
        "creation order does not change the digest"
    );
    assert_eq!(
        canonical_goal_bytes(ordered),
        canonical_goal_bytes(reversed),
        "the serialization sorts by path bytes"
    );

    // Length framing distinguishes adversarial path/content boundaries the
    // legacy delimiter stream equates. The legacy collision is proven here
    // with the same sha2, not with the production serializer.
    assert_eq!(
        legacy_revision(&[("a\nb", b"c")]),
        legacy_revision(&[("a", b"b\nc")]),
        "the legacy stream cannot tell path from content at a newline"
    );
    assert_ne!(
        v2_revision(&[("a\nb", b"c")]),
        v2_revision(&[("a", b"b\nc")]),
        "length framing separates the same boundary"
    );
    // Through the engine, with creatable names: {a:"b", c:"d"} and
    // {a:"b\nc\nd"} serialize to the same legacy stream and must not share a
    // v2 revision.
    assert_eq!(
        legacy_revision(&[("a", b"b"), ("c", b"d")]),
        legacy_revision(&[("a", b"b\nc\nd")]),
        "the creatable pair collides under the legacy calculation"
    );
    assert_ne!(
        canonical_goal_bytes(&[("a", b"b"), ("c", b"d")]),
        canonical_goal_bytes(&[("a", b"b\nc\nd")])
    );
    let split = baseline_of("webv002-framing-split", &[("a", b"b"), ("c", b"d")]);
    let joined = baseline_of("webv002-framing-joined", &[("a", b"b\nc\nd")]);
    assert_ne!(
        split, joined,
        "length framing distinguishes the path/content boundary combination"
    );
}

/// A record-contract tree (the t048 shape): one live residual citing
/// `citation`, one archived stamp for an older legacy freeze, and a synthetic
/// Run whose evidence froze `frozen`.
struct Records {
    root: PathBuf,
}

impl Drop for Records {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Records {
    fn new(label: &str, frozen: &str, citation: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t123-{label}-{}-{}",
            std::process::id(),
            nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in [
            ".arca/goal",
            ".arca/issue/i-100-demo",
            ".arca/residual/archive",
            ".arca/ticket",
            ".ratmac/runs/run-000",
        ] {
            fs::create_dir_all(root.join(dir)).expect("create the record tree folders");
        }
        fs::write(
            root.join(".arca/goal/spec.md"),
            "# Goal spec\n\n\
             | Req ID | Requirement | Source |\n\
             |---|---|---|\n\
             | DEMO-001 | The demo behaves. | [issue DEMO-001](../issue/i-100-demo/spec.md#requirement-records) |\n\
             | DEMO-002 | The archive behaves. | Source. |\n",
        )
        .expect("write the goal spec");
        fs::write(
            root.join(".arca/issue/i-100-demo/index.md"),
            "# Issue i-100-demo\n\n```yaml\nissue-id: \"i-100-demo\"\nstatus: \"integrated\"\n```\n\nSee [goal spec](../../goal/spec.md).\n",
        )
        .expect("write the issue index");
        fs::write(
            root.join(".arca/issue/i-100-demo/spec.md"),
            "# Requirement records\n\n| Req ID | Requirement | Status |\n|---|---|---|\n| DEMO-001 | The demo behaves. | accepted |\n",
        )
        .expect("write the issue spec");
        for name in ["design.md", "test-plan.md", "ubi-lang.md"] {
            fs::write(
                root.join(".arca/issue/i-100-demo").join(name),
                format!("# {name}\n"),
            )
            .expect("write the issue file");
        }
        fs::write(
            root.join(".arca/residual/res-100.md"),
            format!(
                "# Residual Record\n\n```yaml\n\
                 residual-id: \"res-100\"\n\
                 goal-requirement-ref: \"DEMO-001\"\n\
                 frozen-goal-bundle-revision: \"{citation}\"\n\
                 status: \"missing\"\n```\n"
            ),
        )
        .expect("write the live residual");
        fs::write(
            root.join(".arca/residual/archive/res-099.md"),
            format!(
                "# Residual Record\n\n```yaml\n\
                 residual-id: \"res-099\"\n\
                 goal-requirement-ref: \"DEMO-002\"\n\
                 frozen-goal-bundle-revision: \"goal-sha256:{ARCHIVED_STAMP}\"\n\
                 concrete-evidence-refs:\n  - \"src/demo.rs\"\n\
                 status: \"satisfied\"\n```\n"
            ),
        )
        .expect("write the archived residual");
        let lanes = [
            "Regression",
            "Input/Routing",
            "Lifecycle/Model",
            "Durability/Recovery",
            "Output/Filesystem",
            "Cross-Feature",
        ]
        .iter()
        .map(|lane| format!("| `{lane}` | `covered` | Reason. | `none` |\n"))
        .collect::<String>();
        fs::write(
            root.join(".arca/ticket/t-100.md"),
            format!(
                "---\nticket-id: t-100\nresidual-ids:\n  - \"res-100\"\n\
                 dependencies:\nstatus: \"approved\"\n---\n\n\
                 # Ticket: t-100\n\n## Vertical Outcome\n\nOutcome.\n\n\
                 ## Worktree Scope\n\nScope.\n\n\
                 ## P4 Apparent Test Plan\n\n| Apparent Test ID |\n|---|\n| `PT-100-01` |\n\n\
                 ## P5 Hidden Test Public Coverage Manifest\n\n\
                 | Lane | Assessment | Rationale | Hidden IDs |\n|---|---|---|---|\n{lanes}\
                 ## Merge Gate\n\n- Ticket tests pass.\n"
            ),
        )
        .expect("write the ticket");
        fs::write(
            root.join(".ratmac/runs/run-000/evidence.toml"),
            format!("[goal]\nbaseline = \"{frozen}\"\nfrozen = \"{frozen}\"\n"),
        )
        .expect("write the run evidence");
        fs::write(
            root.join(".ratmac/ratmac.toml"),
            "[roots]\n\
             goal = \".arca/goal\"\n\
             issue = \".arca/issue\"\n\
             residual = \".arca/residual\"\n\
             ticket = \".arca/ticket\"\n\
             \n\
             [states.intake]\n\
             prompt = \"Integrate.\"\n\
             guards = [{ kind = \"intake_contract\" }]\n\
             \n\
             [states.gaps]\n\
             prompt = \"Find gaps.\"\n\
             guards = [{ kind = \"record_contract\" }]\n\
             \n\
             [states.build]\n\
             prompt = \"Build.\"\n\
             guards = [{ kind = \"sensitivity_receipts\", root = \"ticket\", ticket = \"t-100.md\" }]\n\
             \n\
             [[transitions]]\n\
             from = \"intake\"\n\
             to = \"gaps\"\n\
             \n\
             [[transitions]]\n\
             from = \"gaps\"\n\
             to = \"build\"\n",
        )
        .expect("write the machine class");
        Records { root }
    }

    fn gate(&self) -> Result<(), Vec<ContractDefect>> {
        gate_records(&self.root, &self.root.join(".ratmac"), "run-000")
    }

    fn archived_bytes(&self) -> Vec<u8> {
        fs::read(self.root.join(".arca/residual/archive/res-099.md"))
            .expect("read the archived stamp")
    }
}

/// Start and freeze a Run over one goal tree, then patch `[goal] frozen` to
/// `value` and step: an unknown fingerprint version refuses without writes.
fn unknown_version_refusal(label: &str, value: &str) {
    let fixture = Fixture::new(label);
    fixture.write_goal_bytes("spec.md", b"# Spec\n\nrequirement one\n");
    fixture.write_goal_bytes("test-list.md", b"# Tests\n\ncheck one\n");
    let started = fixture.rtm(&["start"]);
    assert_eq!(
        started.code,
        Some(0),
        "positive control: {label} starts: {}{}",
        started.out(),
        started.err()
    );
    let froze = fixture.step();
    assert!(
        !froze.out().contains("step refused"),
        "positive control: {label} freezes: {}",
        froze.out()
    );
    let id = fixture.run_id();
    fixture.set_evidence_goal_field("frozen", value);
    let evidence_before = fixture.evidence_bytes();
    let state_before = fixture.state_bytes();
    let log_before = fixture.log_bytes();
    let refused = fixture.step();
    assert_eq!(refused.code, Some(1), "an unknown version exits 1");
    assert!(
        refused.stdout.is_empty(),
        "stdout stays empty: {}",
        refused.out()
    );
    assert_eq!(
        refused.err(),
        format!(
            "rtm: step: goal revision \"{value}\" in .ratmac/runs/{id}/evidence.toml \
             carries an unknown fingerprint version; no evidence was updated\n\
             next: rtm doctor\n"
        ),
        "the refusal names the value and the repair"
    );
    assert_eq!(
        evidence_before,
        fixture.evidence_bytes(),
        "no evidence was updated"
    );
    assert_eq!(state_before, fixture.state_bytes(), "run.toml is untouched");
    assert_eq!(log_before, fixture.log_bytes(), "log.md is untouched");
}

/// WEBV-003: an old baseline/freeze fixture verifies only with the old
/// algorithm, stays byte-identical, and remains parseable in an archived
/// record; a new fixture carries exactly `v2:` plus 64 lowercase hexadecimal
/// digits and is accepted inside the compound `goal-sha256:v2:<digest>`
/// citation; unknown versions, malformed markers, wrong digest lengths,
/// uppercase new digests, and invalid path encodings refuse without writes.
#[test]
fn webv_003() {
    // A newly started Run carries exactly v2: plus 64 lowercase hex digits,
    // with `frozen = ""` at start.
    let grammar_files: &[(&str, &[u8])] = &[
        ("spec.md", b"# Spec\n\nrequirement one\n"),
        ("test-list.md", b"# Tests\n\ncheck one\n"),
    ];
    let grammar = Fixture::new("webv003-shape");
    for (name, bytes) in grammar_files {
        grammar.write_goal_bytes(name, bytes);
    }
    let started = grammar.rtm(&["start"]);
    assert_eq!(
        started.code,
        Some(0),
        "positive control: the grammar fixture starts: {}{}",
        started.out(),
        started.err()
    );
    let grammar_baseline = grammar.baseline();
    assert!(
        is_v2_revision(&grammar_baseline),
        "the baseline carries exactly v2: plus 64 lowercase hex digits: {grammar_baseline}"
    );
    assert_eq!(grammar.frozen(), "", "start does not freeze the goal");
    assert_eq!(
        grammar_baseline,
        v2_revision(grammar_files),
        "the baseline equals the independent v2 computation"
    );

    let legacy_files: &[(&str, &[u8])] = &[("spec.md", b"# Spec\n\nrequirement one\n")];
    let legacy_digest = legacy_revision(legacy_files);
    let v2_digest = v2_revision(legacy_files);
    let other_v2 = v2_revision(&[("spec.md", b"# Spec\n\nrequirement other\n")]);
    let well = "0123456789abcdef".repeat(4);

    // Positive control: the compound v2 citation is accepted for a v2-frozen
    // Run, and the archived stamp stays byte-identical and parseable.
    let ok_v2 = Records::new(
        "webv003-ok-v2",
        &v2_digest,
        &format!("goal-sha256:{v2_digest}"),
    );
    let archived_before = ok_v2.archived_bytes();
    ok_v2.gate().unwrap_or_else(|defects| {
        panic!(
            "the record gate accepts goal-sha256:v2:<digest> for a v2-frozen Run: {}",
            reasons(&defects)
        )
    });
    assert_eq!(
        archived_before,
        ok_v2.archived_bytes(),
        "the archived stamp keeps its bytes"
    );

    // Positive control: the legacy citation still verifies a legacy freeze,
    // and only with the old algorithm.
    assert_ne!(
        legacy_digest,
        v2_revision(legacy_files),
        "the two algorithms never agree"
    );
    let ok_legacy = Records::new(
        "webv003-ok-legacy",
        &legacy_digest,
        &format!("goal-sha256:{legacy_digest}"),
    );
    ok_legacy.gate().unwrap_or_else(|defects| {
        panic!(
            "the legacy citation still verifies the legacy freeze: {}",
            reasons(&defects)
        )
    });

    // Control: a v2 Run refuses a legacy digest citation - no comparison
    // accepts a match under the other algorithm.
    let cross = Records::new(
        "webv003-cross",
        &v2_digest,
        &format!("goal-sha256:{legacy_digest}"),
    );
    let defects = cross
        .gate()
        .expect_err("the other algorithm never verifies");
    let text = reasons(&defects);
    assert!(
        text.contains("cites goal revision")
            && text.contains(&format!("but the frozen revision is {v2_digest}")),
        "the refusal names both revisions: {text}"
    );
    assert!(
        text.contains("res-100") && !text.contains("res-099"),
        "only the live record is named; the archived stamp stays parseable: {text}"
    );

    // Control: a missing goal-sha256 marker refuses by name.
    let markerless = Records::new(
        "webv003-marker",
        &v2_digest,
        &format!("bundle-1:{v2_digest}"),
    );
    let defects = markerless
        .gate()
        .expect_err("a citation without the marker refuses");
    assert!(
        reasons(&defects).contains("records no parseable frozen-goal-bundle-revision citation"),
        "the marker refusal text: {}",
        reasons(&defects)
    );

    // Wrong digest length: not one well-formed revision string.
    let short = Records::new(
        "webv003-short",
        &v2_digest,
        &format!("goal-sha256:v2:{}", &well[..63]),
    );
    let defects = short
        .gate()
        .expect_err("a 63-digit token is not a well-formed revision string");
    assert!(
        reasons(&defects).contains("records no parseable frozen-goal-bundle-revision citation"),
        "a wrong digest length is a parse refusal: {}",
        reasons(&defects)
    );

    // Uppercase new digest.
    let upper = Records::new(
        "webv003-upper",
        &v2_digest,
        &format!("goal-sha256:v2:{}", well.to_uppercase()),
    );
    let defects = upper
        .gate()
        .expect_err("an uppercase v2 digest is not a well-formed revision string");
    assert!(
        reasons(&defects).contains("records no parseable frozen-goal-bundle-revision citation"),
        "an uppercase digest is a parse refusal: {}",
        reasons(&defects)
    );

    // Unknown version marker.
    let v3_citation = Records::new(
        "webv003-v3-citation",
        &v2_digest,
        &format!("goal-sha256:v3:{well}"),
    );
    let defects = v3_citation
        .gate()
        .expect_err("a v3 token is not a well-formed revision string");
    assert!(
        reasons(&defects).contains("records no parseable frozen-goal-bundle-revision citation"),
        "an unknown version marker is a parse refusal: {}",
        reasons(&defects)
    );

    // A well-formed token that contains the frozen without equaling it.
    let contains = Records::new(
        "webv003-contains",
        &legacy_digest,
        &format!("goal-sha256:v2:{legacy_digest}"),
    );
    let defects = contains
        .gate()
        .expect_err("substring coincidence no longer suffices");
    let text = reasons(&defects);
    assert!(
        text.contains(&format!(
            "cites goal revision v2:{legacy_digest}, but the frozen revision is {legacy_digest}"
        )),
        "the refusal names the cited and frozen revisions: {text}"
    );
    assert!(
        text.contains("res-100") && !text.contains("res-099"),
        "the archived stamp stays parseable and quiet: {text}"
    );

    // The token ends at the next `+`, not at the end of the value.
    let plus = Records::new(
        "webv003-plus",
        &v2_digest,
        &format!("goal-sha256:{other_v2}+goal-sha256:{v2_digest}"),
    );
    let defects = plus
        .gate()
        .expect_err("the token is read up to the next +, not the value's end");
    let text = reasons(&defects);
    assert!(
        text.contains(&format!(
            "cites goal revision {other_v2}, but the frozen revision is {v2_digest}"
        )),
        "the refusal names the token's own revision: {text}"
    );

    // Step-side refusals: a recorded frozen value that matches neither
    // grammar refuses without writes.
    unknown_version_refusal("webv003-v3", &format!("v3:{well}"));
    unknown_version_refusal("webv003-uppercase", &format!("v2:{}", well.to_uppercase()));
    unknown_version_refusal("webv003-short-frozen", &well[..63]);

    // Read-side refusal: a relative path that is not valid UTF-8 refuses
    // without writes. Where the host allows the real entry, it is created
    // for real; otherwise the fault-injection seam proves the same bytes.
    let bad = Fixture::new("webv003-non-utf8");
    bad.write_goal_bytes("spec.md", b"# Spec\n\nrequirement one\n");
    let created = write_non_utf8_goal_entry(&bad.root.join(".arca/goal"));
    let mut env: Vec<(&str, &str)> = Vec::new();
    if created.is_none() {
        env.push(("RATMAC_TEST_GOAL_FAULT", "unrepresentable-path"));
    }
    let refused = run_rtm(&bad.root, &["start"], &env);
    assert_eq!(
        refused.code,
        Some(1),
        "an unrepresentable path exits 1: {}{}",
        refused.out(),
        refused.err()
    );
    assert!(
        refused.stdout.is_empty(),
        "stdout stays empty: {}",
        refused.out()
    );
    // The named entry: the real offender when the host allowed it, else the
    // seam's first file in ascending bytewise order of displayed
    // relative-path bytes - this fixture's goal root holds only `spec.md`.
    let named = created
        .as_ref()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "spec.md".to_owned());
    assert_eq!(
        refused.err(),
        format!(
            "rtm: start: read declared goal root: cannot serialize declared goal {}: \
             relative path {named} is not valid UTF-8; refusing lossy conversion\n",
            goal_root_displayed(&bad.root)
        ),
        "the exact refusal bytes, real entry and seam alike"
    );
    assert!(
        !refused.out().contains("next:"),
        "a start refusal carries no next line"
    );
    assert!(
        !bad.root.join(".ratmac/runs").exists(),
        "the refusal mints no Run"
    );
    assert!(
        !bad.root.join(".ratmac/log.md").exists(),
        "the refusal writes no log.md"
    );
    assert!(
        !bad.root.join(".ratmac/mint.toml").exists(),
        "the refusal writes no mint.toml"
    );
    assert_eq!(
        fs::read(bad.goal_path("spec.md")).expect("read the goal file"),
        b"# Spec\n\nrequirement one\n",
        "the goal tree is unchanged"
    );
}

/// WEBV-004: start/freeze/step fixtures cover both versions, a legacy
/// baseline first frozen after the upgrade, and a semantic edit after
/// freeze. Line-ending-only edits affect only the legacy fixture.
#[test]
fn webv_004() {
    let tree: &[(&str, &[u8])] = &[
        ("spec.md", b"# Spec\n\nrequirement one\n"),
        ("notes/deep.md", b"x\r\ny\r\n"),
        ("bin.dat", &[0x00, 0x0d, 0x0a, 0xff]),
    ];

    // A v2 Run: start records the v2 baseline, the freeze records the v2
    // revision, the Run Record names it, and a CRLF->LF edit in text is not
    // drift.
    let v2 = Fixture::new("webv004-v2");
    for (name, bytes) in tree {
        v2.write_goal_bytes(name, bytes);
    }
    let started = v2.rtm(&["start"]);
    assert_eq!(
        started.code,
        Some(0),
        "positive control: the v2 run starts: {}{}",
        started.out(),
        started.err()
    );
    let baseline = v2.baseline();
    assert!(
        is_v2_revision(&baseline),
        "a newly started Run's baseline is a v2 revision: {baseline}"
    );
    let froze = v2.step();
    assert!(
        !froze.out().contains("step refused"),
        "the freezing transition itself is not drift: {}",
        froze.out()
    );
    let frozen = v2.frozen();
    assert!(
        is_v2_revision(&frozen),
        "the v2 run freezes the v2 revision: {frozen}"
    );
    assert_eq!(
        frozen,
        v2_revision(tree),
        "the frozen revision equals the independent v2 computation"
    );
    let status = v2.status();
    assert!(
        status.out().contains(&format!("Goal revision: {frozen}")),
        "the Run Record names the v2 revision: {}",
        status.out()
    );
    v2.write_goal_bytes("notes/deep.md", b"x\ny\n");
    let proceeded = v2.step();
    assert!(
        !proceeded.out().contains("step refused"),
        "a CRLF->LF edit in text canonicalizes to the frozen revision: {}",
        proceeded.out()
    );

    // A semantic edit on the v2 run refuses with today's exact bytes.
    let semantic = Fixture::new("webv004-v2-semantic");
    for (name, bytes) in tree {
        semantic.write_goal_bytes(name, bytes);
    }
    let started = semantic.rtm(&["start"]);
    assert_eq!(
        started.code,
        Some(0),
        "positive control: the semantic fixture starts: {}{}",
        started.out(),
        started.err()
    );
    let froze = semantic.step();
    assert!(
        !froze.out().contains("step refused"),
        "positive control: the semantic fixture freezes: {}",
        froze.out()
    );
    let frozen = semantic.frozen();
    semantic.write_goal_bytes("spec.md", b"# Spec\n\nrequirement two\n");
    let edited: &[(&str, &[u8])] = &[
        ("spec.md", b"# Spec\n\nrequirement two\n"),
        ("notes/deep.md", b"x\r\ny\r\n"),
        ("bin.dat", &[0x00, 0x0d, 0x0a, 0xff]),
    ];
    expect_drift(&semantic, &v2_revision(edited), &frozen);
    semantic.write_goal_bytes("spec.md", b"# Spec\n\nrequirement one\n");
    let cleared = semantic.step();
    assert!(
        !cleared.out().contains("step refused"),
        "reverting the edit clears the drift: {}",
        cleared.out()
    );

    // The same line-ending edit inside a binary file is drift on a v2 run:
    // binary CRLF pairs are raw content.
    let binary = Fixture::new("webv004-v2-binary");
    for (name, bytes) in tree {
        binary.write_goal_bytes(name, bytes);
    }
    let started = binary.rtm(&["start"]);
    assert_eq!(
        started.code,
        Some(0),
        "positive control: the binary fixture starts: {}{}",
        started.out(),
        started.err()
    );
    let froze = binary.step();
    assert!(
        !froze.out().contains("step refused"),
        "positive control: the binary fixture freezes: {}",
        froze.out()
    );
    let frozen = binary.frozen();
    binary.write_goal_bytes("bin.dat", &[0x00, 0x0a, 0xff]);
    let edited_binary: &[(&str, &[u8])] = &[
        ("spec.md", b"# Spec\n\nrequirement one\n"),
        ("notes/deep.md", b"x\r\ny\r\n"),
        ("bin.dat", &[0x00, 0x0a, 0xff]),
    ];
    expect_drift(&binary, &v2_revision(edited_binary), &frozen);

    // A legacy Run: the recorded baseline keeps the legacy calculation, so
    // its first freeze after the upgrade stays legacy, its Run Record shows
    // the bare digest, and the line-ending-only edit in text IS drift.
    let legacy = Fixture::new("webv004-legacy");
    for (name, bytes) in tree {
        legacy.write_goal_bytes(name, bytes);
    }
    let started = legacy.rtm(&["start"]);
    assert_eq!(
        started.code,
        Some(0),
        "positive control: the legacy run starts: {}{}",
        started.out(),
        started.err()
    );
    legacy.set_evidence_goal_field("baseline", &legacy_revision(tree));
    let froze = legacy.step();
    assert!(
        !froze.out().contains("step refused"),
        "a legacy baseline first frozen after the upgrade stays legacy: {}",
        froze.out()
    );
    let frozen = legacy.frozen();
    assert_eq!(
        frozen,
        legacy_revision(tree),
        "the legacy run freezes the legacy revision"
    );
    assert_eq!(frozen.len(), 64, "the legacy revision is a bare digest");
    let status = legacy.status();
    assert!(
        status.out().contains(&format!("Goal revision: {frozen}")),
        "the legacy Run Record names the bare digest: {}",
        status.out()
    );
    legacy.write_goal_bytes("notes/deep.md", b"x\ny\n");
    let edited_legacy: &[(&str, &[u8])] = &[
        ("spec.md", b"# Spec\n\nrequirement one\n"),
        ("notes/deep.md", b"x\ny\n"),
        ("bin.dat", &[0x00, 0x0d, 0x0a, 0xff]),
    ];
    expect_drift(&legacy, &legacy_revision(edited_legacy), &frozen);

    // A semantic edit on the legacy run refuses the same way.
    legacy.write_goal_bytes("spec.md", b"# Spec\n\nrequirement one edited\n");
    let drifted_legacy: &[(&str, &[u8])] = &[
        ("spec.md", b"# Spec\n\nrequirement one edited\n"),
        ("notes/deep.md", b"x\ny\n"),
        ("bin.dat", &[0x00, 0x0d, 0x0a, 0xff]),
    ];
    expect_drift(&legacy, &legacy_revision(drifted_legacy), &frozen);
}
