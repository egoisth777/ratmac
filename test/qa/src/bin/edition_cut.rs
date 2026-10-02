//! edition-cut: the WCP-008 edition proof preflight and its one tag creation.
//!
//! A harness contributor tool, not an Engine command: it takes no locks,
//! touches no `.ratmac/` Engine root, writes no operation log, and its only
//! operational write is the one non-overwriting annotated-tag creation of
//! stage 10. The proof-text contract and the git, trunk, naming, and hole
//! words all come from the shared oracle module [`ratmac_qa::edition`], so
//! the preflight and the audit name the same state in the same bytes. Every
//! refusal is exit 1, empty stdout, and one LF-terminated stderr line
//! `edition-cut: refused; <text>`; the stage texts are fixed by the t-127
//! contract and both suites judge them byte for byte.
//!
//! Usage:
//! `edition-cut --repo <path> --name <edition-NNN> --message-file <path>
//!              --checks <path> [--at <rev>] [--check-only]`

use ratmac_qa::edition::{
    edition_commit, edition_number, edition_tags, git, missing_phrases, trunk_name,
};
use ratmac_qa::support;
use std::ffi::OsString;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(text) => {
            eprintln!("edition-cut: refused; {text}");
            ExitCode::from(1)
        }
    }
}

/// The whole operation: stages 0 through 11, refusing at the first failure.
fn run() -> Result<(), String> {
    // Stage 0: the test seam's arguments, before any git command.
    let seam = Seam::from_env()?;

    // Stage 1: own inputs.
    let invocation = Invocation::parse(std::env::args_os().skip(1).collect())?;
    let Invocation {
        repo,
        name,
        message_file,
        checks_file,
        at,
        check_only,
    } = invocation;
    let shown_message = message_file.to_string_lossy().into_owned();
    let shown_checks = checks_file.to_string_lossy().into_owned();
    let message = read_message(&message_file, &shown_message)?;
    let checks = parse_checks(&checks_file, &shown_checks)?;

    // Stage 2: the proof text, through the same validator the audit uses.
    if message.trim().is_empty() {
        return Err("the message is empty, so the bar is recorded nowhere".to_owned());
    }
    let missing = missing_phrases(&message);
    if !missing.is_empty() {
        return Err(format!(
            "the message does not record {}",
            missing.join(", ")
        ));
    }
    // A blank final line is one holding nothing or only spaces and tabs;
    // trailing spaces after text stay legal and are stored verbatim.
    let blank_final_line = message.strip_suffix('\n').map(|body| {
        match body.rsplit_once('\n') {
            Some((_, last_line)) => last_line,
            None => body,
        }
        .chars()
        .all(|character| character == ' ' || character == '\t')
    });
    if !message.ends_with('\n') || blank_final_line == Some(true) {
        return Err(format!(
            "the message file {shown_message} must end with exactly one newline and no trailing \
             blank line, so the candidate is a normalized annotated-tag message"
        ));
    }

    // Stage 3: resolve the target once.
    let resolved = git(
        &repo,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{at}^{{commit}}"),
        ],
    )
    .map_err(|error| format!("cannot resolve the target {at} to one commit: {error}"))?
    .trim()
    .to_owned();

    // Stage 4: a clean checkout proven to be the resolved commit.
    clean_checkout(&repo, &resolved)?;

    // Stage 5: the existing names are healthy; the well-formed numbers follow.
    let numbers = naming_health(&repo)?;

    // Stage 6: the candidate name is absent and takes the next number.
    name_absent(&repo, &name)?;
    let next = numbers.last().copied().unwrap_or(0) + 1;
    if edition_number(&name) != Some(next) {
        return Err(format!(
            "the next unused number is {next:03}, so the candidate is edition-{next:03}, \
             not {name}"
        ));
    }

    // Stage 7: the resolved commit is reachable from the trunk.
    let trunk = trunk_name(&repo)?;
    let reachable = support::command("git", &repo)
        .args(["merge-base", "--is-ancestor", &resolved, &trunk])
        .output()
        .map_err(|error| format!("git merge-base could not run: {error}"))?;
    if !reachable.status.success() {
        return Err(format!("commit {resolved} is not an ancestor of {trunk}"));
    }

    // Stage 8: the candidate commit's rest records satisfy EDN-001.
    at_rest(&repo, &resolved)?;

    // Stage 9: freshness is execution - every declared check runs and exits 0.
    run_checks(&repo, &shown_checks, &checks)?;

    // The after-checks pause: the only seam this tool carries.
    if let Some(seam) = seam.as_ref() {
        pause_at_after_checks(seam)?;
    }

    if check_only {
        println!(
            "edition-cut: preflight green for {name} at {resolved}; \
             --check-only, nothing was created"
        );
        return Ok(());
    }

    // Stage 10: recheck, then create - one non-overwriting verbatim tag.
    clean_checkout(&repo, &resolved)?;
    name_absent(&repo, &name)?;
    at_rest(&repo, &resolved)?;
    create_tag(&repo, &name, &resolved, &message)?;

    // Stage 11: read the created tag back; a disagreement is never repaired.
    verify_created(&repo, &name, &resolved, &message)?;
    println!(
        "edition-cut: cut {name} at {resolved}; \
         the ledger row follows in the recording landing (ELR-001)"
    );
    Ok(())
}

/// The parsed command surface: exactly the contract's flags, nothing else.
struct Invocation {
    repo: PathBuf,
    name: String,
    message_file: OsString,
    checks_file: OsString,
    at: String,
    check_only: bool,
}

impl Invocation {
    /// Refuse any other token, any valueless flag, and any missing required
    /// flag, before any git command runs.
    fn parse(args: Vec<OsString>) -> Result<Invocation, String> {
        let mut repo = None;
        let mut name = None;
        let mut message_file = None;
        let mut checks_file = None;
        let mut at = None;
        let mut check_only = false;

        let mut tokens = args.into_iter();
        while let Some(token) = tokens.next() {
            let flag = token.to_string_lossy().into_owned();
            match flag.as_str() {
                "--repo" | "--name" | "--message-file" | "--checks" | "--at" => {
                    let value = tokens
                        .next()
                        .ok_or_else(|| format!("the argument {flag} requires a value"))?;
                    match flag.as_str() {
                        "--repo" => repo = Some(value),
                        "--name" => name = Some(value),
                        "--message-file" => message_file = Some(value),
                        "--checks" => checks_file = Some(value),
                        _ => at = Some(value),
                    }
                }
                "--check-only" => check_only = true,
                other => return Err(format!("unknown argument {other}")),
            }
        }

        let repo = repo.ok_or_else(|| "missing required argument --repo".to_owned())?;
        let name = name.ok_or_else(|| "missing required argument --name".to_owned())?;
        let message_file =
            message_file.ok_or_else(|| "missing required argument --message-file".to_owned())?;
        let checks_file =
            checks_file.ok_or_else(|| "missing required argument --checks".to_owned())?;
        let name = name.to_string_lossy().into_owned();
        if edition_number(&name).is_none() {
            return Err(format!(
                "--name {name} is not an edition name of the form edition-NNN"
            ));
        }
        Ok(Invocation {
            repo: PathBuf::from(repo),
            name,
            message_file,
            checks_file,
            at: at
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| "HEAD".to_owned()),
            check_only,
        })
    }
}

/// Stage 1: the message file, readable UTF-8 or a named refusal.
fn read_message(path: &OsString, shown: &str) -> Result<String, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("cannot read the message file {shown}: {error}"))?;
    String::from_utf8(bytes).map_err(|_| {
        format!("the message file {shown} is not valid UTF-8, so no tag can carry it verbatim")
    })
}

/// One declared check: its 1-based line and its whitespace-split pieces.
struct DeclaredCheck {
    line: usize,
    tokens: Vec<String>,
}

/// Stage 1: the checks file - UTF-8, at least one check, every line one
/// program and its arguments (no blank line, no leading whitespace). A final
/// LF ends the last line rather than starting an empty one.
fn parse_checks(path: &OsString, shown: &str) -> Result<Vec<DeclaredCheck>, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("cannot read the checks file {shown}: {error}"))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| format!("the checks file {shown} is not valid UTF-8"))?;
    if text.is_empty() {
        return Err(format!("the checks file {shown} declares no check"));
    }
    let mut body = text.as_str();
    if body.ends_with('\n') {
        body = &body[..body.len() - 1];
    }
    let mut checks = Vec::new();
    for (index, line) in body.split('\n').enumerate() {
        let number = index + 1;
        if line.is_empty() || line.starts_with(char::is_whitespace) {
            return Err(format!(
                "the checks file {shown} line {number} is not one program and its arguments"
            ));
        }
        checks.push(DeclaredCheck {
            line: number,
            tokens: line.split_whitespace().map(str::to_owned).collect(),
        });
    }
    Ok(checks)
}

/// Stage 4 (and its stage 10 recheck): `HEAD` is exactly the resolved commit
/// and `git status --porcelain` is empty.
fn clean_checkout(repo: &Path, resolved: &str) -> Result<(), String> {
    let head = git(repo, &["rev-parse", "HEAD"])?.trim().to_owned();
    if head != resolved {
        return Err(format!(
            "HEAD is {head}, not the declared target commit {resolved}; \
             check out the commit being proven"
        ));
    }
    let porcelain = git(repo, &["status", "--porcelain"])?;
    let entries: Vec<&str> = porcelain
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .collect();
    if !entries.is_empty() {
        return Err(format!(
            "the working tree is not clean at {resolved}: {}",
            entries.join("; ")
        ));
    }
    Ok(())
}

/// Stage 5: no resembling tag is malformed and the well-formed numbers run
/// 001..highest with no hole. Returns the sorted well-formed numbers.
fn naming_health(repo: &Path) -> Result<Vec<u32>, String> {
    let resembling = git(repo, &["tag", "--list", "edition*", "Edition*"])?;
    for tag in resembling
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if edition_number(tag).is_none() {
            return Err(format!(
                "the existing tag {tag} is not exactly edition-NNN with three digits, \
                 so it marks nothing"
            ));
        }
    }
    let mut numbers: Vec<u32> = edition_tags(repo)?
        .iter()
        .filter_map(|tag| edition_number(tag))
        .collect();
    numbers.sort_unstable();
    if let Some(&highest) = numbers.last() {
        for expected in 1..=highest {
            if !numbers.contains(&expected) {
                return Err(format!(
                    "the sequence reaches {highest:03} with no {expected:03}, \
                     so a citation to it resolves to nothing"
                ));
            }
        }
    }
    Ok(numbers)
}

/// Stage 6 (and its stage 10 recheck): the candidate name does not exist.
fn name_absent(repo: &Path, name: &str) -> Result<(), String> {
    if git(
        repo,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/tags/{name}"),
        ],
    )
    .is_ok()
    {
        return Err(format!(
            "{name} already exists, and an edition is never moved or overwritten (EDN-003)"
        ));
    }
    Ok(())
}

/// One entry of a scanned directory in the candidate commit's tree.
struct TreeEntry {
    kind: String,
    path: String,
    leaf: String,
}

/// The direct children of `directory` in `commit`'s tree, in tree order. A
/// missing directory reads as no entries, never as an error.
fn tree_entries(repo: &Path, commit: &str, directory: &str) -> Result<Vec<TreeEntry>, String> {
    let listed = git(repo, &["ls-tree", commit, &format!("{directory}/")])?;
    let mut entries = Vec::new();
    for line in listed.lines() {
        let Some((header, path)) = line.split_once('\t') else {
            continue;
        };
        let kind = header
            .split_whitespace()
            .nth(1)
            .unwrap_or_default()
            .to_owned();
        entries.push(TreeEntry {
            leaf: path.rsplit('/').next().unwrap_or(path).to_owned(),
            path: path.to_owned(),
            kind,
        });
    }
    Ok(entries)
}

/// The first `status:` line of a record's leading front-matter block, with
/// surrounding quotes stripped; `None` when no readable status line exists.
fn status_line(text: &str) -> Option<String> {
    let mut lines = text.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    for line in lines {
        let line = line.trim();
        if line == "---" {
            return None;
        }
        if let Some(value) = line.strip_prefix("status:") {
            return Some(value.trim().trim_matches('"').trim_matches('\'').to_owned());
        }
    }
    None
}

/// The record's bytes at `path` in `commit`'s tree; empty when unreadable,
/// because a record that cannot be read is never at rest.
fn record_bytes(repo: &Path, commit: &str, path: &str) -> String {
    git(repo, &["show", &format!("{commit}:{path}")]).unwrap_or_default()
}

/// A gap record's status: the first `status:` line inside its first block
/// opened by a ` ```yaml ` line and closed by a ` ``` ` line, with surrounding
/// quotes stripped. Real residual records carry their status in that fenced
/// block, never in front matter; `None` when no such line exists.
fn fenced_status_line(text: &str) -> Option<String> {
    let mut lines = text.lines();
    if !lines.any(|line| line.trim() == "```yaml") {
        return None;
    }
    for line in lines {
        let line = line.trim();
        if line == "```" {
            return None;
        }
        if let Some(value) = line.strip_prefix("status:") {
            return Some(value.trim().trim_matches('"').trim_matches('\'').to_owned());
        }
    }
    None
}

/// Stage 8 (and its stage 10 recheck): the candidate commit's rest records -
/// intake, then work items, then gap records, first offender by name order.
fn at_rest(repo: &Path, resolved: &str) -> Result<(), String> {
    for entry in tree_entries(repo, resolved, ".arca/issue")? {
        if entry.kind == "tree" && entry.leaf != "deferred" && entry.leaf != "archive" {
            return Err(format!(
                "the cycle is not at rest: the intake work area holds {}",
                entry.leaf
            ));
        }
    }
    for entry in tree_entries(repo, resolved, ".arca/ticket")? {
        if entry.kind == "blob" && entry.leaf.ends_with(".md") {
            let record = record_bytes(repo, resolved, &entry.path);
            if status_line(&record).as_deref() != Some("passed") {
                return Err(format!(
                    "the cycle is not at rest: the work item {} is not passed",
                    entry.path
                ));
            }
        }
    }
    for entry in tree_entries(repo, resolved, ".arca/residual")? {
        if entry.kind == "blob" && entry.leaf.ends_with(".md") {
            let record = record_bytes(repo, resolved, &entry.path);
            let status = fenced_status_line(&record).unwrap_or_else(|| "unreadable".to_owned());
            if status != "satisfied" {
                return Err(format!(
                    "the cycle is not at rest: the gap record {} is {status}, not satisfied",
                    entry.path
                ));
            }
        }
    }
    Ok(())
}

/// Stage 9: run every declared check in file order, in the repository root,
/// through the harness's scrubbed child environment (inherited `GIT_*`
/// variables removed, stdin null), capturing and discarding its output.
fn run_checks(repo: &Path, shown_checks: &str, checks: &[DeclaredCheck]) -> Result<(), String> {
    for check in checks {
        let spec = check.tokens.join(" ");
        let failure = |verdict: String| {
            format!(
                "the declared check at {shown_checks} line {} ({spec}) {verdict}",
                check.line
            )
        };
        match support::command(&check.tokens[0], repo)
            .args(&check.tokens[1..])
            .output()
        {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                let verdict = format!(
                    "exited {}",
                    output
                        .status
                        .code()
                        .map(|code| code.to_string())
                        .unwrap_or_else(|| "by signal".to_owned())
                );
                return Err(failure(verdict));
            }
            Err(error) => return Err(failure(format!("could not run: {error}"))),
        }
    }
    Ok(())
}

/// Stage 10's write: one non-overwriting `git tag` carrying
/// `--cleanup=verbatim`, the validated bytes on its stdin, and the resolved
/// commit explicitly - never a moving name.
fn create_tag(repo: &Path, name: &str, resolved: &str, message: &str) -> Result<(), String> {
    let args = ["tag", "--cleanup=verbatim", "-F", "-", name, resolved];
    let mut child = support::command("git", repo)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("creating the tag failed: git {args:?} could not run: {error}"))?;
    let written = child
        .stdin
        .as_mut()
        .expect("git tag reads its message from the piped stdin")
        .write_all(message.as_bytes());
    drop(child.stdin.take());
    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(error) => {
            // git could not be waited on, so it names no error of its own;
            // the write error, if any, is the only nameable failure.
            return match written {
                Ok(()) => Err(format!(
                    "creating the tag failed: git tag did not finish: {error}"
                )),
                Err(write_error) => Err(format!(
                    "creating the tag failed: writing the message to git tag failed: {write_error}"
                )),
            };
        }
    };
    if !output.status.success() {
        return Err(format!(
            "creating the tag failed: git {args:?} exited {}: {}",
            output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "by signal".to_owned()),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    // git exited 0 having named no error: only a failed stdin write can
    // still refuse, because git never read the whole validated message.
    if let Err(write_error) = written {
        return Err(format!(
            "creating the tag failed: writing the message to git tag failed: {write_error}"
        ));
    }
    Ok(())
}

/// Stage 11: read the created tag back and compare it with the validated
/// candidate. Any disagreement refuses; the tag is never rewritten.
fn verify_created(repo: &Path, name: &str, resolved: &str, message: &str) -> Result<(), String> {
    let disagreement = |what: String| {
        format!(
            "the created tag {name} does not carry the validated candidate ({what}), \
             and an edition is never rewritten; resolve this by hand"
        )
    };
    let kind = git(repo, &["cat-file", "-t", name])?.trim().to_owned();
    if kind != "tag" {
        return Err(disagreement(format!("its object is a {kind}")));
    }
    let marked = edition_commit(repo, name)?;
    if marked != resolved {
        return Err(disagreement(format!("it marks {marked}, not {resolved}")));
    }
    let stored = support::command("git", repo)
        .args(["cat-file", "tag", name])
        .output()
        .map_err(|error| format!("reading the created tag failed: {error}"))?;
    if !stored.status.success() {
        return Err(format!(
            "reading the created tag failed: git cat-file exited {}: {}",
            stored
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "by signal".to_owned()),
            String::from_utf8_lossy(&stored.stderr).trim()
        ));
    }
    let raw = &stored.stdout;
    let stored_message = raw
        .windows(2)
        .position(|window| window == b"\n\n")
        .map(|separator| &raw[separator + 2..]);
    if stored_message != Some(message.as_bytes()) {
        return Err(disagreement(
            "its message differs from the validated bytes".to_owned(),
        ));
    }
    Ok(())
}

/// The armed after-checks pause: the marker and release paths and the
/// timeout, validated at stage 0 before any git command.
struct Seam {
    marker: OsString,
    release: OsString,
    timeout: Duration,
}

impl Seam {
    /// Unset or empty pauses nothing; any other value but `after-checks`
    /// refuses, as does a missing or invalid companion.
    fn from_env() -> Result<Option<Seam>, String> {
        let Some(boundary) = std::env::var_os("RATMAC_TEST_EDITION_CUT_PAUSE") else {
            return Ok(None);
        };
        let boundary = boundary.to_string_lossy().into_owned();
        if boundary.is_empty() {
            return Ok(None);
        }
        if boundary != "after-checks" {
            return Err(format!(
                "RATMAC_TEST_EDITION_CUT_PAUSE={boundary} names no boundary; \
                 after-checks is the only boundary"
            ));
        }
        let marker = required_path("RATMAC_TEST_EDITION_CUT_PAUSE_MARKER")?;
        let release = required_path("RATMAC_TEST_EDITION_CUT_PAUSE_RELEASE")?;
        let timeout = std::env::var_os("RATMAC_TEST_EDITION_CUT_PAUSE_TIMEOUT_MILLIS")
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default();
        let millis: u64 = timeout.parse().map_err(|_| {
            format!(
                "RATMAC_TEST_EDITION_CUT_PAUSE_TIMEOUT_MILLIS={timeout} \
                     is not a positive integer"
            )
        })?;
        if millis == 0 {
            return Err(format!(
                "RATMAC_TEST_EDITION_CUT_PAUSE_TIMEOUT_MILLIS={timeout} \
                 is not a positive integer"
            ));
        }
        Ok(Some(Seam {
            marker,
            release,
            timeout: Duration::from_millis(millis),
        }))
    }
}

/// A companion variable naming a file path, or the refusal naming it.
fn required_path(name: &str) -> Result<OsString, String> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("RATMAC_TEST_EDITION_CUT_PAUSE requires {name} naming a file path"))
}

/// Publish the marker in one step, then wait for the release until the
/// timeout expires. The one-step rule keeps a reader from ever seeing a
/// partial marker line.
fn pause_at_after_checks(seam: &Seam) -> Result<(), String> {
    publish_marker(&seam.marker)?;
    let release = Path::new(&seam.release);
    let deadline = Instant::now() + seam.timeout;
    while !release.exists() {
        if Instant::now() >= deadline {
            return Err("the injected pause at after-checks expired".to_owned());
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

/// Write `after-checks` and a newline to the marker path through a temporary
/// sibling and one rename, so the marker appears whole or not at all.
fn publish_marker(marker: &OsString) -> Result<(), String> {
    let shown = marker.to_string_lossy();
    let mut temporary = marker.clone();
    temporary.push(format!(".{}.tmp", std::process::id()));
    let published = std::fs::write(&temporary, b"after-checks\n")
        .and_then(|()| std::fs::rename(&temporary, marker));
    published.map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        format!("cannot write the pause marker {shown}: {error}")
    })
}
