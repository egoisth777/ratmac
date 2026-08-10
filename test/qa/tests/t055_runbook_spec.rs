//! t-054 / RBS-001..RBS-005: the runbook specification is the written authority.
//!
//! PT-054-01 `specification_is_routed_and_states_the_shape`
//! PT-054-02 `guard_kind_vocabulary_matches_the_engine`
//! PT-054-03 `decided_behavior_is_back_referenced`
//! PT-054-04 `ownership_rules_name_their_enforcer`
//! PT-054-05 `schema_is_defined_in_exactly_one_place`
//! PT-054-06 `ownership_paths_match_canonical_run_residency`
//! HT-054-01 `invented_kind_fails_the_agreement_check`
//! HT-054-02 `specification_is_tracked`
//! HT-054-03 `a_second_enumeration_fails_the_authority_check`
//!
//! The Machine Class format used to be knowable only by reading the parser.
//! These checks keep one document and one Engine saying the same thing: the
//! document may not invent a kind, and no other live `.arca/` document may
//! define one.

use ratmac::machine::GuardKind;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn spec_path() -> PathBuf {
    repo_root().join(".arca/runbook-spec.md")
}

fn spec_text() -> String {
    fs::read_to_string(spec_path()).unwrap_or_else(|error| {
        panic!("RBS-001: .arca/runbook-spec.md must exist and be readable: {error}")
    })
}

/// The rows of a pipe table section, keyed by the heading that introduces it.
fn table_rows(text: &str, heading: &str) -> Vec<Vec<String>> {
    let start = text
        .find(heading)
        .unwrap_or_else(|| panic!("the specification must carry the section {heading:?}"));
    let rest = &text[start + heading.len()..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    rest[..end]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('|'))
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_owned())
                .collect::<Vec<_>>()
        })
        .filter(|cells| {
            !cells
                .iter()
                .all(|cell| cell.chars().all(|c| c == '-' || c == ':'))
        })
        .collect()
}

fn backticked(cell: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = cell;
    while let Some(open) = rest.find('`') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('`') else { break };
        let token = rest[..close].trim().to_owned();
        if !token.is_empty() {
            found.insert(token);
        }
        rest = &rest[close + 1..];
    }
    found
}

/// The guard kinds the Engine accepts, taken from the Engine itself. Since
/// t-055 the vocabulary is a closed type, so the list is that type's - and the
/// parser must build every name on it, or the scrape below says so.
fn engine_guard_kinds() -> BTreeSet<String> {
    let kinds = GuardKind::VOCABULARY
        .iter()
        .map(|kind| (*kind).to_owned())
        .collect::<BTreeSet<_>>();
    assert!(
        kinds.len() >= 5,
        "the Engine must declare a guard vocabulary, found {kinds:?}"
    );
    let parser = fs::read_to_string(repo_root().join("src/machine.rs")).expect("read machine.rs");
    let built = parser
        .split("fn parse_guard(")
        .nth(1)
        .expect("machine.rs must parse guards into the closed type");
    for kind in &kinds {
        assert!(
            built.contains(&format!("\"{kind}\" =>")) || built.contains(&format!("\"{kind}\" |")),
            "RBS-002: the parser must build every kind in the vocabulary; {kind:?} is unbuilt"
        );
        assert!(
            GuardKind::accepted_fields(kind).is_some(),
            "RBS-002: every kind in the vocabulary must declare its fields; {kind:?} does not"
        );
    }
    kinds
}

fn documented_guard_kinds(text: &str) -> BTreeSet<String> {
    table_rows(text, "## Guard kinds")
        .into_iter()
        .filter(|row| row.len() >= 4)
        .filter(|row| row[0] != "Kind")
        .map(|row| row[0].trim_matches('`').to_owned())
        .collect()
}

#[test]
fn specification_is_routed_and_states_the_shape() {
    let text = spec_text();

    let index =
        fs::read_to_string(repo_root().join(".arca/index.md")).expect("read .arca/index.md");
    assert!(
        index.contains("runbook-spec.md"),
        "RBS-001: .arca/index.md must route the runbook specification"
    );

    for section in [
        "## Top level",
        "## States",
        "## Transitions",
        "## Guard kinds",
        "## Ownership",
        "## Diagnostics",
        "## Back-references",
    ] {
        assert!(
            text.contains(section),
            "RBS-001: the specification must carry the section {section:?}"
        );
    }

    let top = table_rows(&text, "## Top level");
    let top_keys = top
        .iter()
        .flat_map(|row| backticked(&row[0]))
        .collect::<BTreeSet<_>>();
    for key in ["states", "transitions"] {
        assert!(
            top_keys.contains(key),
            "RBS-001: the top-level table must state the {key:?} key, found {top_keys:?}"
        );
    }

    let phase_keys = table_rows(&text, "## States")
        .iter()
        .flat_map(|row| backticked(&row[0]))
        .collect::<BTreeSet<_>>();
    for key in ["prompt", "guards"] {
        assert!(
            phase_keys.contains(key),
            "RBS-001: the phase table must state the {key:?} field, found {phase_keys:?}"
        );
    }

    let transition_keys = table_rows(&text, "## Transitions")
        .iter()
        .flat_map(|row| backticked(&row[0]))
        .collect::<BTreeSet<_>>();
    for key in ["from", "to", "freeze", "blocked-route"] {
        assert!(
            transition_keys.contains(key),
            "RBS-001: the transition table must state the {key:?} field, found {transition_keys:?}"
        );
    }
}

#[test]
fn guard_kind_vocabulary_matches_the_engine() {
    let text = spec_text();
    let documented = documented_guard_kinds(&text);
    let engine = engine_guard_kinds();

    assert_eq!(
        documented, engine,
        "RBS-002: the specification's guard-kind table and the Engine's dispatch must be the same set"
    );

    for row in table_rows(&text, "## Guard kinds") {
        if row[0] == "Kind" {
            continue;
        }
        assert!(
            row.len() >= 4,
            "RBS-002: guard-kind row {:?} must state semantics, required fields, and forbidden fields",
            row[0]
        );
        assert!(
            !row[2].is_empty(),
            "RBS-002: guard-kind {:?} must declare its required fields (use `none` when it has none)",
            row[0]
        );
        assert!(
            !row[3].is_empty(),
            "RBS-002: guard-kind {:?} must declare its forbidden or optional fields",
            row[0]
        );
    }
}

#[test]
fn decided_behavior_is_back_referenced() {
    let text = spec_text();
    let rows = table_rows(&text, "## Back-references");
    let referenced = rows
        .iter()
        .flat_map(|row| backticked(&row[0]))
        .collect::<BTreeSet<_>>();
    for requirement in [
        "R-002", "R-003", "R-011", "R-028", "ETB-001", "ETB-002", "ETB-003", "PGE-003", "PGE-005",
        "PGE-006",
    ] {
        assert!(
            referenced.contains(requirement),
            "RBS-005: {requirement} must be back-referenced by the specification, found {referenced:?}"
        );
    }
    for row in &rows {
        if row[0] == "Requirement" {
            continue;
        }
        assert!(
            row.len() >= 2 && !row[1].is_empty(),
            "RBS-005: back-reference {:?} must say which specification statement preserves it",
            row[0]
        );
    }
}

#[test]
fn ownership_rules_name_their_enforcer() {
    let text = spec_text();
    let rows = table_rows(&text, "## Ownership");
    assert!(
        rows.len() > 1,
        "RBS-003: the ownership section must state at least one rule"
    );
    for row in rows {
        if row[0] == "Rule" {
            continue;
        }
        assert!(
            row.len() >= 2,
            "RBS-003: ownership rule {:?} must name its enforcer or be marked prose-only",
            row[0]
        );
        let enforcer = row[1].clone();
        if enforcer.contains("prose-only") {
            continue;
        }
        let symbols = backticked(&enforcer);
        assert!(
            !symbols.is_empty(),
            "RBS-003: ownership rule {:?} must name a concrete enforcer or be marked prose-only",
            row[0]
        );
        for symbol in symbols {
            let file = symbol.split("::").next().unwrap_or_default().to_owned();
            let candidate = repo_root().join("src").join(format!("{file}.rs"));
            assert!(
                candidate.is_file(),
                "RBS-003: ownership rule {:?} names enforcer {symbol:?}, but src/{file}.rs does not exist",
                row[0]
            );
            if let Some(item) = symbol.split("::").nth(1) {
                let source = fs::read_to_string(&candidate).expect("read enforcer source");
                assert!(
                    source.contains(item),
                    "RBS-003: src/{file}.rs does not define {item:?}"
                );
            }
        }
    }
}

/// PT-054-06 / RBSV-004: ownership uses the canonical Engine root for Machine
/// Class/history/lock files and per-Run state/evidence. It also names the
/// enforcers that superseded the original prose-only claims.
#[test]
fn ownership_paths_match_canonical_run_residency() {
    let text = spec_text();
    let rows = table_rows(&text, "## Ownership");
    let ownership = rows
        .iter()
        .map(|row| row.join(" | "))
        .collect::<Vec<_>>()
        .join("\n");

    for path in [
        ".ratmac/ratmac.toml",
        ".ratmac/log.md",
        ".ratmac/locks/root.lock",
        ".ratmac/runs/<id>/state.toml",
        ".ratmac/runs/<id>/evidence.toml",
        ".ratmac/evidence/<run-id>/",
    ] {
        assert!(
            ownership.contains(path),
            "RBSV-004: ownership must name the canonical path {path}"
        );
    }
    for stale in [
        "`.arca/state.toml`",
        "`.arca/evidence.toml`",
        "`.arca/runs/<id>/state.toml`",
        "`.arca/ratmac.toml`",
    ] {
        assert!(
            !ownership.contains(stale),
            "RBSV-004: ownership must not claim the superseded path {stale}"
        );
    }

    assert!(
        ownership.contains("`scaffold::write_scaffold`")
            && !ownership.contains("no writer of the runbook exists"),
        "RBSV-004: ownership must name the real scaffold writer instead of claiming none exists"
    );
    let writable_verdict_row = rows
        .iter()
        .find(|row| {
            row.first()
                .is_some_and(|rule| rule.contains("verdict rests"))
        })
        .expect("agent-writable verdict ownership row");
    assert!(
        writable_verdict_row[1].contains("`doctor::lint_guards`")
            && !writable_verdict_row[1].contains("prose-only"),
        "RBSV-004: RB302 is mechanically enforced by doctor::lint_guards"
    );
}

/// RBS-004: exactly one place defines the schema. A definition is a per-kind
/// table row — a row whose first cell is a guard kind, which is how a document
/// says "here is what this kind is". Prose that cites a kind while making some
/// other point (a requirement record, a gap record) is a citation, not a
/// second definition, and is left alone.
#[test]
fn schema_is_defined_in_exactly_one_place() {
    let kinds = engine_guard_kinds();
    let mut offenders = Vec::new();
    walk(&repo_root().join(".arca"), &mut |path| {
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            return;
        }
        let shown = path
            .strip_prefix(repo_root())
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        // Historical roots are byte-preserved provenance, and the log is
        // append-only: neither is a live definition.
        if shown.contains("/archive/") || shown.ends_with(".arca/log.md") {
            return;
        }
        if shown.ends_with(".arca/runbook-spec.md") {
            return;
        }
        let text = fs::read_to_string(path).unwrap_or_default();
        for (number, line) in text.lines().enumerate() {
            if let Some(cell) = first_table_cell(line) {
                if kinds.iter().any(|kind| cell == *kind) {
                    offenders.push(format!("{shown}:{}: {}", number + 1, line.trim()));
                }
            }
        }
    });
    assert!(
        offenders.is_empty(),
        "RBS-004: only .arca/runbook-spec.md may define guard kinds row by row; found:\n{}",
        offenders.join("\n")
    );
}

/// The first cell of a markdown table row, unwrapped from code span and
/// alternation (`a` / `b` counts as a definition of `a`).
fn first_table_cell(line: &str) -> Option<String> {
    let line = line.trim();
    if !line.starts_with('|') {
        return None;
    }
    let cell = line.trim_start_matches('|').split('|').next()?.trim();
    let head = cell.split('/').next()?.trim();
    Some(head.trim_matches('`').trim().to_owned())
}

#[test]
fn invented_kind_fails_the_agreement_check() {
    let mut documented = documented_guard_kinds(&spec_text());
    documented.insert("totally_invented_kind".to_owned());
    assert_ne!(
        documented,
        engine_guard_kinds(),
        "HT-054-01: an invented kind must break the agreement check"
    );
}

#[test]
fn specification_is_tracked() {
    let output = Command::new("git")
        .args(["ls-files", "--error-unmatch", ".arca/runbook-spec.md"])
        .current_dir(repo_root())
        .output()
        .expect("run git ls-files");
    assert!(
        output.status.success(),
        "HT-054-02 / AOI-001: the specification must be tracked, not untracked scratch: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
}

#[test]
fn a_second_enumeration_fails_the_authority_check() {
    let kind = engine_guard_kinds()
        .into_iter()
        .next()
        .expect("the Engine dispatches at least one kind");
    let seeded = format!("| `{kind}` | a second definition of the same kind | none |");
    assert_eq!(
        first_table_cell(&seeded).as_deref(),
        Some(kind.as_str()),
        "HT-054-03: a per-kind table row elsewhere must be detected as a second definition"
    );
    assert!(
        first_table_cell("| DRD-003 | prose citing `command_exit` and `files_exact` | ok |")
            .is_some_and(|cell| cell == "DRD-003"),
        "HT-054-03: a requirement row citing kinds must not be mistaken for a definition"
    );
}

fn walk(dir: &Path, visit: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut paths = entries
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            walk(&path, visit);
        } else {
            visit(&path);
        }
    }
}
