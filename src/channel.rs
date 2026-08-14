//! Edition-Channel Pin (ECP-001..ECP-003, issue i-031).
//!
//! A built engine binary comes from one of two channels: `stable` - the
//! newest edition recorded in the repository's own ledger - or `nightly` -
//! the current landing. Resolution is offline: the ledger, the local tag
//! database, and `HEAD` are the only inputs, and a ledger/tag disagreement is
//! a refusal, never a side taken.

use std::path::Path;
use std::process::Command;

/// The channels a built engine binary can come from.
pub const CHANNELS: [&str; 2] = ["stable", "nightly"];

/// One resolved channel: which channel, and the commit it names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolution {
    pub channel: String,
    /// Full 40-hex commit hash the channel resolves to.
    pub commit: String,
    /// The edition name behind a `stable` resolution; empty for `nightly`.
    pub edition: String,
}

/// Resolve `channel` for the repository at `root`, offline.
///
/// - `stable`: the newest row of the editions ledger, refused unless the tag
///   it names still points at the recorded commit (`EDN-003`: a moved tag is
///   a reported difference, and this resolver reports it by refusing).
/// - `nightly`: the current landing, `HEAD`.
pub fn resolve_channel(root: &Path, channel: &str) -> Result<Resolution, String> {
    match channel {
        "nightly" => {
            let commit = git_commit(root, "HEAD")?;
            Ok(Resolution {
                channel: "nightly".into(),
                commit,
                edition: String::new(),
            })
        }
        "stable" => {
            let (edition, recorded) = newest_edition(root)?;
            let tagged = git_commit(root, &format!("refs/tags/{edition}"))
                .map_err(|error| format!("stable: ledger names {edition}, but {error}"))?;
            if tagged != recorded {
                return Err(format!(
                    "stable: the ledger records {edition} at {recorded} but the tag points at {tagged}; \
                     a ledger/tag disagreement is refused, not resolved"
                ));
            }
            Ok(Resolution {
                channel: "stable".into(),
                commit: recorded,
                edition,
            })
        }
        other => Err(format!(
            "unknown channel {other:?}; a channel is one of {CHANNELS:?}, never a default"
        )),
    }
}

/// The newest edition row of the ledger: `(edition name, full commit hash)`.
///
/// The ledger's rows are append-only and never edited, so the newest edition
/// is the last data row. A missing ledger, a ledger with no rows, or a row
/// whose commit is not a whole 40-hex hash refuses by name.
pub fn newest_edition(root: &Path) -> Result<(String, String), String> {
    // ENS-009: the ledger lives under the contributor workflow root; the one
    // named declaration in the scheduler is the only spelling of that folder.
    let path = root
        .join(crate::scheduler::legacy_workflow_dir())
        .join("editions.md");
    let text = std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "stable: cannot read {}: {error}",
            crate::root::displayed(&path)
        )
    })?;
    let mut newest = None;
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix('|') else {
            continue;
        };
        let cells: Vec<&str> = rest.trim_end_matches('|').split('|').collect();
        if cells.len() < 2 {
            continue;
        }
        let edition = cells[0].trim().trim_matches('`').to_owned();
        if !edition.starts_with("edition-") {
            continue;
        }
        let commit = cells[1].trim().trim_matches('`').to_owned();
        if commit.len() != 40 || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "stable: ledger row {edition} records {commit:?}, which is not a whole \
                 40-hex commit hash; a truncated record refuses, never resolves"
            ));
        }
        newest = Some((edition, commit));
    }
    newest.ok_or_else(|| {
        "stable: the editions ledger carries no edition row, so there is no stable to resolve"
            .into()
    })
}

/// Findings for live Runs driven by an off-pin engine (ECP-003).
///
/// A live Run whose recorded `[engine]` carries provenance is checked against
/// the stable resolution: a non-stable channel, or a stable claim whose
/// source commit is not the resolved stable commit, is one finding. A pin
/// without provenance predates this rule and yields none. Read-only.
pub fn live_run_findings(root: &Path, engine_root: &Path) -> Vec<String> {
    let mut findings = Vec::new();
    let runs_root = engine_root.join("runs");
    let Ok(entries) = std::fs::read_dir(&runs_root) else {
        return findings;
    };
    let stable = resolve_channel(root, "stable").ok();
    let mut run_ids: Vec<String> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry
                .path()
                .is_dir()
                .then(|| crate::root::component(entry.file_name()))
        })
        .collect();
    run_ids.sort();
    for run_id in run_ids {
        let run_dir = runs_root.join(&run_id);
        if !run_is_live(&run_dir) {
            continue;
        }
        let evidence = crate::pin::Evidence::load(&run_dir);
        let Some(engine) = evidence.engine else {
            continue;
        };
        let Some(channel) = engine.channel else {
            continue;
        };
        if channel != "stable" {
            findings.push(format!(
                "run {run_id} is live and driven by a {channel} engine, not the stable pin"
            ));
            continue;
        }
        let Some(source) = &engine.source_commit else {
            findings.push(format!(
                "run {run_id} is live and its engine claims stable with no source commit, so the \
                 claim cannot be checked"
            ));
            continue;
        };
        if let Some(stable) = &stable {
            if *source != stable.commit {
                findings.push(format!(
                    "run {run_id} is live and its engine claims stable from {source}, but stable \
                     is {} ({})",
                    stable.commit, stable.edition
                ));
            }
        }
    }
    findings
}

/// A Run is live while its record is `planned` or `executing`.
fn run_is_live(run_dir: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(run_dir.join("run.toml")) else {
        return false;
    };
    text.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "status = \"planned\"" || trimmed == "status = \"executing\""
    })
}

/// One local, offline commit lookup.
fn git_commit(root: &Path, reference: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", &format!("{reference}^{{commit}}")])
        .current_dir(root)
        .output()
        .map_err(|error| format!("git rev-parse could not run: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{reference} does not resolve in the local repository: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
