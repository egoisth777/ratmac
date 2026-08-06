//! Engine lock domains.
//!
//! The Engine has exactly two lock paths.  The short root lock protects
//! minting and shared roster or ledger mutation; a Run lock protects motion of
//! one addressed Run.  Keeping path construction here makes that split
//! mechanically visible to every caller.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::state::StateError;

/// The fixed upper bound for waiting on a contended Engine lock.
pub const WAIT_TIMEOUT: Duration = Duration::from_secs(5);

const RETRY_INTERVAL: Duration = Duration::from_millis(10);
static TOKEN_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// The one root-domain lock path for an Engine root.
pub fn root_path(engine_root: &Path) -> PathBuf {
    engine_root.join("locks").join("root.lock")
}

/// The one motion-lock path for an addressed Run under an Engine root.
pub fn run_path(engine_root: &Path, run_id: &str) -> PathBuf {
    engine_root
        .join("locks")
        .join("runs")
        .join(format!("{run_id}.lock"))
}

/// A held root-domain lock.
#[derive(Debug)]
pub struct RootLock(OwnedLock);

impl RootLock {
    /// Acquire the root lock for minting or shared roster or ledger mutation.
    pub fn acquire(engine_root: &Path) -> Result<Self, StateError> {
        let path = root_path(engine_root);
        acquire(engine_root, path, "root", "root").map(Self)
    }

    /// The exact root-lock path this guard owns.
    pub fn path(&self) -> &Path {
        self.0.path()
    }
}

/// A held motion lock for one Run.
#[derive(Debug)]
pub struct RunLock(OwnedLock);

impl RunLock {
    /// Acquire the lock for motion on `run_id`.
    pub fn acquire(engine_root: &Path, run_id: &str) -> Result<Self, StateError> {
        let path = run_path(engine_root, run_id);
        acquire(engine_root, path, &format!("run:{run_id}"), "run").map(Self)
    }

    /// The exact Run-lock path this guard owns.
    pub fn path(&self) -> &Path {
        self.0.path()
    }
}

#[derive(Debug)]
struct OwnedLock {
    path: PathBuf,
    token: Vec<u8>,
}

impl OwnedLock {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for OwnedLock {
    fn drop(&mut self) {
        // A pathname can be reused after a stale-lock retirement.  Compare the
        // complete owner token before unlinking, so this guard cannot remove a
        // later owner's lock merely because it once owned the same pathname.
        if fs::read(&self.path).ok().as_deref() == Some(self.token.as_slice()) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn acquire(
    engine_root: &Path,
    path: PathBuf,
    guard: &str,
    fault_domain: &str,
) -> Result<OwnedLock, StateError> {
    let parent = path
        .parent()
        .ok_or_else(|| StateError::new(format!("lock path has no parent: {}", path.display())))?;
    fs::create_dir_all(parent).map_err(|error| {
        StateError::new(format!(
            "create lock directory {}: {error}",
            parent.display()
        ))
    })?;
    refuse_legacy(engine_root)?;

    let deadline = Instant::now() + WAIT_TIMEOUT;
    loop {
        match try_acquire(&path, guard) {
            Ok(lock) => {
                if let Err(error) = refuse_legacy(engine_root) {
                    drop(lock);
                    return Err(error);
                }
                inject_lock_fault(fault_domain, &path)?;
                hold_lock_if_requested(fault_domain, &path)?;
                return Ok(lock);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::PermissionDenied
                ) =>
            {
                refuse_legacy(engine_root)?;
                let now = Instant::now();
                if now >= deadline {
                    return Err(StateError::new(format!(
                        "lock wait expired: {}",
                        path.display()
                    )));
                }
                thread::sleep(RETRY_INTERVAL.min(deadline.saturating_duration_since(now)));
            }
            Err(error) => {
                return Err(StateError::new(format!(
                    "create lock {}: {error}",
                    path.display()
                )));
            }
        }
    }
}

fn try_acquire(path: &Path, guard: &str) -> std::io::Result<OwnedLock> {
    let token = owner_token(guard);
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    if let Err(error) = file.write_all(&token).and_then(|()| file.sync_all()) {
        drop(file);
        // This process created the pathname and no other standard acquirer can
        // own it until it is removed, so cleanup here cannot remove another
        // owner's lock.
        let _ = fs::remove_file(path);
        return Err(error);
    }
    Ok(OwnedLock {
        path: path.to_path_buf(),
        token,
    })
}

fn owner_token(guard: &str) -> Vec<u8> {
    let sequence = TOKEN_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!(
        "ratmac-lock-v1\npid={}\nguard={guard}\nnonce={sequence}\n",
        std::process::id()
    )
    .into_bytes()
}

/// Refuse the legacy invocation-lock residue before any command proceeds.
pub(crate) fn refuse_legacy(engine_root: &Path) -> Result<(), StateError> {
    let legacy_path = engine_root.join("schd.lock");
    match fs::symlink_metadata(&legacy_path) {
        Ok(_) => Err(StateError::new(format!(
            "refusing to run: legacy lock {} exists; explicitly migrate or remove that lock, then retry; it was not modified",
            legacy_path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(StateError::new(format!(
            "inspect legacy lock {}: {error}",
            legacy_path.display()
        ))),
    }
}

#[cfg(feature = "test-fault-injection")]
fn inject_lock_fault(domain: &str, path: &Path) -> Result<(), StateError> {
    if std::env::var("RATMAC_TEST_LOCK_FAULT").ok().as_deref() == Some(domain) {
        return Err(StateError::new(format!(
            "injected lock fault after acquiring {domain} lock {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(not(feature = "test-fault-injection"))]
fn inject_lock_fault(_domain: &str, _path: &Path) -> Result<(), StateError> {
    Ok(())
}

#[cfg(feature = "test-fault-injection")]
const MAX_TEST_HOLD_TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(feature = "test-fault-injection")]
fn test_hold_timeout() -> Result<Duration, StateError> {
    let Some(value) = std::env::var("RATMAC_TEST_LOCK_HOLD_TIMEOUT_MILLIS").ok() else {
        return Ok(WAIT_TIMEOUT);
    };
    let milliseconds = value.parse::<u64>().map_err(|error| {
        StateError::new(format!(
            "RATMAC_TEST_LOCK_HOLD_TIMEOUT_MILLIS must be a positive integer: {error}"
        ))
    })?;
    if milliseconds == 0 {
        return Err(StateError::new(
            "RATMAC_TEST_LOCK_HOLD_TIMEOUT_MILLIS must be a positive integer",
        ));
    }
    Ok(Duration::from_millis(milliseconds).min(MAX_TEST_HOLD_TIMEOUT))
}

#[cfg(feature = "test-fault-injection")]
fn hold_lock_if_requested(domain: &str, path: &Path) -> Result<(), StateError> {
    if std::env::var("RATMAC_TEST_LOCK_HOLD").ok().as_deref() != Some(domain) {
        return Ok(());
    }
    let marker = std::env::var_os("RATMAC_TEST_LOCK_MARKER")
        .map(PathBuf::from)
        .ok_or_else(|| StateError::new("RATMAC_TEST_LOCK_HOLD needs RATMAC_TEST_LOCK_MARKER"))?;
    let release = std::env::var_os("RATMAC_TEST_LOCK_RELEASE")
        .map(PathBuf::from)
        .ok_or_else(|| StateError::new("RATMAC_TEST_LOCK_HOLD needs RATMAC_TEST_LOCK_RELEASE"))?;
    fs::write(
        &marker,
        format!("holding {domain} lock {}\n", path.display()),
    )
    .map_err(|error| {
        StateError::new(format!(
            "write lock hold marker {}: {error}",
            marker.display()
        ))
    })?;

    let deadline = Instant::now() + test_hold_timeout()?;
    loop {
        if release.is_file() {
            return Ok(());
        }
        let now = Instant::now();
        if now >= deadline {
            return Err(StateError::new(format!(
                "injected lock hold expired for {domain} lock {}",
                path.display()
            )));
        }
        thread::sleep(RETRY_INTERVAL.min(deadline.saturating_duration_since(now)));
    }
}

#[cfg(not(feature = "test-fault-injection"))]
fn hold_lock_if_requested(_domain: &str, _path: &Path) -> Result<(), StateError> {
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LockRetirement {
    path: PathBuf,
    bytes: Vec<u8>,
}

impl LockRetirement {
    /// Re-check a planned retirement before any unrelated retirement write.
    pub(crate) fn verify(&self) -> Result<(), StateError> {
        let bytes = fs::read(&self.path).map_err(|error| {
            StateError::new(format!(
                "refusing to retire lock {}: ownership cannot be rechecked: {error}",
                self.path.display()
            ))
        })?;
        if bytes == self.bytes {
            Ok(())
        } else {
            Err(StateError::new(format!(
                "refusing to retire lock {}: its owner changed after the abandon request was confirmed",
                self.path.display()
            )))
        }
    }

    /// Retire the exact stale lock checked during planning.
    pub(crate) fn retire(&self) -> Result<(), StateError> {
        self.verify()?;
        fs::remove_file(&self.path).map_err(|error| {
            StateError::new(format!("retire lock {}: {error}", self.path.display()))
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

/// Prepare retirement of a stale root lock.  A live or unverifiable owner
/// refuses rather than allowing abandonment to remove another process's lock.
pub(crate) fn stale_root_retirement(
    engine_root: &Path,
) -> Result<Option<LockRetirement>, StateError> {
    stale_retirement(root_path(engine_root), "root")
}

/// Prepare retirement of a stale addressed-Run lock.
pub(crate) fn stale_run_retirement(
    engine_root: &Path,
    run_id: &str,
) -> Result<Option<LockRetirement>, StateError> {
    stale_retirement(run_path(engine_root, run_id), &format!("run:{run_id}"))
}

fn stale_retirement(
    path: PathBuf,
    expected_guard: &str,
) -> Result<Option<LockRetirement>, StateError> {
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(StateError::new(format!(
                "inspect lock {} for retirement: {error}",
                path.display()
            )));
        }
    };

    let owner = OwnerToken::parse(&bytes).ok_or_else(|| {
        StateError::new(format!(
            "refusing to retire lock {}: owner token is absent or malformed",
            path.display()
        ))
    })?;
    if owner.guard != expected_guard {
        return Err(StateError::new(format!(
            "refusing to retire lock {}: owner token guards {:?}, expected {:?}",
            path.display(),
            owner.guard,
            expected_guard
        )));
    }
    match owner_process_state(owner.pid) {
        OwnerProcessState::Dead => {}
        OwnerProcessState::Alive => {
            return Err(StateError::new(format!(
                "refusing to retire lock {}: process {} still owns guard {:?}",
                path.display(),
                owner.pid,
                owner.guard
            )));
        }
        OwnerProcessState::Unknown => {
            return Err(StateError::new(format!(
                "refusing to retire lock {}: cannot verify whether process {} still owns guard {:?}",
                path.display(),
                owner.pid,
                owner.guard
            )));
        }
    }

    Ok(Some(LockRetirement { path, bytes }))
}

#[derive(Debug)]
struct OwnerToken {
    pid: u32,
    guard: String,
}

impl OwnerToken {
    fn parse(bytes: &[u8]) -> Option<Self> {
        let source = std::str::from_utf8(bytes).ok()?;
        let mut lines = source.lines();
        if lines.next()? != "ratmac-lock-v1" {
            return None;
        }
        let pid = lines.next()?.strip_prefix("pid=")?.parse().ok()?;
        let guard = lines.next()?.strip_prefix("guard=")?.to_owned();
        let _nonce: u64 = lines.next()?.strip_prefix("nonce=")?.parse().ok()?;
        if lines.next().is_some() || guard.is_empty() {
            return None;
        }
        Some(Self { pid, guard })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OwnerProcessState {
    Alive,
    Dead,
    Unknown,
}

#[cfg(windows)]
fn owner_process_state(pid: u32) -> OwnerProcessState {
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_INVALID_PARAMETER, STILL_ACTIVE,
    };
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    };

    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            0,
            pid,
        )
    };
    if handle.is_null() {
        return if unsafe { GetLastError() } == ERROR_INVALID_PARAMETER {
            OwnerProcessState::Dead
        } else {
            OwnerProcessState::Unknown
        };
    }
    let mut exit_code = 0;
    let readable = unsafe { GetExitCodeProcess(handle, &mut exit_code) } != 0;
    let _ = unsafe { CloseHandle(handle) };
    if !readable {
        OwnerProcessState::Unknown
    } else if exit_code == STILL_ACTIVE as u32 {
        OwnerProcessState::Alive
    } else {
        OwnerProcessState::Dead
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn owner_process_state(pid: u32) -> OwnerProcessState {
    match fs::metadata(Path::new("/proc").join(pid.to_string())) {
        Ok(_) => OwnerProcessState::Alive,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => OwnerProcessState::Dead,
        Err(_) => OwnerProcessState::Unknown,
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "android")))]
fn owner_process_state(_pid: u32) -> OwnerProcessState {
    // A platform without a reliable standard process probe must fail closed:
    // confirmed abandonment may retire legacy un-tokened residue but never a
    // tokened lock whose owner cannot be checked.
    OwnerProcessState::Unknown
}
