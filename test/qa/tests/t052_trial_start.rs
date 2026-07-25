//! t-050 / TWL-001..TWL-003, TWL-006: trial identity, atomic start, dry-run.
//!
//! PT-050-01 `clean_start_creates_branch_and_worktree`
//! PT-050-02 `dirty_or_colliding_start_refused`
//! PT-050-03 `numbering_is_deterministic`
//! PT-050-04 `start_rolls_back_completely`
//! PT-050-05 `status_is_read_only_preview`
//! HT-050-01 `malformed_slugs_refuse_before_any_write`
//! HT-050-02 `rollback_removes_only_the_new_branch`
//! HT-050-03 `start_from_inside_a_trial_worktree_refuses`
//!
//! A trial opens from a clean experiment base or not at all. Every fixture
//! here is a throwaway repository, and every negative case is proven against a
//! byte-identical snapshot of refs, tags, index, working tree, worktree
//! registrations, and sibling directories.

use std::fs;
use std::path::Path;
use std::process::Command;

use ratmac_qa::trial::{script_source, Trial, BASE};

/// PT-050-01: a clean start creates exactly the branch and its worktree.
#[test]
fn clean_start_creates_branch_and_worktree() {
    let trial = Trial::new("clean");
    let base_tip = trial.head_of(BASE);

    let output = trial.trial(&["start", "-Slug", "parser"]);
    assert!(
        output.status.success(),
        "a clean start succeeds: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = String::from_utf8_lossy(&output.stdout);

    let branch = "trial-001-parser";
    let worktree = trial.sibling("repo-trial-001-parser");
    assert!(
        report.contains(branch),
        "the report names the branch: {report}"
    );
    assert!(
        report.contains("repo-trial-001-parser"),
        "the report names the worktree path: {report}"
    );

    assert_eq!(
        trial.head_of(branch),
        base_tip,
        "the trial branch starts at exactly the base tip"
    );
    assert!(worktree.is_dir(), "the linked worktree exists on disk");
    let registrations = trial.git_text(&["worktree", "list", "--porcelain"]);
    assert!(
        registrations.contains(&format!("branch refs/heads/{branch}")),
        "the worktree is registered: {registrations}"
    );
    assert_eq!(
        trial.git_text(&["status", "--porcelain"]),
        "",
        "the base checkout stays clean"
    );
    assert_eq!(
        trial.git_text(&["tag", "--list"]),
        "",
        "start creates no tag"
    );
}

/// PT-050-02: every dirty or colliding precondition refuses, mutating nothing.
#[test]
fn dirty_or_colliding_start_refused() {
    let trial = Trial::new("refusals");

    struct Case {
        name: &'static str,
        args: Vec<&'static str>,
        reason: &'static str,
        setup: fn(&Trial),
        teardown: fn(&Trial),
    }

    let cases = [
        Case {
            name: "staged change",
            args: vec!["start", "-Slug", "staged"],
            reason: "clean",
            setup: |trial| {
                fs::write(trial.root.join("staged.txt"), "x\n").expect("write");
                trial.git(&["add", "staged.txt"]);
            },
            teardown: |trial| {
                trial.git(&["reset", "--quiet"]);
                fs::remove_file(trial.root.join("staged.txt")).expect("remove");
            },
        },
        Case {
            name: "unstaged change",
            args: vec!["start", "-Slug", "unstaged"],
            reason: "clean",
            setup: |trial| {
                fs::write(trial.root.join("README.md"), "# dirty\n").expect("write");
            },
            teardown: |trial| {
                fs::write(trial.root.join("README.md"), "# fixture\n").expect("restore");
            },
        },
        Case {
            name: "untracked file",
            args: vec!["start", "-Slug", "untracked"],
            reason: "clean",
            setup: |trial| {
                fs::write(trial.root.join("stray.txt"), "x\n").expect("write");
            },
            teardown: |trial| {
                fs::remove_file(trial.root.join("stray.txt")).expect("remove");
            },
        },
        Case {
            name: "not on the base branch",
            args: vec!["start", "-Slug", "elsewhere"],
            reason: "base",
            setup: |trial| {
                trial.git(&["checkout", "main"]);
            },
            teardown: |trial| {
                trial.git(&["checkout", BASE]);
            },
        },
        Case {
            name: "duplicate branch",
            args: vec!["start", "-Slug", "dup", "-Number", "1"],
            reason: "trial-001-taken",
            setup: |trial| {
                trial.git(&["branch", "trial-001-taken"]);
            },
            teardown: |trial| {
                trial.git(&["branch", "-D", "trial-001-taken"]);
            },
        },
        Case {
            name: "occupied sibling directory",
            args: vec!["start", "-Slug", "occupied", "-Number", "1"],
            reason: "repo-trial-001-occupied",
            setup: |trial| {
                fs::create_dir_all(trial.sibling("repo-trial-001-occupied")).expect("create");
            },
            teardown: |trial| {
                fs::remove_dir_all(trial.sibling("repo-trial-001-occupied")).expect("remove");
            },
        },
        Case {
            name: "archive tag collision",
            args: vec!["start", "-Slug", "tagged", "-Number", "1"],
            reason: "trial-archive/trial-001-tagged",
            setup: |trial| {
                trial.git(&[
                    "tag",
                    "-a",
                    "trial-archive/trial-001-tagged",
                    "-m",
                    "archived",
                ]);
            },
            teardown: |trial| {
                trial.git(&["tag", "-d", "trial-archive/trial-001-tagged"]);
            },
        },
        Case {
            name: "durable log destination collision",
            args: vec!["start", "-Slug", "durable", "-Number", "1"],
            reason: "trials/trial-001-durable",
            setup: |trial| {
                let durable = trial.root.join("trials/trial-001-durable");
                fs::create_dir_all(&durable).expect("create");
                fs::write(durable.join("trial-log.md"), "# log\n").expect("write");
                trial.git(&["add", "-A"]);
                trial.git(&["commit", "-m", "durable log"]);
            },
            teardown: |trial| {
                trial.git(&["reset", "--hard", "HEAD~1"]);
            },
        },
    ];

    for case in cases {
        (case.setup)(&trial);
        let before = trial.snapshot();
        let output = trial.trial(&case.args);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.status.success(),
            "{}: start must refuse. Output: {text}",
            case.name
        );
        assert!(
            text.contains(case.reason),
            "{}: the refusal names {:?}: {text}",
            case.name,
            case.reason
        );
        assert_eq!(
            trial.snapshot(),
            before,
            "{}: the refusal mutates nothing",
            case.name
        );
        (case.teardown)(&trial);
    }
}

/// PT-050-02 (base is not negotiable): the interface offers no base override.
#[test]
fn base_cannot_be_overridden() {
    let trial = Trial::new("fixed-base");
    trial.git(&["checkout", "main"]);
    let before = trial.snapshot();

    for args in [
        vec!["start", "-Slug", "elsewhere", "-Base", "main"],
        vec!["status", "-Base", "main"],
    ] {
        let output = trial.trial(&args);
        assert!(
            !output.status.success(),
            "the interface rejects a base override: {args:?}"
        );
        assert_eq!(
            trial.snapshot(),
            before,
            "the rejected override mutates nothing: {args:?}"
        );
    }
    assert!(
        !trial
            .git_in(
                &trial.root,
                &["rev-parse", "--verify", "trial-001-elsewhere"]
            )
            .status
            .success(),
        "no trial branch was created from main"
    );
}

/// PT-050-03: numbering counts live branches, archive tags, and durable logs,
/// each observable on its own.
#[test]
fn numbering_is_deterministic() {
    let trial = Trial::new("numbering");
    // Three independent sources, each the only evidence of its trial: a live
    // branch, an archive tag, and a durable log directory.
    trial.git(&["branch", "trial-001-live"]);
    let durable = trial.root.join("trials/trial-003-logged");
    fs::create_dir_all(&durable).expect("create durable directory");
    fs::write(durable.join("trial-log.md"), "# log\n").expect("write durable log");
    trial.git(&["add", "-A"]);
    trial.git(&["commit", "-m", "durable log"]);
    trial.git(&[
        "tag",
        "-a",
        "trial-archive/trial-004-tagged",
        "-m",
        "archived",
    ]);

    let preview = trial.text(&["status", "-Slug", "next"]);
    assert!(
        preview.contains("trial-005-next"),
        "the highest number across all three sources decides: {preview}"
    );

    for (number, collision) in [
        ("1", "trial-001-live"),
        ("3", "trials/trial-003-logged"),
        ("4", "trial-archive/trial-004-tagged"),
    ] {
        let before = trial.snapshot();
        let output = trial.trial(&["start", "-Slug", "clash", "-Number", number]);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.status.success(),
            "an explicit occupied number refuses: {number}"
        );
        assert!(
            text.contains(collision),
            "the refusal names the collision {collision}: {text}"
        );
        assert_eq!(trial.snapshot(), before, "the refusal mutates nothing");
    }

    // A durable log alone, higher than every other source, still decides.
    let higher = trial.root.join("trials/trial-006-logged");
    fs::create_dir_all(&higher).expect("create durable directory");
    fs::write(higher.join("trial-log.md"), "# log\n").expect("write durable log");
    trial.git(&["add", "-A"]);
    trial.git(&["commit", "-m", "higher durable log"]);
    let preview = trial.text(&["status", "-Slug", "next"]);
    assert!(
        preview.contains("trial-007-next"),
        "a durable log with the highest number decides on its own: {preview}"
    );

    let output = trial.trial(&["start", "-Slug", "next"]);
    assert!(
        output.status.success(),
        "the inferred number starts: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        trial.head_of("trial-007-next"),
        trial.head_of(BASE),
        "the inferred trial starts at the base tip"
    );
}

/// PT-050-04: a mid-creation failure leaves nothing behind.
#[test]
fn start_rolls_back_completely() {
    let trial = Trial::new("rollback");
    let before = trial.snapshot();

    // `git worktree add -b` creates the branch ref before it prepares the
    // worktree administration directory: blocking that directory fails the
    // command with the new ref already in place.
    let administration = trial.root.join(".git/worktrees");
    fs::write(&administration, b"blocked\n").expect("block worktree administration");

    let output = trial.trial(&["start", "-Slug", "doomed"]);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.status.success(), "the failed start refuses: {text}");
    assert!(
        text.contains("worktree"),
        "the refusal names the failure: {text}"
    );

    fs::remove_file(&administration).expect("clear the injected failure");
    assert_eq!(
        trial.snapshot(),
        before,
        "no branch ref, tag, registration, or sibling directory persists"
    );
    assert!(
        !trial.sibling("repo-trial-001-doomed").exists(),
        "no sibling directory persists"
    );
}

/// PT-050-05: status previews without touching anything.
#[test]
fn status_is_read_only_preview() {
    let trial = Trial::new("status");
    trial.git(&["branch", "trial-001-live"]);
    trial.git(&["tag", "-a", "trial-archive/trial-000-old", "-m", "archived"]);
    let before = trial.snapshot();

    let output = trial.trial(&["status", "-Slug", "preview"]);
    assert!(
        output.status.success(),
        "status succeeds: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = String::from_utf8_lossy(&output.stdout).into_owned();

    for expected in [
        BASE,
        &trial.head_of(BASE)[..12],
        "clean",
        "trial-001-live",
        "trial-archive/trial-000-old",
        "trial-002-preview",
        "repo-trial-002-preview",
        "trial-archive/trial-002-preview",
        "trials/trial-002-preview/trial-log.md",
        "git worktree add",
        "git worktree remove",
        "git merge main",
        "git update-ref -d refs/heads/trial-002-preview",
    ] {
        assert!(
            report.contains(expected),
            "status reports {expected:?}: {report}"
        );
    }
    assert!(
        report.to_lowercase().contains("start")
            && report.to_lowercase().contains("finish")
            && report.to_lowercase().contains("sync"),
        "status previews every mutating verb: {report}"
    );

    assert_eq!(
        trial.snapshot(),
        before,
        "status mutates nothing in the fixture"
    );
}

/// PT-050-05 (real smoke): status is read-only in this checkout too.
#[test]
fn status_smoke_in_this_checkout() {
    let repo = script_source()
        .parent()
        .and_then(Path::parent)
        .expect("repository root")
        .to_path_buf();
    let snapshot = |label: &str| -> String {
        let read = |args: &[&str]| -> String {
            let output = Command::new("git")
                .args(args)
                .current_dir(&repo)
                .output()
                .unwrap_or_else(|error| panic!("{label}: git {args:?}: {error}"));
            String::from_utf8_lossy(&output.stdout).into_owned()
        };
        format!(
            "{}{}{}{}",
            read(&["show-ref"]),
            read(&["tag", "--list"]),
            read(&["worktree", "list", "--porcelain"]),
            read(&["status", "--porcelain"])
        )
    };

    let before = snapshot("before");
    let output = Command::new("pwsh")
        .args(["-NoProfile", "-File", "tools/trial.ps1", "status"])
        .current_dir(&repo)
        .output()
        .expect("invoke pwsh");
    assert!(
        output.status.success(),
        "status succeeds in this checkout: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = String::from_utf8_lossy(&output.stdout);
    assert!(
        report.contains(BASE),
        "the smoke reports the experiment base: {report}"
    );
    assert_eq!(snapshot("after"), before, "the smoke mutates nothing");
}

/// HT-050-01: malformed slugs, numbers, and verbs refuse before any Git write.
#[test]
fn malformed_slugs_refuse_before_any_write() {
    let trial = Trial::new("slugs");
    let before = trial.snapshot();

    for slug in [
        "Parser",
        "two words",
        "héllo",
        "-lead",
        "trail-",
        "double--dash",
        "under_score",
        "",
    ] {
        let output = trial.trial(&["start", "-Slug", slug]);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.status.success(),
            "the malformed slug {slug:?} refuses: {text}"
        );
        assert!(
            text.contains("slug"),
            "the refusal names the slug rule for {slug:?}: {text}"
        );
        assert_eq!(
            trial.snapshot(),
            before,
            "the malformed slug {slug:?} writes nothing"
        );
    }

    for number in ["0", "-1"] {
        for verb in ["start", "status"] {
            let output = trial.trial(&[verb, "-Slug", "topic", "-Number", number]);
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                !output.status.success(),
                "{verb} refuses the malformed number {number}: {text}"
            );
            assert!(
                text.contains("number"),
                "the refusal names the number rule: {text}"
            );
            assert_eq!(trial.snapshot(), before, "the refusal writes nothing");
        }
    }

    for verb in ["banana", "resume", "archive"] {
        let output = trial.trial(&[verb]);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.status.success(),
            "the unknown verb {verb} refuses: {text}"
        );
        assert!(
            text.contains("status, start, finish, sync"),
            "the refusal names the verbs that exist: {text}"
        );
        assert_eq!(trial.snapshot(), before, "the refusal writes nothing");
    }
}

/// HT-050-02: rollback removes the new ref and nothing else, or says exactly
/// how to finish by hand.
#[test]
fn rollback_removes_only_the_new_branch() {
    let trial = Trial::new("rollback-scope");
    trial.git(&["branch", "trial-001-keep"]);
    let kept = trial.head_of("trial-001-keep");
    let before = trial.snapshot();

    let administration = trial.root.join(".git/worktrees");
    fs::write(&administration, b"blocked\n").expect("block worktree administration");
    let output = trial.trial(&["start", "-Slug", "victim"]);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.status.success(), "the failed start refuses");
    fs::remove_file(&administration).expect("clear the injected failure");

    assert_eq!(
        trial.head_of("trial-001-keep"),
        kept,
        "the unrelated live trial branch survives rollback"
    );
    assert!(
        trial
            .git_in(&trial.root, &["rev-parse", "--verify", "trial-002-victim"])
            .status
            .code()
            != Some(0),
        "the branch created by the failed start is gone: {text}"
    );
    assert_eq!(trial.snapshot(), before, "the tree is snapshot-identical");
}

/// HT-050-02 (compare-and-delete): a ref that moved under a failed start is
/// left alone, named, and handed back with recovery commands.
#[test]
fn rollback_never_deletes_a_moved_ref() {
    let trial = Trial::new("moved-ref");
    // A second commit gives the hook somewhere else to point the new ref.
    fs::write(trial.root.join("second.txt"), "second\n").expect("write fixture file");
    trial.git(&["add", "-A"]);
    trial.git(&["commit", "-m", "second"]);
    let elsewhere = trial.git_text(&["rev-parse", "HEAD~1"]).trim().to_owned();
    let base_tip = trial.head_of(BASE);

    // A reference-transaction hook moves the branch the instant start creates
    // it, so rollback meets a ref that no longer holds what it recorded.
    let hooks = trial.root.join(".git/hooks");
    fs::create_dir_all(&hooks).expect("create hooks directory");
    fs::write(
        hooks.join("reference-transaction"),
        format!(
            "#!/bin/sh\n             [ \"$1\" = committed ] || exit 0\n             [ -f .git/hook-fired ] && exit 0\n             while read -r old new ref; do\n             case \"$ref\" in refs/heads/trial-*)\n             : > .git/hook-fired; git update-ref \"$ref\" {elsewhere};;\n             esac; done\nexit 0\n"
        ),
    )
    .expect("install the hook");

    let administration = trial.root.join(".git/worktrees");
    fs::write(&administration, b"blocked\n").expect("block worktree administration");
    let output = trial.trial(&["start", "-Slug", "moved"]);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_file(&administration).expect("clear the injected failure");
    fs::remove_file(hooks.join("reference-transaction")).expect("remove the hook");

    assert!(!output.status.success(), "the failed start refuses: {text}");
    assert_eq!(
        trial
            .git_text(&["rev-parse", "refs/heads/trial-001-moved"])
            .trim(),
        elsewhere,
        "the moved ref is left exactly where it was found: {text}"
    );
    assert_ne!(elsewhere, base_tip, "the hook really moved the ref");
    assert!(
        text.contains("moved to") && text.contains(&elsewhere),
        "the refusal names the moved ref and where it points: {text}"
    );
    assert!(
        text.contains("rollback could not finish"),
        "the refusal says rollback stopped short: {text}"
    );
}

/// HT-050-03: start refuses from inside a trial worktree, with a `cd` hint.
#[test]
fn start_from_inside_a_trial_worktree_refuses() {
    let trial = Trial::new("self-lock");
    assert!(
        trial.trial(&["start", "-Slug", "inside"]).status.success(),
        "the first trial opens"
    );
    let worktree = trial.sibling("repo-trial-001-inside");
    let before = trial.snapshot();

    let output = trial.trial_in(&worktree, &["start", "-Slug", "nested"]);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "start refuses inside a trial worktree: {text}"
    );
    assert!(
        text.contains("cd "),
        "the refusal hints how to leave the worktree: {text}"
    );
    assert_eq!(trial.snapshot(), before, "the refusal mutates nothing");
}
