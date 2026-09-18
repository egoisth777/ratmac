//! Ticket t-110 - the four post-split hidden crates are ported, the roster is
//! complete, and the wired close guard passes (RLR-003, RLR-004; res-165,
//! res-166).
//!
//! The subjects are this repository's own landed crates, its tracked sweep
//! report, its roster, and the shipped sweep tool. A throwaway lanes root
//! (the `t108_lane_sweep.rs::Lanes` shape) proves the stray refusal on a
//! folder that holds a crate the roster never named, and a throwaway
//! declaration runs the sweep's verify mode over the two marked twins in
//! place - the twins are never copied, never unmarked.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ratmac_qa::baseline::{self, Pair};

/// The seven post-split crates `RLR-003` names: four ported by this ticket,
/// three that read green today and needed only the re-sweep.
const POST_SPLIT: [&str; 7] = [
    "t-083", "t-085", "t-092", "t-093", "t-095", "t-096", "t-100",
];

/// The two crates the `t-108` turn marked expired; their markers stay.
const MARKED: [&str; 2] = ["t-078", "t-079"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root resolves")
}

fn sweep_source() -> PathBuf {
    repo_root().join("tools/sweep_lanes.py")
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// One report's `| crate | verdict | detail |` rows.
fn rows_of(report: &str) -> Vec<(String, String, String)> {
    report
        .lines()
        .filter_map(|line| {
            let cells: Vec<&str> = line
                .strip_prefix("| ")?
                .strip_suffix(" |")?
                .split(" | ")
                .collect();
            match cells.as_slice() {
                [id, verdict, detail] if id.starts_with("t-") => {
                    Some((id.to_string(), verdict.to_string(), detail.to_string()))
                }
                _ => None,
            }
        })
        .collect()
}

fn row<'a>(rows: &'a [(String, String, String)], id: &str) -> &'a (String, String, String) {
    rows.iter()
        .find(|(crate_id, _, _)| crate_id == id)
        .unwrap_or_else(|| panic!("the report carries a row for {id}"))
}

/// The roster `.ratmac/lanes.toml` declares, in declaration order.
fn declared_roster(root: &Path) -> Vec<String> {
    let text = fs::read_to_string(root.join(".ratmac/lanes.toml")).expect("read the roster");
    let (_, after) = text
        .split_once("roster = [")
        .expect("the declaration carries a roster list");
    let (list, _) = after.split_once(']').expect("the roster list closes");
    list.split(',')
        .map(|entry| entry.trim().trim_matches('"').to_owned())
        .filter(|entry| !entry.is_empty())
        .collect()
}

/// Every `t-NNN` folder directly under the lanes root.
fn crate_folders(lanes_root: &Path) -> BTreeSet<String> {
    fs::read_dir(lanes_root)
        .expect("read the lanes root")
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            name.len() == 5
                && name.starts_with("t-")
                && name[2..].bytes().all(|b| b.is_ascii_digit())
        })
        .collect()
}

fn sweep_tool(root: &Path, args: &[&str]) -> Output {
    Command::new("python")
        .arg(root.join("tools/sweep_lanes.py"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("invoke the lane sweep")
}

/// A throwaway root with its own sweep tool, declaration, and crates.
struct Scratch {
    root: PathBuf,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Scratch {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t110-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("tools")).expect("create fixture tools");
        fs::copy(sweep_source(), root.join("tools/sweep_lanes.py"))
            .expect("install the shipped sweep script");
        fs::create_dir_all(root.join(".ratmac")).expect("create the declaration directory");
        Self { root }
    }

    fn declare(&self, lanes: &str, roster: &[&str]) {
        let mut declaration = format!(
            "[roots]\nlanes = \"{}\"\nreport = \".ratmac/evidence/lane-sweep/report.md\"\nroster = [\n",
            lanes.replace('\\', "/")
        );
        for crate_id in roster {
            declaration.push_str(&format!("  \"{crate_id}\",\n"));
        }
        declaration.push_str("]\n");
        fs::write(self.root.join(".ratmac/lanes.toml"), declaration)
            .expect("write the declaration");
    }

    /// One self-contained crate whose single lane passes.
    fn green_crate(&self, crate_id: &str) {
        let number = crate_id.trim_start_matches("t-");
        let crate_root = self.root.join("test-hidden").join(crate_id);
        fs::create_dir_all(crate_root.join("src")).expect("create fixture crate");
        fs::create_dir_all(crate_root.join("tests")).expect("create fixture tests");
        fs::write(
            crate_root.join("Cargo.toml"),
            format!(
                "[package]\nname = \"t{number}hidden\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
                 publish = false\n\n[workspace]\n"
            ),
        )
        .expect("write fixture manifest");
        fs::write(crate_root.join("src/lib.rs"), "").expect("write fixture lib");
        fs::write(
            crate_root.join("tests/hidden.rs"),
            format!("#[test]\nfn ht_{number}_01_lane() {{}}\n"),
        )
        .expect("write fixture lane");
    }

    fn report(&self) -> String {
        fs::read_to_string(self.root.join(".ratmac/evidence/lane-sweep/report.md"))
            .expect("the sweep wrote its report")
    }
}

// --- RLRV-002 -----------------------------------------------------------------

/// RLRV-002 (t-110, RLR-003): the tracked report carries a `pass` row for
/// each of the seven post-split crates, and the baseline helper's listing is
/// bare paths - a listing holding `.ratmac/log.md` is found by name, so the
/// newline defect behind `ht_085_03` is dead.
#[test]
fn every_post_split_crate_passes_and_the_baseline_helper_trims_entries() {
    let root = repo_root();
    let engine = root.join("target/debug/rtm-qa");
    let normalize = |text: &str| baseline::normalize(text, &root, &engine);
    assert_eq!(normalize("pending guard: join require=\"all_passed\" min=1\n"),
        normalize("pending guard: join\n"));
    for changed in ["pending guard: join require=\"all_passed\" min=2\n",
        "pending guard: join require=\"any_passed\" min=1\n"] {
        assert_ne!(normalize(changed), normalize("pending guard: join\n"),
            "changed guard arguments must not disappear");
    }
    assert_eq!(normalize("Commands: start, scaffold, skill\n"),
        normalize("Commands: start, scaffold\n"));
    assert_ne!(normalize("Commands: start, scaffold, invented\n"),
        normalize("Commands: start, scaffold\n"));
    let report = fs::read_to_string(root.join(".ratmac/evidence/lane-sweep/report.md"))
        .expect("read the tracked sweep report");
    let rows = rows_of(&report);
    for (crate_id, count) in POST_SPLIT.into_iter().zip([6, 6, 11, 7, 6, 6, 6]) {
        let (_, verdict, detail) = row(&rows, crate_id);
        assert_eq!(
            verdict, "pass",
            "RLRV-002: {crate_id} reads pass in the tracked report: {detail}"
        );
        assert_eq!(detail, &format!("{count} lane(s) green"),
            "RLRV-002: {crate_id} preserves every landed test");
    }

    let pair = Pair::new(
        "t110-listing",
        baseline::today_engine(),
        baseline::DEFAULT_RUNBOOK,
        &[(".ratmac/log.md", "- 2026-09-17: a landing\n")],
    );
    let listed = pair.freeze_paths();
    assert!(
        listed.iter().any(|entry| entry == ".ratmac/log.md"),
        "RLRV-002: the freeze listing names .ratmac/log.md as a bare path: {listed:?}"
    );
    assert!(
        listed.iter().all(|entry| entry.trim_end() == entry),
        "RLRV-002: no listing entry is newline-tipped: {listed:?}"
    );
}

// --- RLRV-003 -----------------------------------------------------------------

/// RLRV-003 (t-110, RLR-004): the roster names every landed crate under the
/// lanes root - `t-108` included - the check exits `0` on this repository,
/// and on a folder holding a crate the roster never named the sweep names
/// the stray and the check refuses with exit `2` naming it.
#[test]
fn the_roster_is_complete_and_the_check_refuses_strays() {
    let root = repo_root();
    let roster = declared_roster(&root);
    assert!(
        roster.iter().any(|id| id == "t-108"),
        "RLRV-003: the roster names t-108: {roster:?}"
    );
    let declared: BTreeSet<String> = roster.iter().cloned().collect();
    assert_eq!(
        declared.len(),
        roster.len(),
        "RLRV-003: the roster repeats no crate: {roster:?}"
    );
    let folders = crate_folders(&root.join("test-hidden"));
    assert_eq!(
        declared, folders,
        "RLRV-003: the roster names exactly the crates the folder holds"
    );

    let check = sweep_tool(&root, &["check"]);
    assert_eq!(
        check.status.code(),
        Some(0),
        "RLRV-003: the close guard's check exits 0 on this repository: {}",
        combined(&check)
    );

    let scratch = Scratch::new("stray");
    scratch.declare("test-hidden", &["t-900"]);
    scratch.green_crate("t-900");
    scratch.green_crate("t-901");
    let swept = sweep_tool(&scratch.root, &["sweep"]);
    assert_eq!(
        swept.status.code(),
        Some(0),
        "the rostered crate passes, so the sweep itself exits 0: {}",
        combined(&swept)
    );
    let report = scratch.report();
    assert!(
        report.contains("stray entries (in the folder, not in the roster): t-901"),
        "RLRV-003: the report names the stray: {report}"
    );
    let check = sweep_tool(&scratch.root, &["check"]);
    let text = combined(&check);
    assert_eq!(
        check.status.code(),
        Some(2),
        "RLRV-003: the check refuses a stray with exit 2: {text}"
    );
    assert!(
        text.contains("t-901") && text.contains("stray"),
        "RLRV-003: the refusal names the stray crate: {text}"
    );
}

// --- RLRV-004 -----------------------------------------------------------------

/// RLRV-004 (t-110, RLR-004): with a declaration that rosters only the two
/// marked twins in place, `sweep --verify-expired` runs them and reports
/// them still refusing beside their markers, while a plain sweep reads them
/// `expired` without running - the markers stay honest and stay in force.
#[test]
fn verify_expired_runs_the_marked_twins_and_reports_them_red() {
    let root = repo_root();
    let roster = declared_roster(&root);
    assert_eq!(roster.iter().cloned().collect::<BTreeSet<_>>(),
        crate_folders(&root.join("test-hidden")),
        "RLRV-004: expiry verification must cover the complete real roster");
    let markers: Vec<_> = MARKED.iter().map(|id| {
        let path = root.join("test-hidden").join(id).join("EXPIRED.toml");
        let bytes = fs::read(&path).expect("read original marker");
        (path, bytes)
    }).collect();
    for crate_id in MARKED {
        assert!(
            root.join("test-hidden")
                .join(crate_id)
                .join("EXPIRED.toml")
                .is_file(),
            "{crate_id} carries its marker"
        );
    }
    let scratch = Scratch::new("verify");
    let lanes = root.join("test-hidden");
    let roster_refs: Vec<_> = roster.iter().map(String::as_str).collect();
    scratch.declare(&lanes.to_string_lossy(), &roster_refs);

    let verified = sweep_tool(&scratch.root, &["sweep", "--verify-expired"]);
    let text = combined(&verified);
    assert_eq!(verified.status.code(), Some(0), "full verification sweep passes: {text}");
    assert!(!scratch.report().contains("stray entries"), "verification skips no crate");
    let rows = rows_of(&scratch.report());
    for crate_id in MARKED {
        let (_, verdict, detail) = row(&rows, crate_id);
        assert_eq!(
            verdict, "expired",
            "RLRV-004: {crate_id} stays expired under verify: {detail}"
        );
        assert!(
            detail.contains("last passed at edition-") && detail.contains("verify: lanes refuse"),
            "RLRV-004: {crate_id} was run and still refuses beside its marker: {detail}\n{text}"
        );
        assert!(!detail.contains("build refused:") && !detail.contains("timed out")
            && !detail.contains("cargo did not run"),
            "expiry evidence must come from actual failing tests: {detail}");
    }

    scratch.declare(&lanes.to_string_lossy(), &MARKED);
    let plain = sweep_tool(&scratch.root, &["sweep"]);
    assert_eq!(
        plain.status.code(),
        Some(0),
        "a plain sweep over two expired crates exits 0: {}",
        combined(&plain)
    );
    let rows = rows_of(&scratch.report());
    for crate_id in MARKED {
        let (_, verdict, detail) = row(&rows, crate_id);
        assert_eq!(
            verdict, "expired",
            "RLRV-004: {crate_id} reads expired: {detail}"
        );
        assert!(
            !detail.contains("verify:"),
            "RLRV-004: a plain sweep never runs a marked lane: {detail}"
        );
    }
    for (path, original) in markers {
        assert_eq!(fs::read(path).expect("read marker after verification"), original,
            "both sweep modes preserve marker bytes");
    }
}
