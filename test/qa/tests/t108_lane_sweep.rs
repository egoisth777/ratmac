//! t-108 / LNR-001..LNR-003: the landed-lane sweep reports one verdict per
//! crate, an expiry marker moves a refusing lane out of both counts, and the
//! prepared close guard refuses only a red, unexpired verdict.
//!
//! LNRV-001 `the_sweep_reports_one_verdict_per_crate_and_names_a_missing_crate`
//! LNRV-002 `the_first_sweep_separates_the_2026_08_21_crates_from_the_rot`
//! LNRV-003 `an_expiry_marker_moves_a_lane_out_of_both_counts_naming_its_edition`
//! LNRV-004 `the_close_guard_refuses_only_a_red_unexpired_verdict`
//! LNRV-005 `a_sweep_changes_only_the_report_and_a_marker_changes_only_its_own_bytes`
//!
//! The surface under test is `tools/sweep_lanes.py`, the link check's sibling
//! under `tools/`: one command enumerating the declared lanes root in id
//! order against the declared roster (`.ratmac/lanes.toml`, the runbook's
//! `[roots]`-table shape), running every crate from the invoking checkout,
//! and writing exactly one report artifact under the declared report root.
//! A crate the roster expects and the folder lacks is named `missing`,
//! never skipped; a refusing crate with no marker reads `red` with its
//! failing lane ids; an in-crate `EXPIRED.toml` naming the last-good
//! edition moves a lane out of both the pass count and the red count; and
//! `check` is the verdict reader the close guard consumes - exit 0 only
//! when every rostered crate reads `pass` or `expired`.
//!
//! LNRV-001 and LNRV-002 run the real sweep over this repository as it
//! stands (`GPH-003`: this repository is the growing fixture - fifty landed
//! crates spanning two rebuild generations), so they compile and run every
//! lane and are correspondingly slow. The two share one sweep per test
//! process through `real_sweep_report`; each crate builds into its own
//! target directory exactly as the lane always has, so a sweep re-executes
//! lanes rather than perturbing how they run. Every other check runs the
//! same shipped script over small fixture lanes roots, including `t-078`
//! and `t-079` twins whose lanes refuse the way the real frozen-source
//! crates do (`GPH-001`: verdicts walking landed history).
//!
//! Hole-poke notes:
//! - Would LNRV-001 pass a sweep that silently skips a crate the folder
//!   lacks? No. The twin removes one rostered crate from a source-only copy
//!   of this repository's lanes root and requires a `missing` row for it,
//!   a non-zero sweep exit, and id-ordered rows regardless.
//! - Would LNRV-001 pass a report without a total? No. The summary counts
//!   are re-derived from the rows and required to agree, row for row.
//! - Would LNRV-002 pass a bare, unexplained failure? No. Every `red` row
//!   must carry detail - failing lane ids, a build-refusal line, a timeout,
//!   or a no-diagnostic statement - and every `expired` row must name an
//!   edition, or the check refuses the report as a whole.
//! - Would LNRV-003 pass a sweep that counts expired lanes as red? No. The
//!   counts are asserted before and after marking: the marked crate leaves
//!   the red count without entering the pass count, and returns on unmark.
//! - Would LNRV-003 pass a marker that names no edition? No. The marker
//!   bytes are read back and the `expired` detail must name the edition;
//!   `mark` also refuses a second marker and `unmark` refuses a bare crate.
//! - Would LNRV-004 pass a diff that rewires more than one guard? No. The
//!   prepared diff is applied with `git apply` to the tracked runbook and
//!   the patched bytes must equal the original plus exactly one contiguous
//!   inserted block carrying one `command_exit` guard - every other line
//!   identical, and the tracked runbook itself still unpatched.
//! - Would LNRV-004 pass a close that reads absence as a pass? No. A
//!   missing report, a doctored-total report, and a red unexpired report
//!   each make the close step refuse, while the same close passes once the
//!   lane is green or marked - proven on a fixture Machine Class carrying
//!   the prepared guard, driven through the real Engine.
//! - Would LNRV-005 pass a sweep that touches anything but its report? No.
//!   A byte-level snapshot of the repository around a full sweep - build
//!   output excluded, the declared skip discipline - must be identical
//!   apart from the report artifact, and a mark/unmark pair must change
//!   exactly the marker's own bytes.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{LazyLock, Mutex};

use ratmac_qa::turn::SKIP;

/// One declared report row: (crate, verdict, detail).
type Row = (String, String, String);

/// This repository's root, resolved from the harness location.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the harness runs from a checkout of this repository")
}

/// The sweep script, resolved from the harness location: the link check's
/// sibling under `tools/`.
fn sweep_source() -> PathBuf {
    repo_root().join("tools/sweep_lanes.py")
}

/// Run the shipped sweep script from `root` with the given arguments.
fn run_sweep(root: &Path, args: &[&str]) -> Output {
    Command::new("python")
        .arg(sweep_source())
        .args(args)
        .current_dir(root)
        .stdin(std::process::Stdio::null())
        .output()
        .expect("invoke the lane sweep")
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Every `| crate | verdict | detail |` row, in file order.
fn rows_of(report: &str) -> Vec<Row> {
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

/// The `(pass, expired, red, missing, total)` counts a report's summary
/// declares - re-derived here from the rows, never trusted from the file.
fn counts_of(rows: &[Row]) -> (usize, usize, usize, usize, usize) {
    let mut counts = (0, 0, 0, 0, 0);
    for (_crate, verdict, _detail) in rows {
        match verdict.as_str() {
            "pass" => counts.0 += 1,
            "expired" => counts.1 += 1,
            "red" => counts.2 += 1,
            "missing" => counts.3 += 1,
            other => panic!("unknown verdict {other:?} in the report"),
        }
        counts.4 += 1;
    }
    counts
}

fn summary_line(report: &str) -> String {
    report
        .lines()
        .find(|line| line.starts_with("- verdicts: "))
        .unwrap_or_else(|| panic!("the report carries a verdicts summary: {report}"))
        .to_owned()
}

/// The roster the tracked declaration lists, in declared order.
fn crate_ids(root: &Path) -> Vec<String> {
    let text = fs::read_to_string(root.join(".ratmac/lanes.toml"))
        .expect("the tracked lane declaration exists");
    let mut ids: Vec<String> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(id) = trimmed
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix("\","))
        {
            if id.starts_with("t-") {
                ids.push(id.to_owned());
            }
        }
    }
    assert!(!ids.is_empty(), "the declaration lists the roster: {text}");
    ids
}

// --- the real sweep, shared once per test process ----------------------------

/// Real sweeps serialize on one mutex: they run the same lanes, and
/// LNRV-005 brackets its own run with tree snapshots.
static SWEEP_MUTEX: Mutex<()> = Mutex::new(());

/// The real sweep over this repository as it stands: every rostered crate
/// under the declared lanes root, run once per test process.
static REAL_REPORT: LazyLock<String> = LazyLock::new(|| {
    let _guard = SWEEP_MUTEX
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let root = repo_root();
    let output = run_sweep(&root, &["sweep"]);
    let text = combined(&output);
    assert!(
        output.status.code().is_some_and(|code| code <= 1),
        "the sweep over this repository completes (red or missing verdicts \
         exit 1, tool errors exit 2): {text}"
    );
    fs::read_to_string(root.join(".ratmac/evidence/lane-sweep/report.md"))
        .expect("the sweep wrote its report under the declared root")
});

fn real_sweep_report() -> String {
    REAL_REPORT.clone()
}

// --- fixture lanes roots ------------------------------------------------------

/// A throwaway lanes root carrying the shipped sweep script, its declared
/// data, and small crates whose lanes pass or refuse on purpose.
struct Lanes {
    root: PathBuf,
}

impl Drop for Lanes {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Lanes {
    /// `crates` pairs a crate id with whether its single lane passes; a
    /// refusing lane names itself so the report's red detail carries ids.
    fn new(label: &str, roster: &[&str], crates: &[(&str, bool)]) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t108-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("tools")).expect("create fixture tools");
        fs::copy(sweep_source(), root.join("tools/sweep_lanes.py"))
            .expect("install the shipped sweep script");

        let mut declaration = String::from(
            "# The fixture lane declaration - the runbook's [roots] shape.\n[roots]\n\
             lanes = \"test-hidden\"\n\
             report = \".ratmac/evidence/lane-sweep/report.md\"\nroster = [\n",
        );
        for crate_id in roster {
            declaration.push_str(&format!("  \"{crate_id}\",\n"));
        }
        declaration.push_str("]\n");
        fs::create_dir_all(root.join(".ratmac")).expect("create fixture declaration directory");
        fs::write(root.join(".ratmac/lanes.toml"), declaration).expect("write fixture declaration");

        for (crate_id, green) in crates {
            let number = crate_id.trim_start_matches("t-");
            let package = crate_id.replace('-', "");
            let crate_root = root.join("test-hidden").join(crate_id);
            fs::create_dir_all(crate_root.join("src")).expect("create fixture crate");
            fs::write(
                crate_root.join("Cargo.toml"),
                format!(
                    "# fixture lane crate for {crate_id}.\n[package]\nname = \"{package}\"\n\
                     version = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n\
                     [workspace]\n"
                ),
            )
            .expect("write fixture manifest");
            let lane = format!(
                "#[test]\nfn ht_{number}_01_lane() {{\n    assert!({green}, \
                 \"the lane refuses: the fixture pins a red verdict\");\n}}\n"
            );
            fs::write(crate_root.join("src/lib.rs"), lane).expect("write fixture lane");
        }

        Lanes { root }
    }

    /// Drive the fixture's own installed copy of the shipped script: a
    /// fixture is a repository in miniature, its declaration and script
    /// resolving inside it, exactly as the close guard would invoke them.
    fn invoke(&self, args: &[&str]) -> Output {
        Command::new("python")
            .arg(self.root.join("tools/sweep_lanes.py"))
            .args(args)
            .current_dir(&self.root)
            .stdin(std::process::Stdio::null())
            .output()
            .expect("invoke the lane sweep")
    }

    fn sweep(&self, extra: &[&str]) -> Output {
        let mut args = vec!["sweep"];
        args.extend_from_slice(extra);
        self.invoke(&args)
    }

    fn mark(&self, crate_id: &str, edition: &str) -> Output {
        self.invoke(&[
            "mark",
            crate_id,
            "--edition",
            edition,
            "--reason",
            "the fixture lane refuses the way the frozen-source crates do",
            "--date",
            "2026-08-10",
        ])
    }

    fn unmark(&self, crate_id: &str) -> Output {
        self.invoke(&["unmark", crate_id])
    }

    fn report(&self) -> String {
        fs::read_to_string(self.root.join(".ratmac/evidence/lane-sweep/report.md"))
            .expect("the fixture sweep wrote its report")
    }

    fn rows(&self) -> Vec<Row> {
        rows_of(&self.report())
    }

    fn verdict_of(&self, crate_id: &str) -> Row {
        self.rows()
            .into_iter()
            .find(|(row, _, _)| row == crate_id)
            .unwrap_or_else(|| panic!("the report carries a row for {crate_id}"))
    }

    fn crate_dir(&self, crate_id: &str) -> PathBuf {
        self.root.join("test-hidden").join(crate_id)
    }
}

// --- LNRV-001 -----------------------------------------------------------------

/// A source-only copy of `source` under `destination`, skipping every
/// declared build-output directory and, when named, one crate entirely -
/// the twin's removed crate.
fn copy_lanes_source(source: &Path, destination: &Path, removed: Option<&str>) {
    fn walk(from: &Path, to: &Path, removed: &Option<String>) {
        fs::create_dir_all(to).expect("create twin directory");
        for entry in fs::read_dir(from).expect("read twin source") {
            let path = entry.expect("read twin entry").path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                if name == SKIP || Some(&name) == removed.as_ref() {
                    continue;
                }
                walk(&path, &to.join(&name), removed);
            } else {
                fs::copy(&path, to.join(&name)).expect("copy twin lane bytes");
            }
        }
    }
    let _ = fs::remove_dir_all(destination);
    walk(source, destination, &removed.map(str::to_owned));
}

/// LNRV-001 (t-108, LNR-001): the sweep over this repository reports one
/// verdict per rostered crate plus a total, and a twin with one rostered
/// crate removed from the folder names that crate rather than skipping it.
#[test]
fn the_sweep_reports_one_verdict_per_crate_and_names_a_missing_crate() {
    let root = repo_root();
    let report = real_sweep_report();

    let roster = crate_ids(&root);
    assert!(
        roster.iter().any(|id| id == "t-105") && roster.iter().any(|id| id == "t-058"),
        "the tracked roster spans the landed crates t-058..t-105 and beyond: {roster:?}"
    );
    let rows = rows_of(&report);
    assert_eq!(
        rows.len(),
        roster.len(),
        "exactly one verdict per rostered crate"
    );
    let ids: Vec<&str> = rows.iter().map(|(id, _, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        roster.as_slice(),
        "the rows are the roster, in id order"
    );
    let (pass, expired, red, missing, total) = counts_of(&rows);
    assert_eq!(total, rows.len());
    assert_eq!(
        summary_line(&report),
        format!(
            "- verdicts: {pass} pass, {expired} expired, {red} red, {missing} missing - {total} crates"
        ),
        "the summary counts re-derive from the rows"
    );

    // The twin: this repository's lanes root with one rostered crate removed
    // and the build output skipped - the folder no longer holds what the
    // roster declares, and the sweep must say so instead of skipping it.
    let parent = std::env::temp_dir().join(format!(
        "ratmac-t108-twin-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    let twin = parent.join("repo");
    copy_lanes_source(
        &root.join("test-hidden"),
        &twin.join("test-hidden"),
        Some("t-105"),
    );
    fs::create_dir_all(twin.join(".ratmac")).expect("create twin declaration directory");
    let mut declaration = String::from(
        "# The twin carries this repository's own declaration.\n[roots]\n\
         lanes = \"test-hidden\"\n\
         report = \".ratmac/evidence/lane-sweep/report.md\"\nroster = [\n",
    );
    for crate_id in &roster {
        declaration.push_str(&format!("  \"{crate_id}\",\n"));
    }
    declaration.push_str("]\n");
    fs::write(twin.join(".ratmac/lanes.toml"), declaration).expect("write twin declaration");
    fs::create_dir_all(twin.join("tools")).expect("create twin tools directory");
    fs::copy(sweep_source(), twin.join("tools/sweep_lanes.py"))
        .expect("install the shipped sweep script in the twin");
    {
        let _guard = SWEEP_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let swept = Command::new("python")
            .arg(twin.join("tools/sweep_lanes.py"))
            .arg("sweep")
            .current_dir(&twin)
            .stdin(std::process::Stdio::null())
            .output()
            .expect("invoke the twin's sweep");
        let text = combined(&swept);
        assert_eq!(
            swept.status.code(),
            Some(1),
            "a sweep that names a missing crate exits 1: {text}"
        );
        let twin_report = fs::read_to_string(twin.join(".ratmac/evidence/lane-sweep/report.md"))
            .expect("the twin sweep wrote its report");
        let twin_rows = rows_of(&twin_report);
        let (_, _, _, twin_missing, twin_total) = counts_of(&twin_rows);
        assert_eq!(
            twin_missing, 1,
            "exactly the removed crate is missing: {twin_report}"
        );
        assert_eq!(
            twin_total,
            roster.len(),
            "the twin still answers for every rostered crate"
        );
        let (row, verdict, detail) = twin_rows
            .iter()
            .find(|(id, _, _)| id == "t-105")
            .expect("the removed crate is named, not skipped");
        assert_eq!(row, "t-105");
        assert_eq!(verdict, "missing");
        assert!(
            detail.contains("the roster expects this crate and the folder lacks it"),
            "the missing verdict says what is missing: {detail}"
        );
        let twin_ids: Vec<&str> = twin_rows.iter().map(|(id, _, _)| id.as_str()).collect();
        assert_eq!(twin_ids, roster.as_slice(), "the twin rows keep id order");
    }
    let _ = fs::remove_dir_all(&parent);
}

// --- LNRV-002 -----------------------------------------------------------------

/// LNRV-002 (t-108, LNR-001): the first sweep over this repository as it
/// stands separates the 2026-08-21 crates from the 2026-08-10 rot -
/// `t-102`..`t-105` (and the 2026-08-25 `t-106`/`t-107`) read `pass`
/// against today's Engine, `t-078` and `t-079` read `expired` naming their
/// last-good edition, and every remaining verdict is `red` with lane ids -
/// never a bare, unexplained failure.
#[test]
fn the_first_sweep_separates_the_2026_08_21_crates_from_the_rot() {
    let report = real_sweep_report();
    let rows = rows_of(&report);

    for crate_id in ["t-102", "t-103", "t-104", "t-105", "t-106", "t-107"] {
        let (row, verdict, detail) = rows
            .iter()
            .find(|(id, _, _)| id == crate_id)
            .unwrap_or_else(|| panic!("the report carries a row for {crate_id}"));
        assert_eq!(row, crate_id);
        assert_eq!(verdict, "pass", "{crate_id} reads pass: {detail}");
    }

    for crate_id in ["t-078", "t-079"] {
        let (row, verdict, detail) = rows
            .iter()
            .find(|(id, _, _)| id == crate_id)
            .unwrap_or_else(|| panic!("the report carries a row for {crate_id}"));
        assert_eq!(row, crate_id);
        assert_eq!(
            verdict, "expired",
            "{crate_id} reads expired once marked, never red: {detail}"
        );
        assert!(
            detail.contains("last passed at edition-"),
            "the expired verdict names the last-good edition: {detail}"
        );
        let marker = fs::read_to_string(
            repo_root()
                .join("test-hidden")
                .join(crate_id)
                .join("EXPIRED.toml"),
        )
        .unwrap_or_else(|error| panic!("{crate_id} carries its in-crate marker: {error}"));
        assert!(
            marker.contains("edition = \"edition-"),
            "the marker names the edition the lane last passed at: {marker}"
        );
    }

    for (crate_id, verdict, detail) in &rows {
        match verdict.as_str() {
            "pass" => {}
            "expired" => assert!(
                detail.contains("last passed at edition-"),
                "{crate_id} expired names an edition: {detail}"
            ),
            "red" => {
                let named_refusal = [
                    "build refused:",
                    "timed out",
                    "cargo exited",
                    "cargo did not run",
                ]
                .iter()
                .any(|phrase| detail.contains(phrase));
                let lane_ids = detail.split(", ").all(|token| {
                    token.len() >= 8
                        && token
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                });
                assert!(
                    !detail.trim().is_empty() && (named_refusal || lane_ids),
                    "{crate_id} red carries its failing lane ids or a named refusal: {detail}"
                );
            }
            other => panic!("{crate_id} reads a verdict outside the vocabulary: {other}"),
        }
    }
    let (pass, expired, red, missing, total) = counts_of(&rows);
    assert_eq!(missing, 0, "this repository holds every rostered crate");
    assert_eq!(
        summary_line(&report),
        format!(
            "- verdicts: {pass} pass, {expired} expired, {red} red, {missing} missing - {total} crates"
        )
    );
}

// --- LNRV-003 -----------------------------------------------------------------

/// LNRV-003 (t-108, LNR-002): a crate whose lanes refuse with no marker
/// reads `red`; writing a marker naming an edition flips its verdict to
/// `expired` and moves it out of both the pass count and the red count;
/// `t-078` and `t-079` twins read `expired` once marked, never `red`.
#[test]
fn an_expiry_marker_moves_a_lane_out_of_both_counts_naming_its_edition() {
    let lanes = Lanes::new(
        "marker",
        &["t-078", "t-079", "t-901"],
        &[("t-078", false), ("t-079", false), ("t-901", true)],
    );

    // Unmarked, both refusing twins read red with their failing lane ids.
    let swept = lanes.sweep(&[]);
    assert_eq!(swept.status.code(), Some(1), "a red sweep exits 1");
    let (row, verdict, detail) = lanes.verdict_of("t-078");
    assert_eq!((row.as_str(), verdict.as_str()), ("t-078", "red"));
    assert!(
        detail.contains("ht_078_01_lane"),
        "the red verdict names its failing lane: {detail}"
    );
    let before = counts_of(&lanes.rows());
    assert_eq!(
        before,
        (1, 0, 2, 0, 3),
        "one pass, two red, nothing expired"
    );

    // Marking is the explicit act: it writes exactly the marker's own bytes
    // (LNRV-005 proves the tree side) and names the last-good edition.
    let marked = lanes.mark("t-078", "edition-001");
    assert!(
        marked.status.success(),
        "mark is an explicit act that succeeds: {}",
        combined(&marked)
    );
    lanes.sweep(&[]);
    let (_, verdict, detail) = lanes.verdict_of("t-078");
    assert_eq!(
        verdict, "expired",
        "the marked twin reads expired, never red: {detail}"
    );
    assert!(
        detail.contains("last passed at edition-001"),
        "the expired verdict names the marker's edition: {detail}"
    );
    let after = counts_of(&lanes.rows());
    assert_eq!(after, (1, 1, 1, 0, 3));
    assert_eq!(
        before.0, after.0,
        "the marked crate left the red count without entering the pass count"
    );

    // The same explicit act on the second twin: both read expired, never red.
    lanes.mark("t-079", "edition-001");
    lanes.sweep(&[]);
    for crate_id in ["t-078", "t-079"] {
        let (_, verdict, detail) = lanes.verdict_of(crate_id);
        assert_eq!(
            verdict, "expired",
            "{crate_id} reads expired once marked: {detail}"
        );
    }
    assert_eq!(counts_of(&lanes.rows()), (1, 2, 0, 0, 3));

    // Verify mode runs an expired lane anyway; the marker stays the fact and
    // what the run observed rides beside it, so recovery is visible.
    let verified = lanes.sweep(&["--verify-expired"]);
    assert_eq!(
        verified.status.code(),
        Some(0),
        "verify keeps the marked lanes out of the red count: {}",
        combined(&verified)
    );
    let (_, verdict, detail) = lanes.verdict_of("t-078");
    assert_eq!(
        verdict, "expired",
        "verify keeps the marker's verdict: {detail}"
    );
    assert!(
        detail.contains("verify: lanes refuse"),
        "verify reports what it saw when it ran the expired lane: {detail}"
    );

    // Unmarking is the same explicit act in reverse: the lane is live again,
    // refuses, and returns to the red count.
    let unmarked = lanes.unmark("t-078");
    assert!(
        unmarked.status.success(),
        "unmark succeeds: {}",
        combined(&unmarked)
    );
    lanes.sweep(&[]);
    let (_, verdict, detail) = lanes.verdict_of("t-078");
    assert_eq!(verdict, "red", "the unmarked twin refuses again: {detail}");
    assert_eq!(counts_of(&lanes.rows()), (1, 1, 1, 0, 3));

    // The explicit acts refuse loudly when repeated or aimed at nothing.
    let double = lanes.mark("t-079", "edition-001");
    assert!(
        !double.status.success() && combined(&double).contains("already carries a marker"),
        "marking a marked crate refuses: {}",
        combined(&double)
    );
    let bare = lanes.unmark("t-901");
    assert!(
        !bare.status.success() && combined(&bare).contains("carries no expiry marker"),
        "unmarking a crate with no marker refuses: {}",
        combined(&bare)
    );
}

// --- LNRV-004 -----------------------------------------------------------------

/// The fixture Machine Class's closing State, carrying the prepared guard
/// verbatim: the edition guard's shape one lane over.
const CLOSE_RUNBOOK: &str = r#"
[states.close]
prompt = "Close the turn: every landed lane runnable or visibly expired."
guards = [{ kind = "command_exit", program = "python", args = ["tools/sweep_lanes.py", "check"], expected = 0 }]

[states.rest]
prompt = "Nothing is open."

[[transitions]]
from = "close"
to = "rest"
"#;

/// One line-insertion shape: the patched runbook must equal the original
/// plus exactly one contiguous block, and that block carries exactly one
/// `command_exit` guard.
fn one_contiguous_insertion(original: &str, patched: &str) -> Vec<String> {
    let before: Vec<&str> = original.lines().collect();
    let after: Vec<&str> = patched.lines().collect();
    assert!(after.len() > before.len(), "the patch adds lines");
    let mut start = 0;
    while start < before.len() && start < after.len() && before[start] == after[start] {
        start += 1;
    }
    assert!(
        start < before.len(),
        "the patch inserts, it never rewrites in place"
    );
    let inserted_len = after.len() - before.len();
    let inserted: Vec<String> = after[start..start + inserted_len]
        .iter()
        .map(|line| line.to_string())
        .collect();
    let tail = &after[start + inserted_len..];
    assert_eq!(
        tail,
        before[start..].as_ref(),
        "every line outside the inserted block is byte-identical"
    );
    inserted
}

fn git_in(root: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("invoke git")
}

/// LNRV-004 (t-108, LNR-003): a close whose report names a red, unexpired
/// lane refuses; the same close passes once the lane is green or marked;
/// the Machine Class diff that wires it adds one `command_exit`-class guard
/// and nothing else - prepared beside the runbook, applied to the tracked
/// file only at the next cycle boundary, because a live Run pins it.
#[test]
fn the_close_guard_refuses_only_a_red_unexpired_verdict() {
    let root = repo_root();
    let diff_path = root.join(".ratmac/close-guard.diff");
    let diff = fs::read_to_string(&diff_path)
        .expect("the prepared close-guard diff lands as a tracked file");
    assert!(
        diff.contains("sweep_lanes.py") && diff.contains("expected = 0"),
        "the prepared diff carries the guard it exists to add: {diff}"
    );
    // The boundary arrived: the tracked runbook carries the wired sweep
    // guard exactly once (it was wired after the pinning Run rested). The
    // prepared-diff properties are proven from the reconstructed pre-wiring
    // base: the diff reverse-applies off the wired runbook and
    // forward-applies back onto that base.
    let runbook = fs::read_to_string(root.join(".ratmac/ratmac.toml")).unwrap();
    assert_eq!(
        runbook.matches("sweep_lanes.py").count(),
        1,
        "the tracked runbook carries the wired sweep guard exactly once: {runbook}"
    );

    let apply_root = std::env::temp_dir().join(format!(
        "ratmac-t108-apply-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    fs::create_dir_all(apply_root.join(".ratmac")).expect("create apply fixture");
    fs::write(apply_root.join(".ratmac/ratmac.toml"), &runbook).expect("copy the runbook");
    // Git cannot open Windows verbatim (`\\?\`) paths from the canonicalized
    // repository root, so the fixture applies the diff from its own tree.
    fs::copy(&diff_path, apply_root.join("close-guard.diff"))
        .expect("stage the prepared diff in the apply fixture");
    let reversed = Command::new("git")
        .args(["apply", "-R", "close-guard.diff"])
        .current_dir(&apply_root)
        .output()
        .expect("invoke git apply -R");
    assert!(
        reversed.status.success(),
        "the wired guard reverse-applies to reconstruct the pre-wiring base: {}",
        combined(&reversed)
    );
    let runbook = fs::read_to_string(apply_root.join(".ratmac/ratmac.toml")).unwrap();
    assert!(
        !runbook.contains("sweep_lanes.py"),
        "the pre-wiring base carries no sweep guard: {runbook}"
    );
    assert!(git_in(&apply_root, &["init", "--quiet"]).status.success());
    assert!(git_in(&apply_root, &["add", "-A"]).status.success());
    assert!(
        git_in(&apply_root, &["commit", "--quiet", "-m", "fixture base"])
            .status
            .success()
    );
    // Git cannot open Windows verbatim (`\\?\`) paths from the canonicalized
    // repository root, so the fixture applies the diff from its own tree.
    fs::copy(&diff_path, apply_root.join("close-guard.diff"))
        .expect("stage the prepared diff in the apply fixture");
    let applied = Command::new("git")
        .arg("apply")
        .arg("close-guard.diff")
        .current_dir(&apply_root)
        .output()
        .expect("invoke git apply");
    assert!(
        applied.status.success(),
        "the prepared diff applies to the tracked runbook: {}",
        combined(&applied)
    );
    let patched = fs::read_to_string(apply_root.join(".ratmac/ratmac.toml")).unwrap();
    let inserted = one_contiguous_insertion(&runbook, &patched);
    let command_guards_added = inserted
        .iter()
        .filter(|line| line.contains("{ kind = \"command_exit\""))
        .count();
    assert_eq!(
        command_guards_added, 1,
        "the diff adds exactly one command_exit guard: {inserted:?}"
    );
    assert_eq!(
        runbook.matches("{ kind = \"command_exit\"").count() + 1,
        patched.matches("{ kind = \"command_exit\"").count(),
        "one guard more than the shipped runbook, no guard removed"
    );
    assert!(
        inserted.iter().any(|line| line.contains("sweep_lanes.py"))
            && inserted.iter().any(|line| line.contains("expected = 0")),
        "the added guard reads the sweep's verdict: {inserted:?}"
    );

    // The patched Machine Class stays valid: over the roots it declares,
    // the doctor reports no findings at all, so the wiring adds one guard
    // and breaks no rule.
    for declared in [
        ".arca/goal",
        ".arca/issue",
        ".arca/residual",
        ".arca/ticket",
    ] {
        fs::create_dir_all(apply_root.join(declared)).expect("create the declared root");
    }
    let doctor = Command::new(ratmac_qa::engine_bin!())
        .arg("doctor")
        .arg(apply_root.join(".ratmac/ratmac.toml"))
        .output()
        .expect("invoke rtm doctor");
    assert!(
        doctor.status.success() && !combined(&doctor).contains("RB"),
        "the patched runbook doctors clean: {}",
        combined(&doctor)
    );

    // Behavioral proof on a fixture Machine Class carrying the prepared
    // guard, driven through the real Engine.
    let close = Lanes::new(
        "close",
        &["t-901", "t-902"],
        &[("t-901", true), ("t-902", false)],
    );
    fs::create_dir_all(close.root.join(".arca/goal")).expect("create fixture goal");
    fs::write(close.root.join(".arca/goal/spec.md"), "# Fixture goal\n")
        .expect("write fixture goal");
    fs::write(close.root.join(".gitignore"), ".ratmac/\ntest-hidden/\n")
        .expect("the Engine's runtime is never tracked");
    fs::write(close.root.join(".ratmac/ratmac.toml"), CLOSE_RUNBOOK)
        .expect("write fixture Machine Class");
    assert!(git_in(&close.root, &["init", "--quiet"]).status.success());
    for args in [
        &["config", "user.email", "close@example.invalid"][..],
        &["config", "user.name", "close fixture"][..],
        &["config", "core.autocrlf", "false"][..],
    ] {
        assert!(git_in(&close.root, args).status.success());
    }
    assert!(git_in(&close.root, &["add", "-A"]).status.success());
    assert!(
        git_in(&close.root, &["commit", "--quiet", "-m", "fixture base"])
            .status
            .success()
    );

    let rtm = |args: &[&str]| -> Output {
        Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&close.root)
            .output()
            .expect("invoke rtm")
    };
    let start = |_fixture: &Lanes| -> String {
        let output = rtm(&["start"]);
        let text = combined(&output);
        assert!(output.status.success(), "start succeeds: {text}");
        text.split("started run ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .expect("start names the minted run id")
            .to_owned()
    };
    let step = |run: &str| combined(&rtm(&["step", "--run", run]));
    let record = |run: &str| {
        fs::read_to_string(close.root.join(format!(".ratmac/runs/{run}/run.toml")))
            .expect("read the Run Record")
    };

    // Red, unexpired: the close refuses and stays.
    close.sweep(&[]);
    let run = start(&close);
    let refusal = step(&run);
    assert!(
        refusal.contains("step refused") && refusal.contains("command_exit"),
        "the close refuses on a red unexpired verdict, naming the guard: {refusal}"
    );
    assert!(
        refusal.contains("python") && refusal.contains("observed exit"),
        "the refusal names the command it ran and the exit it saw: {refusal}"
    );
    assert!(
        refusal.contains("red, unexpired verdict: t-902"),
        "the refusal names the red lane through the check's own words: {refusal}"
    );
    assert!(
        record(&run).contains("state = \"close\""),
        "a refused close leaves the Run in the closing State"
    );

    // Marked: the same close passes once the lane carries a marker.
    close.mark("t-902", "edition-001");
    close.sweep(&[]);
    let marked_run = start(&close);
    let marked_step = step(&marked_run);
    assert!(
        !marked_step.contains("step refused"),
        "the close passes once the lane is marked: {marked_step}"
    );
    assert!(record(&marked_run).contains("state = \"rest\""));

    // Green: unmark the lane, repair it, sweep, and the close passes again.
    close.unmark("t-902");
    fs::write(
        close.crate_dir("t-902").join("src/lib.rs"),
        "#[test]\nfn ht_902_01_lane() {\n    assert!(true, \"repaired\");\n}\n",
    )
    .expect("repair the fixture lane");
    close.sweep(&[]);
    let (_, verdict, _) = close.verdict_of("t-902");
    assert_eq!(verdict, "pass", "the repaired lane reads pass");
    let green_run = start(&close);
    let green_step = step(&green_run);
    assert!(
        !green_step.contains("step refused"),
        "the close passes once the lane is green: {green_step}"
    );

    // Absence is never a pass: no report, and a doctored report, each refuse.
    let report_path = close.root.join(".ratmac/evidence/lane-sweep/report.md");
    fs::remove_file(&report_path).expect("remove the fixture report");
    let absent_run = start(&close);
    let absent_step = step(&absent_run);
    assert!(
        absent_step.contains("step refused") && absent_step.contains("no lane sweep report"),
        "the close refuses with no report to read: {absent_step}"
    );
    close.sweep(&[]);
    let whole = fs::read_to_string(&report_path).unwrap();
    let doctored = whole.replacen("- verdicts: ", "- verdicts: 9 pass, ", 1);
    assert_ne!(doctored, whole, "the fixture edits the total");
    fs::write(&report_path, doctored).expect("doctor the fixture report");
    let doctored_run = start(&close);
    let doctored_step = step(&doctored_run);
    assert!(
        doctored_step.contains("step refused") && doctored_step.contains("the summary says"),
        "the close refuses on a doctored report: {doctored_step}"
    );

    let _ = fs::remove_dir_all(&apply_root);
}

// --- LNRV-005 -----------------------------------------------------------------

/// Every file under `root` except declared build output (any `target`
/// directory, the turn lifecycle's skip list) and `except` - the byte-level
/// snapshot LNRV-005 compares.
fn snapshot_tree(root: &Path, except: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(directory: &Path, base: &Path, except: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries: Vec<PathBuf> = fs::read_dir(directory)
            .expect("read snapshot directory")
            .map(|entry| entry.expect("read snapshot entry").path())
            .collect();
        entries.sort();
        for path in entries {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            if name == ".git" || (path.is_dir() && name == SKIP) {
                continue;
            }
            if path.is_dir() {
                walk(&path, base, except, files);
            } else if path != except {
                let relative = path
                    .strip_prefix(base)
                    .expect("path under the snapshot root")
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(relative, fs::read(&path).expect("read snapshot file"));
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, except, &mut files);
    files
}

/// LNRV-005 (t-108, LNR-001 and LNR-002): a tree snapshot around a full
/// sweep is byte-identical apart from the report artifact; marking and
/// unmarking change exactly the marker's own bytes.
#[test]
fn a_sweep_changes_only_the_report_and_a_marker_changes_only_its_own_bytes() {
    let root = repo_root();
    let report_path = root.join(".ratmac/evidence/lane-sweep/report.md");

    // A full sweep over this repository: only the report artifact differs.
    let _guard = SWEEP_MUTEX
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let before = snapshot_tree(&root, &report_path);
    let swept = run_sweep(&root, &["sweep"]);
    assert!(
        swept.status.code().is_some_and(|code| code <= 1),
        "the full sweep completes: {}",
        combined(&swept)
    );
    let after = snapshot_tree(&root, &report_path);
    assert_eq!(
        before, after,
        "the sweep writes nothing but its report artifact"
    );

    // A marker changes exactly the marker's own bytes: one new file inside
    // the crate it marks, nothing else moved, and unmarking restores it.
    let lanes = Lanes::new(
        "bytes",
        &["t-901", "t-902"],
        &[("t-901", true), ("t-902", false)],
    );
    lanes.sweep(&[]);
    let nowhere = lanes.root.join("nowhere");
    let before_marker = snapshot_tree(&lanes.root, &nowhere);
    assert!(
        lanes.mark("t-902", "edition-001").status.success(),
        "marking succeeds on the fixture"
    );
    let with_marker = snapshot_tree(&lanes.root, &nowhere);
    let marker_relative = "test-hidden/t-902/EXPIRED.toml";
    let mut added: Vec<String> = with_marker
        .keys()
        .filter(|key| !before_marker.contains_key(*key))
        .cloned()
        .collect();
    added.sort();
    assert_eq!(
        added,
        vec![marker_relative.to_owned()],
        "marking adds exactly the marker's own file"
    );
    let removed: Vec<&String> = before_marker
        .keys()
        .filter(|key| !with_marker.contains_key(*key))
        .collect();
    assert!(removed.is_empty(), "marking removes nothing: {removed:?}");
    let marker = with_marker
        .get(marker_relative)
        .expect("the marker file exists");
    assert!(
        String::from_utf8_lossy(marker).contains("edition-001"),
        "the marker's bytes name the edition: {}",
        String::from_utf8_lossy(marker)
    );
    for (key, bytes) in &before_marker {
        assert_eq!(
            with_marker.get(key).expect("unchanged file survives"),
            bytes,
            "marking touched no file but the marker: {key}"
        );
    }
    assert!(
        lanes.unmark("t-902").status.success(),
        "unmarking succeeds on the fixture"
    );
    let after_unmark = snapshot_tree(&lanes.root, &nowhere);
    assert_eq!(
        before_marker, after_unmark,
        "unmarking restores the tree exactly"
    );
}
