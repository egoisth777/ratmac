//! WEB-002 (t-121): the Engine-root tracking policy.
//!
//! One idempotent step, shared by `rtm init` and `Scheduler::start` (after
//! the runbook validation, before the root lock or any runtime write): the
//! resolved Engine root's `.gitignore` carries the four runtime lines
//! `/runs/`, `/mint.toml`, `/locks/`, and `/log.md`, each exactly once,
//! while the runbook and `evidence/` stay trackable. Existing bytes are
//! preserved; a conflicting effective rule, an indexed runtime file, or a
//! failed repository inspection refuses without mutation.
//!
//! Every refusal ends `nothing was modified`, and the step proves it: it
//! stages then replaces, rolls its own bytes back on a verified conflict,
//! and removes any Engine-root directory it alone created. Concurrent steps
//! serialize on a transient lock directly in the Engine root
//! (`lock::PolicyLock`), which never creates `locks/` and removes itself.

use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::machine::MachineClass;
use crate::root::Displayed;

/// The four policy lines, in the contract's order.
const PATTERNS: [&str; 4] = ["/runs/", "/mint.toml", "/locks/", "/log.md"];

/// The policy file's name within the Engine root.
const POLICY_FILE: &str = ".gitignore";

/// The staged copy's name within the Engine root: replaced onto the policy
/// file, never left beside it.
const STAGED_FILE: &str = "gitignore.policy.stage";

/// The runtime probe tails beneath the Engine root, in the contract's
/// verification-table order.
const RUNTIME_PROBES: [&str; 4] = ["runs/run-001", "mint.toml", "locks/root.lock", "log.md"];

/// The runtime paths whose presence in Git's index refuses (class C), in
/// the same order.
const RUNTIME_PATHS: [&str; 4] = ["runs", "mint.toml", "locks", "log.md"];

/// The authoring probe tails beneath the invoking checkout's own Engine
/// directory, in the contract's verification-table order.
const AUTHORING_PROBES: [&str; 2] = [MachineClass::FILE_NAME, "evidence/run-001"];

/// The policy file of one Engine root.
pub(crate) fn policy_path(engine_root: &Path) -> PathBuf {
    engine_root.join(POLICY_FILE)
}

/// What one completed policy step must undo when a later refusal rolls the
/// step back: restore the prior bytes (or remove the file the step created)
/// only while the file still holds the bytes this step wrote, and remove an
/// Engine root the step alone created, only while it is empty.
pub(crate) struct Rollback {
    engine_root: PathBuf,
    policy_path: PathBuf,
    prior: Option<Vec<u8>>,
    written: Vec<u8>,
    created_engine_root: bool,
}

impl Rollback {
    /// Undo this step's writes exactly; never touch another writer's bytes.
    pub(crate) fn rollback(&self) {
        if fs::read(&self.policy_path).is_ok_and(|current| current == self.written) {
            match &self.prior {
                Some(bytes) => {
                    let _ = fs::write(&self.policy_path, bytes);
                }
                None => {
                    let _ = fs::remove_file(&self.policy_path);
                }
            }
        }
        if self.created_engine_root {
            // Succeeds only while the directory is empty, so a directory
            // another writer filled is never removed.
            let _ = fs::remove_dir(&self.engine_root);
        }
    }
}

/// One completed policy step: whether it wrote bytes, whether a repository
/// was verified, and how to undo it.
pub(crate) struct Outcome {
    pub(crate) installed: bool,
    pub(crate) verified: bool,
    pub(crate) rollback: Rollback,
}

/// A policy refusal; the caller prefixes its command (`rtm init: `,
/// `rtm: start: `). Every text ends `nothing was modified`.
pub(crate) struct Refusal(String);

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Run the one policy step for the Engine root `rtm init` installs.
pub(crate) fn install(invoking_checkout: &Path, engine_root: &Path) -> Result<Outcome, Refusal> {
    let policy = policy_path(engine_root);
    // The pause seam's companions are validated before any claim or write.
    check_pause_companions()?;
    let created_engine_root = ensure_engine_root(engine_root, &policy)?;
    // WEB-002 (t-136): the Git inspection that reads no policy bytes runs
    // before the claim, so a waiter never queues behind it. Its refusal
    // texts are unchanged; an indexed runtime file now outranks a busy
    // claim.
    let inside = match inspect(invoking_checkout, engine_root) {
        Ok(inside) => inside,
        Err(refusal) => {
            cleanup_created_root(created_engine_root, engine_root);
            return Err(refusal);
        }
    };
    // The step's serialization: a transient claim directly in the Engine
    // root, never a `locks/` directory, removed when the step ends.
    let lock = crate::lock::PolicyLock::acquire(engine_root).map_err(|error| {
        cleanup_created_root(created_engine_root, engine_root);
        Refusal(format!(
            "refusing to install the ignore policy: cannot serialize the policy step: {error}; \
             nothing was modified"
        ))
    })?;
    let staged = engine_root.join(STAGED_FILE);
    let mut staged_created = false;
    let result = step(
        invoking_checkout,
        engine_root,
        &policy,
        &staged,
        &mut staged_created,
        inside,
        lock,
    );
    match result {
        Ok(mut outcome) => {
            outcome.rollback.created_engine_root = created_engine_root;
            Ok(outcome)
        }
        Err(refusal) => {
            if staged_created {
                let _ = fs::remove_file(&staged);
            }
            cleanup_created_root(created_engine_root, engine_root);
            Err(refusal)
        }
    }
}

/// Remove an Engine root this step alone created, only while it is empty.
fn cleanup_created_root(created: bool, engine_root: &Path) {
    if created {
        let _ = fs::remove_dir(engine_root);
    }
}

/// The pre-claim Git inspection (WEB-002, t-136): the inside-a-work-tree
/// classification and the indexed-runtime refusal. Neither reads the
/// policy bytes, so both run before the claim is taken; a waiter never
/// queues behind them.
fn inspect(invoking_checkout: &Path, engine_root: &Path) -> Result<bool, Refusal> {
    // The `inspect` pause boundary parks here, inside the pre-claim
    // inspection, holding no claim.
    wait_at_policy_pause("inspect")?;
    // Classification first: Git absent or the directory outside a
    // repository means no verification, never a refusal.
    let inside = classify(invoking_checkout)?;
    if inside {
        refuse_indexed_runtime(engine_root)?;
    }
    Ok(inside)
}

/// The serialized body of the policy step. The claim covers only the
/// policy bytes: a caller that writes nothing releases it before its
/// verification, while a writer keeps it through verification and, on a
/// refusal, through restoring the prior bytes.
fn step(
    invoking_checkout: &Path,
    engine_root: &Path,
    policy: &Path,
    staged: &Path,
    staged_created: &mut bool,
    inside: bool,
    lock: crate::lock::PolicyLock,
) -> Result<Outcome, Refusal> {
    let existing = read_policy(policy)?;
    let text = existing
        .as_deref()
        .map(|bytes| String::from_utf8(bytes.to_vec()))
        .transpose()
        .map_err(|_| cannot_read(policy))?
        .unwrap_or_default();
    let missing: Vec<&'static str> = PATTERNS
        .iter()
        .filter(|pattern| !line_present(&text, pattern))
        .copied()
        .collect();
    let installed = !missing.is_empty();
    // The claim lives exactly as long as the bytes do: a non-writer
    // releases it before its verification, which then runs unserialized;
    // a writer keeps it through verification and any restoration.
    let mut claim = Some(lock);
    let mut written = existing.clone().unwrap_or_default();
    if installed {
        if !written.is_empty() && !written.ends_with(b"\n") {
            written.push(b'\n');
        }
        for pattern in &missing {
            written.extend_from_slice(pattern.as_bytes());
            written.push(b'\n');
        }
        write_policy(staged, policy, &written, staged_created)?;
    } else {
        drop(claim.take());
    }
    let rollback = Rollback {
        engine_root: engine_root.to_path_buf(),
        policy_path: policy.to_path_buf(),
        prior: existing,
        written: written.clone(),
        created_engine_root: false,
    };
    if inside {
        if let Err(refusal) = verify(invoking_checkout, engine_root) {
            // The writer's claim survives through this restoration; a
            // non-writer released above and restores its own unchanged
            // bytes.
            rollback.rollback();
            return Err(refusal);
        }
    }
    Ok(Outcome {
        installed,
        verified: inside,
        rollback,
    })
}

/// Whether the Engine root exists as a directory, creating it only when
/// absent. Anything else is class F.
fn ensure_engine_root(engine_root: &Path, policy: &Path) -> Result<bool, Refusal> {
    match fs::symlink_metadata(engine_root) {
        Ok(metadata) if metadata.is_dir() => Ok(false),
        Ok(_) => Err(cannot_write(policy)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match fs::create_dir(engine_root) {
                Ok(()) => Ok(true),
                // A concurrent step may have created the root between the
                // check and the create; only a directory wins that race.
                Err(_) if fs::symlink_metadata(engine_root).is_ok_and(|m| m.is_dir()) => Ok(false),
                Err(_) => Err(cannot_write(policy)),
            }
        }
        Err(_) => Err(cannot_write(policy)),
    }
}

/// The existing policy bytes; class E when they cannot be read as one
/// UTF-8 file.
fn read_policy(policy: &Path) -> Result<Option<Vec<u8>>, Refusal> {
    match fs::read(policy) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(cannot_read(policy)),
    }
}

/// A policy line counts as present when the file holds a line whose text,
/// after removing one trailing `\r`, equals the pattern exactly.
fn line_present(text: &str, pattern: &str) -> bool {
    text.lines()
        .any(|line| line.strip_suffix('\r').unwrap_or(line) == pattern)
}

/// Stage the final bytes beside the policy file, then replace it. Class F
/// when either step cannot write; the fault seams below fail the staged
/// write and the replacement separately.
fn write_policy(
    staged: &Path,
    policy: &Path,
    bytes: &[u8],
    staged_created: &mut bool,
) -> Result<(), Refusal> {
    let staged_write = (|| -> std::io::Result<()> {
        let mut file = fs::File::create(staged)?;
        *staged_created = true;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    })();
    if staged_write.is_err() {
        return Err(cannot_write(policy));
    }
    if let Some(refusal) = injected_fault("staged-write") {
        return Err(refusal);
    }
    if let Some(refusal) = injected_fault("replace") {
        return Err(refusal);
    }
    fs::rename(staged, policy).map_err(|_| cannot_write(policy))
}

/// Classify the invoking checkout: inside a Git work tree, outside one, or
/// a repository that cannot be inspected (class D). Git absent - the
/// executable cannot be spawned - is no verification, never a refusal.
fn classify(invoking_checkout: &Path) -> Result<bool, Refusal> {
    let output = match crate::root::git_command(invoking_checkout)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
    {
        Ok(output) => output,
        Err(_) => return Ok(false),
    };
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim() == "true");
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("not a git repository") {
        return Ok(false);
    }
    Err(cannot_inspect("rev-parse --is-inside-work-tree", &stderr))
}

/// Class C: Git's index must name nothing under the four runtime paths,
/// queried in the Engine root's own checkout (the primary in a linked
/// worktree).
fn refuse_indexed_runtime(engine_root: &Path) -> Result<(), Refusal> {
    let Some(checkout) = engine_root.parent() else {
        return Ok(());
    };
    let runtime: Vec<PathBuf> = RUNTIME_PATHS
        .iter()
        .map(|tail| engine_root.join(tail))
        .collect();
    let query = format!(
        "ls-files -- {}",
        runtime
            .iter()
            .map(|path| path.displayed())
            .collect::<Vec<_>>()
            .join(" ")
    );
    let mut command = crate::root::git_command(checkout);
    command.arg("ls-files").arg("--").args(&runtime);
    let output = match command.output() {
        Ok(output) => output,
        Err(error) => return Err(cannot_inspect(&query, &error.to_string())),
    };
    if !output.status.success() {
        return Err(cannot_inspect(
            &query,
            &String::from_utf8_lossy(&output.stderr),
        ));
    }
    let rows = String::from_utf8_lossy(&output.stdout);
    let rows: Vec<&str> = rows.lines().filter(|row| !row.is_empty()).collect();
    for tail in RUNTIME_PATHS {
        let relative = format!("{}/{}", crate::root::ENGINE_DIR, tail);
        let beneath = format!("{relative}/");
        if let Some(row) = rows
            .iter()
            .find(|row| **row == relative || row.starts_with(&beneath))
        {
            return Err(Refusal(format!(
                "refusing to install the ignore policy: the runtime path {} is already in Git's \
                 index; untrack it with git rm --cached {}, then retry; nothing was modified",
                checkout.join(row).displayed(),
                checkout.join(row).displayed(),
            )));
        }
    }
    Ok(())
}

/// The verified effective result, probed with the policy bytes in place:
/// the runtime probes ignored, then the authoring probes (the policy file
/// first) not ignored, each in the contract's table order. A conflict rolls
/// the bytes back before it refuses.
fn verify(invoking_checkout: &Path, engine_root: &Path) -> Result<(), Refusal> {
    // The `verify` pause boundary parks at the start of effective-rule
    // verification, holding the claim only for a caller that wrote - a
    // non-writer released the claim before this call.
    wait_at_policy_pause("verify")?;
    let Some(engine_checkout) = engine_root.parent() else {
        return Ok(());
    };
    let relative = |tail: &str| format!("{}/{}", crate::root::ENGINE_DIR, tail);
    let mut engine_probes: Vec<String> = RUNTIME_PROBES.iter().map(|tail| relative(tail)).collect();
    engine_probes.push(relative(POLICY_FILE));
    let rows = check_ignore(engine_checkout, &engine_probes)?;
    for (tail, probe) in RUNTIME_PROBES.iter().zip(&engine_probes) {
        if let Some(rule) = effective_rule(&rows, probe) {
            if !rule_ignores(rule) {
                return Err(Refusal(format!(
                    "refusing to install the ignore policy: the runtime path {} would stay \
                     visible to Git ({rule}); remove or repair that rule, then retry; nothing \
                     was modified",
                    engine_root.join(tail).displayed(),
                )));
            }
        } else {
            return Err(Refusal(format!(
                "refusing to install the ignore policy: the runtime path {} would stay visible \
                 to Git (::); remove or repair that rule, then retry; nothing was modified",
                engine_root.join(tail).displayed(),
            )));
        }
    }
    let policy_probe = &engine_probes[RUNTIME_PROBES.len()];
    if let Some(rule) = effective_rule(&rows, policy_probe) {
        if rule_ignores(rule) {
            return Err(authoring_ignored(&engine_root.join(POLICY_FILE), rule));
        }
    }
    let authoring: Vec<String> = AUTHORING_PROBES.iter().map(|tail| relative(tail)).collect();
    let rows = check_ignore(invoking_checkout, &authoring)?;
    for tail in AUTHORING_PROBES {
        if let Some(rule) = effective_rule(&rows, &relative(tail)) {
            if rule_ignores(rule) {
                return Err(authoring_ignored(
                    &invoking_checkout.join(crate::root::ENGINE_DIR).join(tail),
                    rule,
                ));
            }
        }
    }
    Ok(())
}

/// Class B: an authoring probe is ignored.
fn authoring_ignored(path: &Path, rule: &str) -> Refusal {
    Refusal(format!(
        "refusing to install the ignore policy: the authoring path {} would be ignored ({rule}); \
         repair that rule so ratmac.toml and evidence/ stay trackable, then retry; nothing was \
         modified",
        path.displayed(),
    ))
}

/// One batched index-aware `git check-ignore -v --non-matching` query over
/// `probes` in `checkout`, answering `<source>:<linenum>:<pattern>` per
/// path. A failed query inside an established repository is class D.
fn check_ignore(checkout: &Path, probes: &[String]) -> Result<Vec<(String, String)>, Refusal> {
    let query = format!("check-ignore -v --non-matching -- {}", probes.join(" "));
    let mut command = crate::root::git_command(checkout);
    command
        .args(["check-ignore", "-v", "--non-matching", "--"])
        .args(probes);
    let output = match command.output() {
        Ok(output) => output,
        Err(error) => return Err(cannot_inspect(&query, &error.to_string())),
    };
    // Exit 1 is check-ignore's "nothing is ignored" verdict, not a failure.
    if !(output.status.success() || output.status.code() == Some(1)) {
        return Err(cannot_inspect(
            &query,
            &String::from_utf8_lossy(&output.stderr),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(rule, path)| (rule.to_owned(), path.to_owned()))
        .collect())
}

/// The rule `git check-ignore -v` reports for `probe`, exactly as printed;
/// a tracked path is never reported, so it answers `None`.
fn effective_rule<'a>(rows: &'a [(String, String)], probe: &str) -> Option<&'a str> {
    rows.iter()
        .find(|(_, path)| path == probe)
        .map(|(rule, _)| rule.as_str())
}

/// Whether a reported rule ignores its path: a `!` pattern is a negation,
/// and the `::` row of `--non-matching` is no rule at all.
fn rule_ignores(rule: &str) -> bool {
    !rule.starts_with(':')
        && !rule
            .rsplit_once(':')
            .is_some_and(|(_, pattern)| pattern.starts_with('!'))
}

/// Class D: a repository query failed inside an available repository.
fn cannot_inspect(query: &str, stderr: &str) -> Refusal {
    Refusal(format!(
        "refusing to install the ignore policy: cannot inspect the repository: git {query} \
         failed: {}; repair the repository, then retry; nothing was modified",
        stderr.lines().next().unwrap_or_default(),
    ))
}

/// Class E: the policy file cannot be read as one UTF-8 file.
fn cannot_read(policy: &Path) -> Refusal {
    Refusal(format!(
        "refusing to install the ignore policy: cannot read {}; make it a readable UTF-8 file \
         (not a directory), then retry; nothing was modified",
        policy.displayed(),
    ))
}

/// Class F: the destination cannot be created or written.
fn cannot_write(policy: &Path) -> Refusal {
    Refusal(format!(
        "refusing to install the ignore policy: cannot create or write {}; make the Engine root \
         a directory and the policy file writable, then retry; nothing was modified",
        policy.displayed(),
    ))
}

/// The injected init faults, compiled only with `test-fault-injection`.
#[cfg(feature = "test-fault-injection")]
fn injected_fault(boundary: &str) -> Option<Refusal> {
    let fault = std::env::var("RATMAC_TEST_INIT_FAULT").ok()?;
    (fault == boundary).then(|| {
        Refusal(format!(
            "injected init fault at {boundary}; nothing was modified"
        ))
    })
}

/// A build without `test-fault-injection` injects nothing.
#[cfg(not(feature = "test-fault-injection"))]
fn injected_fault(_boundary: &str) -> Option<Refusal> {
    None
}

/// The policy pause seam (t-136), compiled only with `test-fault-injection`.
/// It mirrors the recover pause seam in every rule: the marker content is
/// the boundary name plus LF, release happens when the release file exists,
/// the default timeout is 60000 ms, and an unset, empty, or unrecognized
/// `RATMAC_TEST_POLICY_PAUSE` value is inert. Expiry refuses through the
/// step's ordinary refusal path.
#[cfg(feature = "test-fault-injection")]
fn wait_at_policy_pause(boundary: &str) -> Result<(), Refusal> {
    if std::env::var("RATMAC_TEST_POLICY_PAUSE").as_deref() != Ok(boundary) {
        return Ok(());
    }
    let marker = std::env::var_os("RATMAC_TEST_POLICY_PAUSE_MARKER")
        .map(PathBuf::from)
        .unwrap_or_default();
    let release = std::env::var_os("RATMAC_TEST_POLICY_PAUSE_RELEASE")
        .map(PathBuf::from)
        .unwrap_or_default();
    let timeout = std::env::var("RATMAC_TEST_POLICY_PAUSE_TIMEOUT_MILLIS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(60000);
    if !marker.as_os_str().is_empty() {
        if let Some(parent) = marker.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    Refusal(format!(
                        "refusing to install the ignore policy: the policy pause seam cannot \
                         create the marker directory {}: {error}; nothing was modified",
                        parent.displayed()
                    ))
                })?;
            }
        }
        std::fs::write(&marker, format!("{boundary}\n")).map_err(|error| {
            Refusal(format!(
                "refusing to install the ignore policy: the policy pause seam cannot write \
                 the marker {}: {error}; nothing was modified",
                marker.displayed()
            ))
        })?;
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout);
    loop {
        if release.is_file() {
            return Ok(());
        }
        let now = std::time::Instant::now();
        if now >= deadline {
            return Err(Refusal(format!(
                "refusing to install the ignore policy: injected policy pause at {boundary} \
                 expired; nothing was modified"
            )));
        }
        std::thread::sleep(
            std::time::Duration::from_millis(10).min(deadline.saturating_duration_since(now)),
        );
    }
}

/// A build without `test-fault-injection` pauses nowhere.
#[cfg(not(feature = "test-fault-injection"))]
fn wait_at_policy_pause(_boundary: &str) -> Result<(), Refusal> {
    Ok(())
}

/// The pause seam's companions are validated when the step starts, before
/// any claim or write. Only a recognized boundary arms the seam; an unset,
/// empty, or unrecognized value changes nothing, even with no companion
/// present. The timeout companion is optional and defaults to 60000 ms;
/// when it is set it must be a positive integer.
#[cfg(feature = "test-fault-injection")]
fn check_pause_companions() -> Result<(), Refusal> {
    let Ok(boundary) = std::env::var("RATMAC_TEST_POLICY_PAUSE") else {
        return Ok(());
    };
    if !matches!(boundary.as_str(), "inspect" | "verify") {
        return Ok(());
    }
    for companion in [
        "RATMAC_TEST_POLICY_PAUSE_MARKER",
        "RATMAC_TEST_POLICY_PAUSE_RELEASE",
    ] {
        let missing = std::env::var_os(companion)
            .map(|value| value.is_empty())
            .unwrap_or(true);
        if missing {
            return Err(Refusal(format!(
                "refusing to install the ignore policy: the policy pause seam names no \
                 {companion} companion; the pause test seam requires one before any claim or \
                 write; nothing was modified"
            )));
        }
    }
    if let Ok(value) = std::env::var("RATMAC_TEST_POLICY_PAUSE_TIMEOUT_MILLIS") {
        let timeout_valid = value.parse::<u64>().is_ok_and(|millis| millis > 0);
        if !timeout_valid {
            return Err(Refusal(
                "refusing to install the ignore policy: the policy pause seam names no positive \
                 integer RATMAC_TEST_POLICY_PAUSE_TIMEOUT_MILLIS companion; the pause test seam \
                 requires one before any claim or write; nothing was modified"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

/// A build without `test-fault-injection` checks nothing.
#[cfg(not(feature = "test-fault-injection"))]
fn check_pause_companions() -> Result<(), Refusal> {
    Ok(())
}
