//! PT-034-01: active stale-name audit with an explicit historical allowlist.

use std::fs;
use std::path::{Path, PathBuf};

const LEGACY_PRODUCT: &str = concat!("arca", "-scheduler");
const LEGACY_COMMAND: &str = concat!("sc", "hd");
const ALLOWLIST: &str = "fixtures/rebrand-audit/allowlist.tsv";

#[derive(Debug)]
struct AllowRule {
    pattern: String,
    token: String,
    reason: String,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root must resolve")
}

fn load_allowlist(root: &Path) -> Vec<AllowRule> {
    let path = root.join("test/qa").join(ALLOWLIST);
    fs::read_to_string(path)
        .expect("rebrand audit allowlist must be readable")
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|(line_no, line)| {
            let fields: Vec<_> = line.splitn(3, '\t').collect();
            assert_eq!(
                fields.len(),
                3,
                "allowlist line {} must have three fields",
                line_no + 1
            );
            assert!(
                fields[1] == LEGACY_PRODUCT || fields[1] == LEGACY_COMMAND || fields[1] == "both",
                "allowlist line {} has an invalid token",
                line_no + 1
            );
            assert!(
                !fields[2].trim().is_empty(),
                "allowlist line {} needs a reason",
                line_no + 1
            );
            AllowRule {
                pattern: fields[0].to_owned(),
                token: fields[1].to_owned(),
                reason: fields[2].to_owned(),
            }
        })
        .collect()
}

fn path_matches(pattern: &str, relative: &str) -> bool {
    pattern
        .strip_suffix("/**")
        .map_or(pattern == relative, |prefix| {
            relative == prefix || relative.starts_with(&format!("{prefix}/"))
        })
}

fn rule_matches(rule: &AllowRule, relative: &str, product: bool, command: bool) -> bool {
    if !path_matches(&rule.pattern, relative) {
        return false;
    }
    match rule.token.as_str() {
        token if token == LEGACY_PRODUCT => product,
        token if token == LEGACY_COMMAND => command,
        "both" => product || command,
        _ => false,
    }
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if matches!(name, ".git" | ".arca-private" | "target") {
        return;
    }
    let metadata = fs::symlink_metadata(path).expect("audit path must be readable");
    if metadata.is_dir() {
        for entry in fs::read_dir(path).expect("audit directory must be readable") {
            collect_files(&entry.expect("audit entry must be readable").path(), files);
        }
    } else if metadata.is_file() {
        files.push(path.to_owned());
    }
}

#[test]
fn active_reference_audit() {
    let root = repo_root();
    let rules = load_allowlist(&root);
    assert!(!rules.is_empty(), "the audit allowlist must not be empty");
    let mut used = vec![false; rules.len()];
    let mut files = Vec::new();
    collect_files(&root, &mut files);
    let mut violations = Vec::new();

    for path in files {
        let relative = path
            .strip_prefix(&root)
            .expect("audit file must be under repository root")
            .to_string_lossy()
            .replace('\\', "/");
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (line_no, line) in text.lines().enumerate() {
            let product = line.contains(LEGACY_PRODUCT);
            let command = line.contains(LEGACY_COMMAND);
            if !product && !command {
                continue;
            }
            let mut allowed = false;
            for (index, rule) in rules.iter().enumerate() {
                if rule_matches(rule, &relative, product, command) {
                    used[index] = true;
                    allowed = true;
                }
            }
            if !allowed {
                violations.push(format!("{relative}:{}: {line}", line_no + 1));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "unallowlisted active legacy references:\n{}",
        violations.join("\n")
    );
    let unused: Vec<_> = rules
        .iter()
        .enumerate()
        .filter(|(index, _)| !used[*index])
        .map(|(_, rule)| format!("{} ({})", rule.pattern, rule.reason))
        .collect();
    assert!(
        unused.is_empty(),
        "stale allowlist entries:\n{}",
        unused.join("\n")
    );

    // These files are historical inputs to the audit, not outputs it may rewrite.
    let log = root.join(".arca/log.md");
    let archive = root.join(".arca/ticket/archive");
    assert!(
        log.is_file(),
        "append-only transition log must remain present"
    );
    assert!(
        archive.is_dir(),
        "archived ticket history must remain present"
    );
}
