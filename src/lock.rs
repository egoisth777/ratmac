//! Engine lock domains.
//!
//! The short root lock protects minting and shared roster, ledger, or ticket
//! mutation; a Run lock protects motion of one addressed Run. Each guard keeps
//! its lock kernel advisory claim for its entire lifetime. Windows claims one
//! byte at offset 1,073,741,824, beyond the diagnostic token at byte zero; Unix
//! uses an open-file-description `flock`. Both keep the token readable while
//! the claim remains exclusive. The handle's live kernel claim and its identity
//! still matching the canonical pathname are required before a guard mutates a
//! protected resource or removes its pathname. The token is diagnostic text
//! for wait refusals, not an ownership decision.
//!
//! Every mutation rechecks that the claimed handle still identifies the
//! canonical pathname. On Unix, `unlink` itself names only a pathname, so a
//! replacement between that check and unlink cannot be made atomic with this
//! API. The check narrows that window; an actor able to exploit it already has
//! write access to `locks/` and can delete a lock outright. The next acquirer
//! creates and claims the canonical pathname afresh. Lock components are also
//! checked for links and Windows reparse points before opening them. That is a
//! deliberate accidental-link guard, not a guarantee against a determined
//! local actor racing directory replacement.

use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;

use crate::state::StateError;

/// The fixed upper bound for waiting on a contended Engine lock.
pub const WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const RETRY_INTERVAL: Duration = Duration::from_millis(10);
/// The Windows byte-range claim begins well beyond the diagnostic token.
#[cfg(windows)]
const CLAIM_OFFSET: u64 = 1_073_741_824;
#[cfg(windows)]
const CLAIM_LENGTH: u32 = 1;
static TOKEN_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// The one root-domain lock path for an Engine root.
pub fn root_path(engine_root: &Path) -> PathBuf {
    engine_root.join("locks").join("root.lock")
}

/// The one motion-lock path for an addressed Run under an Engine root.
///
/// This public helper exists for diagnostics and test fixtures. Acquisition
/// validates the same id before it can touch this path.
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
    /// Acquire the root lock for minting or shared roster, ledger, or ticket
    /// mutation.
    pub fn acquire(engine_root: &Path) -> Result<Self, StateError> {
        acquire_until(
            engine_root,
            root_path(engine_root),
            "root",
            "root",
            Instant::now() + WAIT_TIMEOUT,
        )
        .map(Self)
    }

    /// The exact root-lock path this guard owns.
    pub fn path(&self) -> &Path {
        self.0.path()
    }

    /// Revalidate this kernel claim at a shared-mutation boundary.
    pub(crate) fn ensure_current(&self) -> Result<(), StateError> {
        self.0.ensure_current()
    }
}

/// A held motion lock for one Run.
#[derive(Debug)]
pub struct RunLock(OwnedLock);

impl RunLock {
    /// Acquire the lock for motion on a canonical minted `run_id`.
    pub fn acquire(engine_root: &Path, run_id: &str) -> Result<Self, StateError> {
        validate_run_id(run_id)?;
        acquire_until(
            engine_root,
            run_path(engine_root, run_id),
            &format!("run:{run_id}"),
            "run",
            Instant::now() + WAIT_TIMEOUT,
        )
        .map(Self)
    }

    /// Try once to acquire the addressed Run lock without waiting.
    ///
    /// `None` means another kernel lock holder currently owns this Run.
    pub(crate) fn try_acquire(
        engine_root: &Path,
        run_id: &str,
    ) -> Result<Option<Self>, StateError> {
        validate_run_id(run_id)?;
        try_acquire_once(
            engine_root,
            run_path(engine_root, run_id),
            &format!("run:{run_id}"),
            "run",
        )
        .map(|lock| lock.map(Self))
    }

    /// The exact Run-lock path this guard owns.
    pub fn path(&self) -> &Path {
        self.0.path()
    }

    /// Revalidate this kernel claim at a Run-mutation boundary.
    pub(crate) fn ensure_current(&self) -> Result<(), StateError> {
        self.0.ensure_current()
    }
}

/// Acquire root then Run with one monotonic deadline.
///
/// The first attempt proves the required root-before-Run ordering. If its Run
/// claim is contended, root is released and later retries first wait
/// non-owningly for that Run's kernel claim to clear. This avoids repeatedly
/// parking unrelated root work behind a still-busy Run.
pub(crate) fn acquire_root_then_run(
    engine_root: &Path,
    run_id: &str,
) -> Result<(RootLock, RunLock), StateError> {
    validate_run_id(run_id)?;
    let deadline = Instant::now() + WAIT_TIMEOUT;
    let addressed_path = run_path(engine_root, run_id);
    let mut wait_for_run_before_retry = false;
    loop {
        if wait_for_run_before_retry {
            wait_for_run_availability(&addressed_path, deadline)?;
        }
        let root = RootLock(acquire_until(
            engine_root,
            root_path(engine_root),
            "root",
            "root",
            deadline,
        )?);
        // Do not start a fresh Run attempt after the root attempt exhausted
        // the deadline. This refusal names the root path just waited on.
        if Instant::now() >= deadline {
            let root_path = root.path().to_path_buf();
            drop(root);
            return Err(wait_expired(&root_path));
        }
        match RunLock::try_acquire(engine_root, run_id)? {
            // This attempt began before the deadline, so it may complete just
            // afterward without starting another retry.
            Some(run) => return Ok((root, run)),
            None => {
                let expired = Instant::now() >= deadline;
                drop(root);
                // Check after the Run attempt as well: waiting ended on this
                // exact path, not on the root path released above.
                if expired {
                    return Err(wait_expired(&addressed_path));
                }
                wait_for_run_before_retry = true;
            }
        }
    }
}
#[derive(Debug)]
struct OwnedLock {
    path: PathBuf,
    file: Option<File>,
}

impl OwnedLock {
    fn path(&self) -> &Path {
        &self.path
    }

    /// A pathname is not an ownership proof after another actor has replaced
    /// it. The held kernel claim is authoritative only while this handle still
    /// identifies the canonical pathname.
    fn ensure_current(&self) -> Result<(), StateError> {
        let file = self.file.as_ref().ok_or_else(|| {
            StateError::new(format!("lock guard lost its file: {}", self.path.display()))
        })?;
        ensure_claim_is_current(file, &self.path)
    }
}

impl Drop for OwnedLock {
    fn drop(&mut self) {
        let Some(file) = self.file.take() else {
            return;
        };

        // QA-only release seam: with RATMAC_TEST_LOCK_HOLD=release this
        // marker is published before either unlink or kernel unlock. A
        // contender must therefore remain excluded until this guard removes
        // its pathname and releases the claim.
        if let Err(error) = hold_lock_if_requested("release", &self.path) {
            eprintln!(
                "warning: release-boundary hold for lock {} failed: {error}",
                self.path.display()
            );
        }

        // The held kernel claim and this handle's identity are the ownership
        // boundary. While the claim is held, no ordinary Engine acquirer can
        // claim this file; do not consult the diagnostic token for cleanup.
        match same_file_at_path(&file, &self.path) {
            Ok(true) => {
                if let Err(error) = remove_owned_lock(&file, &self.path) {
                    eprintln!(
                        "warning: released lock {} but could not remove its residue: {error}",
                        self.path.display()
                    );
                }
            }
            Ok(false) => eprintln!(
                "warning: released lock {} but did not remove it because the pathname now names another file",
                self.path.display()
            ),
            Err(error) => eprintln!(
                "warning: released lock {} but could not revalidate its pathname before cleanup: {error}",
                self.path.display()
            ),
        }
        if let Err(error) = unlock_file(&file) {
            eprintln!(
                "warning: released lock {} but kernel unlock failed: {error}; closing its handle may release it",
                self.path.display()
            );
        }
        drop(file);
    }
}

/// Attempt only while the deadline admits a new attempt. A nonblocking
/// attempt begun before expiry may finish just afterward, but no retry starts
/// after that bound.
fn acquire_until(
    engine_root: &Path,
    path: PathBuf,
    guard: &str,
    fault_domain: &str,
    deadline: Instant,
) -> Result<OwnedLock, StateError> {
    loop {
        if Instant::now() >= deadline {
            return Err(wait_expired(&path));
        }
        if let Some(lock) = try_acquire_once(engine_root, path.clone(), guard, fault_domain)? {
            return Ok(lock);
        }
        wait_for_retry(&path, deadline)?;
    }
}

fn try_acquire_once(
    engine_root: &Path,
    path: PathBuf,
    guard: &str,
    fault_domain: &str,
) -> Result<Option<OwnedLock>, StateError> {
    prepare_lock_path(engine_root, &path)?;
    refuse_legacy(engine_root)?;

    let mut file = match open_lock_file_checked(&path) {
        Ok(Some(file)) => file,
        Ok(None) => return Ok(None),
        Err(error) => {
            return Err(StateError::new(format!(
                "cannot open lock {}: {error}",
                path.display()
            )))
        }
    };
    match lock_file(&file) {
        Ok(()) => {}
        Err(error) if lock_is_contended(&error) => return Ok(None),
        Err(error) => {
            return Err(StateError::new(format!(
                "lock claim failed for {}: {error}",
                path.display()
            )))
        }
    }
    if let Err(error) = ensure_claim_is_current(&file, &path) {
        unlock_after_failed_claim(&file, &path);
        return Err(error);
    }

    // Write the diagnostic token only after the handle has claimed the
    // canonical pathname. The live kernel claim and handle identity are
    // rechecked before later mutations or cleanup.
    if let Err(error) = write_owner_token(&mut file, guard) {
        unlock_after_failed_claim(&file, &path);
        return Err(StateError::new(format!(
            "write owner token for lock {}: {error}",
            path.display()
        )));
    }
    let lock = OwnedLock {
        path: path.clone(),
        file: Some(file),
    };
    if let Err(error) = refuse_legacy(engine_root) {
        drop(lock);
        return Err(error);
    }
    inject_lock_fault(fault_domain, &path)?;
    hold_lock_if_requested(fault_domain, &path)?;
    lock.ensure_current()?;
    Ok(Some(lock))
}

fn wait_for_retry(path: &Path, deadline: Instant) -> Result<(), StateError> {
    let now = Instant::now();
    if now >= deadline {
        return Err(wait_expired(path));
    }
    thread::sleep(RETRY_INTERVAL.min(deadline.saturating_duration_since(now)));
    Ok(())
}
/// Wait on the addressed Run after a root-first attempt found it busy. The
/// probe never writes a token and always releases any transient kernel claim
/// before the caller can acquire root again.
fn wait_for_run_availability(path: &Path, deadline: Instant) -> Result<(), StateError> {
    loop {
        if Instant::now() >= deadline {
            return Err(wait_expired(path));
        }
        if !run_lock_is_contended(path)? {
            return Ok(());
        }
        wait_for_retry(path, deadline)?;
    }
}

/// `true` only while an existing Run lock has a live kernel claimant. A
/// missing pathname is available; the ordered root-then-Run attempt will
/// create it if it remains absent.
fn run_lock_is_contended(path: &Path) -> Result<bool, StateError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => reject_link_or_reparse(path, &metadata)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(StateError::new(format!(
                "cannot inspect lock {} while waiting: {error}",
                path.display()
            )))
        }
    }
    let file = match open_existing_lock_probe(path) {
        Ok(Some(file)) => file,
        Ok(None) => return Ok(false),
        Err(error) => {
            return Err(StateError::new(format!(
                "cannot probe lock {} while waiting: {error}",
                path.display()
            )))
        }
    };
    match lock_file(&file) {
        Ok(()) => {
            unlock_file(&file).map_err(|error| {
                StateError::new(format!(
                    "cannot release availability probe for lock {}: {error}",
                    path.display()
                ))
            })?;
            Ok(false)
        }
        Err(error) if lock_is_contended(&error) => Ok(true),
        Err(error) => Err(StateError::new(format!(
            "lock availability probe failed for {}: {error}",
            path.display()
        ))),
    }
}

fn wait_expired(path: &Path) -> StateError {
    let diagnostic = match fs::read(path) {
        Ok(bytes) if bytes.is_empty() => "empty owner token".to_owned(),
        Ok(bytes) => {
            let mut token = String::from_utf8_lossy(&bytes)
                .replace(['\r', '\n'], "; ")
                .trim()
                .to_owned();
            token.truncate(256);
            format!("owner token: {token}")
        }
        Err(error) => format!("owner token unavailable: {error}"),
    };
    StateError::new(format!(
        "lock wait expired: {}; {diagnostic}",
        path.display()
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenFailure {
    /// Another live holder's kernel claim or Windows share denies the open.
    Contended,
    /// A Windows holder may have released between the original open and a
    /// non-claiming probe; retry the primary open once before deciding.
    RetryPrimary,
    /// A real filesystem or permission error, not a lock refusal.
    NotContended,
}

fn open_lock_file_checked(path: &Path) -> std::io::Result<Option<File>> {
    match open_lock_file(path) {
        Ok(file) => Ok(Some(file)),
        Err(original) => match classify_open_failure(path, &original) {
            OpenFailure::Contended => Ok(None),
            OpenFailure::RetryPrimary => match open_lock_file(path) {
                Ok(file) => Ok(Some(file)),
                Err(retry) => match classify_open_failure(path, &retry) {
                    // A holder may arrive after the first probe succeeds.
                    // Classify that new failure rather than surfacing the
                    // obsolete original one.
                    OpenFailure::Contended => Ok(None),
                    // One retry is intentional. A second release race or a
                    // real filesystem failure remains a named open failure.
                    OpenFailure::RetryPrimary | OpenFailure::NotContended => Err(retry),
                },
            },
            OpenFailure::NotContended => Err(original),
        },
    }
}

#[cfg(not(windows))]
fn classify_open_failure(_path: &Path, error: &std::io::Error) -> OpenFailure {
    if lock_is_contended(error) {
        OpenFailure::Contended
    } else {
        OpenFailure::NotContended
    }
}

#[cfg(windows)]
fn classify_open_failure(path: &Path, error: &std::io::Error) -> OpenFailure {
    const ERROR_ACCESS_DENIED: i32 = 5;
    const ERROR_SHARING_VIOLATION: i32 = 32;
    let sharing_violation = match error.raw_os_error() {
        Some(ERROR_SHARING_VIOLATION) => true,
        Some(ERROR_ACCESS_DENIED) => false,
        _ => return OpenFailure::NotContended,
    };

    // This deliberately weak open asks for no write or delete access. If it
    // succeeds, a non-blocking shared LockFileEx claim distinguishes a live
    // exclusive holder from an unrelated ACL or directory failure. A holder
    // using this Engine's primary share mode permits this diagnostic open; if
    // the probe itself cannot open after ERROR_SHARING_VIOLATION, that is
    // ambiguous and safely remains contention (possibly a foreign holder).
    // A failed probe after ERROR_ACCESS_DENIED remains the original named
    // filesystem or ACL refusal. A holder can release in the small interval,
    // so a successful probe grants exactly one retry of the primary open.
    let probe = match open_lock_file_without_delete(path) {
        Ok(file) => file,
        Err(_) if sharing_violation => return OpenFailure::Contended,
        Err(_) => return OpenFailure::NotContended,
    };
    let result = match lock_file_shared(&probe) {
        Ok(()) => OpenFailure::RetryPrimary,
        Err(probe_error) if lock_is_contended(&probe_error) => OpenFailure::Contended,
        Err(_) => OpenFailure::NotContended,
    };
    if matches!(result, OpenFailure::RetryPrimary) {
        if let Err(unlock_error) = unlock_file(&probe) {
            eprintln!(
                "warning: Windows shared lock probe on {} could not unlock: {unlock_error}; closing its handle may release it",
                path.display()
            );
        }
    }
    drop(probe);
    result
}

fn prepare_lock_path(engine_root: &Path, path: &Path) -> Result<(), StateError> {
    inspect_lock_directory(engine_root, "Engine root")?;
    let locks = engine_root.join("locks");
    let runs = locks.join("runs");
    if path == root_path(engine_root) {
        ensure_lock_directory(&locks)?;
    } else if path.parent() == Some(runs.as_path()) {
        ensure_lock_directory(&locks)?;
        ensure_lock_directory(&runs)?;
    } else {
        return Err(StateError::new(format!(
            "refusing non-canonical lock path {}; it was not modified",
            path.display()
        )));
    }
    inspect_lock_path_or_absent(path)
}

fn ensure_claim_is_current(file: &File, path: &Path) -> Result<(), StateError> {
    inspect_claim_components(path)?;
    match same_file_at_path(file, path) {
        Ok(true) => Ok(()),
        Ok(false) => Err(StateError::new(format!(
            "lock path was replaced while held: {}; refusing to mutate; nothing was modified",
            path.display()
        ))),
        Err(error) => Err(StateError::new(format!(
            "revalidate lock {} before mutation: {error}",
            path.display()
        ))),
    }
}

fn inspect_claim_components(path: &Path) -> Result<(), StateError> {
    let runs = path.parent().filter(|parent| {
        parent
            .file_name()
            .is_some_and(|name| name == std::ffi::OsStr::new("runs"))
    });
    let locks = runs
        .and_then(Path::parent)
        .or_else(|| path.parent())
        .ok_or_else(|| StateError::new(format!("lock path has no parent: {}", path.display())))?;
    let engine_root = locks.parent().ok_or_else(|| {
        StateError::new(format!(
            "lock directory has no Engine root: {}",
            locks.display()
        ))
    })?;
    inspect_lock_directory(engine_root, "Engine root")?;
    inspect_lock_directory(locks, "lock directory")?;
    if let Some(runs) = runs {
        inspect_lock_directory(runs, "Run-lock directory")?;
    }
    inspect_lock_leaf(path)
}

fn ensure_lock_directory(path: &Path) -> Result<(), StateError> {
    match fs::symlink_metadata(path) {
        Ok(_) => inspect_lock_directory(path, "lock directory"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match fs::create_dir(path) {
                Ok(()) => {}
                Err(create_error) if create_error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(create_error) => {
                    return Err(StateError::new(format!(
                        "create lock directory {}: {create_error}",
                        path.display()
                    )))
                }
            }
            inspect_lock_directory(path, "lock directory")
        }
        Err(error) => Err(StateError::new(format!(
            "inspect lock directory {}: {error}",
            path.display()
        ))),
    }
}

fn inspect_lock_path_or_absent(path: &Path) -> Result<(), StateError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => reject_link_or_reparse(path, &metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(StateError::new(format!(
            "inspect lock path {}: {error}",
            path.display()
        ))),
    }
}

fn inspect_lock_leaf(path: &Path) -> Result<(), StateError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        StateError::new(format!(
            "revalidate lock {} before mutation: {error}",
            path.display()
        ))
    })?;
    reject_link_or_reparse(path, &metadata)
}

fn inspect_lock_directory(path: &Path, kind: &str) -> Result<(), StateError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| StateError::new(format!("inspect {kind} {}: {error}", path.display())))?;
    reject_link_or_reparse(path, &metadata)?;
    if metadata.is_dir() {
        Ok(())
    } else {
        Err(StateError::new(format!(
            "refusing {kind} {}: it is not a directory; it was not modified",
            path.display()
        )))
    }
}

fn reject_link_or_reparse(path: &Path, metadata: &fs::Metadata) -> Result<(), StateError> {
    if is_link_or_reparse(metadata) {
        Err(StateError::new(format!(
            "refusing lock path {}: symbolic link or reparse point; it was not modified",
            path.display()
        )))
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

fn unlock_after_failed_claim(file: &File, path: &Path) {
    if let Err(error) = unlock_file(file) {
        eprintln!(
            "warning: failed acquisition could not unlock {}: {error}; closing its handle may release it",
            path.display()
        );
    }
}

fn open_lock_file(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};

        // A live holder must not permit a different handle to rename or delete
        // the canonical path underneath its LockFileEx claim.
        const GENERIC_READ: u32 = 0x8000_0000;
        const GENERIC_WRITE: u32 = 0x4000_0000;
        const DELETE: u32 = 0x0001_0000;
        options
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .access_mode(GENERIC_READ | GENERIC_WRITE | DELETE);
    }
    options.open(path)
}

/// Open an existing lock for a non-owning availability probe. Unlike a real
/// acquisition it never creates a pathname or asks for delete access.
fn open_existing_lock_probe(path: &Path) -> std::io::Result<Option<File>> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        };

        const GENERIC_READ: u32 = 0x8000_0000;
        const GENERIC_WRITE: u32 = 0x4000_0000;
        options
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .access_mode(GENERIC_READ | GENERIC_WRITE);
    }
    match options.open(path) {
        Ok(file) => Ok(Some(file)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(windows)]
fn open_lock_file_without_delete(path: &Path) -> std::io::Result<File> {
    // Deliberately make this probe as weak as possible. Rust's ordinary
    // read-only open shares read, write, and delete; a holder that denies
    // DELETE to a would-be owner still permits this diagnostic open, whereas
    // a directory or denying ACL does not. This is not a lock claim.
    OpenOptions::new().read(true).open(path)
}

fn write_owner_token(file: &mut File, guard: &str) -> std::io::Result<()> {
    let sequence = TOKEN_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let token = format!(
        "ratmac-lock-v1\npid={}\nguard={guard}\nnonce={sequence}\n",
        std::process::id()
    )
    .into_bytes();
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&token)?;
    file.sync_all()
}

fn validate_run_id(run_id: &str) -> Result<(), StateError> {
    let digits = run_id.strip_prefix("run-").unwrap_or_default();
    let canonical = !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && digits
            .parse::<u64>()
            .ok()
            .is_some_and(|ordinal| ordinal != 0 && format!("run-{ordinal:03}") == run_id);
    if canonical {
        Ok(())
    } else {
        Err(StateError::new(format!(
            "refusing Run lock for non-canonical run id {run_id:?}"
        )))
    }
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
/// Feature-gated QA lock barriers. `root` and `run` pause immediately after
/// acquisition; `release` pauses at the start of `Drop`, while the canonical
/// pathname and kernel claim are both still live. They use
/// `RATMAC_TEST_LOCK_HOLD`, `RATMAC_TEST_LOCK_MARKER`,
/// `RATMAC_TEST_LOCK_RELEASE`, and the optional
/// `RATMAC_TEST_LOCK_HOLD_TIMEOUT_MILLIS`. This function is compiled only
/// with `test-fault-injection`, which the QA binary enables; production builds
/// use the no-op stub below and expose none of these barriers.
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

#[cfg(unix)]
#[link(name = "c")]
unsafe extern "C" {
    fn flock(fd: std::os::raw::c_int, operation: std::os::raw::c_int) -> std::os::raw::c_int;
}

#[cfg(unix)]
fn lock_file(file: &File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;

    const LOCK_EX: std::os::raw::c_int = 2;
    const LOCK_NB: std::os::raw::c_int = 4;
    // SAFETY: `file` is a live descriptor and flock does not retain it. The
    // claim belongs to this open file description, not to the whole process.
    if unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn unlock_file(file: &File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;

    const LOCK_UN: std::os::raw::c_int = 8;
    // SAFETY: `file` is a live descriptor and flock does not retain it.
    if unsafe { flock(file.as_raw_fd(), LOCK_UN) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn lock_is_contended(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::WouldBlock
}

#[cfg(unix)]
fn same_file_at_path(file: &File, path: &Path) -> std::io::Result<bool> {
    use std::os::unix::fs::MetadataExt;

    let held = file.metadata()?;
    match fs::metadata(path) {
        Ok(named) => Ok(held.dev() == named.dev() && held.ino() == named.ino()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(unix)]
fn remove_owned_lock(_file: &File, path: &Path) -> std::io::Result<()> {
    fs::remove_file(path)
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct FileTime {
    low_date_time: u32,
    high_date_time: u32,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct ByHandleFileInformation {
    file_attributes: u32,
    creation_time: FileTime,
    last_access_time: FileTime,
    last_write_time: FileTime,
    volume_serial_number: u32,
    file_size_high: u32,
    file_size_low: u32,
    number_of_links: u32,
    file_index_high: u32,
    file_index_low: u32,
}

#[cfg(windows)]
#[repr(C)]
struct Overlapped {
    internal: usize,
    internal_high: usize,
    offset: u32,
    offset_high: u32,
    event: *mut std::ffi::c_void,
}

#[cfg(windows)]
#[repr(C)]
struct FileDispositionInfo {
    delete_file: u8,
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    #[link_name = "LockFileEx"]
    fn lock_file_ex(
        file: *mut std::ffi::c_void,
        flags: u32,
        reserved: u32,
        bytes_low: u32,
        bytes_high: u32,
        overlapped: *mut Overlapped,
    ) -> i32;
    #[link_name = "UnlockFileEx"]
    fn unlock_file_ex(
        file: *mut std::ffi::c_void,
        reserved: u32,
        bytes_low: u32,
        bytes_high: u32,
        overlapped: *mut Overlapped,
    ) -> i32;
    #[link_name = "GetFileInformationByHandle"]
    fn get_file_information_by_handle(
        file: *mut std::ffi::c_void,
        information: *mut ByHandleFileInformation,
    ) -> i32;
    #[link_name = "SetFileInformationByHandle"]
    fn set_file_information_by_handle(
        file: *mut std::ffi::c_void,
        information_class: i32,
        information: *mut std::ffi::c_void,
        information_size: u32,
    ) -> i32;
}

#[cfg(windows)]
fn claim_overlapped() -> Overlapped {
    Overlapped {
        internal: 0,
        internal_high: 0,
        offset: CLAIM_OFFSET as u32,
        offset_high: (CLAIM_OFFSET >> 32) as u32,
        event: std::ptr::null_mut(),
    }
}

#[cfg(windows)]
fn lock_file(file: &File) -> std::io::Result<()> {
    use std::os::windows::io::AsRawHandle;

    const LOCKFILE_FAIL_IMMEDIATELY: u32 = 0x0000_0001;
    const LOCKFILE_EXCLUSIVE_LOCK: u32 = 0x0000_0002;
    let mut overlapped = claim_overlapped();
    // SAFETY: the live file handle, one-byte high-offset range, and
    // initialized OVERLAPPED are valid for this synchronous non-blocking call.
    if unsafe {
        lock_file_ex(
            file.as_raw_handle(),
            LOCKFILE_FAIL_IMMEDIATELY | LOCKFILE_EXCLUSIVE_LOCK,
            0,
            CLAIM_LENGTH,
            0,
            &mut overlapped,
        )
    } != 0
    {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn lock_file_shared(file: &File) -> std::io::Result<()> {
    use std::os::windows::io::AsRawHandle;

    const LOCKFILE_FAIL_IMMEDIATELY: u32 = 0x0000_0001;
    let mut overlapped = claim_overlapped();
    // SAFETY: this is the same one-byte range as the primary claim, but
    // without EXCLUSIVE_LOCK, solely to classify a weak-open failure.
    if unsafe {
        lock_file_ex(
            file.as_raw_handle(),
            LOCKFILE_FAIL_IMMEDIATELY,
            0,
            CLAIM_LENGTH,
            0,
            &mut overlapped,
        )
    } != 0
    {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn unlock_file(file: &File) -> std::io::Result<()> {
    use std::os::windows::io::AsRawHandle;

    let mut overlapped = claim_overlapped();
    // SAFETY: the live file handle and high-offset range match the successful
    // LockFileEx claim made by this guard.
    if unsafe { unlock_file_ex(file.as_raw_handle(), 0, CLAIM_LENGTH, 0, &mut overlapped) } != 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn lock_is_contended(error: &std::io::Error) -> bool {
    const ERROR_LOCK_VIOLATION: i32 = 33;
    error.kind() == std::io::ErrorKind::WouldBlock
        || error.raw_os_error() == Some(ERROR_LOCK_VIOLATION)
}

#[cfg(windows)]
fn same_file_at_path(file: &File, path: &Path) -> std::io::Result<bool> {
    let held = file_information(file)?;
    let named = match identity_open(path) {
        Ok(file) => file_information(&file)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    Ok(held.volume_serial_number == named.volume_serial_number
        && held.file_index_high == named.file_index_high
        && held.file_index_low == named.file_index_low)
}

#[cfg(windows)]
fn identity_open(path: &Path) -> std::io::Result<File> {
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    let mut options = OpenOptions::new();
    // The primary claimant denies DELETE for its lifetime. This short,
    // read-only self-revalidation must nevertheless *share* that claimant's
    // DELETE access; otherwise Windows rejects our own fresh open with a
    // sharing violation before we can compare file identities. It does not
    // grant a third party deletion because the primary handle still omits
    // FILE_SHARE_DELETE.
    options
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE);
    options.open(path)
}

#[cfg(windows)]
fn file_information(file: &File) -> std::io::Result<ByHandleFileInformation> {
    use std::os::windows::io::AsRawHandle;

    let mut information = ByHandleFileInformation::default();
    // SAFETY: `file` is a live handle and `information` points to valid,
    // writable storage for the duration of this synchronous call.
    if unsafe { get_file_information_by_handle(file.as_raw_handle(), &mut information) } != 0 {
        Ok(information)
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn remove_owned_lock(file: &File, _path: &Path) -> std::io::Result<()> {
    use std::os::windows::io::AsRawHandle;

    const FILE_DISPOSITION_INFO: i32 = 4;
    let mut disposition = FileDispositionInfo { delete_file: 1 };
    // SAFETY: the handle was opened with DELETE access, and this initialized
    // FILE_DISPOSITION_INFO is valid for the synchronous call. The handle
    // remains under this guard's LockFileEx claim until Drop unlocks it.
    if unsafe {
        set_file_information_by_handle(
            file.as_raw_handle(),
            FILE_DISPOSITION_INFO,
            (&mut disposition as *mut FileDispositionInfo).cast(),
            std::mem::size_of::<FileDispositionInfo>() as u32,
        )
    } != 0
    {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(any(unix, windows)))]
fn lock_file(_file: &File) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "kernel advisory locks are unsupported on this platform",
    ))
}

#[cfg(not(any(unix, windows)))]
fn unlock_file(_file: &File) -> std::io::Result<()> {
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn lock_is_contended(_error: &std::io::Error) -> bool {
    false
}

#[cfg(not(any(unix, windows)))]
fn same_file_at_path(_file: &File, _path: &Path) -> std::io::Result<bool> {
    Ok(true)
}

#[cfg(not(any(unix, windows)))]
fn remove_owned_lock(_file: &File, path: &Path) -> std::io::Result<()> {
    fs::remove_file(path)
}
