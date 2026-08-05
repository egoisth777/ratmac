//! t-051 / TWL-004, TWL-005, TWL-008: validated log, ordered finish, containment.
//!
//! PT-051-01 `log_validation_is_strict`
//! PT-051-02 `finish_order_and_recovery`
//! PT-051-03 `finish_refusals_are_safe`
//! PT-051-04 `containment_holds`
//! HT-051-01 `interrupted_finish_resumes`
//! HT-051-02 `locked_worktree_refuses_without_force`
//! HT-051-03 `archive_tag_restores_the_branch`
//!
//! A trial ends reversibly or not at all: the annotated tag preserves the
//! terminal commit before anything is deleted, the durable log is the only
//! file the experiment base gains, and every refusal stops its own step and
//! every later one.

use std::fs;
use std::path::Path;
use std::process::{Child, Command};

use ratmac_qa::trial::{Trial, BASE};

const SLUG: &str = "parser";
const BRANCH: &str = "trial-001-parser";
const WORKTREE: &str = "repo-trial-001-parser";
const DURABLE: &str = "trials/trial-001-parser/trial-log.md";
const TAG: &str = "trial-archive/trial-001-parser";

/// A trial with one implementation commit and a valid committed log.
struct Ready {
    trial: Trial,
    worktree: std::path::PathBuf,
    terminal: String,
    base_before: String,
}

impl Ready {
    fn new(label: &str) -> Self {
        let trial = Trial::new(label);
        let base_before = trial.head_of(BASE);
        let output = trial.trial(&["start", "-Slug", SLUG]);
        assert!(
            output.status.success(),
            "the fixture trial starts: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let worktree = trial.sibling(WORKTREE);

        // Implementation content that must never reach the base.
        fs::write(worktree.join("experiment.rs"), "fn experiment() {}\n").expect("write work");
        commit(&trial, &worktree, "trial: implementation");
        let terminal_before_log = head_in(&trial, &worktree);

        let ready = Ready {
            trial,
            worktree,
            terminal: terminal_before_log,
            base_before,
        };
        ready.commit_log(&ready.valid_log());
        ready
    }

    /// The log the Advisor is expected to author, filled from real facts.
    fn valid_log(&self) -> String {
        log_text(BRANCH, &self.fork_point(), &self.terminal, None)
    }

    fn fork_point(&self) -> String {
        self.trial
            .git_text(&["merge-base", BASE, BRANCH])
            .trim()
            .to_owned()
    }

    fn commit_log(&self, body: &str) {
        fs::write(self.worktree.join("trial-log.md"), body).expect("write log");
        commit(&self.trial, &self.worktree, "trial: log");
    }

    fn finish(&self) -> std::process::Output {
        self.trial.trial(&["finish"])
    }

    fn finish_text(&self) -> String {
        let output = self.finish();
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

fn commit(trial: &Trial, directory: &Path, message: &str) {
    for args in [vec!["add", "-A"], vec!["commit", "-m", message]] {
        let output = trial.git_in(directory, &args);
        assert!(
            output.status.success(),
            "git {args:?} in the trial worktree: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn head_in(trial: &Trial, directory: &Path) -> String {
    String::from_utf8_lossy(&trial.git_in(directory, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_owned()
}

/// The template shape: eight required sections, identity facts first.
fn log_text(branch: &str, base: &str, terminal: &str, drop_section: Option<&str>) -> String {
    let sections = [
        (
            "Identity",
            format!("- trial: {branch}\n- base commit: {base}\n- terminal commit: {terminal}"),
        ),
        ("Hypothesis", "A deterministic parser is faster.".to_owned()),
        ("Procedure", "Rewrote the parser, measured both.".to_owned()),
        ("Commands and tests", "`cargo test --workspace`".to_owned()),
        ("Observations", "The rewrite is 3x faster.".to_owned()),
        ("Verdict", "adopt: fold the rewrite into main".to_owned()),
        ("Recommendations", "Port the parser first.".to_owned()),
        ("Artifacts and diffs", "experiment.rs".to_owned()),
    ];
    let mut text = format!("# Trial log: {branch}\n");
    for (heading, body) in sections {
        if drop_section == Some(heading) {
            continue;
        }
        text.push_str(&format!("\n## {heading}\n\n{body}\n"));
    }
    text
}

/// PT-051-01: validation is mechanical, and it runs before any tag or deletion.
#[test]
fn log_validation_is_strict() {
    let ready = Ready::new("log-valid");

    let preview = ready.trial.text(&["status"]);
    assert!(
        preview.contains("log: valid"),
        "status reports the complete log as valid: {preview}"
    );

    let fork = ready.fork_point();
    let defects: Vec<(&str, String, &str)> = vec![
        (
            "missing section",
            log_text(BRANCH, &fork, &ready.terminal, Some("Verdict")),
            "Verdict",
        ),
        (
            "empty section",
            log_text(BRANCH, &fork, &ready.terminal, None).replace("The rewrite is 3x faster.", ""),
            "Observations",
        ),
        (
            "wrong branch",
            log_text("trial-009-other", &fork, &ready.terminal, None),
            "trial-009-other",
        ),
        (
            "wrong base commit",
            log_text(BRANCH, &"0".repeat(40), &ready.terminal, None),
            "base commit",
        ),
        (
            "terminal commit outside the trial",
            log_text(BRANCH, &fork, &ready.base_before, None),
            "terminal commit",
        ),
    ];

    for (label, body, named) in defects {
        ready.commit_log(&body);
        let before = ready.trial.snapshot();
        let text = ready.finish_text();
        assert!(
            text.contains("log"),
            "the {label} refusal names the log: {text}"
        );
        assert!(
            text.contains(named),
            "the {label} refusal names the defect {named}: {text}"
        );
        assert_eq!(
            ready.trial.git_text(&["tag", "--list"]),
            "",
            "no archive tag is created for the {label}"
        );
        assert_eq!(
            ready.trial.snapshot(),
            before,
            "the {label} refusal mutates nothing"
        );
    }
}

/// PT-051-01 (template consistency): the shipped template is the shape the
/// validator demands, and an unfilled copy of it is refused.
#[test]
fn template_matches_the_validator() {
    let ready = Ready::new("template");
    let template = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.arca/tpl/trial-log.md"),
    )
    .expect("the trial log template ships with the project");

    // Committed as-is, the template is refused: it is all placeholders.
    ready.commit_log(&template);
    let before = ready.trial.snapshot();
    let text = ready.finish_text();
    assert!(
        text.contains("placeholder"),
        "an unfilled template is refused as unfilled: {text}"
    );
    assert_eq!(
        ready.trial.snapshot(),
        before,
        "the refusal mutates nothing"
    );

    // Filled in with real facts - and nothing else changed - it passes.
    let filled = template
        .replace("<trial-branch>", BRANCH)
        .replace(
            "<the commit the trial branched from>",
            &ready.fork_point(),
        )
        .replace("<the last commit of trial work>", &ready.terminal)
        .replace(
            "<what this trial expected to be true, in one or two sentences>",
            "the rewrite is faster",
        )
        .replace("<what was actually done, in order>", "rewrote, measured")
        .replace(
            "<the commands run and the tests that judged them>",
            "cargo test --workspace",
        )
        .replace(
            "<what happened to the feature - numbers, failures, surprises>",
            "3x faster",
        )
        .replace(
            "<what the workflow proved about RatMac - trustworthy gates, false claims, manual catches, and missing evidence>",
            "the receipt gate rejected one false completion claim",
        )
        .replace(
            "<one feature-verdict line starting with adopt:, drop:, or inconclusive: - this line goes into the archive tag>",
            "adopt: fold the rewrite into main",
        )
        .replace(
            "<drop it, or describe the behavior that may re-enter normal P1-P5 development without copying trial bytes>",
            "port the parser through normal development",
        )
        .replace(
            "<one actionable process recommendation per RatMac observation; these route to main before feature work>",
            "bind completion to fresh receipts",
        )
        .replace(
            "<paths, commits, and diffs a reader needs to reconstruct the trial>",
            "experiment.rs",
        );
    assert!(
        !filled.contains('<'),
        "every placeholder in the template has a named replacement here"
    );
    ready.commit_log(&filled);
    let tip = ready.trial.head_of(BRANCH);
    let fork = ready.fork_point();
    let preview = ready.trial.text(&["status"]);
    assert!(
        preview.contains("log: valid"),
        "the filled template validates: {preview}"
    );

    let output = ready.finish();
    assert!(
        output.status.success(),
        "a trial logged on the template finishes: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tag_message = ready.trial.git_text(&["tag", "-n99", "--list", TAG]);
    assert!(
        tag_message.contains("adopt: fold the rewrite into main"),
        "the verdict line reaches the archive tag: {tag_message}"
    );
    assert!(
        tag_message.contains(&tip) && tag_message.contains(&fork),
        "the tag message carries the base and terminal commits: {tag_message}"
    );
}

/// PT-051-02: the ordered finish, and recovery that really recovers.
#[test]
fn finish_order_and_recovery() {
    let ready = Ready::new("finish-order");
    let terminal = ready.trial.head_of(BRANCH);
    let base_before = ready.trial.head_of(BASE);

    let output = ready.finish();
    let report = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "a ready trial finishes: {}{report}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 1. the annotated tag holds the terminal commit
    assert_eq!(
        ready
            .trial
            .git_text(&["rev-parse", &format!("{TAG}^{{commit}}")])
            .trim(),
        terminal,
        "the archive tag preserves the terminal commit"
    );
    assert_eq!(
        ready.trial.git_text(&["cat-file", "-t", TAG]).trim(),
        "tag",
        "the archive tag is annotated"
    );

    // 2. exactly one base commit, carrying exactly the durable log
    let added = ready
        .trial
        .git_text(&["rev-list", &format!("{base_before}..{BASE}")]);
    assert_eq!(
        added.lines().count(),
        1,
        "finish adds exactly one commit to the base: {added}"
    );
    let touched = ready
        .trial
        .git_text(&["diff-tree", "--no-commit-id", "--name-only", "-r", BASE]);
    assert_eq!(
        touched.trim(),
        DURABLE,
        "the base commit carries exactly the durable log"
    );
    assert_eq!(
        fs::read_to_string(ready.trial.root.join(DURABLE)).expect("durable log on the base"),
        ready
            .trial
            .git_text(&["show", &format!("{TAG}^{{commit}}:trial-log.md")]),
        "the durable log is the log the trial committed"
    );

    // 3. and 4. no worktree, no branch
    assert!(
        !ready.worktree.exists(),
        "the linked worktree is removed from disk"
    );
    assert!(
        !ready
            .trial
            .git_text(&["worktree", "list", "--porcelain"])
            .contains(WORKTREE),
        "the worktree registration is gone"
    );
    assert!(
        !ready
            .trial
            .git_in(&ready.trial.root, &["rev-parse", "--verify", BRANCH])
            .status
            .success(),
        "the trial branch is deleted"
    );

    // The report states the four steps in the order the contract fixes.
    let order: Vec<usize> = ["tag", "durable log", "worktree", "branch"]
        .iter()
        .map(|step| {
            report
                .find(step)
                .unwrap_or_else(|| panic!("the report names step {step}: {report}"))
        })
        .collect();
    assert!(
        order.windows(2).all(|pair| pair[0] < pair[1]),
        "the report keeps the fixed order: {report}"
    );

    // Recovery: the printed command recreates the branch at the same commit.
    let recovery = report
        .lines()
        .find(|line| line.contains("git branch") && line.contains(TAG))
        .unwrap_or_else(|| panic!("the report prints the branch recovery command: {report}"))
        .trim()
        .to_owned();
    let arguments: Vec<&str> = recovery.split_whitespace().skip(1).collect();
    let recovered = ready.trial.git_in(&ready.trial.root, &arguments);
    assert!(
        recovered.status.success(),
        "the printed recovery command runs: {recovery} -> {}",
        String::from_utf8_lossy(&recovered.stderr)
    );
    assert_eq!(
        ready.trial.head_of(BRANCH),
        terminal,
        "recovery restores the identical terminal commit"
    );
}

/// PT-051-03: every failing precondition stops its step and all later steps.
#[test]
fn finish_refusals_are_safe() {
    // A dirty trial worktree.
    let dirty = Ready::new("finish-dirty");
    fs::write(dirty.worktree.join("scratch.txt"), "uncommitted\n").expect("dirty the worktree");
    let before = dirty.trial.snapshot();
    let text = dirty.finish_text();
    assert!(
        text.contains("clean") || text.contains("dirty"),
        "the dirty trial refuses naming cleanliness: {text}"
    );
    assert_eq!(dirty.trial.git_text(&["tag", "--list"]), "", "no tag");
    assert_eq!(dirty.trial.snapshot(), before, "nothing is mutated");

    // A trial with no log at all.
    let logless = Trial::new("finish-logless");
    assert!(
        logless.trial(&["start", "-Slug", SLUG]).status.success(),
        "the fixture trial starts"
    );
    let before = logless.snapshot();
    let text = logless.text(&["finish"]);
    assert!(
        text.contains("trial-log.md"),
        "the missing log refusal names the file: {text}"
    );
    assert_eq!(logless.git_text(&["tag", "--list"]), "", "no tag");
    assert_eq!(logless.snapshot(), before, "nothing is mutated");

    // The working directory inside the trial worktree.
    let inside = Ready::new("finish-inside");
    let before = inside.trial.snapshot();
    let output = inside.trial.trial_in(&inside.worktree, &["finish"]);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.status.success(), "finish from inside refuses");
    assert!(
        text.contains("cd "),
        "the refusal hints how to leave the worktree: {text}"
    );
    assert_eq!(inside.trial.snapshot(), before, "nothing is mutated");

    // A same-named tag already pointing somewhere else.
    let clash = Ready::new("finish-tag-clash");
    clash
        .trial
        .git(&["tag", "-a", TAG, "-m", "unrelated", BASE]);
    let before = clash.trial.snapshot();
    let text = clash.finish_text();
    assert!(
        text.contains(TAG),
        "the tag clash refusal names the tag: {text}"
    );
    assert!(
        clash.worktree.exists() && clash.trial.head_of(BRANCH) == clash.trial.head_of(BRANCH),
        "no later step ran"
    );
    assert_eq!(clash.trial.snapshot(), before, "nothing is mutated");

    // Two live trials with no way to tell which one to finish.
    let ambiguous = Ready::new("finish-ambiguous");
    assert!(
        ambiguous
            .trial
            .trial(&["start", "-Slug", "second"])
            .status
            .success(),
        "a second trial starts"
    );
    let before = ambiguous.trial.snapshot();
    let text = ambiguous.finish_text();
    assert!(
        text.contains(BRANCH) && text.contains("trial-002-second"),
        "the ambiguity refusal names every candidate: {text}"
    );
    assert_eq!(ambiguous.trial.snapshot(), before, "nothing is mutated");
}

/// PT-051-04: the base gains the durable log and nothing else; main is untouched.
#[test]
fn containment_holds() {
    let ready = Ready::new("containment");
    let base_before = ready.base_before.clone();
    let main_before = ready.trial.head_of("main");

    let output = ready.finish();
    assert!(
        output.status.success(),
        "the trial finishes: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let diff = ready
        .trial
        .git_text(&["diff", "--name-only", &base_before, BASE]);
    assert_eq!(
        diff.trim(),
        DURABLE,
        "the base diff against its pre-trial tip is exactly the durable log"
    );
    assert!(
        !ready.trial.root.join("experiment.rs").exists(),
        "no trial implementation content reaches the base checkout"
    );
    assert_eq!(
        ready.trial.head_of("main"),
        main_before,
        "main is untouched by the lifecycle"
    );
}

/// HT-051-01: an interruption after the tag is recognized and resumable.
#[test]
fn interrupted_finish_resumes() {
    let ready = Ready::new("resume");
    let terminal = ready.trial.head_of(BRANCH);
    // Step 1 happened; the process died before the durable log commit.
    ready
        .trial
        .git(&["tag", "-a", TAG, "-m", "interrupted", &terminal]);

    let preview = ready.trial.text(&["status"]);
    assert!(
        preview.contains("archive tag: done") || preview.contains("tag already"),
        "status recognizes the tag-only state: {preview}"
    );
    assert!(
        preview.contains("finish"),
        "status prints the resume command: {preview}"
    );

    let output = ready.finish();
    assert!(
        output.status.success(),
        "the interrupted finish resumes: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        ready
            .trial
            .git_text(&["rev-parse", &format!("{TAG}^{{commit}}")])
            .trim(),
        terminal,
        "the tag still holds the terminal commit"
    );
    assert_eq!(
        ready
            .trial
            .git_text(&["tag", "--list"])
            .lines()
            .filter(|line| line.trim() == TAG)
            .count(),
        1,
        "the resumed finish does not duplicate the tag"
    );
    assert!(
        ready.trial.root.join(DURABLE).is_file(),
        "the resumed finish completes the durable log"
    );
}

/// HT-051-02: a held directory refuses by name, with no force and no kill.
#[test]
fn locked_worktree_refuses_without_force() {
    let ready = Ready::new("locked");
    let terminal = ready.trial.head_of(BRANCH);

    // A live process whose working directory is inside the linked worktree:
    // Windows refuses to remove that directory while it is somebody's cwd.
    let mut holder: Child = Command::new("pwsh")
        .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 60"])
        .current_dir(&ready.worktree)
        .spawn()
        .expect("spawn the holding process");

    let text = ready.finish_text();
    let still_running = holder.try_wait().expect("poll the holder").is_none();
    let _ = holder.kill();
    let _ = holder.wait();

    assert!(still_running, "the refusal kills no process: {text}");
    assert!(
        text.contains(WORKTREE),
        "the refusal names the held path: {text}"
    );
    assert!(
        !text.contains("--force") && !text.contains("-Force"),
        "the refusal offers no force flag: {text}"
    );
    // Earlier steps survive; later steps did not run.
    assert_eq!(
        ready
            .trial
            .git_text(&["rev-parse", &format!("{TAG}^{{commit}}")])
            .trim(),
        terminal,
        "the archive tag created before the failure survives"
    );
    assert!(
        ready.trial.root.join(DURABLE).is_file(),
        "the durable log committed before the failure survives"
    );
    assert_eq!(
        ready.trial.head_of(BRANCH),
        terminal,
        "the branch is not deleted after a failed removal"
    );
    assert!(
        text.contains("finish"),
        "the refusal prints how to resume: {text}"
    );
}

/// HT-051-03: the archive tag is a real recovery point.
#[test]
fn archive_tag_restores_the_branch() {
    let ready = Ready::new("restore");
    let terminal = ready.trial.head_of(BRANCH);
    let tree_before = ready
        .trial
        .git_text(&["rev-parse", &format!("{BRANCH}^{{tree}}")]);

    assert!(
        ready.finish().status.success(),
        "the trial finishes cleanly"
    );

    ready.trial.git(&["branch", BRANCH, TAG]);
    assert_eq!(
        ready.trial.head_of(BRANCH),
        terminal,
        "the recreated branch points at the identical terminal commit"
    );
    assert_eq!(
        ready
            .trial
            .git_text(&["rev-parse", &format!("{BRANCH}^{{tree}}")]),
        tree_before,
        "the restored tree is byte-identical"
    );
    assert!(
        ready
            .trial
            .git_text(&["show", &format!("{BRANCH}:experiment.rs")])
            .contains("fn experiment"),
        "the trial implementation content is recoverable"
    );
}
