//! PT-038-01: pre-cutover acceptance runbook for the external identity.
//!
//! This test is deliberately expected-red until the operator performs the
//! authenticated GitHub rename, exact origin update, and checkout move.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const OWNER: &str = "egoisth777";
const OLD_SLUG: &str = concat!("egoisth777/", "arca", "-scheduler");
const TARGET_SLUG: &str = "egoisth777/ratmac";
const OLD_ORIGIN: &str = concat!("git@github.com:egoisth777/", "arca", "-scheduler.git");
const TARGET_ORIGIN: &str = "git@github.com:egoisth777/ratmac.git";
const TARGET_PATH: &str = "E:/repos/projs/skill-dev/ratmac";
const LEGACY_TOKENS: [&str; 3] = [
    OLD_SLUG,
    OLD_ORIGIN,
    concat!("E:/repos/projs/skill-dev/", "arca", "-scheduler"),
];

fn repo_root() -> PathBuf {
    std::env::var_os("RATMAC_ACCEPTANCE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .canonicalize()
                .expect("workspace root must resolve")
        })
}

fn run(root: &Path, program: &str, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .unwrap_or_else(|error| panic!("{program} must execute: {error}"))
}

fn stdout(output: &Output, description: &str) -> String {
    assert!(
        output.status.success(),
        "{description} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).expect("command output must be UTF-8")
}

fn normalized(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn path_matches(pattern: &str, relative: &str) -> bool {
    pattern
        .strip_suffix("/**")
        .map_or(pattern == relative, |prefix| {
            relative == prefix || relative.starts_with(&format!("{prefix}/"))
        })
}

fn active_reference_audit(root: &Path) {
    let allowlist =
        fs::read_to_string(root.join("test/qa/fixtures/external-identity/allowlist.tsv"))
            .expect("external identity allowlist must be readable");
    let rules: Vec<_> = allowlist
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|line| {
            let fields: Vec<_> = line.splitn(3, '\t').collect();
            assert_eq!(
                fields.len(),
                3,
                "each allowlist row needs path, token, reason"
            );
            assert!(
                !fields[2].trim().is_empty(),
                "allowlist reasons must be explicit"
            );
            (
                fields[0].to_owned(),
                fields[1].to_owned(),
                fields[2].to_owned(),
            )
        })
        .collect();
    assert!(!rules.is_empty(), "allowlist must contain explicit rules");

    let tracked = stdout(
        &run(root, "git", &["ls-files", "-z"]),
        "tracked-file inventory",
    );
    let mut violations = Vec::new();
    for relative in tracked.split('\0').filter(|path| !path.is_empty()) {
        let path = root.join(relative);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (line_no, line) in text.lines().enumerate() {
            let Some(token) = LEGACY_TOKENS.iter().find(|token| line.contains(**token)) else {
                continue;
            };
            let allowed = rules.iter().any(|(pattern, allowed_token, _)| {
                path_matches(pattern, relative)
                    && (*allowed_token == "both" || allowed_token == *token)
            });
            if !allowed {
                violations.push(format!("{relative}:{}: {line}", line_no + 1));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "unallowlisted active external-identity references:\n{}",
        violations.join("\n")
    );
}

fn historical_bytes_unchanged(root: &Path) {
    let files = stdout(
        &run(
            root,
            "git",
            &[
                "ls-tree",
                "-r",
                "--name-only",
                "HEAD",
                ".arca/log.md",
                ".arca/issue",
                ".arca/ticket/archive",
            ],
        ),
        "historical-file inventory",
    );
    assert!(
        !files.trim().is_empty(),
        "historical allowlist must cover files"
    );
    for path in files.lines().filter(|path| !path.is_empty()) {
        let unchanged = run(root, "git", &["diff", "--quiet", "HEAD", "--", path]);
        assert!(
            unchanged.status.success(),
            "historical file changed in the worktree: {path}"
        );
    }
}

fn assert_exact_git_identity(root: &Path) {
    let origin = stdout(
        &run(root, "git", &["remote", "get-url", "origin"]),
        "origin check",
    );
    assert_eq!(origin.trim(), TARGET_ORIGIN);
    let config = stdout(
        &run(
            root,
            "git",
            &["config", "--local", "--get", "remote.origin.url"],
        ),
        ".git/config origin check",
    );
    assert_eq!(config.trim(), TARGET_ORIGIN);
    let top = stdout(
        &run(root, "git", &["rev-parse", "--show-toplevel"]),
        "top-level check",
    );
    assert_eq!(normalized(Path::new(top.trim())), TARGET_PATH);
    assert_eq!(
        Path::new(top.trim())
            .file_name()
            .and_then(|name| name.to_str()),
        Some("ratmac")
    );
}

#[test]
fn external_identity_acceptance() {
    let root = repo_root();

    // Check the real API and gh view before trusting any tracked label.
    let api = stdout(
        &run(
            root.as_path(),
            "gh",
            &["api", &format!("repos/{TARGET_SLUG}")],
        ),
        "GitHub API target identity",
    );
    assert!(api.contains(&format!("\"full_name\":\"{TARGET_SLUG}\"")));
    let view = stdout(
        &run(
            root.as_path(),
            "gh",
            &["repo", "view", TARGET_SLUG, "--json", "nameWithOwner"],
        ),
        "gh repo view target identity",
    );
    assert!(view.contains(TARGET_SLUG));

    assert_exact_git_identity(&root);
    active_reference_audit(&root);
    historical_bytes_unchanged(&root);

    let diff_check = run(&root, "git", &["diff", "--check"]);
    assert!(diff_check.status.success(), "git diff --check failed");
    let status = stdout(
        &run(&root, "git", &["status", "--porcelain"]),
        "final clean-state check",
    );
    assert!(
        status.trim().is_empty(),
        "final Git state is not clean: {status}"
    );

    // Keep these captures in the executable assertion surface so changing the
    // pre-cutover contract cannot silently weaken the test.
    assert_ne!(OLD_SLUG, TARGET_SLUG);
    assert_ne!(OLD_ORIGIN, TARGET_ORIGIN);
    assert_eq!(OWNER, TARGET_SLUG.split('/').next().unwrap());
}
