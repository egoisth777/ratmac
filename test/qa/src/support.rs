//! Shared private-test support (WCP-002): owned temporary trees, scrubbed
//! child processes, strict filesystem capture, and isolated fault setup.
//!
//! Standard library only. The QA crate exports this file as
//! `ratmac_qa::support`; a private lane crate without that dependency reuses
//! the same bytes through `#[path = "../../../test/qa/src/support.rs"]`, so
//! adopting it never changes a lane's manifest, lockfile, or feature graph.
//!
//! What stays with each lane: its domain fixtures, which executable it runs,
//! its assertions, and the snapshot projection it compares.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

// --- owned temporary trees ---------------------------------------------------

static NEXT_TREE: AtomicU64 = AtomicU64::new(0);

/// A directory under the system temporary directory that this value alone
/// created and owns; it is removed on drop, including while unwinding.
#[derive(Debug)]
pub struct TempTree {
    path: PathBuf,
    keep: bool,
}

impl TempTree {
    /// Create a fresh, exclusively owned directory whose name carries `label`.
    ///
    /// The directory is made with `create_dir`, never `create_dir_all`, so an
    /// existing path is never adopted: a collision retries with a new name.
    pub fn new(label: &str) -> io::Result<Self> {
        let base = std::env::temp_dir();
        let label: String = label
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                    character
                } else {
                    '-'
                }
            })
            .collect();
        loop {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default();
            let path = base.join(format!(
                "ratmac-{label}-{}-{stamp}-{}",
                std::process::id(),
                NEXT_TREE.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path, keep: false }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(io::Error::new(
                        error.kind(),
                        format!("create temporary tree {}: {error}", path.display()),
                    ))
                }
            }
        }
    }

    /// The owned directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// A path inside the owned directory.
    pub fn join(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.path.join(relative)
    }

    /// Write `bytes` at `relative`, creating parent directories.
    pub fn write(&self, relative: impl AsRef<Path>, bytes: impl AsRef<[u8]>) {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("create {}: {error}", parent.display()));
        }
        fs::write(&path, bytes).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    }

    /// Give up ownership: the directory survives this value and the caller
    /// becomes responsible for removing it.
    pub fn persist(mut self) -> PathBuf {
        self.keep = true;
        std::mem::take(&mut self.path)
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

// --- scrubbed child processes ------------------------------------------------

/// The marker an isolated child test reads to recognize its own entry.
const ISOLATED_TEST: &str = "RATMAC_SUPPORT_ISOLATED_TEST";
/// Where that child records entering its body, in a tree its parent owns.
const ISOLATED_ACK: &str = "RATMAC_SUPPORT_ISOLATED_ACK";

/// Cargo variables that describe the parent harness rather than the host.
/// Host configuration (`CARGO_HOME`, `CARGO_TARGET_DIR`, `CARGO_NET_*`, ...)
/// is not in this list and passes through.
const CARGO_HARNESS: [&str; 10] = [
    "CARGO",
    "CARGO_MANIFEST_DIR",
    "CARGO_MANIFEST_PATH",
    "CARGO_MANIFEST_LINKS",
    "CARGO_CRATE_NAME",
    "CARGO_BIN_NAME",
    "CARGO_PRIMARY_PACKAGE",
    "CARGO_TARGET_TMPDIR",
    "CARGO_RUSTC_CURRENT_DIR",
    "CARGO_MAKEFLAGS",
];
const CARGO_HARNESS_PREFIXES: [&str; 4] = [
    "CARGO_PKG_",
    "CARGO_BIN_EXE_",
    "CARGO_CFG_",
    "CARGO_FEATURE_",
];

/// Whether an inherited variable belongs to the parent harness, a Git
/// redirection, or an Engine test hook, and so never reaches a child.
fn inherited_only(key: &OsStr) -> bool {
    let Some(key) = key.to_str() else {
        return false;
    };
    let upper = key.to_ascii_uppercase();
    upper.starts_with("GIT_")
        || upper.starts_with("RATMAC_TEST_")
        || upper.starts_with("RATMAC_SUPPORT_")
        || upper.starts_with("__CARGO_")
        || CARGO_HARNESS.contains(&upper.as_str())
        || CARGO_HARNESS_PREFIXES
            .iter()
            .any(|prefix| upper.starts_with(prefix))
}

/// Remove every inherited variable a child must not see. Values the caller
/// sets on `command` afterwards are explicit and survive.
pub fn scrub(command: &mut Command) -> &mut Command {
    for (key, _) in std::env::vars_os() {
        if inherited_only(&key) {
            command.env_remove(&key);
        }
    }
    command
}

/// A child process that runs `program` in `directory` with a scrubbed
/// environment and no standard input. The parent environment is untouched.
pub fn command(program: impl AsRef<OsStr>, directory: impl AsRef<Path>) -> Command {
    let mut command = Command::new(program);
    scrub(&mut command);
    command.current_dir(directory).stdin(Stdio::null());
    command
}

/// Fixture Git in `directory`: scrubbed like [`command`], and byte-exact line
/// endings and long paths through command-line settings that persist in
/// neither the fixture's nor the host's configuration.
pub fn git(directory: impl AsRef<Path>) -> Command {
    let mut git = command("git", directory);
    git.args(["-c", "core.autocrlf=false", "-c", "core.longpaths=true"]);
    git
}

/// Merged standard output and standard error, lossily decoded.
pub fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

// --- isolated child tests ----------------------------------------------------

/// Run the libtest case `name` of the current test executable alone, in a
/// child process whose environment is scrubbed and then given `envs`.
///
/// Inside that child, the same call returns `Ok(true)` and the caller runs
/// its body. In the parent it returns `Ok(false)` once the child has proven
/// it selected exactly that one case, entered it, and passed; zero selected
/// cases, a missing or foreign entry acknowledgement, or a failed child are
/// an `Err` carrying the child's output.
pub fn isolated_test(name: &str, envs: &[(&str, &OsStr)]) -> Result<bool, String> {
    if std::env::var_os(ISOLATED_TEST).as_deref() == Some(OsStr::new(name)) {
        if let Some(ack) = std::env::var_os(ISOLATED_ACK) {
            fs::write(&ack, name)
                .map_err(|error| format!("acknowledge isolated test {name} at {ack:?}: {error}"))?;
        }
        return Ok(true);
    }
    let executable =
        std::env::current_exe().map_err(|error| format!("locate test executable: {error}"))?;
    let directory =
        std::env::current_dir().map_err(|error| format!("locate working directory: {error}"))?;
    let owned = TempTree::new("isolated-ack")
        .map_err(|error| format!("prepare isolated test {name}: {error}"))?;
    let ack = owned.join("entered");
    let mut child = command(&executable, &directory);
    child
        .args(["--exact", name, "--test-threads", "1"])
        .env(ISOLATED_TEST, name)
        .env(ISOLATED_ACK, &ack);
    for (key, value) in envs {
        child.env(key, value);
    }
    let output = child
        .output()
        .map_err(|error| format!("launch isolated test {name}: {error}"))?;
    let shown = text(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.lines().any(|line| line.trim() == "running 1 test") {
        return Err(format!(
            "isolated test {name} did not select exactly one case:\n{shown}"
        ));
    }
    match fs::read(&ack) {
        Ok(entered) if entered == name.as_bytes() => {}
        Ok(_) => {
            return Err(format!(
                "isolated test {name} acknowledged a different entry:\n{shown}"
            ))
        }
        Err(_) => {
            return Err(format!(
                "isolated test {name} never acknowledged entering its body:\n{shown}"
            ))
        }
    }
    let passed = format!("test {name} ... ok");
    if !output.status.success() || !stdout.lines().any(|line| line.trim() == passed) {
        return Err(format!("isolated test {name} failed:\n{shown}"));
    }
    Ok(false)
}

// --- in-process Engine hook lanes -------------------------------------------

static HOOK_LANES: Mutex<()> = Mutex::new(());

/// Exclusive use of this process's Engine test hooks (`RATMAC_TEST_*`).
///
/// Taking the lane serializes every in-process lane that selects a hook
/// through the environment and hides any inherited hook value for as long
/// as the lane is held; dropping it restores exactly what was inherited.
pub struct HookLane {
    inherited: Vec<(OsString, OsString)>,
    _serial: MutexGuard<'static, ()>,
}

/// Take the process's hook lane, waiting for any other holder.
pub fn hook_lane() -> HookLane {
    let serial = HOOK_LANES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let inherited: Vec<(OsString, OsString)> = std::env::vars_os()
        .filter(|(key, _)| {
            key.to_str()
                .is_some_and(|key| key.to_ascii_uppercase().starts_with("RATMAC_TEST_"))
        })
        .collect();
    for (key, _) in &inherited {
        std::env::remove_var(key);
    }
    HookLane {
        inherited,
        _serial: serial,
    }
}

impl Drop for HookLane {
    fn drop(&mut self) {
        for (key, _) in std::env::vars_os() {
            if key
                .to_str()
                .is_some_and(|key| key.to_ascii_uppercase().starts_with("RATMAC_TEST_"))
            {
                std::env::remove_var(key);
            }
        }
        for (key, value) in &self.inherited {
            std::env::set_var(key, value);
        }
    }
}

/// One Engine fault point selected for a scope; the previous value returns
/// on drop, even while an assertion unwinds. Hold a [`HookLane`] around it.
pub struct StepFault {
    previous: Option<OsString>,
}

impl StepFault {
    /// Select `point` through `RATMAC_TEST_STEP_FAULT`.
    pub fn set(point: &str) -> Self {
        let previous = std::env::var_os("RATMAC_TEST_STEP_FAULT");
        std::env::set_var("RATMAC_TEST_STEP_FAULT", point);
        Self { previous }
    }
}

impl Drop for StepFault {
    fn drop(&mut self) {
        match &self.previous {
            Some(previous) => std::env::set_var("RATMAC_TEST_STEP_FAULT", previous),
            None => std::env::remove_var("RATMAC_TEST_STEP_FAULT"),
        }
    }
}

// --- strict filesystem capture ------------------------------------------------

/// One captured filesystem node. Links are recorded, never followed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Directory,
    File(Vec<u8>),
    Symlink(PathBuf),
    Other,
}

/// Metadata observed beside a node; distinct from its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub len: u64,
    pub modified: Option<SystemTime>,
    pub readonly: bool,
}

/// A typed capture keyed by path relative to the captured root.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub nodes: BTreeMap<PathBuf, Node>,
    pub metadata: BTreeMap<PathBuf, Metadata>,
}

/// What a capture leaves out, all explicit.
#[derive(Clone, Copy, Debug, Default)]
pub struct CaptureOptions<'a> {
    /// Relative paths skipped with everything beneath them.
    pub excluded_paths: &'a [PathBuf],
    /// Entry names skipped at any depth (for example `target`).
    pub excluded_names: &'a [&'a str],
    /// Directory names skipped at any depth with everything beneath them; a
    /// file or link carrying the same name is still captured.
    pub excluded_directory_names: &'a [&'a str],
    /// Read an absent root as an empty snapshot instead of an error.
    pub missing_root_is_empty: bool,
}

/// Capture every node under `root`. Any unreadable input fails naming its
/// path; nothing is silently dropped and nothing under `root` is written.
pub fn capture(root: &Path, options: CaptureOptions<'_>) -> Result<Snapshot, String> {
    match fs::symlink_metadata(root) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => {
            return Err(format!(
                "capture root {} is not a directory",
                root.display()
            ))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound && options.missing_root_is_empty => {
            return Ok(Snapshot::default())
        }
        Err(error) => return Err(format!("capture root {}: {error}", root.display())),
    }
    let mut snapshot = Snapshot::default();
    walk(root, root, &options, &mut snapshot)?;
    Ok(snapshot)
}

fn walk(
    root: &Path,
    directory: &Path,
    options: &CaptureOptions<'_>,
    snapshot: &mut Snapshot,
) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read {}: {error}", directory.display()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .map_err(|error| format!("relate {}: {error}", path.display()))?
            .to_path_buf();
        if options.excluded_paths.contains(&relative)
            || options
                .excluded_names
                .iter()
                .any(|name| entry.file_name() == OsStr::new(name))
        {
            continue;
        }
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("inspect {}: {error}", path.display()))?;
        let kind = metadata.file_type();
        if kind.is_dir()
            && options
                .excluded_directory_names
                .iter()
                .any(|name| entry.file_name() == OsStr::new(name))
        {
            continue;
        }
        let node = if kind.is_symlink() {
            Node::Symlink(
                fs::read_link(&path)
                    .map_err(|error| format!("read link {}: {error}", path.display()))?,
            )
        } else if kind.is_dir() {
            Node::Directory
        } else if kind.is_file() {
            Node::File(
                fs::read(&path).map_err(|error| format!("read {}: {error}", path.display()))?,
            )
        } else {
            Node::Other
        };
        snapshot.metadata.insert(
            relative.clone(),
            Metadata {
                len: metadata.len(),
                modified: metadata.modified().ok(),
                readonly: metadata.permissions().readonly(),
            },
        );
        let descend = node == Node::Directory;
        snapshot.nodes.insert(relative, node);
        if descend {
            walk(root, &path, options, snapshot)?;
        }
    }
    Ok(())
}

impl Snapshot {
    /// This snapshot unchanged, or an error naming its first link or special
    /// file. [`files`](Self::files) and [`directories`](Self::directories)
    /// leave such nodes out, so call this first wherever one must not pass
    /// unseen.
    pub fn refuse_links(self) -> Result<Self, String> {
        for (path, node) in &self.nodes {
            match node {
                Node::Symlink(target) => {
                    return Err(format!(
                        "captured tree holds a link {} -> {}",
                        path.display(),
                        target.display()
                    ))
                }
                Node::Other => {
                    return Err(format!(
                        "captured tree holds a special file {}",
                        path.display()
                    ))
                }
                Node::Directory | Node::File(_) => {}
            }
        }
        Ok(self)
    }

    /// Regular files only, keyed by `/`-separated path.
    pub fn files(&self) -> BTreeMap<String, Vec<u8>> {
        self.nodes
            .iter()
            .filter_map(|(path, node)| match node {
                Node::File(bytes) => Some((shown(path), bytes.clone())),
                _ => None,
            })
            .collect()
    }

    /// Files by path plus every directory as `path/` with no bytes, so an
    /// empty directory is visible; links are left out.
    pub fn directories(&self) -> BTreeMap<String, Vec<u8>> {
        self.nodes
            .iter()
            .filter_map(|(path, node)| match node {
                Node::File(bytes) => Some((shown(path), bytes.clone())),
                Node::Directory => Some((format!("{}/", shown(path)), Vec::new())),
                _ => None,
            })
            .collect()
    }

    /// Every node tagged by kind: `D\0`, `F\0<bytes>`, `L\0<target>`, `O\0`.
    pub fn tagged(&self) -> BTreeMap<String, Vec<u8>> {
        self.nodes
            .iter()
            .map(|(path, node)| {
                let bytes = match node {
                    Node::Directory => b"D\0".to_vec(),
                    Node::File(bytes) => [b"F\0".as_slice(), bytes.as_slice()].concat(),
                    Node::Symlink(target) => {
                        [b"L\0".as_slice(), target.as_os_str().as_encoded_bytes()].concat()
                    }
                    Node::Other => b"O\0".to_vec(),
                };
                (shown(path), bytes)
            })
            .collect()
    }
}

/// A relative path as `/`-separated text. A name that is not valid Unicode
/// panics rather than alias another name.
fn shown(path: &Path) -> String {
    let parts: Vec<&str> =
        path.components()
            .map(|component| {
                component.as_os_str().to_str().unwrap_or_else(|| {
                    panic!("captured path {path:?} has no faithful text projection")
                })
            })
            .collect();
    parts.join("/")
}
