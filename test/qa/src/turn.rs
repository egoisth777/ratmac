//! Throwaway Git repositories for the turn-lifecycle suite (t-107).
//!
//! Every fixture is a real repository under the temp directory carrying the
//! real `tools/turn.ps1` and its declared data in `.ratmac/turn.toml`, so the
//! suite exercises the shipped script and compares byte-identical snapshots
//! of everything a turn verb can touch: refs, tags, worktree registrations,
//! the primary index, the primary tree, and every sibling working tree.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::trial::digest_tree;

/// The declared trunk branch every fixture uses.
pub const TRUNK: &str = "main";

/// The declared untracked lanes root every fixture uses.
pub const LANES: &str = "lanes";

/// The declared item-records root every fixture uses: the item record for
/// `<item>` is `<root>/<item>.md`, exactly as the declaration resolves it.
pub const ITEM_RECORDS: &str = "items";

/// The declared landing log every fixture uses.
pub const LANDING_LOG: &str = "landlog.md";

/// The declared stamp field every fixture uses.
pub const STAMP_FIELD: &str = "landed-commit";

/// The declared per-lane build output every fixture skips.
pub const SKIP: &str = "target";

/// The marker the declared rerun command leaves in every lane directory, so
/// the trunk rerun is observable without depending on the command's words.
pub const RERUN_MARKER: &str = "rerun.marker";

/// The declared rerun command: one marker file per lane directory.
pub const RERUN_COMMAND: &str = "New-Item -ItemType File -Force -Path rerun.marker | Out-Null";

/// An open turn fixture: primary checkout plus its sibling worktree.
pub struct Turn {
    parent: PathBuf,
    pub root: PathBuf,
}

impl Drop for Turn {
    fn drop(&mut self) {
        // Registered worktrees keep no handles open once the processes that
        // drove them have exited.
        let _ = fs::remove_dir_all(&self.parent);
    }
}

impl Turn {
    /// A repository whose declared trunk is checked out clean, carrying the
    /// real `tools/turn.ps1`, the declared turn data, a lanes root with one
    /// lane and its build output, one item record, and a landing log.
    pub fn new(label: &str) -> Self {
        let parent = std::env::temp_dir().join(format!(
            "ratmac-t107-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&parent);
        let root = parent.join("repo");
        fs::create_dir_all(root.join("tools")).expect("create fixture repository");

        let turn = Turn { parent, root };
        turn.git(&["init", "--initial-branch", "main", "."]);
        turn.git(&["config", "user.email", "turn@example.invalid"]);
        turn.git(&["config", "user.name", "turn fixture"]);
        turn.git(&["config", "core.autocrlf", "false"]);

        // The lanes root is the declared *untracked* root: ignored, committed
        // nowhere - the exact shape the only-copy refusal walks.
        fs::write(
            turn.root.join(".gitignore"),
            "lanes/\nonly-copy-crate/\ndoomed-crate/\n",
        )
        .expect("write fixture gitignore");
        fs::write(turn.root.join("README.md"), "# fixture\n").expect("write fixture file");
        turn.write(format!("{LANES}/crate-a/lane.txt"), "lane a\n");
        turn.write(format!("{LANES}/crate-a/{SKIP}/junk.bin"), "build output\n");
        turn.write(
            format!("{ITEM_RECORDS}/item-1.md"),
            "---\nitem-id: \"item-1\"\nstatus: \"approved\"\nlanded-commit: \"\"\n---\n\n# Item: item-1\n",
        );
        fs::write(turn.root.join(LANDING_LOG), "# landings\n").expect("write landing log");
        turn.write_declaration();
        fs::copy(script_source(), turn.root.join("tools/turn.ps1"))
            .expect("install the script under test");

        turn.git(&["add", "-A"]);
        turn.git(&["commit", "-m", "fixture base"]);
        turn
    }

    /// The fixture's declared turn data, in the runbook's `[roots]`-table
    /// shape, repo-local beside the runbook.
    fn write_declaration(&self) {
        let declaration = format!(
            "# The turn lifecycle's declared data (THK-001..THK-004).\n\
             [roots]\n\
             trunk = \"{TRUNK}\"\n\
             lanes = \"{LANES}\"\n\
             item-records = \"{ITEM_RECORDS}\"\n\
             landing-log = \"{LANDING_LOG}\"\n\
             stamp-field = \"{STAMP_FIELD}\"\n\
             skip = [\"{SKIP}\"]\n\
             lanes-rerun = \"{RERUN_COMMAND}\"\n"
        );
        let directory = self.root.join(".ratmac");
        fs::create_dir_all(&directory).expect("create declaration directory");
        fs::write(directory.join("turn.toml"), declaration).expect("write declaration");
    }

    fn write(&self, relative: String, content: &str) {
        let path = self.root.join(&relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent directory");
        }
        fs::write(path, content).expect("write fixture file");
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
    pub fn turn_in(&self, directory: &Path, args: &[&str]) -> Output {
        let mut all = vec!["-NoProfile", "-File", "tools/turn.ps1"];
        all.extend_from_slice(args);
        Command::new("pwsh")
            .args(&all)
            .current_dir(directory)
            .output()
            .expect("invoke pwsh")
    }

    pub fn turn(&self, args: &[&str]) -> Output {
        self.turn_in(&self.root, args)
    }

    /// Combined stdout and stderr, for wording assertions.
    pub fn text(&self, args: &[&str]) -> String {
        let output = self.turn(args);
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }

    pub fn head_of(&self, reference: &str) -> String {
        self.git_text(&["rev-parse", reference]).trim().to_owned()
    }

    /// The short hash of `reference`, as `git rev-parse --short` renders it -
    /// the identity the stamp step writes.
    pub fn short_of(&self, reference: &str) -> String {
        self.git_text(&["rev-parse", "--short", reference])
            .trim()
            .to_owned()
    }

    /// The sibling worktree path the item's turn opens, by the tool's own
    /// convention: `<parent>/<repo-name>-<item>`.
    pub fn sibling(&self, item: &str) -> PathBuf {
        self.parent.join(format!("repo-{item}"))
    }

    /// Open the item's turn and commit one tracked change inside it, so a
    /// close has real work to land.
    pub fn open_with_work(&self, item: &str, edit: &str) -> PathBuf {
        let opened = self.turn(&["open", "-Item", item]);
        assert!(
            opened.status.success(),
            "open {item} succeeds: {}",
            String::from_utf8_lossy(&opened.stderr)
        );
        let worktree = self.sibling(item);
        let readme = worktree.join("README.md");
        fs::write(&readme, format!("# fixture\n{edit}\n")).expect("write turn work");
        // The ignored work the close must carry: a new lane, committed
        // nowhere, existing only in the worktree until the copy-back runs.
        fs::create_dir_all(worktree.join(LANES).join("crate-b")).expect("create the new lane");
        fs::write(
            worktree.join(LANES).join("crate-b").join("b.txt"),
            "lane b\n",
        )
        .expect("write the new lane");
        let commit = self.git_in(&worktree, &["add", "-A"]);
        assert!(commit.status.success(), "stage the turn work");
        let commit = self.git_in(&worktree, &["commit", "-m", "turn work"]);
        assert!(
            commit.status.success(),
            "commit the turn work: {}",
            String::from_utf8_lossy(&commit.stderr)
        );
        worktree
    }

    /// The value of one `field: "value"` line, verbatim between its quotes -
    /// the test's own reader, independent of the tool under test.
    pub fn field_value(&self, relative: &str, field: &str) -> String {
        let text = fs::read_to_string(self.root.join(relative))
            .unwrap_or_else(|error| panic!("read {relative}: {error}"));
        let prefix = format!("{field}:");
        let line = text
            .lines()
            .find(|line| line.trim_start().starts_with(&prefix))
            .unwrap_or_else(|| panic!("{relative} carries the field {field}"));
        line.trim()
            .strip_prefix(&prefix)
            .expect("the field line carries its colon")
            .trim()
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or_else(|| panic!("the {field} value is quoted"))
            .to_owned()
    }

    /// Everything a turn verb could mutate, in one comparable value: refs,
    /// tags, registrations, primary index and tree, and every sibling tree.
    pub fn snapshot(&self) -> String {
        let refs = self.git_text(&["show-ref"]);
        let tags = self.git_text(&["tag", "--list"]);
        let registrations = self.git_text(&["worktree", "list", "--porcelain"]);
        let status = self.git_text(&["status", "--porcelain"]);
        let index = self.git_text(&["ls-files", "--stage"]);
        let primary = digest_tree(&self.root);
        let mut siblings: Vec<String> = Vec::new();
        for entry in fs::read_dir(&self.parent).expect("read sibling directory") {
            let path = entry.expect("read entry").path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            if name == "repo" || !path.is_dir() {
                continue;
            }
            // A sibling working tree still registered carries a `.git` file;
            // the registration list above covers its identity, the digest
            // covers its bytes.
            let digest = {
                let mut rows = Vec::new();
                digest_skipping_git(&path, &path, &mut rows);
                rows.push(String::new());
                rows.join("\n")
            };
            siblings.push(format!("{name} {digest}"));
        }
        siblings.sort();
        format!(
            "refs:\n{refs}tags:\n{tags}registrations:\n{registrations}status:\n{status}index:\n{index}primary:\n{primary}siblings:\n{}\n",
            siblings.join("\n")
        )
    }
}

/// `digest_tree` over a working tree that may carry the linked-worktree `.git`
/// file, which is skipped along with any `.git` directory.
fn digest_skipping_git(directory: &Path, base: &Path, rows: &mut Vec<String>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)
        .expect("read sibling tree")
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
            digest_skipping_git(&path, base, rows);
        } else {
            let relative = path
                .strip_prefix(base)
                .expect("path under the sibling tree")
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = fs::read(&path).expect("read sibling-tree file");
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            rows.push(format!("{relative} {:x}", hasher.finalize()));
        }
    }
}

pub fn script_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/turn.ps1")
        .canonicalize()
        .expect("the script under test exists")
}
