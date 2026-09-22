//! One walk over the repository's package manifests, resolving every build
//! target to the file it writes.
//!
//! `DEB-002` asks the shop to state the no-collision rule in its own words
//! rather than waiting for a toolchain warning, so this module reads the
//! declarations - explicit `[[bin]]` and `[lib]` tables plus the targets cargo
//! discovers from `src/main.rs` and `src/bin/` - and reports two targets that
//! would write one file. Targets in different workspaces write into different
//! build directories and never collide.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The two target kinds whose output files share one directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// A command a person can run.
    Bin,
    /// A library other code links.
    Lib,
}

impl Kind {
    /// The word a report uses for this kind.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Bin => "bin",
            Kind::Lib => "lib",
        }
    }
}

/// One declared or discovered build target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The package that declares it.
    pub package: String,
    /// The target name, which decides the output file name.
    pub name: String,
    /// Command or library.
    pub kind: Kind,
    /// Repository-relative manifest path that carries the declaration.
    pub manifest: String,
    /// Repository-relative directory of the workspace whose build directory it
    /// writes into. The repository root is `.`.
    pub workspace: String,
    /// Where the target's code comes from: a repository-relative path, or
    /// `declared` when the manifest leaves the path to cargo's convention.
    pub source: String,
}

impl Target {
    /// The build directory entry two colliding targets would fight over.
    pub fn output(&self) -> String {
        let root = if self.workspace == "." {
            String::new()
        } else {
            format!("{}/", self.workspace)
        };
        match self.kind {
            Kind::Bin => format!("{root}target/<profile>/{}", self.name),
            Kind::Lib => format!("{root}target/<profile>/lib{}.rlib", self.name),
        }
    }

    /// How a failure names this declaration.
    pub fn described(&self) -> String {
        format!(
            "{} target `{}` in package `{}` ({}, source {})",
            self.kind.as_str(),
            self.name,
            self.package,
            self.manifest,
            self.source
        )
    }
}

/// What one walk found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Every target the walk resolved, ordered by workspace, kind, then name.
    pub targets: Vec<Target>,
    /// One line per pair of targets that would write the same file.
    pub collisions: Vec<String>,
    pub extra_inputs: Vec<String>,
    pub exclusions: Vec<String>,
}

impl Report {
    /// True when no two targets write one file.
    pub fn is_clean(&self) -> bool {
        self.collisions.is_empty()
    }
}

/// Read the repository index and report every selected build-output collision.
///
/// Untracked directories are never discovered implicitly. A selected manifest
/// that cannot be read or parsed is an error, never a silent skip. Owned
/// synthetic trees use `audit_fixture`; ignored private inputs are explicit extras.
pub fn audit(root: &Path) -> Result<Report, String> {
    audit_with_extras(root, &[])
}

/// Audit indexed manifests and implicit target paths, plus exact extra files.
pub fn audit_with_extras(root: &Path, extras: &[PathBuf]) -> Result<Report, String> {
    let selected = crate::audit_files::select(root, extras)?;
    for entry in &selected.entries {
        if entry.kind != crate::audit_files::EntryKind::Symlink {
            continue;
        }
        let path = &entry.path;
        let parent = path.parent().unwrap_or(Path::new(""));
        let direct_bin =
            parent.ends_with("src/bin") && path.extension().is_some_and(|ext| ext == "rs");
        let nested_bin = parent.parent().is_some_and(|dir| dir.ends_with("src/bin"))
            && path.file_name().is_some_and(|name| name == "main.rs");
        if path.file_name().is_some_and(|name| name == "Cargo.toml")
            || path.ends_with("src/lib.rs")
            || path.ends_with("src/main.rs")
            || direct_bin
            || nested_bin
        {
            return Err(format!("{}: indexed build input is a symlink; target audit inspects link text and refuses to dereference it", path.display()));
        }
    }
    let files = selected
        .entries
        .into_iter()
        .filter(|entry| entry.kind == crate::audit_files::EntryKind::File)
        .map(|entry| (root.join(entry.path), entry.bytes))
        .collect::<BTreeMap<_, _>>();
    let manifests = files
        .keys()
        .filter(|path| path.file_name().is_some_and(|name| name == "Cargo.toml"))
        .cloned()
        .collect();
    let mut report = report_targets(collect_from(root, manifests, Some(&files))?);
    report.extra_inputs = selected.extra_inputs;
    report.exclusions = selected.exclusions;
    Ok(report)
}

/// Inspect an explicitly owned synthetic fixture tree, including untracked files.
pub fn audit_fixture(root: &Path) -> Result<Report, String> {
    Ok(report_targets(collect_from(
        root,
        fixture_manifests(root)?,
        None,
    )?))
}

/// Exact extra build inputs requested by this repository's private-lane
/// declaration. The owning ticket must extend the explicit bin list when a
/// new private source becomes relevant; ignored directories are never expanded.
pub fn declared_lane_inputs(root: &Path) -> Result<Vec<PathBuf>, String> {
    let selected = crate::audit_files::select(root, &[])?;
    let declaration = selected
        .entries
        .iter()
        .find(|entry| entry.path == Path::new(".ratmac/lanes.toml"))
        .ok_or_else(|| "private target coverage needs indexed .ratmac/lanes.toml".to_owned())?;
    if declaration.kind != crate::audit_files::EntryKind::File {
        return Err(".ratmac/lanes.toml must be a regular declaration file".to_owned());
    }
    let text = std::str::from_utf8(&declaration.bytes)
        .map_err(|error| format!("lane declaration: {error}"))?;
    let value: toml::Value = text
        .parse()
        .map_err(|error| format!("lane declaration: {error}"))?;
    let roots = value
        .get("roots")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| "lane declaration has no roots table".to_owned())?;
    let lanes = roots
        .get("lanes")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| "lane declaration has no lanes path".to_owned())?;
    let lanes = Path::new(lanes);
    if lanes.as_os_str().is_empty()
        || lanes
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err("declared private lanes path must stay beneath the repository".to_owned());
    }
    let roster = roots
        .get("roster")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| "lane declaration has no roster".to_owned())?;
    let bins = [
        ("t-070", "src/bin/rtm.rs"),
        ("t-073", "src/bin/hold_racer.rs"),
        ("t-073", "src/bin/abandon_racer.rs"),
        ("t-075", "src/bin/race_step.rs"),
        ("t-104", "src/bin/skill_probe.rs"),
        ("t-104", "src/bin/skill_probe_variant.rs"),
    ];
    let mut files = Vec::new();
    for id in roster {
        let id = id
            .as_str()
            .ok_or_else(|| "lane roster id must be text".to_owned())?;
        if id.len() != 5
            || !id.starts_with("t-")
            || !id[2..].bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(format!("invalid declared private lane {id:?}"));
        }
        let crate_root = lanes.join(id);
        files.push(crate_root.join("Cargo.toml"));
        files.push(crate_root.join("src/lib.rs"));
        for (owner, source) in bins {
            if owner == id {
                files.push(crate_root.join(source));
            }
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn report_targets(mut targets: Vec<Target>) -> Report {
    targets.sort_by(|left, right| {
        (
            &left.workspace,
            left.kind,
            &left.name,
            &left.package,
            &left.manifest,
        )
            .cmp(&(
                &right.workspace,
                right.kind,
                &right.name,
                &right.package,
                &right.manifest,
            ))
    });

    let mut seen: BTreeMap<(String, Kind, String), Target> = BTreeMap::new();
    let mut collisions = Vec::new();
    for target in &targets {
        let key = (target.workspace.clone(), target.kind, target.name.clone());
        match seen.get(&key) {
            None => {
                seen.insert(key, target.clone());
            }
            Some(first) => collisions.push(format!(
                "workspace `{}`: {} and {} both write {}",
                target.workspace,
                first.described(),
                target.described(),
                target.output()
            )),
        }
    }

    Report {
        targets,
        collisions,
        ..Report::default()
    }
}

/// Indexed regular manifests. A linked manifest refuses rather than silently
/// omitting a package or reading through a link; Gitlinks remain boundaries.
pub fn manifests(root: &Path) -> Result<Vec<PathBuf>, String> {
    let selected = crate::audit_files::select(root, &[])?;
    if let Some(entry) = selected.entries.iter().find(|entry| {
        entry.kind == crate::audit_files::EntryKind::Symlink
            && entry
                .path
                .file_name()
                .is_some_and(|name| name == "Cargo.toml")
    }) {
        return Err(format!(
            "{}: indexed manifest is a symlink; refusing to dereference it",
            entry.path.display()
        ));
    }
    Ok(selected
        .entries
        .into_iter()
        .filter(|entry| {
            entry.kind == crate::audit_files::EntryKind::File
                && entry
                    .path
                    .file_name()
                    .is_some_and(|name| name == "Cargo.toml")
        })
        .map(|entry| root.join(entry.path))
        .collect())
}

fn fixture_manifests(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    walk(root, &mut found)?;
    found.sort();
    Ok(found)
}

fn walk(dir: &Path, found: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|error| format!("read {}: {error}", dir.display()))?;
    let mut children = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("enumerate {}: {error}", dir.display()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry
            .file_type()
            .map_err(|error| format!("inspect {}: {error}", path.display()))?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            if name == "target" || name == ".git" {
                continue;
            }
            children.push(path);
        } else if name == "Cargo.toml" {
            found.push(path);
        }
    }
    children.sort();
    for child in children {
        walk(&child, found)?;
    }
    Ok(())
}

/// Resolve every target declared or discovered in indexed repository inputs.
pub fn collect(root: &Path) -> Result<Vec<Target>, String> {
    Ok(audit(root)?.targets)
}

fn collect_from(
    root: &Path,
    manifest_paths: Vec<PathBuf>,
    selected: Option<&BTreeMap<PathBuf, Vec<u8>>>,
) -> Result<Vec<Target>, String> {
    let workspace_roots: Vec<PathBuf> = manifest_paths
        .iter()
        .filter(|path| match read_manifest(path, selected) {
            Ok(document) => document.get("workspace").is_some(),
            Err(_) => false,
        })
        .filter_map(|path| path.parent().map(Path::to_path_buf))
        .collect();

    let mut targets = Vec::new();
    for manifest in &manifest_paths {
        let document = read_manifest(manifest, selected)?;
        let Some(package) = document.get("package").and_then(toml::Value::as_table) else {
            continue;
        };
        let name = package
            .get("name")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("{}: package has no name", relative(root, manifest)))?
            .to_owned();
        let dir = manifest
            .parent()
            .ok_or_else(|| format!("{}: manifest has no directory", manifest.display()))?;
        let workspace = workspace_of(dir, &workspace_roots);
        let workspace = relative(root, &workspace);
        let shown = relative(root, manifest);

        for target in package_targets(&document, &name, dir, root, selected)? {
            targets.push(Target {
                package: name.clone(),
                workspace: workspace.clone(),
                manifest: shown.clone(),
                ..target
            });
        }
    }
    Ok(targets)
}

fn read_manifest(
    path: &Path,
    selected: Option<&BTreeMap<PathBuf, Vec<u8>>>,
) -> Result<toml::Value, String> {
    let text = match selected {
        Some(files) => String::from_utf8(
            files
                .get(path)
                .ok_or_else(|| format!("manifest {} was not selected", path.display()))?
                .clone(),
        )
        .map_err(|error| format!("read {}: {error}", path.display()))?,
        None => {
            fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?
        }
    };
    text.parse::<toml::Value>()
        .map_err(|error| format!("parse {}: {error}", path.display()))
}

/// The nearest ancestor workspace root, or the package's own directory.
fn workspace_of(dir: &Path, roots: &[PathBuf]) -> PathBuf {
    let mut best: Option<&PathBuf> = None;
    for root in roots {
        if dir.starts_with(root) && best.is_none_or(|current| root.starts_with(current)) {
            best = Some(root);
        }
    }
    best.cloned().unwrap_or_else(|| dir.to_path_buf())
}

/// The commands and library one package produces.
fn package_targets(
    document: &toml::Value,
    package: &str,
    dir: &Path,
    root: &Path,
    selected: Option<&BTreeMap<PathBuf, Vec<u8>>>,
) -> Result<Vec<Target>, String> {
    let mut targets: Vec<Target> = Vec::new();
    let blank = |name: String, kind: Kind, source: String| Target {
        package: package.to_owned(),
        name,
        kind,
        manifest: String::new(),
        workspace: String::new(),
        source,
    };

    if let Some(library) = document.get("lib").and_then(toml::Value::as_table) {
        let name = library
            .get("name")
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| package.replace('-', "_"));
        let source = library
            .get("path")
            .and_then(toml::Value::as_str)
            .map(|path| relative(root, &dir.join(path)))
            .unwrap_or_else(|| relative(root, &dir.join("src/lib.rs")));
        targets.push(blank(name, Kind::Lib, source));
    } else if selected.map_or_else(
        || dir.join("src/lib.rs").is_file(),
        |files| files.contains_key(&dir.join("src/lib.rs")),
    ) {
        targets.push(blank(
            package.replace('-', "_"),
            Kind::Lib,
            relative(root, &dir.join("src/lib.rs")),
        ));
    }

    if let Some(bins) = document.get("bin").and_then(toml::Value::as_array) {
        for entry in bins {
            let table = entry
                .as_table()
                .ok_or_else(|| format!("{}: a [[bin]] entry is not a table", package))?;
            let name = table
                .get("name")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| format!("{}: a [[bin]] entry has no name", package))?
                .to_owned();
            let source = table
                .get("path")
                .and_then(toml::Value::as_str)
                .map(|path| relative(root, &dir.join(path)))
                .unwrap_or_else(|| "declared".to_owned());
            targets.push(blank(name, Kind::Bin, source));
        }
    }

    let autobins = document
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get("autobins"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    if autobins {
        for (name, source) in discovered_bins(dir, package, selected)? {
            if targets
                .iter()
                .any(|target| target.kind == Kind::Bin && target.name == name)
            {
                continue;
            }
            targets.push(blank(name, Kind::Bin, relative(root, &source)));
        }
    }

    Ok(targets)
}

/// The commands cargo finds without a manifest entry.
fn discovered_bins(
    dir: &Path,
    package: &str,
    selected: Option<&BTreeMap<PathBuf, Vec<u8>>>,
) -> Result<Vec<(String, PathBuf)>, String> {
    let mut found = Vec::new();
    let main = dir.join("src/main.rs");
    if let Some(files) = selected {
        if files.contains_key(&main) {
            found.push((package.to_owned(), main));
        }
        let bins = dir.join("src/bin");
        for path in files.keys() {
            let Ok(relative) = path.strip_prefix(&bins) else {
                continue;
            };
            if relative.components().count() == 1
                && relative.extension().is_some_and(|ext| ext == "rs")
            {
                found.push((
                    relative.file_stem().unwrap().to_string_lossy().into_owned(),
                    path.clone(),
                ));
            } else if relative.components().count() == 2
                && relative.file_name().is_some_and(|name| name == "main.rs")
            {
                found.push((
                    relative.parent().unwrap().to_string_lossy().into_owned(),
                    path.clone(),
                ));
            }
        }
        found.sort();
        return Ok(found);
    }
    if main.is_file() {
        found.push((package.to_owned(), main));
    }
    let bin_dir = dir.join("src/bin");
    if bin_dir.is_dir() {
        let entries = fs::read_dir(&bin_dir)
            .map_err(|error| format!("read {}: {error}", bin_dir.display()))?;
        for entry in entries {
            let entry =
                entry.map_err(|error| format!("enumerate {}: {error}", bin_dir.display()))?;
            let path = entry.path();
            let kind = entry
                .file_type()
                .map_err(|error| format!("inspect {}: {error}", path.display()))?;
            if kind.is_dir() {
                let nested = path.join("main.rs");
                if nested.is_file() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    found.push((name, nested));
                }
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                let name = path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_default();
                found.push((name, path));
            }
        }
    }
    found.sort();
    Ok(found)
}

/// A repository-relative path with forward slashes; `.` for the root itself.
fn relative(root: &Path, path: &Path) -> String {
    let shown = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    if shown.is_empty() {
        ".".to_owned()
    } else {
        shown
    }
}
