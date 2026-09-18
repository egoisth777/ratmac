//! t-111 / TCE-001: the turn close's final verification runs the repository's
//! declared expiry-aware verifier freshly, refuses every not-pass condition
//! by name, never touches marker bytes, and retries a failed verification
//! without repeating any landed close mutation.
//!
//! TCEV-001 `configured_close_uses_fresh_sweep_and_preserves_expiry`
//! TCEV-002 `failed_final_verification_resumes_without_replaying_close`
//! TCEV-003 `declared_scope_is_generic_compatible_and_checked_before_writes`
//!
//! The surface under test is the shipped `tools/turn.ps1` closed
//! `lanes-rerun-scope` declaration (ADR-0021): omitted or `per-lane` keeps
//! the existing per-lane rerun, `root-once` runs the declared command once
//! from the primary checkout, and any other value refuses before the first
//! write. The turn tool stays generic - it executes the declared command and
//! never interprets an expiry marker - while this repository's profile
//! composes the shipped `tools/sweep_lanes.py` fresh sweep and report check
//! over the ignored `target/turn-close-lanes.md` report, so a close accepts
//! exactly the crates that pass or carry a valid explicit expiry marker.
//!
//! TCEV-001 and TCEV-002 drive real throwaway Git repositories carrying the
//! shipped turn tool, the shipped sweep script, and the repository's own
//! declared verification command and scope - read from the shipped
//! `.ratmac/turn.toml`, never retyped here, so a broken profile cannot hide
//! behind a test-only correct declaration - plus real tiny Cargo crates
//! under the fixture's lanes root. Every rostered crate is a fixture-only
//! `t-9xx` crate: no nested sweep ever runs against this repository's real
//! lanes roster, and every sweep fixture removes the turn helper's
//! non-`t-###` default lane so the roster sees no strays it did not declare
//! on purpose. The verification report root `target/` joins the fixture's
//! ignore rules, so a close's report never dirties the trunk. TCEV-003 stays
//! deliberately generic: custom commands writing their own marker files -
//! never this repository's `EXPIRED.toml` filename - so the scope mechanism
//! is proven without any verifier vocabulary.
//!
//! Hole-poke notes:
//! - Would TCEV-001 pass a close that re-reads an existing report? No. The
//!   stale twin seeds a genuinely green report, flips one lane red without
//!   sweeping, and requires the close to refuse over a fresh report whose
//!   red row names the failing lane id; the stray twin deletes the report
//!   and requires the close to regenerate it, so a check-only close cannot
//!   pass a report it did not just write.
//! - Would TCEV-001 pass a verification without the check? No. The sweep
//!   alone exits 0 over an undeclared stray crate (proven by a direct run),
//!   while the declared close must refuse it.
//! - Would TCEV-001 pass a verifier that renews, rewrites, or removes a
//!   marker? No. Marker bytes are captured and asserted byte-identical after
//!   every close, successful or refused, including a malformed marker's.
//! - Would TCEV-001 pass a sweep that runs a marked lane anyway, or a
//!   marker reading that accepts a blank reason? No. The expired detail
//!   must carry the ordinary sweep's skip wording (the lane was not run),
//!   and a blank-reason marker refuses by its path before any report
//!   exists.
//! - Would TCEV-002 pass a retry that replays a landed mutation? No. Trunk
//!   tip, item-record bytes, and landing-log bytes are captured after the
//!   failed close and asserted identical after the successful retry, with
//!   the landing line counted once.
//! - Would TCEV-002 pass a retry without durable landing evidence? No.
//!   Each twin - a stale stamp, a missing log line, a missing `-Line`, a
//!   surviving worktree folder, a stale detached registration squatting on
//!   the sibling path (the item branch stays absent, so THK-002's ordinary
//!   branch-present resume is never forbidden), unrelated tracked dirt -
//!   deletes the report first, so a retry that entered verification would
//!   rewrite it; its absence proves the refusal came first, and the
//!   unchanged tip proves nothing else moved.
//! - Would TCEV-003 pass a tool that ignores the scope key? No. The
//!   root-once command must leave exactly one marker line at the primary
//!   root and nothing in any lane directory.
//! - Would TCEV-003 pass a scope checked after the first write? No. Every
//!   invalid shape - unknown value, empty string, list - refuses with a
//!   byte-identical whole-repo snapshot, from `open` and `close` both.
//! - Would TCEV-003 pass a dry run that mutates? No. Status is bracketed
//!   by whole-repo snapshots and must name the selected scope.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use ratmac_qa::turn::{
    Turn, ITEM_RECORDS, LANDING_LOG, LANES, RERUN_COMMAND, RERUN_MARKER, SKIP, STAMP_FIELD, TRUNK,
};

/// The item string every fixture addresses - opaque to the tool, never
/// parsed for a ticket shape.
const ITEM: &str = "item-1";

/// The landing-line words every close appends.
const LINE: &str = "the item-1 turn landed";

/// The sweep's in-crate expiry marker filename (LNR-002) - the shipped
/// validator's own vocabulary, used only by the sweep fixtures; the generic
/// TCEV-003 fixtures never name it.
const EXPIRED_MARKER: &str = "EXPIRED.toml";

/// One declared report row: (crate, verdict, detail).
type Row = (String, String, String);

// --- shared helpers -----------------------------------------------------------

/// This repository's root, resolved from the harness location.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the harness runs from a checkout of this repository")
}

/// The shipped sweep script, resolved from the harness location.
fn sweep_source() -> PathBuf {
    repo_root().join("tools/sweep_lanes.py")
}

/// Combined stdout and stderr, for wording assertions.
fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A refused invocation must exit non-zero; its combined text comes back.
fn refused(output: &Output, what: &str) -> String {
    let text = combined(output);
    assert!(!output.status.success(), "TCEV: {what} must refuse, not pass");
    text
}

/// The text must name at least one of the needles, case-insensitively - a
/// refusal's wording is free, but it cannot refuse namelessly.
fn names_any(text: &str, what: &str, needles: &[&str]) {
    let lower = text.to_lowercase();
    assert!(
        needles
            .iter()
            .any(|needle| lower.contains(&needle.to_lowercase())),
        "TCEV: {what} must name one of {needles:?}: {text}"
    );
}

/// How many times `needle` occurs in `haystack`.
fn occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

// --- report reading -----------------------------------------------------------

/// The ignored report the declared verification command writes.
fn report_path(root: &Path) -> PathBuf {
    root.join("target").join("turn-close-lanes.md")
}

fn read_report(turn: &Turn) -> String {
    fs::read_to_string(report_path(&turn.root)).unwrap_or_else(|error| {
        panic!("the close's verification wrote its report: {error}")
    })
}

fn delete_report(turn: &Turn) {
    let _ = fs::remove_file(report_path(&turn.root));
}

/// Every `| crate | verdict | detail |` row, in file order.
fn report_rows(report: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    for line in report.lines() {
        if !line.starts_with("| t-") {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        assert_eq!(cells.len(), 3, "a report row carries three cells: {line}");
        rows.push((
            cells[0].to_owned(),
            cells[1].to_owned(),
            cells[2].to_owned(),
        ));
    }
    rows
}

/// The one row a report carries for `id`, or a panic naming the report.
fn row_of(report: &str, id: &str) -> Row {
    report_rows(report)
        .into_iter()
        .find(|(row, _, _)| row == id)
        .unwrap_or_else(|| panic!("the report carries a row for {id}: {report}"))
}

/// The `- verdicts: ...` summary line, verbatim.
fn report_summary(report: &str) -> String {
    report
        .lines()
        .find(|line| line.starts_with("- verdicts: "))
        .unwrap_or_else(|| panic!("the report carries a verdicts summary: {report}"))
        .to_owned()
}

// --- the shipped profile ------------------------------------------------------

/// The repository's declared close verification (scope, command), read from
/// the shipped `.ratmac/turn.toml` - the profile the fixtures replay, so a
/// broken declaration fails here instead of hiding behind a test-only copy.
fn shipped_verification() -> (String, String) {
    let text = fs::read_to_string(repo_root().join(".ratmac/turn.toml"))
        .expect("the shipped turn declaration exists");
    let document: toml::Value = text.parse().expect("the shipped declaration is valid TOML");
    let roots = document
        .get("roots")
        .and_then(toml::Value::as_table)
        .expect("the shipped declaration carries its [roots] table");
    let scope = roots
        .get("lanes-rerun-scope")
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| {
            panic!("the shipped declaration selects lanes-rerun-scope (ADR-0021): {roots:?}")
        })
        .to_owned();
    assert_eq!(
        scope, "root-once",
        "this repository declares the root-once close verification"
    );
    let command = roots
        .get("lanes-rerun")
        .and_then(toml::Value::as_str)
        .expect("the shipped declaration carries lanes-rerun")
        .to_owned();
    assert!(
        command.contains("sweep_lanes.py"),
        "the declared verifier is the shipped lane sweep: {command}"
    );
    assert!(
        command.contains("sweep") && command.contains("check"),
        "a fresh sweep and the report check compose: {command}"
    );
    assert!(
        command.contains("&&"),
        "the check runs only behind the fresh sweep: {command}"
    );
    assert!(
        command.matches("turn-close-lanes.md").count() >= 2,
        "both halves report to the ignored turn-close report: {command}"
    );
    (scope, command)
}

// --- sweep fixtures -----------------------------------------------------------

/// One real tiny Cargo crate under the fixture's lanes root: a single lane
/// that passes or refuses on purpose, named so a red verdict's detail
/// carries the lane id.
fn write_lane(lanes_root: &Path, id: &str, green: bool) {
    let number = id.trim_start_matches("t-");
    fs::create_dir_all(lanes_root.join(id).join("src")).expect("create the fixture crate");
    fs::write(
        lanes_root.join(id).join("Cargo.toml"),
        format!(
            "# fixture lane crate for {id}.\n[package]\nname = \"{}\"\nversion = \"0.1.0\"\n\
             edition = \"2021\"\npublish = false\n\n[workspace]\n",
            id.replace('-', ""),
        ),
    )
    .expect("write the fixture manifest");
    fs::write(
        lanes_root.join(id).join("src/lib.rs"),
        format!(
            "#[test]\nfn ht_{number}_lane() {{\n    assert!({green}, \
             \"the fixture lane refuses on purpose\");\n}}\n",
        ),
    )
    .expect("write the fixture lane");
}

/// Run the fixture's own installed copy of the shipped sweep script, from
/// the fixture root exactly as the declared command does.
fn fixture_python(turn: &Turn, args: &[&str]) -> Output {
    Command::new("python")
        .arg(turn.root.join("tools/sweep_lanes.py"))
        .args(args)
        .current_dir(&turn.root)
        .stdin(Stdio::null())
        .output()
        .expect("invoke the fixture sweep")
}

/// Mark one fixture crate expired by the real explicit act, with the date
/// pinned so the marker bytes are deterministic.
fn mark_expired(turn: &Turn, id: &str, edition: &str) {
    let marked = fixture_python(
        turn,
        &[
            "mark",
            id,
            "--edition",
            edition,
            "--reason",
            "the fixture lane refuses the way the frozen-source crates do",
            "--date",
            "2026-09-10",
        ],
    );
    assert!(
        marked.status.success(),
        "mark {id} succeeds: {}",
        combined(&marked)
    );
}

/// The in-crate marker path of one fixture crate.
fn marker_path(turn: &Turn, id: &str) -> PathBuf {
    turn.root.join(LANES).join(id).join(EXPIRED_MARKER)
}

/// Install the fixture's turn declaration: the helper's fixture roots plus
/// the shipped verification scope and command, round-tripped through the
/// TOML reader so a malformed embedding fails here, not in the tool.
fn declare_verification(root: &Path, scope: &str, command: &str) {
    let declaration = format!(
        "# The fixture turn declaration - the shipped verification profile.\n\
         [roots]\n\
         trunk = \"{TRUNK}\"\n\
         lanes = \"{LANES}\"\n\
         item-records = \"{ITEM_RECORDS}\"\n\
         landing-log = \"{LANDING_LOG}\"\n\
         stamp-field = \"{STAMP_FIELD}\"\n\
         skip = [\"{SKIP}\"]\n\
         lanes-rerun = \"{command}\"\n\
         lanes-rerun-scope = \"{scope}\"\n"
    );
    let parsed: toml::Value = declaration.parse().expect("the fixture declaration is valid TOML");
    let round = parsed
        .get("roots")
        .and_then(toml::Value::as_table)
        .expect("the fixture declaration carries its [roots] table");
    assert_eq!(
        round.get("lanes-rerun").and_then(toml::Value::as_str),
        Some(command)
    );
    assert_eq!(
        round.get("lanes-rerun-scope").and_then(toml::Value::as_str),
        Some(scope)
    );
    fs::write(root.join(".ratmac/turn.toml"), declaration).expect("write the fixture declaration");
}

/// A throwaway turn repository carrying the shipped turn tool, the shipped
/// sweep script, the shipped verification profile over fixture roots, and
/// real tiny crates under the lanes root - an independent roster this test
/// owns, never this repository's real lanes.
fn sweep_turn(label: &str, roster: &[&str], crates: &[(&str, bool)]) -> Turn {
    let turn = Turn::new(label);
    // The sweep roster knows only t-### crates: the helper's default lane
    // would read as a stray, so the fixture's lanes root holds exactly what
    // the roster declares (plus, in the stray twin, one deliberate stray).
    fs::remove_dir_all(turn.root.join(LANES).join("crate-a"))
        .expect("drop the helper's non-sweep lane");
    // The verification report root stays ignored, so a close's report never
    // dirties the trunk.
    let gitignore = turn.root.join(".gitignore");
    let mut rules = fs::read_to_string(&gitignore).expect("read the fixture ignore rules");
    rules.push_str("target/\n");
    fs::write(&gitignore, rules).expect("ignore the verification report root");
    fs::copy(sweep_source(), turn.root.join("tools/sweep_lanes.py"))
        .expect("install the shipped sweep script");
    let mut lanes_toml = String::from(
        "# The fixture lane declaration - the runbook's [roots] shape.\n\
         [roots]\nlanes = \"lanes\"\nreport = \"target/turn-close-lanes.md\"\nroster = [\n",
    );
    for id in roster {
        lanes_toml.push_str(&format!("  \"{id}\",\n"));
    }
    lanes_toml.push_str("]\n");
    fs::write(turn.root.join(".ratmac/lanes.toml"), lanes_toml)
        .expect("write the fixture lane declaration");
    for &(id, green) in crates {
        write_lane(&turn.root.join(LANES), id, green);
    }
    let (scope, command) = shipped_verification();
    declare_verification(&turn.root, &scope, &command);
    turn.git(&["add", "-A"]);
    turn.git(&["commit", "-m", "fixture: the shipped close-verification profile"]);
    turn
}

/// Open the item's turn and commit one tracked change inside it, so a close
/// has real work to land - without touching the lanes root, whose content
/// the fixture's roster owns byte for byte.
fn open_with_readme_work(turn: &Turn, edit: &str) -> PathBuf {
    let opened = turn.turn(&["open", "-Item", ITEM]);
    assert!(
        opened.status.success(),
        "open succeeds: {}",
        combined(&opened)
    );
    let worktree = turn.sibling(ITEM);
    fs::write(worktree.join("README.md"), format!("# fixture\n{edit}\n"))
        .expect("write the turn work");
    let staged = turn.git_in(&worktree, &["add", "-A"]);
    assert!(staged.status.success(), "stage the turn work");
    let committed = turn.git_in(&worktree, &["commit", "-m", "turn work"]);
    assert!(
        committed.status.success(),
        "commit the turn work: {}",
        combined(&committed)
    );
    worktree
}

fn record_path(turn: &Turn) -> PathBuf {
    turn.root.join(ITEM_RECORDS).join(format!("{ITEM}.md"))
}

fn log_path(turn: &Turn) -> PathBuf {
    turn.root.join(LANDING_LOG)
}

/// Whether the item's branch still exists.
fn branch_exists(turn: &Turn, item: &str) -> bool {
    turn.git_in(
        &turn.root,
        &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{item}")],
    )
    .status
    .success()
}

/// Overwrite one `field: "value"` line of a fixture record - the test's own
/// writer, the inverse of the helper's reader.
fn set_field(path: &Path, field: &str, value: &str) {
    let text = fs::read_to_string(path).expect("read the fixture record");
    let prefix = format!("{field}:");
    let rewritten: String = text
        .lines()
        .map(|line| {
            if line.trim_start().starts_with(&prefix) {
                format!("{field}: \"{value}\"")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<String>>()
        .join("\n");
    fs::write(path, rewritten).expect("write the fixture record");
}

// --- TCEV-001 -----------------------------------------------------------------

/// TCEV-001 (t-111, TCE-001): a close under the repository's declared
/// verifier accepts a passing crate plus a validly expired refusing crate
/// and reports the two outcomes separately; a stale passing report cannot
/// hide a live red lane; a malformed marker, a rostered crate the folder
/// lacks, and an undeclared stray crate each refuse by name; and marker
/// bytes never change.
#[test]
fn configured_close_uses_fresh_sweep_and_preserves_expiry() {
    shipped_verification();

    // --- the positive close: one pass, one valid expiry, separate counts --
    let turn = sweep_turn(
        "tcev1-pass",
        &["t-901", "t-902"],
        &[("t-901", true), ("t-902", false)],
    );
    mark_expired(&turn, "t-902", "edition-037");
    let marker_before = fs::read(marker_path(&turn, "t-902")).expect("the marker exists");
    let worktree = open_with_readme_work(&turn, "the expiry-aware close's work");

    let closed = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    assert!(
        closed.status.success(),
        "a pass plus a valid explicit expiry closes: {}",
        combined(&closed)
    );

    // The verification report sits under the primary root's ignored target/,
    // carries one row per rostered crate, and counts the outcomes separately.
    let report = read_report(&turn);
    let (row, verdict, _detail) = row_of(&report, "t-901");
    assert_eq!(row, "t-901");
    assert_eq!(verdict, "pass", "the green crate reads pass: {report}");
    let (row, verdict, detail) = row_of(&report, "t-902");
    assert_eq!(row, "t-902");
    assert_eq!(verdict, "expired", "the marked crate reads expired: {report}");
    assert!(
        detail.contains("last passed at edition-037"),
        "the expired verdict names the marker's edition: {detail}"
    );
    assert!(
        !detail.contains("verify:"),
        "the ordinary sweep skips a validly marked lane instead of running it: {detail}"
    );
    assert_eq!(
        report_summary(&report),
        "- verdicts: 1 pass, 1 expired, 0 red, 0 missing - 2 crates",
        "the outcomes are reported separately"
    );

    // The marker is an explicit recorded fact the close never touches.
    assert_eq!(
        fs::read(marker_path(&turn, "t-902")).expect("the marker survives the close"),
        marker_before,
        "the marker bytes are preserved"
    );

    // The close landed, and its only write outside the landing is the
    // ignored report: the trunk's dirt is exactly the stamp edits.
    assert!(!worktree.exists(), "the turn worktree is removed");
    assert!(!branch_exists(&turn, ITEM), "the item branch is deleted");
    assert_eq!(
        turn.field_value(&format!("{ITEM_RECORDS}/{ITEM}.md"), STAMP_FIELD),
        turn.short_of(TRUNK),
        "the stamp names the landed trunk tip"
    );
    let status = turn.git_text(&["status", "--porcelain"]);
    let mut dirt: Vec<&str> = status.lines().map(|line| &line[3..]).collect();
    dirt.sort_unstable();
    assert_eq!(
        dirt,
        vec!["items/item-1.md", "landlog.md"],
        "the report is ignored and nothing else is written: {status}"
    );

    // --- a stale passing report cannot hide a live red lane ----------------
    let turn = sweep_turn("tcev1-stale", &["t-901"], &[("t-901", true)]);
    let seeded = fixture_python(&turn, &["sweep", "--report", "target/turn-close-lanes.md"]);
    assert!(
        seeded.status.success(),
        "the seeding sweep is green: {}",
        combined(&seeded)
    );
    assert_eq!(
        row_of(&read_report(&turn), "t-901").1,
        "pass",
        "the seeded report is a genuine all-pass report"
    );
    // The lane goes red after the report was written: only a fresh sweep
    // can see it.
    write_lane(&turn.root.join(LANES), "t-901", false);
    let worktree = open_with_readme_work(&turn, "the stale-report twin's work");

    let output = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    let text = refused(&output, "a close over a stale report and a live red lane");
    names_any(
        &text,
        "the refusal names the verification step",
        &["verif", "rerun"],
    );
    let report = read_report(&turn);
    let (_, verdict, detail) = row_of(&report, "t-901");
    assert_eq!(verdict, "red", "the close re-swept: the report is fresh");
    assert!(
        detail.contains("ht_901_lane"),
        "the live red verdict names its failing lane id: {detail}"
    );
    assert!(!worktree.exists(), "the refusal still follows the cleanup");

    // --- a malformed marker refuses by path, bytes preserved ---------------
    let turn = sweep_turn(
        "tcev1-marker",
        &["t-901", "t-902"],
        &[("t-901", true), ("t-902", true)],
    );
    // A blank-reason marker: valid TOML, still not a valid explicit expiry.
    let malformed = "edition = \"edition-037\"\ndate = \"2026-09-10\"\nreason = \"\"\n";
    fs::write(marker_path(&turn, "t-902"), malformed).expect("write the malformed marker");
    // The shipped sweep itself refuses by marker path - the naming the
    // close's refusal leans on - and dies before any report exists.
    let direct = fixture_python(&turn, &["sweep", "--report", "target/turn-close-lanes.md"]);
    let direct_text = refused(&direct, "the sweep over a malformed marker");
    names_any(
        &direct_text,
        "the sweep names the malformed marker",
        &["t-902/EXPIRED.toml"],
    );
    assert!(
        !report_path(&turn.root).exists(),
        "the denied sweep wrote no report"
    );
    let worktree = open_with_readme_work(&turn, "the malformed-marker twin's work");

    let output = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    let text = refused(&output, "a close over a malformed marker");
    names_any(
        &text,
        "the refusal names the verification step",
        &["verif", "rerun"],
    );
    names_any(
        &text,
        "the refusal names the malformed marker",
        &["t-902", "expired.toml"],
    );
    assert_eq!(
        fs::read(marker_path(&turn, "t-902")).expect("the malformed marker survives"),
        malformed.as_bytes(),
        "the malformed marker's bytes are preserved"
    );
    assert!(
        !report_path(&turn.root).exists(),
        "no report was written over the refusal"
    );
    assert!(!worktree.exists(), "the refusal still follows the cleanup");

    // --- a rostered crate the folder lacks refuses by name -----------------
    let turn = sweep_turn("tcev1-missing", &["t-901", "t-903"], &[("t-901", true)]);
    let worktree = open_with_readme_work(&turn, "the missing-crate twin's work");

    let output = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    refused(&output, "a close over a rostered crate the folder lacks");
    let report = read_report(&turn);
    let (row, verdict, detail) = row_of(&report, "t-903");
    assert_eq!(row, "t-903");
    assert_eq!(verdict, "missing", "the absent crate is named, not skipped: {report}");
    assert!(
        detail.contains("the roster expects this crate and the folder lacks it"),
        "the missing verdict says what is missing: {detail}"
    );
    assert!(!worktree.exists(), "the refusal still follows the cleanup");

    // --- a stray crate: the sweep alone accepts, the close refuses ---------
    let turn = sweep_turn("tcev1-stray", &["t-901"], &[("t-901", true)]);
    let stray = turn.root.join(LANES).join("t-904");
    fs::create_dir_all(&stray).expect("create the stray crate");
    fs::write(stray.join("lane.txt"), "undeclared\n").expect("write the stray crate");
    // The sweep alone does not reject strays: only the check that follows
    // does, so the declared close must run both halves.
    let swept = fixture_python(&turn, &["sweep", "--report", "target/turn-close-lanes.md"]);
    assert!(
        swept.status.success(),
        "the sweep alone accepts a stray crate: {}",
        combined(&swept)
    );
    assert!(
        read_report(&turn).contains("t-904"),
        "the sweep's report notes the stray"
    );
    delete_report(&turn);
    let worktree = open_with_readme_work(&turn, "the stray twin's work");

    let output = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    refused(&output, "a close over an undeclared stray crate");
    let report = read_report(&turn);
    assert!(
        report.contains("stray entries") && report.contains("t-904"),
        "the close regenerated the report and the check refused the stray by name: {report}"
    );
    assert!(!worktree.exists(), "the refusal still follows the cleanup");
}

// --- TCEV-002 -----------------------------------------------------------------

/// TCEV-002 (t-111, TCE-001; THK-002): a close whose final verification
/// fails after cleanup refuses with the retry command; the identical close
/// then retries only the verification - the tip, the stamp bytes, and the
/// log bytes stay untouched and the landing line counts once - while a
/// stale stamp,
/// a missing log line, a missing `-Line`, a surviving worktree folder, a
/// stale detached registration on the sibling path (the item branch stays
/// absent, so an ordinary branch-present interrupted close is never
/// forbidden), and unrelated tracked dirt each refuse before any
/// verification runs.
#[test]
fn failed_final_verification_resumes_without_replaying_close() {
    let turn = sweep_turn("tcev2-retry", &["t-911"], &[("t-911", false)]);
    let worktree = open_with_readme_work(&turn, "the final-verification retry's work");

    // The first close: every mutation lands, the final verification refuses.
    let first = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    let text = refused(&first, "the first close over a red unexpired lane");
    names_any(
        &text,
        "the refusal names the verification step",
        &["verif", "rerun"],
    );
    assert!(
        text.contains("close -Item"),
        "the refusal offers the branchless retry command: {text}"
    );

    // The failure follows the cleanup: steps one through six are done.
    assert!(!worktree.exists(), "the worktree is removed before verification");
    assert!(!branch_exists(&turn, ITEM), "the branch is deleted before verification");
    let tip = turn.head_of(TRUNK);
    let short = turn.short_of(TRUNK);
    assert_eq!(
        turn.field_value(&format!("{ITEM_RECORDS}/{ITEM}.md"), STAMP_FIELD),
        short,
        "the stamp names the landed tip"
    );
    let record_bytes = fs::read(record_path(&turn)).expect("read the stamped record");
    let log_bytes = fs::read(log_path(&turn)).expect("read the appended log");
    assert_eq!(
        occurrences(&String::from_utf8_lossy(&log_bytes), LINE),
        1,
        "the landing line is appended exactly once"
    );
    let report = read_report(&turn);
    let (_, verdict, detail) = row_of(&report, "t-911");
    assert_eq!(verdict, "red", "the verification swept freshly: {report}");
    assert!(
        detail.contains("ht_911_lane"),
        "the refusal's evidence names the failing lane id: {detail}"
    );

    // Every evidence twin deletes the report first: a retry that entered the
    // declared verification would rewrite it, so its absence proves the
    // refusal came first, and the unchanged tip proves nothing else moved.
    let retry = |turn: &Turn, what: &str, needles: &[&str]| {
        delete_report(turn);
        let output = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
        let text = refused(&output, what);
        names_any(&text, what, needles);
        assert!(
            !report_path(&turn.root).exists(),
            "the refused retry never entered the declared verification ({what})"
        );
        assert_eq!(
            turn.head_of(TRUNK),
            tip,
            "the refused retry mutates nothing ({what})"
        );
    };

    // A stale stamp cannot enter the final-only retry.
    set_field(&record_path(&turn), STAMP_FIELD, "0000000");
    retry(
        &turn,
        "the retry over a stale stamp",
        &["stamp", "landed-commit"],
    );
    fs::write(record_path(&turn), &record_bytes).expect("restore the stamped record");

    // A missing landing line cannot enter it either.
    fs::write(log_path(&turn), "# landings\n").expect("strip the landing line");
    retry(&turn, "the retry over a missing log line", &["log", "line"]);
    fs::write(log_path(&turn), &log_bytes).expect("restore the landing log");

    // Nor a retry that supplies no line to check against the log.
    delete_report(&turn);
    let output = turn.turn(&["close", "-Item", ITEM]);
    let text = refused(&output, "the final-only retry without -Line");
    names_any(&text, "the refusal names the missing line", &["line", "log"]);
    assert!(
        !report_path(&turn.root).exists(),
        "the refused retry never entered the declared verification"
    );

    // A surviving worktree folder cannot enter it.
    fs::create_dir_all(turn.sibling(ITEM)).expect("recreate the worktree folder");
    retry(
        &turn,
        "the retry over a surviving worktree folder",
        &["worktree", "repo-item-1"],
    );
    assert!(
        turn.sibling(ITEM).is_dir(),
        "the refused retry deletes no directory itself"
    );
    fs::remove_dir_all(turn.sibling(ITEM)).expect("take the folder away again");

    // A stale registration squatting on the sibling path cannot enter it
    // either. The item branch stays absent - THK-002's ordinary resume still
    // owns branch-present states, so this twin refuses purely on the
    // branchless retry's registration fact: a worktree registered at the
    // expected path, detached from any item branch.
    let sibling = turn.sibling(ITEM).to_string_lossy().into_owned();
    turn.git(&["worktree", "add", "--detach", sibling.as_str(), tip.as_str()]);
    assert!(
        !branch_exists(&turn, ITEM),
        "the twin squats a registration without resurrecting the item branch"
    );
    retry(
        &turn,
        "the retry over a stale registration on the sibling path",
        &["worktree", "repo-item-1", "registered"],
    );
    let registrations = turn.git_text(&["worktree", "list", "--porcelain"]);
    assert!(
        registrations.contains(sibling.as_str()),
        "the refused retry removes no registration itself: {registrations}"
    );
    assert!(
        !branch_exists(&turn, ITEM),
        "the refused retry resurrects no item branch either"
    );
    turn.git(&["worktree", "remove", sibling.as_str()]);

    // Unrelated tracked dirt cannot enter it.
    fs::write(turn.root.join("README.md"), "# fixture\ndirtied\n").expect("dirty the trunk");
    retry(&turn, "the retry over unrelated tracked dirt", &["clean", "dirt", "readme"]);
    turn.git(&["checkout", "--", "README.md"]);

    // The repair, then the identical close: only the verification runs.
    write_lane(&turn.root.join(LANES), "t-911", true);
    delete_report(&turn);
    let retried = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    assert!(
        retried.status.success(),
        "the identical close retries the verification and passes: {}",
        combined(&retried)
    );
    assert_eq!(
        turn.head_of(TRUNK),
        tip,
        "the retry repeats no merge: the landed tip is untouched"
    );
    assert_eq!(
        fs::read(record_path(&turn)).expect("read the record after the retry"),
        record_bytes,
        "the retry rewrites no stamp byte"
    );
    assert_eq!(
        fs::read(log_path(&turn)).expect("read the log after the retry"),
        log_bytes,
        "the retry appends no second landing line"
    );
    assert!(!worktree.exists(), "the retry resurrects no worktree");
    assert!(!branch_exists(&turn, ITEM), "the retry resurrects no branch");
    let report = read_report(&turn);
    let (_, verdict, _detail) = row_of(&report, "t-911");
    assert_eq!(
        verdict, "pass",
        "the retried verification ran freshly over the repaired lane: {report}"
    );
}

// --- TCEV-003 -----------------------------------------------------------------

/// Rewrite the fixture's turn declaration in the helper's shape, with the
/// scope under test and, when named, a custom rerun command - generic
/// commands writing their own marker files, never this repository's marker
/// filename.
fn declare_scope(turn: &Turn, scope: Option<&str>, command: Option<&str>) {
    let mut declaration = format!(
        "# The fixture turn declaration - the scope under test.\n\
         [roots]\n\
         trunk = \"{TRUNK}\"\n\
         lanes = \"{LANES}\"\n\
         item-records = \"{ITEM_RECORDS}\"\n\
         landing-log = \"{LANDING_LOG}\"\n\
         stamp-field = \"{STAMP_FIELD}\"\n\
         skip = [\"{SKIP}\"]\n\
         lanes-rerun = \"{}\"\n",
        command.unwrap_or(RERUN_COMMAND),
    );
    if let Some(scope) = scope {
        declaration.push_str(&format!("lanes-rerun-scope = \"{scope}\"\n"));
    }
    fs::write(turn.root.join(".ratmac/turn.toml"), declaration)
        .expect("write the fixture declaration");
    turn.git(&["add", "-A"]);
    turn.git(&["commit", "-m", "fixture: declare the verification scope"]);
}

/// Append one declaration line to the helper's shipped shape and commit it.
fn append_declaration(turn: &Turn, line: &str) {
    let path = turn.root.join(".ratmac/turn.toml");
    let mut text = fs::read_to_string(&path).expect("read the fixture declaration");
    text.push_str(line);
    text.push('\n');
    fs::write(&path, text).expect("extend the fixture declaration");
    turn.git(&["add", "-A"]);
    turn.git(&["commit", "-m", "fixture: declare the verification scope"]);
}

/// The declared rerun marker must exist in one lane directory.
fn lane_marker(turn: &Turn, lane: &str) {
    let path = turn.root.join(LANES).join(lane).join(RERUN_MARKER);
    assert!(path.is_file(), "the per-lane rerun reached lane {lane}: {path:?}");
}

/// A second lane directory in the primary's untracked lanes root, so the
/// per-lane rerun has more than one lane to reach - the helper's default
/// fixture carries only crate-a, and the generic cases here commit their
/// turn work in README instead of adding a lane the way `open_with_work`
/// does. The lanes root is ignored, so the fixture needs no commit for it.
fn add_second_lane(turn: &Turn) {
    let lane = turn.root.join(LANES).join("crate-b");
    fs::create_dir_all(&lane).expect("create the second fixture lane");
    fs::write(lane.join("b.txt"), "lane b\n").expect("write the second fixture lane");
}

/// TCEV-003 (t-111, TCE-001; THK-004): the scope declaration stays generic -
/// an omitted scope keeps the per-lane default and an explicit `per-lane`
/// declares the same, a root-once command runs exactly once from the primary
/// checkout, every invalid shape refuses before any write from `open` and
/// `close` both, and the dry run names the selected scope while changing
/// nothing.
#[test]
fn declared_scope_is_generic_compatible_and_checked_before_writes() {
    // --- an omitted scope keeps the existing per-lane default --------------
    let turn = Turn::new("tcev3-default");
    add_second_lane(&turn);
    let worktree = open_with_readme_work(&turn, "the default per-lane close's work");
    let closed = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    assert!(
        closed.status.success(),
        "the omitted scope closes exactly as before: {}",
        combined(&closed)
    );
    lane_marker(&turn, "crate-a");
    lane_marker(&turn, "crate-b");
    assert!(!worktree.exists(), "the default close still removes the worktree");
    assert!(
        !turn.root.join(RERUN_MARKER).exists(),
        "the default rerun never runs at the primary root"
    );
    // The dry run names the selected scope and stays read-only.
    let before = turn.snapshot();
    let status = turn.turn(&["status"]);
    assert!(status.status.success(), "status succeeds");
    let text = combined(&status);
    assert!(
        text.to_lowercase().contains("per-lane"),
        "the omitted scope is named as the per-lane default: {text}"
    );
    assert!(
        text.contains(RERUN_MARKER),
        "the plan names the declared command: {text}"
    );
    assert_eq!(turn.snapshot(), before, "status changes nothing");

    // --- an explicit per-lane declaration stays compatible -----------------
    let turn = Turn::new("tcev3-perlane");
    declare_scope(&turn, Some("per-lane"), None);
    add_second_lane(&turn);
    let worktree = open_with_readme_work(&turn, "the explicit per-lane close's work");
    let closed = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    assert!(
        closed.status.success(),
        "an explicit per-lane scope closes exactly as before: {}",
        combined(&closed)
    );
    lane_marker(&turn, "crate-a");
    lane_marker(&turn, "crate-b");
    assert!(!worktree.exists(), "the per-lane close still removes the worktree");
    assert!(
        !turn.root.join(RERUN_MARKER).exists(),
        "an explicit per-lane rerun never runs at the primary root"
    );

    // --- a root-once command runs once, at the primary root ----------------
    let turn = Turn::new("tcev3-root");
    declare_scope(
        &turn,
        Some("root-once"),
        Some("Add-Content -Path .root-ran.txt -Value ran"),
    );
    let before = turn.snapshot();
    let status = turn.turn(&["status"]);
    assert!(status.status.success(), "status succeeds");
    let text = combined(&status);
    assert!(
        text.contains("root-once"),
        "the dry run names the root-once scope: {text}"
    );
    assert_eq!(turn.snapshot(), before, "status changes nothing");
    add_second_lane(&turn);
    let worktree = open_with_readme_work(&turn, "the root-once close's work");
    let closed = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
    assert!(
        closed.status.success(),
        "a root-once close succeeds: {}",
        combined(&closed)
    );
    let ran = turn.root.join(".root-ran.txt");
    let ran_text = fs::read_to_string(&ran)
        .unwrap_or_else(|error| panic!("the root command ran from the primary checkout: {error}"));
    assert_eq!(
        ran_text.lines().count(),
        1,
        "the root command ran exactly once, not once per lane: {ran_text:?}"
    );
    assert_eq!(ran_text.trim(), "ran", "the single run's output is intact");
    assert!(!worktree.exists(), "the root-once close still removes the worktree");
    for lane in ["crate-a", "crate-b"] {
        assert!(
            !turn.root.join(LANES).join(lane).join(".root-ran.txt").exists(),
            "the root command did not run in lane {lane}"
        );
        assert!(
            !turn.root.join(LANES).join(lane).join(RERUN_MARKER).exists(),
            "no per-lane rerun happened beside the root run in lane {lane}"
        );
    }

    // --- every invalid scope shape refuses before any write ----------------
    let shapes = [
        (
            "an unknown value",
            "tcev3-bad-value",
            "lanes-rerun-scope = \"everywhere\"",
        ),
        (
            "an empty value",
            "tcev3-bad-empty",
            "lanes-rerun-scope = \"\"",
        ),
        (
            "a list value",
            "tcev3-bad-list",
            "lanes-rerun-scope = [\"per-lane\"]",
        ),
    ];
    for (shape, label, declaration_line) in shapes {
        let turn = Turn::new(label);
        append_declaration(&turn, declaration_line);
        let before = turn.snapshot();
        let output = turn.turn(&["open", "-Item", ITEM]);
        let text = refused(&output, &format!("open over a scope that is {shape}"));
        names_any(&text, "the refusal names the scope", &["scope"]);
        assert_eq!(
            turn.snapshot(),
            before,
            "a scope that is {shape} refuses with zero mutation"
        );
        let output = turn.turn(&["close", "-Item", ITEM, "-Line", LINE]);
        let text = refused(&output, &format!("close over a scope that is {shape}"));
        names_any(&text, "the refusal names the scope", &["scope"]);
        assert_eq!(
            turn.snapshot(),
            before,
            "a scope that is {shape} refuses with zero mutation"
        );
    }
}
