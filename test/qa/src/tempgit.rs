//! Throwaway Git repositories for QA fixtures.
//!
//! Each `TempRepo` is an isolated repository under the system temp directory
//! with deterministic identity and line-ending settings, removed on drop.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// An isolated Git repository that deletes itself when dropped.
pub struct TempRepo {
    root: PathBuf,
}

impl TempRepo {
    /// Create an initialized repository whose directory name carries `label`.
    pub fn new(label: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after the epoch")
            .as_nanos();
        let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "ratmac-{label}-{}-{stamp}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create temp repository directory");

        let repo = Self { root };
        repo.git(&["init"]);
        repo.git(&["symbolic-ref", "HEAD", "refs/heads/main"]);
        repo.git(&["config", "user.email", "qa@ratmac.test"]);
        repo.git(&["config", "user.name", "ratmac qa"]);
        // Byte-exact fixtures: never rewrite line endings on Windows.
        repo.git(&["config", "core.autocrlf", "false"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo
    }

    /// The repository root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Run a Git command in this repository, panicking on spawn failure.
    pub fn git(&self, args: &[&str]) -> std::process::Output {
        Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .output()
            .unwrap_or_else(|error| panic!("git {args:?} must run: {error}"))
    }

    /// Write `content` to `relative`, creating parent directories.
    pub fn write(&self, relative: &str, content: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent directory");
        }
        fs::write(&path, content).expect("write fixture file");
    }

    /// Stage one path.
    pub fn stage(&self, relative: &str) {
        let output = self.git(&["add", "--", relative]);
        assert!(output.status.success(), "git add {relative} failed");
    }

    /// Stage everything and commit it.
    pub fn commit_all(&self, message: &str) {
        let add = self.git(&["add", "-A"]);
        assert!(add.status.success(), "git add -A failed");
        let commit = self.git(&["commit", "-m", message]);
        assert!(
            commit.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&commit.stderr)
        );
    }

    /// The current commit id.
    pub fn head(&self) -> String {
        let output = self.git(&["rev-parse", "HEAD"]);
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
