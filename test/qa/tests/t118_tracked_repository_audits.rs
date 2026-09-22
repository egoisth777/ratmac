//! WEB-003: repository-content audits select the index and inspect working bytes.

use ratmac_qa::rebrand::{self, Report, Rule, LEGACY_PRODUCT, PRE_CUTOVER_POSITION};
use ratmac_qa::tempgit::TempRepo;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug, PartialEq, Eq)]
enum Node {
    Directory,
    File(Vec<u8>),
    Link(PathBuf),
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Node> {
    fn visit(root: &Path, directory: &Path, entries: &mut BTreeMap<PathBuf, Node>) {
        for entry in fs::read_dir(directory).expect("fixture directory is readable") {
            let entry = entry.expect("fixture entry is readable");
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            let kind = entry.file_type().unwrap();
            if kind.is_symlink() {
                entries.insert(relative, Node::Link(fs::read_link(&path).unwrap()));
            } else if kind.is_dir() {
                entries.insert(relative, Node::Directory);
                visit(root, &path, entries);
            } else {
                entries.insert(relative, Node::File(fs::read(&path).unwrap()));
            }
        }
    }
    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}

fn checked_git(repo: &TempRepo, args: &[&str]) {
    let output = repo.git(args);
    assert!(output.status.success(), "git {args:?}: {}", text(&output));
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn clean_repo(label: &str) -> TempRepo {
    let repo = TempRepo::new(&format!("t118-{label}"));
    checked_git(&repo, &["rev-parse", "--is-inside-work-tree"]);
    repo.write("README.md", "A clean audited input.\n");
    repo.stage("README.md");
    assert!(audit_readonly(&repo, &[]).is_clean());
    repo
}

fn audit_readonly(repo: &TempRepo, rules: &[Rule]) -> Report {
    let before = snapshot(repo.root());
    let report = rebrand::audit(repo.root(), rules);
    assert_eq!(
        snapshot(repo.root()),
        before,
        "an audit must preserve index and all fixture bytes"
    );
    report
}

fn named(report: &Report, path: &str) -> bool {
    report
        .violations
        .iter()
        .any(|line| line.starts_with(&format!("{path}:")))
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Execute the two existing consumers, with only their audit input redirected.
/// Their Engine smoke checks remain isolated by those suites' own fixtures.
fn consumer_suites(repo: &TempRepo) -> Output {
    Command::new("cargo")
        .args([
            "test",
            "--offline",
            "-p",
            "ratmac-qa",
            "--test",
            "t034_rat005",
            "--test",
            "t037_rat008",
            "--no-fail-fast",
            "--",
            "--test-threads=1",
        ])
        .current_dir(repository())
        .env("RATMAC_AUDIT_ROOT", repo.root())
        .env("RATMAC_ACCEPTANCE_ROOT", repository())
        .output()
        .expect("execute the actual acceptance consumer suites")
}

#[test]
fn webv_009() {
    let repo = clean_repo("ignored-nested-checkout");
    repo.write(".gitignore", "scratch/\n");
    repo.write(
        ".arca/log.md",
        &format!("Historical {} wording.\n", PRE_CUTOVER_POSITION),
    );
    repo.write(
        ".arca/ticket/archive/kept.md",
        "Preserved historical carrier directory.\n",
    );
    let list = "test/qa/fixtures/rebrand-audit/allowlist.tsv";
    repo.write(
        list,
        &format!(
            ".arca/log.md\t{0}\tfixture history\n{1}\t{0}\tfixture list\n",
            PRE_CUTOVER_POSITION, list
        ),
    );
    checked_git(&repo, &["add", "--", ".gitignore", ".arca", list]);
    let rules = rebrand::load_allowlist(&rebrand::allowlist_path(repo.root())).unwrap();
    assert!(
        audit_readonly(&repo, &rules).is_clean(),
        "indexed positive control"
    );

    let nested = repo.root().join("scratch/nested");
    fs::create_dir_all(&nested).unwrap();
    let initialized = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&nested)
        .output()
        .unwrap();
    assert!(
        initialized.status.success(),
        "nested fixture init: {}",
        text(&initialized)
    );
    fs::write(nested.join("notes.md"), format!("{}\n", LEGACY_PRODUCT)).unwrap();
    fs::write(
        nested.join(format!("{}_notes.rs", PRE_CUTOVER_POSITION)),
        "clean content\n",
    )
    .unwrap();
    let trap = nested.join("unreadable.bin");
    fs::write(&trap, b"ignored read trap").unwrap();
    let before = snapshot(repo.root());
    #[cfg(windows)]
    let locked = {
        use std::os::windows::fs::OpenOptionsExt;
        let file = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&trap)
            .unwrap();
        assert!(
            fs::read(&trap).is_err(),
            "the ignored trap really denies reads"
        );
        file
    };
    let report = rebrand::audit(repo.root(), &rules);
    #[cfg(windows)]
    drop(locked);
    assert_eq!(snapshot(repo.root()), before);
    assert!(
        report.is_clean(),
        "ignored nested checkout must not enter either audit: {report:?}"
    );

    let clean_consumers = consumer_suites(&repo);
    assert!(
        clean_consumers.status.success(),
        "both real consumers ignore the nested checkout: {}",
        text(&clean_consumers)
    );
    repo.write("live.txt", &format!("{}\n", PRE_CUTOVER_POSITION));
    repo.stage("live.txt");
    let refusing_consumers = consumer_suites(&repo);
    let refusals = text(&refusing_consumers);
    assert!(
        !refusing_consumers.status.success(),
        "a tracked carrier must fail"
    );
    assert!(
        refusals.matches("live.txt:1:").count() >= 2,
        "both actual consumers name the tracked input: {refusals}"
    );
}

#[test]
fn webv_010() {
    let added = clean_repo("staged-addition-and-deletion");
    added.write("added.md", &format!("{}\n", PRE_CUTOVER_POSITION));
    added.stage("added.md");
    assert!(
        named(&audit_readonly(&added, &[]), "added.md"),
        "a staged addition is audited"
    );
    checked_git(&added, &["rm", "--cached", "--", "added.md"]);
    assert!(
        added.root().join("added.md").is_file(),
        "removal from index retains fixture bytes"
    );
    let deleted = audit_readonly(&added, &[]);
    assert!(
        deleted.is_clean(),
        "a staged deletion leaves no audit input even while working bytes remain: {deleted:?}"
    );

    let edited = clean_repo("working-bytes");
    edited.write(
        "README.md",
        &format!("Unstaged {} content.\n", PRE_CUTOVER_POSITION),
    );
    assert!(named(&audit_readonly(&edited, &[]), "README.md"));
    let filename = format!("{}_notes.rs", PRE_CUTOVER_POSITION);
    edited.write(&filename, "No forbidden content here.\n");
    edited.stage(&filename);
    let report = audit_readonly(&edited, &[]);
    assert!(report
        .violations
        .iter()
        .any(|line| line.starts_with(&format!("{filename}:")) && line.contains("path itself")));

    let missing = clean_repo("missing-indexed-file");
    fs::remove_file(missing.root().join("README.md")).unwrap();
    let report = audit_readonly(&missing, &[]);
    assert!(
        named(&report, "README.md"),
        "an unstaged missing input must refuse by name: {report:?}"
    );
}

#[test]
fn webv_011() {
    let repo = clean_repo("untracked-default");
    repo.write("extra.md", &format!("{}\n", PRE_CUTOVER_POSITION));
    let default = audit_readonly(&repo, &[]);
    assert!(
        default.is_clean(),
        "untracked content is invisible unless explicitly selected: {default:?}"
    );

    // An unusable repository is an input failure, not permission for a walk.
    let unavailable = clean_repo("unavailable-index");
    fs::create_dir(unavailable.root().join("saved")).unwrap();
    fs::rename(
        unavailable.root().join(".git"),
        unavailable.root().join("saved/.git"),
    )
    .unwrap();
    let report = audit_readonly(&unavailable, &[]);
    assert!(
        !report.is_clean(),
        "a missing repository index cannot pass: {report:?}"
    );
    assert!(
        report
            .violations
            .iter()
            .any(|line| line.to_ascii_lowercase().contains("git")),
        "name the failed repository inspection: {report:?}"
    );
}

#[test]
fn webv_012() {
    let repo = clean_repo("history-and-bytes");
    repo.write("history/old.md", &format!("{}\n", PRE_CUTOVER_POSITION));
    repo.stage("history/old.md");
    let rules = vec![Rule {
        pattern: "history/old.md".into(),
        token: PRE_CUTOVER_POSITION.into(),
        reason: "preserved historical fixture".into(),
    }];
    assert!(audit_readonly(&repo, &rules).is_clean());
    repo.write("live.md", &format!("{}\n", PRE_CUTOVER_POSITION));
    repo.stage("live.md");
    assert!(named(&audit_readonly(&repo, &rules), "live.md"));
    repo.write("live.md", "Clean again.\n");
    let mut stale = rules.clone();
    stale.push(Rule {
        pattern: "absent.md".into(),
        token: PRE_CUTOVER_POSITION.into(),
        reason: "unused row".into(),
    });
    assert_eq!(
        audit_readonly(&repo, &stale).stale,
        vec!["absent.md (unused row)".to_owned()]
    );

    fs::write(repo.root().join("clean.bin"), [0xff, 0, 2]).unwrap();
    repo.stage("clean.bin");
    assert!(
        audit_readonly(&repo, &rules).is_clean(),
        "clean binary content is legal"
    );
    let mut bytes = vec![0xff, 0x80];
    bytes.extend_from_slice(LEGACY_PRODUCT.as_bytes());
    fs::write(repo.root().join("invalid.bin"), bytes).unwrap();
    repo.stage("invalid.bin");
    let invalid = audit_readonly(&repo, &rules);
    assert!(
        invalid
            .violations
            .iter()
            .any(|line| line.starts_with("invalid.bin:")
                && line.contains("byte")
                && line.contains('2')),
        "invalid UTF-8 still exposes the ASCII token at byte offset 2: {invalid:?}"
    );

    let mut nul_bytes = vec![0, b'x', b'x'];
    nul_bytes.extend_from_slice(PRE_CUTOVER_POSITION.to_ascii_uppercase().as_bytes());
    fs::write(repo.root().join("nul.bin"), nul_bytes).unwrap();
    repo.stage("nul.bin");
    let nul = audit_readonly(&repo, &rules);
    assert!(
        nul.violations
            .iter()
            .any(|line| line.starts_with("nul.bin:")
                && line.contains("byte")
                && line.contains('3')),
        "NUL selects byte scanning with case-insensitive vocabulary: {nul:?}"
    );

    let mut names = vec!["space name.md".to_owned()];
    #[cfg(unix)]
    names.push("line\nbreak.md".to_owned());
    names.sort();
    for name in &names {
        repo.write(name, &format!("{}\n", PRE_CUTOVER_POSITION));
        repo.stage(name);
    }
    let first = audit_readonly(&repo, &rules);
    let second = audit_readonly(&repo, &rules);
    assert_eq!(first, second, "selection and diagnostics have stable order");
    for name in names {
        assert!(
            named(&first, &name),
            "unusual filename must survive selection: {name:?}"
        );
    }
}
