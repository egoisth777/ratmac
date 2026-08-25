//! t-107 / THK-001..THK-004: one repo-local lifecycle opens and closes a
//! work item's turn in the fixed order, and removal refuses while the
//! worktree holds the only copy of an artifact.
//!
//! THKV-001 `open_creates_the_turn_and_every_precondition_refuses_with_zero_mutation`
//! THKV-002 `close_runs_the_fixed_order_and_resumes_without_repeating_a_landed_mutation`
//! THKV-003 `removal_refuses_while_the_worktree_holds_the_only_copy`
//! THKV-004 `the_dry_run_changes_nothing_and_names_the_cd_that_fixes_an_inside_invocation`
//!
//! The surface under test is `tools/turn.ps1`, the trial lifecycle's sibling:
//! `open` checks every precondition - clean trunk, no colliding branch or
//! worktree registration, lanes root present - before the first Git write,
//! then creates the item-named branch with its registered sibling worktree
//! and copies the declared untracked lanes root in, skipping each lane's
//! declared build output. `close` runs the working rules' fixed order -
//! merge, verified copy-back, stamp, log line, worktree removal, branch
//! deletion, trunk rerun - each step refusing on its own failure and blocking
//! every later step, an interrupted close resuming from the first
//! uncompleted step without repeating a landed mutation. A removal refuses
//! while the worktree holds the only copy of any artifact, naming the
//! artifact and the copy-back that would release it, with no force variant.
//! `status` is the dry run: it prints each mutating verb's planned mutations
//! and recovery commands while changing nothing observably.
//!
//! Every fixture is a throwaway repository under the temp directory carrying
//! the real script and its declared data in `.ratmac/turn.toml` (the
//! `[roots]`-table shape, repo-local beside the runbook); negative cases are
//! proven against byte-identical snapshots of refs, tags, worktree
//! registrations, the primary index and tree, and every sibling tree. The
//! t-076 replay carries the destruction history its refusal walks (`GPH-001`):
//! a gitignored crate, committed nowhere, whose only copy lives in the
//! worktree a removal is about to take - the exact shape
//! `git worktree remove` destroys silently, because ignored files do not
//! block it.
//!
//! Hole-poke notes:
//! - Would THKV-001 pass a tool that writes and rolls back on a failed
//!   precondition? No. Every refusal twin compares the whole-repo snapshot
//!   byte for byte, so a partial mutation - a created ref, a registered
//!   worktree, one copied file - fails the equality assert.
//! - Would THKV-001 pass a copy that includes build output, or that touches
//!   the source? No. The worktree's skipped directory is asserted absent
//!   while the primary keeps its build-output file, and the primary's lanes
//!   bytes are inside the snapshot.
//! - Would THKV-002 pass a close that redoes the merge on resume? No. The
//!   interrupted invocation's trunk tip is captured and asserted unchanged
//!   by the resumed close, the diverged case pins exactly one merge commit,
//!   and the fast-forward case pins trunk tip == item tip.
//! - Would THKV-002 pass a close that appends the log line before the
//!   copy-back verifies? No. The failed copy-back twin asserts the stamp
//!   field and the landing log are still untouched, so a close that writes
//!   them earlier fails.
//! - Would THKV-003 pass a guard that only watches untracked-not-ignored
//!   files? No. The refused crate is gitignored - the t-076 shape - and the
//!   refusal still fires, because the inventory reads ignored entries too.
//! - Would THKV-003 pass a force bypass? No. The `-Force` invocation is
//!   asserted to refuse with the identical bytes, mutate nothing, and leave
//!   the crate intact.
//! - Would THKV-004 pass a status that mutates while printing? No. The dry
//!   run is bracketed by the whole-repo snapshot, byte for byte.
//! - Would THKV-004 pass a tool that kills a holder or forces a removal?
//!   No. The script source is scanned for the forbidden operations - forced
//!   Git flags, process control, direct deletes - and the hidden lane
//!   HT-107-04 drives a live holder against the same refusal.

use std::fs;

use ratmac_qa::turn::{
    script_source, Turn, ITEM_RECORDS, LANDING_LOG, LANES, RERUN_MARKER, SKIP, STAMP_FIELD, TRUNK,
};

/// The item string every fixture addresses - opaque to the tool, never
/// parsed for a ticket shape.
const ITEM: &str = "item-1";

/// The landing-line words the closes append.
const LINE: &str = "the item-1 turn landed";

/// A refused invocation must exit non-zero and name its reason.
fn refused(output: &std::process::Output, what: &str, reason: &str) -> String {
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "THKV: {what} must refuse, not pass"
    );
    assert!(
        text.to_lowercase().contains(&reason.to_lowercase()),
        "THKV: {what} must name its reason {reason:?}: {text}"
    );
    text
}

/// How many times `needle` occurs in `haystack`.
fn occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

/// The declared rerun marker must exist in a lane directory after a close.
fn marker(turn: &Turn, lane: &str) -> String {
    let path = turn.root.join(LANES).join(lane).join(RERUN_MARKER);
    assert!(
        path.is_file(),
        "the trunk rerun reached lane {lane}: {path:?}"
    );
    path.to_string_lossy().into_owned()
}

/// THKV-001 (t-107, THK-001): `open` creates the item-named branch, its
/// registered sibling worktree, and the lanes copy with build outputs
/// skipped; a dirty trunk, a colliding branch, a colliding worktree
/// registration, and a missing lanes root each refuse with a named reason
/// and zero mutation.
#[test]
fn open_creates_the_turn_and_every_precondition_refuses_with_zero_mutation() {
    let turn = Turn::new("open");
    let trunk_tip = turn.head_of(TRUNK);

    // The positive case: the turn opens exactly as declared.
    let opened = turn.turn(&["open", "-Item", ITEM]);
    assert!(
        opened.status.success(),
        "a clean open succeeds: {}",
        String::from_utf8_lossy(&opened.stderr)
    );
    let report = String::from_utf8_lossy(&opened.stdout);
    assert!(report.contains(ITEM), "the report names the item: {report}");
    let worktree = turn.sibling(ITEM);
    assert_eq!(
        turn.head_of(ITEM),
        trunk_tip,
        "the item branch starts at exactly the trunk tip"
    );
    assert!(worktree.is_dir(), "the sibling worktree exists on disk");
    let registrations = turn.git_text(&["worktree", "list", "--porcelain"]);
    assert!(
        registrations.contains(&format!("branch refs/heads/{ITEM}")),
        "the worktree is registered on the item branch: {registrations}"
    );

    // The lanes copy: files in, declared build output skipped, source intact.
    let copied = fs::read_to_string(worktree.join(LANES).join("crate-a").join("lane.txt"))
        .expect("the lane file is copied in");
    assert_eq!(copied, "lane a\n", "the copied bytes are the source bytes");
    assert!(
        !worktree.join(LANES).join("crate-a").join(SKIP).exists(),
        "the declared build output is skipped"
    );
    assert!(
        turn.root
            .join(LANES)
            .join("crate-a")
            .join(SKIP)
            .join("junk.bin")
            .is_file(),
        "the primary keeps its build output: the copy never touches the source"
    );
    assert_eq!(
        turn.git_text(&["status", "--porcelain"]),
        "",
        "the primary checkout stays clean"
    );
    assert_eq!(turn.git_text(&["tag", "--list"]), "", "open creates no tag");

    // Put the fixture back the way it started, so every twin meets one
    // precondition failure and nothing else.
    turn.git(&["worktree", "remove", "../repo-item-1"]);
    turn.git(&["branch", "-D", ITEM]);

    // Every refusal twin: a named reason and byte-identical everything.
    struct Case {
        name: &'static str,
        reason: &'static str,
        setup: fn(&Turn),
        teardown: fn(&Turn),
    }
    let cases = [
        // The dirty-trunk family: staged, unstaged, and untracked alike.
        Case {
            name: "staged change",
            reason: "clean",
            setup: |turn| {
                fs::write(turn.root.join("staged.txt"), "x\n").expect("write");
                turn.git(&["add", "staged.txt"]);
            },
            teardown: |turn| {
                turn.git(&["reset", "--quiet"]);
                fs::remove_file(turn.root.join("staged.txt")).expect("remove");
            },
        },
        Case {
            name: "unstaged change",
            reason: "clean",
            setup: |turn| {
                fs::write(turn.root.join("README.md"), "# dirty\n").expect("write");
            },
            teardown: |turn| {
                fs::write(turn.root.join("README.md"), "# fixture\n").expect("restore");
            },
        },
        Case {
            name: "untracked stray",
            reason: "clean",
            setup: |turn| {
                fs::write(turn.root.join("stray.txt"), "x\n").expect("write");
            },
            teardown: |turn| {
                fs::remove_file(turn.root.join("stray.txt")).expect("remove");
            },
        },
        Case {
            name: "colliding branch",
            reason: "branch",
            setup: |turn| {
                turn.git(&["branch", ITEM]);
            },
            teardown: |turn| {
                turn.git(&["branch", "-D", ITEM]);
            },
        },
        Case {
            name: "colliding worktree registration",
            reason: "registered",
            setup: |turn| {
                turn.git(&["worktree", "add", "-b", "other", "../repo-item-1", TRUNK]);
            },
            teardown: |turn| {
                turn.git(&["worktree", "remove", "../repo-item-1"]);
                turn.git(&["branch", "-D", "other"]);
            },
        },
        Case {
            name: "occupied sibling path",
            reason: "exists",
            setup: |turn| {
                fs::create_dir_all(turn.sibling(ITEM)).expect("create the sibling path");
            },
            teardown: |turn| {
                fs::remove_dir(turn.sibling(ITEM)).expect("remove the sibling path");
            },
        },
        Case {
            name: "missing lanes root",
            reason: "lanes",
            setup: |turn| {
                fs::remove_dir_all(turn.root.join(LANES)).expect("remove the lanes root");
            },
            teardown: |turn| {
                fs::create_dir_all(turn.root.join(LANES).join("crate-a").join(SKIP))
                    .expect("restore");
                fs::write(
                    turn.root.join(LANES).join("crate-a").join("lane.txt"),
                    "lane a\n",
                )
                .expect("restore");
                fs::write(
                    turn.root
                        .join(LANES)
                        .join("crate-a")
                        .join(SKIP)
                        .join("junk.bin"),
                    "build output\n",
                )
                .expect("restore");
            },
        },
    ];

    for case in cases {
        (case.setup)(&turn);
        let before = turn.snapshot();
        let output = turn.turn(&["open", "-Item", ITEM]);
        refused(&output, &format!("open over {}", case.name), case.reason);
        assert_eq!(
            turn.snapshot(),
            before,
            "the {} refusal mutates nothing: refs, index, trees, and registrations stay byte-identical",
            case.name
        );
        (case.teardown)(&turn);
    }
}

/// THKV-002 (t-107, THK-002): the close completes merge, verified copy-back,
/// stamp, log line, removal, branch delete, and trunk rerun in order, each
/// observable after the command; a forced failure refuses at its step and
/// blocks every later step, and a re-invocation after repair resumes from the
/// refusal without repeating a landed mutation - the merge commit and log
/// line count exactly once.
#[test]
fn close_runs_the_fixed_order_and_resumes_without_repeating_a_landed_mutation() {
    // --- The happy path: every step lands in order, each observable. -------
    let turn = Turn::new("close-clean");
    let worktree = turn.open_with_work(ITEM, "the turn's work");
    let item_tip = turn.head_of(ITEM);

    let closed = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    assert!(
        closed.status.success(),
        "a close over committed work succeeds: {}",
        String::from_utf8_lossy(&closed.stderr)
    );

    // 1. merge: trunk unmoved, so the landing is a fast-forward.
    assert_eq!(
        turn.head_of(TRUNK),
        item_tip,
        "the merge fast-forwards the trunk to the item tip"
    );
    // 2. verified copy-back: the worktree's new lane is in the primary.
    let copied = fs::read_to_string(turn.root.join(LANES).join("crate-b").join("b.txt"));
    assert_eq!(
        copied.as_deref().ok(),
        Some("lane b\n"),
        "the copy-back carries the worktree's lanes into the primary"
    );
    assert!(
        turn.root
            .join(LANES)
            .join("crate-a")
            .join(SKIP)
            .join("junk.bin")
            .is_file(),
        "the copy-back deletes nothing the primary already held"
    );
    // 3. stamp: the item record's declared field carries the landed tip.
    assert_eq!(
        turn.field_value(&format!("{ITEM_RECORDS}/{ITEM}.md"), STAMP_FIELD),
        turn.short_of(TRUNK),
        "the stamp field names the landed trunk tip"
    );
    // 4. log line: appended once, in the house line shape.
    let log = fs::read_to_string(turn.root.join(LANDING_LOG)).expect("read the landing log");
    let last = log.lines().last().unwrap_or_default();
    assert_eq!(
        occurrences(&log, LINE),
        1,
        "the landing line is appended once"
    );
    assert!(
        last.starts_with("- ") && last.ends_with(&format!(": {LINE}")),
        "the landing line carries the house shape `- <date>: <words>`: {last}"
    );
    assert!(
        last.as_bytes()[6] == b'-' && last.as_bytes()[9] == b'-',
        "the landing line's middle is a calendar date: {last}"
    );
    // 5./6. removal and branch deletion.
    assert!(!worktree.exists(), "the turn worktree is removed");
    let registrations = turn.git_text(&["worktree", "list", "--porcelain"]);
    assert!(
        !registrations.contains(&format!("branch refs/heads/{ITEM}")),
        "the worktree registration is gone: {registrations}"
    );
    let branch = turn.git_in(
        &turn.root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{ITEM}"),
        ],
    );
    assert!(!branch.status.success(), "the item branch is deleted");
    // 7. trunk rerun: the declared rerun command ran in each lane.
    marker(&turn, "crate-a");
    marker(&turn, "crate-b");

    // --- Twin one: an unmergable trunk refuses at the merge. ---------------
    let turn = Turn::new("close-conflicted");
    let worktree = turn.open_with_work(ITEM, "the turn's conflicting work");
    let base = turn.head_of(TRUNK);
    fs::write(turn.root.join("README.md"), "# trunk moved\n").expect("move the trunk");
    turn.git(&["add", "-A"]);
    turn.git(&["commit", "-m", "trunk moves"]);

    let output = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    let text = refused(&output, "close over a conflicted merge", "merge");
    assert!(
        text.contains("README.md"),
        "the merge refusal names the conflicted file: {text}"
    );
    // Later steps did not run.
    assert!(
        !turn.root.join(LANES).join("crate-b").exists(),
        "the copy-back did not run"
    );
    assert_eq!(
        turn.field_value(&format!("{ITEM_RECORDS}/{ITEM}.md"), STAMP_FIELD),
        "",
        "the stamp did not run"
    );
    let log = fs::read_to_string(turn.root.join(LANDING_LOG)).expect("read the landing log");
    assert_eq!(occurrences(&log, LINE), 0, "the log line did not run");
    assert!(worktree.exists(), "the removal did not run");
    assert!(
        turn.git_in(
            &turn.root,
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("refs/heads/{ITEM}")
            ]
        )
        .status
        .success(),
        "the branch deletion did not run"
    );

    // Repair by concluding the conflicted merge, then resume.
    fs::write(turn.root.join("README.md"), "# fixture\nresolved\n").expect("resolve");
    turn.git(&["add", "-A"]);
    turn.git(&["commit", "-m", "conclude the merge"]);
    let merged_tip = turn.head_of(TRUNK);
    assert_eq!(
        turn.git_text(&[
            "rev-list",
            "--merges",
            "--count",
            &format!("{base}..{TRUNK}")
        ])
        .trim(),
        "1",
        "the repair leaves exactly one merge commit"
    );

    let resumed = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    assert!(
        resumed.status.success(),
        "the resumed close succeeds: {}",
        String::from_utf8_lossy(&resumed.stderr)
    );
    assert_eq!(
        turn.head_of(TRUNK),
        merged_tip,
        "the resume does not repeat the landed merge"
    );
    assert_eq!(
        turn.git_text(&[
            "rev-list",
            "--merges",
            "--count",
            &format!("{base}..{TRUNK}")
        ])
        .trim(),
        "1",
        "the merge commit counts exactly once across both invocations"
    );
    let log = fs::read_to_string(turn.root.join(LANDING_LOG)).expect("read the landing log");
    assert_eq!(
        occurrences(&log, LINE),
        1,
        "the log line counts exactly once across both invocations"
    );
    assert!(!worktree.exists(), "the resumed close removes the worktree");

    // --- Twin two: a copy-back that cannot verify refuses at its step. -----
    let turn = Turn::new("close-copyback");
    let worktree = turn.open_with_work(ITEM, "the turn's copy-back work");

    // A directory squatting on a lane file's path: the copy cannot pass.
    let lane_file = turn.root.join(LANES).join("crate-a").join("lane.txt");
    fs::remove_file(&lane_file).expect("remove the lane file");
    fs::create_dir_all(&lane_file).expect("squat on the lane file's path");
    fs::write(lane_file.join("inner.txt"), "not a file\n").expect("fill the squatter");

    let output = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    let text = refused(&output, "close over an unverifiable copy-back", "copy-back");
    assert!(
        text.to_lowercase().contains("verif"),
        "the copy-back refusal names verification: {text}"
    );
    let tip_after_merge = turn.head_of(TRUNK);
    // The merge landed; every later step is blocked.
    assert_eq!(
        turn.field_value(&format!("{ITEM_RECORDS}/{ITEM}.md"), STAMP_FIELD),
        "",
        "the stamp is blocked behind the failed copy-back"
    );
    let log = fs::read_to_string(turn.root.join(LANDING_LOG)).expect("read the landing log");
    assert_eq!(occurrences(&log, LINE), 0, "the log line is blocked");
    assert!(worktree.exists(), "the removal is blocked");
    assert!(
        turn.git_in(
            &turn.root,
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("refs/heads/{ITEM}")
            ]
        )
        .status
        .success(),
        "the branch deletion is blocked"
    );

    // Repair the squatter, then resume: the merge is not redone.
    fs::remove_dir_all(&lane_file).expect("remove the squatter");
    fs::write(&lane_file, "lane a\n").expect("restore the lane file");
    let resumed = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    assert!(
        resumed.status.success(),
        "the resumed close succeeds: {}",
        String::from_utf8_lossy(&resumed.stderr)
    );
    assert_eq!(
        turn.head_of(TRUNK),
        tip_after_merge,
        "the resume does not repeat the landed merge"
    );
    assert_eq!(
        turn.field_value(&format!("{ITEM_RECORDS}/{ITEM}.md"), STAMP_FIELD),
        turn.short_of(TRUNK),
        "the resumed close stamps the landed tip"
    );
    let log = fs::read_to_string(turn.root.join(LANDING_LOG)).expect("read the landing log");
    assert_eq!(
        occurrences(&log, LINE),
        1,
        "the log line counts exactly once"
    );
    assert!(!worktree.exists(), "the resumed close removes the worktree");
}

/// THKV-003 (t-107, THK-003): a removal refuses while the worktree holds the
/// only copy of an untracked artifact, naming the artifact and the copy-back
/// that would release it; a force-flag variant refuses identically; after a
/// verified copy-back the removal proceeds; the t-076 replay - removal
/// invoked before the crate's copy-back - is refused and the crate survives.
#[test]
fn removal_refuses_while_the_worktree_holds_the_only_copy() {
    // The t-076 replay (GPH-001): a gitignored crate, committed nowhere,
    // whose only copy lives in the worktree the close is about to remove -
    // the exact shape `git worktree remove` destroys silently.
    let turn = Turn::new("only-copy");
    let worktree = turn.open_with_work(ITEM, "the turn's only-copy work");
    let crate_dir = worktree.join("only-copy-crate");
    fs::create_dir_all(&crate_dir).expect("create the crate");
    fs::write(crate_dir.join("lib.txt"), "the only copy\n").expect("write the crate");

    let output = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    let refusal = String::from_utf8_lossy(&output.stderr).to_string();
    let text = refused(&output, "removal over the only copy", "only copy");
    assert!(
        text.contains("only-copy-crate"),
        "the refusal names the crate: {text}"
    );
    assert!(
        text.to_lowercase().contains("copy-back") || text.to_lowercase().contains("copy back"),
        "the refusal names the copy-back that releases it: {text}"
    );
    // The earlier steps stand; the removal and everything after did not run.
    let tip = turn.head_of(TRUNK);
    assert_eq!(
        turn.field_value(&format!("{ITEM_RECORDS}/{ITEM}.md"), STAMP_FIELD),
        turn.short_of(TRUNK),
        "the merge, copy-back, stamp, and log line stand behind the refusal"
    );
    let log = fs::read_to_string(turn.root.join(LANDING_LOG)).expect("read the landing log");
    assert_eq!(
        occurrences(&log, LINE),
        1,
        "the log line counts exactly once"
    );
    assert!(worktree.exists(), "the worktree survives the refusal");
    assert!(
        !turn.root.join("only-copy-crate").exists(),
        "the primary holds no copy of the crate yet"
    );
    assert_eq!(
        fs::read_to_string(crate_dir.join("lib.txt"))
            .as_deref()
            .ok(),
        Some("the only copy\n"),
        "the crate survives the refused removal, bytes intact"
    );

    // The force-flag variant refuses identically.
    let before = turn.snapshot();
    let forced = turn.turn(&["close", "-Item", ITEM, "-Line", LINE, "-Force"]);
    assert!(
        !forced.status.success(),
        "the force variant must refuse, not pass"
    );
    assert_eq!(
        String::from_utf8_lossy(&forced.stderr),
        refusal,
        "the force variant refuses with the identical refusal bytes: no force path exists"
    );
    assert_eq!(turn.snapshot(), before, "the force variant mutates nothing");
    assert!(
        fs::read_to_string(crate_dir.join("lib.txt")).is_ok(),
        "the crate survives the force variant"
    );

    // Release by verified copy-back: the crate in the primary, then removal.
    fs::create_dir_all(turn.root.join("only-copy-crate")).expect("copy the crate out");
    fs::write(
        turn.root.join("only-copy-crate").join("lib.txt"),
        "the only copy\n",
    )
    .expect("write the released copy");
    let closed = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    assert!(
        closed.status.success(),
        "after a verified copy-back the removal proceeds: {}",
        String::from_utf8_lossy(&closed.stderr)
    );
    assert!(!worktree.exists(), "the released worktree is removed");
    assert_eq!(
        fs::read_to_string(turn.root.join("only-copy-crate").join("lib.txt"))
            .as_deref()
            .ok(),
        Some("the only copy\n"),
        "the crate survives in the primary checkout"
    );
    assert_eq!(turn.head_of(TRUNK), tip, "the release adds no landing");

    // The one explicit release: obsolete by name, declared by the owner.
    let turn = Turn::new("obsolete");
    let worktree = turn.open_with_work(ITEM, "the turn's obsolete declaration");
    let doomed = worktree.join("doomed-crate");
    fs::create_dir_all(&doomed).expect("create the doomed crate");
    fs::write(doomed.join("lib.txt"), "declared obsolete\n").expect("write the doomed crate");
    let output = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    refused(&output, "removal over an undeclared crate", "doomed-crate");
    let closed = turn.turn(&[
        "close",
        "-Item",
        ITEM,
        "-Line",
        LINE,
        "-Obsolete",
        "doomed-crate",
    ]);
    assert!(
        closed.status.success(),
        "the obsolete-by-name declaration releases the removal: {}",
        String::from_utf8_lossy(&closed.stderr)
    );
    let report = String::from_utf8_lossy(&closed.stdout);
    assert!(
        report.contains("doomed-crate"),
        "the release names the declared artifact: {report}"
    );
    assert!(!worktree.exists(), "the released worktree is removed");
}

/// THKV-004 (t-107, THK-004): the dry run prints each mutating verb's
/// planned mutations and recovery commands while changing nothing
/// observably; an invocation from inside the turn worktree refuses by name
/// with the `cd` that fixes it; no code path kills a process or forces a
/// removal.
#[test]
fn the_dry_run_changes_nothing_and_names_the_cd_that_fixes_an_inside_invocation() {
    let turn = Turn::new("status");
    let worktree = turn.open_with_work(ITEM, "the turn's dry-run work");

    // The dry run reports both mutating verbs, byte-identically around it.
    let before = turn.snapshot();
    let status = turn.turn(&["status", "-Item", ITEM]);
    assert!(
        status.status.success(),
        "the dry run passes: {}",
        String::from_utf8_lossy(&status.stderr)
    );
    let report = String::from_utf8_lossy(&status.stdout);
    assert!(
        report.contains("read-only") && report.contains("nothing below has been applied"),
        "the dry run states it applies nothing: {report}"
    );
    let open_plan = report
        .find("open plan")
        .unwrap_or_else(|| panic!("the dry run prints the open plan: {report}"));
    assert!(
        report[open_plan..].contains(&format!("git worktree add -b {ITEM}")),
        "the open plan names its planned mutation: {report}"
    );
    assert!(
        report[open_plan..].to_lowercase().contains("recovery"),
        "the open plan prints its recovery commands: {report}"
    );
    let close_plan = report
        .find("close plan")
        .unwrap_or_else(|| panic!("the dry run prints the close plan: {report}"));
    let tail = &report[close_plan..];
    let steps = [
        "merge",
        "copy-back",
        "stamp",
        "log line",
        "remove the worktree",
        "delete the branch",
        "re-run the lanes",
    ];
    let mut cursor = 0;
    for step in steps {
        let found = tail[cursor..]
            .find(step)
            .unwrap_or_else(|| panic!("the close plan names the {step} step in order: {tail}"));
        cursor += found + step.len();
    }
    assert!(
        tail.to_lowercase().contains("recovery") || tail.to_lowercase().contains("resume"),
        "the close plan prints its recovery commands: {tail}"
    );
    assert_eq!(
        turn.snapshot(),
        before,
        "the dry run changes nothing: refs, index, trees, tags, and registrations stay byte-identical"
    );

    // The inside invocation refuses by name, with the cd that fixes it.
    let output = turn.turn_in(worktree.as_path(), &["status", "-Item", ITEM]);
    let text = refused(&output, "status from inside the turn worktree", "inside");
    let flat = text.replace('\\', "/");
    let worktree_flat = worktree.to_string_lossy().replace('\\', "/");
    assert!(
        flat.contains(&worktree_flat),
        "the refusal names the worktree it refuses from: {text}"
    );
    let cd = format!("cd {}", turn.root.to_string_lossy().replace('\\', "/"));
    assert!(
        flat.contains(&cd),
        "the refusal prints the cd that fixes it ({cd}): {text}"
    );
    assert_eq!(
        turn.snapshot(),
        before,
        "the inside invocation mutates nothing"
    );

    // No code path kills a process or forces a removal: the script's own
    // source is scanned for the forbidden operations.
    let source = fs::read_to_string(script_source()).expect("read the script under test");
    let lowered = source.to_lowercase();
    for forbidden in [
        "--force",
        "reset --hard",
        "git clean",
        "stop-process",
        "taskkill",
        "get-process",
        "remove-item",
        "rm -rf",
        "push --force",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "no code path forces a removal or kills a process: {forbidden} appears in the script"
        );
    }
}
