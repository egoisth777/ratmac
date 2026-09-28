//! t-121 / WEB-002: engine ignore initialization.
//!
//! Contract: `.arca/ticket/t-121.md` `## Observable Output` (its exact
//! texts, exit codes, and bytes are the oracle below). The Engine is not
//! implemented yet: every check here must compile now and fail for its
//! stated missing behavior, then pass once the contract lands.
//!
//! WEBV-005: a fresh Git repository initializes to the exact three stdout
//! lines and the four-line policy file, Git ignores exactly the runtime
//! probes and none of the authoring probes, and no runbook, Run, commit,
//! index entry, or Git configuration appears.
//! WEBV-006: initialization is idempotent (byte-identical repeats),
//! preserves compatible operator bytes (appending only the missing policy
//! lines), and refuses a conflicting effective rule, an indexed runtime
//! file, a failed repository query, an unreadable policy file, an
//! unwritable destination, or an extra argument without touching a byte.
//! WEBV-007: `rtm start` runs the same policy step before admitting a Run
//! (rolling its bytes back when start refuses later for another reason and
//! refusing as `rtm: start: <policy refusal>` when the policy refuses),
//! linked worktrees install the policy at the shared primary root while
//! their own authoring paths stay trackable, and a no-Git directory gets
//! the local policy with the exact not-verified line.
//! WEBV-008: the staged-write and replace fault points refuse with nothing
//! left behind, and eight concurrent initializations serialize into one
//! complete policy that never duplicates a line nor loses operator bytes.

use ratmac_qa::support::{self, CaptureOptions, TempTree};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

// --- the contract's exact bytes and texts --------------------------------------

/// The policy file's fresh bytes: the four lines, LF-terminated.
const FRESH_POLICY: &[u8] = b"/runs/\n/mint.toml\n/locks/\n/log.md\n";

/// The four runtime probes, relative to a checkout, as the contract lists
/// them for Git verification.
const RUNTIME_PROBES: [&str; 4] = [
    ".ratmac/runs/run-001",
    ".ratmac/mint.toml",
    ".ratmac/locks/root.lock",
    ".ratmac/log.md",
];

/// The three authoring probes that must stay visible to Git.
const AUTHORING_PROBES: [&str; 3] = [
    ".ratmac/.gitignore",
    ".ratmac/ratmac.toml",
    ".ratmac/evidence/run-001",
];

/// Success stdout line 2 inside a verified repository.
const VERIFIED_LINE: &str = "rtm: verified: runs/, mint.toml, locks/, and log.md ignored; \
                            .gitignore, ratmac.toml, and evidence/ trackable";

/// Success stdout line 2 without verification.
const NOT_VERIFIED_LINE: &str = "rtm: repository-level ignore rules were not verified: Git is \
                                absent or the directory is not inside a Git repository";

/// Success stdout line 3 with no runbook at the invoking checkout.
const NEXT_SCAFFOLD_LINE: &str = "next: rtm scaffold .ratmac/ratmac.toml";

/// Success stdout line 3 once a runbook exists at the invoking checkout.
const NEXT_START_LINE: &str = "next: rtm start";

/// `rtm init --help`, exactly (the contract's fenced block plus one LF).
const INIT_HELP: &str = "Usage: rtm init\n\nInstall the Engine-root tracking policy before any \
                         Run is admitted: .ratmac/.gitignore carries the four runtime lines \
                         /runs/, /mint.toml, /locks/, and /log.md, each exactly once, while the \
                         runbook and evidence/ stay trackable. Existing bytes are preserved; a \
                         conflicting effective rule, a runtime file already in Git's index, or a \
                         failed repository inspection refuses and changes nothing. Idempotent and \
                         concurrency-safe: repeating it writes nothing, and no Run, runbook, \
                         commit, or configuration is created. rtm start runs the same policy step \
                         before admitting a Run.\n";

/// The general usage's `Commands:` line once `init` joins the route table.
const COMMANDS_LINE: &str = "Commands: init, start, status, step, hold, abandon, spawn, respawn, \
                            doctor, scaffold, skill";

/// The runbook every start fixture shares: one workflow root and a
/// two-state machine, proven to start a Run in the existing suites.
const RUNBOOK: &str = r#"
[roots]
work = "work"

[states.plan]
prompt = "Plan the cycle."

[states.done]
prompt = "Done."

[[transitions]]
from = "plan"
to = "done"
"#;

// --- shared helpers ------------------------------------------------------------

/// One owned empty file every Git child (direct or beneath `rtm`) sees as
/// `GIT_CONFIG_GLOBAL`, beside `GIT_CONFIG_NOSYSTEM=1`, so the operator's
/// own configuration can never leak into a fixture or be written by one.
struct GitIsolate {
    tree: TempTree,
}

impl GitIsolate {
    fn new(label: &str) -> Self {
        let tree = TempTree::new(label).expect("own the Git isolation tree");
        tree.write("global-git-config", b"");
        Self { tree }
    }

    /// The empty file standing in for the operator's global configuration.
    fn global_config(&self) -> PathBuf {
        self.tree.join("global-git-config")
    }

    /// Point `command` at the isolated Git configuration.
    fn apply(&self, command: &mut Command) {
        command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.global_config());
    }
}

/// This path in the one spelling the Engine shows: forward slashes.
fn rendered(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn exit_code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

/// Fixture Git in `directory` under the isolated configuration.
fn git(isolate: &GitIsolate, directory: &Path, args: &[&str]) -> Output {
    let mut command = support::git(directory);
    command.args(args);
    isolate.apply(&mut command);
    command
        .output()
        .unwrap_or_else(|error| panic!("git {args:?} in {}: {error}", rendered(directory)))
}

/// Fixture Git that must succeed.
fn git_ok(isolate: &GitIsolate, label: &str, directory: &Path, args: &[&str]) -> Output {
    let output = git(isolate, directory, args);
    assert!(
        output.status.success(),
        "{label}: git {args:?}: {}",
        support::text(&output)
    );
    output
}

/// Run the harness Engine in `directory` under the isolated configuration.
fn rtm(isolate: &GitIsolate, directory: &Path, args: &[&str]) -> Output {
    let mut command = support::command(ratmac_qa::engine_bin!(), directory);
    command.args(args);
    isolate.apply(&mut command);
    command
        .output()
        .unwrap_or_else(|error| panic!("invoke rtm {args:?}: {error}"))
}

/// Run the harness Engine with extra explicit environment entries (fault
/// points, ceilings): inherited test hooks stay scrubbed, these survive.
fn rtm_with(
    isolate: &GitIsolate,
    directory: &Path,
    args: &[&str],
    vars: &[(&str, &OsStr)],
) -> Output {
    let mut command = support::command(ratmac_qa::engine_bin!(), directory);
    command.args(args);
    for (key, value) in vars {
        command.env(key, value);
    }
    isolate.apply(&mut command);
    command
        .output()
        .unwrap_or_else(|error| panic!("invoke rtm {args:?}: {error}"))
}

/// A fresh isolated Git repository: local identity only, byte-exact line
/// endings, no global or system configuration reachable.
fn git_repo(isolate: &GitIsolate, label: &str) -> TempTree {
    let tree = TempTree::new(&format!("t121-{label}")).expect("own a fixture tree");
    for args in [
        &["init", "-q"][..],
        &["symbolic-ref", "HEAD", "refs/heads/main"][..],
        &["config", "user.email", "qa@ratmac.test"][..],
        &["config", "user.name", "ratmac qa"][..],
        &["config", "core.autocrlf", "false"][..],
        &["config", "commit.gpgsign", "false"][..],
    ] {
        git_ok(isolate, label, tree.path(), args);
    }
    tree
}

/// A repository whose runbook and work leaf are committed, the shape the
/// existing start suites require before a Run can be admitted.
fn committed_runbook_repo(isolate: &GitIsolate, label: &str) -> TempTree {
    let tree = git_repo(isolate, label);
    tree.write(".ratmac/ratmac.toml", RUNBOOK);
    tree.write("work/item.md", "Contributor-owned bytes.\n");
    git_ok(isolate, label, tree.path(), &["add", "--all"]);
    git_ok(
        isolate,
        label,
        tree.path(),
        &["commit", "-q", "-m", "t-121 fixture base"],
    );
    tree
}

/// Every node below `root` as comparable tagged bytes, `.git` included.
fn full_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    support::capture(root, CaptureOptions::default())
        .unwrap_or_else(|error| panic!("capture {}: {error}", rendered(root)))
        .tagged()
}

/// Every node below `root` as files plus directories (an empty directory
/// is visible as `path/`), `.git` included.
fn tree_with_dirs(root: &Path) -> BTreeMap<String, Vec<u8>> {
    support::capture(root, CaptureOptions::default())
        .unwrap_or_else(|error| panic!("capture {}: {error}", rendered(root)))
        .directories()
}

/// The Engine root's nodes as tagged bytes.
fn ratmac_nodes(checkout: &Path) -> BTreeMap<String, Vec<u8>> {
    full_tree(&checkout.join(".ratmac"))
}

/// The policy file's path under `checkout`, in the displayed spelling.
fn policy_path(checkout: &Path) -> String {
    rendered(&checkout.join(".ratmac").join(".gitignore"))
}

/// Success stdout for a run that wrote the policy: installed line, verified
/// line, and the next line for `runbook` (true: a runbook exists).
fn installed_stdout(checkout: &Path, runbook: bool) -> String {
    format!(
        "rtm: installed the Engine-root tracking policy at {}\n{}\n{}\n",
        policy_path(checkout),
        VERIFIED_LINE,
        if runbook {
            NEXT_START_LINE
        } else {
            NEXT_SCAFFOLD_LINE
        }
    )
}

/// Success stdout for a byte-identical repeat: the already-present line.
fn already_present_stdout(checkout: &Path, runbook: bool) -> String {
    format!(
        "rtm: Engine-root tracking policy already present at {}\n{}\n{}\n",
        policy_path(checkout),
        VERIFIED_LINE,
        if runbook {
            NEXT_START_LINE
        } else {
            NEXT_SCAFFOLD_LINE
        }
    )
}

/// Assert one exact init refusal: exit 1, empty stdout, exact stderr.
fn assert_refusal_exact(label: &str, output: &Output, expected_stderr: &str) {
    assert_eq!(
        exit_code(output),
        1,
        "{label}: the refusal exits 1; stdout: {}; stderr: {}",
        stdout(output),
        stderr(output)
    );
    assert!(
        stdout(output).is_empty(),
        "{label}: a refusal prints nothing on stdout: {}",
        stdout(output)
    );
    assert_eq!(stderr(output), expected_stderr, "{label}: exact stderr");
}

/// Whether Git ignores `relative` in `directory` (exit 0 vs 1 of
/// `git check-ignore -q`).
fn ignored(isolate: &GitIsolate, directory: &Path, relative: &str) -> bool {
    let output = git(isolate, directory, &["check-ignore", "-q", "--", relative]);
    match exit_code(&output) {
        0 => true,
        1 => false,
        other => panic!(
            "git check-ignore -q -- {relative} in {}: exit {other}: {}",
            rendered(directory),
            support::text(&output)
        ),
    }
}

/// The `<source>:<linenum>:<pattern>` of the effective rule for `relative`,
/// exactly as `git check-ignore -v` reports it (the text before the tab).
fn effective_rule(isolate: &GitIsolate, directory: &Path, relative: &str) -> String {
    let output = git(isolate, directory, &["check-ignore", "-v", "--", relative]);
    assert!(
        output.status.success(),
        "git check-ignore -v -- {relative} in {}: {}",
        rendered(directory),
        support::text(&output)
    );
    let text = stdout(&output);
    let first = text
        .lines()
        .next()
        .unwrap_or_else(|| panic!("check-ignore -v answers one row for {relative}: {text}"));
    first
        .split('\t')
        .next()
        .unwrap_or_else(|| panic!("check-ignore -v row carries a tab: {first}"))
        .to_owned()
}

/// The exact class B refusal lines a blanket rule can answer with, behind
/// `prefix` (`rtm: init: ` or `rtm: start: `): one per authoring probe Git
/// itself reports ignored. A tracked probe is never reported by
/// check-ignore, and the contract fixes each probe's text, not the engine's
/// probe order, so the stderr must be exactly one of these.
fn class_b_candidates(isolate: &GitIsolate, checkout: &Path, prefix: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    for probe in AUTHORING_PROBES {
        let output = git(isolate, checkout, &["check-ignore", "-v", "--", probe]);
        if !output.status.success() {
            continue;
        }
        let rule = stdout(&output)
            .lines()
            .next()
            .and_then(|row| row.split('\t').next().map(str::to_owned))
            .unwrap_or_else(|| panic!("check-ignore -v row for {probe}: {}", stdout(&output)));
        candidates.push(format!(
            "{prefix}{}\n",
            class_b_text(&rendered(&checkout.join(probe)), &rule)
        ));
    }
    assert!(
        !candidates.is_empty(),
        "the fixture's blanket rule must ignore at least one authoring probe Git reports"
    );
    candidates
}

/// Class A: a runtime probe stays visible; `<path>` absolute, rule as Git
/// reports it.
fn class_a_text(path: &str, rule: &str) -> String {
    format!(
        "refusing to install the ignore policy: the runtime path {path} would stay visible to Git \
         ({rule}); remove or repair that rule, then retry; nothing was modified"
    )
}

/// Class B: an authoring probe is ignored.
fn class_b_text(path: &str, rule: &str) -> String {
    format!(
        "refusing to install the ignore policy: the authoring path {path} would be ignored \
         ({rule}); repair that rule so ratmac.toml and evidence/ stay trackable, then retry; \
         nothing was modified"
    )
}

/// Class C: a runtime path is already in Git's index.
fn class_c_text(path: &str) -> String {
    format!(
        "refusing to install the ignore policy: the runtime path {path} is already in Git's \
         index; untrack it with git rm --cached {path}, then retry; nothing was modified"
    )
}

/// Class E: the policy file cannot be read.
fn class_e_text(path: &str) -> String {
    format!(
        "refusing to install the ignore policy: cannot read {path}; make it a readable UTF-8 file \
         (not a directory), then retry; nothing was modified"
    )
}

/// Class F: the destination cannot be created or written.
fn class_f_text(path: &str) -> String {
    format!(
        "refusing to install the ignore policy: cannot create or write {path}; make the Engine \
         root a directory and the policy file writable, then retry; nothing was modified"
    )
}

// --- WEBV-005 -------------------------------------------------------------------

/// WEBV-005: initializing a fresh Git repository prints the exact three
/// lines, writes exactly the four policy lines, makes Git ignore the four
/// runtime probes but none of the authoring paths, creates no runbook, Run,
/// commit, or index entry, and touches no Git configuration; `rtm init
/// --help` and the route-table line carry the contract's exact bytes.
#[test]
fn webv_005() {
    let isolate = GitIsolate::new("t121-005-isolate");
    let repo = git_repo(&isolate, "005-fresh");
    let root = repo.path();
    let git_before = full_tree(&root.join(".git"));

    // The positive control every later row in this suite builds on.
    let output = rtm(&isolate, root, &["init"]);
    assert_eq!(
        exit_code(&output),
        0,
        "WEBV-005: `rtm init` succeeds on a fresh repository; stderr: {}",
        stderr(&output)
    );
    assert_eq!(stdout(&output), installed_stdout(root, false));
    assert!(
        stderr(&output).is_empty(),
        "WEBV-005: a successful init prints no error: {}",
        stderr(&output)
    );

    // The installed policy: exactly the fresh bytes, and nothing else in
    // the Engine root - no runbook, runs/, mint.toml, locks/, or log.md.
    let policy = fs::read(root.join(".ratmac").join(".gitignore"))
        .unwrap_or_else(|error| panic!("read the policy file: {error}"));
    assert_eq!(
        policy, FRESH_POLICY,
        "WEBV-005: the fresh policy file holds exactly the four lines"
    );
    assert_eq!(
        ratmac_nodes(root),
        BTreeMap::from([(
            ".gitignore".to_owned(),
            [b"F\0".to_vec(), FRESH_POLICY.to_vec()].concat()
        )]),
        "WEBV-005: init creates the policy file alone"
    );

    // Git verification: the four runtime probes ignored, the authoring
    // probes (including the policy file itself) not.
    for probe in RUNTIME_PROBES {
        assert!(
            ignored(&isolate, root, probe),
            "WEBV-005: {probe} is ignored after init"
        );
    }
    for probe in AUTHORING_PROBES {
        assert!(
            !ignored(&isolate, root, probe),
            "WEBV-005: {probe} stays trackable after init"
        );
    }

    // No commit (HEAD unborn), the index empty.
    let head = git(&isolate, root, &["rev-parse", "--verify", "HEAD"]);
    assert!(
        !head.status.success(),
        "WEBV-005: init commits nothing; HEAD stays unborn: {}",
        support::text(&head)
    );
    let listed = git_ok(&isolate, "WEBV-005", root, &["ls-files"]);
    assert!(
        stdout(&listed).is_empty(),
        "WEBV-005: the index stays empty: {}",
        stdout(&listed)
    );

    // The repository's own storage is untouched: no config write, no staging
    // artifact, nothing under .git at all.
    assert_eq!(
        full_tree(&root.join(".git")),
        git_before,
        "WEBV-005: init writes nothing under .git (configuration included)"
    );

    // The isolated global configuration stands in for the operator's: a
    // global write would land there and be caught.
    let global = fs::read(isolate.global_config())
        .unwrap_or_else(|error| panic!("read the isolated global config: {error}"));
    assert!(
        global.is_empty(),
        "WEBV-005: init writes no global Git configuration"
    );

    // The help forms and the route table, pure usage responses.
    let before_help = full_tree(root);
    let general = rtm(&isolate, root, &["--help"]);
    assert_eq!(
        exit_code(&general),
        0,
        "WEBV-005: `rtm --help` exits 0: {}",
        stderr(&general)
    );
    assert!(stderr(&general).is_empty());
    let general_text = stdout(&general);
    let listed = general_text
        .lines()
        .find(|line| line.starts_with("Commands:"))
        .unwrap_or_else(|| {
            panic!("WEBV-005: the general usage carries a `Commands:` line: {general_text}")
        });
    assert_eq!(
        listed, COMMANDS_LINE,
        "WEBV-005: the route table lists `init` first"
    );
    for help_form in [["init", "--help"], ["init", "-h"]] {
        let output = rtm(&isolate, root, &help_form);
        assert_eq!(
            exit_code(&output),
            0,
            "WEBV-005: `rtm {}` exits 0: {}",
            help_form.join(" "),
            stderr(&output)
        );
        assert_eq!(
            stdout(&output),
            INIT_HELP,
            "WEBV-005: `rtm {}` prints the contract's help exactly",
            help_form.join(" ")
        );
        assert!(
            stderr(&output).is_empty(),
            "WEBV-005: `rtm {}` prints no error",
            help_form.join(" ")
        );
    }
    assert_eq!(
        full_tree(root),
        before_help,
        "WEBV-005: the pure help forms resolve, inspect, read, and write nothing"
    );
}

// --- WEBV-006 -------------------------------------------------------------------

/// WEBV-006: repeating initialization is a byte-identical no-op printing
/// the already-present line; compatible operator bytes survive with only
/// the missing policy lines appended; and each refusal class (visible
/// runtime path, ignored authoring path, indexed runtime file, failed
/// repository query, unreadable policy file, unwritable destination, extra
/// argument) refuses with its exact text and a byte-identical tree.
#[test]
fn webv_006() {
    let isolate = GitIsolate::new("t121-006-isolate");

    // Positive control and idempotence: three runs, the tree frozen after
    // the first.
    let repeat = git_repo(&isolate, "006-repeat");
    let repeat_root = repeat.path();
    let first = rtm(&isolate, repeat_root, &["init"]);
    assert_eq!(
        exit_code(&first),
        0,
        "WEBV-006: the control `rtm init` succeeds: {}",
        stderr(&first)
    );
    assert_eq!(stdout(&first), installed_stdout(repeat_root, false));
    let after_first = full_tree(repeat_root);
    for run in 2..=3 {
        let output = rtm(&isolate, repeat_root, &["init"]);
        assert_eq!(
            exit_code(&output),
            0,
            "WEBV-006: repeat run {run} succeeds: {}",
            stderr(&output)
        );
        assert_eq!(
            stdout(&output),
            already_present_stdout(repeat_root, false),
            "WEBV-006: repeat run {run} prints the already-present line"
        );
        assert!(
            stderr(&output).is_empty(),
            "WEBV-006: repeat run {run} prints no error"
        );
    }
    assert_eq!(
        full_tree(repeat_root),
        after_first,
        "WEBV-006: the repeats write nothing: the tree stays byte-identical"
    );

    // Compatible operator bytes: only the missing policy lines append, each
    // always LF, one separator newline exactly when the bytes lack one.
    let variants: [(&str, &[u8], &[u8]); 4] = [
        (
            "crlf-terminated operator lines",
            b"notes/\r\n*.tmp\r\n",
            b"notes/\r\n*.tmp\r\n/runs/\n/mint.toml\n/locks/\n/log.md\n",
        ),
        (
            "no final newline",
            b"*.tmp",
            b"*.tmp\n/runs/\n/mint.toml\n/locks/\n/log.md\n",
        ),
        (
            "one policy line already present (CRLF)",
            b"/runs/\r\noperator note\n",
            b"/runs/\r\noperator note\n/mint.toml\n/locks/\n/log.md\n",
        ),
        (
            "a zero-byte file",
            b"",
            b"/runs/\n/mint.toml\n/locks/\n/log.md\n",
        ),
    ];
    for (label, start, expected) in variants {
        let repo = git_repo(&isolate, &format!("006-{}", label.replace([' '], "-")));
        let root = repo.path();
        repo.write(".ratmac/.gitignore", start);
        let output = rtm(&isolate, root, &["init"]);
        assert_eq!(
            exit_code(&output),
            0,
            "WEBV-006: {label}: the append succeeds: {}",
            stderr(&output)
        );
        assert_eq!(stdout(&output), installed_stdout(root, false));
        let policy = fs::read(root.join(".ratmac").join(".gitignore"))
            .unwrap_or_else(|error| panic!("read the policy file ({label}): {error}"));
        assert_eq!(
            policy, expected,
            "WEBV-006: {label}: exactly the missing policy lines append"
        );
    }

    // A compatible root rule set (this repository's own shape): compatible
    // rules neither refuse nor change a byte.
    let root_rules = git_repo(&isolate, "006-root-rules");
    let root_rules_root = root_rules.path();
    root_rules.write(
        ".gitignore",
        ".ratmac/runs/\n.ratmac/mint.toml\n.ratmac/locks/\n.ratmac/log.md\n",
    );
    let root_rules_gitignore =
        fs::read(root_rules_root.join(".gitignore")).expect("read root .gitignore");
    let output = rtm(&isolate, root_rules_root, &["init"]);
    assert_eq!(
        exit_code(&output),
        0,
        "WEBV-006: compatible root rules survive: {}",
        stderr(&output)
    );
    assert_eq!(stdout(&output), installed_stdout(root_rules_root, false));
    assert_eq!(
        fs::read(root_rules_root.join(".ratmac").join(".gitignore")).expect("read policy"),
        FRESH_POLICY,
        "WEBV-006: compatible root rules: the policy still installs"
    );
    assert_eq!(
        fs::read(root_rules_root.join(".gitignore")).expect("re-read root .gitignore"),
        root_rules_gitignore,
        "WEBV-006: init changes no operator byte in the root .gitignore"
    );

    // Refusal twin A: a negation after a policy line keeps log.md visible.
    // The positive control runs first on this very fixture: the same
    // command succeeds before the operator plants the negation.
    {
        let repo = git_repo(&isolate, "006-class-a");
        let root = repo.path();
        repo.write(".ratmac/.gitignore", "/log.md\n");
        let control = rtm(&isolate, root, &["init"]);
        assert_eq!(
            exit_code(&control),
            0,
            "WEBV-006: class A's own positive control succeeds: {}",
            stderr(&control)
        );
        repo.write(".ratmac/.gitignore", "/log.md\n!/log.md\n");
        let rule = effective_rule(&isolate, root, ".ratmac/log.md");
        let expected = format!(
            "rtm: init: {}\n",
            class_a_text(&rendered(&root.join(".ratmac").join("log.md")), &rule)
        );
        let before = full_tree(root);
        let output = rtm(&isolate, root, &["init"]);
        assert_refusal_exact("WEBV-006: class A", &output, &expected);
        assert_eq!(
            full_tree(root),
            before,
            "WEBV-006: class A rolls every byte back"
        );
    }

    // Refusal twins B: a blanket rule, from the root .gitignore and from
    // .git/info/exclude. A blanket rule ignores all three authoring probes
    // with one rule, and the contract fixes each probe's text, not the
    // engine's probe order: the stderr must be exactly one of the three.
    for (label, plant) in [("blanket-root", "root"), ("blanket-exclude", "exclude")] {
        let repo = git_repo(&isolate, &format!("006-class-b-{label}"));
        let root = repo.path();
        match plant {
            "root" => repo.write(".gitignore", ".ratmac/\n"),
            _ => {
                let exclude = root.join(".git").join("info").join("exclude");
                let previous = fs::read_to_string(&exclude).unwrap_or_default();
                fs::write(&exclude, format!("{previous}.ratmac/\n"))
                    .unwrap_or_else(|error| panic!("write the exclude file: {error}"));
            }
        }
        let candidates = class_b_candidates(&isolate, root, "rtm: init: ");
        let before = full_tree(root);
        let output = rtm(&isolate, root, &["init"]);
        assert_eq!(
            exit_code(&output),
            1,
            "WEBV-006: class B ({label}) exits 1: {}",
            stderr(&output)
        );
        assert!(
            stdout(&output).is_empty(),
            "WEBV-006: class B ({label}) prints nothing on stdout"
        );
        assert!(
            candidates.contains(&stderr(&output)),
            "WEBV-006: class B ({label}) is exactly one authoring probe's refusal text; \
             expected one of {candidates:?}"
        );
        assert_eq!(
            full_tree(root),
            before,
            "WEBV-006: class B ({label}) rolls every byte back"
        );
    }

    // Refusal twin C: a runtime file already in Git's index. The control
    // init on this very fixture succeeded before the staging.
    {
        let repo = git_repo(&isolate, "006-class-c");
        let root = repo.path();
        let control = rtm(&isolate, root, &["init"]);
        assert_eq!(
            exit_code(&control),
            0,
            "WEBV-006: class C's own positive control succeeds: {}",
            stderr(&control)
        );
        repo.write(".ratmac/log.md", "runtime bytes that were staged\n");
        git_ok(
            &isolate,
            "WEBV-006 class C",
            root,
            &["add", "-f", "--", ".ratmac/log.md"],
        );
        let staged = git_ok(
            &isolate,
            "WEBV-006 class C",
            root,
            &["ls-files", "--", ".ratmac/log.md"],
        );
        assert!(
            stdout(&staged)
                .lines()
                .any(|line| line.trim() == ".ratmac/log.md"),
            "WEBV-006: class C's fixture really holds the runtime file in the index"
        );
        let expected = format!(
            "rtm: init: {}\n",
            class_c_text(&rendered(&root.join(".ratmac").join("log.md")))
        );
        let before = full_tree(root);
        let output = rtm(&isolate, root, &["init"]);
        assert_refusal_exact("WEBV-006: class C", &output, &expected);
        assert_eq!(
            full_tree(root),
            before,
            "WEBV-006: class C leaves the staged index byte-identical"
        );
    }

    // Refusal twin D: a corrupt index in an available repository. The
    // control init on this fixture succeeded before the corruption; the
    // corrupt query must refuse, never downgrade to the not-verified line.
    // The contract leaves `<arguments>` to the failed query's own name, so
    // the fixed head and tail are asserted exactly.
    {
        let repo = git_repo(&isolate, "006-class-d");
        let root = repo.path();
        let control = rtm(&isolate, root, &["init"]);
        assert_eq!(
            exit_code(&control),
            0,
            "WEBV-006: class D's own positive control succeeds: {}",
            stderr(&control)
        );
        fs::write(
            root.join(".git").join("index"),
            b"corrupt index bytes: t-121",
        )
        .expect("corrupt the index");
        let broken = git(&isolate, root, &["ls-files"]);
        assert!(
            !broken.status.success(),
            "WEBV-006: class D's fixture really breaks an index query"
        );
        let before = full_tree(root);
        let output = rtm(&isolate, root, &["init"]);
        assert_eq!(
            exit_code(&output),
            1,
            "WEBV-006: class D exits 1 (never a not-verified success): {}",
            stderr(&output)
        );
        assert!(
            stdout(&output).is_empty(),
            "WEBV-006: class D prints nothing on stdout"
        );
        let text = stderr(&output);
        let head = "rtm: init: refusing to install the ignore policy: cannot inspect the \
                    repository: git ";
        let tail = "; repair the repository, then retry; nothing was modified\n";
        assert!(
            text.starts_with(head),
            "WEBV-006: class D's stderr carries the fixed head; held: {text:?}"
        );
        assert!(
            text.ends_with(tail),
            "WEBV-006: class D's stderr carries the fixed tail; held: {text:?}"
        );
        let query = text
            .strip_prefix(head)
            .and_then(|rest| rest.strip_suffix(tail))
            .unwrap_or_default();
        assert!(
            query.contains(" failed: ") && query.len() > " failed: ".len(),
            "WEBV-006: class D names the failed git query and its first stderr line; \
             held: {query:?}"
        );
        assert_eq!(
            full_tree(root),
            before,
            "WEBV-006: class D leaves the corrupt repository byte-identical"
        );
    }

    // Refusal twin E: the policy file is a directory.
    {
        let repo = git_repo(&isolate, "006-class-e");
        let root = repo.path();
        fs::create_dir_all(root.join(".ratmac").join(".gitignore"))
            .expect("make the policy path a directory");
        let expected = format!("rtm: init: {}\n", class_e_text(&policy_path(root)));
        let before = full_tree(root);
        let output = rtm(&isolate, root, &["init"]);
        assert_refusal_exact("WEBV-006: class E", &output, &expected);
        assert_eq!(
            full_tree(root),
            before,
            "WEBV-006: class E leaves every byte alone"
        );
    }

    // Refusal twin F: the Engine root is a regular file.
    {
        let repo = git_repo(&isolate, "006-class-f");
        let root = repo.path();
        fs::write(root.join(".ratmac"), "not a directory\n").expect("make .ratmac a file");
        let expected = format!("rtm: init: {}\n", class_f_text(&policy_path(root)));
        let before = full_tree(root);
        let output = rtm(&isolate, root, &["init"]);
        assert_refusal_exact("WEBV-006: class F", &output, &expected);
        assert_eq!(
            full_tree(root),
            before,
            "WEBV-006: class F leaves every byte alone"
        );
    }

    // The ordinary usage error: init takes no arguments.
    {
        let repo = git_repo(&isolate, "006-extra-arg");
        let root = repo.path();
        let before = full_tree(root);
        let output = rtm(&isolate, root, &["init", "extra"]);
        assert_eq!(
            exit_code(&output),
            2,
            "WEBV-006: `rtm init extra` is init's usage error (exit 2): {}",
            stderr(&output)
        );
        assert!(
            stdout(&output).is_empty(),
            "WEBV-006: the usage error prints nothing on stdout"
        );
        assert_eq!(
            stderr(&output),
            "rtm: init accepts no arguments\n",
            "WEBV-006: the usage error's exact text"
        );
        assert_eq!(
            full_tree(root),
            before,
            "WEBV-006: the usage error writes nothing"
        );
    }
}

// --- WEBV-007 -------------------------------------------------------------------

/// WEBV-007: a first `rtm start` without prior init admits its Run with
/// unchanged stdout while the policy bytes land and Git ignores the runtime;
/// a start refused after the policy step (an injected root-lock fault)
/// restores the prior bytes; a policy refusal answers as
/// `rtm: start: <refusal>`; a linked worktree installs at the primary's
/// Engine root while its own authoring paths stay trackable; and a no-Git
/// directory gets the local policy with the exact not-verified line.
#[test]
fn webv_007() {
    let isolate = GitIsolate::new("t121-007-isolate");

    // A first start without prior init: succeeds, the policy bytes land,
    // and Git ignores the runtime the Run just created.
    let a = committed_runbook_repo(&isolate, "007-start-a");
    let a_root = a.path();
    let start_a = rtm(&isolate, a_root, &["start"]);
    assert_eq!(
        exit_code(&start_a),
        0,
        "WEBV-007: the first `rtm start` succeeds without prior init: {}",
        stderr(&start_a)
    );
    assert_eq!(
        fs::read(a_root.join(".ratmac").join(".gitignore"))
            .unwrap_or_else(|error| panic!("read the policy file: {error}")),
        FRESH_POLICY,
        "WEBV-007: start's policy step installs the fresh bytes"
    );
    for probe in RUNTIME_PROBES {
        assert!(
            ignored(&isolate, a_root, probe),
            "WEBV-007: {probe} is ignored after the first start"
        );
    }
    for probe in AUTHORING_PROBES {
        assert!(
            !ignored(&isolate, a_root, probe),
            "WEBV-007: {probe} stays trackable after the first start"
        );
    }
    // Git's own status view: runtime is ignored, never merely untracked.
    let plain_status = git_ok(&isolate, "WEBV-007", a_root, &["status", "--porcelain"]);
    let porcelain = stdout(&plain_status);
    let ignored_status = git_ok(
        &isolate,
        "WEBV-007",
        a_root,
        &["status", "--porcelain", "--ignored"],
    );
    let with_ignored = stdout(&ignored_status);
    assert_eq!(
        porcelain
            .lines()
            .filter(|line| line.contains(".ratmac/"))
            .collect::<Vec<_>>(),
        vec!["?? .ratmac/.gitignore"],
        "WEBV-007: plain status sees only the trackable policy file under .ratmac/"
    );
    for line in with_ignored.lines() {
        let Some((tag, path)) = line.split_at_checked(2) else {
            panic!("WEBV-007: a porcelain line is tag plus path: {line:?}")
        };
        if !path.trim_start().starts_with(".ratmac/") {
            continue;
        }
        if path.trim_start() == ".ratmac/.gitignore" {
            assert_eq!(
                tag, "??",
                "WEBV-007: the policy file is untracked, not ignored"
            );
        } else {
            assert_eq!(
                tag, "!!",
                "WEBV-007: the runtime path {path} is ignored, not untracked"
            );
        }
    }
    assert!(
        with_ignored
            .lines()
            .any(|line| line.ends_with(".ratmac/log.md") && line.starts_with("!!")),
        "WEBV-007: --ignored lists the runtime log as ignored: {with_ignored}"
    );

    // Unchanged stdout: the same start on a twin fixture that ran `rtm
    // init` first prints the same bytes; start says nothing about policy.
    let b = committed_runbook_repo(&isolate, "007-start-b");
    let b_root = b.path();
    let init_b = rtm(&isolate, b_root, &["init"]);
    assert_eq!(
        exit_code(&init_b),
        0,
        "WEBV-007: the control init on the twin fixture succeeds: {}",
        stderr(&init_b)
    );
    assert_eq!(stdout(&init_b), installed_stdout(b_root, true));
    let start_b = rtm(&isolate, b_root, &["start"]);
    assert_eq!(
        exit_code(&start_b),
        0,
        "WEBV-007: the control start on the twin fixture succeeds: {}",
        stderr(&start_b)
    );
    assert_eq!(
        stdout(&start_b),
        stdout(&start_a),
        "WEBV-007: the first start's stdout is unchanged by the policy step"
    );

    // A start refused for another reason after the policy step (an injected
    // fault right after the Engine root lock is acquired) restores the
    // prior bytes: no policy file is left.
    let c = committed_runbook_repo(&isolate, "007-start-fault");
    let c_root = c.path();
    let before = tree_with_dirs(c_root);
    let faulted = rtm_with(
        &isolate,
        c_root,
        &["start"],
        &[("RATMAC_TEST_LOCK_FAULT", OsStr::new("root"))],
    );
    assert_eq!(
        exit_code(&faulted),
        1,
        "WEBV-007: the post-policy refusal refuses the start: {}",
        stderr(&faulted)
    );
    assert!(
        stdout(&faulted).is_empty(),
        "WEBV-007: the refused start prints nothing on stdout"
    );
    assert!(
        stderr(&faulted).starts_with("rtm: start:"),
        "WEBV-007: the refusal is start's own: {}",
        stderr(&faulted)
    );
    assert!(
        !c_root.join(".ratmac").join(".gitignore").exists(),
        "WEBV-007: the rolled-back start leaves no policy file"
    );
    let mut after = tree_with_dirs(c_root);
    // The Engine's lock home is start's own artifact, not the policy
    // step's: the faulted start of today may leave the empty directory.
    after.remove(".ratmac/locks/");
    assert_eq!(
        after, before,
        "WEBV-007: the refused start restores the prior bytes exactly"
    );

    // A start the policy itself refuses (a blanket root rule): the class B
    // text behind `rtm: start: `, no Run admitted, nothing written.
    let d = committed_runbook_repo(&isolate, "007-start-policy-refusal");
    let d_root = d.path();
    fs::write(d_root.join(".gitignore"), ".ratmac/\n").expect("plant the blanket rule");
    let candidates = class_b_candidates(&isolate, d_root, "rtm: start: ");
    let before = full_tree(d_root);
    let refused = rtm(&isolate, d_root, &["start"]);
    assert_eq!(
        exit_code(&refused),
        1,
        "WEBV-007: the policy refusal refuses the start: {}",
        stderr(&refused)
    );
    assert!(
        stdout(&refused).is_empty(),
        "WEBV-007: the policy-refused start prints nothing on stdout"
    );
    assert!(
        candidates.contains(&stderr(&refused)),
        "WEBV-007: the refusal is exactly the class B text behind `rtm: start: `; expected \
         one of {candidates:?}"
    );
    assert!(
        !d_root.join(".ratmac").join("runs").exists(),
        "WEBV-007: the policy-refused start admits no Run"
    );
    assert_eq!(
        full_tree(d_root),
        before,
        "WEBV-007: the policy-refused start changes nothing"
    );

    // A linked worktree: init from the linked checkout writes the primary
    // checkout's Engine root, and the linked checkout's own authoring
    // paths stay trackable.
    {
        let tree = TempTree::new("t121-007-worktree").expect("own the worktree fixture tree");
        let primary = tree.join("primary");
        let linked = tree.join("linked");
        fs::create_dir_all(primary.join(".ratmac")).expect("create the runbook directory");
        fs::create_dir_all(primary.join("work")).expect("create the work directory");
        fs::write(primary.join(".ratmac/ratmac.toml"), RUNBOOK).expect("write the runbook");
        fs::write(primary.join("work/item.md"), "Contributor-owned bytes.\n")
            .expect("write the work leaf");
        git_ok(&isolate, "WEBV-007 worktree", &primary, &["init", "-q"]);
        for args in [
            &["config", "user.email", "qa@ratmac.test"][..],
            &["config", "user.name", "ratmac qa"][..],
            &["config", "core.autocrlf", "false"][..],
            &["config", "commit.gpgsign", "false"][..],
        ] {
            git_ok(&isolate, "WEBV-007 worktree", &primary, args);
        }
        git_ok(&isolate, "WEBV-007 worktree", &primary, &["add", "--all"]);
        git_ok(
            &isolate,
            "WEBV-007 worktree",
            &primary,
            &["commit", "-q", "-m", "t-121 worktree base"],
        );
        // The single worktree add that counts, with its target directory.
        let mut add_with_target = support::git(&primary);
        add_with_target
            .args(["worktree", "add", "-q", "-b", "linked-t-121"])
            .arg(&linked);
        isolate.apply(&mut add_with_target);
        let add_output = add_with_target
            .output()
            .expect("create the linked worktree");
        assert!(
            add_output.status.success(),
            "git worktree add: {}",
            support::text(&add_output)
        );
        fs::create_dir_all(linked.join(".ratmac")).expect("create the linked runbook directory");
        fs::write(linked.join(".ratmac/ratmac.toml"), RUNBOOK)
            .expect("give the linked checkout its own runbook bytes");

        let output = rtm(&isolate, &linked, &["init"]);
        assert_eq!(
            exit_code(&output),
            0,
            "WEBV-007: init from the linked checkout succeeds: {}",
            stderr(&output)
        );
        assert_eq!(
            stdout(&output),
            installed_stdout(&primary, true),
            "WEBV-007: the linked init names the primary Engine root and its own next line"
        );
        assert!(
            stderr(&output).is_empty(),
            "WEBV-007: the linked init prints no error"
        );
        assert_eq!(
            fs::read(primary.join(".ratmac").join(".gitignore"))
                .expect("read the primary policy file"),
            FRESH_POLICY,
            "WEBV-007: the policy lands in the primary checkout's Engine root"
        );
        assert!(
            !linked.join(".ratmac").join(".gitignore").exists(),
            "WEBV-007: the linked checkout keeps no policy file of its own"
        );
        for probe in [".ratmac/ratmac.toml", ".ratmac/evidence/run-001"] {
            assert!(
                !ignored(&isolate, &linked, probe),
                "WEBV-007: the linked checkout's {probe} stays trackable"
            );
        }
        assert!(
            ignored(&isolate, &primary, ".ratmac/runs/run-001"),
            "WEBV-007: the shared runtime root is protected"
        );
    }

    // A no-Git directory: the local policy installs with the exact
    // not-verified line, never a verified claim.
    let plain = TempTree::new("t121-007-nogit").expect("own the no-Git fixture tree");
    let ceiling = plain
        .path()
        .parent()
        .expect("the fixture tree has a parent")
        .as_os_str();
    let output = rtm_with(
        &isolate,
        plain.path(),
        &["init"],
        &[("GIT_CEILING_DIRECTORIES", ceiling)],
    );
    assert_eq!(
        exit_code(&output),
        0,
        "WEBV-007: init without Git succeeds: {}",
        stderr(&output)
    );
    assert_eq!(
        stdout(&output),
        format!(
            "rtm: installed the Engine-root tracking policy at {}\n{}\n{}\n",
            policy_path(plain.path()),
            NOT_VERIFIED_LINE,
            NEXT_SCAFFOLD_LINE
        ),
        "WEBV-007: the no-Git run carries the exact not-verified line"
    );
    assert!(
        stderr(&output).is_empty(),
        "WEBV-007: the no-Git run prints no error"
    );
    assert_eq!(
        ratmac_nodes(plain.path()),
        BTreeMap::from([(
            ".gitignore".to_owned(),
            [b"F\0".to_vec(), FRESH_POLICY.to_vec()].concat()
        )]),
        "WEBV-007: the no-Git run creates the policy file alone"
    );
}

// --- WEBV-008 -------------------------------------------------------------------

/// WEBV-008: the staged-write and replace fault points refuse with the
/// exact injected-fault text and leave nothing behind (fresh fixture and
/// operator fixture alike), and eight concurrent initializations on one
/// operator fixture serialize into exactly one complete policy with no
/// duplicate line, no lost operator byte, and no artifact left.
#[test]
fn webv_008() {
    let isolate = GitIsolate::new("t121-008-isolate");

    // Positive control: the same command without the fault succeeds.
    let control = git_repo(&isolate, "008-control");
    let control_output = rtm(&isolate, control.path(), &["init"]);
    assert_eq!(
        exit_code(&control_output),
        0,
        "WEBV-008: the control `rtm init` succeeds: {}",
        stderr(&control_output)
    );
    assert_eq!(
        stdout(&control_output),
        installed_stdout(control.path(), false)
    );

    // The two fault boundaries, each on a fresh fixture and on a fixture
    // whose policy file already holds operator bytes.
    for fault in ["staged-write", "replace"] {
        for shape in ["fresh", "operator"] {
            let repo = git_repo(&isolate, &format!("008-fault-{fault}-{shape}"));
            let root = repo.path();
            if shape == "operator" {
                repo.write(".ratmac/.gitignore", "operator-keep/\n");
            }
            let expected =
                format!("rtm: init: injected init fault at {fault}; nothing was modified\n");
            let before = full_tree(root);
            let output = rtm_with(
                &isolate,
                root,
                &["init"],
                &[("RATMAC_TEST_INIT_FAULT", OsStr::new(fault))],
            );
            assert_refusal_exact(
                &format!("WEBV-008: fault {fault} on the {shape} fixture"),
                &output,
                &expected,
            );
            assert_eq!(
                full_tree(root),
                before,
                "WEBV-008: fault {fault} on the {shape} fixture leaves no staged copy, lock, \
                 or any other leftover anywhere under the fixture"
            );
        }
    }

    // Concurrency: eight simultaneous inits on one operator file whose
    // bytes lack a final newline (the separator is part of the oracle).
    let concurrent = git_repo(&isolate, "008-concurrent");
    let concurrent_root = concurrent.path();
    concurrent.write(".ratmac/.gitignore", "operator-a/\noperator-b");
    let before = full_tree(concurrent_root);
    let children: Vec<_> = (0..8)
        .map(|_| {
            let mut command = support::command(ratmac_qa::engine_bin!(), concurrent_root);
            command.args(["init"]);
            isolate.apply(&mut command);
            command
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap_or_else(|error| panic!("spawn a concurrent init: {error}"))
        })
        .collect();
    let mut any_installed = false;
    for child in children {
        let output = child
            .wait_with_output()
            .unwrap_or_else(|error| panic!("collect a concurrent init: {error}"));
        assert_eq!(
            exit_code(&output),
            0,
            "WEBV-008: every concurrent init exits 0: {}",
            support::text(&output)
        );
        let text = stdout(&output);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines.len(),
            3,
            "WEBV-008: every concurrent init prints its three lines: {text:?}"
        );
        assert!(
            stderr(&output).is_empty(),
            "WEBV-008: a concurrent init prints no error: {}",
            stderr(&output)
        );
        assert!(
            lines[0]
                == format!(
                    "rtm: installed the Engine-root tracking policy at {}",
                    policy_path(concurrent_root)
                )
                || lines[0]
                    == format!(
                        "rtm: Engine-root tracking policy already present at {}",
                        policy_path(concurrent_root)
                    ),
            "WEBV-008: line 1 is the installed or already-present line: {text:?}"
        );
        any_installed |= !lines[0].contains("already present");
        assert_eq!(
            lines[1], VERIFIED_LINE,
            "WEBV-008: line 2 is the verified line"
        );
        assert_eq!(
            lines[2], NEXT_SCAFFOLD_LINE,
            "WEBV-008: line 3 is the scaffold next line"
        );
    }
    assert!(
        any_installed,
        "WEBV-008: at least one concurrent run did the installing"
    );
    let merged: &[u8] = b"operator-a/\noperator-b\n/runs/\n/mint.toml\n/locks/\n/log.md\n";
    assert_eq!(
        fs::read(concurrent_root.join(".ratmac").join(".gitignore"))
            .expect("read the merged policy file"),
        merged,
        "WEBV-008: the operator bytes survive with each policy line exactly once, in order"
    );
    assert_eq!(
        ratmac_nodes(concurrent_root),
        BTreeMap::from([(
            ".gitignore".to_owned(),
            [b"F\0".to_vec(), merged.to_vec()].concat()
        )]),
        "WEBV-008: the Engine root holds the policy file alone - no staging or lock artifact"
    );
    let mut expected_after = before;
    expected_after.insert(
        ".ratmac/.gitignore".to_owned(),
        [b"F\0".to_vec(), merged.to_vec()].concat(),
    );
    assert_eq!(
        full_tree(concurrent_root),
        expected_after,
        "WEBV-008: nothing outside the policy file changed anywhere in the fixture"
    );
}
