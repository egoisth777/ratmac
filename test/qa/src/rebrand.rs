//! SVC-008: one walk and one enumerated allowlist for every retired spelling.
//!
//! The audit suite, the acceptance suite, and the state-vocabulary suite all
//! load this module, so a row added for one is honoured by the others in the
//! same run. Nothing here writes: it reads the tree and reports.

use std::fs;
use std::path::{Path, PathBuf};

/// The retired product name.
pub const LEGACY_PRODUCT: &str = concat!("arca", "-scheduler");

/// The retired command name.
pub const LEGACY_COMMAND: &str = concat!("sc", "hd");

/// The retired spelling of the machine position. Matched without regard to
/// case, so `Phase`, `phases`, and `PhasePrompt` are all caught.
pub const PRE_CUTOVER_POSITION: &str = concat!("ph", "ase");

/// The allowlist, relative to the `test/qa` crate root.
pub const ALLOWLIST: &str = "fixtures/rebrand-audit/allowlist.tsv";

/// Directory names the walk never descends into.
const SKIPPED: [&str; 3] = [".git", ".arca-private", "target"];

/// One enumerated carrier: a path pattern, the token it may carry, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub pattern: String,
    pub token: String,
    pub reason: String,
}

/// Which retired spellings one line carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hits {
    pub product: bool,
    pub command: bool,
    pub position: bool,
}

impl Hits {
    fn any(self) -> bool {
        self.product || self.command || self.position
    }
}

/// What one audit run found.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Report {
    /// `path:line: text` for every live occurrence no row allows.
    pub violations: Vec<String>,
    /// `pattern (reason)` for every row that matched nothing.
    pub stale: Vec<String>,
}

impl Report {
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty() && self.stale.is_empty()
    }
}

/// The tree this audit walks: the repository holding the `test/qa` crate,
/// unless `RATMAC_AUDIT_ROOT` points a lane at a throwaway copy of it. Both
/// suites resolve their root here, so one run always judges one tree.
pub fn repo_root() -> PathBuf {
    if let Some(root) = std::env::var_os("RATMAC_AUDIT_ROOT") {
        return PathBuf::from(root)
            .canonicalize()
            .expect("RATMAC_AUDIT_ROOT must name an existing directory");
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root must resolve")
}

/// The allowlist path inside `root`.
pub fn allowlist_path(root: &Path) -> PathBuf {
    root.join("test/qa").join(ALLOWLIST)
}

/// Load the enumerated allowlist. A row missing a field, carrying an unknown
/// token, or leaving its reason blank is an error, never a silent skip.
pub fn load_allowlist(path: &Path) -> Result<Vec<Rule>, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("read allowlist {}: {error}", path.display()))?;
    let mut rules = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let number = index + 1;
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.splitn(3, '\t').collect();
        if fields.len() != 3 {
            return Err(format!(
                "allowlist line {number} must have three tab-separated fields: path, token, reason"
            ));
        }
        if !matches!(
            fields[1],
            token if token == LEGACY_PRODUCT
                || token == LEGACY_COMMAND
                || token == PRE_CUTOVER_POSITION
                || token == "both"
        ) {
            return Err(format!(
                "allowlist line {number} names an unknown token {:?}",
                fields[1]
            ));
        }
        if fields[2].trim().is_empty() {
            return Err(format!("allowlist line {number} needs a reason"));
        }
        rules.push(Rule {
            pattern: fields[0].to_owned(),
            token: fields[1].to_owned(),
            reason: fields[2].to_owned(),
        });
    }
    Ok(rules)
}

/// Whether a `path` or `prefix/**` pattern covers this relative path.
pub fn path_matches(pattern: &str, relative: &str) -> bool {
    pattern
        .strip_suffix("/**")
        .map_or(pattern == relative, |prefix| {
            relative == prefix || relative.starts_with(&format!("{prefix}/"))
        })
}

fn rule_matches(rule: &Rule, relative: &str, hits: Hits) -> bool {
    if !path_matches(&rule.pattern, relative) {
        return false;
    }
    match rule.token.as_str() {
        token if token == LEGACY_PRODUCT => hits.product,
        token if token == LEGACY_COMMAND => hits.command,
        token if token == PRE_CUTOVER_POSITION => hits.position,
        "both" => hits.product || hits.command,
        _ => false,
    }
}

/// Which retired spellings this line carries.
pub fn hits(line: &str) -> Hits {
    Hits {
        product: line.contains(LEGACY_PRODUCT),
        command: line.contains(LEGACY_COMMAND),
        position: line.to_ascii_lowercase().contains(PRE_CUTOVER_POSITION),
    }
}

/// Every readable file below `root`, skipping runtime and private trees.
pub fn collect_files(root: &Path) -> Vec<PathBuf> {
    fn walk(path: &Path, files: &mut Vec<PathBuf>) {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if SKIPPED.contains(&name) {
            return;
        }
        let Ok(metadata) = fs::symlink_metadata(path) else {
            return;
        };
        if metadata.is_dir() {
            let Ok(entries) = fs::read_dir(path) else {
                return;
            };
            let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
            paths.sort();
            for entry in paths {
                walk(&entry, files);
            }
        } else if metadata.is_file() {
            files.push(path.to_owned());
        }
    }
    let mut files = Vec::new();
    walk(root, &mut files);
    files
}

/// Walk `root` and report every unallowlisted occurrence and every row that
/// matched nothing. A path that spells a retired name is a carrier too, so
/// renaming a file cannot smuggle one past the walk. The walk only reads.
pub fn audit(root: &Path, rules: &[Rule]) -> Report {
    let mut used = vec![false; rules.len()];
    let mut report = Report::default();
    let allow = |relative: &str, found: Hits, used: &mut Vec<bool>| {
        let mut allowed = false;
        for (position, rule) in rules.iter().enumerate() {
            if rule_matches(rule, relative, found) {
                used[position] = true;
                allowed = true;
            }
        }
        allowed
    };
    for path in collect_files(root) {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let named = hits(&relative);
        if named.any() && !allow(&relative, named, &mut used) {
            report.violations.push(format!(
                "{relative}: the path itself names a retired spelling"
            ));
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let found = hits(line);
            if !found.any() {
                continue;
            }
            if !allow(&relative, found, &mut used) {
                report
                    .violations
                    .push(format!("{relative}:{}: {}", index + 1, line.trim()));
            }
        }
    }
    report.stale = rules
        .iter()
        .zip(used)
        .filter(|(_, seen)| !seen)
        .map(|(rule, _)| format!("{} ({})", rule.pattern, rule.reason))
        .collect();
    report
}
