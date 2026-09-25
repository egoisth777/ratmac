//! t-115 / WRS-007: abandon confirmation hints.
//!
//! Every live abandonment hint, example, and malformed-option diagnostic
//! separates addressed Run retirement from unaddressed leftover-lock cleanup.
//! An addressed Run is always shown its own `abandon <run id>` phrase, in any
//! option order. A missing address while a Run is admitted first requires
//! `--run <id>`, lists the roster, and teaches the addressed usage - never a
//! project-name phrase that cannot retire those Runs, and never a Run chosen
//! for the caller. Only the no-admitted-Run leftover-lock path teaches the
//! project-name phrase. Every refusal leaves the whole fixture byte-identical.

use ratmac_qa::support::{self, CaptureOptions, TempTree};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

/// The fixture project's directory name, so its project phrase is known.
const PROJECT: &str = "hintproj";

/// The addressed usage, exactly as `rtm abandon --help` states it.
const ADDRESSED: &str = "rtm abandon --run <id> --confirm \"abandon <run id>\"";

fn stale_lock_token(guard: &str) -> String {
    // This PID is deliberately outside the practical process-id range on the
    // supported test hosts, so the token models an owner that has exited.
    format!("ratmac-lock-v1\npid=2000000000\nguard={guard}\nnonce=0\n")
}

/// A project with `runs` admitted Runs, owned by one temporary tree.
struct Project {
    _tree: TempTree,
    root: PathBuf,
}

impl Project {
    fn new(label: &str, runs: usize) -> Self {
        let tree = TempTree::new(&format!("t115-{label}")).expect("own a fixture tree");
        let root = tree.join(PROJECT);
        for (relative, body) in [
            ("src/lib.rs", "pub fn work() {}\n"),
            (".arca/goal/spec.md", "# Spec\n"),
            (
                ".ratmac/ratmac.toml",
                "[states.intake]\nprompt = \"Integrate the issues.\"\n\n\
                 [states.build]\nprompt = \"Build the ticket.\"\n\n\
                 [[transitions]]\nfrom = \"intake\"\nto = \"build\"\n",
            ),
        ] {
            tree.write(PathBuf::from(PROJECT).join(relative), body);
        }
        let project = Project { _tree: tree, root };
        for _ in 0..runs {
            let (ok, text) = project.say(&["start"]);
            assert!(ok, "the fixture Run starts: {text}");
        }
        assert_eq!(project.roster().len(), runs, "every start mints one Run");
        project
    }

    fn rtm(&self, args: &[&str]) -> Output {
        support::command(ratmac_qa::engine_bin!(), &self.root)
            .args(args)
            .output()
            .expect("invoke rtm")
    }

    fn say(&self, args: &[&str]) -> (bool, String) {
        let output = self.rtm(args);
        (output.status.success(), support::text(&output))
    }

    fn abandon(&self, args: &[&str]) -> (bool, String) {
        let mut all = vec!["abandon"];
        all.extend_from_slice(args);
        self.say(&all)
    }

    /// Minted Run ids, sorted.
    fn roster(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(self.root.join(".ratmac/runs")) else {
            return Vec::new();
        };
        let mut ids: Vec<String> = entries
            .map(|entry| entry.expect("roster entry is readable"))
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        ids.sort();
        ids
    }

    fn admitted(&self, id: &str) -> bool {
        self.root
            .join(".ratmac/runs")
            .join(id)
            .join("run.toml")
            .is_file()
    }

    /// Every byte of the fixture, links refused.
    fn bytes(&self) -> BTreeMap<String, Vec<u8>> {
        support::capture(&self.root, CaptureOptions::default())
            .and_then(|snapshot| snapshot.refuse_links())
            .expect("capture the fixture")
            .tagged()
    }

    fn project_phrase(&self) -> String {
        format!("abandon {PROJECT}")
    }
}

/// Phrase-specific hints: every quoted string right after `--confirm ` or
/// after `phrase `.
fn phrase_hints(text: &str) -> Vec<String> {
    let mut hints = Vec::new();
    for marker in ["--confirm \"", "phrase \""] {
        for (at, _) in text.match_indices(marker) {
            let rest = &text[at + marker.len()..];
            let end = rest.find('"').expect("a quoted hint closes");
            hints.push(rest[..end].to_owned());
        }
    }
    hints
}

/// A direct library plan's refusal. In-process callers (respawn, adapters)
/// skip the command's own residue check at dispatch.
fn library_refusal(root: &Path, confirmation: Option<&str>, run: Option<&str>) -> String {
    let request = ratmac::abandon::AbandonRequest {
        confirmation: confirmation.map(str::to_owned),
        run: run.map(str::to_owned),
    };
    match ratmac::abandon::plan_abandon(root, &request) {
        Ok(plan) => panic!("the library plan must refuse {request:?}: {plan:?}"),
        Err(refusal) => refusal.to_string(),
    }
}

/// Pre-split residue refuses a direct library plan first: the refusal names
/// the residue, teaches no phrase, and changes nothing.
fn assert_library_residue_first(project: &Project, cases: &[(Option<&str>, Option<&str>)]) {
    let before = project.bytes();
    for &(confirmation, run) in cases {
        let text = library_refusal(&project.root, confirmation, run);
        assert!(
            text.contains("state.toml"),
            "the library refusal names the residue for {confirmation:?} {run:?}: {text:?}"
        );
        assert!(
            phrase_hints(&text).is_empty(),
            "the library refusal teaches no phrase for {confirmation:?} {run:?}: {text:?}"
        );
        assert_eq!(
            project.bytes(),
            before,
            "nothing changes for {confirmation:?} {run:?}"
        );
    }
}

/// The first concrete addressed command a diagnostic shows, as arguments
/// after `rtm abandon`; the `<id>` usage template is not concrete.
fn concrete_command(text: &str) -> Option<Vec<String>> {
    const HEAD: &str = "rtm abandon --run ";
    for (at, _) in text.match_indices(HEAD) {
        let rest = &text[at + HEAD.len()..];
        let id = rest.split(' ').next()?;
        if id.starts_with('<') {
            continue;
        }
        let tail = rest[id.len()..].strip_prefix(" --confirm \"")?;
        let phrase = &tail[..tail.find('"')?];
        return Some(vec![
            "--run".to_owned(),
            id.to_owned(),
            "--confirm".to_owned(),
            phrase.to_owned(),
        ]);
    }
    None
}

/// The paragraphs of a text, split on blank lines.
fn paragraphs(text: &str) -> Vec<String> {
    text.replace("\r\n", "\n")
        .split("\n\n")
        .map(str::to_owned)
        .collect()
}

/// A paragraph that teaches the project-name phrase frames it as
/// leftover-lock cleanup and never pairs it with an addressed Run.
fn assert_project_phrase_framed(source: &str, text: &str) {
    for paragraph in paragraphs(text) {
        if paragraph.contains("\"abandon <project") {
            assert!(
                paragraph.contains("leftover lock"),
                "{source}: the project phrase is taught only as leftover-lock cleanup: {paragraph:?}"
            );
            assert!(
                !paragraph.contains("--run <id> --confirm \"abandon <project"),
                "{source}: an addressed Run is never paired with the project phrase: {paragraph:?}"
            );
        }
    }
}

/// WRSV-007-01: with one and with several admitted Runs, an unaddressed
/// abandon - bare, with any confirmation, or with an unknown option -
/// requires `--run <id>`, lists the roster, and teaches the addressed usage
/// instead of a project-name retirement command; a missing or blank `--run`
/// value, in either option order, is a missing address and teaches the same
/// addressed usage; nothing changes.
#[test]
fn wrsv_007_01() {
    for count in [1, 2] {
        let project = Project::new(&format!("unaddressed-{count}"), count);
        let roster = project.roster();
        // Positive control: every Run is admitted and addressable.
        for id in &roster {
            assert!(project.admitted(id), "{id} is admitted");
            let (ok, text) = project.say(&["status", "--run", id]);
            assert!(ok, "{id} is addressable: {text}");
        }
        let before = project.bytes();
        let project_phrase = project.project_phrase();
        let cases: [&[&str]; 7] = [
            &[],
            &["--confirm"],
            &["--confirm", "yes"],
            &["--confirm", &project_phrase],
            &["--confirm", "abandon run-001"],
            &["--frob"],
            &["--frob", "--confirm", &project_phrase],
        ];
        for args in cases {
            let (ok, text) = project.abandon(args);
            assert!(
                !ok,
                "an unaddressed abandon refuses with {count} Run(s): {args:?}"
            );
            assert!(
                text.contains("requires --run <id>"),
                "the refusal first requires an address for {args:?}: {text:?}"
            );
            let listed = text
                .split("runs: ")
                .nth(1)
                .unwrap_or_else(|| panic!("the refusal shows the roster for {args:?}: {text:?}"));
            for id in &roster {
                assert!(
                    listed.contains(id.as_str()),
                    "the roster names {id} for {args:?}: {text:?}"
                );
            }
            assert!(
                text.contains(ADDRESSED),
                "the refusal teaches the addressed usage for {args:?}: {text:?}"
            );
            assert!(
                !text.contains(&project_phrase),
                "no project-name retirement phrase is offered for {args:?}: {text:?}"
            );
            assert!(
                !text.contains("abandon run-"),
                "no Run is chosen for the caller for {args:?}: {text:?}"
            );
            assert_eq!(
                project.bytes(),
                before,
                "the refusal changes nothing for {args:?}"
            );
        }
        // A valueless or blank `--run` is a missing address: it names the
        // flag, lists the roster, teaches the addressed usage, and still
        // chooses no Run - not even the only one.
        let missing: [&[&str]; 6] = [
            &["--run"],
            &["--run", ""],
            &["--run", " "],
            &["--run", "--confirm", "abandon run-001"],
            &["--confirm", "abandon run-001", "--run"],
            &["--confirm", &project_phrase, "--run", ""],
        ];
        for args in missing {
            let (ok, text) = project.abandon(args);
            assert!(
                !ok,
                "a missing address refuses with {count} Run(s): {args:?}"
            );
            assert!(
                text.contains("--run needs a run id"),
                "the refusal names the missing address for {args:?}: {text:?}"
            );
            let listed = text
                .split("runs: ")
                .nth(1)
                .unwrap_or_else(|| panic!("the refusal shows the roster for {args:?}: {text:?}"));
            for id in &roster {
                assert!(
                    listed.contains(id.as_str()),
                    "the roster names {id} for {args:?}: {text:?}"
                );
            }
            assert!(
                text.contains(ADDRESSED),
                "a missing address teaches the addressed usage for {args:?}: {text:?}"
            );
            assert!(
                !text.contains(&project_phrase) && !text.contains("abandon run-"),
                "no phrase is chosen for the caller for {args:?}: {text:?}"
            );
            assert_eq!(
                project.bytes(),
                before,
                "the refusal changes nothing for {args:?}"
            );
        }
        for id in &roster {
            assert!(project.admitted(id), "{id} stays admitted");
        }
    }
}

/// WRSV-007-02: with an addressed Run, an omitted, valueless, or truncated
/// confirmation and an unknown option are refused in either option order,
/// and every phrase-specific hint is the phrase that then retires that Run.
#[test]
fn wrsv_007_02() {
    let project = Project::new("addressed", 2);
    let target = "run-002";
    let phrase = "abandon run-002";
    let command = "rtm abandon --run run-002 --confirm \"abandon run-002\"";
    // Positive control: both Runs are admitted, so the address is the only
    // thing that tells them apart.
    assert!(project.admitted("run-001") && project.admitted(target));
    let before = project.bytes();
    let project_phrase = project.project_phrase();
    // (arguments, shows the whole command, is an unknown option)
    let cases: [(&[&str], bool, bool); 11] = [
        (&["--run", target], false, false),
        (&["--run", target, "--confirm"], true, false),
        (&["--confirm", "--run", target], true, false),
        (&["--confirm", "abandon", "--run", target], false, false),
        (&["--run", target, "--confirm", "abandon"], false, false),
        (
            &["--run", target, "--confirm", "abandon run-00"],
            false,
            false,
        ),
        (
            &["--confirm", "abandon run-001", "--run", target],
            false,
            false,
        ),
        (
            &["--confirm", &project_phrase, "--run", target],
            false,
            false,
        ),
        (&["--frob", "--run", target], true, true),
        (&["--run", target, "--frob"], true, true),
        (
            &["--confirm", phrase, "--frob", "--run", target],
            true,
            true,
        ),
    ];
    for (args, shows_command, unknown) in cases {
        let (ok, text) = project.abandon(args);
        assert!(!ok, "the malformed request refuses: {args:?}");
        let hints = phrase_hints(&text);
        assert!(
            !hints.is_empty(),
            "the refusal names the phrase for {args:?}: {text:?}"
        );
        assert!(
            hints.iter().all(|hint| hint == phrase),
            "every phrase hint is {phrase:?} for {args:?}: {hints:?} in {text:?}"
        );
        if shows_command {
            assert!(
                text.contains(command),
                "the diagnostic shows the whole addressed command for {args:?}: {text:?}"
            );
        }
        if unknown {
            assert!(
                text.contains("unsupported option --frob")
                    && text.contains("--run <id>")
                    && !text.contains("the only option is"),
                "the unknown-option diagnostic names both options for {args:?}: {text:?}"
            );
        }
        assert_eq!(
            project.bytes(),
            before,
            "the refusal changes nothing for {args:?}"
        );
    }
    // The hinted phrase is the accepted one, whichever option comes first.
    let (ok, text) = project.abandon(&["--confirm", phrase, "--run", target]);
    assert!(ok, "the hinted phrase retires {target}: {text}");
    assert!(!project.admitted(target), "{target} is retired");
    assert!(project.admitted("run-001"), "run-001 is untouched");
}

/// WRSV-007-03: the phrase and command a diagnostic displays really retire
/// the addressed Run; a project-name phrase for that Run still refuses
/// without changing anything.
#[test]
fn wrsv_007_03() {
    let project = Project::new("execute", 2);
    let target = "run-001";
    // Every refusal below is compared against the tree before the first one.
    let before = project.bytes();
    let (ok, unconfirmed) = project.abandon(&["--run", target]);
    assert!(!ok, "an unconfirmed abandon refuses: {unconfirmed}");
    assert_eq!(
        project.bytes(),
        before,
        "the unconfirmed refusal changes nothing"
    );
    let shown = phrase_hints(&unconfirmed);
    assert_eq!(
        shown,
        vec!["abandon run-001".to_owned()],
        "the refusal displays one exact phrase: {unconfirmed:?}"
    );
    let (ok, valueless) = project.abandon(&["--run", target, "--confirm"]);
    assert!(!ok, "a valueless confirmation refuses: {valueless}");
    assert_eq!(
        project.bytes(),
        before,
        "the valueless refusal changes nothing"
    );
    let command = concrete_command(&valueless)
        .unwrap_or_else(|| panic!("the diagnostic displays the whole command: {valueless:?}"));
    assert_eq!(
        command,
        ["--run", target, "--confirm", shown[0].as_str()],
        "the displayed command carries the displayed phrase: {valueless:?}"
    );
    let (ok, bare) = project.abandon(&[]);
    assert!(
        !ok && bare.contains(ADDRESSED),
        "a bare abandon teaches the addressed usage: {bare:?}"
    );
    assert_eq!(project.bytes(), before, "the bare refusal changes nothing");

    // The project phrase never retires a Run, addressed or not.
    let project_phrase = project.project_phrase();
    for args in [
        vec!["--run", target, "--confirm", &project_phrase],
        vec!["--confirm", &project_phrase, "--run", target],
        vec!["--confirm", &project_phrase],
    ] {
        let (ok, text) = project.abandon(&args);
        assert!(!ok, "the project phrase refuses: {args:?}: {text}");
        assert_eq!(project.bytes(), before, "nothing changes for {args:?}");
    }

    // Executing the displayed command retires exactly the addressed Run.
    let mut args = vec!["abandon"];
    args.extend(command.iter().map(String::as_str));
    let (ok, text) = project.say(&args);
    assert!(ok, "the displayed command retires {target}: {text}");
    assert!(!project.admitted(target), "{target} is retired");
    let log = fs::read_to_string(project.root.join(".ratmac/log.md")).expect("read history");
    assert!(
        log.contains("Abandoned: Run run-001"),
        "history records the retirement: {log:?}"
    );
    assert!(project.admitted("run-002"), "run-002 is untouched");
    let (ok, text) = project.say(&["status", "--run", "run-002"]);
    assert!(ok, "run-002 still answers: {text}");
}

/// WRSV-007-04: with only a leftover lock, every diagnostic keeps the
/// project-name cleanup phrase and frames it as leftover-lock cleanup;
/// with no Run and no leftover lock nothing is taught at all; pre-split
/// residue still refuses first without a phrase; help, module examples,
/// working instructions, and other live hints agree.
#[test]
fn wrsv_007_04() {
    let project = Project::new("leftover", 0);
    let lock = project.root.join(".ratmac/locks/root.lock");
    fs::create_dir_all(lock.parent().expect("lock folder")).expect("create lock folder");
    fs::write(&lock, stale_lock_token("root")).expect("seed a leftover lock");
    let phrase = project.project_phrase();

    // Pre-split residue refuses first and teaches no phrase.
    let residue = project.root.join(".ratmac/state.toml");
    fs::write(&residue, "state = \"intake\"\n").expect("seed pre-split residue");
    let before = project.bytes();
    let residue_cases: [&[&str]; 4] = [&[], &["--confirm"], &["--frob"], &["--confirm", &phrase]];
    for args in residue_cases {
        let (ok, text) = project.abandon(args);
        assert!(!ok, "residue refuses {args:?}: {text}");
        assert!(
            text.contains("state.toml"),
            "the residue refusal names the residue for {args:?}: {text:?}"
        );
        assert!(
            phrase_hints(&text).is_empty(),
            "the residue refusal teaches no phrase for {args:?}: {text:?}"
        );
        assert_eq!(project.bytes(), before, "nothing changes for {args:?}");
    }
    // A direct library plan refuses the residue first too, before any
    // phrase is shown or checked.
    assert_library_residue_first(
        &project,
        &[(None, None), (Some("yes"), None), (Some(&phrase), None)],
    );
    fs::remove_file(&residue).expect("clear the residue");

    // With a Run admitted, residue still refuses the library plan before the
    // address is resolved: no roster refusal, no Run phrase.
    let admitted = Project::new("residue-admitted", 1);
    fs::write(
        admitted.root.join(".ratmac/state.toml"),
        "state = \"intake\"\n",
    )
    .expect("seed pre-split residue beside an admitted Run");
    assert_library_residue_first(
        &admitted,
        &[
            (None, None),
            (Some("yes"), None),
            (Some("abandon run-001"), None),
            (None, Some("run-999")),
            (None, Some("run-001")),
            (Some("abandon run-001"), Some("run-001")),
        ],
    );
    assert!(admitted.admitted("run-001"), "run-001 stays admitted");

    // With no Run admitted, only the leftover-lock phrase is taught.
    let before = project.bytes();
    let cases: [(&[&str], bool); 4] = [
        (&[], false),
        (&["--confirm"], false),
        (&["--confirm", "abandon"], false),
        (&["--frob"], true),
    ];
    for (args, unknown) in cases {
        let (ok, text) = project.abandon(args);
        assert!(!ok, "the unconfirmed cleanup refuses: {args:?}: {text}");
        let hints = phrase_hints(&text);
        assert!(
            !hints.is_empty() && hints.iter().all(|hint| *hint == phrase),
            "every hint is the project phrase for {args:?}: {text:?}"
        );
        assert!(
            text.contains("leftover lock"),
            "the hint names leftover-lock cleanup for {args:?}: {text:?}"
        );
        if unknown {
            assert!(
                text.contains("--run <id>") && !text.contains("the only option is"),
                "the unknown-option diagnostic names both options: {text:?}"
            );
        }
        assert_eq!(project.bytes(), before, "nothing changes for {args:?}");
    }
    // Positive control: the project phrase retires the leftover lock.
    let (ok, text) = project.abandon(&["--confirm", &phrase]);
    assert!(ok, "the project phrase retires the leftover lock: {text}");
    assert!(!lock.exists(), "the leftover lock is retired");

    // With no Run and no leftover lock - never seeded, or already retired -
    // there is nothing to clean up, so no diagnostic teaches the project
    // phrase and every refusal changes nothing.
    let empty = Project::new("empty", 0);
    for (label, quiet) in [("never locked", &empty), ("after cleanup", &project)] {
        let before = quiet.bytes();
        let cases: [&[&str]; 4] = [&[], &["--confirm"], &["--frob"], &["--confirm", &phrase]];
        for args in cases {
            let (ok, text) = quiet.abandon(args);
            assert!(!ok, "{label}: nothing to retire refuses {args:?}: {text}");
            assert!(
                text.to_ascii_lowercase().contains("nothing to retire"),
                "{label}: the refusal says there is nothing to retire for {args:?}: {text:?}"
            );
            assert!(
                phrase_hints(&text).is_empty() && !text.contains(&phrase),
                "{label}: no cleanup phrase is taught for {args:?}: {text:?}"
            );
            assert_eq!(
                quiet.bytes(),
                before,
                "{label}: nothing changes for {args:?}"
            );
        }
    }

    // Help, module example, working instructions, and bootstrap hint agree.
    let (ok, help) = project.say(&["abandon", "--help"]);
    assert!(ok, "help answers: {help}");
    assert!(
        help.starts_with(&format!("Usage: {ADDRESSED}")),
        "help leads with the addressed usage: {help:?}"
    );
    assert_project_phrase_framed("rtm abandon --help", &help);

    let repo = ratmac_qa::baseline::repo_root();
    let module: String = fs::read_to_string(repo.join("src/abandon.rs"))
        .expect("read the abandon module")
        .lines()
        .filter_map(|line| line.strip_prefix("//!"))
        .map(|line| format!("{}\n", line.trim_start()))
        .collect();
    assert!(
        module.contains(ADDRESSED),
        "the module example is the addressed usage: {module:?}"
    );
    assert_project_phrase_framed("src/abandon.rs", &module);

    let schema = fs::read_to_string(repo.join(".arca/schema.md")).expect("read the schema");
    let start = schema
        .find("## Abandoning a Run")
        .expect("the schema explains abandonment");
    let rest = &schema[start + 2..];
    let section = &rest[..rest.find("\n## ").unwrap_or(rest.len())];
    assert!(
        section.contains(ADDRESSED),
        "the working instructions show the addressed usage: {section:?}"
    );
    assert_project_phrase_framed(".arca/schema.md", section);

    let bootstrap = fs::read_to_string(repo.join("tools/rtm.ps1")).expect("read the bootstrap");
    for line in bootstrap
        .lines()
        .filter(|line| line.contains("rtm abandon"))
    {
        assert!(
            line.contains(ADDRESSED),
            "the bootstrap retires a Run by its own phrase: {line:?}"
        );
    }

    // A drifted runbook's refusal names the addressed Run's exact command.
    let drifted = Project::new("drifted", 1);
    let runbook = drifted.root.join(".ratmac/ratmac.toml");
    let mut bytes = fs::read(&runbook).expect("read the runbook");
    bytes.extend_from_slice(b"# drift\n");
    fs::write(&runbook, bytes).expect("drift the runbook");
    let before = drifted.bytes();
    let (ok, text) = drifted.say(&["step", "--run", "run-001"]);
    assert!(!ok, "a drifted runbook refuses the step: {text}");
    assert_eq!(drifted.bytes(), before, "the pin refusal changes nothing");
    assert!(
        text.contains("runbook pin mismatch"),
        "the refusal is the pin mismatch: {text:?}"
    );
    assert!(
        text.contains("rtm abandon --run run-001 --confirm \"abandon run-001\""),
        "the pin refusal names the Run's own retirement command: {text:?}"
    );
}
