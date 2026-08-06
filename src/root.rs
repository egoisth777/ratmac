//! Engine-root resolution.
//!
//! The checkout that invokes `rtm` supplies tracked, workflow-authored files;
//! Git worktree metadata supplies the repository-wide runtime root.  A missing
//! or unusable Git executable deliberately falls back to a checkout-local
//! Engine root, so resolution is offline and dependency-free.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The Engine-owned directory at either the primary or invoking checkout root.
pub const ENGINE_DIR: &str = ".ratmac";

/// The Machine Class file name is owned by `MachineClass`, the runbook's one
/// reader; this module only addresses it inside the invoking checkout.
use crate::machine::MachineClass;

/// The two roots relevant to one Engine invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Roots {
    invoking_checkout_root: PathBuf,
    engine_root: PathBuf,
}

impl Roots {
    /// Resolve the invoking checkout and its Engine runtime root.
    ///
    /// In a Git worktree, the first non-bare worktree in Git's porcelain
    /// listing supplies the shared Engine root. Any failed or unsuitable Git
    /// query intentionally uses the invoking checkout's own `.ratmac/`
    /// directory instead.
    pub fn resolve(invoking_checkout_root: impl AsRef<Path>) -> Self {
        let invoking_checkout_root = absolute(invoking_checkout_root.as_ref());
        let engine_root = git_primary_checkout_root(&invoking_checkout_root)
            .unwrap_or_else(|| invoking_checkout_root.clone())
            .join(ENGINE_DIR);
        Self {
            invoking_checkout_root,
            engine_root,
        }
    }

    /// The checkout from which the command was invoked.
    pub fn invoking_checkout_root(&self) -> &Path {
        &self.invoking_checkout_root
    }

    /// The resolved, potentially primary-checkout, Engine runtime root.
    pub fn engine_root(&self) -> &Path {
        &self.engine_root
    }

    /// The Machine Class path, which is always read from the invoking checkout.
    pub fn machine_class_path(&self) -> PathBuf {
        self.invoking_checkout_root
            .join(ENGINE_DIR)
            .join(MachineClass::FILE_NAME)
    }
}

/// Resolve both roots for an invocation rooted at `invoking_checkout_root`.
pub fn resolve(invoking_checkout_root: impl AsRef<Path>) -> Roots {
    Roots::resolve(invoking_checkout_root)
}

fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

fn git_primary_checkout_root(invoking_checkout_root: &Path) -> Option<PathBuf> {
    let inside_work_tree = git_command(invoking_checkout_root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .ok()?;
    if !inside_work_tree.status.success()
        || String::from_utf8(inside_work_tree.stdout).ok()?.trim() != "true"
    {
        return None;
    }

    let worktrees = git_command(invoking_checkout_root)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .ok()?;
    if !worktrees.status.success() {
        return None;
    }
    let worktrees = String::from_utf8(worktrees.stdout).ok()?;
    // ENS-003 requires one roster and id namespace per repository, so all
    // worktrees share one Engine root. A bare store has no worktree and can
    // never be that root.
    let primary_checkout = worktrees
        .split("\n\n")
        .find(|record| !record.lines().any(|line| line == "bare"))?
        .lines()
        .find_map(|line| line.strip_prefix("worktree "))?;
    let primary_checkout = PathBuf::from(primary_checkout);
    if !primary_checkout.is_absolute() {
        return None;
    }

    let git_dir = git_command(invoking_checkout_root)
        .args(["rev-parse", "--path-format=absolute", "--git-dir"])
        .output()
        .ok()?;
    if !git_dir.status.success() {
        return None;
    }
    let git_dir = String::from_utf8(git_dir.stdout).ok()?;
    let git_dir = PathBuf::from(git_dir.lines().next()?);
    // Git storage is not a worktree. In particular, a separate Git directory
    // can appear in this position, so reject it rather than infer a root.
    if !git_dir.is_absolute() || primary_checkout == git_dir {
        return None;
    }

    Some(primary_checkout)
}

fn git_command(invoking_checkout_root: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .current_dir(invoking_checkout_root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_WORK_TREE");
    command
}
