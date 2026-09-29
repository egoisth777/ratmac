//! t-127 / WCP-008: the edition proof preflight refuses every unproven cut.
//!
//! WCPV-022 `wcpv_022`, WCPV-023 `wcpv_023`, WCPV-024 `wcpv_024`.
//!
//! The `edition-cut` carrier is a harness contributor tool the implementer of
//! this ticket still owns, so these checks never name it at compile time: they
//! build it once per process with the usual scrubbed cargo invocation and fail
//! with a clear message while it does not exist. Every fixture is an isolated
//! temporary repository; the `edition` audit functions are the shared oracle.

use ratmac_qa::edition::{
    audit_editions, audit_sequence, edition_commit, edition_tags, repo_root, AuditFinding,
    EXAMPLE_BAR_MESSAGE, LEDGER_PATH, RECORDED_BAR,
};
use ratmac_qa::support::{self, TempTree};
use ratmac_qa::tempgit::TempRepo;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::LazyLock;
use std::thread;
use std::time::{Duration, Instant};

/// The prefix every refusal line carries, exactly as the contract fixes it.
const REFUSED_PREFIX: &str = "edition-cut: refused; ";

/// A declared check that exits 0 on a clean fixture, writing nothing.
const GREEN_CHECKS: &str = "git rev-parse HEAD\n";

/// Build the `edition-cut` harness binary once per process, or fail with the
/// reason the build gave: the carrier does not exist yet, so a red run reports
/// exactly that instead of a compile-time reference to a missing item.
fn edition_cut_binary() -> PathBuf {
    static BINARY: LazyLock<Result<PathBuf, String>> = LazyLock::new(build_edition_cut);
    match BINARY.as_ref() {
        Ok(binary) => binary.clone(),
        Err(reason) => panic!("WCPV-022/023/024: {reason}"),
    }
}

/// Build the carrier once per process, or say why in cargo's own words. The
/// failure is data, never a panic, so every red test reports it verbatim.
fn build_edition_cut() -> Result<PathBuf, String> {
    let root = repo_root();
    let build = support::command("cargo", &root)
        .args([
            "build",
            "--offline",
            "-p",
            "ratmac-qa",
            "--bin",
            "edition-cut",
        ])
        .output()
        .map_err(|error| format!("run cargo build for the edition-cut binary: {error}"))?;
    if !build.status.success() {
        return Err(format!(
            "`cargo build --offline -p ratmac-qa --bin edition-cut` must succeed; \
             the edition-cut carrier does not exist yet:\n{}",
            support::text(&build)
        ));
    }
    let binary = root
        .join("target")
        .join("debug")
        .join(format!("edition-cut{}", std::env::consts::EXE_SUFFIX));
    if !binary.is_file() {
        return Err(format!(
            "the edition-cut binary must exist at {} after the build",
            binary.display()
        ));
    }
    Ok(binary)
}

/// Give one `edition-cut` child process the isolated Git configuration every
/// fixture uses: no system config and a private empty global config.
fn isolated_git(command: &mut Command) {
    static GLOBAL: LazyLock<TempTree> = LazyLock::new(|| {
        let tree =
            TempTree::new("t127-edition-cut-gitconfig").expect("create the isolated git config");
        tree.write("gitconfig", "");
        tree
    });
    let tree = &*GLOBAL;
    command.env("GIT_CONFIG_NOSYSTEM", "1");
    command.env("GIT_CONFIG_GLOBAL", tree.join("gitconfig"));
}

/// One `edition-cut` invocation: the candidate on the command line, the tool
/// run from a neutral directory outside every fixture repository.
struct Candidate<'a> {
    repo: &'a Path,
    name: &'a str,
    message: &'a Path,
    checks: &'a Path,
    at: Option<&'a str>,
    check_only: bool,
}

fn run_edition_cut(binary: &Path, cwd: &Path, candidate: &Candidate<'_>) -> Output {
    let mut command = support::command(binary, cwd);
    isolated_git(&mut command);
    command
        .arg("--repo")
        .arg(candidate.repo)
        .arg("--name")
        .arg(candidate.name)
        .arg("--message-file")
        .arg(candidate.message)
        .arg("--checks")
        .arg(candidate.checks);
    if let Some(at) = candidate.at {
        command.arg("--at").arg(at);
    }
    if candidate.check_only {
        command.arg("--check-only");
    }
    command.output().expect("run edition-cut")
}

/// The message file and checks file of one invocation, outside the repository
/// so a candidate never dirties the tree stage 4 must find clean.
fn files(label: &str, message: &str, checks: &str) -> (TempTree, PathBuf, PathBuf) {
    let tree = TempTree::new(label).expect("create the candidate files tree");
    tree.write("message.txt", message);
    tree.write("checks.txt", checks);
    let message_path = tree.join("message.txt");
    let checks_path = tree.join("checks.txt");
    (tree, message_path, checks_path)
}

/// An isolated repository with one clean commit on `main`, at rest.
fn clean_repo(label: &str) -> TempRepo {
    let repo = TempRepo::new(label);
    repo.write("src/lib.rs", "pub fn fixture() {}\n");
    repo.commit_all("fixture base");
    repo
}

/// Every ref of a fixture with its object id, plus `HEAD`: the before/after
/// snapshot that proves a refusal moved nothing.
fn refs(repo: &TempRepo) -> BTreeSet<String> {
    let output = repo.git(&["for-each-ref", "--format=%(refname) %(objectname)"]);
    assert!(
        output.status.success(),
        "list the refs of {}: {}",
        repo.root().display(),
        support::text(&output)
    );
    let mut lines: BTreeSet<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    lines.insert(format!("HEAD {}", repo.head()));
    lines
}

/// The working-tree state of a fixture.
fn porcelain(repo: &TempRepo) -> String {
    let output = repo.git(&["status", "--porcelain"]);
    assert!(output.status.success(), "read the tree state");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A green `--check-only` line, byte for byte.
fn check_only_line(name: &str, hash: &str) -> String {
    format!(
        "edition-cut: preflight green for {name} at {hash}; \
         --check-only, nothing was created\n"
    )
}

/// A green cut line, byte for byte.
fn cut_line(name: &str, hash: &str) -> String {
    format!(
        "edition-cut: cut {name} at {hash}; \
         the ledger row follows in the recording landing (ELR-001)\n"
    )
}

/// Assert the exact refusal shape: exit 1, empty stdout, one stderr line.
fn assert_refused(output: &Output, text: &str, twin: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{twin}: a refusal exits 1; stdout:\n{stdout}stderr:\n{stderr}"
    );
    assert_eq!(stdout, "", "{twin}: a refusal writes nothing to stdout");
    assert_eq!(
        stderr,
        format!("{REFUSED_PREFIX}{text}\n"),
        "{twin}: the refusal is one exact stderr line"
    );
}

/// Assert the exact success shape: exit 0, empty stderr, one stdout line.
fn assert_green(output: &Output, line: &str, twin: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{twin}: a green run exits 0; stdout:\n{stdout}stderr:\n{stderr}"
    );
    assert_eq!(stdout, line, "{twin}: the green run prints one exact line");
    assert_eq!(stderr, "", "{twin}: a green run writes no stderr");
}

/// Wait until the paused child publishes its marker, or name its early exit.
fn wait_for_marker(marker: &Path, child: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if marker.is_file() {
            return;
        }
        match child.try_wait() {
            Ok(Some(status)) => panic!(
                "the paused edition-cut exited with {status} before writing the pause marker; \
                 the after-checks seam did not pause"
            ),
            Ok(None) => {}
            Err(error) => panic!("inspect the paused edition-cut child: {error}"),
        }
        assert!(
            Instant::now() < deadline,
            "edition-cut never wrote the pause marker {}",
            marker.display()
        );
        thread::sleep(Duration::from_millis(20));
    }
}

/// The message of a candidate that carries every bar phrase except `phrase`,
/// still ending with exactly one newline and no trailing blank line.
fn without_phrase(phrase: &str) -> String {
    let deficient = EXAMPLE_BAR_MESSAGE.replace(phrase, "REDACTED");
    let still_missing: Vec<&str> = RECORDED_BAR
        .iter()
        .copied()
        .filter(|bar| !deficient.contains(bar))
        .collect();
    assert_eq!(
        still_missing,
        vec![phrase],
        "the fixture for {phrase:?} removes exactly that proof element"
    );
    deficient
}

/// WCPV-022: candidate text missing each required proof element is rejected
/// before any tag exists with the missing phrases named in bar order, while
/// accepted text passes the same validator the audit uses, so the stage-2
/// refusal and the audit's `documented` finding name the same phrases for the
/// same bytes.
#[test]
fn wcpv_022() {
    let binary = edition_cut_binary();

    // Positive control: on a clean, at-rest fixture the accepted bar message
    // passes the validator, and the audit documents the same bytes.
    let green = clean_repo("t127-022-green");
    let (carrier, message, checks) =
        files("t127-022-green-files", EXAMPLE_BAR_MESSAGE, GREEN_CHECKS);
    let head = green.head();
    let before = refs(&green);
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: green.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: true,
        },
    );
    assert_green(
        &output,
        &check_only_line("edition-001", &head),
        "the accepted candidate text passes the same validator",
    );
    assert_eq!(refs(&green), before, "the check-only run creates no ref");
    assert!(
        edition_tags(green.root())
            .expect("list the green fixture's tags")
            .is_empty(),
        "no tag exists after the accepted text passed"
    );
    let audited = clean_repo("t127-022-audit-accepted");
    audited.git(&["tag", "-a", "edition-001", "-m", EXAMPLE_BAR_MESSAGE]);
    assert!(
        audit_editions(audited.root())
            .expect("audit the accepted message")
            .is_empty(),
        "the audit documents the same accepted bytes"
    );

    // Refusal twins: stage 2 reads the message before any git command, so one
    // at-rest repository serves every text twin without a tag ever existing.
    let text = clean_repo("t127-022-text");
    let before = refs(&text);

    for phrase in RECORDED_BAR {
        let (twin_files, message, checks) = files(
            &format!("t127-022-missing-{}", phrase.len()),
            &without_phrase(phrase),
            GREEN_CHECKS,
        );
        let output = run_edition_cut(
            &binary,
            twin_files.path(),
            &Candidate {
                repo: text.root(),
                name: "edition-001",
                message: &message,
                checks: &checks,
                at: None,
                check_only: false,
            },
        );
        assert_refused(
            &output,
            &format!("the message does not record {phrase}"),
            &format!("a candidate missing {phrase:?}"),
        );
        assert_eq!(
            refs(&text),
            before,
            "the stage-2 refusal for {phrase:?} creates no ref"
        );
    }

    // Every element missing: the phrases are named in RECORDED_BAR order, and
    // the audit's `documented` finding for the same bytes carries the same
    // detail, so both name one validator's answer.
    let all_missing = "edition test\n";
    let (twin_files, message, checks) = files("t127-022-missing-all", all_missing, GREEN_CHECKS);
    let output = run_edition_cut(
        &binary,
        twin_files.path(),
        &Candidate {
            repo: text.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: false,
        },
    );
    let audited = clean_repo("t127-022-audit-missing-all");
    audited.git(&["tag", "-a", "edition-001", "-m", all_missing]);
    let findings = audit_editions(audited.root()).expect("audit the message missing everything");
    assert_eq!(
        findings,
        vec![AuditFinding {
            tag: "edition-001".to_owned(),
            property: "documented".to_owned(),
            detail: format!("the message does not record {}", RECORDED_BAR.join(", ")),
        }],
        "the audit names every missing phrase in bar order"
    );
    assert_refused(
        &output,
        &findings[0].detail,
        "a candidate missing every element",
    );
    assert_eq!(refs(&text), before, "the refusal creates no ref");

    // The empty message is the validator's other branch, in the audit's words.
    let (twin_files, message, checks) = files("t127-022-empty", "\n", GREEN_CHECKS);
    let output = run_edition_cut(
        &binary,
        twin_files.path(),
        &Candidate {
            repo: text.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: false,
        },
    );
    let audited = clean_repo("t127-022-audit-empty");
    audited.git(&["tag", "-a", "edition-001", "-m", ""]);
    let findings = audit_editions(audited.root()).expect("audit the empty message");
    assert_eq!(
        findings,
        vec![AuditFinding {
            tag: "edition-001".to_owned(),
            property: "documented".to_owned(),
            detail: "the message is empty, so the bar is recorded nowhere".to_owned(),
        }],
        "the audit reports the empty message in its own words"
    );
    assert_refused(&output, &findings[0].detail, "an empty candidate message");
    assert_eq!(refs(&text), before, "the refusal creates no ref");

    // A complete but unnormalized tail refuses before any tag exists.
    for (label, tail) in [
        (
            "no-newline",
            EXAMPLE_BAR_MESSAGE.strip_suffix('\n').unwrap(),
        ),
        ("blank-line", &format!("{EXAMPLE_BAR_MESSAGE}\n")),
    ] {
        let (twin_files, message, checks) =
            files(&format!("t127-022-shape-{label}"), tail, GREEN_CHECKS);
        let output = run_edition_cut(
            &binary,
            twin_files.path(),
            &Candidate {
                repo: text.root(),
                name: "edition-001",
                message: &message,
                checks: &checks,
                at: None,
                check_only: false,
            },
        );
        assert_refused(
            &output,
            &format!(
                "the message file {} must end with exactly one newline and no trailing blank \
                 line, so the candidate is a normalized annotated-tag message",
                message.display()
            ),
            &format!("a candidate whose tail is {label}"),
        );
        assert_eq!(refs(&text), before, "the shape refusal creates no ref");
    }
    assert_eq!(refs(&text), before, "no text twin ever created a ref");
}

/// WCPV-023: every refusal twin - dirty tree, stale proof, failed check,
/// moved target, duplicate name, invalid sequence, unreachable target, and
/// each unfinished-cycle record planted alone in an otherwise clean committed
/// fixture - exits 1 with the contract's exact stderr line before creating
/// any tag, leaving tag refs and existing annotations byte-identical.
#[test]
fn wcpv_023() {
    let binary = edition_cut_binary();

    // Positive control: the same clean fixture's whole preflight is green.
    let green = clean_repo("t127-023-green");
    let (carrier, message, checks) =
        files("t127-023-green-files", EXAMPLE_BAR_MESSAGE, GREEN_CHECKS);
    let head = green.head();
    let before = refs(&green);
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: green.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: true,
        },
    );
    assert_green(
        &output,
        &check_only_line("edition-001", &head),
        "the clean fixture's preflight",
    );
    assert_eq!(refs(&green), before, "a green check-only run moves no ref");

    // Dirty tree: an untracked entry refuses at stage 4 and stays the
    // caller's dirt.
    let dirty = clean_repo("t127-023-dirty");
    let dirt_head = dirty.head();
    dirty.write("dirt.txt", "loose\n");
    let (carrier, message, checks) =
        files("t127-023-dirty-files", EXAMPLE_BAR_MESSAGE, GREEN_CHECKS);
    let before = refs(&dirty);
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: dirty.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: false,
        },
    );
    assert_refused(
        &output,
        &format!("the working tree is not clean at {dirt_head}: ?? dirt.txt"),
        "the dirty tree",
    );
    assert_eq!(refs(&dirty), before, "the dirty refusal moves no ref");
    assert_eq!(porcelain(&dirty), "?? dirt.txt\n", "the dirt is untouched");

    // Stale proof: the declared check passed at the parent commit but fails
    // at the candidate, so only its execution can count.
    let stale = TempRepo::new("t127-023-stale");
    stale.write("src/lib.rs", "pub fn fixture() {}\n");
    stale.write("flag", "still proven\n");
    stale.commit_all("base with the proven flag");
    let removed = stale.git(&["rm", "--quiet", "--", "flag"]);
    assert!(removed.status.success(), "remove the proven flag");
    stale.commit_all("the flag is gone");
    let (carrier, message, checks) = files(
        "t127-023-stale-files",
        EXAMPLE_BAR_MESSAGE,
        "git cat-file -e HEAD:flag\n",
    );
    let before = refs(&stale);
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: stale.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: false,
        },
    );
    assert_refused(
        &output,
        &format!(
            "the declared check at {} line 1 (git cat-file -e HEAD:flag) exited 1",
            checks.display()
        ),
        "the stale proof",
    );
    assert_eq!(refs(&stale), before, "the stale-proof refusal moves no ref");

    // Failed check: a green tree whose declared check simply fails.
    let failed = TempRepo::new("t127-023-failed");
    failed.write("one.txt", "one\n");
    failed.write("two.txt", "two\n");
    failed.commit_all("base with differing files");
    let (carrier, message, checks) = files(
        "t127-023-failed-files",
        EXAMPLE_BAR_MESSAGE,
        "git diff --no-index one.txt two.txt\n",
    );
    let before = refs(&failed);
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: failed.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: false,
        },
    );
    assert_refused(
        &output,
        &format!(
            "the declared check at {} line 1 (git diff --no-index one.txt two.txt) exited 1",
            checks.display()
        ),
        "the failed check",
    );
    assert_eq!(
        refs(&failed),
        before,
        "the failed-check refusal moves no ref"
    );

    // Moved target: `--at` names an older commit while `HEAD` has advanced.
    let moved = clean_repo("t127-023-moved");
    let older = moved.head();
    moved.write("src/later.rs", "pub fn later() {}\n");
    moved.commit_all("advance past the declared target");
    let newer = moved.head();
    let (carrier, message, checks) =
        files("t127-023-moved-files", EXAMPLE_BAR_MESSAGE, GREEN_CHECKS);
    let before = refs(&moved);
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: moved.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: Some(&older),
            check_only: false,
        },
    );
    assert_refused(
        &output,
        &format!(
            "HEAD is {newer}, not the declared target commit {older}; \
             check out the commit being proven"
        ),
        "the moved target",
    );
    assert_eq!(
        refs(&moved),
        before,
        "the moved-target refusal moves no ref"
    );

    // Duplicate name: the existing edition stands, byte-identical.
    let duplicate = clean_repo("t127-023-duplicate");
    duplicate.git(&["tag", "-a", "edition-001", "-m", EXAMPLE_BAR_MESSAGE]);
    let standing = duplicate.head();
    let (carrier, message, checks) = files(
        "t127-023-duplicate-files",
        EXAMPLE_BAR_MESSAGE,
        GREEN_CHECKS,
    );
    let before = refs(&duplicate);
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: duplicate.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: false,
        },
    );
    assert_refused(
        &output,
        "edition-001 already exists, and an edition is never moved or overwritten (EDN-003)",
        "the duplicate name",
    );
    assert_eq!(
        refs(&duplicate),
        before,
        "the duplicate-name refusal leaves every ref, the tag included, as found"
    );
    assert_eq!(
        edition_commit(duplicate.root(), "edition-001").expect("resolve the standing edition"),
        standing,
        "the standing edition still marks the same commit"
    );

    // Invalid sequence, three shapes: a malformed resembling name, a hole
    // below the top, and a candidate that skips the next unused number.
    let malformed = clean_repo("t127-023-malformed");
    malformed.git(&["tag", "edition-2"]);
    let (carrier, message, checks) = files(
        "t127-023-malformed-files",
        EXAMPLE_BAR_MESSAGE,
        GREEN_CHECKS,
    );
    let before = refs(&malformed);
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: malformed.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: false,
        },
    );
    assert_refused(
        &output,
        "the existing tag edition-2 is not exactly edition-NNN with three digits, \
         so it marks nothing",
        "the malformed resembling tag",
    );
    assert_eq!(refs(&malformed), before, "the naming refusal moves no ref");

    let hole = clean_repo("t127-023-hole");
    hole.git(&["tag", "-a", "edition-001", "-m", EXAMPLE_BAR_MESSAGE]);
    hole.git(&["tag", "-a", "edition-003", "-m", EXAMPLE_BAR_MESSAGE]);
    let (carrier, message, checks) =
        files("t127-023-hole-files", EXAMPLE_BAR_MESSAGE, GREEN_CHECKS);
    let before = refs(&hole);
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: hole.root(),
            name: "edition-004",
            message: &message,
            checks: &checks,
            at: None,
            check_only: false,
        },
    );
    assert_refused(
        &output,
        "the sequence reaches 003 with no 002, so a citation to it resolves to nothing",
        "the hole below the top",
    );
    assert_eq!(refs(&hole), before, "the hole refusal moves no ref");

    let skip = clean_repo("t127-023-skip");
    skip.git(&["tag", "-a", "edition-001", "-m", EXAMPLE_BAR_MESSAGE]);
    let (carrier, message, checks) =
        files("t127-023-skip-files", EXAMPLE_BAR_MESSAGE, GREEN_CHECKS);
    let before = refs(&skip);
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: skip.root(),
            name: "edition-003",
            message: &message,
            checks: &checks,
            at: None,
            check_only: false,
        },
    );
    assert_refused(
        &output,
        "the next unused number is 002, so the candidate is edition-002, not edition-003",
        "the candidate that skips a number",
    );
    assert_eq!(
        refs(&skip),
        before,
        "the skipped-number refusal moves no ref"
    );

    // Unreachable target: the proven commit never merged to the trunk.
    let aside = clean_repo("t127-023-unreachable");
    aside.git(&["checkout", "--quiet", "-b", "sidetrack"]);
    aside.write("src/aside.rs", "pub fn aside() {}\n");
    aside.commit_all("work that never merged");
    let aside_head = aside.head();
    let (carrier, message, checks) = files(
        "t127-023-unreachable-files",
        EXAMPLE_BAR_MESSAGE,
        GREEN_CHECKS,
    );
    let before = refs(&aside);
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: aside.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: false,
        },
    );
    assert_refused(
        &output,
        &format!("commit {aside_head} is not an ancestor of main"),
        "the unreachable target",
    );
    assert_eq!(refs(&aside), before, "the unreachable refusal moves no ref");

    // Unfinished cycle, each offender planted alone in an otherwise clean
    // committed fixture whose declared checks pass.
    let intake = clean_repo("t127-023-intake");
    intake.write(".arca/issue/i-900-pending/index.md", "an intake bundle\n");
    intake.commit_all("a pending intake bundle");
    let (carrier, message, checks) =
        files("t127-023-intake-files", EXAMPLE_BAR_MESSAGE, GREEN_CHECKS);
    let before = refs(&intake);
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: intake.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: false,
        },
    );
    assert_refused(
        &output,
        "the cycle is not at rest: the intake work area holds i-900-pending",
        "the pending intake",
    );
    assert_eq!(refs(&intake), before, "the intake refusal moves no ref");

    let deferred = clean_repo("t127-023-deferred");
    deferred.write(".arca/issue/deferred/i-901-later/index.md", "deferred\n");
    deferred.commit_all("a deferred bundle");
    let (carrier, message, checks) =
        files("t127-023-deferred-files", EXAMPLE_BAR_MESSAGE, GREEN_CHECKS);
    let deferred_head = deferred.head();
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: deferred.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: true,
        },
    );
    assert_green(
        &output,
        &check_only_line("edition-001", &deferred_head),
        "a deferred bundle does not block the intake area",
    );

    let open = clean_repo("t127-023-open");
    open.write(
        ".arca/ticket/t-900-open.md",
        "---\nticket-id: \"t-900\"\nstatus: \"approved\"\n---\n\n# Open work\n\nNot closed yet.\n",
    );
    open.commit_all("an open work item");
    let (carrier, message, checks) =
        files("t127-023-open-files", EXAMPLE_BAR_MESSAGE, GREEN_CHECKS);
    let before = refs(&open);
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: open.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: false,
        },
    );
    assert_refused(
        &output,
        "the cycle is not at rest: the work item .arca/ticket/t-900-open.md is not passed",
        "the open work item",
    );
    assert_eq!(refs(&open), before, "the open-item refusal moves no ref");

    let closed = clean_repo("t127-023-closed");
    closed.write(
        ".arca/ticket/t-901-done.md",
        "---\nticket-id: \"t-901\"\nstatus: \"passed\"\n---\n\n# Done work\n",
    );
    closed.commit_all("a passed work item");
    let (carrier, message, checks) =
        files("t127-023-closed-files", EXAMPLE_BAR_MESSAGE, GREEN_CHECKS);
    let closed_head = closed.head();
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: closed.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: true,
        },
    );
    assert_green(
        &output,
        &check_only_line("edition-001", &closed_head),
        "a passed work item is at rest",
    );

    let gap = clean_repo("t127-023-gap");
    gap.write(
        ".arca/residual/res-900.md",
        "# Residual Record\n\n```yaml\nresidual-id: \"res-900\"\nstatus: \"missing\"\n```\n",
    );
    gap.commit_all("an unproven gap record");
    let (carrier, message, checks) = files("t127-023-gap-files", EXAMPLE_BAR_MESSAGE, GREEN_CHECKS);
    let before = refs(&gap);
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: gap.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: false,
        },
    );
    assert_refused(
        &output,
        "the cycle is not at rest: the gap record .arca/residual/res-900.md is missing, \
         not satisfied",
        "the unproven gap record",
    );
    assert_eq!(refs(&gap), before, "the gap refusal moves no ref");

    let proven = clean_repo("t127-023-gap-proven");
    proven.write(
        ".arca/residual/res-901.md",
        "# Residual Record\n\n```yaml\nresidual-id: \"res-901\"\nstatus: \"satisfied\"\n```\n",
    );
    proven.commit_all("a satisfied gap record");
    let (carrier, message, checks) = files(
        "t127-023-gap-proven-files",
        EXAMPLE_BAR_MESSAGE,
        GREEN_CHECKS,
    );
    let proven_head = proven.head();
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: proven.root(),
            name: "edition-001",
            message: &message,
            checks: &checks,
            at: None,
            check_only: true,
        },
    );
    assert_green(
        &output,
        &check_only_line("edition-001", &proven_head),
        "a satisfied gap record is at rest",
    );

    // The stage-10 recheck: the branch advances while the invocation is
    // paused after its checks, and the byte-identical moved-target refusal
    // is the catch - still with no tag created.
    let raced = clean_repo("t127-023-recheck");
    let resolved = raced.head();
    let pause_files = TempTree::new("t127-023-recheck-seam").expect("create the pause seam tree");
    let marker = pause_files.join("paused.marker");
    let release = pause_files.join("release");
    let (message_tree, message, checks) =
        files("t127-023-recheck-files", EXAMPLE_BAR_MESSAGE, GREEN_CHECKS);
    let mut child = {
        let mut command = support::command(&binary, pause_files.path());
        isolated_git(&mut command);
        command
            .arg("--repo")
            .arg(raced.root())
            .arg("--name")
            .arg("edition-001")
            .arg("--message-file")
            .arg(&message)
            .arg("--checks")
            .arg(&checks)
            .env("RATMAC_TEST_EDITION_CUT_PAUSE", "after-checks")
            .env("RATMAC_TEST_EDITION_CUT_PAUSE_MARKER", &marker)
            .env("RATMAC_TEST_EDITION_CUT_PAUSE_RELEASE", &release)
            .env("RATMAC_TEST_EDITION_CUT_PAUSE_TIMEOUT_MILLIS", "120000")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn the paused edition-cut")
    };
    wait_for_marker(&marker, &mut child);
    raced.write("late.txt", "the branch advanced during the pause\n");
    raced.commit_all("advance the branch during the pause");
    let advanced = raced.head();
    std::fs::write(&release, "release\n").expect("release the paused edition-cut");
    let output = child
        .wait_with_output()
        .expect("collect the raced edition-cut");
    drop(message_tree);
    assert_refused(
        &output,
        &format!(
            "HEAD is {advanced}, not the declared target commit {resolved}; \
             check out the commit being proven"
        ),
        "the branch advanced between the checks and the write",
    );
    assert!(
        edition_tags(raced.root())
            .expect("list the raced fixture's tags")
            .is_empty(),
        "the recheck catch creates no tag"
    );
    assert_eq!(porcelain(&raced), "", "the recheck catch writes no file");
}

/// WCPV-024: a proven clean fixture creates exactly one annotated tag whose
/// message is the exact validated bytes and whose target is the checked
/// commit, `audit_editions` passes immediately while the full
/// `audit_sequence` still reports only the missing ledger row until the
/// ELR-001 recording landing, and a repeated cut refuses leaving the tag
/// byte-identical.
#[test]
fn wcpv_024() {
    let binary = edition_cut_binary();

    // The fixture: one recorded, healthy edition and a clean later commit on
    // `main`, at rest, so the next cut is `edition-002` at `HEAD`.
    let repo = clean_repo("t127-024-cut");
    let first = repo.head();
    repo.git(&["tag", "-a", "edition-001", "-m", EXAMPLE_BAR_MESSAGE]);
    repo.write(
        LEDGER_PATH,
        &format!(
            "# Editions\n\n| Edition | Commit | What it marks |\n| :--- | :--- | :--- |\n\
             | `edition-001` | `{first}` | fixture |\n"
        ),
    );
    repo.commit_all("record the first edition");
    let head = repo.head();
    assert!(
        audit_sequence(repo.root())
            .expect("audit the fixture's sequence before the cut")
            .is_empty(),
        "the fixture starts with a healthy recorded sequence"
    );

    let (carrier, message, checks) = files("t127-024-files", EXAMPLE_BAR_MESSAGE, GREEN_CHECKS);
    let before = refs(&repo);
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: repo.root(),
            name: "edition-002",
            message: &message,
            checks: &checks,
            at: None,
            check_only: false,
        },
    );
    assert_green(
        &output,
        &cut_line("edition-002", &head),
        "the proven clean cut",
    );

    // Exactly one new ref: the annotated tag on the checked commit.
    let created = repo.git(&["rev-parse", "refs/tags/edition-002"]);
    assert!(created.status.success(), "resolve the created tag");
    let created_id = String::from_utf8_lossy(&created.stdout).trim().to_owned();
    let mut after = before.clone();
    after.insert(format!("refs/tags/edition-002 {created_id}"));
    assert_eq!(
        refs(&repo),
        after,
        "the cut changes exactly one thing: one new annotated tag ref"
    );
    assert_eq!(porcelain(&repo), "", "the tool writes no working-tree file");
    assert_eq!(
        edition_tags(repo.root()).expect("list the cut fixture's tags"),
        vec!["edition-001".to_owned(), "edition-002".to_owned()],
        "the cut creates exactly one new edition"
    );
    let kind = repo.git(&["cat-file", "-t", "refs/tags/edition-002"]);
    assert_eq!(
        String::from_utf8_lossy(&kind.stdout).trim(),
        "tag",
        "the created ref points at a tag object"
    );
    assert_eq!(
        edition_commit(repo.root(), "edition-002").expect("resolve the new edition"),
        head,
        "the new edition marks exactly the checked commit"
    );
    let stored = repo.git(&["cat-file", "-p", "refs/tags/edition-002"]);
    let raw = String::from_utf8_lossy(&stored.stdout);
    let (_, message_bytes) = raw
        .split_once("\n\n")
        .expect("the tag object separates its headers from its message");
    assert_eq!(
        message_bytes, EXAMPLE_BAR_MESSAGE,
        "the stored annotation is the exact validated candidate bytes"
    );

    // The post-cut audit boundary: the annotation audit passes immediately,
    // and the structural audit reports only the missing ledger row.
    assert!(
        audit_editions(repo.root())
            .expect("audit the cut fixture's editions")
            .is_empty(),
        "the cut edition is annotated, documented, and reachable"
    );
    assert_eq!(
        audit_sequence(repo.root()).expect("audit the cut fixture's sequence"),
        vec![AuditFinding {
            tag: "edition-002".to_owned(),
            property: "immutable".to_owned(),
            detail: format!("{LEDGER_PATH} records nothing for it, so a move would be invisible"),
        }],
        "until the ELR-001 recording landing, the just-cut edition lacks its ledger row and \
         nothing else is wrong"
    );

    // A repeated cut refuses and leaves the tag byte-identical.
    let output = run_edition_cut(
        &binary,
        carrier.path(),
        &Candidate {
            repo: repo.root(),
            name: "edition-002",
            message: &message,
            checks: &checks,
            at: None,
            check_only: false,
        },
    );
    assert_refused(
        &output,
        "edition-002 already exists, and an edition is never moved or overwritten (EDN-003)",
        "the repeated cut",
    );
    assert_eq!(
        refs(&repo),
        after,
        "the repeated cut leaves every ref, the created tag included, byte-identical"
    );
    let recheck = repo.git(&["rev-parse", "refs/tags/edition-002"]);
    assert_eq!(
        String::from_utf8_lossy(&recheck.stdout).trim(),
        created_id,
        "the tag object was not rewritten"
    );
}
