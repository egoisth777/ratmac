//! The growing-fixture resolver (t-100 / GPH-003).
//!
//! `GPH-003` makes this repository the one fixture whose past is guaranteed
//! to keep growing, so no check may pin a run id or a ticket path: the
//! addressed run and its ticket are resolved from the tree at runtime, and
//! every this-repository check shares this one resolution.

use std::fs;
use std::path::{Path, PathBuf};

/// This repository's root, resolved from the harness location - never
/// hard-coded, and verified to be a repository before any check reads it.
pub fn repo_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root resolves");
    assert!(
        root.join(".arca/schema.md").is_file(),
        "the resolved root {} is this repository, not a stray directory",
        root.display()
    );
    root
}

/// The most recent retired ticket run: the highest-numbered run under
/// `.ratmac/evidence/` whose receipts name a ticket that is archived. Grows
/// with the repository; never a snapshot.
pub struct RetiredRun {
    pub run_id: String,
    /// Relative address of the archived ticket, e.g. `.arca/ticket/archive/t-098.md`.
    pub ticket_rel: String,
    /// One receipt file of that run, for write-nothing watches.
    pub receipt: PathBuf,
}

/// The stage a resolved run's repository is in, from the tree alone.
#[derive(Debug, PartialEq, Eq)]
pub enum SprintStage {
    /// The addressed ticket is archived and not live: the state the
    /// this-repository checks expect.
    BetweenSprints,
    /// The addressed ticket is still live: a turn is executing.
    MidSprint,
    /// The run's evidence is gone: an archive sweep retired it.
    Swept,
}

pub fn latest_retired(root: &Path) -> RetiredRun {
    let evidence = root.join(".ratmac/evidence");
    let mut runs: Vec<String> = fs::read_dir(&evidence)
        .expect("the evidence root is readable")
        .filter_map(|entry| {
            let entry = entry.expect("evidence entry is readable");
            entry
                .path()
                .is_dir()
                .then(|| entry.file_name().to_string_lossy().into_owned())
        })
        .collect();
    runs.sort();
    for run_id in runs.into_iter().rev() {
        let run_dir = evidence.join(&run_id);
        let Some(receipt) = first_receipt(&run_dir) else {
            continue;
        };
        let text = fs::read_to_string(&receipt).expect("receipt is readable");
        let Some(ticket_id) = toml_str(&text, "ticket-id") else {
            continue;
        };
        let ticket_rel = format!(".arca/ticket/archive/{ticket_id}.md");
        if root.join(&ticket_rel).is_file() {
            return RetiredRun {
                run_id,
                ticket_rel,
                receipt,
            };
        }
    }
    panic!(
        "no retired run with an archived ticket exists under {}",
        evidence.display()
    );
}

/// The stage the repository is in with respect to the resolved run - each
/// answer names what was observed, so a mismatch is a difference, not a guess.
pub fn sprint_stage(root: &Path, run: &RetiredRun) -> SprintStage {
    let ticket_id = Path::new(&run.ticket_rel)
        .file_name()
        .expect("ticket address has a file name")
        .to_string_lossy()
        .into_owned();
    if root.join(".arca/ticket").join(&ticket_id).is_file() {
        return SprintStage::MidSprint;
    }
    if !root.join(".ratmac/evidence").join(&run.run_id).is_dir() {
        return SprintStage::Swept;
    }
    SprintStage::BetweenSprints
}

fn first_receipt(run_dir: &Path) -> Option<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(run_dir)
        .ok()?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension().is_some_and(|e| e == "toml")).then_some(path)
        })
        .collect();
    files.sort();
    files.into_iter().next()
}

fn toml_str(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.strip_prefix(key)?.trim_start().strip_prefix('=')?;
        Some(rest.trim().trim_matches('"').to_owned())
    })
}
