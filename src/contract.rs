//! PGE-001, PGE-002: intake and record contract gates.
//!
//! The gates read the records themselves, so a status edit cannot route the
//! loop:
//!
//! - *intake* — issue ids are unique across intake, deferred, and archive;
//!   exact ask dispositions determine whether a five-file bundle is live in
//!   the deferred buffer or completed in archive; integrated claims contribute
//!   accepted requirement IDs to the goal; live links resolve;
//! - *records* — exactly one residual per requirement citing the frozen goal
//!   revision, `satisfied` only with concrete evidence, every gap owned by
//!   exactly one ticket, acyclic ticket dependencies, and complete ticket
//!   sections.
//!
//! Both run in-process, inside the pinned gate boundary.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// The five files every issue folder carries.
pub const ISSUE_FILES: [&str; 5] = [
    "design.md",
    "index.md",
    "spec.md",
    "test-plan.md",
    "ubi-lang.md",
];

/// The sections every ticket carries.
const TICKET_SECTIONS: [&str; 5] = [
    "## Vertical Outcome",
    "## Worktree Scope",
    "## P4 Apparent Test Plan",
    "## P5 Hidden Test Public Coverage Manifest",
    "## Merge Gate",
];

/// The hidden-test lanes a ticket must assess.
const HIDDEN_LANES: [&str; 6] = [
    "Regression",
    "Input/Routing",
    "Lifecycle/Model",
    "Durability/Recovery",
    "Output/Filesystem",
    "Cross-Feature",
];

/// The gate kinds that mechanize PGE-001..PGE-003, and what each proves.
const REQUIRED_GATES: [(&str, &str); 3] = [
    ("PGE-001", "intake_contract"),
    ("PGE-002", "record_contract"),
    ("PGE-003", "sensitivity_receipts"),
];

/// A record that does not meet its contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractDefect {
    /// The artifact a reviewer must open: an issue folder, residual, or ticket.
    pub artifact: String,
    pub reason: String,
}

impl fmt::Display for ContractDefect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.artifact, self.reason)
    }
}

fn defect(artifact: impl Into<String>, reason: impl Into<String>) -> ContractDefect {
    ContractDefect {
        artifact: artifact.into(),
        reason: reason.into(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IssueLocation {
    Intake,
    Deferred,
    Archive,
}

impl IssueLocation {
    fn is_live(self) -> bool {
        self != Self::Archive
    }
}

/// PGE-001: verify intake completion over the working tree.
///
/// This compatibility entry point resolves the contract's fixed role names
/// from the reviewed runbook before reading any workflow record.
pub fn gate_intake(workspace: &Path) -> Result<(), Vec<ContractDefect>> {
    let roots = contract_roots(workspace, &["goal", "issue"])?;
    gate_intake_at(&roots[0], &roots[1])
}

/// Evaluate intake records under the already resolved `goal` and `issue`
/// roots. Schema-known leaf names are the only addresses added here.
pub fn gate_intake_at(goal_root: &Path, issue_root: &Path) -> Result<(), Vec<ContractDefect>> {
    let mut defects = Vec::new();
    let goal_path = goal_root.join("spec.md");
    let shown_goal = shown(&goal_path);
    let goal = fs::read_to_string(&goal_path).unwrap_or_default();
    if goal.trim().is_empty() {
        defects.push(defect(
            &shown_goal,
            "goal authority is missing or empty, so no integration claim can be checked",
        ));
    }
    let goal_ids = requirement_ids(&goal);
    let mut seen_ids: BTreeMap<String, String> = BTreeMap::new();

    for (name, folder, location) in issue_folders(issue_root) {
        let shown = shown(&folder);

        if let Some(first) = seen_ids.insert(name.clone(), shown.clone()) {
            defects.push(defect(
                &shown,
                format!("issue id {name} is duplicated; first carrier is {first}"),
            ));
        }

        // Five-file shape: exactly these files, no more, no fewer.
        let mut present: BTreeSet<String> = BTreeSet::new();
        match fs::read_dir(&folder) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        defects.push(defect(
                            &shown,
                            format!(
                                "unexpected directory {:?} inside a five-file issue folder",
                                entry.file_name().to_string_lossy()
                            ),
                        ));
                        continue;
                    }
                    present.insert(entry.file_name().to_string_lossy().into_owned());
                }
            }
            Err(error) => {
                defects.push(defect(
                    &shown,
                    format!("issue folder is unreadable: {error}"),
                ));
                continue;
            }
        }
        for required in ISSUE_FILES {
            if !present.contains(required) {
                defects.push(defect(
                    &shown,
                    format!("five-file shape is broken: {required} is missing"),
                ));
            }
        }
        for found in &present {
            if !ISSUE_FILES.contains(&found.as_str()) {
                defects.push(defect(
                    &shown,
                    format!("five-file shape is broken: unexpected file {found}"),
                ));
            }
        }

        let index = fs::read_to_string(folder.join("index.md")).unwrap_or_default();
        let status = yaml_field(&index, "status").unwrap_or_default();
        let spec = fs::read_to_string(folder.join("spec.md")).unwrap_or_default();
        let accepted = requirements_with_disposition(&spec, "accepted");
        let duplicate = requirements_with_disposition(&spec, "duplicate");
        let deferred = requirements_with_disposition(&spec, "deferred");
        let has_deferred = !deferred.is_empty();

        match location {
            IssueLocation::Intake => {
                if has_deferred {
                    defects.push(defect(
                        &shown,
                        format!(
                            "spec contains a deferred ask; the complete issue must live under {}/deferred with status deferred",
                            crate::root::displayed(issue_root)
                        ),
                    ));
                }
                if !matches!(status.as_str(), "integrated" | "rejected") {
                    defects.push(defect(
                        &shown,
                        format!(
                            "issue status is {:?}; an issue left in the intake work area must end integrated or rejected",
                            if status.is_empty() { "absent" } else { &status }
                        ),
                    ));
                }
            }
            IssueLocation::Deferred => {
                if status != "deferred" {
                    defects.push(defect(
                        &shown,
                        format!(
                            "deferred-buffer issue status is {:?}; location requires deferred",
                            if status.is_empty() { "absent" } else { &status }
                        ),
                    ));
                }
                if !has_deferred {
                    defects.push(defect(
                        &shown,
                        "deferred-buffer issue has no ask whose disposition is deferred",
                    ));
                }
            }
            IssueLocation::Archive => {
                if !matches!(status.as_str(), "integrated" | "rejected") {
                    defects.push(defect(
                        &shown,
                        format!(
                            "archived issue status is {:?}; archive requires integrated or rejected",
                            if status.is_empty() { "absent" } else { &status }
                        ),
                    ));
                }
                if has_deferred {
                    defects.push(defect(
                        &shown,
                        format!(
                            "archived issue still contains a deferred ask; restore the complete bundle to {}/deferred",
                            crate::root::displayed(issue_root)
                        ),
                    ));
                }
            }
        }

        if status == "deferred" && !has_deferred {
            defects.push(defect(
                &shown,
                "claims deferred, but spec contains no deferred ask",
            ));
        }
        if status == "integrated" && accepted.is_empty() && duplicate.is_empty() {
            defects.push(defect(
                &shown,
                "claims integrated, but spec contains no accepted or duplicate ask",
            ));
        }
        if status == "rejected" && !accepted.is_empty() {
            defects.push(defect(
                &shown,
                "claims rejected, but spec still contains accepted requirements",
            ));
        }
        if matches!(status.as_str(), "integrated" | "deferred") {
            for requirement in &accepted {
                if !goal_ids.contains(requirement) {
                    defects.push(defect(
                        &shown,
                        format!(
                            "claims {status}, but accepted requirement {requirement} is absent from the goal authority"
                        ),
                    ));
                }
            }
        }

        // Archived links are frozen provenance. Intake and deferred links are live.
        if location.is_live() {
            for file in ISSUE_FILES {
                let path = folder.join(file);
                let Ok(text) = fs::read_to_string(&path) else {
                    continue;
                };
                for target in markdown_targets(&text) {
                    if !resolves(&folder, &target) {
                        defects.push(defect(
                            format!("{shown}/{file}"),
                            format!("link target does not resolve: {target}"),
                        ));
                    }
                }
            }
        }
    }

    // Reverse links from the goal must resolve too.
    for target in markdown_targets(&goal) {
        if !resolves(goal_root, &target) {
            defects.push(defect(
                &shown_goal,
                format!("reverse link does not resolve: {target}"),
            ));
        }
    }

    if defects.is_empty() {
        Ok(())
    } else {
        Err(defects)
    }
}
/// PGE-002: verify residual and ticket record contracts.
///
/// This compatibility entry point resolves the fixed contract roles from the
/// reviewed runbook before reading workflow records.
pub fn gate_records(
    workspace: &Path,
    engine_root: &Path,
    run_id: &str,
) -> Result<(), Vec<ContractDefect>> {
    let roots = contract_roots(workspace, &["goal", "residual", "ticket"])?;
    gate_records_at(
        workspace,
        &roots[0],
        &roots[1],
        &roots[2],
        engine_root,
        run_id,
    )
}

/// Evaluate records using resolved fixed-role roots and schema-known leaves.
pub fn gate_records_at(
    workspace: &Path,
    goal_root: &Path,
    residual_root: &Path,
    ticket_root: &Path,
    engine_root: &Path,
    run_id: &str,
) -> Result<(), Vec<ContractDefect>> {
    let mut defects = Vec::new();
    let run_dir = engine_root.join("runs").join(run_id);
    let frozen = crate::pin::Evidence::load(&run_dir).goal_frozen;
    if frozen.is_none() {
        defects.push(defect(
            format!(".ratmac/runs/{run_id}/evidence.toml"),
            "the goal is not frozen, so no residual can cite a frozen revision",
        ));
    }
    let unmechanized = unproven_mechanization(workspace);
    let goal = fs::read_to_string(goal_root.join("spec.md")).unwrap_or_default();
    let goal_ids = requirement_ids(&goal);

    // Active and archived residuals are one namespace.
    let mut by_requirement: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut gaps: Vec<(String, String)> = Vec::new();
    let mut residual_paths = files_in(residual_root.to_path_buf(), "md");
    residual_paths.extend(files_in(residual_root.join("archive"), "md"));
    residual_paths.sort();
    for path in residual_paths {
        let id = stem(&path);
        let shown = shown(&path);
        let text = fs::read_to_string(&path).unwrap_or_default();

        let Some(requirement) = yaml_field(&text, "goal-requirement-ref") else {
            defects.push(defect(
                &shown,
                "record is unreadable: no goal-requirement-ref field",
            ));
            continue;
        };
        let Some(status) = yaml_field(&text, "status") else {
            defects.push(defect(&shown, "record is unreadable: no status field"));
            continue;
        };
        if !goal_ids.is_empty() && !goal_ids.contains(&requirement) {
            defects.push(defect(
                &shown,
                format!("cites requirement {requirement}, which is absent from the goal authority"),
            ));
        }
        by_requirement
            .entry(requirement.clone())
            .or_default()
            .push(shown.clone());

        let cited = yaml_field(&text, "frozen-goal-bundle-revision").unwrap_or_default();
        if let Some(frozen) = frozen.as_deref() {
            if !cited.contains(frozen) {
                defects.push(defect(
                    &shown,
                    format!("cites goal revision {cited}, but the frozen revision is {frozen}"),
                ));
            }
        }

        match status.as_str() {
            "satisfied" => {
                if yaml_list(&text, "concrete-evidence-refs").is_empty() {
                    defects.push(defect(
                        &shown,
                        "status is satisfied with no concrete evidence references",
                    ));
                }
                if let Some(missing) = unmechanized.iter().find(|gap| gap.artifact == requirement) {
                    defects.push(defect(
                        &shown,
                        format!("status is satisfied, but {}", missing.reason),
                    ));
                }
            }
            "missing" | "partial" => gaps.push((id, shown)),
            other => defects.push(defect(
                &shown,
                format!("unknown status {other:?}; expected missing, partial, or satisfied"),
            )),
        }
    }
    for requirement in &goal_ids {
        if !by_requirement.contains_key(requirement) {
            defects.push(defect(
                format!("requirement {requirement}"),
                format!(
                    "has no residual record across {}/*.md and {}/archive/*.md; exactly one is required",
                    shown(residual_root),
                    shown(residual_root)
                ),
            ));
        }
    }
    for (requirement, residuals) in &by_requirement {
        if residuals.len() > 1 {
            defects.push(defect(
                format!("requirement {requirement}"),
                format!(
                    "has {} residual records ({}); exactly one is allowed",
                    residuals.len(),
                    residuals.join(", ")
                ),
            ));
        }
    }

    // Tickets.
    let mut owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut dependencies: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in files_in(ticket_root.to_path_buf(), "md") {
        let id = stem(&path);
        let shown = shown(&path);
        let text = fs::read_to_string(&path).unwrap_or_default();

        for section in TICKET_SECTIONS {
            if !text.contains(section) {
                defects.push(defect(
                    &shown,
                    format!("required section {section:?} is missing"),
                ));
            }
        }
        if text.contains("## P5 Hidden Test Public Coverage Manifest") {
            for lane in HIDDEN_LANES {
                if !text.contains(&format!("`{lane}`")) {
                    defects.push(defect(
                        &shown,
                        format!("hidden-lane assessment for {lane} is missing"),
                    ));
                }
            }
        } else {
            defects.push(defect(
                &shown,
                "hidden-lane assessments are missing entirely",
            ));
        }

        for residual in yaml_list(&text, "residual-ids") {
            owners.entry(residual).or_default().push(id.clone());
        }
        dependencies.insert(id.clone(), yaml_list(&text, "dependencies"));
    }

    for (gap, shown) in &gaps {
        let ticket_owners = owners.get(gap).cloned().unwrap_or_default();
        match ticket_owners.len() {
            1 => {}
            0 => defects.push(defect(
                shown,
                "gap is owned by no ticket; exactly one ticket must own it",
            )),
            count => defects.push(defect(
                shown,
                format!(
                    "gap is owned by {count} tickets ({}); exactly one ticket must own it",
                    ticket_owners.join(", ")
                ),
            )),
        }
    }

    for cycle in find_cycles(&dependencies) {
        defects.push(defect(
            shown(&ticket_root.join(format!("{}.md", cycle[0]))),
            format!("ticket dependencies form a cycle: {}", cycle.join(" -> ")),
        ));
    }

    if defects.is_empty() {
        Ok(())
    } else {
        Err(defects)
    }
}

/// Requirements whose mechanizing gate the project's Runbook does not declare.
///
/// A loop that never runs a gate cannot report the gate's requirement
/// satisfied: absence of a check is not evidence.
pub fn unproven_mechanization(root: &Path) -> Vec<ContractDefect> {
    let declared = declared_gate_kinds(root);
    REQUIRED_GATES
        .iter()
        .filter(|(_, kind)| !declared.contains(*kind))
        .map(|(requirement, kind)| {
            defect(
                *requirement,
                format!(
                    "classified missing: the Runbook declares no gate of kind {kind}, so nothing verifies this requirement"
                ),
            )
        })
        .collect()
}

/// Every guard kind the project's Runbook declares.
fn declared_gate_kinds(root: &Path) -> BTreeSet<String> {
    // TRP-001: the contract gate asks the parser what the runbook declares.
    let Ok(class) = crate::machine::MachineClass::load_from_project_root(root) else {
        return BTreeSet::new();
    };
    class
        .phases()
        .values()
        .flat_map(|phase| phase.guards())
        .map(|guard| guard.name().to_owned())
        .collect()
}

fn contract_roots(workspace: &Path, roles: &[&str]) -> Result<Vec<PathBuf>, Vec<ContractDefect>> {
    let engine = crate::root::resolve(workspace);
    let class =
        crate::machine::MachineClass::load_from_project_root(workspace).map_err(|error| {
            vec![defect(
                "runbook",
                format!("{}: {}", error.code(), error.message()),
            )]
        })?;
    let roots = class
        .validate_roots(engine.invoking_checkout_root(), engine.engine_root())
        .map_err(|error| vec![defect("runbook", error.to_string())])?;
    roles
        .iter()
        .map(|role| {
            roots
                .resolve(role)
                .map_err(|error| vec![defect("runbook", error.to_string())])
        })
        .collect()
}

fn shown(path: &Path) -> String {
    crate::root::displayed(path)
}

/// Issue bundles across intake, deferred, and archive.
fn issue_folders(issue_root: &Path) -> Vec<(String, PathBuf, IssueLocation)> {
    let locations = [
        (issue_root.to_path_buf(), IssueLocation::Intake),
        (issue_root.join("deferred"), IssueLocation::Deferred),
        (issue_root.join("archive"), IssueLocation::Archive),
    ];
    let mut folders = Vec::new();
    for (directory, location) in locations {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
            else {
                continue;
            };
            if name.starts_with("i-") {
                folders.push((name, path, location));
            }
        }
    }
    folders.sort_by(|left, right| left.1.cmp(&right.1));
    folders
}

fn files_in(dir: PathBuf, extension: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file()
                        && path.extension().and_then(|ext| ext.to_str()) == Some(extension)
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Requirement IDs stated in a goal or issue table: `| ID | ... |`.
fn requirement_ids(text: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let first = trimmed
            .trim_start_matches('|')
            .split('|')
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches('`')
            .to_owned();
        if is_requirement_id(&first) {
            ids.insert(first);
        }
    }
    ids
}

/// Requirement IDs of issue rows carrying one exact disposition cell.
fn requirements_with_disposition(spec: &str, wanted: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for line in spec.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = trimmed
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().trim_matches('`'))
            .collect();
        let Some(first) = cells.first() else {
            continue;
        };
        if !is_requirement_id(first) {
            continue;
        }
        let disposition = cells
            .iter()
            .skip(1)
            .find(|cell| matches!(**cell, "accepted" | "rejected" | "duplicate" | "deferred"));
        if disposition.copied() == Some(wanted) {
            ids.insert((*first).to_owned());
        }
    }
    ids
}

/// `ABC-001` shaped: uppercase letters, a dash, digits.
fn is_requirement_id(text: &str) -> bool {
    let Some((prefix, number)) = text.split_once('-') else {
        return false;
    };
    !prefix.is_empty()
        && prefix
            .chars()
            .all(|character| character.is_ascii_uppercase())
        && !number.is_empty()
        && number.chars().all(|character| character.is_ascii_digit())
}

/// Relative markdown link targets, without anchors.
fn markdown_targets(text: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let bytes: Vec<char> = text.chars().collect();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == ']' && bytes.get(index + 1) == Some(&'(') {
            let mut end = index + 2;
            while end < bytes.len() && bytes[end] != ')' {
                end += 1;
            }
            if end < bytes.len() {
                let target: String = bytes[index + 2..end].iter().collect();
                if !target.starts_with("http") && !target.starts_with('#') {
                    targets.push(
                        target
                            .split('#')
                            .next()
                            .unwrap_or_default()
                            .trim()
                            .to_owned(),
                    );
                }
                index = end;
            }
        }
        index += 1;
    }
    targets
}

fn resolves(base: &Path, target: &str) -> bool {
    if target.is_empty() {
        return true;
    }
    base.join(target).exists()
}

/// Read `key: "value"` from a record's YAML block.
fn yaml_field(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        let rest = trimmed.strip_prefix(key)?.strip_prefix(':')?;
        Some(rest.trim().trim_matches('"').to_owned())
    })
}

/// Read a `key:` block list of `- "value"` entries.
fn yaml_list(text: &str, key: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim_start().starts_with(&format!("{key}:")) {
            inside = true;
            continue;
        }
        if inside {
            let item = trimmed.trim();
            if let Some(entry) = item.strip_prefix("- ") {
                values.push(entry.trim().trim_matches('"').to_owned());
            } else if item.is_empty() {
                continue;
            } else {
                break;
            }
        }
    }
    values
}

/// Every dependency cycle, each reported as the ring of tickets involved.
fn find_cycles(graph: &BTreeMap<String, Vec<String>>) -> Vec<Vec<String>> {
    let mut cycles: Vec<Vec<String>> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for start in graph.keys() {
        if seen.contains(start) {
            continue;
        }
        let mut stack: Vec<String> = Vec::new();
        if let Some(cycle) = walk(graph, start, &mut stack, &mut BTreeSet::new()) {
            for node in &cycle {
                seen.insert(node.clone());
            }
            let mut normalized = cycle.clone();
            normalized.sort();
            if !cycles.iter().any(|existing| {
                let mut other = existing.clone();
                other.sort();
                other == normalized
            }) {
                cycles.push(cycle);
            }
        }
    }
    cycles
}

fn walk(
    graph: &BTreeMap<String, Vec<String>>,
    node: &str,
    stack: &mut Vec<String>,
    visited: &mut BTreeSet<String>,
) -> Option<Vec<String>> {
    if let Some(position) = stack.iter().position(|entry| entry == node) {
        let mut ring = stack[position..].to_vec();
        ring.push(node.to_owned());
        return Some(ring);
    }
    if !visited.insert(node.to_owned()) {
        return None;
    }
    stack.push(node.to_owned());
    for next in graph.get(node).cloned().unwrap_or_default() {
        if let Some(cycle) = walk(graph, &next, stack, visited) {
            return Some(cycle);
        }
    }
    stack.pop();
    None
}
