//! WEB-003: repository-content audits select the index and inspect working bytes.

use ratmac_qa::audit_files::{self, EntryKind};
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

fn extras_readonly(repo: &TempRepo, extras: &[PathBuf]) -> Report {
    let before = snapshot(repo.root());
    let report = rebrand::audit_with_extras(repo.root(), &[], extras);
    assert_eq!(
        snapshot(repo.root()),
        before,
        "extra selection is read-only"
    );
    report
}

fn directory_link(target: &Path, path: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, path).expect("create isolated directory link");
    #[cfg(windows)]
    {
        // Directory junctions need no symbolic-link privilege on Windows.
        let script = path.parent().unwrap().join("make-audit-junction.ps1");
        fs::write(&script, "param([string]$Destination, [string]$Source)\n$ErrorActionPreference = 'Stop'\nNew-Item -ItemType Junction -Path $Destination -Target $Source | Out-Null\n").unwrap();
        let output = Command::new("pwsh")
            .args(["-NoProfile", "-File"])
            .arg(&script)
            .arg("-Destination")
            .arg(path)
            .arg("-Source")
            .arg(target)
            .output()
            .expect("create isolated directory junction");
        assert!(
            output.status.success(),
            "junction fixture: {}",
            text(&output)
        );
    }
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
    let selection = audit_files::select(repo.root(), &[]).expect("indexed input set");
    assert!(selection
        .entries
        .iter()
        .all(|entry| !entry.path.starts_with("scratch")));

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

    let explicit = extras_readonly(&repo, &["extra.md".into(), "extra.md".into()]);
    assert!(
        named(&explicit, "extra.md"),
        "an explicit input remains subject to the content audit"
    );
    assert_eq!(explicit.extra_inputs, vec!["extra.md".to_owned()]);
    assert_eq!(
        explicit
            .violations
            .iter()
            .filter(|line| line.starts_with("extra.md:"))
            .count(),
        1
    );
    let selected = audit_files::select(repo.root(), &["extra.md".into()]).unwrap();
    assert_eq!(
        selected
            .entries
            .iter()
            .filter(|entry| entry.path == Path::new("extra.md"))
            .count(),
        1
    );

    fs::create_dir(repo.root().join("directory")).unwrap();
    for extra in [
        PathBuf::from("missing.md"),
        PathBuf::from("directory"),
        repo.root().join("README.md"),
        PathBuf::from("../README.md"),
        PathBuf::from("directory/../../README.md"),
    ] {
        let report = extras_readonly(&repo, std::slice::from_ref(&extra));
        assert!(
            !report.is_clean(),
            "invalid explicit input passed: {extra:?}"
        );
        assert!(
            !report.violations.is_empty(),
            "invalid input must be a named failure"
        );
    }
    #[cfg(windows)]
    for extra in [PathBuf::from("C:relative.md"), PathBuf::from(r"\rooted.md")] {
        assert!(!extras_readonly(&repo, &[extra]).is_clean());
    }

    let outside = clean_repo("outside-extra-target");
    directory_link(outside.root(), &repo.root().join("bridge"));
    let outside_before = snapshot(outside.root());
    let report = extras_readonly(&repo, &["bridge/README.md".into()]);
    assert!(
        !report.is_clean(),
        "a directory link cannot turn external clean bytes into an allowed extra"
    );
    assert!(
        report.violations.iter().any(|line| line.contains("bridge")),
        "name the escaping input: {report:?}"
    );
    assert_eq!(snapshot(outside.root()), outside_before);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            outside.root().join("README.md"),
            repo.root().join("file-link"),
        )
        .unwrap();
        assert!(!extras_readonly(&repo, &["file-link".into()]).is_clean());
        repo.write("ordinary.md", "An indexed regular file.\n");
        repo.stage("ordinary.md");
        fs::remove_file(repo.root().join("ordinary.md")).unwrap();
        std::os::unix::fs::symlink(
            repo.root().join("README.md"),
            repo.root().join("ordinary.md"),
        )
        .unwrap();
        assert!(
            audit_files::select(repo.root(), &[]).is_err(),
            "an indexed regular file cannot become a link even to an in-root target"
        );
    }
    index_listing_controls();
    index_kind_controls();

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
    assert!(
        invalid
            .violations
            .iter()
            .any(|line| line.starts_with("invalid.bin:byte 2:")),
        "the binary location is exactly zero-based byte 2: {invalid:?}"
    );

    let mut nul_bytes = vec![0, b'x', b'x'];
    nul_bytes.extend_from_slice(PRE_CUTOVER_POSITION.to_ascii_uppercase().as_bytes());
    fs::write(repo.root().join("nul-content.bin"), nul_bytes).unwrap();
    repo.stage("nul-content.bin");
    let nul = audit_readonly(&repo, &rules);
    assert!(
        nul.violations
            .iter()
            .any(|line| line.starts_with("nul-content.bin:")
                && line.contains("byte")
                && line.contains('3')),
        "NUL selects byte scanning with case-insensitive vocabulary: {nul:?}"
    );
    assert!(
        nul.violations
            .iter()
            .any(|line| line.starts_with("nul-content.bin:byte 3:")),
        "the NUL-containing location is exactly zero-based byte 3: {nul:?}"
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
    #[cfg(unix)]
    {
        let literal_backslash = r"literal\name.md";
        repo.write(literal_backslash, &format!("{}\n", PRE_CUTOVER_POSITION));
        repo.stage(literal_backslash);
        let report = audit_readonly(&repo, &rules);
        assert!(
            named(&report, literal_backslash),
            "Unix backslash is filename content, not a separator"
        );
    }
    selected_read_failure();
}

fn index_listing_controls() {
    let hash = "1".repeat(40);
    let valid = format!("100644 {hash} 0\tspace name.md\0");
    let parsed = audit_files::parse_index_listing(valid.as_bytes()).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].path, Path::new("space name.md"));
    assert_eq!(parsed[0].kind, EntryKind::File);
    let newline = format!("100644 {hash} 0\tline\nbreak.md\0");
    assert_eq!(
        audit_files::parse_index_listing(newline.as_bytes()).unwrap()[0].path,
        Path::new("line\nbreak.md")
    );
    for malformed in [
        valid.trim_end_matches('\0').to_owned(),
        format!("100644 {hash} 0 path.md\0"),
        format!("100644 {hash} 1\tconflicted.md\0"),
        format!("100644 {hash} 7\tbad-stage.md\0"),
        format!("999999 {hash} 0\tbad-mode.md\0"),
        "100644 not-a-hash 0\tbad-id.md\0".to_owned(),
        format!("100644 {hash} 0\t../escape.md\0"),
        format!("100644 {hash} 0\t/absolute.md\0"),
        format!("100644 {hash} 0\t\0"),
        format!("{valid}{valid}"),
    ] {
        assert!(
            audit_files::parse_index_listing(malformed.as_bytes()).is_err(),
            "malformed selection accepted: {malformed:?}"
        );
    }
}

fn index_kind_controls() {
    let repo = clean_repo("index-kinds");
    repo.commit_all("Fixture repository identity");
    let commit = repo.head();
    checked_git(
        &repo,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            "160000",
            &commit,
            "unpopulated",
        ],
    );
    assert!(!repo.root().join("unpopulated").exists());
    let selection =
        audit_files::select(repo.root(), &[]).expect("unpopulated Gitlink is a boundary");
    let boundary = selection
        .entries
        .iter()
        .find(|entry| entry.path == Path::new("unpopulated"))
        .unwrap();
    assert_eq!(boundary.kind, EntryKind::Gitlink);
    assert!(boundary.bytes.is_empty());
    assert!(
        selection
            .exclusions
            .iter()
            .any(|line| line.contains("unpopulated")),
        "a Gitlink exclusion is explicit"
    );

    checked_git(&repo, &["config", "core.symlinks", "false"]);
    let link_text = format!("../outside/{}.md", PRE_CUTOVER_POSITION);
    repo.write("link-text", &link_text);
    let hashed = repo.git(&["hash-object", "-w", "--", "link-text"]);
    assert!(
        hashed.status.success(),
        "write fixture link blob: {}",
        text(&hashed)
    );
    let hash = String::from_utf8(hashed.stdout).unwrap();
    checked_git(
        &repo,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            "120000",
            hash.trim(),
            "link-text",
        ],
    );
    assert!(fs::symlink_metadata(repo.root().join("link-text"))
        .unwrap()
        .is_file());
    let selection =
        audit_files::select(repo.root(), &[]).expect("regular symlink-text checkout is supported");
    let link = selection
        .entries
        .iter()
        .find(|entry| entry.path == Path::new("link-text"))
        .unwrap();
    assert_eq!(link.kind, EntryKind::Symlink);
    assert_eq!(link.bytes, link_text.as_bytes());
    assert!(
        named(&audit_readonly(&repo, &[]), "link-text"),
        "link text is audited rather than followed"
    );
}

fn selected_read_failure() {
    let repo = clean_repo("unreadable-selected-file");
    repo.write("selected.bin", "Clean selected content.\n");
    repo.stage("selected.bin");
    let path = repo.root().join("selected.bin");
    let before = snapshot(repo.root());
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();
        assert!(
            fs::read(&path).is_err(),
            "selected fixture genuinely denies reads"
        );
        let report = rebrand::audit(repo.root(), &[]);
        drop(held);
        assert!(
            named(&report, "selected.bin"),
            "unreadable selected bytes must be a named failure: {report:?}"
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let original = fs::metadata(&path).unwrap().permissions();
        fs::set_permissions(&path, fs::Permissions::from_mode(0)).unwrap();
        if fs::read(&path).is_err() {
            let report = rebrand::audit(repo.root(), &[]);
            fs::set_permissions(&path, original).unwrap();
            assert!(
                named(&report, "selected.bin"),
                "unreadable selected bytes must be a named failure: {report:?}"
            );
        } else {
            fs::set_permissions(&path, original).unwrap();
            eprintln!(
                "read-denial fixture unavailable under an identity that bypasses Unix permissions"
            );
        }
    }
    assert_eq!(snapshot(repo.root()), before);
}
