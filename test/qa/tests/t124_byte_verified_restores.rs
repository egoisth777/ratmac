//! t-124 / WCP-005: byte verified restores - the turn tool's `checkpoint`
//! and `restore` verbs freeze and re-verify the exact bytes of every
//! declared tracked mutation target.
//!
//! Every fixture is a throwaway repository under the temp directory built by
//! the turn-lifecycle family (`ratmac_qa::turn`), carrying the real
//! `tools/turn.ps1` and its declared data. Git in a fixture - the tool's own
//! children included - reads a fixture-local empty global configuration file
//! through `GIT_CONFIG_GLOBAL` with `GIT_CONFIG_NOSYSTEM=1`, so no host or
//! global setting is read or written; the only configuration a test changes
//! is the fixture repository's own. Each suite begins with today's `status`
//! verb as its positive control, then drives the new verbs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use ratmac_qa::snapshot::sha256_file;
use ratmac_qa::support::{capture, command, CaptureOptions, Snapshot, TempTree};
use ratmac_qa::turn::{Turn, ITEM_RECORDS, RERUN_MARKER, TRUNK};

/// The item string every fixture addresses.
const ITEM: &str = "item-1";

/// The item record bytes the fixture commits - the second declared target of
/// the round-trip check.
const RECORD_BYTES: &[u8] =
    b"---\nitem-id: \"item-1\"\nstatus: \"approved\"\nlanded-commit: \"\"\n---\n\n# Item: item-1\n";

// --- isolated fixture plumbing ------------------------------------------------

/// An owned tree holding nothing but the empty global Git configuration file
/// every fixture-local child reads instead of the host's.
fn isolation(label: &str) -> (TempTree, PathBuf) {
    let tree = TempTree::new(&format!("t124-{label}")).expect("create the isolation tree");
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

/// Fixture Git with the same isolation, and no other setting at all: the
/// drift check below must see the fixture repository's own configuration.
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
        "t124: git {args:?} fails: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
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
    assert_eq!(actual, expected, "t124: {what} must be exactly these lines");
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
        "t124: today's status verb stays green: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        lines(&output.stdout).first().map(String::as_str),
        Some("turn status (read-only; nothing below has been applied)"),
        "t124: the status report keeps its first line"
    );
}

// --- whole-tree snapshot helpers ----------------------------------------------

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

// --- shared fixture facts -----------------------------------------------------

/// The frozen manifest's path inside a fixture.
fn manifest_path(turn: &Turn) -> PathBuf {
    turn.root
        .join(format!("{ITEM_RECORDS}/{ITEM}-checkpoint-manifest.txt"))
}

/// The exact manifest bytes a checkpoint of `targets` must freeze, sorted
/// bytewise ascending, LF-terminated, with the digests and blob ids this
/// test recomputes itself.
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

// --- t-131's review/proofs setup (a fixture-only adjustment) -------------

/// t-131 (WCP-004) inserts a gate into the checkpoint verb: the freeze
/// accepts only reviewed, proven green work, so every checkpoint positive
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
        "t124 t-131 setup: the accepted review is recorded over the green bytes (the review verb must exist): {}{}",
        String::from_utf8_lossy(&review.stdout),
        String::from_utf8_lossy(&review.stderr)
    );
    let proofs = turn_isolated(global, &turn.root, &["proofs", "-Item", ITEM]);
    assert!(
        proofs.status.success(),
        "t124 t-131 setup: the proof set is earned behind the accepted review (the proofs verb must exist): {}{}",
        String::from_utf8_lossy(&proofs.stdout),
        String::from_utf8_lossy(&proofs.stderr)
    );
}

// --- the checks ------------------------------------------------------------

/// WCPV-013: a drifted line-ending conversion that leaves the tracked status clean while the bytes differ is refused by path after the restore, never accepted on a clean status alone.
#[test]
fn wcpv_013() {
    let (_isolation, global) = isolation("wcpv013");
    let turn = Turn::new("wcpv013");
    status_control(&global, &turn);
    // t-131 (WCP-004): the trace marker joins the ignore list first, so the
    // earned proof set changes no reviewed row.
    declare_trace_marker_ignored(&global, &turn);

    // t-125 (WCP-006) tightens this verb: checkpoint records only a HEAD
    // committed under the fixed safety subject, so the fixture's green work
    // lands under it before the freeze - a fixture-only adjustment.
    git_ok(
        &global,
        &turn.root,
        &[
            "commit",
            "--allow-empty",
            "-m",
            "item-1: checkpoint - not a landing",
        ],
    );
    // t-131 (WCP-004): the checkpoint gate demands the accepted review and
    // the earned proof receipt - a fixture-only adjustment.
    review_and_proofs(&global, &turn);
    let head = turn.head_of(TRUNK);
    let readme = turn.root.join("README.md");
    let manifest = manifest_path(&turn);

    let frozen = turn_isolated(
        &global,
        &turn.root,
        &["checkpoint", "-Item", ITEM, "-Target", "README.md"],
    );
    assert!(
        frozen.status.success(),
        "t124 WCPV-013: checkpoint freezes the clean tree (the checkpoint verb must exist): {}{}",
        String::from_utf8_lossy(&frozen.stdout),
        String::from_utf8_lossy(&frozen.stderr)
    );
    assert!(
        frozen.stderr.is_empty(),
        "t124: a successful checkpoint prints no stderr: {}",
        String::from_utf8_lossy(&frozen.stderr)
    );
    assert_lines(
        &frozen.stdout,
        &[
            "turn checkpoint frozen",
            "  item:      item-1",
            &format!("  commit:    {head}"),
            "  manifest:  items/item-1-checkpoint-manifest.txt (1 target, round-trip proven)",
            "  damage:    mutate only the declared targets, then run: pwsh -File tools/turn.ps1 restore -Item item-1",
        ],
        "the checkpoint report",
    );
    let frozen_manifest = fs::read(&manifest).expect("the frozen manifest exists");
    assert_eq!(
        String::from_utf8_lossy(&frozen_manifest),
        expected_manifest(&global, &turn, &head, &["README.md"]),
        "t124 WCPV-013: the frozen manifest's exact bytes"
    );

    // Deliberate damage, then the settings drift the historical counterexample
    // names: repository-local conversion the frozen bytes cannot reproduce.
    fs::write(&readme, "# damaged\n").expect("damage the declared target");
    git_ok(&global, &turn.root, &["config", "core.autocrlf", "true"]);
    let before = tree(&turn);

    let verified = turn_isolated(&global, &turn.root, &["restore", "-Item", ITEM]);
    assert!(
        !verified.status.success(),
        "t124 WCPV-013: a restore whose bytes differ refuses, clean status or not: {}{}",
        String::from_utf8_lossy(&verified.stdout),
        String::from_utf8_lossy(&verified.stderr)
    );
    assert_lines(&verified.stdout, &[], "the refusal prints no success line");
    assert_lines(
        &verified.stderr,
        &[
            "turn refused; the restored bytes of README.md do not match its frozen digest",
            "  a clean Git status is not acceptance: the effective line-ending or checkout-filter settings changed since the checkpoint froze these bytes;",
            "  restore the settings the manifest records for README.md (text unspecified, eol unspecified, filter unspecified; core.autocrlf false, core.eol unset), then run restore again;",
            "  no unrelated file was touched.",
        ],
        "the digest-mismatch refusal",
    );

    // The refusal's ground, stated by the fixture itself: every porcelain row
    // untracked while the target's bytes are the converted, changed ones.
    for row in porcelain(&global, &turn.root) {
        assert!(
            row.starts_with("??"),
            "t124 WCPV-013: the drifted status stays clean, every row untracked: {row}"
        );
    }
    assert_eq!(
        fs::read(&readme).expect("reread the target"),
        b"# fixture\r\n".as_slice(),
        "t124 WCPV-013: the target holds the restored-but-unverified converted bytes"
    );
    assert_eq!(
        fs::read(&manifest).expect("reread the manifest"),
        frozen_manifest,
        "t124 WCPV-013: the refused restore leaves the manifest unchanged"
    );
    assert_eq!(
        changed_paths(&before, &tree(&turn)),
        vec!["README.md".to_owned()],
        "t124 WCPV-013: the refused restore touches no unrelated file"
    );
}

/// WCPV-014: a byte-preserving checkpoint/restore round trip returns every declared target to its frozen digests, while a tampered manifest row and a target the checkpoint commit does not carry each refuse.
#[test]
fn wcpv_014() {
    let (_isolation, global) = isolation("wcpv014");
    let turn = Turn::new("wcpv014");
    status_control(&global, &turn);
    // t-131 (WCP-004): the trace marker joins the ignore list first, so the
    // earned proof set changes no reviewed row.
    declare_trace_marker_ignored(&global, &turn);

    // t-125 (WCP-006) tightens this verb: checkpoint records only a HEAD
    // committed under the fixed safety subject, so the fixture's green work
    // lands under it before the freeze - a fixture-only adjustment.
    git_ok(
        &global,
        &turn.root,
        &[
            "commit",
            "--allow-empty",
            "-m",
            "item-1: checkpoint - not a landing",
        ],
    );
    // t-131 (WCP-004): the checkpoint gate demands the accepted review and
    // the earned proof receipt - a fixture-only adjustment.
    review_and_proofs(&global, &turn);
    let head = turn.head_of(TRUNK);
    let readme = turn.root.join("README.md");
    let record = turn.root.join("items/item-1.md");
    let manifest = manifest_path(&turn);
    let targets = ["README.md", "items/item-1.md"];

    let frozen = turn_isolated(
        &global,
        &turn.root,
        &[
            "checkpoint",
            "-Item",
            ITEM,
            "-Target",
            "README.md,items/item-1.md",
        ],
    );
    assert!(
        frozen.status.success(),
        "t124 WCPV-014: checkpoint freezes both declared targets (the checkpoint verb must exist): {}{}",
        String::from_utf8_lossy(&frozen.stdout),
        String::from_utf8_lossy(&frozen.stderr)
    );
    assert!(
        frozen.stderr.is_empty(),
        "t124: a successful checkpoint prints no stderr: {}",
        String::from_utf8_lossy(&frozen.stderr)
    );
    assert_lines(
        &frozen.stdout,
        &[
            "turn checkpoint frozen",
            "  item:      item-1",
            &format!("  commit:    {head}"),
            "  manifest:  items/item-1-checkpoint-manifest.txt (2 targets, round-trip proven)",
            "  damage:    mutate only the declared targets, then run: pwsh -File tools/turn.ps1 restore -Item item-1",
        ],
        "the checkpoint report",
    );
    let green_manifest = expected_manifest(&global, &turn, &head, &targets);
    assert_eq!(
        String::from_utf8_lossy(&fs::read(&manifest).expect("the frozen manifest exists")),
        green_manifest,
        "t124 WCPV-014: the frozen manifest's exact bytes"
    );

    // Deliberate damage to every declared target, then the round trip back.
    fs::write(&readme, "# damaged\n").expect("damage the first target");
    fs::write(&record, "tampered record\n").expect("damage the second target");
    let verified = turn_isolated(&global, &turn.root, &["restore", "-Item", ITEM]);
    assert!(
        verified.status.success(),
        "t124 WCPV-014: restore returns every declared target (the restore verb must exist): {}{}",
        String::from_utf8_lossy(&verified.stdout),
        String::from_utf8_lossy(&verified.stderr)
    );
    assert!(
        verified.stderr.is_empty(),
        "t124: a successful restore prints no stderr: {}",
        String::from_utf8_lossy(&verified.stderr)
    );
    assert_lines(
        &verified.stdout,
        &[
            "turn restore verified",
            "  item:      item-1",
            &format!("  commit:    {head}"),
            "  manifest:  items/item-1-checkpoint-manifest.txt",
            "  targets:   2 restored; every recomputed digest matches its frozen digest; index and tracked tree clean",
        ],
        "the restore report",
    );
    let manifest_text =
        String::from_utf8_lossy(&fs::read(&manifest).expect("reread the manifest")).into_owned();
    for row in manifest_text
        .lines()
        .filter(|row| row.starts_with("target "))
    {
        let fields: Vec<&str> = row.split('\t').collect();
        assert_eq!(
            fields.len(),
            6,
            "t124 WCPV-014: a target row is `target <path>` and five tab-separated fields: {row}"
        );
        let path = fields[0].strip_prefix("target ").expect("the row's label");
        assert_eq!(
            sha256_file(&turn.root.join(path)),
            fields[1],
            "t124 WCPV-014: the recomputed digest of {path} matches its frozen digest"
        );
    }
    assert_eq!(
        fs::read(&readme).expect("reread the first target"),
        b"# fixture\n".as_slice(),
        "t124 WCPV-014: the round trip preserves the first target's bytes"
    );
    assert_eq!(
        fs::read(&record).expect("reread the second target"),
        RECORD_BYTES,
        "t124 WCPV-014: the round trip preserves the second target's bytes"
    );
    for row in porcelain(&global, &turn.root) {
        assert!(
            row.starts_with("??"),
            "t124 WCPV-014: the restored tree is clean, every row untracked: {row}"
        );
    }

    // The tampered manifest: one row's blob id edited to a foreign object id.
    let real_blob = git_ok(
        &global,
        &turn.root,
        &["rev-parse", &format!("{head}:README.md")],
    );
    let fake_blob = "0".repeat(40);
    let tampered = green_manifest.replace(&real_blob, &fake_blob);
    assert_eq!(
        tampered.len(),
        green_manifest.len(),
        "t124: the tampered manifest keeps the row's width"
    );
    fs::write(&manifest, &tampered).expect("tamper the manifest");
    let tamper_refusal = turn_isolated(&global, &turn.root, &["restore", "-Item", ITEM]);
    assert!(
        !tamper_refusal.status.success(),
        "t124 WCPV-014: a tampered manifest refuses the restore: {}{}",
        String::from_utf8_lossy(&tamper_refusal.stdout),
        String::from_utf8_lossy(&tamper_refusal.stderr)
    );
    assert_lines(
        &tamper_refusal.stdout,
        &[],
        "the refusal prints no success line",
    );
    assert_lines(
        &tamper_refusal.stderr,
        &[
            &format!(
                "turn refused; the manifest row for README.md does not match the recorded checkpoint {head}: blob {real_blob} where the manifest carries {fake_blob}"
            ),
            "  the manifest was edited, or it records a different commit;",
            "  re-run checkpoint -Item item-1 from the green state, or repair the manifest to the recorded facts.",
        ],
        "the tampered-manifest refusal",
    );
    assert_eq!(
        fs::read(&readme).expect("reread the first target"),
        b"# fixture\n".as_slice(),
        "t124 WCPV-014: the refused restore writes nothing"
    );

    // The missing target: a manifest row naming a path the checkpoint commit
    // does not carry.
    let missing = green_manifest.replace("target items/item-1.md\t", "target items/gone.md\t");
    fs::write(&manifest, &missing).expect("name a missing target");
    let missing_refusal = turn_isolated(&global, &turn.root, &["restore", "-Item", ITEM]);
    assert!(
        !missing_refusal.status.success(),
        "t124 WCPV-014: a target the checkpoint commit does not carry refuses the restore: {}{}",
        String::from_utf8_lossy(&missing_refusal.stdout),
        String::from_utf8_lossy(&missing_refusal.stderr)
    );
    assert_lines(
        &missing_refusal.stdout,
        &[],
        "the refusal prints no success line",
    );
    assert_lines(
        &missing_refusal.stderr,
        &[
            &format!(
                "turn refused; the manifest names items/gone.md, which the recorded checkpoint commit {head} does not carry"
            ),
            "  the manifest and the commit disagree; re-run checkpoint -Item item-1 from the green state, or repair the manifest.",
        ],
        "the missing-target refusal",
    );
    assert_eq!(
        fs::read(&manifest).expect("reread the manifest"),
        missing.into_bytes(),
        "t124 WCPV-014: the refused restore never deletes the manifest"
    );
}

/// WCPV-015: a clean or smudge filter the effective settings cannot round-trip refuses the checkpoint before anything is written, and a restore that leaves a tracked path dirty never claims success or touches an unrelated file.
#[test]
fn wcpv_015() {
    let (_isolation, global) = isolation("wcpv015");
    let turn = Turn::new("wcpv015filter");
    status_control(&global, &turn);
    // t-131 (WCP-004): the trace marker joins the ignore list first, so the
    // earned proof set changes no reviewed row.
    declare_trace_marker_ignored(&global, &turn);

    // The fixture-local filter: its clean half is the identity, its smudge
    // half appends bytes, so the status stays clean while checking the
    // recorded blob out can never reproduce the worktree bytes.
    fs::write(turn.root.join("filtered.txt"), "payload\n").expect("write the filtered target");
    fs::write(
        turn.root.join(".gitattributes"),
        "filtered.txt filter=mangle\n",
    )
    .expect("declare the fixture-local filter");
    git_ok(
        &global,
        &turn.root,
        &["config", "filter.mangle.clean", "cat"],
    );
    git_ok(
        &global,
        &turn.root,
        &["config", "filter.mangle.smudge", "cat; echo x"],
    );
    git_ok(&global, &turn.root, &["add", "-A"]);
    // t-125 (WCP-006): the round-trip refusal must stay reachable behind the
    // new safety-commit check, so the fixture commits under its subject.
    git_ok(
        &global,
        &turn.root,
        &["commit", "-m", "item-1: checkpoint - not a landing"],
    );
    // t-131 (WCP-004): this round-trip refusal sits behind the inserted
    // checkpoint gate, so the fixture records the accepted review and earns
    // the proof receipt first - exactly as t-125's landing kept t-124's
    // twins reachable - and the refusal's bytes below stay byte-identical.
    review_and_proofs(&global, &turn);

    let before = tree(&turn);
    let proof = turn_isolated(
        &global,
        &turn.root,
        &["checkpoint", "-Item", ITEM, "-Target", "filtered.txt"],
    );
    assert!(
        !proof.status.success(),
        "t124 WCPV-015: a filter the settings cannot round-trip refuses the checkpoint"
    );
    assert_lines(&proof.stdout, &[], "the refusal prints no success line");
    assert_lines(
        &proof.stderr,
        &[
            "turn refused; the checkout round-trip for filtered.txt is not proven",
            "  checking the recorded blob out through the effective settings must reproduce the current bytes, and cleaning the current bytes must reproduce the recorded blob;",
            "  the effective line-ending or checkout-filter settings cannot reproduce these bytes after a restore; fix the attributes, filters, or core.autocrlf and core.eol, or correct the file, then run checkpoint again.",
            "  nothing was written.",
        ],
        "the round-trip refusal",
    );
    assert_eq!(
        changed_paths(&before, &tree(&turn)),
        Vec::<String>::new(),
        "t124 WCPV-015: the refused checkpoint writes nothing anywhere"
    );
    assert!(
        !manifest_path(&turn).exists(),
        "t124 WCPV-015: the refused checkpoint freezes no manifest"
    );

    let clean_fixture = Turn::new("wcpv015clean");
    // t-131 (WCP-004): the trace marker joins the ignore list first, so the
    // earned proof set changes no reviewed row.
    declare_trace_marker_ignored(&global, &clean_fixture);
    let cleaned = clean_fixture.root.join("cleaned.txt");
    fs::write(&cleaned, "payload\n").expect("write the clean-direction target");
    // Age the file past git's racy-clean window, then commit it with no
    // filter in force anywhere: the recorded blob is the raw worktree bytes,
    // and the untouched file's stat data will be trusted afterwards.
    std::thread::sleep(std::time::Duration::from_secs(2));
    git_ok(&global, &clean_fixture.root, &["add", "cleaned.txt"]);
    // t-125 (WCP-006): same fixture-only subject adjustment as above.
    git_ok(
        &global,
        &clean_fixture.root,
        &["commit", "-m", "item-1: checkpoint - not a landing"],
    );
    // t-131 (WCP-004): this clean-direction round-trip refusal sits behind
    // the inserted checkpoint gate, so the accepted review and the earned
    // proof receipt come first; the refusal's bytes below stay
    // byte-identical, and the untracked review evidence beside the tracked
    // tree is the setup's own footprint.
    review_and_proofs(&global, &clean_fixture);
    // The filter is attached only AFTER the commit, through untracked
    // `.git/info/attributes`: git status trusts its stat cache for the
    // untouched file, so the tracked tree stays clean, while the altering
    // clean half - the smudge half is the identity - makes cleaning the
    // current bytes produce something other than the recorded blob. Only
    // the proof's clean direction fails here, so a tool that kept the
    // checkout check but dropped the clean check passes the smudge twin
    // above yet cannot pass this one.
    fs::create_dir_all(clean_fixture.root.join(".git/info"))
        .expect("create the attributes directory");
    fs::write(
        clean_fixture.root.join(".git/info/attributes"),
        "cleaned.txt filter=alter\n",
    )
    .expect("declare the clean-direction filter");
    git_ok(
        &global,
        &clean_fixture.root,
        &["config", "filter.alter.smudge", "cat"],
    );
    git_ok(
        &global,
        &clean_fixture.root,
        &["config", "filter.alter.clean", "cat; echo c"],
    );
    // t-131 (WCP-004): the review and proof evidence sits untracked beside
    // the clean tracked tree, so the fixture proves cleanliness by every
    // row staying untracked.
    for row in porcelain(&global, &clean_fixture.root) {
        assert!(
            row.starts_with("??"),
            "t124 WCPV-015: the clean-direction fixture's tracked tree is clean, every row untracked: {row}"
        );
    }

    let clean_before = tree(&clean_fixture);
    let clean_proof = turn_isolated(
        &global,
        &clean_fixture.root,
        &["checkpoint", "-Item", ITEM, "-Target", "cleaned.txt"],
    );
    assert!(
        !clean_proof.status.success(),
        "t124 WCPV-015: a filter whose clean half mangles the bytes refuses the checkpoint"
    );
    assert_lines(
        &clean_proof.stdout,
        &[],
        "the refusal prints no success line",
    );
    assert_lines(
        &clean_proof.stderr,
        &[
            "turn refused; the checkout round-trip for cleaned.txt is not proven",
            "  checking the recorded blob out through the effective settings must reproduce the current bytes, and cleaning the current bytes must reproduce the recorded blob;",
            "  the effective line-ending or checkout-filter settings cannot reproduce these bytes after a restore; fix the attributes, filters, or core.autocrlf and core.eol, or correct the file, then run checkpoint again.",
            "  nothing was written.",
        ],
        "the clean-direction round-trip refusal",
    );
    assert_eq!(
        changed_paths(&clean_before, &tree(&clean_fixture)),
        Vec::<String>::new(),
        "t124 WCPV-015: the refused checkpoint writes nothing anywhere"
    );
    assert!(
        !manifest_path(&clean_fixture).exists(),
        "t124 WCPV-015: the refused checkpoint freezes no manifest"
    );

    // A restore with unrelated unsaved work beside the declared damage:
    // t-125 (WCP-006) moves this boundary ahead of the mutation, so the
    // refusal names the unrelated rows, claims no success, and leaves the
    // damaged target, the index, and every unrelated file exactly as found.
    let restore_fixture = Turn::new("wcpv015restore");
    // t-131 (WCP-004): the trace marker joins the ignore list first, so the
    // earned proof set changes no reviewed row.
    declare_trace_marker_ignored(&global, &restore_fixture);
    // t-125 (WCP-006) tightens this verb: checkpoint records only a HEAD
    // committed under the fixed safety subject, so the fixture's green work
    // lands under it before the freeze - a fixture-only adjustment.
    git_ok(
        &global,
        &restore_fixture.root,
        &[
            "commit",
            "--allow-empty",
            "-m",
            "item-1: checkpoint - not a landing",
        ],
    );
    // t-131 (WCP-004): the checkpoint gate demands the accepted review and
    // the earned proof receipt - a fixture-only adjustment.
    review_and_proofs(&global, &restore_fixture);
    let head = restore_fixture.head_of(TRUNK);
    let frozen = turn_isolated(
        &global,
        &restore_fixture.root,
        &["checkpoint", "-Item", ITEM, "-Target", "README.md"],
    );
    assert!(
        frozen.status.success(),
        "t124 WCPV-015: checkpoint freezes the clean tree (the checkpoint verb must exist): {}{}",
        String::from_utf8_lossy(&frozen.stdout),
        String::from_utf8_lossy(&frozen.stderr)
    );
    assert_lines(
        &frozen.stdout,
        &[
            "turn checkpoint frozen",
            "  item:      item-1",
            &format!("  commit:    {head}"),
            "  manifest:  items/item-1-checkpoint-manifest.txt (1 target, round-trip proven)",
            "  damage:    mutate only the declared targets, then run: pwsh -File tools/turn.ps1 restore -Item item-1",
        ],
        "the checkpoint report",
    );
    let readme = restore_fixture.root.join("README.md");
    let landlog = restore_fixture.root.join("landlog.md");
    fs::write(&readme, "# damaged\n").expect("damage the declared target");
    fs::write(&landlog, "# landings\nunrelated dirt\n").expect("dirty an unrelated tracked file");
    let before = tree(&restore_fixture);
    let rows_before = porcelain(&global, &restore_fixture.root);

    let dirty = turn_isolated(&global, &restore_fixture.root, &["restore", "-Item", ITEM]);
    assert!(
        !dirty.status.success(),
        "t124 WCPV-015: unrelated unsaved work refuses the restore before any write: {}{}",
        String::from_utf8_lossy(&dirty.stdout),
        String::from_utf8_lossy(&dirty.stderr)
    );
    assert_lines(&dirty.stdout, &[], "the refusal prints no success line");
    assert_lines(
        &dirty.stderr,
        &[
            "turn refused; the working tree carries unsaved work unrelated to the declared targets; commit or put aside these entries first:",
            "   M landlog.md",
            "  a restore may undo only the declared targets' damage;",
            "  nothing was written.",
        ],
        "the unrelated-unsaved-work refusal",
    );
    assert_eq!(
        fs::read(&readme).expect("reread the target"),
        b"# damaged\n".as_slice(),
        "t124 WCPV-015: the refusal leaves the damaged target exactly as found"
    );
    assert_eq!(
        fs::read(&landlog).expect("reread the unrelated file"),
        b"# landings\nunrelated dirt\n".as_slice(),
        "t124 WCPV-015: the refused restore leaves the unrelated file untouched"
    );
    assert_eq!(
        porcelain(&global, &restore_fixture.root),
        rows_before,
        "t124 WCPV-015: the refusal leaves the index and the tracked rows exactly as found"
    );
    assert_eq!(
        changed_paths(&before, &tree(&restore_fixture)),
        Vec::<String>::new(),
        "t124 WCPV-015: the refused restore writes nothing anywhere"
    );
}
