//! t-128 / WEB-007: the deterministic trace channel - the four public
//! checks WEBV-025 through WEBV-028.
//!
//! WEBV-025: with `RATMAC_TRACE` unset, empty, and exactly `0`, every
//! observable - stdout bytes, stderr bytes, exit status, and the filesystem
//! before and after - is byte-identical to the unset baseline and holds no
//! record prefix anywhere. The no-work proof (a disabled `event` never calls
//! its closure, never formats, never allocates) is the counting-allocator
//! unit test in `src/diag.rs`'s `#[cfg(test)]`; this check observes that unit
//! test by reading the module's source bytes, because the public seam
//! `ratmac::diag` does not exist before implementation (raised as an open
//! question, not guessed).
//! WEBV-026: with `RATMAC_TRACE=1`, controlled fixtures exercise root
//! resolution, transition-input edge choice, refusal order, root-before-Run
//! lock order, and mint decisions; records carry the pinned codes and field
//! order, appear only on stderr, and ordinary stdout and state match the
//! disabled run of the same command over the same fixture shape.
//! WEBV-027: byte-equivalent isolated fixtures at different absolute paths
//! and different clocks produce byte-identical record sequences; quoted
//! values escape exactly by the one escaping rule; no timestamp, duration,
//! absolute host path, process id, secret-bearing environment value, or
//! arbitrary child diagnostic appears in a record; mandatory errors keep
//! their established actionable text and absolute paths, asserted separately
//! from the record checks.
//! WEBV-028: an exhaustive source walk that reads the bytes of every file
//! under `src/` finds standard-error emission only in the owning module;
//! invalid toggle values refuse before any work; a failed standard error
//! stream changes no decision; the operation-log fault seam works beside
//! tracing with byte-identical log bytes; and gate children neither inherit
//! the toggle nor observe record text.
//!
//! The toggle is never set on this process's environment: every traced run
//! is a child `Command` carrying its own environment, and every fixture is
//! an owned `TempTree` whose name is unique by construction.

use ratmac_qa::support::{self, TempTree};
use ratmac_qa::tempgit::TempRepo;
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
#[cfg(windows)]
use std::os::windows::ffi::OsStringExt;

/// The record prefix: `rtm-trace:` plus one space, eleven bytes.
const PREFIX: &str = "rtm-trace: ";
/// The toggle this whole suite judges.
const TRACE_VAR: &str = "RATMAC_TRACE";
/// The operation-log fault seam, set beside tracing in WEBV-028.
const OP_LOG_VAR: &str = "RATMAC_TEST_OPERATION_LOG";

// --- shared helpers ----------------------------------------------------------

/// The repository root this suite is compiled against.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The Engine's one displayed spelling: forward slashes.
fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Whether `bytes` contains the record prefix anywhere.
fn contains_prefix(bytes: &[u8]) -> bool {
    bytes
        .windows(PREFIX.len())
        .any(|window| window == PREFIX.as_bytes())
}

/// Whether `bytes` contains `needle` as an exact byte sequence.
fn contains_bytes(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|window| window == needle)
}

/// One contract value escaping of `value`: inside double quotes a backslash
/// renders `\\`, a double quote `\"`, line feed `\n`, carriage return `\r`,
/// tab `\t`; every other byte below 0x20 and byte 0x7F renders `\u00XX` with
/// two lowercase hex digits; every other byte renders verbatim.
fn escape_value(value: &[u8]) -> Vec<u8> {
    let mut escaped = Vec::new();
    for &byte in value {
        match byte {
            b'\\' => escaped.extend_from_slice(b"\\\\"),
            b'"' => escaped.extend_from_slice(b"\\\""),
            b'\n' => escaped.extend_from_slice(b"\\n"),
            b'\r' => escaped.extend_from_slice(b"\\r"),
            b'\t' => escaped.extend_from_slice(b"\\t"),
            0x00..=0x1f | 0x7f => escaped.extend_from_slice(format!("\\u00{byte:02x}").as_bytes()),
            _ => escaped.push(byte),
        }
    }
    escaped
}

/// Split one command's stderr into its record lines (prefix stripped) and
/// every other line, requiring the whole stream to end its last line with a
/// line feed and no record to carry a carriage return.
fn split_records(label: &str, stderr: &[u8]) -> (Vec<String>, Vec<String>) {
    assert!(
        stderr.is_empty() || stderr.ends_with(b"\n"),
        "{label}: stderr must end its last line with a line feed; held {stderr:?}"
    );
    let text = String::from_utf8_lossy(stderr);
    let mut records = Vec::new();
    let mut other = Vec::new();
    for line in text.split('\n').filter(|line| !line.is_empty()) {
        if let Some(record) = line.strip_prefix(PREFIX) {
            assert!(
                !record.is_empty() && !record.contains('\r'),
                "{label}: a record is one complete escaped line; held {line:?}"
            );
            records.push(record.to_owned());
        } else {
            other.push(line.to_owned());
        }
    }
    (records, other)
}

/// Assert a command's stderr carries exactly `expected` records, in order.
fn assert_records(label: &str, stderr: &[u8], expected: &[&str]) {
    let (records, _) = split_records(label, stderr);
    let expected: Vec<String> = expected.iter().map(|line| (*line).to_owned()).collect();
    assert_eq!(
        records, expected,
        "{label}: records must match the pinned sequence"
    );
}

/// The fixture's whole tree as comparable file bytes.
fn tree(label: &str, root: &Path) -> BTreeMap<String, Vec<u8>> {
    support::capture(root, support::CaptureOptions::default())
        .unwrap_or_else(|error| panic!("{label}: capture {}: {error}", shown(root)))
        .files()
}

/// A fresh standalone fixture holding only `runbook` at
/// `.ratmac/ratmac.toml`; root resolution falls back to the invoking
/// checkout, so the first record of any traced invocation there is
/// `root_selection resolution="checkout-local"`.
fn fixture(label: &str, runbook: &str) -> TempTree {
    let owned = TempTree::new(&format!("t128-{label}")).expect("own a fixture tree");
    owned.write(".ratmac/ratmac.toml", runbook);
    owned
}

/// Run the harness Engine in `directory` with a scrubbed environment and the
/// toggle set to exactly `toggle` (None leaves the variable unset); the test
/// process's own environment is never touched.
fn rtm(label: &str, directory: &Path, toggle: Option<&OsStr>, args: &[&str]) -> Output {
    let mut command = support::command(ratmac_qa::engine_bin!(), directory);
    command.args(args);
    if let Some(value) = toggle {
        command.env(TRACE_VAR, value);
    }
    command
        .output()
        .unwrap_or_else(|error| panic!("{label}: invoke rtm {args:?}: {error}"))
}

/// Spawn `command` with piped standard streams and wait for it, bounded by
/// `seconds`; a child that outlives the bound is killed (a child this test
/// started) and reported. Returns the child's process id with its output.
fn run_bounded(label: &str, command: &mut Command, seconds: u64) -> (u32, Output) {
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("{label}: spawn rtm: {error}"));
    let id = child.id();
    let output = finish_bounded(label, &mut child, seconds);
    (id, output)
}

/// Wait for an already spawned child, bounded by `seconds`.
fn finish_bounded(label: &str, child: &mut std::process::Child, seconds: u64) -> Output {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(error) => panic!("{label}: wait for rtm: {error}"),
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{label}: rtm did not finish within {seconds}s");
        }
        thread::sleep(Duration::from_millis(25));
    }
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();
    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    if let Some(pipe) = stdout_pipe.as_mut() {
        pipe.read_to_end(&mut stdout_bytes)
            .unwrap_or_else(|error| panic!("{label}: read rtm stdout: {error}"));
    }
    if let Some(pipe) = stderr_pipe.as_mut() {
        pipe.read_to_end(&mut stderr_bytes)
            .unwrap_or_else(|error| panic!("{label}: read rtm stderr: {error}"));
    }
    let status = child
        .wait()
        .unwrap_or_else(|error| panic!("{label}: wait for rtm: {error}"));
    Output {
        status,
        stdout: stdout_bytes,
        stderr: stderr_bytes,
    }
}

/// Wait until `path` exists as a file, bounded by `seconds`.
fn wait_for_file(label: &str, path: &Path, seconds: u64) {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while !path.is_file() {
        if Instant::now() > deadline {
            panic!("{label}: {} never appeared within {seconds}s", shown(path));
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// Every regular file under `root`, as `/`-separated paths relative to it.
fn walk_files(label: &str, root: &Path) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let entries = fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("{label}: read {}: {error}", shown(&directory)));
        for entry in entries {
            let entry = entry.expect("a readable directory entry");
            let path = entry.path();
            let metadata = entry.metadata().expect("read directory entry metadata");
            if metadata.is_dir() {
                stack.push(path);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .expect("the walked path stays under its root");
                found.push((shown(relative), path));
            }
        }
    }
    found.sort();
    found
}

/// The 1-based line number of `offset` within `bytes`.
fn line_of(bytes: &[u8], offset: usize) -> usize {
    1 + bytes[..offset]
        .iter()
        .filter(|&&byte| byte == b'\n')
        .count()
}

// --- fixtures ----------------------------------------------------------------

/// The straight-line machine: one input-free edge `a -> b`.
const STRAIGHT_RUNBOOK: &str = r#"
[states.a]
prompt = "A."

[states.b]
prompt = "B."

[[transitions]]
from = "a"
to = "b"
"#;

/// Two failing artifact guards in declaration order, so a refused step fires
/// `files_exact` then `file_contains`, each with its own path field.
const GUARDED_RUNBOOK: &str = r#"
[states.guarded]
prompt = "Produce the guarded artifacts."
guards = [
  { kind = "files_exact", path = "artifacts", entries = ["marker.txt"] },
  { kind = "file_contains", path = "artifacts/result.yaml", contains = "result: ok" },
]

[states.complete]
prompt = "Complete."

[[transitions]]
from = "guarded"
to = "complete"
"#;

/// The input-routed machine: `review` declares inputs and both outgoing
/// edges carry one, so a passed step fires `edge_selection` with `input`.
const BRANCH_RUNBOOK: &str = r#"
[states.review]
prompt = "Review."
inputs = ["approve", "rework"]

[states.approved]
prompt = "Approved."

[states.rework]
prompt = "Rework."

[[transitions]]
from = "review"
to = "approved"
input = "approve"

[[transitions]]
from = "review"
to = "rework"
input = "rework"
"#;

/// The declared input value that carries every byte class the escaping rule
/// names: a double quote, a tab, a line feed, a carriage return, a
/// backslash, one byte below 0x20, and byte 0x7F.
const WEIRD_INPUT: &str = "q\"u\tb\nNL\rCR\\SL\u{1}\u{7f}";

/// The TOML spelling of that value inside the runbook and the verdict.
const WEIRD_TOML: &str = "q\\\"u\\tb\\nNL\\rCR\\\\SL\\u0001\\u007f";

/// The input-routed machine whose declared input exercises value escaping.
fn weird_runbook() -> String {
    format!(
        "[states.review]\n\
         prompt = \"Review.\"\n\
         inputs = [\"{WEIRD_TOML}\", \"plain\"]\n\
         \n\
         [states.approved]\n\
         prompt = \"Approved.\"\n\
         \n\
         [states.rework]\n\
         prompt = \"Rework.\"\n\
         \n\
         [[transitions]]\n\
         from = \"review\"\n\
         to = \"approved\"\n\
         input = \"{WEIRD_TOML}\"\n\
         \n\
         [[transitions]]\n\
         from = \"review\"\n\
         to = \"rework\"\n\
         input = \"plain\"\n"
    )
}

/// The live verdict that selects the escaping edge.
fn weird_verdict() -> String {
    format!("state = \"review\"\ninput = \"{WEIRD_TOML}\"\nrationale = \"why\"\n")
}

/// The gate-child diagnostic WEBV-027 plants: the refusal must carry it, no
/// record may.
const GATE_DIAGNOSTIC: &str = "webv027-gate-diagnostic-5c82";

/// The secret-bearing environment value WEBV-027 sets on traced children.
const SECRET_VAR: &str = "WEBV027_SECRET_TOKEN";
const SECRET_VALUE: &str = "webv027-secret-9d3f61";

/// The twin machine: one failing artifact guard and one failing gate child
/// that speaks `GATE_DIAGNOSTIC` on its standard error.
fn gate_runbook() -> String {
    let probe = env!("CARGO_BIN_EXE_guard-probe").replace('\\', "\\\\");
    format!(
        "[states.gated]\n\
         prompt = \"Pass the gate.\"\n\
         guards = [\n\
         {{ kind = \"files_exact\", path = \"artifacts\", entries = [\"marker.txt\"] }},\n\
         {{ kind = \"command_exit\", program = \"{probe}\", args = [\"say\", \
         \"{GATE_DIAGNOSTIC}\"], expected = 0 }},\n\
         ]\n\
         \n\
         [states.done]\n\
         prompt = \"Done.\"\n\
         \n\
         [[transitions]]\n\
         from = \"gated\"\n\
         to = \"done\"\n"
    )
}

/// One contention round: a holder child takes the root lock and parks on the
/// `RATMAC_TEST_LOCK_HOLD` seam, then an abandonment of `run-001` contends
/// for it. Returns the contender's process id and output; the holder always
/// runs to success once released.
fn contention_round(label: &str, project: &Path, traced: bool) -> (u32, Output) {
    let scratch = TempTree::new(&format!("t128-{label}-hold")).expect("own a hold tree");
    let marker = scratch.join("marker");
    let release = scratch.join("release");
    let mut holder = support::command(ratmac_qa::engine_bin!(), project);
    holder
        .args(["start"])
        .env("RATMAC_TEST_LOCK_HOLD", "root")
        .env("RATMAC_TEST_LOCK_MARKER", &marker)
        .env("RATMAC_TEST_LOCK_RELEASE", &release)
        .env("RATMAC_TEST_LOCK_HOLD_TIMEOUT_MILLIS", "30000")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    let mut holder_child = holder
        .spawn()
        .unwrap_or_else(|error| panic!("{label}: spawn the lock holder: {error}"));
    wait_for_file(label, &marker, 15);

    let mut contender = support::command(ratmac_qa::engine_bin!(), project);
    contender
        .args([
            "abandon",
            "--confirm",
            "abandon run-001",
            "--run",
            "run-001",
        ])
        .env(SECRET_VAR, SECRET_VALUE);
    if traced {
        contender.env(TRACE_VAR, "1");
    }
    let (contender_id, contender_output) = run_bounded(label, &mut contender, 30);

    fs::write(&release, "release\n")
        .unwrap_or_else(|error| panic!("{label}: write {}: {error}", shown(&release)));
    let holder_output = finish_bounded(label, &mut holder_child, 35);
    assert!(
        holder_output.status.success(),
        "{label}: the holder must finish once released: {}",
        String::from_utf8_lossy(&holder_output.stderr)
    );
    (contender_id, contender_output)
}

// --- WEBV-025 -----------------------------------------------------------------

/// WEBV-025: the disabled toggle states are today's byte-identical baseline,
/// and the no-work allocation proof exists in the owning module.
///
/// Contract sentences judged: the toggle table's disabled row ("Every
/// observable - stdout bytes, stderr bytes, exit status, files before/after,
/// operation-log bytes, every fault and pause seam - is byte-identical to
/// today's build on this repository"), the record-grammar sentence "Nothing
/// else is ever prefixed `rtm-trace: `", and the no-work proof paragraph:
/// "The unit test (in the owning module's `#[cfg(test)]`, part of the
/// workspace run WEBV-025 names) installs a counting global allocator and
/// asserts three budgets separately" - observed by reading `src/diag.rs`'s
/// bytes because the seam does not exist before implementation.
#[test]
fn webv_025() {
    // The disabled states are byte-identical to the unset baseline.
    let baseline = fixture("webv025-baseline", STRAIGHT_RUNBOOK);
    let zero = fixture("webv025-zero", STRAIGHT_RUNBOOK);
    let empty = fixture("webv025-empty", STRAIGHT_RUNBOOK);
    let before = tree("WEBV-025 baseline before", baseline.path());
    let runs = [
        rtm("WEBV-025 unset start", baseline.path(), None, &["start"]),
        rtm(
            "WEBV-025 zero start",
            zero.path(),
            Some(OsStr::new("0")),
            &["start"],
        ),
        rtm(
            "WEBV-025 empty start",
            empty.path(),
            Some(OsStr::new("")),
            &["start"],
        ),
    ];
    for (index, output) in runs.iter().enumerate() {
        let label = ["unset", "0", "empty"][index];
        assert!(
            output.status.success(),
            "WEBV-025: {label} start must succeed"
        );
        assert!(
            !contains_prefix(&output.stdout) && !contains_prefix(&output.stderr),
            "WEBV-025: {label} start must carry no record prefix anywhere"
        );
    }
    for (index, other) in runs.iter().enumerate().skip(1) {
        let label = ["unset", "0", "empty"][index];
        assert_eq!(
            runs[0].stdout, other.stdout,
            "WEBV-025: stdout with {label} must be byte-identical to the unset baseline"
        );
        assert_eq!(
            runs[0].stderr, other.stderr,
            "WEBV-025: stderr with {label} must be byte-identical to the unset baseline"
        );
        assert_eq!(
            runs[0].status.code(),
            other.status.code(),
            "WEBV-025: exit status with {label} must equal the unset baseline"
        );
    }
    let after = tree("WEBV-025 baseline after", baseline.path());
    for (index, sibling) in [zero.path(), empty.path()].iter().enumerate() {
        let label = ["0", "empty"][index];
        assert_eq!(
            after,
            tree("WEBV-025 sibling after", sibling),
            "WEBV-025: the filesystem with {label} must be byte-identical to the unset baseline"
        );
    }
    for (path, bytes) in &before {
        assert_eq!(
            after.get(path),
            Some(bytes),
            "WEBV-025: a disabled run must leave {path} byte-identical"
        );
    }

    // A refused step is also unchanged across the disabled states.
    let refusal_base = fixture("webv025-refusal-base", GUARDED_RUNBOOK);
    let refusal_zero = fixture("webv025-refusal-zero", GUARDED_RUNBOOK);
    for project in [refusal_base.path(), refusal_zero.path()] {
        let start = rtm("WEBV-025 refusal start", project, None, &["start"]);
        assert!(
            start.status.success(),
            "WEBV-025: refusal fixture start must succeed"
        );
    }
    let refused_unset = rtm(
        "WEBV-025 unset refused step",
        refusal_base.path(),
        None,
        &["step", "--run", "run-001"],
    );
    let refused_zero = rtm(
        "WEBV-025 zero refused step",
        refusal_zero.path(),
        Some(OsStr::new("0")),
        &["step", "--run", "run-001"],
    );
    assert_eq!(
        refused_unset.stdout, refused_zero.stdout,
        "WEBV-025: a refused step's stdout with 0 must be byte-identical to the unset baseline"
    );
    assert_eq!(
        refused_unset.stderr, refused_zero.stderr,
        "WEBV-025: a refused step's stderr with 0 must be byte-identical to the unset baseline"
    );
    assert_eq!(
        refused_unset.status.code(),
        refused_zero.status.code(),
        "WEBV-025: a refused step's exit status with 0 must equal the unset baseline"
    );
    assert_eq!(
        tree("WEBV-025 refusal base after", refusal_base.path()),
        tree("WEBV-025 refusal zero after", refusal_zero.path()),
        "WEBV-025: a refused step's filesystem with 0 must be byte-identical to the unset baseline"
    );

    // The no-work proof: the owning module's counting-allocator unit test.
    let diag = repo_root().join("src/diag.rs");
    let bytes = fs::read(&diag).unwrap_or_else(|error| {
        panic!(
            "WEBV-025 no-work proof: read {}: {error}; the owning module and its \
             allocation-counting unit test are the missing behavior this check names",
            shown(&diag)
        )
    });
    for marker in ["#[cfg(test)]", "GlobalAlloc", "RATMAC_TRACE"] {
        assert!(
            contains_bytes(&bytes, marker.as_bytes()),
            "WEBV-025 no-work proof: src/diag.rs must carry the counting-allocator unit \
             test for the disabled event, the absent-variable latch, and the present \
             read; missing {marker:?}"
        );
    }
}

// --- WEBV-026 -----------------------------------------------------------------

/// WEBV-026: with `1`, controlled fixtures exercise root resolution, edge
/// choice, refusal order, root-before-Run lock order, and mint decisions;
/// records have stable codes and ordering, appear only on stderr, and
/// ordinary stdout and state match the disabled execution.
///
/// Contract sentences judged: the enabled toggle row ("stderr additionally
/// carries the trace records defined below, in decision order ... stdout
/// bytes, exit codes, all files, and the operation log stay byte-identical to
/// the disabled run of the same command over the same fixture"); the event
/// vocabulary table's rows for `root_selection`, `edge_selection`, the
/// refusal codes, `lock_acquire`, `lock_release`, `mint_read`, and
/// `mint_write`; and the firing-points paragraph that pins the exact
/// `rtm start` sequence, the refused-step sequence, and the root-before-Run
/// order, including the amended resolution rule: `checkout-local` is for a
/// fixture outside any Git repository, and any Git repository - including a
/// single-checkout one whose primary is the invoking checkout itself - reads
/// `repository-primary`.
#[test]
fn webv_026() {
    // The pinned `rtm start` sequence on a fresh standalone fixture.
    let start_on = fixture("webv026-start-on", STRAIGHT_RUNBOOK);
    let start_off = fixture("webv026-start-off", STRAIGHT_RUNBOOK);
    let traced_start = rtm(
        "WEBV-026 traced start",
        start_on.path(),
        Some(OsStr::new("1")),
        &["start"],
    );
    let plain_start = rtm("WEBV-026 plain start", start_off.path(), None, &["start"]);
    assert!(
        traced_start.status.success() && plain_start.status.success(),
        "WEBV-026: both starts must succeed"
    );
    assert_records(
        "WEBV-026 traced start",
        &traced_start.stderr,
        &[
            "code=\"root_selection\" resolution=\"checkout-local\"",
            "code=\"lock_acquire\" domain=\"policy\"",
            "code=\"lock_release\" domain=\"policy\"",
            "code=\"lock_acquire\" domain=\"root\"",
            "code=\"mint_read\" recorded=\"absent\" roster=\"0\"",
            "code=\"mint_write\" reserved=\"run-001\"",
            "code=\"lock_release\" domain=\"root\"",
        ],
    );
    assert!(
        !contains_prefix(&traced_start.stdout),
        "WEBV-026: records appear only on stderr, never on stdout"
    );
    let (records, other) = split_records("WEBV-026 traced start", &traced_start.stderr);
    assert_eq!(
        records.len(),
        7,
        "WEBV-026: the pinned start sequence is closed"
    );
    assert!(
        other.is_empty(),
        "WEBV-026: tracing adds records to stderr and changes nothing else"
    );
    assert_eq!(
        traced_start.stdout, plain_start.stdout,
        "WEBV-026: a traced start's stdout must equal the disabled run's"
    );
    assert_eq!(
        tree("WEBV-026 start on after", start_on.path()),
        tree("WEBV-026 start off after", start_off.path()),
        "WEBV-026: a traced start's files must equal the disabled run's"
    );

    // A Git-initialised standalone repository - one whose primary is the
    // invoking checkout itself - reads `repository-primary`, because Git's
    // worktree listing supplied the shared Engine root.
    let repository = TempRepo::new("t128-webv026-git-primary");
    repository.write(".ratmac/ratmac.toml", STRAIGHT_RUNBOOK);
    repository.stage(".ratmac/ratmac.toml");
    repository.commit_all("WEBV-026 fixture base");
    let inside = support::git(repository.root())
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .unwrap_or_else(|error| panic!("WEBV-026: probe the fixture's Git identity: {error}"));
    assert!(
        inside.status.success() && String::from_utf8_lossy(&inside.stdout).trim() == "true",
        "WEBV-026 control: the repository-primary fixture must be a Git repository"
    );
    let repository_start = rtm(
        "WEBV-026 traced repository start",
        repository.root(),
        Some(OsStr::new("1")),
        &["start"],
    );
    assert!(
        repository_start.status.success(),
        "WEBV-026: the repository start must succeed"
    );
    let (repository_records, _) =
        split_records("WEBV-026 traced repository start", &repository_start.stderr);
    assert_eq!(
        repository_records.first().map(String::as_str),
        Some("code=\"root_selection\" resolution=\"repository-primary\""),
        "WEBV-026: a Git repository's first record names the worktree-listing \
         resolution, so the standalone Git fixture reads repository-primary"
    );

    // A refused step: one refusal-code record per failure, in failure order.
    let refused_on = fixture("webv026-refused-on", GUARDED_RUNBOOK);
    let refused_off = fixture("webv026-refused-off", GUARDED_RUNBOOK);
    for project in [refused_on.path(), refused_off.path()] {
        let start = rtm("WEBV-026 refused fixture start", project, None, &["start"]);
        assert!(
            start.status.success(),
            "WEBV-026: refused fixture start must succeed"
        );
    }
    let traced_step = rtm(
        "WEBV-026 traced refused step",
        refused_on.path(),
        Some(OsStr::new("1")),
        &["step", "--run", "run-001"],
    );
    let plain_step = rtm(
        "WEBV-026 plain refused step",
        refused_off.path(),
        None,
        &["step", "--run", "run-001"],
    );
    assert_records(
        "WEBV-026 traced refused step",
        &traced_step.stderr,
        &[
            "code=\"root_selection\" resolution=\"checkout-local\"",
            "code=\"lock_acquire\" domain=\"run\" run=\"run-001\"",
            "code=\"files_exact\" run=\"run-001\" path=\"artifacts\"",
            "code=\"file_contains\" run=\"run-001\" path=\"artifacts/result.yaml\"",
            "code=\"lock_release\" domain=\"run\" run=\"run-001\"",
        ],
    );
    assert!(
        !contains_prefix(&traced_step.stdout),
        "WEBV-026: records appear only on stderr, never on stdout"
    );
    assert_eq!(
        traced_step.stdout, plain_step.stdout,
        "WEBV-026: a refused step's refusal render stays exactly today's"
    );
    assert_eq!(
        traced_step.status.code(),
        plain_step.status.code(),
        "WEBV-026: a refused step is stdout plus an unchanged exit code"
    );
    assert_eq!(
        tree("WEBV-026 refused on after", refused_on.path()),
        tree("WEBV-026 refused off after", refused_off.path()),
        "WEBV-026: a traced refused step's files must equal the disabled run's"
    );

    // A passed input-routed step fires `edge_selection` with `input`.
    let branch_on = fixture("webv026-branch-on", BRANCH_RUNBOOK);
    let branch_off = fixture("webv026-branch-off", BRANCH_RUNBOOK);
    for project in [branch_on.path(), branch_off.path()] {
        let start = rtm("WEBV-026 branch fixture start", project, None, &["start"]);
        assert!(
            start.status.success(),
            "WEBV-026: branch fixture start must succeed"
        );
        fs::write(
            project.join(".ratmac/runs/run-001/verdict.toml"),
            "state = \"review\"\ninput = \"approve\"\nrationale = \"why\"\n",
        )
        .expect("WEBV-026: publish the branch verdict");
    }
    let traced_branch = rtm(
        "WEBV-026 traced branch step",
        branch_on.path(),
        Some(OsStr::new("1")),
        &["step", "--run", "run-001"],
    );
    let plain_branch = rtm(
        "WEBV-026 plain branch step",
        branch_off.path(),
        None,
        &["step", "--run", "run-001"],
    );
    assert_records(
        "WEBV-026 traced branch step",
        &traced_branch.stderr,
        &[
            "code=\"root_selection\" resolution=\"checkout-local\"",
            "code=\"lock_acquire\" domain=\"run\" run=\"run-001\"",
            "code=\"edge_selection\" run=\"run-001\" from=\"review\" input=\"approve\" to=\"approved\"",
            "code=\"lock_release\" domain=\"run\" run=\"run-001\"",
        ],
    );
    assert_eq!(
        traced_branch.stdout, plain_branch.stdout,
        "WEBV-026: a passed step's stdout must equal the disabled run's"
    );
    assert_eq!(
        tree("WEBV-026 branch on after", branch_on.path()),
        tree("WEBV-026 branch off after", branch_off.path()),
        "WEBV-026: a passed step's files must equal the disabled run's"
    );

    // A passed straight-line step omits `input` entirely.
    let traced_straight = rtm(
        "WEBV-026 traced straight step",
        start_on.path(),
        Some(OsStr::new("1")),
        &["step", "--run", "run-001"],
    );
    assert!(
        traced_straight.status.success(),
        "WEBV-026: the straight step must succeed"
    );
    assert_records(
        "WEBV-026 traced straight step",
        &traced_straight.stderr,
        &[
            "code=\"root_selection\" resolution=\"checkout-local\"",
            "code=\"lock_acquire\" domain=\"run\" run=\"run-001\"",
            "code=\"edge_selection\" run=\"run-001\" from=\"a\" to=\"b\"",
            "code=\"lock_release\" domain=\"run\" run=\"run-001\"",
        ],
    );

    // Root-before-Run lock order, observed as bytes on an abandonment.
    let traced_abandon = rtm(
        "WEBV-026 traced abandon",
        refused_on.path(),
        Some(OsStr::new("1")),
        &[
            "abandon",
            "--confirm",
            "abandon run-001",
            "--run",
            "run-001",
        ],
    );
    let plain_abandon = rtm(
        "WEBV-026 plain abandon",
        refused_off.path(),
        None,
        &[
            "abandon",
            "--confirm",
            "abandon run-001",
            "--run",
            "run-001",
        ],
    );
    assert!(
        traced_abandon.status.success() && plain_abandon.status.success(),
        "WEBV-026: both abandonments must succeed"
    );
    let (abandon_records, _) = split_records("WEBV-026 traced abandon", &traced_abandon.stderr);
    let root_at = abandon_records
        .iter()
        .position(|record| record == "code=\"lock_acquire\" domain=\"root\"");
    let run_at = abandon_records
        .iter()
        .position(|record| record == "code=\"lock_acquire\" domain=\"run\" run=\"run-001\"");
    assert!(
        root_at.is_some() && run_at.is_some(),
        "WEBV-026: the abandonment must acquire both lock domains: {abandon_records:?}"
    );
    assert!(
        root_at.unwrap() < run_at.unwrap(),
        "WEBV-026: the root acquisition must precede the Run acquisition of the same \
         decision: {abandon_records:?}"
    );
    for domain in ["root", "run", "policy"] {
        let acquired = abandon_records
            .iter()
            .filter(|record| **record == format!("code=\"lock_acquire\" domain=\"{domain}\""))
            .count();
        let released = abandon_records
            .iter()
            .filter(|record| **record == format!("code=\"lock_release\" domain=\"{domain}\""))
            .count();
        assert_eq!(
            acquired, released,
            "WEBV-026: every acquired {domain} lock is released exactly once"
        );
    }
    assert_eq!(
        traced_abandon.stdout, plain_abandon.stdout,
        "WEBV-026: an abandonment's stdout must equal the disabled run's"
    );
    assert_eq!(
        tree("WEBV-026 abandon on after", refused_on.path()),
        tree("WEBV-026 abandon off after", refused_off.path()),
        "WEBV-026: an abandonment's files must equal the disabled run's"
    );

    // Mint decisions on the second start: the record held 1, the roster held
    // 1, and the mint reserved run-002.
    let second_start = rtm(
        "WEBV-026 traced second start",
        start_on.path(),
        Some(OsStr::new("1")),
        &["start"],
    );
    assert!(
        second_start.status.success(),
        "WEBV-026: the second start must succeed"
    );
    let (second_records, _) = split_records("WEBV-026 traced second start", &second_start.stderr);
    let mint_reads: Vec<&String> = second_records
        .iter()
        .filter(|record| record.starts_with("code=\"mint_read\""))
        .collect();
    let mint_writes: Vec<&String> = second_records
        .iter()
        .filter(|record| record.starts_with("code=\"mint_write\""))
        .collect();
    assert_eq!(
        mint_reads,
        vec![&"code=\"mint_read\" recorded=\"1\" roster=\"1\"".to_owned()],
        "WEBV-026: the second start's mint read must report the held and roster ordinals"
    );
    assert_eq!(
        mint_writes,
        vec![&"code=\"mint_write\" reserved=\"run-002\"".to_owned()],
        "WEBV-026: the second start's mint write must reserve the next canonical id"
    );
}

// --- WEBV-027 -----------------------------------------------------------------

/// WEBV-027: determinism and privacy. Byte-equivalent isolated fixtures at
/// different absolute paths and clocks produce byte-identical record
/// sequences; quoted values escape exactly; no timestamp, duration, absolute
/// host path, process id, secret-bearing environment value, or arbitrary
/// child diagnostic appears in a record; and mandatory errors keep their
/// established actionable text and paths, asserted separately.
///
/// Contract sentences judged: "Two invocations of the same command over
/// byte-equivalent isolated fixtures - different absolute locations,
/// different clocks - with tracing enabled produce byte-identical record
/// sequences"; "No record ever contains a timestamp, a duration, an absolute
/// host path, a process id, an environment variable's value, a lock owner
/// token or nonce, or free-form child output"; the record-grammar escaping
/// sentence ("a State name, input value, or guard path containing quotes,
/// spaces, or newlines round-trips byte-exactly"); and "Mandatory errors keep
/// their established actionable text and paths byte-for-byte - for example
/// `lock wait expired: <path>; <owner-token diagnostic>` still names the
/// absolute lock path".
#[test]
fn webv_027() {
    let runbook = gate_runbook();

    // First fixture: start, then a refused step, both traced, with a secret
    // environment value riding along.
    let first = fixture("webv027-first", &runbook);
    let (first_start_id, first_start) = {
        let mut command = support::command(ratmac_qa::engine_bin!(), first.path());
        command
            .args(["start"])
            .env(TRACE_VAR, "1")
            .env(SECRET_VAR, SECRET_VALUE);
        run_bounded("WEBV-027 first traced start", &mut command, 30)
    };
    assert!(
        first_start.status.success(),
        "WEBV-027: the first start must succeed"
    );
    let (first_start_records, _) =
        split_records("WEBV-027 first traced start", &first_start.stderr);
    assert!(
        !first_start_records.is_empty(),
        "WEBV-027: start with tracing enabled must emit trace records; stderr held {:?}",
        String::from_utf8_lossy(&first_start.stderr)
    );
    let (first_step_id, first_step) = {
        let mut command = support::command(ratmac_qa::engine_bin!(), first.path());
        command
            .args(["step", "--run", "run-001"])
            .env(TRACE_VAR, "1")
            .env(SECRET_VAR, SECRET_VALUE);
        run_bounded("WEBV-027 first traced step", &mut command, 30)
    };
    let (first_step_records, _) = split_records("WEBV-027 first traced step", &first_step.stderr);
    assert!(
        !first_step_records.is_empty(),
        "WEBV-027: a refused step with tracing enabled must emit trace records; stderr held {:?}",
        String::from_utf8_lossy(&first_step.stderr)
    );
    let first_text = String::from_utf8_lossy(&first_step.stdout).into_owned();
    assert!(
        first_text.contains("step refused") && first_text.contains(GATE_DIAGNOSTIC),
        "WEBV-027 control: the refusal must carry the gate child's diagnostic: {first_text}"
    );

    // Second fixture at another absolute location, past another clock tick.
    thread::sleep(Duration::from_millis(1100));
    let second = fixture("webv027-second", &runbook);
    let (second_start_id, second_start) = {
        let mut command = support::command(ratmac_qa::engine_bin!(), second.path());
        command
            .args(["start"])
            .env(TRACE_VAR, "1")
            .env(SECRET_VAR, SECRET_VALUE);
        run_bounded("WEBV-027 second traced start", &mut command, 30)
    };
    assert!(
        second_start.status.success(),
        "WEBV-027: the second start must succeed"
    );
    let (second_start_records, _) =
        split_records("WEBV-027 second traced start", &second_start.stderr);
    let (second_step_id, second_step) = {
        let mut command = support::command(ratmac_qa::engine_bin!(), second.path());
        command
            .args(["step", "--run", "run-001"])
            .env(TRACE_VAR, "1")
            .env(SECRET_VAR, SECRET_VALUE);
        run_bounded("WEBV-027 second traced step", &mut command, 30)
    };
    let (second_step_records, _) =
        split_records("WEBV-027 second traced step", &second_step.stderr);
    assert_eq!(
        first_start_records, second_start_records,
        "WEBV-027: byte-equivalent fixtures must produce byte-identical start records"
    );
    assert_eq!(
        first_step_records, second_step_records,
        "WEBV-027: byte-equivalent fixtures must produce byte-identical step records"
    );

    // Escaping: the input value round-trips byte-exactly through the one
    // escaping rule.
    let escape = fixture("webv027-escape", &weird_runbook());
    let escape_start = rtm(
        "WEBV-027 escape fixture start",
        escape.path(),
        Some(OsStr::new("1")),
        &["start"],
    );
    assert!(
        escape_start.status.success(),
        "WEBV-027: the escape fixture start must succeed"
    );
    fs::write(
        escape.path().join(".ratmac/runs/run-001/verdict.toml"),
        weird_verdict(),
    )
    .expect("WEBV-027: publish the escaping verdict");
    let escape_step = rtm(
        "WEBV-027 traced escape step",
        escape.path(),
        Some(OsStr::new("1")),
        &["step", "--run", "run-001"],
    );
    assert!(
        escape_step.status.success(),
        "WEBV-027: the escape step must succeed"
    );
    let escaped_input = String::from_utf8(escape_value(WEIRD_INPUT.as_bytes()))
        .expect("the contract escaping of this ASCII value is ASCII");
    let expected_edge = format!(
        "code=\"edge_selection\" run=\"run-001\" from=\"review\" input=\"{escaped_input}\" \
         to=\"approved\""
    );
    assert_records(
        "WEBV-027 traced escape step",
        &escape_step.stderr,
        &[
            "code=\"root_selection\" resolution=\"checkout-local\"",
            "code=\"lock_acquire\" domain=\"run\" run=\"run-001\"",
            &expected_edge,
            "code=\"lock_release\" domain=\"run\" run=\"run-001\"",
        ],
    );

    // Privacy: no record names a host path, a process id, the secret, the
    // owner token, a nonce, or the gate child's output, and none carries a
    // long digit run a timestamp or duration would need.
    let mut records = Vec::new();
    records.extend(first_start_records.iter().cloned());
    records.extend(first_step_records.iter().cloned());
    records.extend(second_start_records.iter().cloned());
    records.extend(second_step_records.iter().cloned());
    records.extend(
        split_records("WEBV-027 traced escape step", &escape_step.stderr)
            .0
            .iter()
            .cloned(),
    );
    let mut forbidden: Vec<String> = vec![SECRET_VALUE.to_owned(), GATE_DIAGNOSTIC.to_owned()];
    for project in [first.path(), second.path(), escape.path()] {
        forbidden.push(shown(project));
        forbidden.push(project.to_string_lossy().into_owned());
        if let Ok(canonical) = fs::canonicalize(project) {
            forbidden.push(shown(&canonical));
            forbidden.push(canonical.to_string_lossy().into_owned());
        }
    }
    for id in [
        first_start_id,
        first_step_id,
        second_start_id,
        second_step_id,
    ] {
        forbidden.push(id.to_string());
    }
    forbidden.retain(|value| !value.is_empty());
    for record in &records {
        for value in &forbidden {
            assert!(
                !record.contains(value.as_str()),
                "WEBV-027: no record may contain {value:?}: {record:?}"
            );
        }
        assert!(
            !contains_bytes(record.as_bytes(), b"ratmac-lock-v1"),
            "WEBV-027: no record may carry a lock owner token: {record:?}"
        );
        assert!(
            longest_digit_run(record.as_bytes()) < 9,
            "WEBV-027: no record may carry a timestamp- or duration-length digit run: {record:?}"
        );
    }

    // Mandatory errors keep their established actionable text and paths,
    // separately from the record checks: a traced abandonment that contends
    // with a held root lock still refuses naming the absolute lock path.
    let (traced_id, traced_refusal) = contention_round("webv027-traced", first.path(), true);
    let (plain_id, plain_refusal) = contention_round("webv027-plain", second.path(), false);
    for (label, project, id, refusal) in [
        ("traced", first.path(), traced_id, &traced_refusal),
        ("plain", second.path(), plain_id, &plain_refusal),
    ] {
        assert!(
            !refusal.status.success() && refusal.stdout.is_empty(),
            "WEBV-027: the {label} contender must refuse with empty stdout: {}",
            String::from_utf8_lossy(&refusal.stderr)
        );
        let (records, other) = split_records(label, &refusal.stderr);
        let line = other.first().unwrap_or_else(|| {
            panic!("WEBV-027: the {label} contender must state its refusal on stderr")
        });
        let engine_root = shown(&project.join(".ratmac"));
        let prefix = format!(
            "rtm: abandon refused; lock wait expired: {engine_root}/locks/root.lock; \
             owner token: ratmac-lock-v1; pid="
        );
        assert!(
            line.starts_with(&prefix),
            "WEBV-027: the {label} refusal keeps today's text and absolute lock path: {line}"
        );
        let diagnostic = line
            .strip_prefix(&prefix)
            .unwrap_or_default()
            .trim_start_matches(|digit: char| digit.is_ascii_digit());
        assert!(
            diagnostic.starts_with("; guard=root; nonce="),
            "WEBV-027: the {label} refusal keeps today's owner-token diagnostic after the \
             owner's pid: {line}"
        );
        assert_eq!(
            other.len(),
            1,
            "WEBV-027: the {label} refusal is one line beside any records"
        );
        for record in &records {
            for value in [&engine_root, &id.to_string()] {
                assert!(
                    !record.contains(value.as_str()),
                    "WEBV-027: no record of the {label} contender may name {value:?}: {record:?}"
                );
            }
        }
    }
    assert_eq!(
        traced_refusal.status.code(),
        plain_refusal.status.code(),
        "WEBV-027: tracing must not change the contention refusal's exit code"
    );
}

/// The longest run of ASCII decimal digits in `bytes`.
fn longest_digit_run(bytes: &[u8]) -> usize {
    let mut longest = 0;
    let mut current = 0;
    for &byte in bytes {
        if byte.is_ascii_digit() {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    longest
}

// --- WEBV-028 -----------------------------------------------------------------

/// The invariant part of the invalid-toggle refusal line.
const INVALID_TOGGLE_TEXT: &str =
    " is not a valid trace setting; set it to 1 to enable the decision trace \
     or 0 to disable it; nothing was run";

/// The expected stderr bytes of an invalid-toggle refusal for `value`.
fn invalid_toggle_stderr(value: &[u8]) -> Vec<u8> {
    let mut expected = b"rtm: RATMAC_TRACE=\"".to_vec();
    expected.extend_from_slice(&escape_value(value));
    expected.extend_from_slice(b"\"");
    expected.extend_from_slice(INVALID_TOGGLE_TEXT.as_bytes());
    expected.push(b'\n');
    expected
}

/// WEBV-028: one owner, handled refusals, tolerated stderr failure, working
/// fault seams, and no proof dependency on the channel.
///
/// Contract sentences judged: "The exhaustive source check walks every file
/// under `src/` ... and asserts that none except `src/diag.rs` contains any
/// of the byte sequences `eprintln!`, `eprint!`, or `stderr(` - with two
/// standing exceptions" (observed by reading source bytes, the only way a
/// suite that must compile without `ratmac::diag` can see it); the
/// invalid-toggle row ("Refusal before any work: exit 2, stdout empty,
/// stderr exactly one LF-terminated line ... No `next:` line ... it writes
/// nothing"); "A failed or closed standard error never changes a decision";
/// "any of them may be set together with `RATMAC_TRACE=1`; the combination
/// adds records to stderr and changes nothing else"; and "Every child
/// process the Engine spawns ... runs with `RATMAC_TRACE` removed from its
/// environment and every other inherited variable unchanged" plus "none
/// parses `rtm-trace: ` text".
#[test]
fn webv_028() {
    // The exhaustive source inventory: only the owning module touches the
    // standard error stream, with the two standing child-pipe exceptions.
    let src = repo_root().join("src");
    // Each of the two pinned files may carry this byte sequence exactly
    // once; a second occurrence in either file fails the check.
    let pinned_pipe = b".stderr(std::process::Stdio::piped())";
    let mut violations = Vec::new();
    for (relative, path) in walk_files("WEBV-028 src walk", &src) {
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("WEBV-028: read {}: {error}", shown(&path)));
        if relative == "diag.rs" {
            continue;
        }
        for marker in ["eprintln!", "eprint!"] {
            let mut offset = 0;
            while let Some(found) = bytes[offset..]
                .windows(marker.len())
                .position(|window| window == marker.as_bytes())
            {
                let at = offset + found;
                violations.push(format!("{relative}:{}: {marker}", line_of(&bytes, at)));
                offset = at + 1;
            }
        }
        let exception_file = relative == "scheduler.rs" || relative == "review.rs";
        let mut exception_spent = false;
        let mut offset = 0;
        while let Some(found) = bytes[offset..]
            .windows(b"stderr(".len())
            .position(|window| window == b"stderr(")
        {
            let at = offset + found;
            let pinned_here = at > 0 && bytes[at - 1..].starts_with(pinned_pipe);
            if exception_file && pinned_here && !exception_spent {
                exception_spent = true;
            } else if exception_file && pinned_here {
                violations.push(format!(
                    "{relative}:{}: a second pinned child-pipe configuration; \
                     one is the standing exception",
                    line_of(&bytes, at)
                ));
            } else {
                violations.push(format!("{relative}:{}: stderr(", line_of(&bytes, at)));
            }
            offset = at + 1;
        }
    }
    assert!(
        violations.is_empty(),
        "WEBV-028: only src/diag.rs may touch the standard error stream; today's sites \
         must route through the owning module: {violations:?}"
    );

    // Nothing under `src/` outside the owning module consumes record text.
    let mut parsers = Vec::new();
    for (relative, path) in walk_files("WEBV-028 src walk", &src) {
        if relative == "diag.rs" {
            continue;
        }
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("WEBV-028: read {}: {error}", shown(&path)));
        if contains_bytes(&bytes, b"rtm-trace") {
            parsers.push(relative);
        }
    }
    assert!(
        parsers.is_empty(),
        "WEBV-028: no guard, gate, or receipt may parse record text: {parsers:?}"
    );

    // Invalid toggle values refuse before any work: exit 2, empty stdout,
    // exactly one refusal line, no `next:` line, and nothing written.
    for value in ["true", "01", " 1", "2"] {
        let scratch = fixture("webv028-invalid-help", STRAIGHT_RUNBOOK);
        let refused = rtm(
            "WEBV-028 invalid toggle help",
            scratch.path(),
            Some(OsStr::new(value)),
            &["--help"],
        );
        assert_eq!(
            refused.status.code(),
            Some(2),
            "WEBV-028: RATMAC_TRACE={value:?} must refuse with exit 2 before help rendering"
        );
        assert!(
            refused.stdout.is_empty(),
            "WEBV-028: the toggle refusal writes nothing to stdout for {value:?}"
        );
        assert_eq!(
            refused.stderr,
            invalid_toggle_stderr(value.as_bytes()),
            "WEBV-028: the toggle refusal for {value:?} is exactly one escaped line"
        );
    }
    #[cfg(windows)]
    let raw_value = OsString::from_wide(&['x' as u16, 0xD800, '1' as u16]);
    #[cfg(unix)]
    let raw_value = OsString::from_vec(vec![b'x', 0xFF, b'1']);
    {
        let scratch = fixture("webv028-invalid-raw", STRAIGHT_RUNBOOK);
        let refused = rtm(
            "WEBV-028 invalid raw toggle help",
            scratch.path(),
            Some(&raw_value),
            &["--help"],
        );
        assert_eq!(
            refused.status.code(),
            Some(2),
            "WEBV-028: a non-UTF-8 toggle must refuse with exit 2"
        );
        assert!(
            refused.stdout.is_empty(),
            "WEBV-028: the raw toggle refusal writes nothing to stdout"
        );
        assert_eq!(
            refused.stderr,
            invalid_toggle_stderr(raw_value.as_encoded_bytes()),
            "WEBV-028: the raw toggle refusal renders every byte verbatim or as \\u00XX"
        );
    }
    {
        let scratch = fixture("webv028-invalid-start", STRAIGHT_RUNBOOK);
        let before = tree("WEBV-028 invalid start before", scratch.path());
        let refused = rtm(
            "WEBV-028 invalid toggle start",
            scratch.path(),
            Some(OsStr::new("true")),
            &["start"],
        );
        assert_eq!(
            refused.status.code(),
            Some(2),
            "WEBV-028: an invalid toggle refuses before any work"
        );
        assert!(refused.stdout.is_empty(), "WEBV-028: nothing runs");
        assert_eq!(
            refused.stderr,
            invalid_toggle_stderr(b"true"),
            "WEBV-028: the refusal is exactly one line with no next: line"
        );
        assert_eq!(
            tree("WEBV-028 invalid start after", scratch.path()),
            before,
            "WEBV-028: the toggle refusal precedes every lock and every write"
        );
    }

    // A failed standard error never changes a decision: a traced start whose
    // stderr pipe loses its reader still succeeds with the full report.
    let control = fixture("webv028-stderr-control", STRAIGHT_RUNBOOK);
    let control_start = rtm(
        "WEBV-028 control start",
        control.path(),
        Some(OsStr::new("1")),
        &["start"],
    );
    assert!(
        control_start.status.success(),
        "WEBV-028: the control start must succeed"
    );
    let broken = fixture("webv028-stderr-broken", STRAIGHT_RUNBOOK);
    let broken_start = {
        let mut command = support::command(ratmac_qa::engine_bin!(), broken.path());
        command
            .args(["start"])
            .env(TRACE_VAR, "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
        let mut child = command
            .spawn()
            .unwrap_or_else(|error| panic!("WEBV-028: spawn the broken-stderr start: {error}"));
        drop(child.stderr.take());
        finish_bounded("WEBV-028 broken-stderr start", &mut child, 30)
    };
    assert_eq!(
        broken_start.status.code(),
        Some(0),
        "WEBV-028: a failed standard error must not change the start's exit code"
    );
    assert_eq!(
        broken_start.stdout, control_start.stdout,
        "WEBV-028: a failed standard error must not change stdout by one byte"
    );
    assert_eq!(
        tree("WEBV-028 broken after", broken.path()),
        tree("WEBV-028 control after", control.path()),
        "WEBV-028: a failed standard error must not change any file byte"
    );

    // The operation-log seam works beside tracing with identical log bytes.
    let logs = TempTree::new("t128-webv028-oplog").expect("own an operation-log tree");
    let plain = fixture("webv028-oplog-plain", STRAIGHT_RUNBOOK);
    let traced = fixture("webv028-oplog-traced", STRAIGHT_RUNBOOK);
    let plain_log = logs.join("plain.log");
    let traced_log = logs.join("traced.log");
    let plain_start = {
        let mut command = support::command(ratmac_qa::engine_bin!(), plain.path());
        command.args(["start"]).env(OP_LOG_VAR, &plain_log);
        command
            .output()
            .unwrap_or_else(|error| panic!("WEBV-028: invoke the plain logged start: {error}"))
    };
    let traced_start = {
        let mut command = support::command(ratmac_qa::engine_bin!(), traced.path());
        command
            .args(["start"])
            .env(OP_LOG_VAR, &traced_log)
            .env(TRACE_VAR, "1");
        command
            .output()
            .unwrap_or_else(|error| panic!("WEBV-028: invoke the traced logged start: {error}"))
    };
    assert!(
        plain_start.status.success() && traced_start.status.success(),
        "WEBV-028: both logged starts must succeed"
    );
    let plain_log_bytes = fs::read(&plain_log).expect("WEBV-028: the plain log exists");
    let traced_log_bytes = fs::read(&traced_log).expect("WEBV-028: the traced log exists");
    assert!(
        !plain_log_bytes.is_empty(),
        "WEBV-028: the operation-log seam must record the start's operations"
    );
    assert_eq!(
        plain_log_bytes, traced_log_bytes,
        "WEBV-028: tracing beside the seam changes nothing but stderr records"
    );
    assert_eq!(
        plain_start.stdout, traced_start.stdout,
        "WEBV-028: stdout is unchanged"
    );
    let (traced_records, _) = split_records("WEBV-028 traced logged start", &traced_start.stderr);
    assert!(
        !traced_records.is_empty(),
        "WEBV-028: the seam and tracing combine; records still reach stderr"
    );
    assert!(
        !contains_prefix(&plain_start.stderr),
        "WEBV-028: the seam alone emits no records"
    );

    // A gate child neither inherits the toggle nor loses any other inherited
    // variable: the guard passes exactly when the child's own environment
    // holds the marker and not the toggle.
    let absent = fixture("webv028-gate-absent", &gate_env_runbook(true));
    let present = fixture("webv028-gate-present", &gate_env_runbook(false));
    for project in [absent.path(), present.path()] {
        let start = {
            let mut command = support::command(ratmac_qa::engine_bin!(), project);
            command
                .args(["start"])
                .env(TRACE_VAR, "1")
                .env("WEBV028_INHERIT", "marker");
            command
                .output()
                .unwrap_or_else(|error| panic!("WEBV-028: invoke the gate fixture start: {error}"))
        };
        assert!(
            start.status.success(),
            "WEBV-028: the gate fixture start must succeed"
        );
    }
    let absent_step = {
        let mut command = support::command(ratmac_qa::engine_bin!(), absent.path());
        command
            .args(["step", "--run", "run-001"])
            .env(TRACE_VAR, "1")
            .env("WEBV028_INHERIT", "marker");
        command
            .output()
            .unwrap_or_else(|error| panic!("WEBV-028: invoke the absent-assert step: {error}"))
    };
    assert!(
        absent_step.status.success()
            && !String::from_utf8_lossy(&absent_step.stdout).contains("step refused"),
        "WEBV-028: the gate child must run with RATMAC_TRACE removed and every other \
         inherited variable unchanged: {}{}",
        String::from_utf8_lossy(&absent_step.stdout),
        String::from_utf8_lossy(&absent_step.stderr)
    );
    let (absent_records, _) = split_records("WEBV-028 absent-assert step", &absent_step.stderr);
    assert!(
        absent_records
            .iter()
            .any(|record| record.starts_with("code=\"edge_selection\"")),
        "WEBV-028: the passing gate still fires its edge record: {absent_records:?}"
    );
    let present_step = {
        let mut command = support::command(ratmac_qa::engine_bin!(), present.path());
        command
            .args(["step", "--run", "run-001"])
            .env(TRACE_VAR, "1")
            .env("WEBV028_INHERIT", "marker");
        command
            .output()
            .unwrap_or_else(|error| panic!("WEBV-028: invoke the present-assert step: {error}"))
    };
    assert!(
        String::from_utf8_lossy(&present_step.stdout).contains("step refused"),
        "WEBV-028 control: the guard discriminates the toggle, so with tracing \
         enabled it must refuse: {}",
        String::from_utf8_lossy(&present_step.stdout)
    );
}

/// The gate-child environment probe runbook: a `command_exit` guard whose
/// program is the platform shell, exiting 0 exactly when the child's own
/// environment holds the marker variable and - for `absent` - does not hold
/// the toggle.
fn gate_env_runbook(absent: bool) -> String {
    #[cfg(windows)]
    let (program, first, second) = if absent {
        (
            "cmd",
            "/c",
            "if defined RATMAC_TRACE (exit 3) else (if \"%WEBV028_INHERIT%\"==\"marker\" \
             (exit 0) else (exit 4))",
        )
    } else {
        (
            "cmd",
            "/c",
            "if defined RATMAC_TRACE (if \"%WEBV028_INHERIT%\"==\"marker\" (exit 0) else \
             (exit 4)) else (exit 3)",
        )
    };
    #[cfg(unix)]
    let (program, first, second) = if absent {
        (
            "sh",
            "-c",
            "test -z \"$RATMAC_TRACE\" && test \"$WEBV028_INHERIT\" = marker",
        )
    } else {
        (
            "sh",
            "-c",
            "test -n \"$RATMAC_TRACE\" && test \"$WEBV028_INHERIT\" = marker",
        )
    };
    let toml_second = second.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "[states.gated]\n\
         prompt = \"Pass the gate.\"\n\
         guards = [{{ kind = \"command_exit\", program = \"{program}\", args = [\"{first}\", \
         \"{toml_second}\"], expected = 0 }}]\n\
         \n\
         [states.done]\n\
         prompt = \"Done.\"\n\
         \n\
         [[transitions]]\n\
         from = \"gated\"\n\
         to = \"done\"\n"
    )
}
