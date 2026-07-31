//! t-057 / AAL-001..AAL-004: authoring that starts valid and repairs by name.
//!
//! PT-057-01 `scaffold_output_is_doctor_clean`
//! PT-057-02 `seeded_defects_are_repaired_by_code`
//! PT-057-03 `repair_table_covers_exactly_the_engine_codes`
//! PT-057-04 `instructions_define_no_schema`
//! HT-057-01 `the_command_surface_survives_the_new_subcommand`
//! HT-057-02 `scaffold_refuses_every_hostile_path`
//! HT-057-03 `the_scaffold_is_a_runnable_machine`
//! HT-057-04 `a_refused_scaffold_leaves_no_trace`
//! HT-057-05 `scaffolding_creates_exactly_one_file`
//! HT-057-06 `every_documented_code_is_seeded_and_repaired`
//!
//! The drill below is the point of the ticket. A stand-in author starts from
//! `rtm scaffold`, seeds one defect, and then loops: run `rtm doctor --json`,
//! read the finding's **code** and **location**, look the code up in the repair
//! table of `.arca/runbook-authoring.md`, and apply the action that row names.
//! It never reads `src/`, never reads the finding's message, and knows no
//! schema: the code-to-action mapping lives in the instructions, so a wrong
//! code produces a wrong repair and the loop fails to reach clean.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use ratmac::cli;
use ratmac_qa::json::Json;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn instructions_text() -> String {
    fs::read_to_string(repo_root().join(".arca/runbook-authoring.md"))
        .expect("AAL-001: .arca/runbook-authoring.md must exist")
}

fn specification_text() -> String {
    fs::read_to_string(repo_root().join(".arca/runbook-spec.md")).expect("read runbook-spec.md")
}

/// A throwaway directory to author in.
struct Bench {
    root: PathBuf,
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Bench {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t058-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create bench");
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

/// Run `rtm` from `root` and return (exit code, output).
fn rtm(root: &Path, args: &[&str]) -> (i32, String) {
    let mut output = Vec::new();
    let mut argv = vec!["rtm"];
    argv.extend_from_slice(args);
    match cli::run_from(argv, root, &mut output) {
        Ok(code) => (code, String::from_utf8_lossy(&output).into_owned()),
        Err(error) => (
            error.exit_code(),
            format!("{}{error}", String::from_utf8_lossy(&output)),
        ),
    }
}

/// Every entry in a markdown table's first column, for one `## section`.
fn table_codes(text: &str, section: &str) -> BTreeSet<String> {
    let start = text
        .find(section)
        .unwrap_or_else(|| panic!("missing section {section:?}"));
    let rest = &text[start..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |at| at + 3);
    let mut codes = BTreeSet::new();
    for line in rest[..end].lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let first = line
            .trim_matches('|')
            .split('|')
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches('`');
        if first.len() == 5
            && first.starts_with("RB")
            && first[2..]
                .chars()
                .all(|character| character.is_ascii_digit())
        {
            codes.insert(first.to_owned());
        }
    }
    codes
}

/// The repair table, read the way the stand-in reads it: code -> action token.
///
/// The mapping is the instructions' content, not the drill's: change a row and
/// the drill changes with it.
fn repair_actions() -> BTreeMap<String, String> {
    let text = instructions_text();
    let start = text
        .find("## Repair table")
        .expect("AAL-003: the instructions carry a repair table");
    let rest = &text[start..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |at| at + 3);
    let mut actions = BTreeMap::new();
    for line in rest[..end].lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells = line
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>();
        if cells.len() < 3 {
            continue;
        }
        let code = cells[0].trim_matches('`');
        if code.len() != 5 || !code.starts_with("RB") {
            continue;
        }
        let action = cells[cells.len() - 1].trim_matches('`');
        assert!(
            !action.is_empty() && action.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
            "AAL-003: {code} must name one action token, found {action:?}"
        );
        actions.insert(code.to_owned(), action.to_owned());
    }
    actions
}

// ---------------------------------------------------------------------------
// The stand-in author.
// ---------------------------------------------------------------------------

/// Where a finding points. Parsed from the finding's `location` field alone -
/// the stand-in never sees the message.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Spot {
    File,
    Phase(String),
    Guard(String, usize),
    Edge(String, String),
    Index(usize),
}

fn parse_spot(location: &str) -> Spot {
    // `<path> [phases.build] prompt` - the ownership audit's own wording.
    if let Some(at) = location.find("[phases.") {
        let rest = &location[at + "[phases.".len()..];
        if let Some(end) = rest.find(']') {
            return Spot::Phase(rest[..end].to_owned());
        }
    }
    if let Some(rest) = location.strip_prefix("phase ") {
        let mut parts = rest.splitn(2, "\" guard ");
        let name = parts
            .next()
            .unwrap_or_default()
            .trim_matches('"')
            .to_owned();
        if let Some(index) = parts.next() {
            return Spot::Guard(name, index.trim().parse().unwrap_or(0));
        }
        return Spot::Phase(rest.trim_matches('"').to_owned());
    }
    if let Some(rest) = location.strip_prefix("transition ") {
        if let Some((from, to)) = rest.split_once(" -> ") {
            return Spot::Edge(
                from.trim().trim_matches('"').to_owned(),
                to.trim().trim_matches('"').to_owned(),
            );
        }
        if let Ok(index) = rest.trim().parse::<usize>() {
            return Spot::Index(index);
        }
    }
    Spot::File
}

/// The runbook as the author edits it: lines, grouped into `[...]` blocks.
struct Draft {
    lines: Vec<String>,
}

impl Draft {
    fn new(text: &str) -> Self {
        Self {
            lines: text.lines().map(str::to_owned).collect(),
        }
    }

    fn text(&self) -> String {
        let mut out = self.lines.join("\n");
        out.push('\n');
        out
    }

    /// The half-open line range of the block whose header is `header`.
    fn block(&self, header: &str) -> Option<(usize, usize)> {
        let start = self.lines.iter().position(|line| line.trim() == header)?;
        let end = self.lines[start + 1..]
            .iter()
            .position(|line| line.trim_start().starts_with('['))
            .map_or(self.lines.len(), |at| start + 1 + at);
        Some((start, end))
    }

    fn transition_blocks(&self) -> Vec<(usize, usize)> {
        let mut blocks = Vec::new();
        for (index, line) in self.lines.iter().enumerate() {
            if line.trim() == "[[transitions]]" {
                let end = self.lines[index + 1..]
                    .iter()
                    .position(|line| line.trim_start().starts_with('['))
                    .map_or(self.lines.len(), |at| index + 1 + at);
                blocks.push((index, end));
            }
        }
        blocks
    }

    fn field(&self, block: (usize, usize), key: &str) -> Option<String> {
        self.lines[block.0..block.1].iter().find_map(|line| {
            let (name, value) = line.split_once('=')?;
            (name.trim() == key).then(|| value.trim().trim_matches('"').to_owned())
        })
    }

    fn remove(&mut self, block: (usize, usize)) {
        self.lines.drain(block.0..block.1);
    }

    fn append_edge(&mut self, from: &str, to: &str) {
        if !self.lines.last().is_some_and(|line| line.trim().is_empty()) {
            self.lines.push(String::new());
        }
        self.lines.push("[[transitions]]".to_owned());
        self.lines.push(format!("from = \"{from}\""));
        self.lines.push(format!("to = \"{to}\""));
    }

    /// Replace one phase block with the scaffold's version of it, or with a
    /// bare working phase when the scaffold has no such phase.
    fn restore_phase(&mut self, name: &str, scaffold: &str) {
        let header = format!("[phases.{name}]");
        let replacement = Draft::new(scaffold)
            .block(&header)
            .map(|block| Draft::new(scaffold).lines[block.0..block.1].to_vec())
            .unwrap_or_else(|| {
                vec![
                    header.clone(),
                    "prompt = \"Do the work, then report what you produced.\"".to_owned(),
                    String::new(),
                ]
            });
        if let Some(block) = self.block(&header) {
            self.lines.splice(block.0..block.1, replacement);
        } else {
            self.lines.extend(replacement);
        }
    }

    /// Drop the phase and every edge that touches it.
    fn drop_phase(&mut self, name: &str) {
        if let Some(block) = self.block(&format!("[phases.{name}]")) {
            self.remove(block);
        }
        loop {
            let touching = self.transition_blocks().into_iter().find(|block| {
                self.field(*block, "from").as_deref() == Some(name)
                    || self.field(*block, "to").as_deref() == Some(name)
            });
            match touching {
                Some(block) => self.remove(block),
                None => break,
            }
        }
    }

    fn guards_line(&self, phase: &str) -> Option<usize> {
        let block = self.block(&format!("[phases.{phase}]"))?;
        self.lines[block.0..block.1]
            .iter()
            .position(|line| line.trim_start().starts_with("guards"))
            .map(|at| block.0 + at)
    }

    /// The inline guard tables of one phase, as `{ ... }` spans.
    fn guard_spans(line: &str) -> Vec<(usize, usize)> {
        let mut spans = Vec::new();
        let bytes = line.as_bytes();
        let mut start = None;
        for (index, byte) in bytes.iter().enumerate() {
            match byte {
                b'{' => start = Some(index),
                b'}' => {
                    if let Some(open) = start.take() {
                        spans.push((open, index + 1));
                    }
                }
                _ => {}
            }
        }
        spans
    }

    fn drop_guard(&mut self, phase: &str, index: usize) {
        let Some(at) = self.guards_line(phase) else {
            return;
        };
        let line = self.lines[at].clone();
        let spans = Self::guard_spans(&line);
        if spans.len() <= 1 || index >= spans.len() {
            self.lines.remove(at);
            return;
        }
        let mut kept = spans
            .iter()
            .enumerate()
            .filter(|(position, _)| *position != index)
            .map(|(_, span)| line[span.0..span.1].to_owned())
            .collect::<Vec<_>>();
        kept.sort_by_key(|_| 0); // keep declaration order
        self.lines[at] = format!("guards = [{}]", kept.join(", "));
    }

    fn exempt_guard(&mut self, phase: &str, index: usize) {
        let Some(at) = self.guards_line(phase) else {
            return;
        };
        let line = self.lines[at].clone();
        let spans = Self::guard_spans(&line);
        let Some(span) = spans.get(index) else {
            return;
        };
        let guard = &line[span.0..span.1];
        if guard.contains("exempt") {
            return;
        }
        let patched = format!(
            "{}, exempt = true }}",
            guard.trim_end_matches('}').trim_end()
        );
        self.lines[at] = format!("{}{}{}", &line[..span.0], patched, &line[span.1..]);
    }
    fn remove_field(&mut self, block: (usize, usize), key: &str) {
        if let Some(at) = self.lines[block.0..block.1].iter().position(|line| {
            line.split_once('=')
                .is_some_and(|(name, _)| name.trim() == key)
        }) {
            self.lines.remove(block.0 + at);
        }
    }

    fn straighten_branch(&mut self, phase: &str) {
        if let Some(block) = self.block(&format!("[phases.{phase}]")) {
            self.remove_field(block, "inputs");
        }
        let ordinary = self
            .transition_blocks()
            .into_iter()
            .filter(|block| {
                self.field(*block, "from").as_deref() == Some(phase)
                    && self.field(*block, "blocked-route").as_deref() != Some("true")
            })
            .collect::<Vec<_>>();
        for block in ordinary.iter().skip(1).rev() {
            self.remove(*block);
        }
        if let Some(block) = self.transition_blocks().into_iter().find(|block| {
            self.field(*block, "from").as_deref() == Some(phase)
                && self.field(*block, "blocked-route").as_deref() != Some("true")
        }) {
            self.remove_field(block, "input");
        }
    }
}

/// One repair, chosen by code and applied at the location the finding names.
fn apply(
    action: &str,
    spot: &Spot,
    findings: &[(String, String)],
    text: &str,
    scaffold: &str,
) -> String {
    let mut draft = Draft::new(text);
    match action {
        "restore-file" => return scaffold.to_owned(),
        "restore-location" => match spot {
            Spot::Phase(name) => draft.restore_phase(name, scaffold),
            Spot::Guard(phase, _) => {
                if let Some(at) = draft.guards_line(phase) {
                    draft.lines.remove(at);
                }
            }
            _ => return scaffold.to_owned(),
        },
        "drop-transition" => match spot {
            Spot::Edge(from, to) => {
                let doomed = draft.transition_blocks().into_iter().rev().find(|block| {
                    draft.field(*block, "from").as_deref() == Some(from.as_str())
                        && draft.field(*block, "to").as_deref() == Some(to.as_str())
                });
                if let Some(block) = doomed {
                    draft.remove(block);
                }
            }
            Spot::Index(index) => {
                if let Some(block) = draft.transition_blocks().get(*index).copied() {
                    draft.remove(block);
                }
            }
            _ => {}
        },
        "straighten-branch" => {
            let phase = match spot {
                Spot::Phase(name) | Spot::Edge(name, _) => Some(name.as_str()),
                _ => None,
            };
            if let Some(phase) = phase {
                draft.straighten_branch(phase);
            }
        }
        "break-cycle" => {
            if let Some(block) = draft.transition_blocks().into_iter().next_back() {
                draft.remove(block);
            }
        }
        "drop-phase" => {
            if let Spot::Phase(name) = spot {
                draft.drop_phase(name);
            }
        }
        "merge-initial" | "connect-terminal" => {
            if let Spot::Phase(name) = spot {
                // The other findings of the same code name the other ends.
                let partner = findings
                    .iter()
                    .filter_map(|(_, location)| match parse_spot(location) {
                        Spot::Phase(other) if other != *name => Some(other),
                        _ => None,
                    })
                    .next();
                if let Some(partner) = partner {
                    if action == "merge-initial" {
                        draft.append_edge(&partner, name);
                    } else {
                        draft.append_edge(name, &partner);
                    }
                }
            }
        }
        "pin-command" => {
            if let Spot::Guard(phase, index) = spot {
                draft.exempt_guard(phase, *index);
            }
        }
        "drop-guard" => {
            if let Spot::Guard(phase, index) = spot {
                draft.drop_guard(phase, *index);
            }
        }
        other => panic!("the drill does not implement the action {other:?}"),
    }
    draft.text()
}

/// The findings of `rtm doctor --json <path>`, as (code, location) pairs. The
/// message is deliberately dropped here: the stand-in cannot read it.
fn findings(root: &Path, path: &Path) -> (i32, Vec<(String, String)>) {
    let shown = path.to_string_lossy().into_owned();
    let (code, output) = rtm(root, &["doctor", "--json", &shown]);
    let value =
        Json::parse(&output).unwrap_or_else(|error| panic!("doctor JSON: {error}\n{output}"));
    let list = value
        .as_object()
        .and_then(|object| object.get("findings"))
        .and_then(Json::as_array)
        .expect("findings array")
        .iter()
        .map(|finding| {
            (
                finding.field("code").expect("code").to_owned(),
                finding.field("location").expect("location").to_owned(),
            )
        })
        .collect();
    (code, list)
}

/// Run the loop until the doctor is clean, and report which codes were repaired.
fn drive_to_clean(bench: &Bench, name: &str, seeded: &str, scaffold: &str) -> Vec<String> {
    let path = bench.path(name);
    if seeded.is_empty() {
        let _ = fs::remove_file(&path);
    } else {
        fs::write(&path, seeded).expect("seed the runbook");
    }
    let actions = repair_actions();
    let root = repo_root();
    let mut repaired = Vec::new();
    for _ in 0..10 {
        let (exit, list) = if path.is_file() {
            findings(&root, &path)
        } else {
            // A missing file still has a diagnosis, and the table names it.
            findings(&root, &path)
        };
        if exit == 0 {
            assert!(list.is_empty(), "exit 0 means no findings: {list:?}");
            return repaired;
        }
        let (code, location) = list
            .first()
            .cloned()
            .expect("a non-zero exit names a finding");
        let action = actions.get(&code).unwrap_or_else(|| {
            panic!("AAL-003: the repair table has no row for {code}, so the loop cannot proceed")
        });
        let text = fs::read_to_string(&path).unwrap_or_default();
        let repaired_text = apply(action, &parse_spot(&location), &list, &text, scaffold);
        fs::write(&path, repaired_text).expect("write the repair");
        repaired.push(code);
    }
    panic!("AAL-004: the loop did not reach clean; repaired so far: {repaired:?}");
}

/// The scaffold text, produced by the Engine, not by this test.
fn scaffold_text(bench: &Bench, name: &str) -> String {
    let path = bench.path(name);
    let shown = path.to_string_lossy().into_owned();
    let (code, report) = rtm(&repo_root(), &["scaffold", &shown]);
    assert_eq!(code, 0, "AAL-002: scaffolding must succeed: {report}");
    let text = fs::read_to_string(&path).expect("the scaffold is written");
    let _ = fs::remove_file(&path);
    text
}

/// One seeded defect per documented code: (code, the runbook to seed).
fn seeds(scaffold: &str) -> Vec<(&'static str, String)> {
    let plain = |extra: &str| format!("{scaffold}{extra}");
    vec![
        ("RB101", String::new()),
        ("RB102", plain("\nthis is not = = toml\n")),
        ("RB103", scaffold.replace("[phases.build]", "[phases.build]\nextra = 1")),
        ("RB104", format!("status = \"planned\"\n\n{scaffold}")),
        (
            "RB105",
            scaffold.replace(
                "[phases.review]\nprompt = \"Review the work against the ticket and report the verdict.\"",
                "[phases.review]",
            ),
        ),
        (
            "RB106",
            scaffold.replace(
                "[phases.build]",
                "[phases.build]\nguards = [{ kind = \"no_such_kind\" }]",
            ),
        ),
        (
            "RB107",
            scaffold.replace(
                "[phases.build]",
                "[phases.build]\nguards = [{ kind = \"intake_contract\", path = \"somewhere\" }]",
            ),
        ),
        ("RB108", plain("\n[[transitions]]\nfrom = \"review\"\nto = \"ghost\"\n")),
        (
            "RB109",
            plain("\n[[transitions]]\nfrom = \"review\"\nto = \"build\"\nfreeze = \"tree\"\n"),
        ),
        (
            "RB110",
            scaffold.replace(
                "prompt = \"Review the work against the ticket and report the verdict.\"",
                "prompt = 42",
            ),
        ),
        ("RB201", "[phases]\n".to_owned()),
        ("RB202", plain("\n[[transitions]]\nfrom = \"review\"\nto = \"build\"\n")),
        (
            "RB203",
            plain("\n[phases.stray]\nprompt = \"An orphan with no way in.\"\n"),
        ),
        (
            "RB204",
            plain(
                "\n[phases.left]\nprompt = \"An island.\"\n\n[phases.right]\nprompt = \"The other half.\"\n\n\
                 [[transitions]]\nfrom = \"left\"\nto = \"right\"\n\n[[transitions]]\nfrom = \"right\"\nto = \"left\"\n",
            ),
        ),
        (
            "RB205",
            format!(
                "{}\n[phases.done]\nprompt = \"A second ending.\"\n\n\
                 [[transitions]]\nfrom = \"build\"\nto = \"done\"\ninput = \"done\"\n",
                scaffold
                    .replace(
                        "[phases.build]",
                        "[phases.build]\ninputs = [\"review\", \"done\"]"
                    )
                    .replace(
                        "from = \"build\"\nto = \"review\"",
                        "from = \"build\"\nto = \"review\"\ninput = \"review\""
                    )
            ),
        ),
        (
            "RB206",
            plain(
                "\n[[transitions]]\nfrom = \"review\"\nto = \"build\"\nblocked-route = true\n\n\
                 [[transitions]]\nfrom = \"review\"\nto = \"build\"\nblocked-route = true\n",
            ),
        ),
        ("RB207", plain("\n[[transitions]]\nfrom = \"review\"\nto = \"review\"\n")),
        (
            "RB208",
            scaffold.replace("[phases.build]", "[phases.build]\ninputs = []"),
        ),
        (
            "RB209",
            plain("\n[[transitions]]\nfrom = \"build\"\nto = \"review\"\n"),
        ),
        (
            "RB210",
            format!(
                "{}\n[[transitions]]\nfrom = \"build\"\nto = \"review\"\ninput = \"two\"\n",
                scaffold
                    .replace(
                        "[phases.build]",
                        "[phases.build]\ninputs = [\"one\", \"two\", \"three\"]"
                    )
                    .replace(
                        "from = \"build\"\nto = \"review\"",
                        "from = \"build\"\nto = \"review\"\ninput = \"one\""
                    )
            ),
        ),
        (
            "RB211",
            format!(
                "{}\n[[transitions]]\nfrom = \"build\"\nto = \"review\"\ninput = \"one\"\n",
                scaffold
                    .replace(
                        "[phases.build]",
                        "[phases.build]\ninputs = [\"one\", \"two\"]"
                    )
                    .replace(
                        "from = \"build\"\nto = \"review\"",
                        "from = \"build\"\nto = \"review\"\ninput = \"one\""
                    )
            ),
        ),
        (
            "RB212",
            scaffold.replace(
                "from = \"build\"\nto = \"review\"",
                "from = \"build\"\nto = \"review\"\ninput = \"foreign\"",
            ),
        ),
        (
            "RB213",
            plain(
                "\n[[transitions]]\nfrom = \"review\"\nto = \"build\"\nblocked-route = true\ninput = \"hold\"\n",
            ),
        ),
        (
            "RB301",
            scaffold.replace(
                "[phases.build]",
                "[phases.build]\nguards = [{ kind = \"command_exit\", program = \"no-such-program-anywhere\", expected = 0 }]",
            ),
        ),
        (
            "RB302",
            scaffold.replace(
                "[phases.build]",
                "[phases.build]\nguards = [{ kind = \"files_exact\", path = \"artifacts\" }]",
            ),
        ),
        (
            "RB401",
            scaffold.replace(
                "prompt = \"Review the work against the ticket and report the verdict.\"",
                "prompt = \"Record the verdict in .arca/state.toml when you are done.\"",
            ),
        ),
    ]
}

/// PT-057-01 / AAL-002: the scaffold is clean, and it never overwrites.
#[test]
fn scaffold_output_is_doctor_clean() {
    let bench = Bench::new("clean");
    let path = bench.path("fresh.toml");
    let shown = path.to_string_lossy().into_owned();
    let root = repo_root();

    let (code, report) = rtm(&root, &["scaffold", &shown]);
    assert_eq!(code, 0, "AAL-002: scaffolding succeeds: {report}");
    assert!(
        report.contains(&shown) || report.contains("fresh.toml"),
        "AAL-002: the scaffold names what it wrote: {report}"
    );

    let (code, report) = rtm(&root, &["doctor", &shown]);
    assert_eq!(
        code, 0,
        "AAL-002: the scaffold must be doctor-clean: {report}"
    );
    let (_, list) = findings(&root, &path);
    assert!(
        list.is_empty(),
        "AAL-002: zero findings, not just no errors: {list:?}"
    );

    let before = fs::read(&path).expect("read the scaffold");
    let (code, report) = rtm(&root, &["scaffold", &shown]);
    assert_eq!(
        code, 2,
        "AAL-002: scaffolding over a file refuses: {report}"
    );
    assert_eq!(
        before,
        fs::read(&path).expect("read the scaffold again"),
        "AAL-002: the refusal leaves the file byte-identical"
    );
}

/// PT-057-02 / AAL-003, AAL-004: seeded defects are repaired by code.
#[test]
fn seeded_defects_are_repaired_by_code() {
    let bench = Bench::new("repair");
    let scaffold = scaffold_text(&bench, "seed-source.toml");
    for (code, seeded) in seeds(&scaffold) {
        let repaired = drive_to_clean(&bench, &format!("{code}.toml"), &seeded, &scaffold);
        assert!(
            repaired.contains(&code.to_owned()),
            "AAL-004: the {code} defect must be repaired through its own code, not incidentally: {repaired:?}"
        );
    }
}

/// PT-057-03 / AAL-003: the repair table and the Engine's codes are one set.
#[test]
fn repair_table_covers_exactly_the_engine_codes() {
    let documented = table_codes(&specification_text(), "## Diagnostics");
    let repairs = repair_actions().keys().cloned().collect::<BTreeSet<_>>();
    assert!(
        !documented.is_empty(),
        "the specification must table the codes"
    );
    assert_eq!(
        repairs, documented,
        "AAL-003: every code needs a repair row and no row may invent one\n  rows only: {:?}\n  codes only: {:?}",
        repairs.difference(&documented).collect::<Vec<_>>(),
        documented.difference(&repairs).collect::<Vec<_>>()
    );
}

/// PT-057-04 / AAL-001: the instructions define no schema and are routed.
#[test]
fn instructions_define_no_schema() {
    let text = instructions_text();
    let index = fs::read_to_string(repo_root().join(".arca/index.md")).expect("read index");
    assert!(
        index.contains("runbook-authoring.md"),
        "AAL-001: .arca/index.md must route the instructions"
    );

    // Every schema term, taken from the specification rather than a list kept
    // here - a term added there is a term this test starts policing.
    let specification = specification_text();
    let mut terms = BTreeSet::new();
    for line in specification.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let first = line
            .trim_matches('|')
            .split('|')
            .next()
            .unwrap_or_default()
            .trim();
        if first.starts_with('`') && first.ends_with('`') {
            let term = first.trim_matches('`');
            if !term.is_empty()
                && term
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c == '-')
            {
                terms.insert(term.to_owned());
            }
        }
    }
    assert!(
        terms.len() > 10,
        "the specification must name the schema terms it owns: {terms:?}"
    );

    // Spans of link labels that point into the specification. A `[` that is
    // not a link start closes the span at its own `]`, so an unlinked term can
    // never hide inside a runaway span.
    let mut cited = Vec::new();
    let mut at = 0;
    while let Some(open) = text[at..].find('[') {
        let open = at + open;
        at = open + 1;
        let Some(close) = text[open..].find(']') else {
            break;
        };
        let close = open + close;
        if !text[close..].starts_with("](") {
            continue;
        }
        let Some(end) = text[close..].find(')') else {
            break;
        };
        if text[close..close + end].contains("runbook-spec.md") {
            cited.push((open, close));
        }
    }

    for term in &terms {
        let needle = format!("`{term}`");
        let mut at = 0;
        while let Some(found) = text[at..].find(&needle) {
            let found = at + found;
            assert!(
                cited
                    .iter()
                    .any(|(start, end)| found >= *start && found < *end),
                "AAL-001: {needle} is a schema term; the instructions may only name it inside a link into the specification (byte {found})"
            );
            at = found + needle.len();
        }
    }
}

/// HT-057-01 (Regression): the new subcommand disturbs nothing.
#[test]
fn the_command_surface_survives_the_new_subcommand() {
    let root = repo_root();
    for command in [
        "start", "status", "step", "hold", "abandon", "doctor", "scaffold",
    ] {
        let (code, help) = rtm(&root, &[command, "--help"]);
        assert_eq!(code, 0, "{command} --help succeeds");
        assert_eq!(
            help.matches("Usage:").count(),
            1,
            "{command} --help prints exactly one usage: {help}"
        );
    }
    let (_, general) = rtm(&root, &["--help"]);
    assert!(
        general.contains("scaffold"),
        "AAL-002: the command list names scaffold: {general}"
    );
    // A near miss is not the command: the surface names what it offers and
    // nothing that merely resembles it.
    for unknown in ["bogus", "scaffolding", "scaffol"] {
        let (code, _) = rtm(&root, &[unknown]);
        assert_ne!(code, 0, "{unknown} is refused");
    }
}

/// HT-057-02 (Input/Routing): every hostile path refuses by name.
#[test]
fn scaffold_refuses_every_hostile_path() {
    let bench = Bench::new("hostile");
    let root = repo_root();
    let existing = bench.path("taken.toml");
    fs::write(&existing, "keep me\n").expect("write the existing file");
    let directory = bench.root.to_string_lossy().into_owned();
    let orphan = bench
        .path("missing/deeper/runbook.toml")
        .to_string_lossy()
        .into_owned();
    let taken = existing.to_string_lossy().into_owned();

    for args in [
        vec!["scaffold"],
        vec!["scaffold", taken.as_str()],
        vec!["scaffold", directory.as_str()],
        vec!["scaffold", orphan.as_str()],
        vec!["scaffold", taken.as_str(), taken.as_str()],
        vec!["scaffold", "--force", taken.as_str()],
    ] {
        let (code, report) = rtm(&root, &args);
        assert_eq!(code, 2, "AAL-002: {args:?} must refuse: {report}");
        assert!(
            !report.trim().is_empty(),
            "AAL-002: {args:?} must say what is wrong"
        );
    }
    assert_eq!(
        fs::read_to_string(&existing).expect("read the existing file"),
        "keep me\n",
        "AAL-002: a refused scaffold never overwrites"
    );
}

/// HT-057-03 (Lifecycle/Model): the scaffold is a machine, not just a file.
#[test]
fn the_scaffold_is_a_runnable_machine() {
    let bench = Bench::new("runnable");
    let project = bench.path("project");
    fs::create_dir_all(project.join(".arca")).expect("create the project");
    let runbook = project.join(".arca/ratmac.toml");
    let shown = runbook.to_string_lossy().into_owned();
    let (code, report) = rtm(&repo_root(), &["scaffold", &shown]);
    assert_eq!(code, 0, "scaffolding the project runbook: {report}");

    let (code, report) = rtm(&project, &["start"]);
    assert_eq!(code, 0, "AAL-002: the scaffold starts: {report}");
    // FDC-004: address the minted run, read off the plural roster.
    let run_id = fs::read_dir(project.join(".arca/runs"))
        .expect("list the runs roster")
        .map(|entry| entry.expect("roster entry is readable"))
        .find(|entry| entry.path().is_dir())
        .expect("the started run appears on the roster")
        .file_name()
        .to_string_lossy()
        .into_owned();
    let (code, first) = rtm(&project, &["status", "--run", &run_id]);
    assert_eq!(code, 0, "status after start: {first}");
    let (code, stepped) = rtm(&project, &["step", "--run", &run_id]);
    assert_eq!(code, 0, "AAL-002: the scaffold steps: {stepped}");
    assert_ne!(
        first, stepped,
        "AAL-002: the Run actually routes to the next Phase"
    );
}

/// HT-057-04 (Durability/Recovery): a refused scaffold leaves no trace.
#[test]
fn a_refused_scaffold_leaves_no_trace() {
    let bench = Bench::new("trace");
    let existing = bench.path("taken.toml");
    fs::write(&existing, "keep me\n").expect("write the existing file");
    let before = snapshot(&bench.root);

    let root = repo_root();
    let taken = existing.to_string_lossy().into_owned();
    let orphan = bench
        .path("missing/deeper.toml")
        .to_string_lossy()
        .into_owned();
    for target in [taken.as_str(), orphan.as_str()] {
        let (code, report) = rtm(&root, &["scaffold", target]);
        assert_eq!(code, 2, "{target:?} refuses: {report}");
        assert_eq!(before, snapshot(&bench.root), "{target:?} leaves no trace");
    }
}

/// HT-057-05 (Output/Filesystem): scaffolding creates exactly one file.
#[test]
fn scaffolding_creates_exactly_one_file() {
    let bench = Bench::new("output");
    let before = snapshot(&bench.root);
    let path = bench.path("only.toml");
    let (code, _) = rtm(&repo_root(), &["scaffold", &path.to_string_lossy()]);
    assert_eq!(code, 0, "scaffolding succeeds");
    let after = snapshot(&bench.root);
    let created = after
        .iter()
        .filter(|(entry, _)| !before.iter().any(|(old, _)| old == entry))
        .collect::<Vec<_>>();
    assert_eq!(
        created.len(),
        1,
        "AAL-002: exactly one file is created: {:?}",
        created.iter().map(|(entry, _)| entry).collect::<Vec<_>>()
    );
    assert_eq!(created[0].0, path, "AAL-002: at the requested path");
}

/// HT-057-06 (Cross-Feature): every documented code is seeded and repaired.
#[test]
fn every_documented_code_is_seeded_and_repaired() {
    let bench = Bench::new("crossfeature");
    let scaffold = scaffold_text(&bench, "cross-source.toml");
    let documented = table_codes(&specification_text(), "## Diagnostics");
    let seeded = seeds(&scaffold)
        .iter()
        .map(|(code, _)| (*code).to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        seeded, documented,
        "AAL-004: the drill must seed one defect per documented code\n  seeded only: {:?}\n  documented only: {:?}",
        seeded.difference(&documented).collect::<Vec<_>>(),
        documented.difference(&seeded).collect::<Vec<_>>()
    );
}

fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut entries = Vec::new();
    collect(root, &mut entries);
    entries
}

fn collect(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect(&path, out);
        } else {
            out.push((path.clone(), fs::read(&path).unwrap_or_default()));
        }
    }
}
