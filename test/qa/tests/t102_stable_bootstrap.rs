//! t-102 / ELR-002: the stable bootstrap resolves from the invoking checkout
//! and builds the tagged tree in a separate checkout.
//!
//! ELRV-002, the planned test this file carries, drives the REAL
//! `tools/rtm.ps1` - copied into the fixture so the script under test is the
//! one this ticket changes - over a fixture with a past (`GPH-001`): commit A
//! is tagged `edition-001` while A's own `.arca/editions.md` row cites a
//! stale hash, and the later invoking checkout B carries the row that agrees
//! with the tag, which is the permanent shape `ELR-001` gives every edition.
//!
//! The fixture's engine is a zero-dependency offline `rtm` bin that prints the
//! two provenance stamps the real bootstrap sets before building
//! (`RTM_CHANNEL`, `RTM_SOURCE_COMMIT`), so a successful stable build can be
//! executed and judged without ever building the real engine in a fixture.
//!
//! Hole-poking, recorded as the assignment asks:
//! - A script that reads the ledger from the checkout it builds in (the
//!   defect `ELR-002` names) judges A's stale row, refuses with "but the tag
//!   points at", and fails (a)'s exit-zero requirement.
//! - A script that skips the tree-identity check fails (c) twice over: the
//!   rerun must refuse even though a built, stampable engine already sits in
//!   the dirtied checkout's `target/debug` (first from the build, reinforced
//!   by the test's plant), so a check-skipping bootstrap locates and stamps
//!   it and exits 0; and a skipper that rebuilds instead answers with the
//!   build-failure line the test forbids.
//! - A script that builds by checking the tagged commit out in the invoking
//!   tree, or that fails every build, fails (a): no linked checkout at the
//!   tagged commit, no built binary inside it, and the invoking tree's
//!   `target/` must not even exist.
//! - Coupling kept deliberately: the pinned texts are the contract surface -
//!   "but the tag points at" stays reserved for the invoking checkout's own
//!   disagreement, and the tree check must refuse naming "build checkout"
//!   before any build attempt, so "did not produce an Engine" must not
//!   appear. Rewording the build-failure refusal later means re-reading this
//!   test, not just re-running it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ratmac_qa::edition::{self, LEDGER_PATH};
use ratmac_qa::tempgit::TempRepo;

/// The whole 40-hex hash commit A's own stale row cites: well-formed, so a
/// script that wrongly judges it refuses at the disagreement, never at the
/// format rule.
fn stale_hash() -> String {
    "0".repeat(40)
}

/// The editions ledger as one row citing `commit` for `edition-001`.
fn ledger(commit: &str) -> String {
    format!(
        "# Editions\n\n| Edition | Commit | What it marks |\n| :--- | :--- | :--- |\n\
         | `edition-001` | `{commit}` | The first edition. |\n"
    )
}

/// The manifest of the fixture's offline-buildable engine: a workspace of one
/// zero-dependency bin named exactly as the bootstrap builds it.
fn fixture_manifest() -> String {
    "[workspace]\n\n[package]\nname = \"rtm\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
     [[bin]]\nname = \"rtm\"\npath = \"src/main.rs\"\n"
        .into()
}

/// The fixture's engine source: it answers with the two provenance stamps the
/// real bootstrap sets in the environment before `cargo build`, mirroring how
/// the real engine records its channel and source commit.
fn fixture_engine_source() -> String {
    "fn main() {\n    \
     println!(\"channel={}\", option_env!(\"RTM_CHANNEL\").unwrap_or(\"\"));\n    \
     println!(\"source-commit={}\", option_env!(\"RTM_SOURCE_COMMIT\").unwrap_or(\"\"));\n}\n"
        .into()
}

/// The bootstrap under test: this ticket's `tools/rtm.ps1`, not a copy the
/// fixture could drift from.
fn bootstrap_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/rtm.ps1")
        .canonicalize()
        .expect("the bootstrap under test exists")
}

/// The `GPH-001` fixture: an edition whose tagged commit A carries a stale
/// ledger row, and an invoking checkout B whose row agrees with the tag.
fn past_edition() -> (TempRepo, String, String) {
    let repo = TempRepo::new("t102-past");
    repo.write("Cargo.toml", &fixture_manifest());
    repo.write("src/main.rs", &fixture_engine_source());
    repo.write("README.md", "fixture\n");
    repo.write(LEDGER_PATH, &ledger(&stale_hash()));
    repo.commit_all("edition one");
    let a = repo.head();
    repo.git(&[
        "tag",
        "-a",
        "edition-001",
        "-m",
        edition::EXAMPLE_BAR_MESSAGE,
    ]);

    // B, the invoking checkout: the ledger catches up to the tag, and the
    // real bootstrap is installed at the path its guidance names.
    repo.write(LEDGER_PATH, &ledger(&a));
    fs::create_dir_all(repo.root().join("tools")).expect("create fixture tools directory");
    fs::copy(bootstrap_source(), repo.root().join("tools/rtm.ps1"))
        .expect("install the bootstrap under test");
    repo.commit_all("ledger catches up; install the bootstrap");
    let b = repo.head();
    (repo, a, b)
}

/// `pwsh -NoProfile -File tools/rtm.ps1 -Channel stable` from `root`.
fn bootstrap_stable(root: &Path) -> Output {
    Command::new("pwsh")
        .args(["-NoProfile", "-File", "tools/rtm.ps1", "-Channel", "stable"])
        .current_dir(root)
        .output()
        .expect("invoke the stable bootstrap")
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn engine_file_name() -> &'static str {
    if cfg!(windows) {
        "rtm.exe"
    } else {
        "rtm"
    }
}

/// Every linked worktree of `repo` other than its main checkout, with the
/// commit each stands at, in `git worktree list` order.
fn linked_worktrees(repo: &TempRepo) -> Vec<(PathBuf, String)> {
    fn flush(
        path: &mut Option<PathBuf>,
        head: &mut Option<String>,
        rows: &mut Vec<(PathBuf, String)>,
    ) {
        if let (Some(path), Some(head)) = (path.take(), head.take()) {
            rows.push((path, head));
        }
    }

    let listed = repo.git(&["worktree", "list", "--porcelain"]);
    assert!(
        listed.status.success(),
        "git worktree list must run: {}",
        String::from_utf8_lossy(&listed.stderr)
    );
    let main = repo.root().canonicalize().expect("canonical main checkout");
    let mut rows = Vec::new();
    let mut path = None;
    let mut head = None;
    for line in String::from_utf8_lossy(&listed.stdout).lines() {
        if let Some(rest) = line.strip_prefix("worktree ") {
            flush(&mut path, &mut head, &mut rows);
            path = Some(PathBuf::from(rest));
        } else if let Some(rest) = line.strip_prefix("HEAD ") {
            head = Some(rest.to_owned());
        }
    }
    flush(&mut path, &mut head, &mut rows);
    rows.into_iter()
        .filter(|(path, _)| {
            path.canonicalize()
                .map(|canonical| canonical != main)
                .unwrap_or(true)
        })
        .collect()
}

/// ELRV-002: the stable bootstrap resolves the newest edition row in the
/// invoking checkout, builds the tagged tree in a clean separate checkout,
/// refuses an invoking-checkout ledger/tag disagreement, and refuses a build
/// checkout whose tree differs from the tagged commit before stamping.
#[test]
fn stable_resolves_from_the_invoking_checkout_and_builds_the_tagged_tree() {
    // (a) Resolution belongs to the invoking checkout: B's row agrees with
    // the tag, so the bootstrap proceeds even though A's own row is stale,
    // builds the tagged commit in a separate checkout, and stamps it.
    {
        let (repo, a, _b) = past_edition();
        let run = bootstrap_stable(repo.root());
        let report = text(&run);
        assert!(
            run.status.success(),
            "ELR-002: invoked at B with -Channel stable the bootstrap must resolve \
             edition-001 from B's ledger, build the tagged commit in a separate clean \
             checkout, and succeed; it refused at resolution inside the invoking tree \
             instead:\n{report}"
        );
        let others = linked_worktrees(&repo);
        let worktree = others
            .iter()
            .find(|(_, head)| head == &a)
            .map(|(path, _)| path.clone())
            .unwrap_or_else(|| {
                panic!(
                    "the engine is built in a separate linked checkout at the tagged \
                     commit {a}; registered: {others:?}"
                )
            });
        let name = engine_file_name();
        let built = [
            worktree.join("target/release"),
            worktree.join("target/debug"),
        ]
        .into_iter()
        .map(|dir| dir.join(name))
        .find(|bin| bin.exists())
        .unwrap_or_else(|| {
            panic!(
                "a built engine must sit inside the build checkout at {}",
                worktree.join("target").display()
            )
        });
        let stamp = Command::new(&built).output().expect("run the built engine");
        let provenance = text(&stamp);
        assert!(
            provenance.contains("channel=stable"),
            "the built engine carries the stable channel stamp:\n{provenance}"
        );
        assert!(
            provenance.contains(&format!("source-commit={a}")),
            "the built engine carries the tagged commit as its source:\n{provenance}"
        );
        assert!(
            !repo.root().join("target").exists(),
            "the bootstrap builds only in the separate checkout; the invoking tree \
             stays unwritten"
        );
    }

    // (b) An invoking-checkout ledger/tag disagreement refuses at resolution,
    // before any build checkout exists.
    {
        let (repo, _a, b) = past_edition();
        repo.write(LEDGER_PATH, &ledger(&b));
        repo.commit_all("ledger drifts from the tag");
        let run = bootstrap_stable(repo.root());
        let report = text(&run);
        assert!(
            !run.status.success(),
            "an invoking-checkout ledger/tag disagreement refuses:\n{report}"
        );
        assert!(
            report.contains("the ledger records") && report.contains("but the tag points at"),
            "the refusal names the ledger/tag disagreement:\n{report}"
        );
        assert!(
            linked_worktrees(&repo).is_empty(),
            "a resolution refusal happens before any build checkout exists"
        );
    }

    // (c) A build checkout whose tree differs from the tagged commit refuses
    // before stamping.
    {
        let (repo, a, _b) = past_edition();
        let first = bootstrap_stable(repo.root());
        let first_report = text(&first);
        let others = linked_worktrees(&repo);
        let worktree = match others.iter().find(|(_, head)| head == &a) {
            Some((worktree, _)) => worktree.clone(),
            None => {
                assert!(
                    first_report.to_lowercase().contains("build checkout"),
                    "the bootstrap must name its build checkout even before one can be \
                     reused:\n{first_report}"
                );
                panic!(
                    "no separate build checkout at the tagged commit exists to dirty; \
                     the tree-identity refusal cannot be exercised"
                );
            }
        };
        // Dirty the tree both ways a check can look: a modified tracked file
        // and an extra untracked one, both outside the declared build output
        // (target, Cargo.lock) the bootstrap is allowed to write.
        let readme = worktree.join("README.md");
        let mut dirtied = fs::read_to_string(&readme).expect("read the build checkout readme");
        dirtied.push_str("dirtied by qa\n");
        fs::write(&readme, dirtied).expect("dirty a tracked file");
        fs::write(worktree.join("planted-by-qa.txt"), "dirtied by qa\n")
            .expect("plant an untracked file");
        // A hashable prebuilt engine where a check-skipping bootstrap would
        // find and stamp it; a faithful one refuses before reaching it.
        let prebuilt = worktree.join("target").join("debug");
        fs::create_dir_all(&prebuilt).expect("create the prebuilt directory");
        fs::copy(ratmac_qa::engine_bin!(), prebuilt.join(engine_file_name()))
            .expect("plant a prebuilt engine");

        let rerun = bootstrap_stable(repo.root());
        let refusal = text(&rerun);
        assert!(
            !rerun.status.success(),
            "a build checkout whose tree differs from the tagged commit refuses before \
             stamping; the built and planted engines must never be reached:\n{refusal}"
        );
        assert!(
            refusal.to_lowercase().contains("build checkout"),
            "the refusal names the build checkout:\n{refusal}"
        );
        assert!(
            !refusal.contains("did not produce an Engine"),
            "the refusal fires at the tree check, before any build attempt:\n{refusal}"
        );
    }
}
