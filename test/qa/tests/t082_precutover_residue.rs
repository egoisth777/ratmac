//! t-082 / SVC-005, SVC-006: pre-cutover leftovers refuse and instruct.
//!
//! SVCV-003 `a_precutover_runbook_refuses_by_its_own_code`
//! SVCV-005 `every_entry_point_refuses_precutover_records`
//! SVCV-006 `the_documented_table_equals_the_engine_table`
//!
//! A project still written the old way is told so, by name, and nothing is
//! touched: no migration, no Run, no evidence file, no changed byte.

use ratmac::doctor::{self, Severity};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A machine written the settled way.
const VALID_RUNBOOK: &str = "[roots]\n\
     ticket = \".arca/ticket\"\n\n\
     [states.intake]\nprompt = \"Integrate the issues.\"\n\n\
     [states.build]\nprompt = \"Build the ticket.\"\n\n\
     [[transitions]]\nfrom = \"intake\"\nto = \"build\"\n";

/// The same machine, still written the pre-cutover way.
const PRECUTOVER_RUNBOOK: &str = "[roots]\n\
     ticket = \".arca/ticket\"\n\n\
     [phases.intake]\nprompt = \"Integrate the issues.\"\n\n\
     [phases.build]\nprompt = \"Build the ticket.\"\n\n\
     [[transitions]]\nfrom = \"intake\"\nto = \"build\"\n";

/// Every diagnostic code the format carried before this ticket, with the
/// severity it carried. Frozen here on purpose: the row promises each of
/// these keeps its exact identity while State wording changes.
const PRE_CUTOVER_CODES: [(&str, &str); 37] = [
    ("RB101", "error"),
    ("RB102", "error"),
    ("RB103", "error"),
    ("RB104", "error"),
    ("RB105", "error"),
    ("RB106", "error"),
    ("RB107", "error"),
    ("RB108", "error"),
    ("RB109", "error"),
    ("RB110", "error"),
    ("RB201", "error"),
    ("RB202", "error"),
    ("RB203", "error"),
    ("RB204", "error"),
    ("RB205", "warning"),
    ("RB206", "warning"),
    ("RB207", "warning"),
    ("RB208", "error"),
    ("RB209", "error"),
    ("RB210", "error"),
    ("RB211", "error"),
    ("RB212", "error"),
    ("RB213", "error"),
    ("RB214", "error"),
    ("RB301", "error"),
    ("RB302", "warning"),
    ("RB401", "error"),
    ("RB501", "error"),
    ("RB502", "error"),
    ("RB503", "error"),
    ("RB504", "error"),
    ("RB505", "error"),
    ("RB506", "error"),
    ("RB601", "error"),
    ("RB602", "error"),
    ("RB603", "error"),
    ("RB604", "error"),
];

/// The one code this ticket adds.
const NEW_CODE: &str = "RB111";

struct Fixture {
    base: PathBuf,
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

impl Fixture {
    /// A checkout-shaped project carrying the given runbook.
    fn new(label: &str, runbook: &str) -> Self {
        let base = std::env::temp_dir().join(format!(
            "ratmac-t082-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&base);
        let root = base.join("project");
        for dir in [".arca/ticket", ".arca/goal", ".ratmac"] {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        fs::write(root.join(".ratmac/ratmac.toml"), runbook).expect("write the runbook");
        fs::write(root.join(".arca/goal/spec.md"), "# Fixture goal\n").expect("write the goal");
        fs::write(
            root.join(".arca/ticket/t-900.md"),
            "---\nticket-id: t-900\nresidual-ids:\n  - \"res-900\"\nstatus: \"executing\"\n---\n\n\
             # Ticket: t-900\n",
        )
        .expect("write the ticket");
        Fixture { base, root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    fn runbook_path(&self) -> PathBuf {
        self.root.join(".ratmac/ratmac.toml")
    }

    /// Plant a Run directory holding a record under the given filename, with
    /// the position carried by the given field name.
    fn plant_record(&self, run_id: &str, filename: &str, position_field: &str) -> PathBuf {
        let run_dir = self.root.join(".ratmac/runs").join(run_id);
        fs::create_dir_all(&run_dir).expect("create the run directory");
        let record = run_dir.join(filename);
        fs::write(
            &record,
            format!(
                "{position_field} = \"build\"\nstatus = \"executing\"\n\
                 goal_revision = \"\"\ninput_revision = \"\"\noutput_revision = \"\"\n\
                 active_refs = []\nblocker = \"\"\n"
            ),
        )
        .expect("plant the record");
        fs::write(self.root.join(".ratmac/mint.toml"), "highest = 1\n").expect("plant the mint");
        record
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Path, length and content hash of every file below `dir`, so "nothing was
/// touched" can be checked as one comparison.
fn snapshot(dir: &Path) -> BTreeMap<String, (u64, Vec<u8>)> {
    let mut out = BTreeMap::new();
    fn walk(dir: &Path, base: &Path, out: &mut BTreeMap<String, (u64, Vec<u8>)>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, base, out);
            } else if let Ok(bytes) = fs::read(&path) {
                let shown = path
                    .strip_prefix(base)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(shown, (bytes.len() as u64, bytes));
            }
        }
    }
    walk(dir, dir, &mut out);
    out
}

/// The documented diagnostics table: code -> severity word.
fn documented_table() -> BTreeMap<String, String> {
    let spec = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.arca/runbook-spec.md")
        .canonicalize()
        .expect("locate .arca/runbook-spec.md");
    let text = fs::read_to_string(spec).expect("read .arca/runbook-spec.md");
    let start = text
        .find("## Diagnostics")
        .expect("the specification tables the diagnostics");
    let rest = &text[start..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |at| at + 3);
    let mut table = BTreeMap::new();
    for line in rest[..end].lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() < 2 {
            continue;
        }
        let code = cells[0].trim_matches('`');
        if code.len() == 5
            && code.starts_with("RB")
            && code[2..].chars().all(|c| c.is_ascii_digit())
        {
            table.insert(code.to_owned(), cells[1].trim_matches('`').to_owned());
        }
    }
    table
}

/// SVCV-003: a runbook that still declares the pre-cutover table refuses with
/// its own code, names the file and the repair, never falls back to the
/// generic unknown-key code, and starts nothing.
#[test]
fn a_precutover_runbook_refuses_by_its_own_code() {
    let fixture = Fixture::new("precutover-runbook", PRECUTOVER_RUNBOOK);
    let before = snapshot(&fixture.root);

    let findings = doctor::diagnose(&fixture.runbook_path());
    let codes: Vec<&str> = findings.iter().map(doctor::Finding::code).collect();
    assert!(
        codes.contains(&NEW_CODE),
        "SVCV-003: a pre-cutover runbook must report {NEW_CODE}: {codes:?}"
    );
    assert!(
        !codes.contains(&"RB103"),
        "SVCV-003: the dedicated code replaces the generic unknown-key code: {codes:?}"
    );
    let finding = findings
        .iter()
        .find(|finding| finding.code() == NEW_CODE)
        .expect("the reported finding was just located");
    assert_eq!(
        finding.severity(),
        Severity::Error,
        "SVCV-003: the pre-cutover refusal is an error"
    );
    let spoken = format!("{} {}", finding.location(), finding.message());
    assert!(
        spoken.contains("ratmac.toml"),
        "SVCV-003: the refusal names the runbook file: {spoken}"
    );
    assert!(
        spoken.contains("states") && spoken.to_ascii_lowercase().contains("rename"),
        "SVCV-003: the refusal names the repair: {spoken}"
    );

    for args in [
        vec!["doctor"],
        vec!["start"],
        vec!["status"],
        vec!["step", "--run", "run-001"],
    ] {
        let output = fixture.rtm(&args);
        let spoken = text(&output);
        assert!(
            !output.status.success(),
            "SVCV-003: `rtm {}` must refuse a pre-cutover runbook: {spoken}",
            args.join(" ")
        );
        assert!(
            spoken.contains(NEW_CODE),
            "SVCV-003: `rtm {}` must refuse by {NEW_CODE}: {spoken}",
            args.join(" ")
        );
        assert!(
            !spoken.contains("RB103"),
            "SVCV-003: `rtm {}` must not fall back to the generic code: {spoken}",
            args.join(" ")
        );
    }

    assert!(
        !fixture.root.join(".ratmac/runs").exists(),
        "SVCV-003: no Run directory is created for a pre-cutover project"
    );
    assert_eq!(
        snapshot(&fixture.root),
        before,
        "SVCV-003: a refused pre-cutover project is left byte-identical"
    );
}

/// SVCV-005: a Run Record carrying the pre-cutover position field, or sitting
/// at the pre-cutover filename, makes every addressed command refuse by name
/// and change nothing.
#[test]
fn every_entry_point_refuses_precutover_records() {
    let residues: [(&str, &str, &str); 2] = [
        ("old-field", "run.toml", "phase"),
        ("old-filename", "state.toml", "state"),
    ];
    for (label, filename, field) in residues {
        let fixture = Fixture::new(label, VALID_RUNBOOK);
        let record = fixture.plant_record("run-001", filename, field);
        let shown = record.file_name().expect("the record has a name");
        let before = snapshot(&fixture.root);

        for args in [
            vec!["status", "--run", "run-001"],
            vec!["step", "--run", "run-001"],
            vec![
                "hold",
                "t-900",
                "--blocker",
                ".arca/issue/i-777-blocker",
                "--confirm",
                "hold t-900",
                "--run",
                "run-001",
            ],
            vec![
                "abandon",
                "--run",
                "run-001",
                "--confirm",
                "abandon run-001",
            ],
            vec!["start"],
            vec!["status"],
        ] {
            let output = fixture.rtm(&args);
            let spoken = text(&output);
            assert!(
                !output.status.success(),
                "SVCV-005 ({label}): `rtm {}` must refuse pre-cutover residue: {spoken}",
                args.join(" ")
            );
            assert!(
                spoken.contains(&shown.to_string_lossy().to_string()) || spoken.contains("run-001"),
                "SVCV-005 ({label}): `rtm {}` must name the artifact: {spoken}",
                args.join(" ")
            );
            assert!(
                spoken.to_ascii_lowercase().contains("rename")
                    && (spoken.contains("run.toml") || spoken.contains("state")),
                "SVCV-005 ({label}): `rtm {}` must name the repair: {spoken}",
                args.join(" ")
            );
            assert_eq!(
                snapshot(&fixture.root),
                before,
                "SVCV-005 ({label}): `rtm {}` must leave the project byte-identical",
                args.join(" ")
            );
        }
    }
}

/// SVCV-006: the documented code table and the Engine's are one table again -
/// every pre-cutover code keeps its number and severity, and this cutover
/// added exactly one code. Codes a later ticket allocates are not this
/// check's business: it guards the frozen rows against drift, not the size of
/// the table.
#[test]
fn the_documented_table_equals_the_engine_table() {
    let documented = documented_table();
    let mut expected: BTreeMap<String, String> = PRE_CUTOVER_CODES
        .iter()
        .map(|(code, severity)| ((*code).to_owned(), (*severity).to_owned()))
        .collect();
    expected.insert(NEW_CODE.to_owned(), "error".to_owned());
    let carried = documented
        .iter()
        .filter(|(code, _)| expected.contains_key(*code))
        .map(|(code, severity)| (code.clone(), severity.clone()))
        .collect::<BTreeMap<String, String>>();
    assert_eq!(
        carried, expected,
        "SVCV-006: the documented table carries the frozen table plus {NEW_CODE} unchanged"
    );

    // The Engine's own table, read off findings it actually emits.
    let fixture = Fixture::new("engine-table", PRECUTOVER_RUNBOOK);
    let emitted: BTreeSet<String> = doctor::diagnose(&fixture.runbook_path())
        .iter()
        .map(|finding| {
            assert_eq!(
                finding.severity(),
                Severity::Error,
                "SVCV-006: {} keeps the severity the table documents",
                finding.code()
            );
            finding.code().to_owned()
        })
        .collect();
    assert!(
        emitted.contains(NEW_CODE),
        "SVCV-006: the Engine emits {NEW_CODE}: {emitted:?}"
    );
    for code in &emitted {
        assert!(
            documented.contains_key(code),
            "SVCV-006: the Engine emits {code}, which the table does not document"
        );
    }
}
