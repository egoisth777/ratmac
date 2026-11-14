//! t-104 / AOP-003, AOP-004: the engine writes its own operator skill;
//! t-148 / WTC-003: its move into place waits out a brief hold.
//!
//! AOPV-003 `the_skill_subcommand_writes_once_and_never_overwrites`
//! AOPV-004 `the_skill_teaches_invariants_and_drives_a_run_to_terminal`
//! WTCV-007 `wtcv_007` (Windows)
//! WTCV-008 `wtcv_008` (Windows)
//! WTCV-009 `wtcv_009`
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
//! WTCV-007..009 judge the move into place through the two test-only pauses
//! a `test-fault-injection` build of the engine holds: the move pause
//! (`RATMAC_TEST_SKILL_MOVE_PAUSE=before-first-attempt`), just before the
//! first attempt's check of the requested path, and the denial pause
//! (`RATMAC_TEST_SKILL_DENIAL_PAUSE=after-first-denial`), just after the
//! first denied move - each with its `_MARKER`, `_RELEASE`, and
//! `_TIMEOUT_MILLIS` companions, the pause pattern of `src/nested.rs`. A
//! check arms them only on the one `rtm skill` child it starts (SVE-006),
//! with the marker and release files in a scratch tree of their own outside
//! the project, so no project snapshot sees them. Every wait acts on a
//! marker's whole line, boundary name and newline (SVE-005), and a command
//! that exits without publishing that line fails the wait, naming its exit
//! and output; a marker is otherwise only checked absent after the command
//! has exited. The hold opens the partial folder's `SKILL.md` for reading
//! with read, write, and delete sharing (`FILE_SHARE_READ |
//! FILE_SHARE_WRITE | FILE_SHARE_DELETE`), the least exclusive hold another
//! program can take; on this host it denies the folder move with raw code 5
//! (`Access is denied.`), so the ticket's default mode is the one used.
//! Each check starts with an unheld write at `REFERENCE_PATH` that must
//! exit 0: the valid positive control, and the byte reference a retried
//! write must match.
//!
//! Hole-poke notes:
//! - Would WTCV-007 pass on an engine that never retries, or with a hold
//!   that denies nothing? No. It waits for the denial pause, which only a
//!   denied move reaches; without a denial the command exits unpaused and
//!   the wait fails, and without the retry the denied move refuses.
//! - Would WTCV-009 pass on an engine that checks the requested path only
//!   before the write? No. The folder appears after the write, at the move
//!   pause, and the denial pause's release is made in advance, so a command
//!   that moves or retries instead refuses with another text or leaves the
//!   denial marker behind.
//! - Would AOPV-003 pass if the command wrote the folder but no stamp? No.
//!   `SKILL.md` must contain the doctor-reported digest verbatim, so an
//!   unstamped folder fails the containment assert even with a perfect
//!   shape - and the digest is read live from the doctor the same test run,
//!   so a stale or hand-kept value cannot pass either.
//! - Would AOPV-004 pass on an empty `SKILL.md`? No. The forbidden-token
//!   scans would pass vacuously - which is exactly why the nine loop and
//!   never-touch markers are pinned against `SKILL.md` itself, so empty or
//!   marker-free content fails before the drive begins.

use ratmac_qa::support::{self, MarkerLine, MarkerWait, TempTree};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

/// The fresh caller-given path every test writes the skill at: named for the
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
    /// The owned scratch tree; dropping it removes the fixture.
    _base: TempTree,
    root: PathBuf,
}

impl Fixture {
    /// A throwaway project whose `.ratmac` and `skills` parents already
    /// exist, so both the scaffold and the skill write at a legal path.
    fn new(label: &str) -> Self {
        let base = TempTree::new(&format!("t104-{label}")).expect("create fixture tree");
        let root = base.join("project");
        fs::create_dir_all(root.join(".ratmac")).expect("create the Engine root");
        fs::create_dir_all(root.join("skills")).expect("create the skill parent");
        Fixture { _base: base, root }
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
        // WRS-006: a passed Run's addressed status is its history view, which
        // shows no current prompt and says so; every other render, including
        // every step, keeps its prose.
        let history = argv.starts_with(&["rtm", "status"])
            && followed_text.contains("Status: passed\n")
            && followed_text.contains("Current instructions: unavailable");
        assert!(
            history
                || prompts
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

// --- t-148 / WTC-003: the skill move retry ----------------------------------

/// The hidden sibling `rtm skill` builds `SKILL_PATH` in before the move into
/// place: `.<name>.rtm-skill-partial` beside the requested path.
const PARTIAL: &str = "skills/.ratmac-operator.rtm-skill-partial";

/// Where each WTC-003 check's unheld control write lands.
const REFERENCE_PATH: &str = "skills/reference-skill";

/// Today's success line for `SKILL_PATH`, exactly as standard output carries
/// it.
#[cfg(windows)]
const WROTE_SKILL_PATH: &str =
    "Wrote skills/ratmac-operator. Its SKILL.md carries the identity stamp of the engine that wrote it.\n";

/// The bound WTCV-008 states: the lock module's five-second wait
/// (`WAIT_TIMEOUT`), measured from the first denial.
#[cfg(windows)]
const MOVE_RETRY_BOUND: Duration = Duration::from_secs(5);

/// `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`: the least
/// exclusive hold another program can take on a file.
#[cfg(windows)]
const SHARE_READ_WRITE_DELETE: u32 = 0x1 | 0x2 | 0x4;

/// Each pause's timeout companion: the seam's 30-second cap, far beyond any
/// wait a check makes before it releases.
const PAUSE_TIMEOUT_MILLIS: &str = "30000";

/// How long a check waits for a pause's whole line.
const PAUSE_LIMIT: Duration = Duration::from_secs(20);

/// One slice of that wait; between slices the check asks whether the command
/// has already exited.
const PAUSE_SLICE: Duration = Duration::from_millis(200);

/// How long a released command may take to exit before the check kills it
/// and fails.
const EXIT_LIMIT: Duration = Duration::from_secs(30);

/// The two test-only pauses of the skill move.
#[derive(Clone, Copy)]
enum Pause {
    /// `RATMAC_TEST_SKILL_MOVE_PAUSE`: before the first attempt's check of
    /// the requested path.
    Move,
    /// `RATMAC_TEST_SKILL_DENIAL_PAUSE`: after the first denied move.
    Denial,
}

impl Pause {
    /// The one boundary the pause holds, which its marker's line names.
    fn boundary(self) -> &'static str {
        match self {
            Pause::Move => "before-first-attempt",
            Pause::Denial => "after-first-denial",
        }
    }

    /// `<stem>.marker` and `<stem>.release` name its files in the pause tree.
    fn stem(self) -> &'static str {
        match self {
            Pause::Move => "move",
            Pause::Denial => "denial",
        }
    }
}

/// One `rtm skill SKILL_PATH` child with both pauses armed in its own
/// environment only (SVE-006): the test process's environment never holds a
/// `RATMAC_TEST_*` setting. Its marker and release files live in a scratch
/// tree of its own, outside the project. Dropping it kills and reaps the
/// child before that tree goes, so a failing check never leaves a paused
/// engine running.
struct Armed {
    child: Child,
    pauses: TempTree,
}

impl Armed {
    fn start(fixture: &Fixture, label: &str) -> Self {
        let pauses =
            TempTree::new(&format!("t104-{label}-pause")).expect("create the pause file tree");
        // Each setting path is spelled `pauses.join(..)` in place: SVEV-015
        // follows that local into `Armed`'s `pauses` field, which ties the
        // waits in `await_pause` to these settings; spelled through a local
        // closure, the same paths left those waits untied.
        let child = support::command(ratmac_qa::engine_bin!(), &fixture.root)
            .args(["skill", SKILL_PATH])
            .env("RATMAC_TEST_SKILL_MOVE_PAUSE", Pause::Move.boundary())
            .env(
                "RATMAC_TEST_SKILL_MOVE_PAUSE_MARKER",
                pauses.join(format!("{}.marker", Pause::Move.stem())),
            )
            .env(
                "RATMAC_TEST_SKILL_MOVE_PAUSE_RELEASE",
                pauses.join(format!("{}.release", Pause::Move.stem())),
            )
            .env(
                "RATMAC_TEST_SKILL_MOVE_PAUSE_TIMEOUT_MILLIS",
                PAUSE_TIMEOUT_MILLIS,
            )
            .env("RATMAC_TEST_SKILL_DENIAL_PAUSE", Pause::Denial.boundary())
            .env(
                "RATMAC_TEST_SKILL_DENIAL_PAUSE_MARKER",
                pauses.join(format!("{}.marker", Pause::Denial.stem())),
            )
            .env(
                "RATMAC_TEST_SKILL_DENIAL_PAUSE_RELEASE",
                pauses.join(format!("{}.release", Pause::Denial.stem())),
            )
            .env(
                "RATMAC_TEST_SKILL_DENIAL_PAUSE_TIMEOUT_MILLIS",
                PAUSE_TIMEOUT_MILLIS,
            )
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start the armed rtm skill");
        Armed { child, pauses }
    }

    fn marker(&self, pause: Pause) -> PathBuf {
        self.pauses.join(format!("{}.marker", pause.stem()))
    }

    /// Let the command past `pause`: its release file now exists.
    fn release(&self, pause: Pause) {
        let release = self.pauses.join(format!("{}.release", pause.stem()));
        fs::write(&release, b"release\n")
            .unwrap_or_else(|error| panic!("write {}: {error}", release.display()));
    }

    /// Wait until `pause`'s marker holds its whole boundary line, asking
    /// between short slices whether the command has exited: a command that
    /// exits without publishing the line fails the check, naming its exit and
    /// output.
    ///
    /// Each slice is the shared whole-line pause-marker wait (SVE-005),
    /// `support::await_marker_line` with the boundary line, closing newline
    /// included, as its exact line: the one place this suite reads a marker
    /// while its command runs.
    fn await_pause(&mut self, pause: Pause, id: &str) {
        // Spelled from the `pauses` field rather than through `marker`:
        // SVEV-015 ties a wait on this path to `start`'s `_MARKER`
        // settings; through `marker`'s result it left the wait untied.
        let marker = self.pauses.join(format!("{}.marker", pause.stem()));
        let line = format!("{}\n", pause.boundary());
        let started = Instant::now();
        loop {
            let last =
                match support::await_marker_line(&marker, MarkerLine::Exact(&line), PAUSE_SLICE) {
                    Ok(_) => return,
                    Err(MarkerWait::OtherText(bytes)) => panic!(
                        "{id}: {} holds another line than {line:?}: {:?}",
                        marker.display(),
                        String::from_utf8_lossy(&bytes)
                    ),
                    Err(MarkerWait::NotYetPublished(last)) => last,
                };
            if let Some(status) = self.child.try_wait().expect("poll the armed rtm skill") {
                let output = self.output(status);
                panic!(
                    "{id}: rtm skill never published the whole line {line:?} at {}; it exited \
                     {status}: {}",
                    marker.display(),
                    text(&output)
                );
            }
            assert!(
                started.elapsed() < PAUSE_LIMIT,
                "{id}: rtm skill published no whole line {line:?} at {} within {PAUSE_LIMIT:?}; \
                 last read: {:?}",
                marker.display(),
                last.map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            );
        }
    }

    /// Wait at most `EXIT_LIMIT` for the released command to exit: its
    /// output, and the moment its exit was seen.
    fn finish(&mut self, id: &str) -> (Output, Instant) {
        let started = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().expect("poll the armed rtm skill") {
                let exited = Instant::now();
                return (self.output(status), exited);
            }
            if started.elapsed() >= EXIT_LIMIT {
                let _ = self.child.kill();
                let status = self.child.wait().expect("reap the killed rtm skill");
                panic!(
                    "{id}: the released rtm skill did not exit within {EXIT_LIMIT:?}; killed: {}",
                    text(&self.output(status))
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// The exited command's output, read from its pipes.
    fn output(&mut self, status: ExitStatus) -> Output {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        if let Some(mut pipe) = self.child.stdout.take() {
            pipe.read_to_end(&mut stdout)
                .expect("read rtm skill's stdout");
        }
        if let Some(mut pipe) = self.child.stderr.take() {
            pipe.read_to_end(&mut stderr)
                .expect("read rtm skill's stderr");
        }
        Output {
            status,
            stdout,
            stderr,
        }
    }
}

impl Drop for Armed {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Open `file` for reading with read, write, and delete sharing, the way
/// another program - an indexer, a scanner - briefly holds a just-written
/// file. Holding the partial folder's `SKILL.md` this way denies moving that
/// folder with raw code 5 on this host.
#[cfg(windows)]
fn hold(file: &Path) -> fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .read(true)
        .share_mode(SHARE_READ_WRITE_DELETE)
        .open(file)
        .unwrap_or_else(|error| panic!("hold {}: {error}", file.display()))
}

/// The valid positive control each WTC-003 check starts from: an unheld
/// `rtm skill` at `REFERENCE_PATH` exits 0, and its folder's files are the
/// byte reference a retried write must match.
fn unheld_reference(fixture: &Fixture, id: &str) -> BTreeMap<String, Vec<u8>> {
    let written = fixture.rtm(&["skill", REFERENCE_PATH]);
    assert!(
        written.status.success(),
        "{id}: the unheld control write exits 0: {}",
        text(&written)
    );
    snapshot(&fixture.root.join(REFERENCE_PATH))
}

/// WTCV-007 / WTC-003: a brief hold ends and the move completes. At the move
/// pause the check holds the partial folder's `SKILL.md`, releases, waits for
/// the denial pause - which only a denied move reaches - closes the file,
/// and releases again: the command exits 0 with today's success line, its
/// folder is byte-identical to the unheld write, and no partial folder
/// remains.
#[cfg(windows)]
#[test]
fn wtcv_007() {
    let fixture = Fixture::new("wtcv-007");
    let reference = unheld_reference(&fixture, "WTCV-007");
    let mut armed = Armed::start(&fixture, "wtcv-007");
    armed.await_pause(Pause::Move, "WTCV-007");
    let held = hold(&fixture.root.join(PARTIAL).join("SKILL.md"));
    armed.release(Pause::Move);
    armed.await_pause(Pause::Denial, "WTCV-007");
    drop(held);
    armed.release(Pause::Denial);
    let (output, _) = armed.finish("WTCV-007");
    let shown = text(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "WTCV-007: the retried move exits 0: {shown}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        WROTE_SKILL_PATH,
        "WTCV-007: standard output is today's success line: {shown}"
    );
    assert_eq!(
        snapshot(&fixture.root.join(SKILL_PATH)),
        reference,
        "WTCV-007: the retried folder's files are byte-identical to the unheld write"
    );
    assert!(
        !fixture.root.join(PARTIAL).exists(),
        "WTCV-007: no partial folder remains: {shown}"
    );
}

/// WTCV-008 / WTC-003: a hold that outlasts the bound keeps today's refusal.
/// The check holds the partial folder's `SKILL.md` from the move pause until
/// the command exits: exit 2 with exactly today's `cannot write` line carrying
/// the access denial, no sooner than five seconds after the move pause was
/// released, and nothing at the requested path.
#[cfg(windows)]
#[test]
fn wtcv_008() {
    let fixture = Fixture::new("wtcv-008");
    unheld_reference(&fixture, "WTCV-008");
    let mut armed = Armed::start(&fixture, "wtcv-008");
    armed.await_pause(Pause::Move, "WTCV-008");
    let held = hold(&fixture.root.join(PARTIAL).join("SKILL.md"));
    armed.release(Pause::Move);
    // Taken once the release file exists, so the measured wait never exceeds
    // the real one.
    let released = Instant::now();
    armed.await_pause(Pause::Denial, "WTCV-008");
    armed.release(Pause::Denial);
    let (output, exited) = armed.finish("WTCV-008");
    drop(held);
    let shown = text(&output);
    assert_eq!(
        output.status.code(),
        Some(2),
        "WTCV-008: a denial outlasting the bound refuses with exit 2: {shown}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "rtm: skill: cannot write skills/ratmac-operator: Access is denied. (os error 5)\n",
        "WTCV-008: standard error is exactly today's refusal with the denial"
    );
    let waited = exited.duration_since(released);
    assert!(
        waited >= MOVE_RETRY_BOUND,
        "WTCV-008: the refusal comes no sooner than {MOVE_RETRY_BOUND:?} after the release, \
         not after {waited:?}"
    );
    match fs::symlink_metadata(fixture.root.join(SKILL_PATH)) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        other => panic!("WTCV-008: the requested path stays absent, not {other:?}"),
    }
}

/// WTCV-009 / WTC-003: a path that appears is never overwritten. While the
/// command waits at the move pause the check creates a folder holding one
/// marker file at the requested path, then releases: without reaching the
/// denial pause the command exits 2 with today's `already exists` refusal,
/// the folder and its file are byte-identical, and no partial folder remains.
#[test]
fn wtcv_009() {
    let fixture = Fixture::new("wtcv-009");
    unheld_reference(&fixture, "WTCV-009");
    let mut armed = Armed::start(&fixture, "wtcv-009");
    // Released in advance: a command that reached the denial pause would pass
    // it at once and leave its marker, never hang.
    armed.release(Pause::Denial);
    armed.await_pause(Pause::Move, "WTCV-009");
    let occupied = fixture.root.join(SKILL_PATH);
    fs::create_dir(&occupied).expect("create a folder at the requested path");
    fs::write(occupied.join("marker.txt"), b"wtcv-009 occupant\n")
        .expect("write the occupant's marker file");
    let occupant = snapshot(&occupied);
    armed.release(Pause::Move);
    let (output, _) = armed.finish("WTCV-009");
    let shown = text(&output);
    assert_eq!(
        output.status.code(),
        Some(2),
        "WTCV-009: a path that appeared refuses with exit 2: {shown}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "rtm: skill: skills/ratmac-operator already exists; the skill write never overwrites - \
         choose a new path or delete that one yourself\n",
        "WTCV-009: standard error is exactly today's occupied-path refusal"
    );
    assert!(
        !armed.marker(Pause::Denial).exists(),
        "WTCV-009: the command never reached the pause after a denied move: {shown}"
    );
    assert_eq!(
        snapshot(&occupied),
        occupant,
        "WTCV-009: the folder and its marker file stay byte-identical"
    );
    assert!(
        !fixture.root.join(PARTIAL).exists(),
        "WTCV-009: no partial folder remains: {shown}"
    );
}
