//! t-125 / WCP-006: checkpoint only restoration - the turn tool's damage
//! pair records exactly one safety commit per checkpoint and restores only
//! from the manifest's recorded checkpoint, never from a backup, the index,
//! a newest file, a caller-supplied revision, or an inverse edit.
//!
//! Every fixture is a throwaway repository under the temp directory built by
//! the turn-lifecycle family (`ratmac_qa::turn`), carrying the real
//! `tools/turn.ps1` and its declared data, with the fixture's reviewed green
//! work committed under the fixed safety subject `<item>: checkpoint - not a
//! landing` - the one commit `checkpoint` may record. Git in a fixture - the
//! tool's own children included - reads a fixture-local empty global
//! configuration file through `GIT_CONFIG_GLOBAL` with
//! `GIT_CONFIG_NOSYSTEM=1`; the pause seam's environment variables ride the
//! child invocation alone. Each check opens with a supported checkpoint and
//! restore that must succeed with t-124's exact success bytes, then drives
//! the refusal twins with before/after snapshots proving nothing was written.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Output};
use std::time::{Duration, SystemTime};

use ratmac_qa::snapshot::sha256_file;
use ratmac_qa::support::{
    capture, command, CaptureOptions, MarkerLine, MarkerWait, Snapshot, TempTree,
};
use ratmac_qa::turn::{Turn, ITEM_RECORDS, RERUN_MARKER, TRUNK};
use sha2::{Digest, Sha256};

/// The item string every fixture addresses.
const ITEM: &str = "item-1";

/// The fixed safety subject: the one commit `checkpoint` records and the one
/// subject `restore` accepts at the recorded checkpoint (`-Item` value
/// followed by `: checkpoint - not a landing`).
const SAFETY_SUBJECT: &str = "item-1: checkpoint - not a landing";

// --- isolated fixture plumbing ------------------------------------------------

/// An owned tree holding nothing but the empty global Git configuration file
/// every fixture-local child reads instead of the host's, plus this suite's
/// pause-seam marker and release files - always outside the fixture
/// repositories the verbs address.
fn isolation(label: &str) -> (TempTree, PathBuf) {
    let tree = TempTree::new(&format!("t125-{label}")).expect("create the isolation tree");
    let global = tree.join("git-global.cfg");
    fs::write(&global, b"").expect("write the empty global config");
    (tree, global)
}

/// The tool under test, run in `directory` with scrubbed children and Git
/// configuration isolated to the fixture-local empty global file.
fn turn_isolated(global: &Path, directory: &Path, args: &[&str]) -> Output {
    let mut all = vec!["-NoProfile", "-File", "tools/turn.ps1"];
    all.extend_from_slice(args);
    let mut invocation = command("pwsh", directory);
    invocation
        .args(&all)
        .env("GIT_CONFIG_GLOBAL", global)
        .env("GIT_CONFIG_NOSYSTEM", "1");
    invocation.output().expect("invoke pwsh")
}

/// The restore verb paused at the `after-validate` boundary - the seam
/// t-124 landed, its variables set on this child alone - so a check can
/// observe and plant state while the verb holds before its first mutation.
fn paused_restore(global: &Path, turn: &Turn, marker: &Path, release: &Path) -> Child {
    let mut invocation = command("pwsh", &turn.root);
    invocation
        // The spawned verb's streams must be piped, or wait_with_output
        // below has nothing to capture: command() only nulls stdin.
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .args([
            "-NoProfile",
            "-File",
            "tools/turn.ps1",
            "restore",
            "-Item",
            ITEM,
        ])
        .env("GIT_CONFIG_GLOBAL", global)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("RATMAC_TEST_TURN_PAUSE", "after-validate")
        .env("RATMAC_TEST_TURN_PAUSE_MARKER", marker)
        .env("RATMAC_TEST_TURN_PAUSE_RELEASE", release)
        .env("RATMAC_TEST_TURN_PAUSE_TIMEOUT_MILLIS", "60000");
    invocation.spawn().expect("spawn the paused restore")
}

/// Wait up to `limit` for the pause seam's expected whole marker line.
fn await_marker(
    marker: &Path,
    expected: MarkerLine<'_>,
    limit: Duration,
) -> Result<String, MarkerWait> {
    ratmac_qa::support::await_marker_line(marker, expected, limit)
}

/// Fixture Git with the same isolation, and no other setting at all.
fn git_isolated(global: &Path, directory: &Path, args: &[&str]) -> Output {
    let mut invocation = command("git", directory);
    invocation
        .args(args)
        .env("GIT_CONFIG_GLOBAL", global)
        .env("GIT_CONFIG_NOSYSTEM", "1");
    invocation.output().expect("invoke git")
}

/// Successful isolated Git, its stdout trimmed.
fn git_ok(global: &Path, directory: &Path, args: &[&str]) -> String {
    let output = git_isolated(global, directory, args);
    assert!(
        output.status.success(),
        "t125: git {args:?} fails: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// The fixture's reviewed green work: one README change committed under the
/// given subject, staging nothing else - untracked evidence beside it stays
/// untracked.
fn green_commit(global: &Path, turn: &Turn, readme: &str, subject: &str) {
    fs::write(turn.root.join("README.md"), readme).expect("write the fixture's green work");
    git_ok(global, &turn.root, &["add", "--", "README.md"]);
    git_ok(global, &turn.root, &["commit", "-m", subject]);
}

// --- exact-output helpers -----------------------------------------------------

/// Whole lines of one captured stream, each stripped of the console newline's
/// carriage return, so every assertion is exact per line.
fn lines(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(|line| line.to_owned())
        .collect()
}

/// A stream must be exactly these lines, byte for byte per line.
fn assert_lines(bytes: &[u8], expected: &[&str], what: &str) {
    let actual = lines(bytes);
    assert_eq!(actual, expected, "t125: {what} must be exactly these lines");
}

/// The fixture's `git status --porcelain` rows under the same isolation.
fn porcelain(global: &Path, directory: &Path) -> Vec<String> {
    lines(&git_isolated(global, directory, &["status", "--porcelain"]).stdout)
}

/// Today's `status` verb stays green - the positive control that precedes
/// every refusal twin.
fn status_control(global: &Path, turn: &Turn) {
    let output = turn_isolated(global, &turn.root, &["status", "-Item", ITEM]);
    assert!(
        output.status.success(),
        "t125: today's status verb stays green: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        lines(&output.stdout).first().map(String::as_str),
        Some("turn status (read-only; nothing below has been applied)"),
        "t125: the status report keeps its first line"
    );
}

/// A refusal: exit non-zero, stdout empty, stderr exactly these lines.
fn assert_refusal(output: &Output, stderr: &[String], what: &str) {
    assert!(
        !output.status.success(),
        "t125: {what} refuses: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_lines(&output.stdout, &[], "the refusal prints no success line");
    let expected: Vec<&str> = stderr.iter().map(String::as_str).collect();
    assert_lines(&output.stderr, &expected, what);
}

// --- whole-state snapshot helpers ----------------------------------------------

/// Every node of the fixture tree, minus the `.git` directory a Git verb
/// legitimately rewrites.
fn tree(turn: &Turn) -> Snapshot {
    capture(
        &turn.root,
        CaptureOptions {
            excluded_directory_names: &[".git"],
            ..CaptureOptions::default()
        },
    )
    .expect("capture the fixture tree")
}

/// The `/`-separated relative paths whose node differs between two captures -
/// added, removed, or with different bytes.
fn changed_paths(before: &Snapshot, after: &Snapshot) -> Vec<String> {
    let mut changed = Vec::new();
    for (path, node) in &after.nodes {
        if before.nodes.get(path) != Some(node) {
            changed.push(path.to_string_lossy().replace('\\', "/"));
        }
    }
    for path in before.nodes.keys() {
        if !after.nodes.contains_key(path) {
            changed.push(path.to_string_lossy().replace('\\', "/"));
        }
    }
    changed.sort();
    changed
}

/// The Git state a refusal must leave untouched: refs, index, and porcelain
/// rows under the same isolation.
fn git_state(global: &Path, turn: &Turn) -> String {
    let refs = git_ok(global, &turn.root, &["show-ref"]);
    let index = git_ok(global, &turn.root, &["ls-files", "--stage"]);
    let status = git_ok(global, &turn.root, &["status", "--porcelain"]);
    format!("refs:\n{refs}\nindex:\n{index}\nstatus:\n{status}\n")
}

/// The configuration bytes no verb may write: the fixture repository's own
/// `.git/config` and the isolated global file every child reads.
fn configs(global: &Path, turn: &Turn) -> (Vec<u8>, Vec<u8>) {
    let repository = fs::read(turn.root.join(".git/config"))
        .expect("read the fixture's repository configuration");
    let isolated = fs::read(global).expect("read the isolated global configuration");
    (repository, isolated)
}

/// A misleading backup's observable facts: its bytes and its mtime.
fn backup_facts(path: &Path) -> (Vec<u8>, Option<SystemTime>) {
    let bytes = fs::read(path).expect("read the backup's bytes");
    let modified = fs::metadata(path)
        .expect("read the backup's metadata")
        .modified()
        .ok();
    (bytes, modified)
}

/// A backup a restore never reads keeps its bytes and its mtime exactly.
fn assert_backup_unchanged(before: &(Vec<u8>, Option<SystemTime>), path: &Path, what: &str) {
    assert_eq!(
        backup_facts(path),
        *before,
        "t125: {what}: the misleading backup's bytes and mtime are unchanged"
    );
}

// --- shared fixture facts -----------------------------------------------------

/// The frozen manifest's path inside a fixture.
fn manifest_path(turn: &Turn) -> PathBuf {
    turn.root
        .join(format!("{ITEM_RECORDS}/{ITEM}-checkpoint-manifest.txt"))
}

/// The exact manifest bytes a checkpoint of `targets` must freeze, sorted
/// bytewise ascending, LF-terminated, with the digests and blob ids this
/// test recomputes itself - t-124's bytes, unchanged.
fn expected_manifest(global: &Path, turn: &Turn, head: &str, targets: &[&str]) -> String {
    let mut text = format!(
        "item {ITEM}\ncommit {head}\nsetting core.autocrlf false\nsetting core.eol unset\n"
    );
    let mut rows: Vec<String> = Vec::new();
    for target in targets {
        let digest = sha256_file(&turn.root.join(target));
        let blob = git_ok(
            global,
            &turn.root,
            &["rev-parse", &format!("{head}:{target}")],
        );
        rows.push(format!(
            "target {target}\t{digest}\t{blob}\tunspecified\tunspecified\tunspecified\n"
        ));
    }
    rows.sort();
    for row in rows {
        text.push_str(&row);
    }
    text
}

/// A manifest row set this test forges for another revision, every row made
/// to agree with that revision - the input an arbitrary-revision restore
/// would need if one existed.
fn manifest_for(global: &Path, turn: &Turn, commit: &str, target: &str) -> String {
    let blob = git_ok(
        global,
        &turn.root,
        &["rev-parse", &format!("{commit}:{target}")],
    );
    let bytes = git_isolated(
        global,
        &turn.root,
        &["cat-file", "blob", &format!("{commit}:{target}")],
    )
    .stdout;
    let digest = {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    };
    format!(
        "item {ITEM}\ncommit {commit}\nsetting core.autocrlf false\nsetting core.eol unset\n\
         target {target}\t{digest}\t{blob}\tunspecified\tunspecified\tunspecified\n"
    )
}

// --- t-131's review/proofs setup (a fixture-only adjustment) -------------

/// t-131 (WCP-004) inserts a gate into the checkpoint verb: the freeze
/// accepts only reviewed, proven green work, so every supported checkpoint
/// below first records an accepted review and earns the proof receipt. The
/// fixture's declared `lanes-rerun` command is already a trivial trace
/// command - this commits its root marker to the fixture's ignore list
/// first, so earning the proof set changes no reviewed row. A fixture-only
/// adjustment citing t-131's Observable Output; no refusal text, manifest
/// byte, or success byte in this suite changes.
fn declare_trace_marker_ignored(global: &Path, turn: &Turn) {
    let gitignore = turn.root.join(".gitignore");
    let current = fs::read_to_string(&gitignore).expect("read the fixture's ignore list");
    if current.lines().any(|line| line == RERUN_MARKER) {
        return;
    }
    fs::write(&gitignore, format!("{current}{RERUN_MARKER}\n"))
        .expect("ignore the declared trace marker");
    git_ok(global, &turn.root, &["add", ".gitignore"]);
    git_ok(
        global,
        &turn.root,
        &["commit", "-m", "fixture: ignore the declared trace marker"],
    );
}

/// The accepted review and the earned proof receipt the inserted checkpoint
/// gate demands, recorded over the fixture's current green bytes before
/// each freeze - a renewed freeze re-records and re-earns, exactly the
/// renewed-review rule of t-131 (WCP-004).
fn review_and_proofs(global: &Path, turn: &Turn) {
    let review = turn_isolated(
        global,
        &turn.root,
        &["review", "-Item", ITEM, "-Verdict", "accepted"],
    );
    assert!(
        review.status.success(),
        "t125 t-131 setup: the accepted review is recorded over the green bytes (the review verb must exist): {}{}",
        String::from_utf8_lossy(&review.stdout),
        String::from_utf8_lossy(&review.stderr)
    );
    let proofs = turn_isolated(global, &turn.root, &["proofs", "-Item", ITEM]);
    assert!(
        proofs.status.success(),
        "t125 t-131 setup: the proof set is earned behind the accepted review (the proofs verb must exist): {}{}",
        String::from_utf8_lossy(&proofs.stdout),
        String::from_utf8_lossy(&proofs.stderr)
    );
}

/// A supported checkpoint: exit 0, stderr empty, t-124's exact success
/// report, and the manifest's exact bytes recording the safety commit.
fn checkpoint_success(global: &Path, turn: &Turn, head: &str, targets: &[&str], what: &str) {
    // t-131 (WCP-004): the inserted checkpoint gate demands the accepted
    // review and the earned proof receipt over the current green bytes.
    review_and_proofs(global, turn);
    let spelling = targets.join(",");
    let frozen = turn_isolated(
        global,
        &turn.root,
        &["checkpoint", "-Item", ITEM, "-Target", &spelling],
    );
    assert!(
        frozen.status.success(),
        "t125: {what}: the supported checkpoint succeeds: {}{}",
        String::from_utf8_lossy(&frozen.stdout),
        String::from_utf8_lossy(&frozen.stderr)
    );
    assert!(
        frozen.stderr.is_empty(),
        "t125: {what}: a successful checkpoint prints no stderr: {}",
        String::from_utf8_lossy(&frozen.stderr)
    );
    let noun = if targets.len() == 1 {
        "target"
    } else {
        "targets"
    };
    assert_lines(
        &frozen.stdout,
        &[
            "turn checkpoint frozen",
            "  item:      item-1",
            &format!("  commit:    {head}"),
            &format!(
                "  manifest:  items/{ITEM}-checkpoint-manifest.txt ({} {noun}, round-trip proven)",
                targets.len()
            ),
            &format!(
                "  damage:    mutate only the declared targets, then run: \
                 pwsh -File tools/turn.ps1 restore -Item {ITEM}"
            ),
        ],
        "the checkpoint report",
    );
    assert_eq!(
        String::from_utf8_lossy(
            &fs::read(manifest_path(turn)).expect("the frozen manifest exists")
        ),
        expected_manifest(global, turn, head, targets),
        "t125: {what}: the frozen manifest's exact bytes"
    );
}

/// A supported restore: exit 0, stderr empty, t-124's exact success report,
/// and a tracked tree whose every row is untracked.
fn restore_success(global: &Path, turn: &Turn, head: &str, count: usize, what: &str) {
    let verified = turn_isolated(global, &turn.root, &["restore", "-Item", ITEM]);
    assert!(
        verified.status.success(),
        "t125: {what}: the supported restore succeeds: {}{}",
        String::from_utf8_lossy(&verified.stdout),
        String::from_utf8_lossy(&verified.stderr)
    );
    assert!(
        verified.stderr.is_empty(),
        "t125: {what}: a successful restore prints no stderr: {}",
        String::from_utf8_lossy(&verified.stderr)
    );
    assert_lines(
        &verified.stdout,
        &[
            "turn restore verified",
            "  item:      item-1",
            &format!("  commit:    {head}"),
            &format!("  manifest:  items/{ITEM}-checkpoint-manifest.txt"),
            &format!(
                "  targets:   {count} restored; every recomputed digest matches its frozen digest; \
                 index and tracked tree clean"
            ),
        ],
        "the restore report",
    );
    for row in porcelain(global, &turn.root) {
        assert!(
            row.starts_with("??"),
            "t125: {what}: the restored tree is clean, every row untracked: {row}"
        );
    }
}

// --- the checks ----------------------------------------------------------------

/// WCPV-016: two source generations and a misleading backup cannot make the
/// supported restore select stale bytes - only the recorded checkpoint is
/// used, and a manifest recording the older generation while `HEAD` is the
/// newer refuses the stale row with both ids, writing nothing.
#[test]
fn wcpv_016() {
    let (_isolation, global) = isolation("wcpv016");
    let turn = Turn::new("wcpv016");
    // t-131 (WCP-004): the trace marker joins the ignore list first, so the
    // earned proof set changes no reviewed row.
    declare_trace_marker_ignored(&global, &turn);
    status_control(&global, &turn);

    let readme = turn.root.join("README.md");
    let record = turn.root.join(format!("{ITEM_RECORDS}/{ITEM}.md"));
    let manifest = manifest_path(&turn);
    let targets = ["README.md", "items/item-1.md"];
    let record_bytes = fs::read(&record).expect("read the item record's green bytes");

    // Generation one: reviewed green work under the safety subject, frozen
    // and restored - the positive control with t-124's exact success bytes.
    green_commit(&global, &turn, "# fixture one\n", SAFETY_SUBJECT);
    let x = turn.head_of(TRUNK);
    checkpoint_success(&global, &turn, &x, &targets, "the generation-one control");
    fs::write(&readme, "# damaged one\n").expect("damage the first target");
    fs::write(&record, "damaged record\n").expect("damage the second target");
    restore_success(
        &global,
        &turn,
        &x,
        targets.len(),
        "the generation-one control",
    );
    assert_eq!(
        fs::read(&readme).expect("reread the first target"),
        b"# fixture one\n".as_slice(),
        "t125 WCPV-016: the control returns the first target to its green bytes"
    );
    assert_eq!(
        fs::read(&record).expect("reread the second target"),
        record_bytes,
        "t125 WCPV-016: the control returns the second target to its green bytes"
    );
    let x_manifest = fs::read(&manifest).expect("keep the generation-one manifest bytes");

    // Generation two: further green work under the same subject, re-frozen -
    // an existing manifest, current or stale, never blocks a fresh one.
    green_commit(&global, &turn, "# fixture two\n", SAFETY_SUBJECT);
    let y = turn.head_of(TRUNK);
    checkpoint_success(
        &global,
        &turn,
        &y,
        &targets,
        "the generation-two re-checkpoint",
    );

    // A misleading backup planted under the checkout: newer-looking bytes
    // than generation two, at the newest mtime in the tree.
    let backup = turn.root.join("README.md.bak");
    let newest = turn.root.join("latest/README.md");
    fs::write(&backup, "# fixture three - newer looking\n").expect("plant the .bak backup");
    fs::create_dir_all(newest.parent().expect("the latest directory's parent"))
        .expect("create the latest directory");
    fs::write(&newest, "# fixture four - newest looking\n").expect("plant the latest copy");
    let backup_before = backup_facts(&backup);
    let newest_before = backup_facts(&newest);

    // Damage, then the supported restore: exactly Y's bytes - never the
    // backup's - and the backup's bytes and mtime survive the invocation.
    fs::write(&readme, "# damaged two\n").expect("damage the target again");
    restore_success(
        &global,
        &turn,
        &y,
        targets.len(),
        "the generation-two restore",
    );
    assert_eq!(
        fs::read(&readme).expect("reread the target"),
        b"# fixture two\n".as_slice(),
        "t125 WCPV-016: the restore selects the recorded generation's bytes, not the backup's"
    );
    assert_backup_unchanged(&backup_before, &backup, "the generation-two restore");
    assert_backup_unchanged(&newest_before, &newest, "the generation-two restore");
    assert_eq!(
        fs::read(&manifest).expect("reread the manifest"),
        expected_manifest(&global, &turn, &y, &targets).into_bytes(),
        "t125 WCPV-016: the successful restore leaves the manifest recording Y"
    );

    // The stale twin: a manifest left recording X while HEAD is Y refuses
    // the stale row with both ids and writes nothing.
    fs::write(&manifest, &x_manifest).expect("restore the generation-one manifest");
    fs::write(&readme, "# damaged stale\n").expect("damage the target for the stale twin");
    let before = tree(&turn);
    let state_before = git_state(&global, &turn);
    let config_before = configs(&global, &turn);
    let stale = turn_isolated(&global, &turn.root, &["restore", "-Item", ITEM]);
    assert_refusal(
        &stale,
        &[
            format!("turn refused; the recorded checkpoint {x} is stale: HEAD is {y}"),
            "  the damage protocol never moves HEAD; this manifest does not name the commit the \
             checkout stands on."
                .to_owned(),
            "  re-run checkpoint -Item item-1 from the current safety commit, or inspect the \
             manifest before restoring."
                .to_owned(),
            "  nothing was written.".to_owned(),
        ],
        "the stale-manifest restore",
    );
    assert_eq!(
        fs::read(&readme).expect("reread the target"),
        b"# damaged stale\n".as_slice(),
        "t125 WCPV-016: the stale refusal keeps the target's damaged bytes"
    );
    assert_eq!(
        fs::read(&manifest).expect("reread the manifest"),
        x_manifest,
        "t125 WCPV-016: the stale refusal leaves the manifest exactly as found"
    );
    assert_eq!(
        changed_paths(&before, &tree(&turn)),
        Vec::<String>::new(),
        "t125 WCPV-016: the stale refusal writes nothing anywhere"
    );
    assert_eq!(
        git_state(&global, &turn),
        state_before,
        "t125 WCPV-016: the stale refusal leaves refs, index, and status untouched"
    );
    assert_eq!(
        configs(&global, &turn),
        config_before,
        "t125 WCPV-016: the stale refusal writes no configuration"
    );
    assert_backup_unchanged(&backup_before, &backup, "the stale refusal");
    assert_backup_unchanged(&newest_before, &newest, "the stale refusal");
}

/// WCPV-017: every route to a backup-copy, latest-file, implicit-index, or
/// arbitrary-revision restore mode refuses before any destructive write - or,
/// for the index-sourced shape, never exists - and the spelling, untracked,
/// stale, safety-subject, and unrelated-work refusals fire by name with
/// nothing written.
#[test]
fn wcpv_017() {
    let (isolation, global) = isolation("wcpv017");

    // --- the positive control, then the source-mode twins on one fixture ---
    let turn = Turn::new("wcpv017a");
    // t-131 (WCP-004): the trace marker joins the ignore list first, so the
    // earned proof set changes no reviewed row.
    declare_trace_marker_ignored(&global, &turn);
    status_control(&global, &turn);
    let readme = turn.root.join("README.md");
    let manifest = manifest_path(&turn);

    green_commit(&global, &turn, "# fixture one\n", SAFETY_SUBJECT);
    let x = turn.head_of(TRUNK);
    checkpoint_success(&global, &turn, &x, &["README.md"], "the control");
    fs::write(&readme, "# damaged one\n").expect("damage the target");
    restore_success(&global, &turn, &x, 1, "the control");

    // Implicit-index is absent: red bytes staged into the index plus a
    // further worktree mutation are fully undone - the restored bytes are
    // the frozen digests, never the staged red bytes.
    fs::write(&readme, "# staged red\n").expect("stage red bytes into the index");
    git_ok(&global, &turn.root, &["add", "--", "README.md"]);
    fs::write(&readme, "# worktree red\n").expect("mutate the worktree past the index");
    restore_success(&global, &turn, &x, 1, "the implicit-index absence");
    assert_eq!(
        fs::read(&readme).expect("reread the target"),
        b"# fixture one\n".as_slice(),
        "t125 WCPV-017: the restore undoes the staged red bytes, not just the worktree's"
    );

    // Backup-copy is absent: a manifest row naming a backup is malformed,
    // and a planted misleading backup is never read.
    let backup = turn.root.join("README.md.bak");
    let newest = turn.root.join("latest/README.md");
    fs::write(&backup, "# newer looking\n").expect("plant the .bak backup");
    fs::create_dir_all(newest.parent().expect("the latest directory's parent"))
        .expect("create the latest directory");
    fs::write(&newest, "# newest looking\n").expect("plant the latest copy");
    let backup_before = backup_facts(&backup);
    let newest_before = backup_facts(&newest);
    let with_backup_row = format!(
        "{}backup README.md.bak\n",
        expected_manifest(&global, &turn, &x, &["README.md"])
    );
    fs::write(&manifest, &with_backup_row).expect("add a backup row to the manifest");
    let before = tree(&turn);
    let state_before = git_state(&global, &turn);
    let config_before = configs(&global, &turn);
    let backup_source = turn_isolated(&global, &turn.root, &["restore", "-Item", ITEM]);
    assert_refusal(
        &backup_source,
        &[
            format!(
                "turn refused; the checkpoint manifest items/{ITEM}-checkpoint-manifest.txt is \
                 malformed: a target row is not the six-field tab-separated shape: backup \
                 README.md.bak"
            ),
            format!(
                "  repair the manifest, or re-run checkpoint -Item {ITEM} from the green state."
            ),
        ],
        "the backup-row manifest restore",
    );
    assert_eq!(
        fs::read(&manifest).expect("reread the manifest"),
        with_backup_row.into_bytes(),
        "t125 WCPV-017: the malformed refusal leaves the manifest bytes as they read"
    );
    assert_eq!(
        changed_paths(&before, &tree(&turn)),
        Vec::<String>::new(),
        "t125 WCPV-017: the backup-source refusal writes nothing anywhere"
    );
    assert_eq!(git_state(&global, &turn), state_before);
    assert_eq!(configs(&global, &turn), config_before);
    assert_backup_unchanged(&backup_before, &backup, "the backup-source refusal");
    assert_backup_unchanged(&newest_before, &newest, "the backup-source refusal");

    // Arbitrary revision is absent: a manifest edited to name an older
    // revision, every row made to agree, still refuses - the stale row - and
    // no revision ever reaches the tool from the command line: every
    // parameter the script does not declare dies at PowerShell binding.
    let base = git_ok(&global, &turn.root, &["rev-parse", &format!("{x}~1")]);
    let base_manifest = manifest_for(&global, &turn, &base, "README.md");
    fs::write(&manifest, &base_manifest).expect("edit the manifest to the older revision");
    fs::write(&readme, "# revision damage\n").expect("damage the target");
    let before = tree(&turn);
    let state_before = git_state(&global, &turn);
    let config_before = configs(&global, &turn);
    let arbitrary = turn_isolated(&global, &turn.root, &["restore", "-Item", ITEM]);
    assert_refusal(
        &arbitrary,
        &[
            format!("turn refused; the recorded checkpoint {base} is stale: HEAD is {x}"),
            "  the damage protocol never moves HEAD; this manifest does not name the commit the \
             checkout stands on."
                .to_owned(),
            "  re-run checkpoint -Item item-1 from the current safety commit, or inspect the \
             manifest before restoring."
                .to_owned(),
            "  nothing was written.".to_owned(),
        ],
        "the arbitrary-revision manifest restore",
    );
    assert_eq!(
        fs::read(&readme).expect("reread the target"),
        b"# revision damage\n".as_slice(),
        "t125 WCPV-017: the arbitrary-revision refusal keeps the damaged bytes"
    );
    assert_eq!(
        fs::read(&manifest).expect("reread the manifest"),
        base_manifest.into_bytes(),
        "t125 WCPV-017: the arbitrary-revision refusal leaves the manifest as found"
    );
    assert_eq!(changed_paths(&before, &tree(&turn)), Vec::<String>::new());
    assert_eq!(git_state(&global, &turn), state_before);
    assert_eq!(configs(&global, &turn), config_before);

    let before = tree(&turn);
    for (parameter, argument, phrase) in [
        (
            "-Revision",
            "zzz",
            "A parameter cannot be found that matches parameter name 'Revision'.",
        ),
        (
            "-Source",
            "zzz",
            "A parameter cannot be found that matches parameter name 'Source'.",
        ),
        (
            "-Backup",
            "zzz",
            "A parameter cannot be found that matches parameter name 'Backup'.",
        ),
        (
            "",
            "zzz",
            "A positional parameter cannot be found that accepts argument 'zzz'.",
        ),
    ] {
        let arguments: Vec<&str> = if parameter.is_empty() {
            vec!["restore", "-Item", ITEM, argument]
        } else {
            vec!["restore", "-Item", ITEM, parameter, argument]
        };
        let binding = turn_isolated(&global, &turn.root, &arguments);
        assert_eq!(
            binding.status.code(),
            Some(1),
            "t125 WCPV-017: the undeclared source parameter {parameter} dies at the shell with \
             exit 1: {}{}",
            String::from_utf8_lossy(&binding.stdout),
            String::from_utf8_lossy(&binding.stderr)
        );
        assert!(
            binding.stdout.is_empty(),
            "t125 WCPV-017: a binding death prints no stdout: {}",
            String::from_utf8_lossy(&binding.stdout)
        );
        assert!(
            !String::from_utf8_lossy(&binding.stderr).contains("turn refused;"),
            "t125 WCPV-017: a binding death is the shell's own, never a tool refusal: {}",
            String::from_utf8_lossy(&binding.stderr)
        );
        assert!(
            lines(&binding.stderr)
                .iter()
                .any(|line| line.contains("turn.ps1:") && line.contains(phrase)),
            "t125 WCPV-017: the binding death names the script and the rejected input: {}",
            String::from_utf8_lossy(&binding.stderr)
        );
    }
    assert_eq!(
        changed_paths(&before, &tree(&turn)),
        Vec::<String>::new(),
        "t125 WCPV-017: a binding death reads and writes no repository byte"
    );

    // Escaping path: the spelling row of the checkpoint validation.
    let before = tree(&turn);
    let escaping = turn_isolated(
        &global,
        &turn.root,
        &["checkpoint", "-Item", ITEM, "-Target", "../outside.txt"],
    );
    assert_refusal(
        &escaping,
        &[
            "turn refused; the declared target ../outside.txt must stay repository-relative"
                .to_owned(),
            "  a target is one repository-relative path: no drive, no leading '/', no backslash, \
             no '..' component, no tab, no newline."
                .to_owned(),
        ],
        "the escaping-path checkpoint",
    );
    assert_eq!(changed_paths(&before, &tree(&turn)), Vec::<String>::new());

    // Untracked-lane target: a path under the declared untracked lanes root
    // is not a tracked regular file at the safety commit.
    let before = tree(&turn);
    let state_before = git_state(&global, &turn);
    let lane = turn_isolated(
        &global,
        &turn.root,
        &[
            "checkpoint",
            "-Item",
            ITEM,
            "-Target",
            "lanes/crate-a/lane.txt",
        ],
    );
    assert_refusal(
        &lane,
        &[
            format!(
                "turn refused; the declared target lanes/crate-a/lane.txt is not a tracked \
             regular file at {x}"
            ),
            "  damage may be planned only for targets the checkpoint commit tracks;".to_owned(),
            "  commit the file first, or drop it from -Target.".to_owned(),
        ],
        "the untracked-lane checkpoint",
    );
    assert_eq!(changed_paths(&before, &tree(&turn)), Vec::<String>::new());
    assert_eq!(git_state(&global, &turn), state_before);

    // --- the stale twin on its own fixture ------------------------------------
    let stale_fixture = Turn::new("wcpv017stale");
    // t-131 (WCP-004): the trace marker joins the ignore list first, so the
    // earned proof set changes no reviewed row.
    declare_trace_marker_ignored(&global, &stale_fixture);
    green_commit(&global, &stale_fixture, "# fixture one\n", SAFETY_SUBJECT);
    let stale_x = stale_fixture.head_of(TRUNK);
    checkpoint_success(
        &global,
        &stale_fixture,
        &stale_x,
        &["README.md"],
        "the stale twin",
    );
    let stale_manifest =
        fs::read(manifest_path(&stale_fixture)).expect("keep the first-generation manifest");
    green_commit(&global, &stale_fixture, "# fixture two\n", SAFETY_SUBJECT);
    let stale_y = stale_fixture.head_of(TRUNK);
    fs::write(manifest_path(&stale_fixture), &stale_manifest)
        .expect("write the older generation's manifest back");
    fs::write(stale_fixture.root.join("README.md"), "# stale damage\n").expect("damage the target");
    let before = tree(&stale_fixture);
    let state_before = git_state(&global, &stale_fixture);
    let config_before = configs(&global, &stale_fixture);
    let stale = turn_isolated(&global, &stale_fixture.root, &["restore", "-Item", ITEM]);
    assert_refusal(
        &stale,
        &[
            format!("turn refused; the recorded checkpoint {stale_x} is stale: HEAD is {stale_y}"),
            "  the damage protocol never moves HEAD; this manifest does not name the commit the \
             checkout stands on."
                .to_owned(),
            "  re-run checkpoint -Item item-1 from the current safety commit, or inspect the \
             manifest before restoring."
                .to_owned(),
            "  nothing was written.".to_owned(),
        ],
        "the stale-checkpoint restore",
    );
    assert_eq!(
        fs::read(stale_fixture.root.join("README.md")).expect("reread the target"),
        b"# stale damage\n".as_slice(),
        "t125 WCPV-017: the stale refusal keeps the damaged bytes"
    );
    assert_eq!(
        changed_paths(&before, &tree(&stale_fixture)),
        Vec::<String>::new()
    );
    assert_eq!(git_state(&global, &stale_fixture), state_before);
    assert_eq!(configs(&global, &stale_fixture), config_before);

    // --- HEAD not the safety commit, at checkpoint and at restore -------------
    let wrong_item = Turn::new("wcpv017wrongitem");
    green_commit(
        &global,
        &wrong_item,
        "# fixture one\n",
        "t-999: checkpoint - not a landing",
    );
    let wrong_head = wrong_item.head_of(TRUNK);
    let before = tree(&wrong_item);
    let config_before = configs(&global, &wrong_item);
    let unsafe_checkpoint = turn_isolated(
        &global,
        &wrong_item.root,
        &["checkpoint", "-Item", ITEM, "-Target", "README.md"],
    );
    assert_refusal(
        &unsafe_checkpoint,
        &[
            format!(
                "turn refused; HEAD {wrong_head} is not the safety commit of {ITEM}: its subject \
                 is t-999: checkpoint - not a landing"
            ),
            format!(
                "  the working rules fix the safety commit's subject exactly '{ITEM}: checkpoint \
                 - not a landing';"
            ),
            "  commit the reviewed green work under that subject, then run checkpoint again."
                .to_owned(),
            "  nothing was written.".to_owned(),
        ],
        "the checkpoint from another item's safety commit",
    );
    assert_eq!(
        changed_paths(&before, &tree(&wrong_item)),
        Vec::<String>::new(),
        "t125 WCPV-017: the unsafe checkpoint writes nothing anywhere"
    );
    assert!(
        !manifest_path(&wrong_item).exists(),
        "t125 WCPV-017: the unsafe checkpoint freezes no manifest"
    );
    assert_eq!(configs(&global, &wrong_item), config_before);

    let unsafe_restore = Turn::new("wcpv017restoresubject");
    // t-131 (WCP-004): the trace marker joins the ignore list first, so the
    // earned proof set changes no reviewed row.
    declare_trace_marker_ignored(&global, &unsafe_restore);
    green_commit(&global, &unsafe_restore, "# fixture one\n", SAFETY_SUBJECT);
    let subject_x = unsafe_restore.head_of(TRUNK);
    checkpoint_success(
        &global,
        &unsafe_restore,
        &subject_x,
        &["README.md"],
        "the restore twin",
    );
    green_commit(
        &global,
        &unsafe_restore,
        "# landed feature\n",
        "landed feature under review",
    );
    let subject_z = unsafe_restore.head_of(TRUNK);
    let landed_manifest = manifest_for(&global, &unsafe_restore, &subject_z, "README.md");
    fs::write(manifest_path(&unsafe_restore), &landed_manifest)
        .expect("edit the manifest to name HEAD itself");
    fs::write(unsafe_restore.root.join("README.md"), "# subject damage\n")
        .expect("damage the target");
    let before = tree(&unsafe_restore);
    let state_before = git_state(&global, &unsafe_restore);
    let config_before = configs(&global, &unsafe_restore);
    let unsafe_head = turn_isolated(&global, &unsafe_restore.root, &["restore", "-Item", ITEM]);
    assert_refusal(
        &unsafe_head,
        &[
            format!(
                "turn refused; HEAD {subject_z} is not the safety commit of {ITEM}: its subject \
                 is landed feature under review"
            ),
            format!(
                "  the working rules fix the safety commit's subject exactly '{ITEM}: checkpoint \
                 - not a landing';"
            ),
            format!(
                "  restore only from a manifest the tool froze from that commit; re-run \
                 checkpoint -Item {ITEM}, or repair the manifest."
            ),
            "  nothing was written.".to_owned(),
        ],
        "the restore whose HEAD is not the safety commit",
    );
    assert_eq!(
        fs::read(unsafe_restore.root.join("README.md")).expect("reread the target"),
        b"# subject damage\n".as_slice(),
        "t125 WCPV-017: the unsafe-HEAD refusal keeps the damaged bytes"
    );
    assert_eq!(
        fs::read(manifest_path(&unsafe_restore)).expect("reread the manifest"),
        landed_manifest.into_bytes(),
        "t125 WCPV-017: the unsafe-HEAD refusal leaves the manifest as found"
    );
    assert_eq!(
        changed_paths(&before, &tree(&unsafe_restore)),
        Vec::<String>::new()
    );
    assert_eq!(git_state(&global, &unsafe_restore), state_before);
    assert_eq!(configs(&global, &unsafe_restore), config_before);

    // --- unrelated unsaved work, found directly and through the pause seam ----
    let dirt = Turn::new("wcpv017dirt");
    // t-131 (WCP-004): the trace marker joins the ignore list first, so the
    // earned proof set changes no reviewed row.
    declare_trace_marker_ignored(&global, &dirt);
    green_commit(&global, &dirt, "# fixture one\n", SAFETY_SUBJECT);
    let dirt_x = dirt.head_of(TRUNK);
    checkpoint_success(&global, &dirt, &dirt_x, &["README.md"], "the dirt twin");
    let dirt_manifest = fs::read(manifest_path(&dirt)).expect("keep the frozen manifest bytes");
    let landlog = dirt.root.join("landlog.md");
    fs::write(dirt.root.join("README.md"), "# dirt damage\n").expect("damage the target");
    fs::write(&landlog, "# landings\nunrelated dirt\n").expect("dirty an unrelated tracked file");
    let before = tree(&dirt);
    let state_before = git_state(&global, &dirt);
    let config_before = configs(&global, &dirt);
    let dirty = turn_isolated(&global, &dirt.root, &["restore", "-Item", ITEM]);
    assert_refusal(
        &dirty,
        &[
            "turn refused; the working tree carries unsaved work unrelated to the declared \
             targets; commit or put aside these entries first:"
                .to_owned(),
            "   M landlog.md".to_owned(),
            "  a restore may undo only the declared targets' damage;".to_owned(),
            "  nothing was written.".to_owned(),
        ],
        "the unrelated-unsaved-work restore",
    );
    assert_eq!(
        fs::read(dirt.root.join("README.md")).expect("reread the target"),
        b"# dirt damage\n".as_slice(),
        "t125 WCPV-017: the unrelated-work refusal keeps the target's damaged bytes"
    );
    assert_eq!(
        fs::read(&landlog).expect("reread the unrelated file"),
        b"# landings\nunrelated dirt\n".as_slice(),
        "t125 WCPV-017: the unrelated-work refusal keeps the unrelated dirt as found"
    );
    assert_eq!(
        fs::read(manifest_path(&dirt)).expect("reread the manifest"),
        dirt_manifest,
        "t125 WCPV-017: the unrelated-work refusal leaves the manifest as found"
    );
    assert_eq!(changed_paths(&before, &tree(&dirt)), Vec::<String>::new());
    assert_eq!(git_state(&global, &dirt), state_before);
    assert_eq!(configs(&global, &dirt), config_before);

    // The same refusal through the pause seam: the verb holds after its
    // validation at `after-validate`, the dirt lands while it holds, and the
    // released verb refuses before its first mutation.
    let paused = Turn::new("wcpv017pause");
    // t-131 (WCP-004): the trace marker joins the ignore list first, so the
    // earned proof set changes no reviewed row.
    declare_trace_marker_ignored(&global, &paused);
    green_commit(&global, &paused, "# fixture one\n", SAFETY_SUBJECT);
    let pause_x = paused.head_of(TRUNK);
    checkpoint_success(&global, &paused, &pause_x, &["README.md"], "the pause twin");
    fs::write(paused.root.join("README.md"), "# pause damage\n").expect("damage the target");
    let marker = isolation.join("wcpv017.marker");
    let release = isolation.join("wcpv017.release");
    let child = paused_restore(&global, &paused, &marker, &release);
    let line = await_marker(
        &marker,
        MarkerLine::Exact("after-validate\n"),
        Duration::from_secs(30),
    )
    .expect("t125 WCPV-017: the pause seam publishes its marker");
    assert_eq!(
        line, "after-validate\n",
        "t125 WCPV-017: the pause marker carries its whole boundary line"
    );
    let landlog = paused.root.join("landlog.md");
    fs::write(&landlog, "# landings\npause dirt\n").expect("plant dirt while the verb holds");
    let before = tree(&paused);
    let state_before = git_state(&global, &paused);
    fs::write(&release, b"release\n").expect("release the paused verb");
    let output = child.wait_with_output().expect("collect the released verb");
    assert_refusal(
        &output,
        &[
            "turn refused; the working tree carries unsaved work unrelated to the declared \
             targets; commit or put aside these entries first:"
                .to_owned(),
            "   M landlog.md".to_owned(),
            "  a restore may undo only the declared targets' damage;".to_owned(),
            "  nothing was written.".to_owned(),
        ],
        "the paused verb's unrelated-unsaved-work refusal",
    );
    assert_eq!(
        fs::read(paused.root.join("README.md")).expect("reread the target"),
        b"# pause damage\n".as_slice(),
        "t125 WCPV-017: the pause-seam refusal keeps the target's damaged bytes"
    );
    assert_eq!(
        fs::read(&landlog).expect("reread the unrelated file"),
        b"# landings\npause dirt\n".as_slice(),
        "t125 WCPV-017: the pause-seam refusal keeps the planted dirt as found"
    );
    assert_eq!(changed_paths(&before, &tree(&paused)), Vec::<String>::new());
    assert_eq!(git_state(&global, &paused), state_before);
}

/// WCPV-018: a damage check interrupted mid-check recovers through the same
/// recorded checkpoint and re-derives every frozen hash, the next damage
/// check needs no new checkpoint, and a checkpoint amended into the landing
/// refuses the stale row.
#[test]
fn wcpv_018() {
    let (isolation, global) = isolation("wcpv018");
    let turn = Turn::new("wcpv018");
    // t-131 (WCP-004): the trace marker joins the ignore list first, so the
    // earned proof set changes no reviewed row.
    declare_trace_marker_ignored(&global, &turn);
    status_control(&global, &turn);

    let readme = turn.root.join("README.md");
    let manifest = manifest_path(&turn);

    // The positive control: green work under the safety subject, frozen,
    // damaged, and restored with t-124's exact success bytes.
    green_commit(&global, &turn, "# fixture one\n", SAFETY_SUBJECT);
    let x = turn.head_of(TRUNK);
    checkpoint_success(&global, &turn, &x, &["README.md"], "the recovery control");
    let frozen_manifest = fs::read(&manifest).expect("keep the frozen manifest bytes");
    fs::write(&readme, "# damaged one\n").expect("damage the target");
    restore_success(&global, &turn, &x, 1, "the recovery control");
    assert_eq!(
        fs::read(&readme).expect("reread the target"),
        b"# fixture one\n".as_slice(),
        "t125 WCPV-018: the control returns the target to its green bytes"
    );

    // Interrupt a damage check mid-check: the verb holds at the
    // `after-validate` boundary, and the kill leaves everything as found.
    fs::write(&readme, "# interrupted\n").expect("damage the target again");
    let marker = isolation.join("wcpv018.marker");
    let release = isolation.join("wcpv018.release");
    let mut child = paused_restore(&global, &turn, &marker, &release);
    let line = await_marker(
        &marker,
        MarkerLine::Exact("after-validate\n"),
        Duration::from_secs(30),
    )
    .expect("t125 WCPV-018: the pause seam publishes its marker");
    assert_eq!(
        line, "after-validate\n",
        "t125 WCPV-018: the pause marker carries its whole boundary line"
    );
    assert_eq!(
        fs::read(&readme).expect("reread the target at the boundary"),
        b"# interrupted\n".as_slice(),
        "t125 WCPV-018: the interrupted verb has written nothing at the boundary"
    );
    child.kill().expect("kill the interrupted restore");
    let _ = child.wait();
    assert_eq!(
        fs::read(&readme).expect("reread the target after the kill"),
        b"# interrupted\n".as_slice(),
        "t125 WCPV-018: the killed verb wrote nothing"
    );

    // Recovery: the same recorded checkpoint restores the damage and
    // re-derives every frozen hash.
    restore_success(&global, &turn, &x, 1, "the recovery restore");
    assert_eq!(
        fs::read(&readme).expect("reread the target"),
        b"# fixture one\n".as_slice(),
        "t125 WCPV-018: the recovery restore re-derives the frozen bytes"
    );

    // The next damage check needs no new checkpoint: HEAD never moved, so
    // the same manifest stays current and the second restore verifies again.
    fs::write(&readme, "# second damage\n").expect("damage the target a third time");
    restore_success(&global, &turn, &x, 1, "the second damage check");
    assert_eq!(
        fs::read(&manifest).expect("reread the manifest"),
        frozen_manifest,
        "t125 WCPV-018: the repeated damage check needed no new checkpoint"
    );

    // The amend twin: the checkpoint amended into the landing moves HEAD,
    // and the manifest still naming the pre-amend safety commit refuses the
    // stale row - the recorded checkpoint is never applied onto the landing.
    git_ok(
        &global,
        &turn.root,
        &["commit", "--amend", "-m", "landed: fixture feature folded"],
    );
    let z = turn.head_of(TRUNK);
    assert_ne!(z, x, "t125 WCPV-018: the amend moved HEAD");
    fs::write(&readme, "# amend damage\n").expect("damage the target for the amend twin");
    let before = tree(&turn);
    let state_before = git_state(&global, &turn);
    let config_before = configs(&global, &turn);
    let amended = turn_isolated(&global, &turn.root, &["restore", "-Item", ITEM]);
    assert_refusal(
        &amended,
        &[
            format!("turn refused; the recorded checkpoint {x} is stale: HEAD is {z}"),
            "  the damage protocol never moves HEAD; this manifest does not name the commit the \
             checkout stands on."
                .to_owned(),
            "  re-run checkpoint -Item item-1 from the current safety commit, or inspect the \
             manifest before restoring."
                .to_owned(),
            "  nothing was written.".to_owned(),
        ],
        "the amend-into-landing restore",
    );
    assert_eq!(
        fs::read(&readme).expect("reread the target"),
        b"# amend damage\n".as_slice(),
        "t125 WCPV-018: the amend refusal keeps the damaged bytes"
    );
    assert_eq!(
        fs::read(&manifest).expect("reread the manifest"),
        frozen_manifest,
        "t125 WCPV-018: the amend refusal leaves the manifest naming the safety commit"
    );
    assert_eq!(changed_paths(&before, &tree(&turn)), Vec::<String>::new());
    assert_eq!(git_state(&global, &turn), state_before);
    assert_eq!(configs(&global, &turn), config_before);
}

// --- SVEV-014: in-place pause markers through the whole-line wait ------------

/// The longest a case's waiter waits for the line its started command
/// publishes.
const MARKER_LONG: Duration = Duration::from_secs(60);
/// The short wait made before the paused command starts: nothing publishes
/// then, so the planted bytes are all the waiter can see.
const MARKER_SHORT: Duration = Duration::from_millis(300);
/// Every pause seam's own timeout, far beyond any case's release.
const SEAM_TIMEOUT_MILLIS: &str = "60000";

/// A paused command's release file, written on normal cleanup and on
/// unwinding alike, so a failing case never strands a paused command.
struct Release {
    path: PathBuf,
    written: bool,
}

impl Release {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            written: false,
        }
    }

    fn now(&mut self) {
        if !self.written {
            fs::write(&self.path, b"release\n").expect("SVEV-014: write the release file");
            self.written = true;
        }
    }
}

impl Drop for Release {
    fn drop(&mut self) {
        if !self.written {
            let _ = fs::write(&self.path, b"release\n");
        }
    }
}

/// A started paused command; dropped unfinished, it is killed and reaped
/// before any tree it works in is removed.
struct Started(Option<Child>);

impl Started {
    fn finish(mut self, case: &str) -> Output {
        self.0
            .take()
            .expect("SVEV-014: the started command is still owned")
            .wait_with_output()
            .unwrap_or_else(|error| {
                panic!("SVEV-014 {case}: collect the released command: {error}")
            })
    }
}

impl Drop for Started {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// The Engine under test (`rtm-qa`, fault injection on) in `directory`:
/// scrubbed, Git isolated to `global`, and only the case's settings added.
fn engine(
    directory: &Path,
    global: &Path,
    args: &[&str],
    vars: &[(&str, &OsStr)],
) -> std::process::Command {
    let mut invocation = command(ratmac_qa::engine_bin!(), directory);
    invocation
        .args(args)
        .env("GIT_CONFIG_GLOBAL", global)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    for (key, value) in vars {
        invocation.env(key, value);
    }
    invocation
}

/// One Engine command that must succeed while a case is set up.
fn engine_ok(case: &str, directory: &Path, global: &Path, args: &[&str]) {
    let output = engine(directory, global, args, &[])
        .output()
        .expect("SVEV-014: invoke rtm");
    assert!(
        output.status.success(),
        "SVEV-014 {case}: rtm {args:?} must succeed while setting up: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The one Run a fresh Engine root holds.
fn only_run(root: &Path) -> String {
    let runs: Vec<String> = fs::read_dir(root.join(".ratmac/runs"))
        .expect("SVEV-014: list the Run roster")
        .map(|entry| entry.expect("SVEV-014: read a roster entry"))
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        runs.len(),
        1,
        "SVEV-014: a fresh Engine root holds one Run: {runs:?}"
    );
    runs[0].clone()
}

/// Every spelling of `path` a command may print, longest first.
fn root_spellings(path: &Path) -> Vec<String> {
    let mut texts = vec![path.display().to_string()];
    if let Ok(canonical) = fs::canonicalize(path) {
        let text = canonical.display().to_string();
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            texts.push(rest.to_owned());
        }
        texts.push(text);
    }
    let mut forms: Vec<String> = Vec::new();
    for text in texts {
        for form in [text.replace('\\', "/"), text] {
            if !forms.contains(&form) {
                forms.push(form);
            }
        }
    }
    forms.sort_by_key(|form| std::cmp::Reverse(form.len()));
    forms
}

/// A released command's verdict - exit code, stdout, and stderr - with each
/// fixture root shown as `<root>` and each `also` pair replaced.
fn verdict(output: &Output, roots: &[&Path], also: &[(&str, &str)]) -> String {
    let mut text = format!(
        "exit: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for root in roots {
        for form in root_spellings(root) {
            text = text.replace(&form, "<root>");
        }
    }
    for (from, to) in also {
        text = text.replace(from, to);
    }
    text
}

/// Plant `planted` at the marker path, when given, and prove t125's waiter
/// reports it as not yet published before the paused command starts.
fn plant_before_start(case: &str, marker: &Path, expected: MarkerLine<'_>, planted: Option<&[u8]>) {
    if let Some(bytes) = planted {
        fs::write(marker, bytes).expect("SVEV-014: plant the marker bytes");
        assert_eq!(
            await_marker(marker, expected, MARKER_SHORT),
            Err(MarkerWait::NotYetPublished(Some(bytes.to_vec()))),
            "SVEV-014 {case}: before the paused command starts, t125's await_marker must \
             report the planted {:?} as not yet published",
            String::from_utf8_lossy(bytes)
        );
    }
}

/// Wait for the started command's whole line and check it is the case's.
fn whole_line(case: &str, marker: &Path, expected: MarkerLine<'_>) {
    let line = await_marker(marker, expected, MARKER_LONG).unwrap_or_else(|wait| {
        panic!("SVEV-014 {case}: the paused command's whole line never arrived: {wait}")
    });
    let whole = line.ends_with('\n') && line.matches('\n').count() == 1;
    let accepted = match expected {
        MarkerLine::Exact(text) => line == text,
        MarkerLine::Prefix(prefix) => line.starts_with(prefix),
    };
    assert!(
        whole && accepted,
        "SVEV-014 {case}: the waiter acts only on the expected whole line, got {line:?}"
    );
}

/// One paused Engine command: plant (when asked) and check the short wait,
/// start the command with the case's settings, act on its whole line,
/// release it, and return its verdict with every fixture root replaced.
#[allow(clippy::too_many_arguments)]
fn paused_engine(
    case: &str,
    directory: &Path,
    global: &Path,
    args: &[&str],
    vars: &[(&str, &OsStr)],
    marker: &Path,
    mut release: Release,
    expected: MarkerLine<'_>,
    planted: Option<&[u8]>,
    roots: &[&Path],
) -> String {
    plant_before_start(case, marker, expected, planted);
    let started = Started(Some(
        engine(directory, global, args, vars)
            .spawn()
            .unwrap_or_else(|error| panic!("SVEV-014 {case}: start the paused command: {error}")),
    ));
    whole_line(case, marker, expected);
    release.now();
    let output = started.finish(case);
    assert!(
        output.status.success(),
        "SVEV-014 {case}: the released command succeeds: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    verdict(&output, roots, &[])
}

/// The blocker the hold case names: one complete issue folder.
const HOLD_BLOCKER: &str = ".arca/issue/i-777-blocker";

/// A started Run sitting in `build` with one executing ticket, one unproven
/// residual, and one complete blocker issue (the t050 fixture's shape).
fn hold_fixture(case: &str, root: &Path, global: &Path) -> String {
    for dir in [
        ".arca/ticket",
        ".arca/residual",
        HOLD_BLOCKER,
        ".ratmac",
        "src",
    ] {
        fs::create_dir_all(root.join(dir)).expect("SVEV-014: create the hold fixture");
    }
    fs::write(root.join("src/lib.rs"), "pub fn work() {}\n").expect("SVEV-014: write source");
    for name in [
        "index.md",
        "spec.md",
        "design.md",
        "test-plan.md",
        "ubi-lang.md",
    ] {
        fs::write(
            root.join(HOLD_BLOCKER).join(name),
            format!("# {name}\n\n```yaml\nstatus: \"pending\"\n```\n"),
        )
        .expect("SVEV-014: write the blocker issue");
    }
    fs::write(
        root.join(".ratmac/ratmac.toml"),
        "[roots]\n\
         ticket = \".arca/ticket\"\n\
         issue = \".arca/issue\"\n\
         residual = \".arca/residual\"\n\n\
         [states.intake]\nprompt = \"Integrate the issues.\"\n\n\
         [states.build]\nprompt = \"Build the ticket.\"\n\n\
         [states.build-review]\nprompt = \"Review the ticket.\"\n\n\
         [[transitions]]\nfrom = \"intake\"\nto = \"build\"\n\n\
         [[transitions]]\nfrom = \"build\"\nto = \"intake\"\nblocked-route = true\n\n\
         [[transitions]]\nfrom = \"build\"\nto = \"build-review\"\n",
    )
    .expect("SVEV-014: write the machine class");
    fs::write(
        root.join(".arca/ticket/t-900.md"),
        "---\nticket-id: t-900\nresidual-ids:\n  - \"res-900\"\n\
         planned-test-refs:\n  - \"PT-900-01\"\nstatus: \"executing\"\n---\n\n\
         # Ticket: t-900\n\n## Merge Gate\n\n- Quality: `cargo --version` passes.\n",
    )
    .expect("SVEV-014: write the ticket");
    fs::write(
        root.join(".arca/residual/res-900.md"),
        "# Residual Record\n\n```yaml\nresidual-id: \"res-900\"\n\
         goal-requirement-ref: \"DEMO-001\"\nstatus: \"missing\"\n```\n",
    )
    .expect("SVEV-014: write the residual");
    engine_ok(case, root, global, &["start"]);
    let run = only_run(root);
    engine_ok(case, root, global, &["step", "--run", &run]);
    run
}

/// The t-136 start runbook: a committed root the lock-hold case starts in.
const LOCK_RUNBOOK: &str =
    "\n[roots]\nwork = \"work\"\n\n[states.plan]\nprompt = \"Plan the cycle.\"\n\n\
     [states.review]\nprompt = \"Review the cycle.\"\n\n[states.done]\nprompt = \"Done.\"\n\n\
     [[transitions]]\nfrom = \"plan\"\nto = \"review\"\n\n\
     [[transitions]]\nfrom = \"review\"\nto = \"done\"\n";

/// A fresh isolated Git repository (the t-136 shape), its runbook and work
/// leaf committed when `committed`.
fn git_fixture(global: &Path, label: &str, committed: bool) -> TempTree {
    let tree = TempTree::new(&format!("t125-{label}")).expect("SVEV-014: own a Git fixture");
    for args in [
        &["init", "-q"][..],
        &["symbolic-ref", "HEAD", "refs/heads/main"][..],
        &["config", "user.email", "qa@ratmac.test"][..],
        &["config", "user.name", "ratmac qa"][..],
        &["config", "core.autocrlf", "false"][..],
        &["config", "commit.gpgsign", "false"][..],
    ] {
        git_ok(global, tree.path(), args);
    }
    if committed {
        fs::create_dir_all(tree.join(".ratmac")).expect("SVEV-014: create the Engine tree");
        fs::create_dir_all(tree.join("work")).expect("SVEV-014: create the work root");
        fs::write(tree.join(".ratmac/ratmac.toml"), LOCK_RUNBOOK)
            .expect("SVEV-014: write the runbook");
        fs::write(tree.join("work/item.md"), "Contributor-owned bytes.\n")
            .expect("SVEV-014: write the work leaf");
        git_ok(global, tree.path(), &["add", "--all"]);
        git_ok(
            global,
            tree.path(),
            &["commit", "-q", "-m", "t-125 SVEV-014 fixture base"],
        );
    }
    tree
}

/// The t-134 parent machine: `nest` runs the declared child program.
const NESTED_PARENT: &str =
    "\nprogram = \"alpha\"\n\n[states.plan]\nprompt = \"Plan the parent work.\"\n\n\
     [states.nest]\nprompt = \"Run the declared nested child program.\"\n\
     guards = [{ kind = \"nested_run\", program = \"beta\", root = \"../child\" }]\n\n\
     [states.done]\nprompt = \"The parent work is done.\"\n\n\
     [[transitions]]\nfrom = \"plan\"\nto = \"nest\"\n\n\
     [[transitions]]\nfrom = \"nest\"\nto = \"done\"\n";

/// The t-134 child whose Run is terminal the moment the endpoint admits it.
const NESTED_CHILD: &str =
    "\nprogram = \"beta\"\n\n[states.final]\nprompt = \"The child work is already complete.\"\n";

/// One t-134 fixture project under `sandbox`.
fn nested_project(sandbox: &TempTree, name: &str, runbook: &str) -> PathBuf {
    let dir = sandbox.join(name);
    for sub in [".ratmac", ".arca/goal", "src", "work"] {
        fs::create_dir_all(dir.join(sub)).expect("SVEV-014: create the project tree");
    }
    fs::write(dir.join(".arca/goal/spec.md"), "# Fixture goal\n").expect("SVEV-014: write goal");
    fs::write(dir.join(".ratmac/ratmac.toml"), runbook).expect("SVEV-014: write runbook");
    fs::write(dir.join("src/lib.rs"), "pub fn fixture() {}\n").expect("SVEV-014: write source");
    dir
}

/// A parent Run sitting in `nest` beside its terminal child project.
fn nested_fixture(case: &str, global: &Path) -> (TempTree, PathBuf, String) {
    let sandbox = TempTree::new("t125-svev014-nested").expect("SVEV-014: own the sandbox");
    let parent = nested_project(&sandbox, "parent", NESTED_PARENT);
    nested_project(&sandbox, "child", NESTED_CHILD);
    engine_ok(case, &parent, global, &["start"]);
    let run = only_run(&parent);
    engine_ok(case, &parent, global, &["step", "--run", &run]);
    (sandbox, parent, run)
}

/// The turn tool's restore paused at `after-validate` (checkpoint first).
fn turn_case(planted: Option<&[u8]>) -> String {
    const CASE: &str = "turn tool after-validate";
    let (isolation, global) = isolation("svev014-turn");
    let turn = Turn::new("svev014-turn");
    declare_trace_marker_ignored(&global, &turn);
    green_commit(&global, &turn, "# fixture one\n", SAFETY_SUBJECT);
    let checkpoint = turn.head_of(TRUNK);
    checkpoint_success(
        &global,
        &turn,
        &checkpoint,
        &["README.md"],
        "the SVEV-014 turn case",
    );
    fs::write(turn.root.join("README.md"), "# pause damage\n").expect("SVEV-014: damage");
    let marker = isolation.join("svev014.marker");
    let mut release = Release::new(isolation.join("svev014.release"));
    let expected = MarkerLine::Exact("after-validate\n");
    plant_before_start(CASE, &marker, expected, planted);
    let started = Started(Some(paused_restore(&global, &turn, &marker, &release.path)));
    whole_line(CASE, &marker, expected);
    release.now();
    let output = started.finish(CASE);
    assert!(
        output.status.success(),
        "SVEV-014 {CASE}: the released restore succeeds: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    verdict(
        &output,
        &[&turn.root, isolation.path()],
        &[(checkpoint.as_str(), "<checkpoint>")],
    )
}

/// `rtm hold` paused at the in-place `before-state-write` barrier.
fn hold_case(planted: Option<&[u8]>) -> String {
    const CASE: &str = "hold barrier before-state-write";
    let (isolation, global) = isolation("svev014-hold");
    let fixture = TempTree::new("t125-svev014-hold").expect("SVEV-014: own the hold fixture");
    let run = hold_fixture(CASE, fixture.path(), &global);
    let marker = isolation.join("hold.marker");
    let release = Release::new(isolation.join("hold.release"));
    let release_path = release.path.clone();
    let confirm = format!("hold {run}");
    paused_engine(
        CASE,
        fixture.path(),
        &global,
        &[
            "hold",
            "--run",
            &run,
            "--blocker",
            HOLD_BLOCKER,
            "--confirm",
            &confirm,
        ],
        &[
            ("RATMAC_TEST_HOLD_BARRIER", OsStr::new("before-state-write")),
            ("RATMAC_TEST_HOLD_BARRIER_MARKER", marker.as_os_str()),
            ("RATMAC_TEST_HOLD_BARRIER_RELEASE", release_path.as_os_str()),
            (
                "RATMAC_TEST_HOLD_BARRIER_TIMEOUT_MILLIS",
                OsStr::new(SEAM_TIMEOUT_MILLIS),
            ),
        ],
        &marker,
        release,
        MarkerLine::Exact("holding before before-state-write\n"),
        planted,
        &[fixture.path(), isolation.path()],
    )
}

/// `rtm start` holding the root lock.
fn lock_case(planted: Option<&[u8]>) -> String {
    const CASE: &str = "lock hold root";
    let (isolation, global) = isolation("svev014-lock");
    let repo = git_fixture(&global, "svev014-lock", true);
    let partial = format!(
        "holding root lock {}",
        slashed(
            &fs::canonicalize(repo.path())
                .expect("SVEV-014: canonicalize the lock fixture")
                .join(".ratmac/locks/root.lock")
        )
    );
    let planted = planted.map(|bytes| {
        if bytes.is_empty() {
            bytes
        } else {
            partial.as_bytes()
        }
    });
    let marker = isolation.join("lock.marker");
    let release = Release::new(isolation.join("lock.release"));
    let release_path = release.path.clone();
    paused_engine(
        CASE,
        repo.path(),
        &global,
        &["start"],
        &[
            ("RATMAC_TEST_LOCK_HOLD", OsStr::new("root")),
            ("RATMAC_TEST_LOCK_MARKER", marker.as_os_str()),
            ("RATMAC_TEST_LOCK_RELEASE", release_path.as_os_str()),
            (
                "RATMAC_TEST_LOCK_HOLD_TIMEOUT_MILLIS",
                OsStr::new(SEAM_TIMEOUT_MILLIS),
            ),
        ],
        &marker,
        release,
        MarkerLine::Prefix("holding root lock "),
        planted,
        &[repo.path(), isolation.path()],
    )
}

/// `rtm init` paused at the policy step `verify`.
fn policy_case(planted: Option<&[u8]>) -> String {
    const CASE: &str = "policy pause verify";
    let (isolation, global) = isolation("svev014-policy");
    let repo = git_fixture(&global, "svev014-policy", false);
    let marker = isolation.join("policy.marker");
    let release = Release::new(isolation.join("policy.release"));
    let release_path = release.path.clone();
    paused_engine(
        CASE,
        repo.path(),
        &global,
        &["init"],
        &[
            ("RATMAC_TEST_POLICY_PAUSE", OsStr::new("verify")),
            ("RATMAC_TEST_POLICY_PAUSE_MARKER", marker.as_os_str()),
            ("RATMAC_TEST_POLICY_PAUSE_RELEASE", release_path.as_os_str()),
            (
                "RATMAC_TEST_POLICY_PAUSE_TIMEOUT_MILLIS",
                OsStr::new(SEAM_TIMEOUT_MILLIS),
            ),
        ],
        &marker,
        release,
        MarkerLine::Exact("verify\n"),
        planted,
        &[repo.path(), isolation.path()],
    )
}

/// The parent `rtm step` paused after recording its nested entry.
fn nested_case(planted: Option<&[u8]>) -> String {
    const CASE: &str = "nested pause after-record";
    let (isolation, global) = isolation("svev014-nested");
    let (sandbox, parent, run) = nested_fixture(CASE, &global);
    let marker = isolation.join("record.marker");
    let release = Release::new(isolation.join("record.release"));
    let release_path = release.path.clone();
    paused_engine(
        CASE,
        &parent,
        &global,
        &["step", "--run", &run],
        &[
            ("RATMAC_TEST_NESTED_PAUSE", OsStr::new("after-record")),
            ("RATMAC_TEST_NESTED_PAUSE_MARKER", marker.as_os_str()),
            ("RATMAC_TEST_NESTED_PAUSE_RELEASE", release_path.as_os_str()),
            (
                "RATMAC_TEST_NESTED_PAUSE_TIMEOUT_MILLIS",
                OsStr::new(SEAM_TIMEOUT_MILLIS),
            ),
        ],
        &marker,
        release,
        MarkerLine::Exact("after-record\n"),
        planted,
        &[sandbox.path(), isolation.path()],
    )
}

/// The child endpoint paused right after admitting the child Run.
fn endpoint_case(planted: Option<&[u8]>) -> String {
    const CASE: &str = "endpoint pause after-admit";
    let (isolation, global) = isolation("svev014-endpoint");
    let (sandbox, parent, run) = nested_fixture(CASE, &global);
    let marker = isolation.join("admit.marker");
    let release = Release::new(isolation.join("admit.release"));
    let release_path = release.path.clone();
    paused_engine(
        CASE,
        &parent,
        &global,
        &["step", "--run", &run],
        &[
            ("RATMAC_TEST_ENDPOINT_PAUSE", OsStr::new("after-admit")),
            ("RATMAC_TEST_ENDPOINT_PAUSE_MARKER", marker.as_os_str()),
            (
                "RATMAC_TEST_ENDPOINT_PAUSE_RELEASE",
                release_path.as_os_str(),
            ),
            (
                "RATMAC_TEST_ENDPOINT_PAUSE_TIMEOUT_MILLIS",
                OsStr::new(SEAM_TIMEOUT_MILLIS),
            ),
        ],
        &marker,
        release,
        MarkerLine::Exact("after-admit\n"),
        planted,
        &[sandbox.path(), isolation.path()],
    )
}

/// The host verifier t-132 pins; the recover case's keys and its one
/// decision signature are made with it.
const KEYGEN: &str = "C:/WINDOWS/System32/OpenSSH/ssh-keygen.exe";

/// The enrolled bench of the recover case: `lead` approves, two reviews.
const RECOVER_REVIEWERS: [&str; 3] = ["a", "b", "lead"];

fn sha256_text(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// The one spelling paths carry in the recover case's arguments.
fn slashed(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Unsigned 64-bit big-endian length, then the bytes (the t-129 framing).
fn framed(body: &mut Vec<u8>, field: &[u8]) {
    body.extend_from_slice(&(field.len() as u64).to_be_bytes());
    body.extend_from_slice(field);
}

/// One host `ssh-keygen` call that must succeed.
fn keygen(dir: &Path, args: &[&str]) {
    assert!(
        Path::new(KEYGEN).is_file(),
        "SVEV-014: the host verifier {KEYGEN} must exist"
    );
    let output = command(KEYGEN, dir)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("SVEV-014: run ssh-keygen {args:?}: {error}"));
    assert!(
        output.status.success(),
        "SVEV-014: ssh-keygen {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// One `rtm` command line run in-process through the library entry, so the
/// installed recovery trust participates: exit code, stdout, message.
fn in_process(root: &Path, args: &[String]) -> (i32, String, String) {
    let mut full = vec!["rtm".to_owned()];
    full.extend(args.iter().cloned());
    let mut stdout = Vec::new();
    let (code, message) = match ratmac::cli::run_from(full, root, &mut stdout) {
        Ok(code) => (code, String::new()),
        Err(error) => (error.exit_code(), error.to_string()),
    };
    (code, String::from_utf8_lossy(&stdout).into_owned(), message)
}

/// One in-process command that must succeed while the recover case is set up.
fn in_process_ok(root: &Path, args: &[&str]) -> String {
    let owned: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
    let (code, stdout, message) = in_process(root, &owned);
    assert_eq!(
        code, 0,
        "SVEV-014 recover pause: rtm {args:?} must succeed while setting up: {stdout}{message}"
    );
    stdout
}

/// The recovery entry runs in this process, so its Git descendants need the
/// same empty global configuration as the child-process cases. The hook
/// lane above serializes this case; restore both settings while unwinding.
struct InProcessGit(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl InProcessGit {
    fn new(global: &Path) -> Self {
        let saved = ["GIT_CONFIG_GLOBAL", "GIT_CONFIG_NOSYSTEM"]
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect();
        std::env::set_var("GIT_CONFIG_GLOBAL", global);
        std::env::set_var("GIT_CONFIG_NOSYSTEM", "1");
        Self(saved)
    }
}

impl Drop for InProcessGit {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// The t-132 machine body: intake -> build -> done with blocked self-routes.
const RECOVER_BODY: &str = "[roots]\nissue = \"work\"\nticket = \"tickets\"\n\n\
     [states.intake]\nprompt = \"Intake the request.\"\n\n\
     [states.build]\nprompt = \"Build the change.\"\n\n\
     [states.done]\nprompt = \"Ship the change.\"\n\n\
     [[transitions]]\nfrom = \"intake\"\nto = \"build\"\n\n\
     [[transitions]]\nfrom = \"build\"\nto = \"done\"\n\n\
     [[transitions]]\nfrom = \"intake\"\nto = \"intake\"\nblocked-route = true\n\n\
     [[transitions]]\nfrom = \"build\"\nto = \"build\"\nblocked-route = true\n";

/// `rtm recover` paused before its amendment rename: the t-132 governed
/// fixture (enrollment installed through the t-129 trust seam), a drifted
/// rule, and one decision signed by the Approver. The Engine runs
/// in-process, so the case holds the process's hook lane.
fn recover_case(planted: Option<&[u8]>) -> String {
    const CASE: &str = "recover pause before-rename";
    let _lane = ratmac_qa::support::hook_lane();
    let (isolation, global) = isolation("svev014-recover");
    let _git = InProcessGit::new(&global);
    let keys = TempTree::new("t125-svev014-keys").expect("SVEV-014: own the key tree");
    let mut reviewers: Vec<(String, String)> = Vec::new();
    for name in RECOVER_REVIEWERS {
        let path = keys.join(name);
        keygen(
            keys.path(),
            &[
                "-t",
                "ed25519",
                "-f",
                &path.to_string_lossy(),
                "-N",
                "",
                "-C",
                name,
            ],
        );
        let public = fs::read_to_string(keys.join(format!("{name}.pub")))
            .expect("SVEV-014: read the public key");
        reviewers.push((name.to_owned(), public.trim().to_owned()));
    }
    let mut signers = String::new();
    for (label, public) in &reviewers {
        let parts: Vec<&str> = public.split_whitespace().collect();
        signers.push_str(&format!("{label} {} {}\n", parts[0], parts[1]));
    }
    keys.write("allowed_signers", &signers);
    let signers_path = keys.join("allowed_signers");
    let mut policy = b"ratmac-policy-v1".to_vec();
    policy.push(0);
    framed(&mut policy, b"lead");
    framed(&mut policy, b"2");
    framed(&mut policy, b"false");
    for (label, public) in &reviewers {
        framed(&mut policy, label.as_bytes());
        framed(&mut policy, public.as_bytes());
    }
    let policy_digest = sha256_text(&policy);
    let verifier_digest = sha256_text(&fs::read(KEYGEN).expect("SVEV-014: read the verifier"));
    let signers_digest = sha256_text(signers.as_bytes());

    let repo = git_fixture(&global, "svev014-recover", false);
    repo.write("work/content.txt", "Contributor bytes.\n");
    repo.write(
        "work/blocker.txt",
        "Opaque blocker; its content is not a gate.\n",
    );
    repo.write(
        "tickets/t-100.md",
        "---\nticket-id: \"t-100\"\nstatus: \"approved\"\n---\n",
    );
    repo.write("goal/spec.md", "# Goal\n\nThe frozen goal baseline.\n");
    repo.write(".ratmac/ratmac.toml", RECOVER_BODY);
    git_ok(&global, repo.path(), &["add", "--all"]);
    git_ok(
        &global,
        repo.path(),
        &["commit", "-q", "-m", "fixture baseline"],
    );
    let root = repo.path().to_path_buf();
    let scope = slashed(&fs::canonicalize(&root).expect("SVEV-014: canonicalize the fixture"));
    let mut manifest = format!(
        "project-scope = \"{scope}\"\npolicy-digest = \"{policy_digest}\"\napprover = \"lead\"\n\
         minimum-reviews = 2\napprover-may-review = false\nverifier-path = \"{KEYGEN}\"\n\
         verifier-digest = \"{verifier_digest}\"\nsigners-path = \"{}\"\n\
         signers-digest = \"{signers_digest}\"\n[builder]\nactor = {:?}\nkeys = []\n",
        slashed(&signers_path),
        if cfg!(windows) {
            format!(
                "{}\\{}",
                std::env::var("USERDOMAIN").unwrap_or_default(),
                std::env::var("USERNAME").unwrap_or_default()
            )
        } else {
            std::env::var("USER").unwrap_or_else(|_| "builder".to_owned())
        }
    );
    let mut review = format!(
        "\n[review]\nmanifest = \"{}\"\npolicy-digest = \"{policy_digest}\"\n\
         verifier-digest = \"{verifier_digest}\"\nsigners-digest = \"{signers_digest}\"\n\
         minimum-reviews = 2\napprover = \"lead\"\napprover-may-review = false\nwork = [\"work\"]\n",
        slashed(&keys.join("enrollment-manifest.toml"))
    );
    for (label, public) in &reviewers {
        manifest.push_str(&format!(
            "\n[[reviewers]]\nlabel = \"{label}\"\nkey = \"{public}\"\n"
        ));
        review.push_str(&format!(
            "\n[[review.reviewers]]\nlabel = \"{label}\"\nkey = \"{public}\"\n"
        ));
    }
    keys.write("enrollment-manifest.toml", manifest.as_bytes());
    let _trust = ratmac::review::test::install(
        &root.join(".ratmac"),
        ratmac::review::test::TestTrust {
            manifest_bytes: manifest.into_bytes(),
            signers_path,
            verifier_path: PathBuf::from(KEYGEN),
            verifier_sha256: verifier_digest,
        },
    );
    let governed = format!("{RECOVER_BODY}{review}");
    repo.write(".ratmac/ratmac.toml", &governed);
    git_ok(&global, repo.path(), &["add", "--all"]);
    git_ok(
        &global,
        repo.path(),
        &["commit", "-q", "-m", "governed fixture baseline"],
    );
    in_process_ok(&root, &["start"]);
    let run = only_run(&root);
    // The aged-record rule correction: the runbook drifts by one comment.
    repo.write(
        ".ratmac/ratmac.toml",
        format!("{governed}# corrected by the archived-record fix: the aged rule was wrong.\n"),
    );
    let intent = keys.join("recover-intent.bin");
    in_process_ok(
        &root,
        &[
            "intent",
            "--run",
            &run,
            "--recover",
            "--out",
            &slashed(&intent),
        ],
    );
    keygen(
        keys.path(),
        &[
            "-Y",
            "sign",
            "-f",
            &keys.join("lead").to_string_lossy(),
            "-n",
            "ratmac-recover-v1",
            &intent.to_string_lossy(),
        ],
    );
    let decision = slashed(&keys.join("recover-intent.bin.sig"));

    let marker = isolation.join("recover.marker");
    let expected = MarkerLine::Exact("before-rename\n");
    plant_before_start(CASE, &marker, expected, planted);
    let release_path = isolation.join("recover.release");
    std::env::set_var("RATMAC_TEST_RECOVER_PAUSE", "before-rename");
    std::env::set_var("RATMAC_TEST_RECOVER_PAUSE_MARKER", &marker);
    std::env::set_var("RATMAC_TEST_RECOVER_PAUSE_RELEASE", &release_path);
    std::env::set_var(
        "RATMAC_TEST_RECOVER_PAUSE_TIMEOUT_MILLIS",
        SEAM_TIMEOUT_MILLIS,
    );
    let args: Vec<String> = [
        "recover",
        "--run",
        &run,
        "--decision",
        &decision,
        "--confirm",
    ]
    .iter()
    .map(|arg| (*arg).to_owned())
    .chain([format!("recover {run}")])
    .collect();
    let (code, stdout, message) = std::thread::scope(|scope| {
        let mut release = Release::new(release_path.clone());
        let worker = scope.spawn(|| in_process(&root, &args));
        whole_line(CASE, &marker, expected);
        release.now();
        worker
            .join()
            .unwrap_or_else(|_| panic!("SVEV-014 {CASE}: the in-process recover panicked"))
    });
    for key in [
        "RATMAC_TEST_RECOVER_PAUSE",
        "RATMAC_TEST_RECOVER_PAUSE_MARKER",
        "RATMAC_TEST_RECOVER_PAUSE_RELEASE",
        "RATMAC_TEST_RECOVER_PAUSE_TIMEOUT_MILLIS",
    ] {
        std::env::remove_var(key);
    }
    assert_eq!(
        code, 0,
        "SVEV-014 {CASE}: the released recovery succeeds: {stdout}{message}"
    );
    let mut text = format!("exit: {code}\nstdout:\n{stdout}\nmessage:\n{message}");
    for tree in [root.as_path(), keys.path(), isolation.path()] {
        for form in root_spellings(tree) {
            text = text.replace(&form, "<root>");
        }
    }
    text
}

/// SVEV-014 / SVE-005: for the turn tool's `after-validate` pause and each
/// Engine pause point that writes its marker in place, an empty file and, in
/// a second run, the boundary text without its newline are planted at the
/// marker path before the paused command starts. Called with a short limit
/// before the start, t125's waiter reports not yet published; once the
/// command starts, the waiter acts only after the whole line arrives, and
/// the case's released verdict matches a run with nothing planted. Rename
/// publishers are not cases here (HT-142-06 covers them).
#[test]
fn svev_014() {
    type Case = fn(Option<&[u8]>) -> String;
    let cases: [(&str, &[u8], Case); 7] = [
        (
            "turn tool after-validate",
            &b"after-validate"[..],
            turn_case,
        ),
        (
            "hold barrier before-state-write",
            &b"holding before before-state-write"[..],
            hold_case,
        ),
        ("lock hold root", &b"holding root lock"[..], lock_case),
        ("policy pause verify", &b"verify"[..], policy_case),
        (
            "nested pause after-record",
            &b"after-record"[..],
            nested_case,
        ),
        (
            "endpoint pause after-admit",
            &b"after-admit"[..],
            endpoint_case,
        ),
        (
            "recover pause before-rename",
            &b"before-rename"[..],
            recover_case,
        ),
    ];
    for (case, partial, run) in cases {
        let control = run(None);
        for planted in [&b""[..], partial] {
            assert_eq!(
                run(Some(planted)),
                control,
                "SVEV-014 {case}: with {:?} planted before the start, the released verdict \
                 must match the run with nothing planted",
                String::from_utf8_lossy(planted)
            );
        }
    }
}
