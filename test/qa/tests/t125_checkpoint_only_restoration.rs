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

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Output};
use std::time::{Duration, Instant, SystemTime};

use ratmac_qa::snapshot::sha256_file;
use ratmac_qa::support::{capture, command, CaptureOptions, Snapshot, TempTree};
use ratmac_qa::turn::{Turn, ITEM_RECORDS, TRUNK};
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

/// Wait until the pause seam publishes its whole-line marker.
fn await_marker(marker: &Path) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !marker.exists() {
        assert!(
            Instant::now() < deadline,
            "t125: the pause marker {} never appeared",
            marker.display()
        );
        std::thread::sleep(Duration::from_millis(25));
    }
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

/// A supported checkpoint: exit 0, stderr empty, t-124's exact success
/// report, and the manifest's exact bytes recording the safety commit.
fn checkpoint_success(global: &Path, turn: &Turn, head: &str, targets: &[&str], what: &str) {
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
    green_commit(&global, &paused, "# fixture one\n", SAFETY_SUBJECT);
    let pause_x = paused.head_of(TRUNK);
    checkpoint_success(&global, &paused, &pause_x, &["README.md"], "the pause twin");
    fs::write(paused.root.join("README.md"), "# pause damage\n").expect("damage the target");
    let marker = isolation.join("wcpv017.marker");
    let release = isolation.join("wcpv017.release");
    let child = paused_restore(&global, &paused, &marker, &release);
    await_marker(&marker);
    assert_eq!(
        fs::read(&marker).expect("read the pause marker"),
        b"after-validate\n".as_slice(),
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
    await_marker(&marker);
    assert_eq!(
        fs::read(&marker).expect("read the pause marker"),
        b"after-validate\n".as_slice(),
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
