//! t-137 / SVE-001: a test Engine is the executable Cargo reported.
//!
//! A helper that builds one of this repository's binaries and then launches
//! it must run exactly the `executable` of that binary's `compiler-artifact`
//! record under `cargo build --message-format=json`, never a path joined from
//! a fixed folder such as `target/debug`. These checks ask the shared
//! function `ratmac_qa::baseline::built_executable` - and the freeze Engine
//! build - for a binary while Cargo's output has moved, and compare each
//! answer with what a separate `--message-format=json` build of the same
//! root, arguments, and environment reports.
//!
//! Redirects reach child processes only: this process's environment never
//! changes, every redirect names its own fresh folder, and stand-ins are
//! planted only inside scratch copies of the repository.

use ratmac_qa::baseline::{self, built_executable, FREEZE_COMMIT};
use ratmac_qa::json::Json;
use ratmac_qa::support::{self, TempTree};
use std::any::Any;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// The caller's `cargo build` arguments for the test Engine.
const RTM_QA: [&str; 5] = ["--offline", "-p", "ratmac-qa", "--bin", "rtm-qa"];

/// The build command a refusal for those arguments names.
const RTM_QA_BUILD: &str = "cargo build --message-format=json --offline -p ratmac-qa --bin rtm-qa";

/// The freeze Engine's build arguments, as the baseline support builds it.
const FREEZE_BUILD: [&str; 5] = [
    "--offline",
    "--bin",
    "rtm",
    "--features",
    "test-fault-injection",
];

/// The variables that move Cargo's build output.
const REDIRECTS: [&str; 3] = [
    "CARGO_TARGET_DIR",
    "CARGO_BUILD_TARGET_DIR",
    "CARGO_BUILD_TARGET",
];

/// The case a child of this suite runs; set only on the children it spawns.
const CASE: &str = "RATMAC_T137_CASE";
/// Where that child acknowledges entering its case, in a tree the parent owns.
const ACK: &str = "RATMAC_T137_ACK";
/// The build root the child hands the shared function.
const ROOT: &str = "RATMAC_T137_ROOT";
/// The fresh folder the child's redirect names; Cargo's report lies inside it.
const FRESH: &str = "RATMAC_T137_FRESH";

/// Stand-ins planted at a scratch copy's fixed folder: text, not a program,
/// so a launch of one could never pass for the Engine.
const STAND_IN_RTM_QA: &[u8] =
    b"t-137 stand-in: an older rtm-qa at the fixed folder; never run it\n";
const STAND_IN_RTM: &[u8] = b"t-137 stand-in: an older rtm at the fixed folder; never run it\n";

/// A compile error only the compiler's own messages carry into a refusal.
const PLANTED_ERROR: &str = "t-137 SVEV-002: this source file was made not to compile";

/// SVEV-001: wherever Cargo's output moves - `CARGO_TARGET_DIR`,
/// `CARGO_BUILD_TARGET_DIR`, `build.target-dir` in a configuration file, or
/// `CARGO_BUILD_TARGET` - the shared function returns exactly the
/// `executable` Cargo reported for `rtm-qa`, and the freeze Engine build
/// returns what Cargo reported for `rtm`.
///
/// Every redirect runs in its own child process with its own fresh folder.
/// The parent runs every case and asserts once, so a failure shows each
/// case's reason.
#[test]
fn svev_001() {
    if let Some(case) = child_case() {
        let fresh = enter(&case);
        let verdict = if case == "e" {
            freeze_child(&fresh)
        } else {
            rtm_qa_child(&env_path(ROOT), &fresh)
        };
        return settle(verdict);
    }
    let repo = baseline::repo_root();
    let host = host_target(&repo);
    let outcomes = [
        (
            "(a) CARGO_TARGET_DIR names a fresh folder; root: the repository".to_owned(),
            variable_case(&repo, "a", "CARGO_TARGET_DIR"),
        ),
        (
            "(b) CARGO_BUILD_TARGET_DIR names a fresh folder, CARGO_TARGET_DIR absent; \
             root: the repository"
                .to_owned(),
            variable_case(&repo, "b", "CARGO_BUILD_TARGET_DIR"),
        ),
        (
            "(c) build.target-dir in a scratch copy's .cargo/config.toml names a fresh folder, \
             every redirect variable cleared; root: the scratch copy"
                .to_owned(),
            config_case(&repo),
        ),
        (
            format!(
                "(d) CARGO_BUILD_TARGET={host}, CARGO_TARGET_DIR names a fresh folder; \
                 root: the repository"
            ),
            host_target_case(&repo, &host),
        ),
        (
            format!(
                "(e) the freeze build under CARGO_BUILD_TARGET={host} and CARGO_TARGET_DIR, \
                 TMP and TEMP naming a fresh folder"
            ),
            freeze_case(&host),
        ),
    ];
    let failed = outcomes
        .iter()
        .filter(|(_, outcome)| outcome.is_err())
        .count();
    let report: Vec<String> = outcomes
        .iter()
        .map(|(label, outcome)| match outcome {
            Ok(()) => format!("{label}: ok"),
            Err(reason) => format!("{label}: FAILED\n    {}", reason.replace('\n', "\n    ")),
        })
        .collect();
    assert!(
        failed == 0,
        "SVEV-001: under every redirect the shared function must return exactly the executable \
         Cargo reported for rtm-qa, and the freeze build what Cargo reported for rtm; \
         {failed} of {} cases failed:\n{}",
        outcomes.len(),
        report.join("\n")
    );
}

/// SVEV-002: a scratch copy whose fixed folder holds an older `rtm-qa`,
/// built with `CARGO_TARGET_DIR` naming a fresh folder. The valid case comes
/// first: the shared function returns exactly Cargo's report, which runs as
/// the Engine, while the stand-in keeps its bytes. Then it refuses a binary
/// the build never reported, naming it and the build, and a build that
/// cannot compile, carrying the compiler's text.
#[test]
fn svev_002() {
    if let Some(case) = child_case() {
        let fresh = enter(&case);
        return settle(older_binary_child(&fresh));
    }
    let fresh = fresh_tree("t137-svev002-output").unwrap_or_else(|error| panic!("{error}"));
    let folder = fresh.path().as_os_str();
    if let Err(reason) = run_case(
        "svev_002",
        "svev-002",
        &[(FRESH, folder), ("CARGO_TARGET_DIR", folder)],
    ) {
        panic!(
            "SVEV-002: with an older rtm-qa at a scratch copy's fixed folder and CARGO_TARGET_DIR \
             naming a fresh folder, the shared function must return exactly Cargo's report, \
             refuse a binary the build never reported, and carry a failed build's compiler \
             text:\n{reason}"
        );
    }
}

// --- the parent's cases ------------------------------------------------------

/// A case whose one redirect is `variable` naming a fresh folder, building
/// the repository itself.
fn variable_case(repo: &Path, case: &str, variable: &str) -> Result<(), String> {
    let fresh = fresh_tree(&format!("t137-{case}-output"))?;
    let folder = fresh.path().as_os_str();
    run_case(
        "svev_001",
        case,
        &[
            (ROOT, repo.as_os_str()),
            (FRESH, folder),
            (variable, folder),
        ],
    )
}

/// `build.target-dir` in a scratch copy's own `.cargo/config.toml`, with
/// every redirect variable cleared from the child; the checkout's own Cargo
/// configuration is absent before the case and after it.
fn config_case(repo: &Path) -> Result<(), String> {
    let own = [
        repo.join(".cargo").join("config.toml"),
        repo.join(".cargo").join("config"),
    ];
    if let Some(present) = own.iter().find(|path| path.exists()) {
        return Err(format!(
            "FIXTURE: the checkout already holds {}; this case needs it absent",
            present.display()
        ));
    }
    let scratch = fresh_tree("t137-c-copy")?;
    let fresh = fresh_tree("t137-c-output")?;
    copy_repository(repo, scratch.path())?;
    let setting = format!("[build]\ntarget-dir = {}\n", toml_string(fresh.path()));
    plant(
        &scratch.join(".cargo").join("config.toml"),
        setting.as_bytes(),
    )?;
    let outcome = run_case(
        "svev_001",
        "c",
        &[
            (ROOT, scratch.path().as_os_str()),
            (FRESH, fresh.path().as_os_str()),
        ],
    );
    match own.iter().find(|path| path.exists()) {
        Some(written) => Err(format!(
            "the checkout's own Cargo configuration {} was written during the case; \
             the case itself: {outcome:?}",
            written.display()
        )),
        None => outcome,
    }
}

/// `CARGO_BUILD_TARGET` set to the host target, with `CARGO_TARGET_DIR`
/// naming a fresh folder, building the repository itself.
fn host_target_case(repo: &Path, host: &str) -> Result<(), String> {
    let fresh = fresh_tree("t137-d-output")?;
    let folder = fresh.path().as_os_str();
    run_case(
        "svev_001",
        "d",
        &[
            (ROOT, repo.as_os_str()),
            (FRESH, folder),
            ("CARGO_TARGET_DIR", folder),
            ("CARGO_BUILD_TARGET", OsStr::new(host)),
        ],
    )
}

/// The freeze build under the same host-target redirect, with `TMP` and
/// `TEMP` naming a fresh folder so the freeze tree and its build are new.
fn freeze_case(host: &str) -> Result<(), String> {
    let output = fresh_tree("t137-e-output")?;
    let temporary = fresh_tree("t137-e-temp")?;
    let temp = temporary.path().as_os_str();
    run_case(
        "svev_001",
        "e",
        &[
            (FRESH, temp),
            ("TMP", temp),
            ("TEMP", temp),
            ("CARGO_TARGET_DIR", output.path().as_os_str()),
            ("CARGO_BUILD_TARGET", OsStr::new(host)),
        ],
    )
}

/// The redirect variables a case must not inherit: the parent removes
/// exactly these and the child proves they are gone.
fn cleared(case: &str) -> &'static [&'static str] {
    match case {
        "b" => &["CARGO_TARGET_DIR", "CARGO_BUILD_TARGET"],
        "c" => &REDIRECTS,
        "d" | "e" => &["CARGO_BUILD_TARGET_DIR"],
        _ => &["CARGO_BUILD_TARGET_DIR", "CARGO_BUILD_TARGET"],
    }
}

/// Run case `case` of the test `test` alone, in a child of this test
/// executable: scrubbed like every support child, the case's cleared
/// variables removed, then given `set`. `Ok` once the child proves it entered
/// that case and passed; otherwise the child's own account of its failure.
fn run_case(test: &str, case: &str, set: &[(&str, &OsStr)]) -> Result<(), String> {
    let marks = fresh_tree("t137-case-ack")?;
    let ack = marks.join("entered");
    let executable = std::env::current_exe()
        .map_err(|error| format!("FIXTURE: locate the test executable: {error}"))?;
    let directory = std::env::current_dir()
        .map_err(|error| format!("FIXTURE: locate the working directory: {error}"))?;
    let mut child = support::command(&executable, &directory);
    child
        .args(["--exact", test, "--test-threads", "1"])
        .env(CASE, case)
        .env(ACK, &ack);
    for name in cleared(case) {
        child.env_remove(name);
    }
    for (name, value) in set {
        child.env(name, value);
    }
    let output = child
        .output()
        .map_err(|error| format!("FIXTURE: launch case {case}: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.lines().any(|line| line.trim() == "running 1 test") {
        return Err(format!(
            "FIXTURE: case {case} did not select exactly one test:\n{}",
            support::text(&output)
        ));
    }
    if fs::read(&ack).ok().as_deref() != Some(case.as_bytes()) {
        return Err(format!(
            "FIXTURE: case {case} never entered its body:\n{}",
            support::text(&output)
        ));
    }
    let passed = format!("test {test} ... ok");
    if output.status.success() && stdout.lines().any(|line| line.trim() == passed) {
        return Ok(());
    }
    Err(child_failure(test, &output))
}

/// A failed child's own account: its panic text from libtest's failure
/// section, or its whole output when that section is missing.
fn child_failure(test: &str, output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let header = format!("---- {test} stdout ----");
    let section: Vec<&str> = stdout
        .lines()
        .skip_while(|line| line.trim() != header)
        .skip(1)
        .take_while(|line| line.trim() != "failures:")
        .filter(|line| !line.starts_with("note: run with `RUST_BACKTRACE"))
        .collect();
    let section = section.join("\n").trim().to_owned();
    if section.is_empty() {
        support::text(output)
    } else {
        section
    }
}

/// The host target `rustc -vV` reports: the one target offline builds here
/// have a standard library for.
fn host_target(repo: &Path) -> String {
    let output = support::command("rustc", repo)
        .arg("-vV")
        .output()
        .unwrap_or_else(|error| panic!("FIXTURE: run rustc -vV: {error}"));
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .unwrap_or_else(|| {
            panic!(
                "FIXTURE: rustc -vV named no host target:\n{}",
                support::text(&output)
            )
        })
}

// --- inside a child ----------------------------------------------------------

/// The case this process was started for, when it is a child of this suite.
fn child_case() -> Option<String> {
    std::env::var_os(CASE).map(|case| case.to_string_lossy().into_owned())
}

/// Enter the case: acknowledge it, prove the redirect variables it must not
/// inherit are gone, and return the fresh folder Cargo's report must lie in.
fn enter(case: &str) -> PathBuf {
    if let Some(ack) = std::env::var_os(ACK) {
        fs::write(&ack, case)
            .unwrap_or_else(|error| panic!("FIXTURE: acknowledge case {case}: {error}"));
    }
    if let Some(name) = cleared(case)
        .iter()
        .find(|name| std::env::var_os(name).is_some())
    {
        panic!("FIXTURE: case {case} still inherits {name}");
    }
    env_path(FRESH)
}

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(
        std::env::var_os(name)
            .unwrap_or_else(|| panic!("FIXTURE: the parent names no {name} for this case")),
    )
}

fn settle(verdict: Result<(), String>) {
    if let Err(reason) = verdict {
        panic!("{reason}");
    }
}

/// One redirect: the shared function's answer for `rtm-qa` against Cargo's
/// own report for the same root, arguments, and environment.
fn rtm_qa_child(root: &Path, fresh: &Path) -> Result<(), String> {
    let returned = built_executable(root, &RTM_QA, "rtm-qa");
    let reported = reported_executable(root, &RTM_QA, "rtm-qa", None)?;
    under_fresh(&reported, fresh)?;
    judge("built_executable", returned, &reported)
}

/// The freeze build under the host-target redirect: `baseline::engine()`
/// must return what Cargo reports for `rtm` when the freeze tree is built
/// into the freeze build's own output folder.
fn freeze_child(fresh: &Path) -> Result<(), String> {
    let temp = std::env::temp_dir();
    if !lies_under(&temp, fresh) {
        return Err(format!(
            "FIXTURE: TMP and TEMP did not move the temporary folder into {}: it is {}",
            fresh.display(),
            temp.display()
        ));
    }
    let returned = std::panic::catch_unwind(|| baseline::engine().to_path_buf())
        .map_err(|payload| panic_text(&*payload));
    // Where the freeze build keeps its tree and its own output folder.
    let base = temp.join(format!("ratmac-freeze-{FREEZE_COMMIT}"));
    let reported = reported_executable(
        &base.join("tree"),
        &FREEZE_BUILD,
        "rtm",
        Some(&base.join("build")),
    )
    .map_err(|error| format!("{error}\n  baseline::engine() gave: {returned:?}"))?;
    under_fresh(&reported, fresh)?;
    judge("baseline::engine()", returned, &reported)
}

/// SVEV-002 inside its child, where `CARGO_TARGET_DIR` names `fresh`.
fn older_binary_child(fresh: &Path) -> Result<(), String> {
    let scratch = fresh_tree("t137-svev002-copy")?;
    copy_repository(&baseline::repo_root(), scratch.path())?;
    let root = scratch.path();
    let shown_root = root.display().to_string();
    let fixed = root.join("target").join("debug");
    let suffix = std::env::consts::EXE_SUFFIX;
    let stand_in = fixed.join(format!("rtm-qa{suffix}"));
    let other_stand_in = fixed.join(format!("rtm{suffix}"));
    plant(&stand_in, STAND_IN_RTM_QA)?;
    plant(&other_stand_in, STAND_IN_RTM)?;

    // The valid case first: a good build answers with Cargo's own report.
    let returned = built_executable(root, &RTM_QA, "rtm-qa");
    let reported = reported_executable(root, &RTM_QA, "rtm-qa", None)?;
    under_fresh(&reported, fresh)?;
    if let Ok(path) = &returned {
        if *path == stand_in {
            return Err(format!(
                "built_executable returned the planted stand-in {} - the older file at the \
                 fixed folder - instead of Cargo's report\n  Cargo reported: {reported}",
                path.display()
            ));
        }
    }
    judge("built_executable", returned, &reported)?;
    runs_as_engine(Path::new(&reported))?;
    keeps(&stand_in, STAND_IN_RTM_QA)?;

    // A binary that build never reports: refused by name, nothing returned.
    match built_executable(root, &RTM_QA, "rtm") {
        Ok(path) => {
            return Err(format!(
                "asked for `rtm`, which `{RTM_QA_BUILD}` never reports, built_executable \
                 returned {} instead of refusing",
                path.display()
            ))
        }
        Err(refusal) => names(
            &refusal,
            "the refusal for `rtm`",
            &[
                ("the binary", "`rtm`"),
                ("the build command", RTM_QA_BUILD),
                ("the build root", &shown_root),
            ],
        )
        .and_then(|()| reports(&refusal, &reported))?,
    }
    keeps(&other_stand_in, STAND_IN_RTM)?;

    // A source file made not to compile: the refusal carries the compiler's text.
    append(
        &root.join("src").join("bin").join("rtm.rs"),
        &format!("\ncompile_error!(\"{PLANTED_ERROR}\");\n"),
    )?;
    // The compiler's own rendering opens with this line; a dump of the raw
    // JSON records would carry the message with an escaped line break instead.
    let rendered = format!("error: {PLANTED_ERROR}\n");
    match built_executable(root, &RTM_QA, "rtm-qa") {
        Ok(path) => {
            return Err(format!(
                "a build that cannot compile returned {} instead of refusing",
                path.display()
            ))
        }
        Err(refusal) => names(
            &refusal,
            "the refusal for a build that cannot compile",
            &[
                ("the compiler's rendered error text", &rendered),
                ("Cargo's standard error", "could not compile"),
                ("the binary", "`rtm-qa`"),
                ("the build command", RTM_QA_BUILD),
                ("the build root", &shown_root),
            ],
        )?,
    }
    keeps(&stand_in, STAND_IN_RTM_QA)?;
    keeps(&other_stand_in, STAND_IN_RTM)
}

// --- the comparison and its judgments ----------------------------------------

/// What Cargo itself reports for `binary`: the one distinct non-empty
/// `executable` among that target's `compiler-artifact` records when `root`
/// is built with `args` in this process's environment - plus
/// `CARGO_TARGET_DIR` when `target_dir` is given, as the freeze build sets
/// it. Read here, independently of the function under test.
fn reported_executable(
    root: &Path,
    args: &[&str],
    binary: &str,
    target_dir: Option<&Path>,
) -> Result<String, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut build = Command::new(cargo);
    build
        .arg("build")
        .arg("--message-format=json")
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null());
    if let Some(folder) = target_dir {
        build.env("CARGO_TARGET_DIR", folder);
    }
    let shown = format!(
        "the comparison build `cargo build --message-format=json {}` in {}",
        args.join(" "),
        root.display()
    );
    let output = build
        .output()
        .map_err(|error| format!("FIXTURE: {shown} did not start: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "FIXTURE: {shown} failed:\n{}",
            support::text(&output)
        ));
    }
    let mut paths = BTreeSet::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Ok(message) = Json::parse(line) else {
            continue;
        };
        let target = message.as_object().and_then(|fields| fields.get("target"));
        if message.field("reason") == Some("compiler-artifact")
            && target.and_then(|target| target.field("name")) == Some(binary)
        {
            if let Some(path) = message.field("executable").filter(|path| !path.is_empty()) {
                paths.insert(path.to_owned());
            }
        }
    }
    if paths.len() == 1 {
        Ok(paths.into_iter().next().unwrap_or_default())
    } else {
        Err(format!(
            "FIXTURE: {shown} reported {} executables for `{binary}`: {paths:?}",
            paths.len()
        ))
    }
}

/// The answer must be exactly the string Cargo reported, naming a file.
fn judge(who: &str, returned: Result<PathBuf, String>, reported: &str) -> Result<(), String> {
    match returned {
        Err(refusal) => Err(format!(
            "{who} gave no path: {refusal}\n  Cargo reported: {reported}"
        )),
        Ok(path) if path.as_os_str() != OsStr::new(reported) => Err(format!(
            "{who} returned {}\n  Cargo reported: {reported}",
            path.display()
        )),
        Ok(path) if !path.is_file() => Err(format!(
            "{who} returned Cargo's report {reported}, but no file is there"
        )),
        Ok(_) => Ok(()),
    }
}

/// A comparison is only worth something when the redirect moved the build.
fn under_fresh(reported: &str, fresh: &Path) -> Result<(), String> {
    if lies_under(Path::new(reported), fresh) {
        Ok(())
    } else {
        Err(format!(
            "FIXTURE: Cargo reported {reported}, outside the fresh folder {} - the redirect \
             never moved the build, so the comparison would prove nothing",
            fresh.display()
        ))
    }
}

fn lies_under(path: &Path, folder: &Path) -> bool {
    match (fs::canonicalize(path), fs::canonicalize(folder)) {
        (Ok(path), Ok(folder)) => path.starts_with(folder),
        _ => path.starts_with(folder),
    }
}

/// A refusal must carry each named piece (see `carries`); the missing ones
/// are listed with the full refusal.
fn names(refusal: &str, what: &str, pieces: &[(&str, &str)]) -> Result<(), String> {
    let missing: Vec<String> = pieces
        .iter()
        .filter(|(_, piece)| !carries(refusal, piece))
        .map(|(label, piece)| format!("{label} ({piece:?})"))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{what} must name {}; it does not:\n{refusal}",
            missing.join(", ")
        ))
    }
}

/// A refusal for a binary the build never reported lists what Cargo did
/// report: here, `rtm-qa`'s executable as Cargo spelled it (quoted forms
/// that double each backslash are accepted too).
fn reports(refusal: &str, reported: &str) -> Result<(), String> {
    if carries(refusal, reported) {
        Ok(())
    } else {
        Err(format!(
            "the refusal for `rtm` must list what Cargo reported, including rtm-qa's \
             executable {reported}; it does not:\n{refusal}"
        ))
    }
}

/// Whether `text` carries `piece` as written, or in the quoted form that
/// doubles each backslash (a path shown through `{:?}`).
fn carries(text: &str, piece: &str) -> bool {
    text.contains(piece) || text.contains(&piece.replace('\\', "\\\\"))
}

/// Cargo's report runs as the Engine: `--help` answers with the Engine's
/// usage from an empty folder, resolving, reading, and writing nothing.
fn runs_as_engine(path: &Path) -> Result<(), String> {
    let folder = fresh_tree("t137-help")?;
    let output = support::command(path, folder.path())
        .arg("--help")
        .output()
        .map_err(|error| format!("launch {} --help: {error}", path.display()))?;
    if output.status.success()
        && String::from_utf8_lossy(&output.stdout).starts_with("Usage: rtm <command>")
    {
        Ok(())
    } else {
        Err(format!(
            "Cargo's reported {} did not answer --help as the Engine:\n{}",
            path.display(),
            support::text(&output)
        ))
    }
}

fn keeps(path: &Path, bytes: &[u8]) -> Result<(), String> {
    match fs::read(path) {
        Ok(now) if now == bytes => Ok(()),
        Ok(now) => Err(format!(
            "the stand-in at {} lost its bytes: now {:?}",
            path.display(),
            String::from_utf8_lossy(&now)
        )),
        Err(error) => Err(format!(
            "the stand-in at {} is gone: {error}",
            path.display()
        )),
    }
}

fn panic_text(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_owned()
    } else {
        "a panic without a message".to_owned()
    }
}

// --- scratch copies ----------------------------------------------------------

fn fresh_tree(label: &str) -> Result<TempTree, String> {
    TempTree::new(label).map_err(|error| format!("FIXTURE: {error}"))
}

/// Copy what a build of `rtm-qa` reads - the workspace manifest and lockfile,
/// the Engine's sources and packaged profile, and the test crate - from
/// `repo` into `to`, never a `target/` folder.
fn copy_repository(repo: &Path, to: &Path) -> Result<(), String> {
    for file in ["Cargo.toml", "Cargo.lock"] {
        fs::copy(repo.join(file), to.join(file))
            .map_err(|error| format!("FIXTURE: copy {file}: {error}"))?;
    }
    for folder in ["src", "profiles", "test/qa"] {
        copy_tree(&repo.join(folder), &to.join(folder))?;
    }
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|error| format!("FIXTURE: create {}: {error}", to.display()))?;
    let entries =
        fs::read_dir(from).map_err(|error| format!("FIXTURE: list {}: {error}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("FIXTURE: list {}: {error}", from.display()))?;
        let kind = entry
            .file_type()
            .map_err(|error| format!("FIXTURE: inspect {}: {error}", entry.path().display()))?;
        let copy = to.join(entry.file_name());
        if kind.is_dir() {
            if entry.file_name() != "target" {
                copy_tree(&entry.path(), &copy)?;
            }
        } else if kind.is_file() {
            fs::copy(entry.path(), &copy)
                .map_err(|error| format!("FIXTURE: copy {}: {error}", entry.path().display()))?;
        }
    }
    Ok(())
}

fn plant(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("FIXTURE: create {}: {error}", parent.display()))?;
    }
    fs::write(path, bytes).map_err(|error| format!("FIXTURE: write {}: {error}", path.display()))
}

fn append(path: &Path, text: &str) -> Result<(), String> {
    fs::OpenOptions::new()
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .map_err(|error| format!("FIXTURE: append to {}: {error}", path.display()))
}

/// `path` as a TOML basic string.
fn toml_string(path: &Path) -> String {
    let text = path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!("\"{text}\"")
}
