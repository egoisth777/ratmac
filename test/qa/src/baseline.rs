//! The freeze Engine, side by side with today's.
//!
//! `SVC-007` says the state-vocabulary cutover changed names only. Proving
//! that needs the before as well as the after, so this module builds the
//! Engine as it stood at the freeze commit into a throwaway directory and
//! offers the one translation table that separates a renamed word from a
//! changed behavior.
//!
//! Nothing here touches the repository: the freeze tree is extracted with
//! `git archive` into a temporary directory and built with its own build
//! directory, so no checkout, index, or worktree is disturbed.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

/// The commit the goal bundle was frozen at, before the first rename landed.
pub const FREEZE_COMMIT: &str = "4f78de5";

/// The words the cutover changed, longest first so a longer phrase is
/// erased before any word inside it.
///
/// Each row reads: what the freeze wrote, what today writes, and the neutral
/// token both collapse to. Comparing two texts after this collapse asks the
/// only question this ticket cares about - is anything different once the
/// vocabulary is set aside?
pub const VOCABULARY: &[(&str, &str, &str)] = &[
    // The doctor's roster line. At the freeze the bare word `State` on this
    // line meant the file, which is exactly the collision this cutover
    // resolved, so the label now names the Run Record instead.
    (
        "State: .ratmac/runs/",
        "Run Record: .ratmac/runs/",
        "<roster-line>",
    ),
    ("State File", "Run Record", "<record>"),
    ("state file", "run record", "<record>"),
    ("state.toml", "run.toml", "<record-file>"),
    ("state_path", "record_path", "<record-path>"),
    ("PhasePrompt", "StatePrompt", "<prompt-type>"),
    ("Phases", "States", "<positions>"),
    ("phases", "states", "<positions>"),
    ("Phase", "State", "<position>"),
    ("phase", "state", "<position>"),
];

/// Translate freeze-era text into today's words.
pub fn to_today(text: &str) -> String {
    let mut out = text.to_owned();
    for (before, after, _) in VOCABULARY {
        out = out.replace(before, after);
    }
    out
}

/// Erase the vocabulary from a text, whichever side of the cutover wrote it.
///
/// Two texts with the same canonical form differ only in the words the
/// cutover renamed; any other edit survives the erasure and shows up as a
/// difference.
pub fn canonical(text: &str) -> String {
    let mut out = text.to_owned();
    for (before, after, token) in VOCABULARY {
        out = out.replace(before, token).replace(after, token);
    }
    out
}

/// The repository this harness was compiled from.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root resolves")
}

/// The freeze Engine, built once per test process.
///
/// Returns the built command's path. The extraction directory lives for the
/// life of the process on purpose: several checks share one build.
pub fn engine() -> &'static Path {
    static ENGINE: LazyLock<PathBuf> =
        LazyLock::new(|| build(&repo_root()).expect("the freeze Engine builds offline"));
    ENGINE.as_path()
}

fn build(repo_root: &Path) -> Result<PathBuf, String> {
    // Keyed by the freeze commit alone, so repeated runs reuse one build;
    // cargo's own build lock serializes two processes that arrive together.
    let base = std::env::temp_dir().join(format!("ratmac-freeze-{FREEZE_COMMIT}"));
    let tree = base.join("tree");
    let build_dir = base.join("build");
    fs::create_dir_all(&tree).map_err(|error| format!("create {}: {error}", tree.display()))?;

    let tarball = base.join("freeze.tar");
    let archive = Command::new("git")
        .args([
            "archive",
            "--format=tar",
            "--output",
            &tarball.to_string_lossy(),
            FREEZE_COMMIT,
        ])
        .current_dir(repo_root)
        .output()
        .map_err(|error| format!("run git archive: {error}"))?;
    if !archive.status.success() {
        return Err(format!(
            "git archive {FREEZE_COMMIT} failed: {}",
            String::from_utf8_lossy(&archive.stderr)
        ));
    }

    let extract = Command::new("tar")
        .args(["-xf", &tarball.to_string_lossy()])
        .current_dir(&tree)
        .output()
        .map_err(|error| format!("run tar: {error}"))?;
    if !extract.status.success() {
        return Err(format!(
            "extracting the freeze tree failed: {}",
            String::from_utf8_lossy(&extract.stderr)
        ));
    }

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let built = Command::new(cargo)
        // The same feature set the harness's own Engine is built with, so
        // fault-injection scenarios can be compared at all.
        .args([
            "build",
            "--offline",
            "--bin",
            "rtm",
            "--features",
            "test-fault-injection",
        ])
        .current_dir(&tree)
        .env("CARGO_TARGET_DIR", &build_dir)
        .output()
        .map_err(|error| format!("build the freeze Engine: {error}"))?;
    if !built.status.success() {
        return Err(format!(
            "the freeze Engine did not build: {}",
            String::from_utf8_lossy(&built.stderr)
        ));
    }

    let binary = build_dir
        .join("debug")
        .join(format!("rtm{}", std::env::consts::EXE_SUFFIX));
    if !binary.is_file() {
        return Err(format!("no freeze Engine at {}", binary.display()));
    }
    Ok(binary)
}

/// Today's Engine, rebuilt before the first comparison.
///
/// Cargo hands a built binary's path only to the package that declares it, so
/// a private lane crate cannot read `CARGO_BIN_EXE_rtm-qa`. Building the same
/// target here gives every caller the same fresh artifact - never a stale one
/// left in the build directory by an older run.
pub fn today_engine() -> &'static Path {
    static ENGINE: LazyLock<PathBuf> = LazyLock::new(|| {
        let root = repo_root();
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
        let built = Command::new(cargo)
            .args(["build", "--offline", "-p", "ratmac-qa", "--bin", "rtm-qa"])
            .current_dir(&root)
            .output()
            .expect("build today's Engine");
        assert!(
            built.status.success(),
            "today's Engine must build: {}",
            String::from_utf8_lossy(&built.stderr)
        );
        let binary = root
            .join("target/debug")
            .join(format!("rtm-qa{}", std::env::consts::EXE_SUFFIX));
        assert!(binary.is_file(), "no Engine at {}", binary.display());
        binary
    });
    ENGINE.as_path()
}

/// One command a caller can run, written in the freeze's words.
#[derive(Debug, Clone)]
pub struct Scenario {
    /// How a failure names this run.
    pub name: String,
    /// The arguments; they are translated for today's Engine.
    pub args: Vec<String>,
    /// Environment the command runs with, for the fault-injection lanes.
    pub env: Vec<(String, String)>,
}

/// Name one command and its arguments.
pub fn scenario(name: &str, args: &[&str]) -> Scenario {
    Scenario {
        name: name.to_owned(),
        args: args.iter().map(|argument| (*argument).to_owned()).collect(),
        env: Vec::new(),
    }
}

impl Scenario {
    /// Add one environment variable to this command.
    #[must_use]
    pub fn with_env(mut self, name: &str, value: &str) -> Self {
        self.env.push((name.to_owned(), value.to_owned()));
        self
    }
}

/// What one command did, with everything volatile removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The scenario that produced it.
    pub name: String,
    /// The process exit code, or `None` when a signal ended it.
    pub code: Option<i32>,
    /// Merged stdout and stderr, normalized.
    pub text: String,
}

/// Run one command and normalize everything a comparison must ignore.
pub fn run(engine: &Path, root: &Path, scenario: &Scenario, translate: bool) -> Outcome {
    let owned: Vec<String> = scenario
        .args
        .iter()
        .map(|argument| {
            let argument = if translate {
                to_today(argument)
            } else {
                (*argument).to_owned()
            };
            // The confirmation phrase names the project directory, which
            // differs between the two fixture trees, so a scenario writes
            // `<project>` and each side fills in its own name.
            argument.replace(
                "<project>",
                &root
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            )
        })
        .collect();
    let mut command = Command::new(engine);
    command.args(&owned).current_dir(root);
    for (name, value) in &scenario.env {
        // A marker or release path names a file, so it needs translating too.
        let value = if translate {
            to_today(value)
        } else {
            value.clone()
        };
        command.env(name, value.replace("<root>", &root.to_string_lossy()));
    }
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("run `{}`: {error}", scenario.name));
    let raw = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Outcome {
        name: scenario.name.clone(),
        code: output.status.code(),
        text: normalize(&raw, root, engine),
    }
}

/// Strip the parts of a report that differ between two runs of the same
/// command: the tree it ran in, the command's own path, content hashes, and
/// elapsed times.
pub fn normalize(text: &str, root: &Path, engine: &Path) -> String {
    let mut out = text.replace("\r\n", "\n");
    for path in [engine.to_path_buf(), root.to_path_buf()] {
        let shown = path.to_string_lossy().into_owned();
        // Windows canonicalization prefixes a verbatim marker the reports
        // never print, so the bare form has to be masked as well.
        let bare = shown.strip_prefix("\\\\?\\").unwrap_or(&shown).to_owned();
        for form in [shown, bare] {
            let windows = form.replace('/', "\\");
            for spelling in [
                form.clone(),
                form.replace('\\', "/"),
                windows.clone(),
                // TOML doubles a backslash, so a recorded path reads
                // differently in a file than it does in a report.
                windows.replace('\\', "\\\\"),
            ] {
                out = out.replace(&spelling, "<path>");
            }
        }
    }
    let mut cleaned = String::with_capacity(out.len());
    for line in out.lines() {
        cleaned.push_str(&mask_hex(line));
        cleaned.push('\n');
    }
    cleaned
}

/// Replace every run of 16 or more hex digits with a placeholder: two builds
/// of different sources never share a hash, and a hash is not behavior.
fn mask_hex(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut run = String::new();
    for character in line.chars() {
        if character.is_ascii_hexdigit() {
            run.push(character);
            continue;
        }
        flush(&mut run, &mut out);
        out.push(character);
    }
    flush(&mut run, &mut out);
    out
}

fn flush(run: &mut String, out: &mut String) {
    if run.len() >= 16 {
        out.push_str("<hash>");
    } else {
        out.push_str(run);
    }
    run.clear();
}

/// Every file under `root`, with its text content, keyed by a
/// forward-slashed relative path. Binary files are recorded by length.
pub fn tree(root: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(base, &path, out);
            continue;
        }
        let shown = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let body = match fs::read(&path) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => text.replace("\r\n", "\n"),
                Err(error) => format!("<{} bytes>", error.into_bytes().len()),
            },
            Err(error) => format!("<unreadable: {error}>"),
        };
        out.insert(shown, body);
    }
}

/// The behavioral suites [res-122](../../../.arca/residual/res-122.md) names
/// as the ones that must stay meaning-for-meaning identical.
pub const BEHAVIORAL_SUITES: &[&str] = &[
    "t047_receipts",
    "t048_contract_gates",
    "t049_completion_gate",
    "t050_blocked_route",
    "t051_abandon",
    "t059_run_residency",
    "t061_input_routing",
    "t063_run_completion",
    "t066_spawn_ledger",
    "t067_cycle_termination",
    "t073_lock_split",
    "t075_engine_transition_log",
];

/// Changes to a baseline line that this sprint did not make, each with the
/// ticket that did and the reason it is not a behavior change.
///
/// A line is excused only when it contains one of these fragments, so the
/// exception stays narrow and reviewable rather than a blanket skip.
pub const DECLARED_EXCEPTIONS: &[(&str, &str)] = &[(
    "CARGO_BIN_EXE_rtm",
    "t-086 gave the harness build its own target name; every suite now finds \
     the Engine through ratmac_qa::engine_bin!()",
)];

/// A file as the freeze commit holds it.
pub fn freeze_file(repo_root: &Path, relative: &str) -> String {
    let output = Command::new("git")
        .args(["show", &format!("{FREEZE_COMMIT}:{relative}")])
        .current_dir(repo_root)
        .output()
        .expect("read a file out of the freeze commit");
    assert!(
        output.status.success(),
        "{relative} must exist at the freeze: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("the freeze file is UTF-8")
}

/// Every `#[test]` function name in a suite.
pub fn test_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut marked = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[test]") {
            marked = true;
            continue;
        }
        if !marked {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("fn ") {
            names.push(rest.split('(').next().unwrap_or_default().trim().to_owned());
            marked = false;
        }
    }
    names
}

/// Every assertion a suite makes, as one whitespace-flattened string each.
///
/// Flattening matters: `rustfmt` moves an assertion across lines without
/// changing what it claims, and that must not read as a weakened check.
pub fn assertions(source: &str) -> Vec<String> {
    let characters: Vec<char> = source.chars().collect();
    let mut found = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        let ahead: String = characters[index..].iter().take(12).collect();
        let Some(name) = ["assert_eq!", "assert_ne!", "assert!", "panic!"]
            .iter()
            .find(|name| ahead.starts_with(**name))
        else {
            index += 1;
            continue;
        };
        let start = index + name.len();
        let mut depth = 0usize;
        let mut end = start;
        while end < characters.len() {
            match characters[end] {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            end += 1;
        }
        let body: String = characters[start..end.min(characters.len())]
            .iter()
            .collect();
        found.push(format!("{name}{}", flatten(&body)));
        index = end.max(index + 1);
    }
    found
}

/// Flatten an assertion so `rustfmt`'s line breaks cannot read as a change.
///
/// Whitespace between two word characters is kept as one space, because that
/// is prose inside a message; whitespace anywhere else - after a bracket,
/// around a comma, before a quote - is dropped, because that is only the
/// formatter deciding where a longer name no longer fits on one line.
fn flatten(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < characters.len() {
        if !characters[index].is_whitespace() {
            out.push(characters[index]);
            index += 1;
            continue;
        }
        let mut ahead = index;
        while ahead < characters.len() && characters[ahead].is_whitespace() {
            ahead += 1;
        }
        let word = |character: Option<char>| {
            character.is_some_and(|character| character.is_alphanumeric() || character == '_')
        };
        if word(out.chars().last()) && word(characters.get(ahead).copied()) {
            out.push(' ');
        }
        index = ahead;
    }
    out.trim().to_owned()
}

/// Compare one suite as the freeze wrote it against the same suite today,
/// and name every check that lost or changed its meaning.
///
/// An empty answer means the only differences were renamed words.
pub fn inventory_differences(relative: &str, before: &str, after: &str) -> Vec<String> {
    let mut failures = Vec::new();

    let today_names: Vec<String> = test_names(after)
        .iter()
        .map(|name| canonical(name))
        .collect();
    for name in test_names(before) {
        if !today_names.contains(&canonical(&name)) {
            failures.push(format!("{relative}: the freeze check `{name}` is gone"));
        }
    }

    if after.matches("#[ignore").count() > before.matches("#[ignore").count() {
        failures.push(format!(
            "{relative}: a check was turned into an ignored one"
        ));
    }

    let today_assertions: Vec<String> = assertions(after)
        .iter()
        .map(|assertion| canonical(assertion))
        .collect();
    for assertion in assertions(before) {
        if today_assertions.contains(&canonical(&assertion)) {
            continue;
        }
        if DECLARED_EXCEPTIONS
            .iter()
            .any(|(fragment, _)| assertion.contains(fragment))
        {
            continue;
        }
        failures.push(format!(
            "{relative}: the freeze asserted `{assertion}`, and no check asserts it today"
        ));
    }

    let (before_count, after_count) = (assertions(before).len(), today_assertions.len());
    if after_count < before_count {
        failures.push(format!(
            "{relative}: {before_count} assertions at the freeze, {after_count} today"
        ));
    }
    failures
}

/// The runbook the paired fixtures use unless a check supplies its own.
pub const DEFAULT_RUNBOOK: &str = "[roots]\n\
     ticket = \".arca/ticket\"\n\
     \n\
     [phases.intake]\n\
     prompt = \"Integrate the issues.\"\n\
     \n\
     [phases.build]\n\
     prompt = \"Build the ticket.\"\n\
     guards = [{ kind = \"files_exact\", root = \"ticket\", path = \"done\", entries = [\"done.txt\"] }]\n\
     \n\
     [phases.review]\n\
     prompt = \"Review the ticket.\"\n\
     \n\
     [[transitions]]\n\
     from = \"intake\"\n\
     to = \"build\"\n\
     \n\
     [[transitions]]\n\
     from = \"build\"\n\
     to = \"review\"\n\
     \n\
     [[transitions]]\n\
     from = \"build\"\n\
     to = \"intake\"\n\
     blocked-route = true\n";

/// What one scenario looked like on both sides of the cutover.
#[derive(Debug, Clone)]
pub struct Comparison {
    /// Everything that differed by more than a renamed word.
    pub differences: Vec<String>,
    /// The exit code the freeze Engine returned, so a check can require the
    /// scenario set to move the machine and not only collect refusals.
    pub freeze_code: Option<i32>,
}

/// Two identical projects - one written in the freeze's words, one in
/// today's - each with the Engine of its own era.
///
/// Every comparison this ticket makes runs the same command against both and
/// asks whether anything but the vocabulary differs.
pub struct Pair {
    /// The project the freeze Engine runs in.
    pub freeze_root: PathBuf,
    /// The project today's Engine runs in.
    pub today_root: PathBuf,
    freeze_engine: PathBuf,
    today_engine: PathBuf,
}

impl Pair {
    /// Build both projects from one description, written in freeze words.
    ///
    /// `extra` adds or replaces files, relative to the project root; its
    /// contents are translated for the today side exactly as the runbook is.
    pub fn new(label: &str, today_engine: &Path, runbook: &str, extra: &[(&str, &str)]) -> Self {
        let stamp = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        );
        // Both trees end in the same directory name on purpose: the name of
        // the project a caller is in shows up in reports, and a difference
        // there would read as a behavior difference.
        let freeze_root = std::env::temp_dir()
            .join(format!("ratmac-{label}-freeze-{stamp}"))
            .join("project");
        let today_root = std::env::temp_dir()
            .join(format!("ratmac-{label}-today-{stamp}"))
            .join("project");
        for (root, today) in [(&freeze_root, false), (&today_root, true)] {
            write_project(root, runbook, extra, today);
        }
        Self {
            freeze_root,
            today_root,
            freeze_engine: engine().to_path_buf(),
            today_engine: today_engine.to_path_buf(),
        }
    }

    /// Run one command on both sides, once each, and report what differed.
    ///
    /// One run per side is deliberate: these commands mutate the project, so
    /// a second probing run would move the machine and desync the pair.
    pub fn compare(&self, scenario: &Scenario) -> Comparison {
        let before = run(&self.freeze_engine, &self.freeze_root, scenario, false);
        let after = run(&self.today_engine, &self.today_root, scenario, true);
        let mut differences = Vec::new();
        if before.code != after.code {
            differences.push(format!(
                "`{}`: exit code {:?} at the freeze, {:?} today",
                scenario.name, before.code, after.code
            ));
        }
        let (expected, seen) = (canonical(&before.text), canonical(&after.text));
        if expected != seen {
            differences.push(format!(
                "`{}`: the report changed by more than its words\n--- freeze, vocabulary erased ---\n{expected}--- today, vocabulary erased ---\n{seen}",
                scenario.name
            ));
        }
        Comparison {
            differences,
            freeze_code: before.code,
        }
    }

    /// Write one file into both projects, in each side's own words.
    ///
    /// Scenarios that hand the Engine an input mid-Run need this: the file
    /// only makes sense once a Run directory exists.
    pub fn write_both(&self, relative: &str, body: &str) {
        for (root, today) in [(&self.freeze_root, false), (&self.today_root, true)] {
            let path = root.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create a directory for a mid-Run file");
            }
            let body = if today {
                to_today(body)
            } else {
                body.to_owned()
            };
            fs::write(path, body).expect("write a mid-Run file");
        }
    }

    /// Remove one file from both projects, ignoring an absent one.
    pub fn remove_both(&self, relative: &str) {
        for root in [&self.freeze_root, &self.today_root] {
            let _ = fs::remove_file(root.join(relative));
        }
    }

    /// One file's bytes on each side, with the vocabulary erased.
    pub fn both(&self, relative: &str) -> (Option<String>, Option<String>) {
        let read = |root: &Path| {
            fs::read_to_string(root.join(relative))
                .ok()
                .map(|body| canonical(&body))
        };
        (read(&self.freeze_root), read(&self.today_root))
    }

    /// Every path the freeze Engine's project holds, vocabulary erased.
    ///
    /// A lane uses this to prove its scenarios actually built something, so a
    /// comparison of two empty trees can never read as a passing proof.
    pub fn freeze_paths(&self) -> Vec<String> {
        tree(&self.freeze_root)
            .keys()
            .map(|path| canonical(path))
            .collect()
    }

    /// Name every difference between what the two Engines left on disk.
    pub fn tree_differences(&self) -> Vec<String> {
        let before = self.canonical_tree(&self.freeze_root, &self.freeze_engine);
        let after = self.canonical_tree(&self.today_root, &self.today_engine);
        let mut differences = Vec::new();
        for (path, body) in &before {
            match after.get(path) {
                None => differences.push(format!(
                    "the freeze wrote `{path}`; today's Engine did not"
                )),
                Some(today) if today != body => differences.push(format!(
                    "`{path}` differs by more than its words\n--- freeze, vocabulary erased ---\n{body}--- today, vocabulary erased ---\n{today}"
                )),
                Some(_) => {}
            }
        }
        for path in after.keys() {
            if !before.contains_key(path) {
                differences.push(format!(
                    "today's Engine wrote `{path}`, which the freeze never wrote"
                ));
            }
        }
        differences
    }

    /// One tree with volatile bytes masked and the vocabulary erased.
    ///
    /// The two Engines are different builds by construction, so the path each
    /// records for itself and every hash over differing bytes must differ;
    /// those are masked, and everything else must match.
    fn canonical_tree(&self, root: &Path, engine: &Path) -> BTreeMap<String, String> {
        tree(root)
            .iter()
            .map(|(path, body)| (canonical(path), canonical(&normalize(body, root, engine))))
            .collect()
    }
}

/// Write one project, in the freeze's words or today's.
fn write_project(root: &Path, runbook: &str, extra: &[(&str, &str)], today: bool) {
    let _ = fs::remove_dir_all(root);
    for directory in [
        ".arca/ticket",
        ".arca/residual",
        ".arca/issue/i-777-blocker",
        ".ratmac",
        "src",
    ] {
        fs::create_dir_all(root.join(directory)).expect("create the fixture tree");
    }

    let mut files: Vec<(String, String)> = vec![
        ("src/lib.rs".to_owned(), "pub fn work() {}\n".to_owned()),
        (".ratmac/ratmac.toml".to_owned(), runbook.to_owned()),
        (
            ".arca/ticket/t-900.md".to_owned(),
            "---\nticket-id: t-900\nresidual-ids:\n  - \"res-900\"\n\
             planned-test-refs:\n  - \"PT-900-01\"\nstatus: \"executing\"\n---\n\n\
             # Ticket: t-900\n\n## Merge Gate\n\n- Quality: `cargo --version` passes.\n"
                .to_owned(),
        ),
        (
            ".arca/residual/res-900.md".to_owned(),
            "# Residual Record\n\n```yaml\nresidual-id: \"res-900\"\n\
             goal-requirement-ref: \"DEMO-001\"\nstatus: \"missing\"\n```\n"
                .to_owned(),
        ),
    ];
    for name in [
        "index.md",
        "spec.md",
        "design.md",
        "test-plan.md",
        "ubi-lang.md",
    ] {
        files.push((
            format!(".arca/issue/i-777-blocker/{name}"),
            format!("# {name}\n\n```yaml\nstatus: \"pending\"\n```\n"),
        ));
    }
    for (relative, body) in extra {
        files.push(((*relative).to_owned(), (*body).to_owned()));
    }

    for (relative, body) in files {
        let path = root.join(&relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create a fixture directory");
        }
        let body = if today { to_today(&body) } else { body };
        fs::write(path, body).expect("write a fixture file");
    }
}
