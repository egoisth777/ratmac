//! Throwaway Git repositories for QA fixtures.
//!
//! Each `TempRepo` is an isolated repository under the system temp directory
//! with deterministic identity and line-ending settings, removed on drop.
//! Ownership and Git process isolation come from [`crate::support`].

use crate::support::{self, TempTree};
use std::path::Path;

/// An isolated Git repository that deletes itself when dropped.
pub struct TempRepo {
    tree: TempTree,
}

impl TempRepo {
    /// Create an initialized repository whose directory name carries `label`.
    pub fn new(label: &str) -> Self {
        let tree = TempTree::new(label).expect("create temp repository directory");
        let repo = Self { tree };
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
        self.tree.path()
    }

    /// Run a Git command in this repository, panicking on spawn failure.
    /// Inherited Git redirections never reach it.
    pub fn git(&self, args: &[&str]) -> std::process::Output {
        support::command("git", self.root())
            .args(args)
            .output()
            .unwrap_or_else(|error| panic!("git {args:?} must run: {error}"))
    }

    /// Write `content` to `relative`, creating parent directories.
    pub fn write(&self, relative: &str, content: &str) {
        self.tree.write(relative, content);
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
