//! Real-repository lane runs shared within one test process (WCP-002).
//!
//! The lane sweep suites (`t108_lane_sweep`, `t109_pre_split_ports`,
//! `t110_post_split_ports`) run together in the default `lane_sweeps`
//! aggregate test binary. Separate binaries never overlapped; one process
//! does. So every suite step that runs this repository's real lanes, or
//! reads the tracked sweep report they rewrite, holds [`exclusive`]: two
//! suites never drive the same crate or race the report at once.
//!
//! A suite may reuse a lane observation it genuinely executed earlier in the
//! same process only through [`Observations`]: while the checkout still
//! matches, byte for byte, what that run saw from start to finish.
//! Observations live in memory only - never a persistent cache.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use sha2::{Digest, Sha256};

use crate::support::{self, CaptureOptions};

static REAL_LANES: Mutex<()> = Mutex::new(());

/// Serialize one step over this repository's real lanes or its tracked
/// sweep report within the test process. A failed holder never wedges the
/// next suite: poisoning is ignored.
pub fn exclusive() -> MutexGuard<'static, ()> {
    REAL_LANES.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Digest of every input a lane's `cargo test` can observe from disk: every
/// node of the checkout at `root` - sources, manifests, lockfiles, build
/// scripts, in-checkout Cargo configuration, and any data a test reads - and
/// the Cargo configuration Cargo discovers outside it (`.cargo/config` and
/// `.cargo/config.toml` in each ancestor directory and in Cargo's home; see
/// [`outside_digest`]).
///
/// Left out: Git history (`.git`, which no lane reads here), directories
/// named `target` (build output, not input), and the toolchain and process
/// environment, which stay fixed for the life of one test process. Any byte
/// change, addition, removal, or kind change of a covered node moves it.
pub fn inputs_digest(root: &Path) -> String {
    let checkout = support::capture(
        root,
        CaptureOptions {
            excluded_names: &[".git"],
            excluded_directory_names: &["target"],
            ..CaptureOptions::default()
        },
    )
    .unwrap_or_else(|error| panic!("capture lane inputs under {}: {error}", root.display()));
    let mut hasher = Sha256::new();
    for (path, tagged) in checkout.tagged() {
        record(&mut hasher, b"checkout", path.as_bytes(), Some(&tagged));
    }
    record_outside(&mut hasher, root);
    format!("{:x}", hasher.finalize())
}

/// Digest of the Cargo configuration a command run inside `root` discovers
/// outside the checkout: `.cargo/config` and `.cargo/config.toml` in each
/// ancestor directory, in the home `CARGO_HOME` names, and in the default
/// home. Both homes count because a child whose environment drops
/// `CARGO_HOME` (the sweep's per-crate command does) reads the default one.
/// A caller whose own checkout snapshot must leave something out - the real
/// sweep excludes the report it rewrites - pairs that snapshot with this.
pub fn outside_digest(root: &Path) -> String {
    let mut hasher = Sha256::new();
    record_outside(&mut hasher, root);
    format!("{:x}", hasher.finalize())
}

fn record_outside(hasher: &mut Sha256, root: &Path) {
    for path in outside_configuration(root) {
        let bytes = match fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => panic!("read Cargo configuration {}: {error}", path.display()),
        };
        record(
            hasher,
            b"outside",
            path.as_os_str().as_encoded_bytes(),
            bytes.as_deref(),
        );
    }
}

/// One length-framed row, so no two distinct input sets hash alike.
fn record(hasher: &mut Sha256, scope: &[u8], name: &[u8], bytes: Option<&[u8]>) {
    for field in [scope, name] {
        hasher.update((field.len() as u64).to_le_bytes());
        hasher.update(field);
    }
    match bytes {
        Some(bytes) => {
            hasher.update([1]);
            hasher.update((bytes.len() as u64).to_le_bytes());
            hasher.update(bytes);
        }
        None => hasher.update([0]),
    }
}

/// Cargo configuration files a command run inside `root` also discovers.
fn outside_configuration(root: &Path) -> Vec<PathBuf> {
    let mut directories: Vec<PathBuf> = root
        .ancestors()
        .skip(1)
        .map(|directory| directory.join(".cargo"))
        .collect();
    directories.extend(std::env::var_os("CARGO_HOME").map(PathBuf::from));
    directories.extend(
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .map(|home| PathBuf::from(home).join(".cargo")),
    );
    directories
        .into_iter()
        .flat_map(|directory| {
            [OsStr::new("config"), OsStr::new("config.toml")].map(|name| directory.join(name))
        })
        .collect()
}

/// Lane observations genuinely executed in this process, one per key.
pub struct Observations<T> {
    runs: Mutex<BTreeMap<String, (String, Arc<T>)>>,
}

impl<T> Observations<T> {
    pub const fn new() -> Self {
        Self {
            runs: Mutex::new(BTreeMap::new()),
        }
    }

    /// The observation for `key` over the checkout at `root`. An earlier one
    /// is reused only when [`inputs_digest`] now equals the digest taken
    /// both before and after that run. Otherwise `run` executes now, and its
    /// result is kept for reuse only if the inputs did not move while it
    /// ran. Callers over this repository hold [`exclusive`].
    pub fn observe(&self, root: &Path, key: &str, run: impl FnOnce() -> T) -> Arc<T> {
        let before = inputs_digest(root);
        let mut runs = self.runs.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((seen, observed)) = runs.get(key) {
            if *seen == before {
                return Arc::clone(observed);
            }
        }
        let observed = Arc::new(run());
        if inputs_digest(root) == before {
            runs.insert(key.to_owned(), (before, Arc::clone(&observed)));
        } else {
            runs.remove(key);
        }
        observed
    }
}

impl<T> Default for Observations<T> {
    fn default() -> Self {
        Self::new()
    }
}
