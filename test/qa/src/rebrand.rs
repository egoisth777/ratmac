//! SVC-008: one indexed input selection and one allowlist for retired spellings.
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
    /// Explicit caller-selected files, separate from content exemptions.
    pub extra_inputs: Vec<String>,
    /// Named repository boundaries whose contents were not traversed.
    pub exclusions: Vec<String>,
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

/// Ordinary indexed working-file paths. Use `audit_files::select` for a full
/// inventory including link text and Gitlink boundaries, without dereferencing
/// those links. A failed selection never returns an incomplete file list.
pub fn collect_files(root: &Path) -> Vec<PathBuf> {
    crate::audit_files::select(root, &[])
        .unwrap_or_else(|error| panic!("audit input selection failed: {error}"))
        .entries
        .into_iter()
        .filter(|entry| entry.kind == crate::audit_files::EntryKind::File)
        .map(|entry| root.join(entry.path))
        .collect()
}

/// Select the index at `root` and report every unallowlisted occurrence and row that
/// matched nothing. A path that spells a retired name is a carrier too, so
/// renaming a file cannot smuggle one past the walk. The walk only reads.
pub fn audit(root: &Path, rules: &[Rule]) -> Report {
    audit_with_extras(root, rules, &[])
}

/// Audit the index and exactly the additional files the caller declares.
pub fn audit_with_extras(root: &Path, rules: &[Rule], extras: &[PathBuf]) -> Report {
    let mut used = vec![false; rules.len()];
    let mut report = Report::default();
    let selected = match crate::audit_files::select(root, extras) {
        Ok(selected) => selected,
        Err(error) => {
            report.violations.push(error);
            return report;
        }
    };
    report.extra_inputs = selected.extra_inputs;
    report.exclusions = selected.exclusions;
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
    for entry in selected.entries {
        let relative = entry
            .path
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        let named = hits(&relative);
        if named.any() && !allow(&relative, named, &mut used) {
            report.violations.push(format!(
                "{relative}: the path itself names a retired spelling"
            ));
        }
        if entry.kind == crate::audit_files::EntryKind::Gitlink {
            continue;
        }
        let text = std::str::from_utf8(&entry.bytes)
            .ok()
            .filter(|_| !entry.bytes.contains(&0));
        let Some(text) = text else {
            for (token, position) in [
                (LEGACY_PRODUCT, false),
                (LEGACY_COMMAND, false),
                (PRE_CUTOVER_POSITION, true),
            ] {
                for (offset, window) in entry.bytes.windows(token.len()).enumerate() {
                    if !(if position {
                        window.eq_ignore_ascii_case(token.as_bytes())
                    } else {
                        window == token.as_bytes()
                    }) {
                        continue;
                    }
                    let found = Hits {
                        product: token == LEGACY_PRODUCT,
                        command: token == LEGACY_COMMAND,
                        position,
                    };
                    if !allow(&relative, found, &mut used) {
                        report.violations.push(format!(
                            "{relative}:byte {offset}: retired spelling {token:?}"
                        ));
                    }
                }
            }
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
