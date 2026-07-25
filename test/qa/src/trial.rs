//! Throwaway Git repositories for the trial-worktree lifecycle suites.
//!
//! Every fixture is a real repository under the temp directory carrying the
//! real `tools/trial.ps1`, so the suites exercise the shipped script and
//! compare byte-identical snapshots of everything a lifecycle verb can touch.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sha2::{Digest, Sha256};

pub const BASE: &str = "exp/ratmac-deterministic";

pub struct Trial {
    parent: PathBuf,
    pub root: PathBuf,
}

impl Drop for Trial {
    fn drop(&mut self) {
        // Registered worktrees keep no handles open once the process exits.
        let _ = fs::remove_dir_all(&self.parent);
    }
}

impl Trial {
    /// A repository whose experiment base is checked out clean, carrying the
    /// real `tools/trial.ps1` under test.
    pub fn new(label: &str) -> Self {
        let parent = std::env::temp_dir().join(format!(
            "ratmac-t052-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&parent);
        let root = parent.join("repo");
        fs::create_dir_all(root.join("tools")).expect("create fixture repository");

        let trial = Trial { parent, root };
        trial.git(&["init", "--initial-branch", "main", "."]);
        trial.git(&["config", "user.email", "trial@example.invalid"]);
        trial.git(&["config", "user.name", "trial fixture"]);
        trial.git(&["config", "core.autocrlf", "false"]);
        fs::write(trial.root.join("README.md"), "# fixture\n").expect("write fixture file");
        fs::copy(script_source(), trial.root.join("tools/trial.ps1")).expect("install the script");
        trial.git(&["add", "-A"]);
        trial.git(&["commit", "-m", "fixture base"]);
        trial.git(&["branch", BASE]);
        trial.git(&["checkout", BASE]);
        trial
    }

    pub fn git_in(&self, directory: &Path, args: &[&str]) -> Output {
        Command::new("git")
            .args(args)
            .current_dir(directory)
            .output()
            .expect("invoke git")
    }

    pub fn git(&self, args: &[&str]) -> Output {
        let output = self.git_in(&self.root, args);
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    pub fn git_text(&self, args: &[&str]) -> String {
        String::from_utf8_lossy(&self.git(args).stdout).into_owned()
    }

    /// Run the script under test from a chosen working directory.
    pub fn trial_in(&self, directory: &Path, args: &[&str]) -> Output {
        let mut all = vec!["-NoProfile", "-File", "tools/trial.ps1"];
        all.extend_from_slice(args);
        Command::new("pwsh")
            .args(&all)
            .current_dir(directory)
            .output()
            .expect("invoke pwsh")
    }

    pub fn trial(&self, args: &[&str]) -> Output {
        self.trial_in(&self.root, args)
    }

    pub fn text(&self, args: &[&str]) -> String {
        let output = self.trial(args);
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }

    /// Everything a lifecycle operation could mutate, in one comparable value.
    pub fn snapshot(&self) -> String {
        let refs = self.git_text(&["show-ref"]);
        let tags = self.git_text(&["tag", "--list"]);
        let registrations = self.git_text(&["worktree", "list", "--porcelain"]);
        let status = self.git_text(&["status", "--porcelain"]);
        let index = self.git_text(&["ls-files", "--stage"]);
        let worktree_digest = digest_tree(&self.root);
        let mut siblings: Vec<String> = fs::read_dir(&self.parent)
            .expect("read sibling directory")
            .map(|entry| {
                entry
                    .expect("read entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        siblings.sort();
        format!(
            "refs:\n{refs}tags:\n{tags}registrations:\n{registrations}status:\n{status}index:\n{index}worktree:\n{worktree_digest}siblings:\n{}\n",
            siblings.join("\n")
        )
    }

    pub fn head_of(&self, reference: &str) -> String {
        self.git_text(&["rev-parse", reference]).trim().to_owned()
    }

    pub fn sibling(&self, name: &str) -> PathBuf {
        self.parent.join(name)
    }
}

/// Path plus content of every working-tree file outside `.git`, so a refusal
/// cannot quietly rewrite tracked, dirty, or untracked bytes and still compare
/// equal.
pub fn digest_tree(root: &Path) -> String {
    fn walk(directory: &Path, base: &Path, rows: &mut Vec<String>) {
        let mut entries: Vec<PathBuf> = fs::read_dir(directory)
            .expect("read working tree")
            .map(|entry| entry.expect("read entry").path())
            .collect();
        entries.sort();
        for path in entries {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            if name == ".git" {
                continue;
            }
            if path.is_dir() {
                walk(&path, base, rows);
            } else {
                let relative = path
                    .strip_prefix(base)
                    .expect("path under the working tree")
                    .to_string_lossy()
                    .replace('\\', "/");
                let bytes = fs::read(&path).expect("read working-tree file");
                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                rows.push(format!("{relative} {:x}", hasher.finalize()));
            }
        }
    }
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows.push(String::new());
    rows.join("\n")
}

pub fn script_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/trial.ps1")
        .canonicalize()
        .expect("the script under test exists")
}
