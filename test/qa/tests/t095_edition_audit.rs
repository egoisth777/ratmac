//! t-095 / EDN-001: every edition is annotated, documents the bar, and stays reachable.
//!
//! `EDNV-004` `every_edition_is_annotated_documented_and_reachable`
//!
//! The closing guard `t-094` landed proves a tag named `edition-*` points at the
//! commit being left. It cannot tell an annotated tag from a lightweight one and
//! cannot read the message - `EDN-002` says so in its own stated limit. This is
//! the check that reads them: an edition cut wrong is found by a machine, not by
//! a person following a citation that no longer means anything.
//!
//! The audit never creates, moves, or deletes a tag, and it re-reads the
//! repository every run rather than trusting a frozen list.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The pattern the closing guard uses, so both halves agree on what a candidate
/// edition is. A name that only looks like one must be reported, not skipped.
pub const EDITION_PREFIX: &str = "edition-";

/// The phrases an edition's message must carry to record the bar `EDN-001`
/// names. Each is the plain command or check a reader can re-run at that commit.
const RECORDED_BAR: [&str; 6] = [
    "Proven at this commit:",
    "cargo test",
    "cargo fmt",
    "cargo clippy",
    "check_links.py",
    "rtm doctor",
];

/// One thing wrong with one tag, named so a reader knows what to fix.
#[derive(Debug, PartialEq, Eq)]
pub struct AuditFinding {
    pub tag: String,
    pub property: String,
    pub detail: String,
}

impl std::fmt::Display for AuditFinding {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}: {}", self.tag, self.property, self.detail)
    }
}

/// Run a read-only version-control command, or say why the answer is unavailable
/// rather than guessing one.
pub fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output: Output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("git {args:?} could not run: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {args:?} exited {}: {}",
            output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "by signal".to_owned()),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Every tag whose name begins with the edition prefix, in name order. A
/// near-miss like `editions-002` is deliberately not a candidate by prefix, so
/// the Cross-Feature lane checks it is still reported by the sequence half.
pub fn edition_tags(root: &Path) -> Result<Vec<String>, String> {
    let listed = git(root, &["tag", "--list", &format!("{EDITION_PREFIX}*")])?;
    let mut tags: Vec<String> = listed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    tags.sort();
    Ok(tags)
}

/// Audit the edition tags of one repository. An empty tag set is a stated pass:
/// a repository with no editions has cut none wrong.
pub fn audit_editions(root: &Path) -> Result<Vec<AuditFinding>, String> {
    let mut findings = Vec::new();
    for tag in edition_tags(root)? {
        let kind = git(root, &["cat-file", "-t", &tag])?.trim().to_owned();
        if kind != "tag" {
            findings.push(AuditFinding {
                tag: tag.clone(),
                property: "annotated".to_owned(),
                detail: format!("the tag object is a {kind}, so nothing records what was proven"),
            });
            // A lightweight tag has no message and no tagger: the remaining
            // message checks would only repeat this one finding.
            continue;
        }

        let message = git(root, &["tag", "--list", "--format=%(contents)", &tag])?;
        let missing: Vec<&str> = RECORDED_BAR
            .iter()
            .copied()
            .filter(|phrase| !message.contains(phrase))
            .collect();
        if message.trim().is_empty() {
            findings.push(AuditFinding {
                tag: tag.clone(),
                property: "documented".to_owned(),
                detail: "the message is empty, so the bar is recorded nowhere".to_owned(),
            });
        } else if !missing.is_empty() {
            findings.push(AuditFinding {
                tag: tag.clone(),
                property: "documented".to_owned(),
                detail: format!("the message does not record {}", missing.join(", ")),
            });
        }

        // Reachable means: the commit this edition marks is an ancestor of the
        // trunk, so a citation to it resolves in a fresh clone that has main.
        let commit = git(root, &["rev-list", "-n", "1", &tag])?.trim().to_owned();
        let trunk = trunk_name(root)?;
        let reachable = Command::new("git")
            .args(["merge-base", "--is-ancestor", &commit, &trunk])
            .current_dir(root)
            .output()
            .map_err(|error| format!("git merge-base could not run: {error}"))?;
        if !reachable.status.success() {
            findings.push(AuditFinding {
                tag: tag.clone(),
                property: "reachable".to_owned(),
                detail: format!("commit {commit} is not an ancestor of {trunk}"),
            });
        }
    }
    findings.sort_by(|left, right| (&left.tag, &left.property).cmp(&(&right.tag, &right.property)));
    Ok(findings)
}

/// The line of development an edition must sit on. `main` where it exists, so
/// the answer does not depend on which branch a contributor happens to be on.
pub fn trunk_name(root: &Path) -> Result<String, String> {
    if git(root, &["rev-parse", "--verify", "--quiet", "main"]).is_ok() {
        return Ok("main".to_owned());
    }
    Err("the repository has no main branch, so reachability has no stated answer".to_owned())
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Render findings for a failure message: one per line, named.
pub fn report(findings: &[AuditFinding]) -> String {
    let mut text = String::new();
    for finding in findings {
        let _ = writeln!(text, "  {finding}");
    }
    text
}

/// `EDNV-004`: this repository's editions are all annotated, all record the bar,
/// and all sit on the trunk - and a planted defect of each kind is named.
#[test]
fn every_edition_is_annotated_documented_and_reachable() {
    let root = repo_root();
    let findings = audit_editions(&root).expect("audit this repository's editions");
    assert!(
        findings.is_empty(),
        "every edition of this repository is annotated, documented, and reachable:\n{}",
        report(&findings)
    );
    assert!(
        !edition_tags(&root)
            .expect("list this repository's editions")
            .is_empty(),
        "this repository has at least one edition, so the audit above proved something"
    );

    // Each planted defect is reported by name, in a repository of its own.
    let lightweight = Fixture::create("lightweight");
    lightweight.git(&["tag", "edition-001"]);
    let findings = audit_editions(&lightweight.root).expect("audit the lightweight edition");
    assert_eq!(
        findings,
        vec![AuditFinding {
            tag: "edition-001".to_owned(),
            property: "annotated".to_owned(),
            detail: "the tag object is a commit, so nothing records what was proven".to_owned(),
        }],
        "a lightweight edition is reported as not annotated"
    );

    let undocumented = Fixture::create("undocumented");
    undocumented.git(&["tag", "-a", "edition-001", "-m", "cut it"]);
    let findings = audit_editions(&undocumented.root).expect("audit the undocumented edition");
    assert_eq!(findings.len(), 1, "one finding: {}", report(&findings));
    assert_eq!(findings[0].property, "documented");
    assert!(
        findings[0].detail.contains("cargo test")
            && findings[0].detail.contains("rtm doctor")
            && findings[0].detail.contains("Proven at this commit:"),
        "the report names the phrases the message is missing: {}",
        findings[0].detail
    );

    let unreachable = Fixture::create("unreachable");
    unreachable.git(&["checkout", "--quiet", "-b", "sidetrack"]);
    std::fs::write(unreachable.root.join("src/lib.rs"), "pub fn aside() {}\n")
        .expect("write on the sidetrack");
    unreachable.commit("work that never merged");
    unreachable.git(&["tag", "-a", "edition-001", "-m", BAR_MESSAGE]);
    unreachable.git(&["checkout", "--quiet", "main"]);
    let findings = audit_editions(&unreachable.root).expect("audit the unreachable edition");
    assert_eq!(findings.len(), 1, "one finding: {}", report(&findings));
    assert_eq!(findings[0].property, "reachable");
    assert!(
        findings[0].detail.contains("not an ancestor of main"),
        "the report says what the commit is not reachable from: {}",
        findings[0].detail
    );
}

/// A message that records the bar the way `edition-001` does.
pub const BAR_MESSAGE: &str = "edition test\n\nProven at this commit:\n- cargo test --workspace green; cargo fmt --check clean; cargo clippy clean; tools/check_links.py clean\n- rtm doctor exits 0\n";

pub struct Fixture {
    pub root: PathBuf,
}

impl Fixture {
    pub fn create(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t095-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).expect("create fixture tree");
        std::fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").expect("write source");
        let fixture = Self { root };
        fixture.git(&["init", "--quiet", "--initial-branch=main"]);
        fixture.git(&["config", "user.email", "fixture@example.invalid"]);
        fixture.git(&["config", "user.name", "Fixture"]);
        fixture.git(&["config", "core.autocrlf", "false"]);
        fixture.commit("fixture base");
        fixture
    }

    pub fn git(&self, args: &[&str]) -> String {
        git(&self.root, args).unwrap_or_else(|error| panic!("fixture git {args:?}: {error}"))
    }

    pub fn commit(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "--quiet", "-m", message]);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
