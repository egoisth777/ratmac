//! t-108 / LNR-001..LNR-003: the landed-lane sweep reports one verdict per
//! crate, an expiry marker moves a refusing lane out of both counts, and the
//! prepared close guard refuses only a red, unexpired verdict.
//!
//! LNRV-001 `the_sweep_reports_one_verdict_per_crate_and_names_a_missing_crate`
//! LNRV-002 `the_first_sweep_separates_the_2026_08_21_crates_from_the_rot`
//! LNRV-003 `an_expiry_marker_moves_a_lane_out_of_both_counts_naming_its_edition`
//! LNRV-004 `the_close_guard_refuses_only_a_red_unexpired_verdict`
//! LNRV-005 `a_sweep_changes_only_the_report_and_a_marker_changes_only_its_own_bytes`
//! SVEV-019 `svev_019` (t-138 / SVE-003: sweep crates build apart)
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
//! process through `real_sweep` - LNRV-005 reads the same genuinely executed
//! sweep and its bracketing tree snapshots, reused only while the tree still
//! matches the one that sweep left (WCP-002); each crate builds in a folder
//! of its own - its own target directory, or a folder named after it inside
//! the folder a Cargo configuration names (SVE-003) - so a sweep re-executes
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
//! - Would SVEV-019 pass a sweep that puts both crates in one folder inside
//!   the configured folder, or names a crate's folder after anything but
//!   its id? No. Each test executable must sit at `<configured folder>/<crate
//!   id>/debug/deps`, neither crate's package-named output may appear in the
//!   other crate's folder, and the same fixture must still build in each
//!   crate's own `target` without the setting and in the one named folder
//!   under `--target-dir`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf, Prefix};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use ratmac_qa::lane_runs;
use ratmac_qa::support::TempTree;
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

/// One genuinely executed sweep over this repository, bracketed by the
/// LNRV-005 tree snapshots (the report artifact excluded).
struct RealSweep {
    before: BTreeMap<String, Vec<u8>>,
    after: BTreeMap<String, Vec<u8>>,
    /// `lane_runs::outside_digest` before and after the run: Cargo
    /// configuration outside the checkout that the snapshots cannot see.
    outside_before: String,
    outside_after: String,
    code: Option<i32>,
    text: String,
    report: Option<String>,
}

/// The real sweep over this repository as it stands: every rostered crate
/// under the declared lanes root. Real sweeps serialize on the process-wide
/// lane lock - they run the same lanes, and the snapshots bracket exactly
/// one run. A recorded sweep is reused only when the tree did not move while
/// it ran and the tree, the Cargo configuration outside it, and the report
/// still match what it left; otherwise the sweep runs again. LNRV-005 still
/// asserts the no-movement fact itself.
fn real_sweep() -> Arc<RealSweep> {
    static SWEPT: Mutex<Option<Arc<RealSweep>>> = Mutex::new(None);
    let _lanes = lane_runs::exclusive();
    let root = repo_root();
    let report_path = root.join(".ratmac/evidence/lane-sweep/report.md");
    let current = snapshot_tree(&root, &report_path);
    let outside = lane_runs::outside_digest(&root);
    let mut swept = SWEPT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(sweep) = swept.as_ref() {
        if sweep.before == sweep.after
            && sweep.after == current
            && sweep.outside_before == sweep.outside_after
            && sweep.outside_after == outside
            && fs::read_to_string(&report_path).ok() == sweep.report
        {
            return Arc::clone(sweep);
        }
    }
    let output = run_sweep(&root, &["sweep"]);
    let sweep = Arc::new(RealSweep {
        before: current,
        after: snapshot_tree(&root, &report_path),
        outside_before: outside,
        outside_after: lane_runs::outside_digest(&root),
        code: output.status.code(),
        text: combined(&output),
        report: fs::read_to_string(&report_path).ok(),
    });
    *swept = Some(Arc::clone(&sweep));
    sweep
}

fn real_sweep_report() -> String {
    let sweep = real_sweep();
    assert!(
        sweep.code.is_some_and(|code| code <= 1),
        "the sweep over this repository completes (red or missing verdicts \
         exit 1, tool errors exit 2): {}",
        sweep.text
    );
    sweep
        .report
        .clone()
        .expect("the sweep wrote its report under the declared root")
}

// --- fixture lanes roots ------------------------------------------------------

/// A throwaway lanes root carrying the shipped sweep script, its declared
/// data, and small crates whose lanes pass or refuse on purpose.
struct Lanes {
    _tree: TempTree,
    root: PathBuf,
}

impl Lanes {
    /// `crates` pairs a crate id with whether its single lane passes; a
    /// refusing lane names itself so the report's red detail carries ids.
    fn new(label: &str, roster: &[&str], crates: &[(&str, bool)]) -> Self {
        let tree = TempTree::new(&format!("t108-{label}")).expect("create the lanes root");
        let root = tree.path().to_path_buf();
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

        Lanes { _tree: tree, root }
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

/// LNRV-001 (t-108, LNR-001): report replacement preserves complete verdicts
/// through a bounded Windows denial without broadening other failure paths.
#[test]
fn the_sweep_keeps_complete_verdicts_across_report_replace_denials() {
    let lanes = Lanes::new(
        "report-replace-denials",
        &["t-002", "t-001"],
        &[("t-001", false), ("t-002", false)],
    );
    let mut markers = Vec::new();
    for crate_id in ["t-001", "t-002"] {
        let marked = lanes.mark(crate_id, "edition-001");
        assert!(marked.status.success(), "{}", combined(&marked));
        let path = lanes.crate_dir(crate_id).join("EXPIRED.toml");
        markers.push((path.clone(), fs::read(path).expect("read fixture marker")));
    }

    let control = lanes.sweep(&[]);
    assert!(
        control.status.success(),
        "unobstructed expired-crate control: {}",
        combined(&control)
    );
    let control_report = lanes.report();
    let control_rows = rows_of(&control_report);
    assert_eq!(counts_of(&control_rows), (0, 2, 0, 0, 2));
    assert_eq!(
        control_rows
            .iter()
            .map(|(crate_id, _, _)| crate_id.as_str())
            .collect::<Vec<_>>(),
        ["t-001", "t-002"]
    );
    assert_eq!(
        summary_line(&control_report),
        "- verdicts: 0 pass, 2 expired, 0 red, 0 missing - 2 crates"
    );
    println!("unobstructed control passed: two complete expired verdicts and total");

    // The imported copy owns every hold, clock, and error. No Python cache or
    // process-wide hook can move the checkout or interfere with sibling tests.
    const PROBES: &str = r#"
import ctypes
import importlib.util
import math
import os
from pathlib import Path
import sys
from types import SimpleNamespace

spec = importlib.util.spec_from_file_location('sweep_under_test', sys.argv[1])
sweep = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sweep)
real_replace = os.replace
root = Path.cwd()
config = sweep.load_config(str(root / '.ratmac/lanes.toml'))
config.update(lanes_root=str(root / config['lanes']), verify=False,
              timeout=1, target_dir=None)
report = root / config['report']
config['report'] = str(report)
roster = ['t-001', 't-002']
assert config['roster'] == roster
summary = '- verdicts: 0 pass, 2 expired, 0 red, 0 missing - 2 crates'

def complete(text, verify=False):
    assert text.startswith('# Lane sweep report\n') and text.endswith('\n'), text
    assert text.count('# Lane sweep report') == 1, text
    assert text.splitlines().count(summary) == 1, text
    assert f'- verify-expired: {"yes" if verify else "no"}' in text.splitlines(), text
    rows = [line for line in text.splitlines() if line.startswith('| t-')]
    assert len(rows) == len(roster), rows
    for crate, row in zip(roster, rows):
        assert row.startswith(f'| {crate} | expired | last passed at edition-001 '), row

def clean(destination):
    leftovers = list(destination.parent.glob('.lane-sweep-*.tmp'))
    assert not leftovers, f'leftover temporary reports: {leftovers}'

def inspect(source, destination, expected_destination, old, attempts, verify=False):
    source, destination = Path(source), Path(destination)
    assert destination == expected_destination, (destination, expected_destination)
    assert source.parent == destination.parent, (source, destination)
    assert source.name.startswith('.lane-sweep-') and source.suffix == '.tmp', source
    assert destination.read_bytes() == old, 'old report changed before atomic replacement'
    body = source.read_bytes()
    complete(body.decode('utf-8'), verify)
    if attempts:
        assert (source, body) == attempts[0], 'retry rebuilt or changed the staged report'
    attempts.append((source, body))

class DeleteDeny:
    def __init__(self, path):
        from ctypes import wintypes
        kernel = ctypes.WinDLL('kernel32', use_last_error=True)
        create = kernel.CreateFileW
        create.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD,
                           wintypes.LPVOID, wintypes.DWORD, wintypes.DWORD,
                           wintypes.HANDLE]
        create.restype = wintypes.HANDLE
        self.close_handle = kernel.CloseHandle
        self.close_handle.argtypes = [wintypes.HANDLE]
        self.close_handle.restype = wintypes.BOOL
        # GENERIC_READ; FILE_SHARE_READ | FILE_SHARE_WRITE, NOT FILE_SHARE_DELETE.
        self.handle = create(str(path), 0x80000000, 0x3, None, 3, 0, None)
        if self.handle == ctypes.c_void_p(-1).value:
            raise ctypes.WinError(ctypes.get_last_error())

    def close(self):
        if self.handle is not None:
            assert self.close_handle(self.handle), ctypes.WinError(ctypes.get_last_error())
            self.handle = None

def native_recovery():
    old = report.read_bytes() + b'\nprevious complete publication\n'
    report.write_bytes(old)
    held = DeleteDeny(report)
    attempts, denials, successes = [], [], []
    sweep.os = SimpleNamespace(**vars(os))
    def replace(source, destination):
        inspect(source, destination, report, old, attempts)
        try:
            real_replace(source, destination)
        except PermissionError as error:
            assert error.winerror == 5, repr(error)
            denials.append(error)
            print(f'native denied-attempt observed: PermissionError winerror={error.winerror}',
                  flush=True)
            held.close()
            raise
        successes.append(destination)
    sweep.os.replace = replace
    try:
        result = sweep.main(['sweep'])
    except PermissionError as error:
        assert denials and error is denials[-1], 'failure did not reach the native deny'
        assert report.read_bytes() == old, 'single-try failure damaged the old report'
        clean(report)
        raise AssertionError('native WinError5 was observed and released, but the same sweep '
                             'failed instead of publishing its complete report') from error
    finally:
        held.close()
    assert result == 0, result
    assert denials and len(successes) == 1 and len(attempts) >= 2, attempts
    assert report.read_bytes() == attempts[0][1], 'published report differs from staged report'
    complete(report.read_text(encoding='utf-8'))
    clean(report)
    print('native recovery passed: one atomic publication after a genuine WinError5', flush=True)

complete(report.read_text(encoding='utf-8'))
clean(report)
# Keep this first after the real, unobstructed control: the baseline must fail
# on an observed native denial, before any injected error or clock is involved.
if os.name == 'nt':
    native_recovery()
else:
    print('native Windows handle case not applicable on this host', flush=True)

config['report'] = str(report.parent / 'clock-probes' / 'report.md')
probe_report = Path(config['report'])
probe_report.parent.mkdir(parents=True, exist_ok=True)
rows = [(crate, 'expired', 'last passed at edition-001 (marked 2026-08-10)')
        for crate in roster]

class Clock:
    def __init__(self):
        self.now, self.reads, self.sleeps = 100.0, 0, []
        self.advance = lambda delay: setattr(self, 'now', self.now + delay)
    def monotonic(self):
        self.reads += 1
        assert self.reads < 100, 'retry loop did not stop under the controlled clock'
        return self.now
    def sleep(self, delay):
        assert 0 < delay <= 0.010000001, f'not the bounded ten-millisecond interval: {delay}'
        self.sleeps.append(delay)
        self.advance(delay)

def install(clock, windows=True):
    sweep.os = SimpleNamespace(**vars(os))
    sweep.os.name = 'nt' if windows else 'posix'
    sweep.sys = SimpleNamespace(**vars(sys))
    sweep.sys.platform = 'win32' if windows else 'linux'
    sweep.time = SimpleNamespace(monotonic=clock.monotonic, sleep=clock.sleep)

def denied(label, code=5):
    error = PermissionError(13, label)
    error.winerror = code
    return error

def seed():
    clean(probe_report)
    old = report.read_bytes() + b'\nold complete probe publication\n'
    probe_report.write_bytes(old)
    return old

def unchanged(old):
    assert probe_report.read_bytes() == old, 'failed publication changed the old report'
    clean(probe_report)

def expect_error(expected, action):
    try:
        action()
    except BaseException as error:
        assert error is expected, f'expected original/last error {expected!r}; got {error!r}'
    else:
        raise AssertionError(f'expected publication failure: {expected!r}')

def first_denial_origin():
    clock, attempts, old = Clock(), [], seed()
    install(clock)
    error = denied('first denial after slow staging/replace work')
    def replace(source, destination):
        inspect(source, destination, probe_report, old, attempts)
        if len(attempts) == 1:
            clock.now += 20.0
            raise error
        assert len(attempts) == 2, attempts
        assert clock.now < 125.0, 'retry started after the first-denial deadline'
        real_replace(source, destination)
    sweep.os.replace = replace
    text = sweep.write_report(config, rows, [])
    assert len(attempts) == 2 and clock.sleeps == [0.01], clock.sleeps
    assert probe_report.read_bytes() == attempts[0][1] == text.encode('utf-8')
    complete(text)
    clean(probe_report)
    print('first-denial origin passed: pre-denial time does not consume the window', flush=True)

def fixed_deadline_and_last_error():
    clock, attempts, errors, old = Clock(), [], [], seed()
    install(clock)
    deadline = clock.now + 5.0
    def advance(delay):
        if len(clock.sleeps) == 1:
            assert math.isclose(delay, 0.01, abs_tol=1e-9), delay
            clock.now = deadline - 0.005
        else:
            assert len(clock.sleeps) == 2, 'deadline restarted after another denial'
            assert math.isclose(delay, 0.005, abs_tol=1e-9), delay
            clock.now = deadline
    clock.advance = advance
    def replace(source, destination):
        assert clock.now < deadline, 'retry started at or after the five-second deadline'
        inspect(source, destination, probe_report, old, attempts)
        assert len(attempts) <= 2, 'deadline restarted'
        error = denied(f'lasting denial {len(attempts)}')
        errors.append(error)
        raise error
    sweep.os.replace = replace
    try:
        sweep.write_report(config, rows, [])
    except PermissionError as error:
        assert len(errors) == 2 and error is errors[-1], 'did not raise the last denial'
    else:
        raise AssertionError('lasting denial unexpectedly published a report')
    assert clock.now == deadline and len(clock.sleeps) == 2, clock.sleeps
    unchanged(old)
    print('fixed five-second deadline passed: clipped final wait and last error', flush=True)

def overslept_deadline():
    clock, attempts, old = Clock(), [], seed()
    install(clock)
    error = denied('holder released only after an overslept deadline')
    clock.advance = lambda delay: setattr(clock, 'now', 105.001)
    def replace(source, destination):
        inspect(source, destination, probe_report, old, attempts)
        if len(attempts) == 1:
            raise error
        # A late attempt would really succeed; it must never start.
        real_replace(source, destination)
    sweep.os.replace = replace
    expect_error(error, lambda: sweep.write_report(config, rows, []))
    assert len(attempts) == 1 and clock.sleeps == [0.01], clock.sleeps
    unchanged(old)
    print('overslept deadline passed: no late retry even after release', flush=True)

def other_errors_are_single_try():
    cases = [
        (True, denied('Windows sharing violation is not access denied', 32)),
        (True, PermissionError(13, 'permission error without a Windows code')),
        (True, OSError(5, 'errno 5 is not winerror 5')),
        (True, FileNotFoundError(2, 'missing target parent')),
        (False, denied('non-Windows error with a synthetic winerror attribute')),
        (False, PermissionError(13, 'ordinary non-Windows permission denial')),
    ]
    for windows, error in cases:
        clock, attempts, old = Clock(), [], seed()
        install(clock, windows)
        def replace(source, destination):
            inspect(source, destination, probe_report, old, attempts)
            assert len(attempts) == 1, f'broadened retry: {error!r}'
            raise error
        sweep.os.replace = replace
        expect_error(error, lambda: sweep.write_report(config, rows, []))
        assert len(attempts) == 1 and not clock.sleeps, (error, clock.sleeps)
        unchanged(old)
    # Eligibility must also be rechecked after an eligible first denial.
    clock, attempts, old = Clock(), [], seed()
    install(clock)
    errors = [denied('eligible first denial'), denied('subsequent error 32', 32)]
    def replace(source, destination):
        inspect(source, destination, probe_report, old, attempts)
        assert len(attempts) <= 2, 'retried an ineligible subsequent error'
        raise errors[len(attempts) - 1]
    sweep.os.replace = replace
    expect_error(errors[-1], lambda: sweep.write_report(config, rows, []))
    assert len(attempts) == 2 and clock.sleeps == [0.01], clock.sleeps
    unchanged(old)
    print('classification passed: only Windows PermissionError winerror 5 retries', flush=True)

def recovery_does_not_rerun_crates():
    clock, attempts, calls, old = Clock(), [], [], seed()
    install(clock)
    error = denied('recoverable publication after all verdicts are computed')
    config['verify'] = True
    def run_crate(current_config, crate, timeout):
        assert current_config is config and timeout == 1
        calls.append(crate)
        assert calls.count(crate) == 1, f'publication reran crate {crate}'
        return 'pass', ''
    sweep.run_crate = run_crate
    def replace(source, destination):
        inspect(source, destination, probe_report, old, attempts, verify=True)
        assert calls == roster, 'publication began before all verdicts were computed'
        if len(attempts) == 1:
            raise error
        assert len(attempts) == 2, attempts
        real_replace(source, destination)
    sweep.os.replace = replace
    assert sweep.sweep(config) == 0
    assert calls == roster and len(attempts) == 2, (calls, attempts)
    assert probe_report.read_bytes() == attempts[0][1]
    complete(probe_report.read_text(encoding='utf-8'), verify=True)
    clean(probe_report)
    print('no-crate-rerun passed: each verified expired crate ran exactly once', flush=True)

first_denial_origin()
fixed_deadline_and_last_error()
overslept_deadline()
other_errors_are_single_try()
recovery_does_not_rerun_crates()
"#;
    let probes = Command::new("python")
        .args(["-B", "-c", PROBES])
        .arg(lanes.root.join("tools/sweep_lanes.py"))
        .current_dir(&lanes.root)
        .stdin(std::process::Stdio::null())
        .output()
        .expect("invoke child-local report replacement probes");
    print!("{}", combined(&probes));
    assert!(
        probes.status.success(),
        "report replacement probes failed: {}",
        probes.status
    );
    let published = lanes.report();
    assert_eq!(counts_of(&rows_of(&published)), (0, 2, 0, 0, 2));
    assert_eq!(published.matches("# Lane sweep report").count(), 1);
    assert_eq!(published.matches("- verdicts: ").count(), 1);
    for (path, before) in markers {
        assert_eq!(
            fs::read(&path).expect("read marker after report replacement probes"),
            before,
            "report publication changed expiry marker {}",
            path.display()
        );
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
    let parent = TempTree::new("t108-twin").expect("create the twin tree");
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
        let _guard = lane_runs::exclusive();
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

    let apply_tree = TempTree::new("t108-apply").expect("create the apply tree");
    let apply_root = apply_tree.path().to_path_buf();
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
    fs::write(
        close.root.join(".gitignore"),
        ".ratmac/runs/\n.ratmac/mint.toml\n.ratmac/locks/\n.ratmac/log.md\ntest-hidden/\n",
    )
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
    // A full sweep over this repository: only the report artifact differs.
    let swept = real_sweep();
    assert!(
        swept.code.is_some_and(|code| code <= 1),
        "the full sweep completes: {}",
        swept.text
    );
    assert_eq!(
        swept.before, swept.after,
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

// --- SVEV-019 -----------------------------------------------------------------

/// The SVEV-019 fixture crates: each crate id with its package name. The
/// package names differ from the ids and from each other, so either crate's
/// build output can be told apart by name in any folder.
const BUILD_APART_CRATES: [(&str, &str); 2] =
    [("t-001", "svev019_first"), ("t-002", "svev019_second")];

/// Each fixture crate's one lane: it writes the path of the test executable
/// it runs as into `executable.txt` inside its own crate.
const RECORDING_LANE: &str = r#"#[test]
fn records_its_test_executable() {
    let executable = std::env::current_exe().expect("the test executable has a path");
    let path = executable.to_str().expect("the test executable's path is UTF-8");
    let record = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("executable.txt");
    std::fs::write(record, path).expect("record the test executable's path");
}
"#;

/// The SVEV-019 fixture: a lanes root of two standard-library-only crates
/// below a `.cargo/config.toml` whose `build.target-dir` names the
/// configured folder, a tree of its own outside the fixture; a third tree is
/// the folder the explicit `--target-dir` opt-in names. All three trees are
/// removed when this value drops.
struct BuildApart {
    root: TempTree,
    configured: TempTree,
    named: TempTree,
}

impl BuildApart {
    fn new() -> Self {
        let root = TempTree::new("svev019-fixture").expect("create the SVEV-019 fixture");
        let configured = TempTree::new("svev019-configured").expect("create the configured folder");
        let named = TempTree::new("svev019-named").expect("create the named folder");
        root.write(
            ".ratmac/lanes.toml",
            "# The SVEV-019 fixture lane declaration - the runbook's [roots] shape.\n\
             [roots]\nlanes = \"test-hidden\"\n\
             report = \".ratmac/evidence/lane-sweep/report.md\"\n\
             roster = [\"t-001\", \"t-002\"]\n",
        );
        for (crate_id, package) in BUILD_APART_CRATES {
            root.write(
                format!("test-hidden/{crate_id}/Cargo.toml"),
                format!(
                    "# SVEV-019 fixture lane crate for {crate_id}.\n[package]\n\
                     name = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
                     publish = false\n\n[workspace]\n"
                ),
            );
            root.write(format!("test-hidden/{crate_id}/src/lib.rs"), RECORDING_LANE);
        }
        let fixture = Self {
            root,
            configured,
            named,
        };
        fixture.set_configured(true);
        fixture
    }

    fn crate_dir(&self, crate_id: &str) -> PathBuf {
        self.root.join("test-hidden").join(crate_id)
    }

    /// Put the setting in place - `.cargo/config.toml` in the fixture root,
    /// above both crates, with `build.target-dir` naming the configured
    /// folder - or take it away.
    fn set_configured(&self, setting: bool) {
        if setting {
            let folder = self
                .configured
                .path()
                .to_str()
                .expect("the configured folder's path is UTF-8");
            assert!(
                !folder.contains('\''),
                "the configured folder fits a TOML literal string: {folder}"
            );
            self.root.write(
                ".cargo/config.toml",
                format!(
                    "# Every build below this folder goes to one configured folder.\n\
                     [build]\ntarget-dir = '{folder}'\n"
                ),
            );
        } else {
            let file = self.root.join(".cargo/config.toml");
            fs::remove_file(&file)
                .unwrap_or_else(|error| panic!("take away {}: {error}", file.display()));
        }
    }

    /// One `sweep` through the shipped script and the fixture's declaration,
    /// `extra` after the verb. Both crates must read green and each lane must
    /// have recorded its test executable afresh; the recorded paths come back
    /// in roster order.
    fn sweep(&self, extra: &[&str], what: &str) -> [PathBuf; 2] {
        let report = self.root.join(".ratmac/evidence/lane-sweep/report.md");
        // Nothing an earlier sweep left may pass for this sweep's report or
        // records.
        let _ = fs::remove_file(&report);
        for (crate_id, _) in BUILD_APART_CRATES {
            let _ = fs::remove_file(self.crate_dir(crate_id).join("executable.txt"));
        }
        let declaration = self.root.join(".ratmac/lanes.toml");
        let mut args = vec![
            "sweep",
            "--config",
            declaration.to_str().expect("the fixture's path is UTF-8"),
        ];
        args.extend_from_slice(extra);
        let output = run_sweep(self.root.path(), &args);
        let text = fs::read_to_string(&report).unwrap_or_else(|error| {
            panic!(
                "SVEV-019: {what} wrote no report at {} ({error}); the sweep said: {}",
                report.display(),
                combined(&output)
            )
        });
        let rows = rows_of(&text);
        let verdicts: Vec<(&str, &str)> = rows
            .iter()
            .map(|(crate_id, verdict, _)| (crate_id.as_str(), verdict.as_str()))
            .collect();
        assert!(
            output.status.code() == Some(0) && verdicts == [("t-001", "pass"), ("t-002", "pass")],
            "SVEV-019: {what} must read both crates green; observed exit {:?} and rows \
             {rows:?}; the sweep said: {}",
            output.status.code(),
            combined(&output)
        );
        BUILD_APART_CRATES.map(|(crate_id, _)| {
            let record = self.crate_dir(crate_id).join("executable.txt");
            let recorded = fs::read_to_string(&record).unwrap_or_else(|error| {
                panic!(
                    "SVEV-019: in {what}, {crate_id}'s lane recorded no test executable at {} \
                     ({error})",
                    record.display()
                )
            });
            PathBuf::from(recorded.trim())
        })
    }
}

/// A path as comparable parts. `canonicalize` resolves short names and
/// letter case the way the file system does; the `\\?\` prefix it adds on
/// Windows is reduced to the plain drive or share, the same way on both
/// sides of every comparison.
fn path_parts(path: &Path) -> Vec<String> {
    let resolved = path
        .canonicalize()
        .unwrap_or_else(|error| panic!("SVEV-019: resolve {}: {error}", path.display()));
    resolved
        .components()
        .map(|component| match component {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                    format!("{}:", char::from(letter).to_ascii_uppercase())
                }
                Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => format!(
                    r"\\{}\{}",
                    server.to_string_lossy(),
                    share.to_string_lossy()
                ),
                _ => component.as_os_str().to_string_lossy().into_owned(),
            },
            other => other.as_os_str().to_string_lossy().into_owned(),
        })
        .collect()
}

/// The folder Cargo built a test executable in, as path parts: a test
/// executable sits at `<build folder>/debug/deps/<name>`.
fn build_folder(executable: &[String]) -> Option<&[String]> {
    match executable {
        [folder @ .., debug, deps, _name] if debug == "debug" && deps == "deps" => Some(folder),
        _ => None,
    }
}

/// Where a test executable was built, in words, against `folder` (called
/// `called`): in that folder itself, in a folder inside it, or outside it.
fn placement(executable: &[String], folder: &[String], called: &str) -> String {
    match build_folder(executable) {
        Some(built) if built == folder => format!("in {called} itself"),
        Some(built) if built.starts_with(folder) => {
            format!("in {} inside {called}", built[folder.len()..].join("/"))
        }
        Some(_) => format!("outside {called}"),
        None => "outside Cargo's `debug/deps` layout".to_owned(),
    }
}

/// Every path below `folder` whose name carries `name`: a crate's build
/// output (fingerprints, objects, test executables) carries its crate name.
/// A folder that does not exist holds nothing.
fn named_entries(folder: &Path, name: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = Vec::new();
    if folder.is_dir() {
        pending.push(folder.to_path_buf());
    }
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("SVEV-019: list {}: {error}", directory.display()));
        for entry in entries {
            let path = entry.expect("read a build output entry").path();
            if path
                .file_name()
                .is_some_and(|file| file.to_string_lossy().contains(name))
            {
                found.push(path.display().to_string());
            }
            if path.is_dir() {
                pending.push(path);
            }
        }
    }
    found.sort();
    found
}

/// SVEV-019 (t-138, SVE-003): by default the sweep builds each crate in a
/// folder no other crate builds into, even when a Cargo configuration file
/// above both crates sends every build to one folder: each crate's folder
/// then sits inside that configured folder, named after the crate. Without
/// the setting each crate builds in its own `target`, and the explicit
/// `--target-dir` opt-in still sends both crates to the one folder it names.
#[test]
fn svev_019() {
    let fixture = BuildApart::new();
    let [(first, first_package), (second, second_package)] = BUILD_APART_CRATES;

    // One default sweep over the fixture as built, the setting in place,
    // reads both crates green. Where it built them is judged last, after the
    // two sweeps whose readings hold both today and after the change, so a
    // failure there is about the configured folder alone. What each crate's
    // folder inside the configured folder holds is read now, before any later
    // sweep runs.
    let configured_run = fixture.sweep(&[], "the default sweep under the configured folder");
    let crossed = [
        (
            first,
            second,
            named_entries(&fixture.configured.join(first), second_package),
        ),
        (
            second,
            first,
            named_entries(&fixture.configured.join(second), first_package),
        ),
    ];

    // The same root swept again without the setting: each crate builds in
    // its own `target`.
    fixture.set_configured(false);
    let unconfigured_run = fixture.sweep(&[], "the default sweep without the setting");
    let own_targets = BUILD_APART_CRATES.map(|(crate_id, _)| {
        let mut parts = path_parts(&fixture.crate_dir(crate_id));
        parts.push("target".to_owned());
        parts
    });
    let built = unconfigured_run.each_ref().map(|path| path_parts(path));
    assert!(
        build_folder(&built[0]) == Some(&own_targets[0][..])
            && build_folder(&built[1]) == Some(&own_targets[1][..]),
        "SVEV-019: without the setting each crate must build in its own `target`; observed \
         {first}'s test executable at {} ({}) and {second}'s at {} ({})",
        unconfigured_run[0].display(),
        placement(&built[0], &own_targets[0], "its own `target`"),
        unconfigured_run[1].display(),
        placement(&built[1], &own_targets[1], "its own `target`"),
    );

    // And swept again with `--target-dir`. The reading chosen: the setting is
    // put back and the opt-in names a third folder, outside the fixture and
    // the configured folder, so the explicit opt-in must still win over the
    // configured folder and send both crates to the one folder it names, as
    // it does today.
    fixture.set_configured(true);
    let named_folder = fixture
        .named
        .path()
        .to_str()
        .expect("the named folder's path is UTF-8");
    let opted_run = fixture.sweep(
        &["--target-dir", named_folder],
        "the sweep with `--target-dir`",
    );
    let named = path_parts(fixture.named.path());
    let built = opted_run.each_ref().map(|path| path_parts(path));
    assert!(
        built
            .iter()
            .all(|parts| build_folder(parts) == Some(&named[..])),
        "SVEV-019: with `--target-dir {named_folder}` both crates must build in that one named \
         folder; observed {first}'s test executable at {} ({}) and {second}'s at {} ({})",
        opted_run[0].display(),
        placement(&built[0], &named, "the named folder"),
        opted_run[1].display(),
        placement(&built[1], &named, "the named folder"),
    );

    // The default sweep under the configured folder must have built each
    // crate in a folder of its own inside it, named after the crate: the
    // crate's id, so `<configured folder>/t-001`. Today both builds share the
    // configured folder itself.
    let configured = path_parts(fixture.configured.path());
    let expected = BUILD_APART_CRATES.map(|(crate_id, _)| {
        let mut parts = configured.clone();
        parts.push(crate_id.to_owned());
        parts
    });
    let built = configured_run.each_ref().map(|path| path_parts(path));
    let words = built
        .each_ref()
        .map(|parts| placement(parts, &configured, "the configured folder"));
    let finding = if words[0] == words[1] {
        format!("both paths lie {}", words[0])
    } else {
        format!(
            "{first}'s path lies {} and {second}'s {}",
            words[0], words[1]
        )
    };
    assert!(
        build_folder(&built[0]) == Some(&expected[0][..])
            && build_folder(&built[1]) == Some(&expected[1][..]),
        "SVEV-019: the default sweep must build each crate in a folder of its own inside the \
         configured folder {}, named after the crate; observed {first}'s test executable at {} \
         and {second}'s at {}: {finding}",
        fixture.configured.path().display(),
        configured_run[0].display(),
        configured_run[1].display(),
    );

    // Neither crate's build output appears in the other's folder.
    for (owner, intruder, entries) in &crossed {
        assert!(
            entries.is_empty(),
            "SVEV-019: {intruder}'s build output appears in {owner}'s folder inside the \
             configured folder: {entries:?}"
        );
    }
}
