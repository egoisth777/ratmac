//! t-052 / TWL-007, TWL-009, TWL-010: merge-only sync, closed interface, ownership.
//!
//! PT-052-01 `sync_is_merge_only`
//! PT-052-02 `interface_is_closed_and_offline`
//! PT-052-03 `ownership_and_windows_rule_enforced`
//! HT-052-01 `conflicted_sync_stays_visible`
//! HT-052-02 `no_forbidden_operation_reaches_the_script`
//! HT-052-03 `sync_leaves_live_trials_alone`
//!
//! Fixes flow main-first and reach the experiment base only through an
//! explicit merge. The script may do exactly four things, offline, and the
//! guidance says who is allowed to do them.

use std::fs;
use std::path::Path;

use ratmac_qa::trial::{script_source, Trial, BASE};

/// Commit a change on `main` and return to the experiment base.
fn fix_on_main(trial: &Trial, file: &str, body: &str) -> String {
    trial.git(&["checkout", "main"]);
    fs::write(trial.root.join(file), body).expect("write the fix");
    trial.git(&["add", "-A"]);
    trial.git(&["commit", "-m", "main: fix"]);
    let head = trial.head_of("main");
    trial.git(&["checkout", BASE]);
    head
}

fn source_text() -> String {
    fs::read_to_string(script_source()).expect("read the script under test")
}

/// PT-052-01: main-first fixes reach the base only by merging.
#[test]
fn sync_is_merge_only() {
    let trial = Trial::new("sync-merge");
    let fix = fix_on_main(&trial, "fix.txt", "fixed\n");
    let base_before = trial.head_of(BASE);

    let output = trial.trial(&["sync"]);
    let report = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "a clean base syncs: {}{report}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        trial.root.join("fix.txt").is_file(),
        "the fix reaches the base checkout"
    );
    assert!(
        trial
            .git_text(&["merge-base", "--is-ancestor", &fix, BASE])
            .is_empty(),
        "the base history now contains the main commit"
    );
    assert_eq!(trial.head_of("main"), fix, "sync never moves main");
    assert_ne!(
        trial.head_of(BASE),
        base_before,
        "the base advanced through the merge"
    );

    // A dirty base refuses, and the refusal costs nothing.
    fs::write(trial.root.join("scratch.txt"), "uncommitted\n").expect("dirty the base");
    let before = trial.snapshot();
    let output = trial.trial(&["sync"]);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.status.success(), "a dirty base refuses: {text}");
    assert!(
        text.contains("clean"),
        "the refusal names cleanliness: {text}"
    );
    assert_eq!(trial.snapshot(), before, "the refusal mutates nothing");

    // Sync runs from the base, not from main.
    fs::remove_file(trial.root.join("scratch.txt")).expect("clean the base");
    trial.git(&["checkout", "main"]);
    let before = trial.snapshot();
    let output = trial.trial(&["sync"]);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.status.success(), "sync from main refuses: {text}");
    assert!(
        text.contains(BASE),
        "the refusal names the base to check out: {text}"
    );
    assert_eq!(trial.snapshot(), before, "the refusal mutates nothing");
}

/// HT-052-01: a conflicted merge is left exactly as Git left it.
#[test]
fn conflicted_sync_stays_visible() {
    let trial = Trial::new("sync-conflict");
    fix_on_main(&trial, "shared.txt", "from main\n");
    fs::write(trial.root.join("shared.txt"), "from the base\n").expect("write the base side");
    trial.git(&["add", "-A"]);
    trial.git(&["commit", "-m", "base: conflicting change"]);
    let base_before = trial.head_of(BASE);

    let output = trial.trial(&["sync"]);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "a conflicted merge exits non-zero"
    );
    assert!(
        text.contains("shared.txt"),
        "the refusal lists the conflicted file: {text}"
    );

    let content = fs::read_to_string(trial.root.join("shared.txt")).expect("read the conflict");
    assert!(
        content.contains("<<<<<<<") && content.contains(">>>>>>>"),
        "the conflict markers are visible: {content}"
    );
    assert!(
        trial.root.join(".git/MERGE_HEAD").is_file(),
        "the merge is still in progress - nothing was aborted"
    );
    assert_eq!(
        trial.head_of(BASE),
        base_before,
        "no commit was made behind the human's back"
    );
    assert!(
        trial
            .git_text(&["diff", "--name-only", "--diff-filter=U"])
            .contains("shared.txt"),
        "git still reports the unmerged path"
    );
}

/// PT-052-02: exactly four verbs, and nothing that reaches past this repository.
#[test]
fn interface_is_closed_and_offline() {
    let source = source_text();

    let verbs: Vec<String> = source
        .lines()
        .skip_while(|line| !line.trim_start().starts_with("switch ($Verb)"))
        .take_while(|line| !line.trim().starts_with("default"))
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix('\'')
                .and_then(|rest| rest.split('\'').next())
                .map(str::to_owned)
        })
        .collect();
    assert_eq!(
        verbs,
        vec!["status", "start", "finish", "sync"],
        "the dispatch offers exactly the four documented verbs"
    );

    // Every git subcommand the script can run, taken from its call sites.
    let mut subcommands: Vec<String> = Vec::new();
    let mut flags: Vec<String> = Vec::new();
    for (index, _) in source.match_indices("Invoke-Git -Arguments @(") {
        let rest = &source[index..];
        let open = rest.find('(').expect("argument list opens");
        let close = rest[open..].find(')').expect("argument list closes") + open;
        let arguments: Vec<String> = rest[open + 1..close]
            .split(',')
            .map(|argument| argument.trim().trim_matches('\'').to_owned())
            .collect();
        for argument in &arguments {
            if argument.starts_with('-') && !argument.starts_with("-C") {
                flags.push(argument.clone());
            }
        }
        let first = arguments
            .iter()
            .find(|argument| !argument.starts_with('-') && !argument.starts_with('$'))
            .cloned()
            .unwrap_or_default();
        let subcommand = if first == "-C" || arguments.first().map(String::as_str) == Some("-C") {
            arguments
                .get(2)
                .cloned()
                .unwrap_or_default()
                .trim_matches('\'')
                .to_owned()
        } else {
            first
        };
        if !subcommand.is_empty() {
            subcommands.push(subcommand);
        }
    }
    subcommands.sort();
    subcommands.dedup();

    let allowed = [
        "add",
        "branch",
        "cat-file",
        "commit",
        "diff",
        "diff-tree",
        "for-each-ref",
        "log",
        "merge",
        "merge-base",
        "rev-parse",
        "show",
        "status",
        "tag",
        "update-ref",
        "worktree",
    ];
    for subcommand in &subcommands {
        assert!(
            allowed.contains(&subcommand.as_str()),
            "git {subcommand} is not in the audited command set: {subcommands:?}"
        );
    }
    assert!(
        !subcommands.is_empty(),
        "the audit really found the call sites"
    );

    for flag in &flags {
        assert!(
            !flag.eq_ignore_ascii_case("--force") && !flag.eq_ignore_ascii_case("-f"),
            "no git call forces anything: {flags:?}"
        );
    }
}

/// HT-052-02: no forbidden operation hides anywhere in the script.
#[test]
fn no_forbidden_operation_reaches_the_script() {
    let source = source_text().to_ascii_lowercase();
    let forbidden = [
        "git push",
        "'push'",
        "git fetch",
        "'fetch'",
        "'pull'",
        "'clone'",
        "'reset'",
        "'rebase'",
        "'--abort'",
        "@('gc'",
        "'prune'",
        "@('clean'",
        "'clean',",
        "--global",
        "invoke-webrequest",
        "invoke-restmethod",
        "start-bitstransfer",
        "install-module",
        "install-package",
        "winget",
        "choco",
        "npm ",
        "pip ",
        "stop-process",
        "taskkill",
        "get-process",
        "remove-item",
        "$env:path =",
        "setx",
    ];
    for needle in forbidden {
        assert!(
            !source.contains(needle),
            "the script must never carry {needle:?}"
        );
    }
}

/// HT-052-03: syncing the base leaves live trials exactly where they are.
#[test]
fn sync_leaves_live_trials_alone() {
    let trial = Trial::new("sync-trials");
    assert!(
        trial.trial(&["start", "-Slug", "parser"]).status.success(),
        "a live trial exists"
    );
    let branch_before = trial.head_of("trial-001-parser");
    let worktree = trial.sibling("repo-trial-001-parser");
    fs::write(worktree.join("work.txt"), "in progress\n").expect("leave work in the trial");
    fix_on_main(&trial, "fix.txt", "fixed\n");

    let output = trial.trial(&["sync"]);
    assert!(
        output.status.success(),
        "the base syncs with a live trial open: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        trial.head_of("trial-001-parser"),
        branch_before,
        "the trial branch is untouched"
    );
    assert!(
        worktree.join("work.txt").is_file(),
        "uncommitted trial work survives"
    );
    assert!(
        trial
            .git_text(&["worktree", "list", "--porcelain"])
            .contains("repo-trial-001-parser"),
        "the trial worktree stays registered"
    );
    assert!(
        !worktree.join("fix.txt").exists(),
        "the merge does not leak into the trial worktree"
    );
}

/// PT-052-03: the guidance states who may do this, and finish enforces it.
#[test]
fn ownership_and_windows_rule_enforced() {
    let index =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.arca/index.md"))
            .expect("read the contributor guidance");
    let section = index
        .split("## ")
        .find(|block| block.starts_with("Trial worktrees"))
        .expect("the guidance carries a trial-worktree section");

    for needle in [
        "tools/trial.ps1",
        "start",
        "status",
        "finish",
        "sync",
        "Advisor",
        "Subagent",
        "Main-Agent",
        "primary checkout",
    ] {
        assert!(
            section.contains(needle),
            "the guidance states {needle:?}: {section}"
        );
    }
    assert!(
        section.contains("never")
            && (section.contains("working directory") || section.contains("cd ")),
        "the guidance states the Windows working-directory rule: {section}"
    );

    // The rule is not only written down: finish refuses from inside the trial.
    let trial = Trial::new("ownership");
    assert!(
        trial.trial(&["start", "-Slug", "parser"]).status.success(),
        "a trial exists to stand inside"
    );
    let worktree = trial.sibling("repo-trial-001-parser");
    let before = trial.snapshot();
    let output = trial.trial_in(&worktree, &["finish"]);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.status.success(), "finish from inside refuses");
    assert!(
        text.contains("cd "),
        "the refusal says where to stand instead: {text}"
    );
    assert_eq!(trial.snapshot(), before, "the refusal mutates nothing");
}
