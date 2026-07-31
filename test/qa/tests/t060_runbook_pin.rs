//! t-059 / FDC-005: the runbook pin stays hash-only; flat-layout residue refuses.
//!
//! PT-059-01 `pin_is_hash_only_with_no_per_run_copy`
//! PT-059-02 `flat_layout_residue_refuses_and_instructs`
//!
//! The runbook pin is a recorded hash and nothing more: `rtm start` records
//! the SHA-256 of the canonical `.arca/ratmac.toml` in the run's evidence,
//! every later Scheduler read of the class compares against it, and a
//! mismatch refuses naming observed and expected identity — the Engine-pin
//! refusal shape — while no code path ever copies the runbook anywhere.
//! Meeting a flat-layout residue — a pre-plural `.arca/state.toml` on disk —
//! the Engine refuses, names the observed fact and the repair, and modifies
//! nothing: the lock-refusal precedent, never an auto-migration. Both trace
//! to `RRV-003`; residue variants and full write-site enumeration stay with
//! the hidden lanes (`HT-059-02`, `HT-059-05`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Fixture {
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    /// A temp project with a valid two-phase runbook, not yet started.
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t059-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".arca")).expect("create fixture tree");
        fs::create_dir_all(root.join("src")).expect("create source tree");
        fs::write(root.join("src/lib.rs"), "pub fn work() {}\n").expect("write source");
        fs::create_dir_all(root.join(".arca/goal")).expect("create goal tree");
        fs::write(root.join(".arca/goal/spec.md"), "# Spec\n").expect("write goal");
        fs::write(
            root.join(".arca/ratmac.toml"),
            "[phases.intake]\nprompt = \"Integrate the issues.\"\n\n\
             [phases.build]\nprompt = \"Build the ticket.\"\n\n\
             [[transitions]]\nfrom = \"intake\"\nto = \"build\"\n",
        )
        .expect("write runbook");
        Fixture { root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Listing `.arca/runs/` IS the roster: the run ids, read off artifacts.
    fn roster(&self) -> Vec<String> {
        let runs = self.path(".arca/runs");
        assert!(
            runs.is_dir(),
            "FDC-005: runs must reside under the plural .arca/runs/ path — listing it IS the roster"
        );
        let mut ids: Vec<String> = fs::read_dir(&runs)
            .expect("the runs directory must be listable")
            .map(|entry| entry.expect("roster entry is readable"))
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        ids.sort();
        ids
    }
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// SHA-256 of a byte string, lowercase hex — the repo-wide pin convention.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Every file under `root`: relative forward-slashed path → exact bytes.
fn tree_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, into: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).expect("snapshot directory is listable") {
            let path = entry.expect("snapshot entry is readable").path();
            if path.is_dir() {
                walk(root, &path, into);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .expect("snapshot paths sit under the root")
                    .to_string_lossy()
                    .replace('\\', "/");
                into.insert(
                    relative,
                    fs::read(&path).expect("snapshot file is readable"),
                );
            }
        }
    }
    let mut snapshot = BTreeMap::new();
    walk(root, root, &mut snapshot);
    snapshot
}

/// Byte-identical trees, or a panic naming the first created/deleted/changed path.
fn assert_trees_equal(
    before: &BTreeMap<String, Vec<u8>>,
    after: &BTreeMap<String, Vec<u8>>,
    context: &str,
) {
    for path in before.keys() {
        assert!(after.contains_key(path), "{context}: {path} was deleted");
    }
    for (path, bytes) in after {
        match before.get(path) {
            None => panic!("{context}: {path} appeared"),
            Some(previous) => {
                assert!(previous == bytes, "{context}: {path} changed");
            }
        }
    }
}

/// PT-059-01 (RRV-003): after `rtm start`, the run's evidence records the
/// canonical runbook's SHA-256; no copy of the runbook exists under
/// `.arca/runs/<id>/` or anywhere else; editing the runbook then stepping
/// refuses naming observed and expected hash and modifies nothing.
#[test]
fn pin_is_hash_only_with_no_per_run_copy() {
    let fixture = Fixture::new("hash-only-pin");
    let runbook_bytes =
        fs::read(fixture.path(".arca/ratmac.toml")).expect("the canonical runbook is readable");
    let expected_hash = sha256_hex(&runbook_bytes);

    let start = fixture.rtm(&["start"]);
    assert!(
        start.status.success(),
        "start must succeed on a valid project: {}",
        combined(&start)
    );

    let roster = fixture.roster();
    assert_eq!(
        roster.len(),
        1,
        "FDC-005: one start mints exactly one run; found {roster:?}"
    );
    let id = roster[0].clone();

    // The pin: the run's evidence records the runbook's SHA-256 at start.
    let evidence_rel = format!(".arca/runs/{id}/evidence.toml");
    let evidence = fs::read_to_string(fixture.path(&evidence_rel)).unwrap_or_else(|error| {
        panic!("FDC-005: the run's evidence must exist at {evidence_rel}: {error}")
    });
    assert!(
        evidence.contains(&expected_hash),
        "FDC-005: rtm start must record the canonical runbook's SHA-256 \
         ({expected_hash}) in the run's evidence; it is absent from {evidence_rel}:\n{evidence}"
    );

    // Hash-only: no per-run copy — no file anywhere in the project carries
    // the runbook's bytes besides the canonical `.arca/ratmac.toml`.
    let copies: Vec<String> = tree_snapshot(&fixture.root)
        .into_iter()
        .filter(|(path, bytes)| bytes == &runbook_bytes && path != ".arca/ratmac.toml")
        .map(|(path, _)| path)
        .collect();
    assert!(
        copies.is_empty(),
        "FDC-005: the pin stays hash-only — no copy of the runbook may exist, found {copies:?}"
    );
    assert!(
        !fixture
            .path(&format!(".arca/runs/{id}/ratmac.toml"))
            .exists(),
        "FDC-005: no per-run runbook file may exist under the run's directory"
    );

    // Drift the runbook by one appended comment: still valid TOML, so a
    // refusal is attributable to the pin, never to a parse error.
    let mut drifted_bytes = runbook_bytes.clone();
    drifted_bytes.extend_from_slice(b"\n# drift: one appended comment\n");
    fs::write(fixture.path(".arca/ratmac.toml"), &drifted_bytes)
        .expect("drift the canonical runbook");
    let observed_hash = sha256_hex(&drifted_bytes);

    let before = tree_snapshot(&fixture.root);
    let step = fixture.rtm(&["step", "--run", &id]);
    assert!(
        !step.status.success(),
        "FDC-005: stepping a run whose recorded pin mismatches the on-disk runbook must refuse; \
         it succeeded: {}",
        combined(&step)
    );
    let text = combined(&step);
    for (role, hash) in [("expected", &expected_hash), ("observed", &observed_hash)] {
        assert!(
            text.contains(hash.as_str()),
            "FDC-005: the pin refusal must name observed and expected identity — the {role} \
             hash {hash} is absent from: {text}"
        );
    }
    let after = tree_snapshot(&fixture.root);
    assert_trees_equal(
        &before,
        &after,
        "FDC-005: the pin refusal must write nothing",
    );
}

/// PT-059-02 (RRV-003): on a plural-layout project carrying a planted flat
/// `.arca/state.toml`, every engine invocation refuses, names the residue
/// path and the repair, and leaves the whole tree byte-identical — nothing
/// migrated, nothing deleted, never a silent adoption.
#[test]
fn flat_layout_residue_refuses_and_instructs() {
    let fixture = Fixture::new("flat-residue");
    let start = fixture.rtm(&["start"]);
    assert!(
        start.status.success(),
        "start must succeed on a valid project: {}",
        combined(&start)
    );
    let id = fixture
        .roster()
        .first()
        .expect("FDC-005: the started run must appear on the roster")
        .clone();

    // Plant the residue: a well-formed pre-plural flat State File. Garbage
    // and sibling-file variants belong to the hidden lane (HT-059-02).
    fs::write(
        fixture.path(".arca/state.toml"),
        "phase = \"intake\"\nstatus = \"planned\"\n",
    )
    .expect("plant the flat-layout residue");

    let before = tree_snapshot(&fixture.root);
    let commands: [&[&str]; 3] = [
        &["start"],
        &["status", "--run", id.as_str()],
        &["step", "--run", id.as_str()],
    ];
    for args in commands {
        let shown = args.join(" ");
        let refused = fixture.rtm(args);
        assert!(
            !refused.status.success(),
            "FDC-005: `rtm {shown}` on a project carrying flat-layout residue must refuse, \
             never adopt it silently: {}",
            combined(&refused)
        );
        let text = combined(&refused).replace('\\', "/").to_lowercase();
        assert!(
            text.contains(".arca/state.toml"),
            "FDC-005: the residue refusal for `rtm {shown}` must name the observed fact — \
             the residue path .arca/state.toml is absent from: {text}"
        );
        assert!(
            ["remove", "delete", "migrate", "move"]
                .iter()
                .any(|verb| text.contains(verb)),
            "FDC-005: the residue refusal for `rtm {shown}` must instruct the repair — \
             no repair instruction (remove/delete/migrate/move) appears in: {text}"
        );
        let after = tree_snapshot(&fixture.root);
        assert_trees_equal(
            &before,
            &after,
            &format!(
                "FDC-005: the residue refusal for `rtm {shown}` must leave the whole tree \
                 byte-identical"
            ),
        );
    }
}
