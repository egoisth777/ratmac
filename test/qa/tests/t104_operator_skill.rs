//! t-104 / AOP-003, AOP-004: the engine writes its own operator skill.
//!
//! AOPV-003 `the_skill_subcommand_writes_once_and_never_overwrites`
//! AOPV-004 `the_skill_teaches_invariants_and_drives_a_run_to_terminal`
//!
//! The subcommand under test is exactly the one the design names: the working
//! name `rtm skill <path>` (issue i-033, Proposed mechanics route 2), a
//! sibling of the scaffold carrying the scaffold's own discipline - the
//! caller gives a path that does not exist yet, the command writes exactly
//! one folder there, and an occupied path refuses having written nothing.
//! The folder written at the caller's path is the design's thin
//! `ratmac-operator` skill itself - `SKILL.md` plus a `references/` folder -
//! mirroring how `rtm scaffold <path>` puts its one file at the path it is
//! given, so the fixture's fresh path is named `ratmac-operator` and must
//! hold `SKILL.md` and `references/` directly beneath it.
//!
//! AOPV-003 judges the write discipline with whole-tree snapshots around
//! both writes. The fresh path must add exactly one folder - every new file
//! sits under the caller's path, every pre-existing byte is unchanged - whose
//! `SKILL.md` carries the writing engine's identity stamp: the same
//! 64-character lowercase sha256 the argument-free doctor reports for the
//! running executable (DFP-001), parsed live from that doctor render, never
//! hard-coded. The occupied path - pre-created empty, so an existence check
//! that peeks at content cannot pass - must refuse naming the path while the
//! snapshot stays byte-identical.
//!
//! AOPV-004 scans every written file for the two things the skill must never
//! carry: CLI flag tokens (every whitespace-separated token, stripped of
//! markdown wrapping, that spells `--<lowercase...>` - zero exceptions,
//! because AOP-003 enumerates no flags and the engine's own addressing
//! refusal already teaches the run id, so no invariant needs one) and quoted
//! command output (the verbatim markers of the engine's own renders:
//! `next:`, `Exit Guards:`, `pending guard:`, `step refused`, `State: `,
//! `Status: `). It then pins the invariant loop and the never-touch rules in
//! `SKILL.md` itself, and finally drives a scaffolded runbook Run to
//! terminal using only what the skill teaches plus live engine output: the
//! loop orients with `rtm status`, and every later act is the exact command
//! a rendered taught line names, mechanically executed until a render
//! deliberately teaches nothing - the terminal omission - and the Run reads
//! `Status: passed`.
//!
//! Hole-poke notes:
//! - Would AOPV-003 pass if the command wrote the folder but no stamp? No.
//!   `SKILL.md` must contain the doctor-reported digest verbatim, so an
//!   unstamped folder fails the containment assert even with a perfect
//!   shape - and the digest is read live from the doctor the same test run,
//!   so a stale or hand-kept value cannot pass either.
//! - Would AOPV-004 pass on an empty `SKILL.md`? No. The forbidden-token
//!   scans would pass vacuously - which is exactly why the nine loop and
//!   never-touch markers are pinned against `SKILL.md` itself, so empty or
//!   marker-free content fails before the drive begins.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The fresh caller-given path both tests write the skill at: named for the
/// design's `ratmac-operator` folder, under a parent the fixture creates.
const SKILL_PATH: &str = "skills/ratmac-operator";

/// The snapshot-key prefix of every file a skill write may add.
const SKILL_PREFIX: &str = "skills/ratmac-operator/";

/// The occupied path: pre-created as an empty folder, so only an existence
/// check - never a content peek - can refuse it.
const OCCUPIED_PATH: &str = "skills/occupied-skill";

/// Render markers of the engine's own output (`src/teach.rs` PREFIX, the
/// State Prompt's guard list, the status report fields, the step-refusal
/// render). Quoting any of them in the skill is copying what running the
/// command would have printed (AOP-004).
const QUOTED_OUTPUT_MARKERS: [&str; 6] = [
    "next:",
    "Exit Guards:",
    "pending guard:",
    "step refused",
    "State: ",
    "Status: ",
];

/// Each invariant the skill must teach, as (the invariant, the marker its
/// prose must carry). All are pinned against `SKILL.md` itself: the entry
/// document carries the whole loop, references only deepen it.
const LOOP_MARKERS: [(&str, &str); 9] = [
    ("orient through the engine's own report", "rtm status"),
    ("read the State Prompt the engine renders", "prompt"),
    ("place the artifacts the pending guards declare", "artifact"),
    ("step the Run", "rtm step"),
    ("branch on the refusal's stable code", "refusal"),
    ("branch on the refusal's stable code", "code"),
    ("reach everything current through engine output", "output"),
    ("never write under the engine root", ".ratmac"),
    ("never write under the engine root", "never write"),
];

struct Fixture {
    base: PathBuf,
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

impl Fixture {
    /// A throwaway project whose `.ratmac` and `skills` parents already
    /// exist, so both the scaffold and the skill write at a legal path.
    fn new(label: &str) -> Self {
        let base = std::env::temp_dir().join(format!(
            "ratmac-t104-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&base);
        let root = base.join("project");
        fs::create_dir_all(root.join(".ratmac")).expect("create the Engine root");
        fs::create_dir_all(root.join("skills")).expect("create the skill parent");
        Fixture { base, root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Every file under the project root, keyed by `/`-separated relative path.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).expect("read directory") {
            let entry = entry.expect("read directory entry");
            if entry.file_type().expect("read entry type").is_dir() {
                walk(base, &entry.path(), files);
            } else {
                let relative = entry
                    .path()
                    .strip_prefix(base)
                    .expect("entry under root")
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(relative, fs::read(entry.path()).expect("read file"));
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

/// Every file of the written skill, keyed by `/`-separated relative path.
fn skill_files(skill: &Path) -> BTreeMap<String, String> {
    fn walk(base: &Path, dir: &Path, files: &mut BTreeMap<String, String>) {
        for entry in fs::read_dir(dir).expect("read the skill folder") {
            let entry = entry.expect("read the skill folder entry");
            if entry.file_type().expect("read entry type").is_dir() {
                walk(base, &entry.path(), files);
            } else {
                let relative = entry
                    .path()
                    .strip_prefix(base)
                    .expect("entry under the skill folder")
                    .to_string_lossy()
                    .replace('\\', "/");
                let content = fs::read_to_string(entry.path())
                    .unwrap_or_else(|error| panic!("read the skill file {relative}: {error}"));
                files.insert(relative, content);
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(skill, skill, &mut files);
    files
}

/// The running engine's identity, read live from the argument-free doctor's
/// own report (DFP-001): the complete 64-character lowercase digest of the
/// exact executable being run - the same binary both tests invoke.
fn engine_sha256(doctor_report: &str) -> String {
    let line = doctor_report
        .lines()
        .find(|line| line.starts_with("Engine: "))
        .expect("the doctor reports the Engine identity");
    const MARKER: &str = "(sha256: ";
    let start = line
        .find(MARKER)
        .expect("the Engine line carries the executable's sha256")
        + MARKER.len();
    let digest = line[start..]
        .split(')')
        .next()
        .expect("the digest closes its parenthesis");
    assert_eq!(
        digest.len(),
        64,
        "DFP-001: the identity stamp is the complete 64-character digest"
    );
    assert!(
        digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "the digest is lowercase hexadecimal: {digest}"
    );
    digest.to_owned()
}

/// Every CLI flag token in `text`: a whitespace-separated token, stripped of
/// markdown and punctuation wrapping, that spells `--<lowercase...>`. A
/// superset of the ` --[a-z]` shape, so backticks or parentheses cannot
/// smuggle a flag past the scan.
fn flag_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for word in text.split_whitespace() {
        let word = word
            .trim_start_matches(['(', '`'])
            .trim_end_matches([')', '`', ',', '.', ';', ':']);
        let bytes = word.as_bytes();
        if bytes.len() >= 3
            && bytes[0] == b'-'
            && bytes[1] == b'-'
            && bytes[2].is_ascii_lowercase()
            && !tokens.iter().any(|token| token == word)
        {
            tokens.push(word.to_owned());
        }
    }
    tokens
}

/// The authored `prompt = "..."` lines of a runbook, in declaration order -
/// the drive's only knowledge of the machine's prose.
fn authored_prompts(runbook: &str) -> Vec<String> {
    runbook
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("prompt = \"")?;
            Some(rest.strip_suffix('"')?.to_owned())
        })
        .collect()
}

/// Walk the skill's loop mechanically: `seed` is the orienting render, and
/// every following act is the exact command that render's one taught line
/// names, until a render deliberately teaches nothing - the terminal
/// omission. Every followed command must be accepted (exit 0), and every
/// followed render still carries authored State Prompt prose, so the loop's
/// read-the-prompt step is grounded at each turn, not assumed.
fn follow_taught_lines(
    fixture: &Fixture,
    seed: String,
    prompts: &[String],
    context: &str,
) -> String {
    let mut rendered = seed;
    for _ in 0..16 {
        let taught: Vec<&str> = rendered
            .lines()
            .filter(|line| line.starts_with("next: "))
            .collect();
        if taught.is_empty() {
            // The terminal omission: the engine stands behind nothing more.
            return rendered;
        }
        assert_eq!(
            taught.len(),
            1,
            "AOPV-004: {context}: one taught act at a time: {rendered}"
        );
        let argv = taught[0]
            .strip_prefix("next: ")
            .unwrap_or_else(|| panic!("AOPV-004: {context}: malformed taught line"))
            .split_whitespace()
            .collect::<Vec<_>>();
        let followed = fixture.rtm(&argv);
        let followed_text = text(&followed);
        assert!(
            followed.status.success(),
            "AOPV-004: {context}: the engine accepts the act it taught ({}): {followed_text}",
            taught[0]
        );
        assert!(
            prompts
                .iter()
                .any(|prompt| followed_text.contains(prompt.as_str())),
            "AOPV-004: {context}: each followed render still shows authored prompt prose: \
             {followed_text}"
        );
        rendered = followed_text;
    }
    panic!("AOPV-004: {context}: the taught loop never reached a terminal render: {rendered}");
}

/// AOPV-003 / AOP-003: `rtm skill <path>` writes one identity-stamped skill
/// folder at a fresh path and refuses an occupied one having written nothing
/// - the scaffold's discipline, carried from one file to one folder.
#[test]
fn the_skill_subcommand_writes_once_and_never_overwrites() {
    let fixture = Fixture::new("writes");

    // The running engine's identity, read live from the doctor's own report:
    // the stamp must be this digest, never a test-hardcoded one.
    let stamp = engine_sha256(&text(&fixture.rtm(&["doctor"])));

    // Fresh path: one folder written, exit 0, nothing else anywhere.
    let before = snapshot(&fixture.root);
    let written = fixture.rtm(&["skill", SKILL_PATH]);
    let written_text = text(&written);
    assert!(
        written.status.success(),
        "AOPV-003: rtm skill at a fresh path exits 0: {written_text}"
    );
    assert!(
        fixture.root.join(SKILL_PATH).is_dir(),
        "AOPV-003: the caller's path becomes the skill folder: {written_text}"
    );
    let after = snapshot(&fixture.root);
    let added: Vec<&String> = after
        .keys()
        .filter(|key| !before.contains_key(*key))
        .collect();
    for key in &added {
        assert!(
            key.starts_with(SKILL_PREFIX),
            "AOPV-003: every written byte sits under the caller's path, not {key}: {written_text}"
        );
    }
    for (key, bytes) in &before {
        assert_eq!(
            after.get(key),
            Some(bytes),
            "AOPV-003: no pre-existing byte changes: {key}"
        );
    }
    let skill_md = after
        .get(&format!("{SKILL_PREFIX}SKILL.md"))
        .unwrap_or_else(|| panic!("AOPV-003: the skill folder carries SKILL.md: {added:?}"))
        .to_vec();
    assert!(
        String::from_utf8_lossy(&skill_md).contains(&stamp),
        "AOPV-003: SKILL.md carries the writing engine's identity stamp; the doctor reports \
         {stamp}: {}",
        String::from_utf8_lossy(&skill_md)
    );
    let references_prefix = format!("{SKILL_PREFIX}references/");
    assert!(
        after.keys().any(|key| key.starts_with(&references_prefix)),
        "AOPV-003: the skill folder carries reference files beside SKILL.md: {added:?}"
    );

    // Occupied path: refused by name, nothing written anywhere - even though
    // the existing folder is empty, existence alone is the whole rule.
    fs::create_dir_all(fixture.root.join(OCCUPIED_PATH))
        .expect("pre-create the occupied path as an empty folder");
    let settled = snapshot(&fixture.root);
    let refused = fixture.rtm(&["skill", OCCUPIED_PATH]);
    let refused_text = text(&refused);
    assert!(
        !refused.status.success(),
        "AOPV-003: an occupied path refuses, even an empty folder: {refused_text}"
    );
    assert!(
        refused_text.contains("occupied-skill"),
        "AOPV-003: the refusal names the existing path: {refused_text}"
    );
    assert_eq!(
        settled,
        snapshot(&fixture.root),
        "AOPV-003: the refused write leaves the whole tree byte-identical"
    );
}

/// AOPV-004 / AOP-004: the written skill teaches only invariant behavior -
/// no flag tokens, no quoted output, the loop and the never-touch rules -
/// and a scaffolded Run driven by only that teaching plus live engine
/// output reaches terminal.
#[test]
fn the_skill_teaches_invariants_and_drives_a_run_to_terminal() {
    let fixture = Fixture::new("drive");

    // The runbook the drive runs on is the scaffold's own output (AAL-002),
    // and the authored prompts are parsed from what the scaffold wrote - the
    // drive never hard-codes the machine's prose.
    let scaffolded = fixture.rtm(&["scaffold", ".ratmac/ratmac.toml"]);
    let scaffolded_text = text(&scaffolded);
    assert!(
        scaffolded.status.success(),
        "the fixture runbook is the engine's own scaffold: {scaffolded_text}"
    );
    let runbook = fs::read_to_string(fixture.root.join(".ratmac/ratmac.toml"))
        .expect("read the scaffolded runbook");
    let prompts = authored_prompts(&runbook);
    assert_eq!(
        prompts.len(),
        2,
        "the scaffold declares one initial and one terminal State"
    );

    // The skill, written by the engine at a fresh path.
    let written = fixture.rtm(&["skill", SKILL_PATH]);
    let written_text = text(&written);
    assert!(
        written.status.success(),
        "AOPV-004: rtm skill writes the skill to scan: {written_text}"
    );
    let files = skill_files(&fixture.root.join(SKILL_PATH));

    // No CLI flag tokens in any written file. Zero exceptions: AOP-003
    // enumerates no flags, and the engine's own addressing refusal teaches
    // the run id, so no invariant the skill teaches needs one.
    for (name, content) in &files {
        let tokens = flag_tokens(content);
        assert!(
            tokens.is_empty(),
            "AOPV-004: {name} enumerates no CLI flags: {tokens:?}"
        );
    }

    // No quoted command output in any written file: the verbatim markers of
    // the engine's own renders. Anything current is reached by running the
    // command, never by copying what it printed.
    for (name, content) in &files {
        for marker in QUOTED_OUTPUT_MARKERS {
            assert!(
                !content.contains(marker),
                "AOPV-004: {name} must not quote engine output - {marker} is a render, \
                 not a rule"
            );
        }
    }

    // The invariant loop and the never-touch rules live in SKILL.md itself.
    let skill_md = files
        .get("SKILL.md")
        .expect("AOPV-004: SKILL.md is the skill's entry document");
    for (invariant, marker) in LOOP_MARKERS {
        assert!(
            skill_md.contains(marker),
            "AOPV-004: SKILL.md must teach the invariant {invariant:?} (marker {marker:?})"
        );
    }

    // The drive: only what the skill teaches plus live engine output. The
    // loop mints its Run, orients with `rtm status`, and from then on every
    // act is the exact command a rendered taught line names.
    let started = fixture.rtm(&["start"]);
    let started_text = text(&started);
    assert!(
        started.status.success(),
        "the drive mints its Run the one permitted way: {started_text}"
    );
    let walked = follow_taught_lines(
        &fixture,
        text(&fixture.rtm(&["status"])),
        &prompts,
        "the oriented loop",
    );
    let terminal_prompt = prompts
        .last()
        .expect("the terminal State's authored prompt");
    assert!(
        walked.contains(terminal_prompt.as_str()),
        "AOPV-004: the walk's last render is the terminal State's own prompt: {walked}"
    );

    // Settling the loop the same taught way: the Run reads passed.
    let confirmed = follow_taught_lines(
        &fixture,
        text(&fixture.rtm(&["status"])),
        &prompts,
        "the settled loop",
    );
    assert!(
        confirmed.contains("Status: passed"),
        "AOPV-004: the skill's loop drives the scaffolded Run to terminal: {confirmed}"
    );
}
