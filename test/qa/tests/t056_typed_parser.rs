//! t-055 / TRP-001..TRP-006: one typed reader of the runbook.
//!
//! PT-055-01 `unknown_guard_kind_is_a_parse_error`
//! PT-055-02 `per_kind_fields_are_validated_at_parse_time`
//! PT-055-03 `the_runbook_has_exactly_one_reader`
//! PT-055-04 `every_authored_guard_survives_the_parse`
//! PT-055-05 `absent_runbook_refuses_by_name`
//! PT-055-06 `decided_refusals_are_unchanged`
//! HT-055-01 `the_projects_own_runbook_parses_typed`
//! HT-055-02 `hostile_runbooks_refuse_without_panic`
//! HT-055-03 `a_retained_guard_still_refuses_a_real_step`
//! HT-055-04 `refusal_under_a_live_run_mutates_nothing`
//! HT-055-05 `an_unreadable_runbook_is_not_an_absent_one`
//! HT-055-06 `freeze_blocked_route_and_pinning_survive_the_typed_path`
//!
//! The runbook has one reader: `MachineClass`. Guard kinds are closed, each
//! kind owns its fields, every authored guard is retained in order, and an
//! absent runbook refuses by name instead of yielding an empty machine.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use ratmac::machine::{GuardKind, MachineClass};
use ratmac::{cli, Scheduler, StepRequest};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A throwaway project root holding a runbook and nothing else.
struct Project {
    root: PathBuf,
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Project {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t056-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".arca")).expect("create project");
        Self { root }
    }

    fn with_runbook(label: &str, source: &str) -> Self {
        let project = Self::new(label);
        project.write_runbook(source);
        project
    }

    fn write_runbook(&self, source: &str) {
        fs::write(self.root.join(".arca/ratmac.toml"), source).expect("write runbook");
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn rtm(&self, command: &str) -> String {
        let mut output = Vec::new();
        match cli::run_from(["rtm", command], &self.root, &mut output) {
            Ok(()) => String::from_utf8_lossy(&output).into_owned(),
            Err(error) => error.to_string(),
        }
    }
}

/// The guard-kind rows of `.arca/runbook-spec.md`: kind -> (required, optional)
/// field names, read out of the backticked cells. The specification is the
/// authority (RBS-004); this test reads it rather than restating it.
fn spec_field_sets() -> BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> {
    let text = fs::read_to_string(repo_root().join(".arca/runbook-spec.md"))
        .expect("read .arca/runbook-spec.md");
    let start = text
        .find("## Guard kinds")
        .expect("the specification tables the guard kinds");
    let rest = &text[start..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |at| at + 3);
    let mut rows = BTreeMap::new();
    for line in rest[..end].lines() {
        let line = line.trim();
        if !line.starts_with("|") {
            continue;
        }
        let cells = line
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>();
        if cells.len() < 4 {
            continue;
        }
        let kind = cells[0].trim_matches('`');
        if kind == "Kind" || kind.chars().all(|c| c == '-' || c == ':') {
            continue;
        }
        rows.insert(
            kind.to_owned(),
            (backticked(cells[2]), backticked(cells[3])),
        );
    }
    assert!(
        rows.len() >= 7,
        "the specification must table every guard kind, found {}",
        rows.len()
    );
    rows
}

/// Every backticked token in a specification cell.
fn backticked(cell: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = cell;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else { break };
        let token = after[..close].trim();
        if !token.is_empty() && token != "none" {
            found.insert(token.to_owned());
        }
        rest = &after[close + 1..];
    }
    found
}

/// A one-phase runbook whose single guard is the given inline table.
fn runbook_with_guard(guard: &str) -> String {
    format!("[phases.build]\nprompt = \"Build it.\"\nguards = [{guard}]\n")
}

/// The parse error for a runbook, or a panic naming what was expected.
fn refusal(source: &str) -> String {
    match MachineClass::from_toml(source) {
        Ok(_) => panic!("this runbook must refuse:\n{source}"),
        Err(error) => error.to_string(),
    }
}

/// PT-055-01 / TRP-002: an unknown kind is a typed parse error naming the
/// kind, the phase, and the guard's position - never a silently skipped guard.
#[test]
fn unknown_guard_kind_is_a_parse_error() {
    let source = "\
[phases.build]
prompt = \"Build it.\"
guards = [
  { kind = \"files_exact\", path = \"artifacts\" },
  { kind = \"no_such_kind\", path = \"artifacts\" },
]
";
    let message = refusal(source);
    for expected in ["no_such_kind", "build", "1"] {
        assert!(
            message.contains(expected),
            "TRP-002: the refusal must name {expected:?}: {message}"
        );
    }

    let project = Project::with_runbook("unknown-kind", source);
    let error = Scheduler::open(project.path("")).expect_err("an unknown kind builds no machine");
    assert!(
        error.to_string().contains("no_such_kind"),
        "TRP-002: no rtm command may proceed past an unknown kind: {error}"
    );
}

/// PT-055-02 / TRP-003: each kind's own field list is checked where the field
/// is written. The accepted sets are the specification's, not a second copy.
#[test]
fn per_kind_fields_are_validated_at_parse_time() {
    let spec = spec_field_sets();

    for (kind, (required, optional)) in &spec {
        let accepted = GuardKind::accepted_fields(kind)
            .unwrap_or_else(|| panic!("TRP-002: {kind:?} must be a known kind"))
            .iter()
            .map(|field| (*field).to_owned())
            .collect::<BTreeSet<_>>();
        let declared = required.union(optional).cloned().collect::<BTreeSet<_>>();
        assert_eq!(
            accepted, declared,
            "TRP-003: the fields {kind:?} accepts must equal the specification's row"
        );

        // A foreign field refuses where it is written.
        let guard = format!("{{ kind = \"{kind}\", nonesuch = \"x\" }}");
        let message = refusal(&runbook_with_guard(&guard));
        assert!(
            message.contains("nonesuch") && message.contains(kind.as_str()),
            "TRP-003: a field foreign to {kind:?} must refuse naming kind and field: {message}"
        );

        // Every required field is required.
        for missing in required {
            let present = required
                .iter()
                .filter(|field| *field != missing)
                .map(|field| format!(", {field} = {}", sample_value(field)))
                .collect::<String>();
            let guard = format!("{{ kind = \"{kind}\"{present} }}");
            let message = refusal(&runbook_with_guard(&guard));
            assert!(
                message.contains(missing.as_str()) && message.contains(kind.as_str()),
                "TRP-003: {kind:?} without {missing:?} must refuse naming both: {message}"
            );
        }
    }
}

/// A well-typed sample for a specification field name.
fn sample_value(field: &str) -> String {
    match field {
        "expected" => "0".to_owned(),
        "exempt" => "true".to_owned(),
        "args" | "entries" | "files" => "[\"a\"]".to_owned(),
        _ => "\"a\"".to_owned(),
    }
}

/// PT-055-03 / TRP-001: one reader. No module outside the parser turns runbook
/// text into TOML, every module that reads the runbook goes through
/// `MachineClass`, and the Scheduler's guard work takes the typed value.
#[test]
fn the_runbook_has_exactly_one_reader() {
    let src = repo_root().join("src");
    let mut raw_parsers = Vec::new();
    let mut unrouted = Vec::new();
    for entry in fs::read_dir(&src).expect("read src/") {
        let path = entry.expect("read src/ entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        if name == "machine.rs" {
            continue;
        }
        let text = fs::read_to_string(&path).expect("read module");
        let lines = text.lines().collect::<Vec<_>>();
        let mut reads_runbook = false;
        for (index, line) in lines.iter().enumerate() {
            let code = line.trim();
            if code.starts_with("//") {
                continue;
            }
            if !code.contains("ratmac.toml") {
                continue;
            }
            reads_runbook = true;
            // A raw TOML parse in the neighbourhood of the runbook path is a
            // second reader of the runbook, whatever it is called.
            let window = lines[index..lines.len().min(index + 4)].join("\n");
            if window.contains("parse::<toml::Value>")
                || window.contains("toml::Value =")
                || window.contains("toml::from_str")
            {
                raw_parsers.push(format!("{name}:{}: {code}", index + 1));
            }
        }
        if reads_runbook && !text.contains("MachineClass") {
            unrouted.push(name);
        }
    }
    assert!(
        raw_parsers.is_empty(),
        "TRP-001: only src/machine.rs may parse runbook TOML; found:\n{}",
        raw_parsers.join("\n")
    );
    assert!(
        unrouted.is_empty(),
        "TRP-001: every reader of the runbook must go through MachineClass; found: {unrouted:?}"
    );

    let scheduler = fs::read_to_string(src.join("scheduler.rs")).expect("read scheduler.rs");
    assert!(
        !scheduler.contains("toml::map::Map<String, toml::Value>"),
        "TRP-001: guard evaluation must take typed guards, not raw TOML tables"
    );
    for typed in [
        "GuardKind::FilesExact",
        "GuardKind::CommandExit",
        "definition.guards()",
    ] {
        assert!(
            scheduler.contains(typed),
            "TRP-001: the Scheduler must consume the typed class ({typed})"
        );
    }
}

/// PT-055-04 / TRP-004: every authored guard is on the typed class, in
/// declaration order, with the fields the author wrote.
#[test]
fn every_authored_guard_survives_the_parse() {
    let source = "\
[phases.build]
prompt = \"Build it.\"
guards = [
  { kind = \"files_exact\", path = \"artifacts\" },
  { kind = \"files_exact\", path = \"out\", entries = [\"a.txt\", \"b.txt\"] },
  { kind = \"file_contains\", path = \"a.txt\", contains = \"ready\" },
  { kind = \"file_contains\", path = \"b.txt\", contains = \"done\" },
  { kind = \"command_exit\", program = \"rustc\", args = [\"--version\"], expected = 0, exempt = true },
  { kind = \"command_exit\", program = \"cargo\", expected = 1 },
  { kind = \"sensitivity_receipts\", ticket = \".arca/ticket/t-900.md\" },
  { kind = \"completion_gate\", ticket = \".arca/ticket/t-900.md\" },
  { kind = \"intake_contract\" },
  { kind = \"record_contract\" },
]

[phases.review]
prompt = \"Review it.\"
guards = [{ kind = \"record_contract\" }, { kind = \"intake_contract\" }]

[[transitions]]
from = \"build\"
to = \"review\"
";
    let class = MachineClass::from_toml(source).expect("every declared guard is well formed");

    let build = class.phases().get("build").expect("build phase");
    let kinds = build
        .guards()
        .iter()
        .map(GuardKind::name)
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            "files_exact",
            "files_exact",
            "file_contains",
            "file_contains",
            "command_exit",
            "command_exit",
            "sensitivity_receipts",
            "completion_gate",
            "intake_contract",
            "record_contract",
        ],
        "TRP-004: guards are retained in declaration order"
    );

    let review = class.phases().get("review").expect("review phase");
    assert_eq!(
        review
            .guards()
            .iter()
            .map(GuardKind::name)
            .collect::<Vec<_>>(),
        vec!["record_contract", "intake_contract"],
        "TRP-004: order is per phase, not global"
    );

    // The fields survive too, not only the kinds.
    let rendered = build
        .guards()
        .iter()
        .flat_map(GuardKind::rendered_fields)
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>();
    for expected in [
        "path=\"artifacts\"",
        "path=\"out\"",
        "entries=[\"a.txt\", \"b.txt\"]",
        "contains=\"ready\"",
        "program=\"rustc\"",
        "args=[\"--version\"]",
        "expected=0",
        "expected=1",
    ] {
        assert!(
            rendered.iter().any(|field| field == expected),
            "TRP-004: the authored field {expected} must survive: {rendered:?}"
        );
    }
}

/// PT-055-05 / TRP-005: an absent runbook is a named refusal, never an empty
/// machine that lets a command look like it worked.
#[test]
fn absent_runbook_refuses_by_name() {
    let project = Project::new("absent");
    for command in ["start", "status", "step"] {
        let report = project.rtm(command);
        assert!(
            report.contains("ratmac.toml"),
            "TRP-005: {command} must refuse naming the missing runbook: {report}"
        );
        assert!(
            !project.path(".arca/state.toml").exists(),
            "TRP-005: {command} must build no Run from an absent runbook"
        );
    }

    let error =
        Scheduler::open(project.path("")).expect_err("an absent runbook yields no Scheduler");
    assert!(
        error.to_string().contains("ratmac.toml"),
        "TRP-005: the refusal names the runbook: {error}"
    );
}

/// PT-055-06 / TRP-006: the refusals that were already decided still refuse,
/// and still say why.
#[test]
fn decided_refusals_are_unchanged() {
    // R-002 / R-003: status is not a Machine Class dimension.
    for source in [
        "status = \"planned\"\n[phases.build]\nprompt = \"p\"\n",
        "[phases.build]\nprompt = \"p\"\nstatus = \"planned\"\n",
        "[phases.build]\nprompt = \"p\"\n[[transitions]]\nfrom = \"build\"\nto = \"build\"\nstatus = \"planned\"\n",
    ] {
        let message = refusal(source);
        assert!(
            message.contains("status is not a Machine Class dimension"),
            "R-002/R-003: the status refusal must keep its wording: {message}"
        );
    }

    // R-011: unknown keys are hard errors wherever they appear.
    for (source, key) in [
        ("[phases.build]\nprompt = \"p\"\nextra = 1\n", "extra"),
        ("[phases.build]\nprompt = \"p\"\n[bogus]\nx = 1\n", "bogus"),
        (
            "[phases.build]\nprompt = \"p\"\n[[transitions]]\nfrom = \"build\"\nto = \"build\"\nwhen = \"now\"\n",
            "when",
        ),
    ] {
        let message = refusal(source);
        assert!(
            message.contains("unknown key") && message.contains(key),
            "R-011: the unknown-key refusal must name {key:?}: {message}"
        );
    }

    // R-028: a phase without a string prompt still refuses.
    for source in ["[phases.build]\n", "[phases.build]\nprompt = 42\n"] {
        assert!(
            refusal(source).contains("prompt"),
            "R-028: the prompt refusal must name the field"
        );
    }

    // ETB-003: the only freeze is the goal freeze.
    let message = refusal(
        "[phases.build]\nprompt = \"p\"\n[[transitions]]\nfrom = \"build\"\nto = \"build\"\nfreeze = \"tree\"\n",
    );
    assert!(
        message.contains("freeze") && message.contains("goal"),
        "ETB-003: an unknown freeze must refuse naming the only accepted value: {message}"
    );
}

/// HT-055-01 (Regression): the repository's own runbook parses typed, with its
/// guards retained. The suite that follows runs against this same reader.
#[test]
fn the_projects_own_runbook_parses_typed() {
    let source =
        fs::read_to_string(repo_root().join(".arca/ratmac.toml")).expect("read the own runbook");
    let class = MachineClass::from_toml(&source).expect("the project's own runbook is valid");
    let guards = class
        .phases()
        .values()
        .flat_map(|phase| phase.guards())
        .count();
    assert!(
        guards > 0,
        "TRP-004: the project's own guards must survive the parse"
    );
}

/// HT-055-02 (Input/Routing): hostile shapes refuse with a located message and
/// none of them panics.
#[test]
fn hostile_runbooks_refuse_without_panic() {
    let cases = [
        ("[phases.build]\nprompt = \"p\"\nguards = 1\n", "guards"),
        ("[phases.build]\nprompt = \"p\"\nguards = [1]\n", "guard"),
        (
            "[phases.build]\nprompt = \"p\"\nguards = [{ kind = \"\" }]\n",
            "kind",
        ),
        (
            "[phases.build]\nprompt = \"p\"\nguards = [{ kind = 7 }]\n",
            "kind",
        ),
        (
            "[phases.build]\nprompt = \"p\"\nguards = [{ path = \"a\" }]\n",
            "kind",
        ),
        (
            "[phases.build]\nprompt = \"p\"\nguards = [{ kind = \"files_exact\", path = 7 }]\n",
            "path",
        ),
        (
            "[phases.build]\nprompt = \"p\"\nguards = [{ kind = \"command_exit\", program = \"x\", expected = \"0\" }]\n",
            "expected",
        ),
        ("[phases]\n", "phase"),
        ("[phases.\"\"]\nprompt = \"p\"\n", "empty"),
    ];
    for (source, expected) in cases {
        let message = refusal(source);
        assert!(
            message.contains(expected),
            "TRP-002: the refusal for\n{source}\nmust name {expected:?}: {message}"
        );
        assert!(
            message.contains("build") || message.contains("phase") || message.contains("ratmac"),
            "TRP-002: the refusal must locate itself: {message}"
        );
    }
}

/// HT-055-03 (Lifecycle/Model): a retained guard is still the guard that
/// decides a real step, and it names itself when it refuses.
#[test]
fn a_retained_guard_still_refuses_a_real_step() {
    let project = Project::with_runbook(
        "real-step",
        "\
[phases.build]
prompt = \"Build it.\"
guards = [
  { kind = \"files_exact\", path = \"artifacts\" },
  { kind = \"file_contains\", path = \"artifacts/release.txt\", contains = \"ready\" },
]

[phases.done]
prompt = \"Done.\"

[[transitions]]
from = \"build\"
to = \"done\"
",
    );
    fs::create_dir_all(project.path("artifacts")).expect("create artifacts");
    fs::write(project.path("artifacts/release.txt"), "not yet\n").expect("write artifact");

    let mut scheduler = Scheduler::open(project.path("")).expect("open the project");
    scheduler.start().expect("start the Run");
    let outcome = scheduler
        .step(StepRequest::new("built it"))
        .expect("a refused step is not an error");
    let report = format!("{outcome:?}");
    assert!(
        report.contains("file_contains") && report.contains("release.txt"),
        "TRP-004: the retained guard must refuse and name itself: {report}"
    );

    fs::write(project.path("artifacts/release.txt"), "ready\n").expect("satisfy the guard");
    let outcome = scheduler
        .step(StepRequest::new("built it"))
        .expect("the satisfied step advances");
    assert!(
        format!("{outcome:?}").contains("Advanced"),
        "TRP-004: satisfying every retained guard advances the Run"
    );
}

/// HT-055-04 (Durability/Recovery): deleting the runbook under a live Run makes
/// every command refuse, and none of them touches Scheduler-owned state.
#[test]
fn refusal_under_a_live_run_mutates_nothing() {
    let project = Project::with_runbook(
        "live-run",
        "[phases.build]\nprompt = \"Build it.\"\n[phases.done]\nprompt = \"Done.\"\n[[transitions]]\nfrom = \"build\"\nto = \"done\"\n",
    );
    let mut scheduler = Scheduler::open(project.path("")).expect("open the project");
    scheduler.start().expect("start the Run");

    let owned = [".arca/state.toml", ".arca/log.md", ".arca/evidence.toml"];
    let before = owned
        .iter()
        .map(|name| fs::read(project.path(name)).unwrap_or_default())
        .collect::<Vec<_>>();

    fs::remove_file(project.path(".arca/ratmac.toml")).expect("delete the runbook");
    for command in ["status", "step", "start"] {
        let report = project.rtm(command);
        assert!(
            report.contains("ratmac.toml"),
            "TRP-005: {command} must refuse naming the runbook: {report}"
        );
    }

    let after = owned
        .iter()
        .map(|name| fs::read(project.path(name)).unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(
        before, after,
        "R-017: a refusal leaves every Scheduler-owned file byte-identical"
    );
}

/// HT-055-05 (Output/Filesystem): unreadable is not absent, and the refusal
/// says which one it met.
#[test]
fn an_unreadable_runbook_is_not_an_absent_one() {
    let project = Project::new("unreadable");
    fs::create_dir_all(project.path(".arca/ratmac.toml")).expect("runbook path is a directory");

    let error = Scheduler::open(project.path("")).expect_err("a directory is not a runbook");
    let message = error.to_string();
    assert!(
        message.contains("ratmac.toml"),
        "TRP-005: the refusal names the path: {message}"
    );
    assert!(
        !message.contains("absent") && !message.contains("no runbook"),
        "TRP-005: an unreadable runbook must not be reported as absent: {message}"
    );
}

/// HT-055-06 (Cross-Feature): freeze, blocked routes, and pinning all ride on
/// the typed value and behave as they did.
#[test]
fn freeze_blocked_route_and_pinning_survive_the_typed_path() {
    let source = "\
[phases.intake]
prompt = \"Intake.\"
guards = [{ kind = \"command_exit\", program = \"rustc\", args = [\"--version\"], expected = 0, exempt = true }]

[phases.build]
prompt = \"Build.\"

[phases.blocked]
prompt = \"Blocked.\"

[[transitions]]
from = \"intake\"
to = \"build\"
freeze = \"goal\"

[[transitions]]
from = \"build\"
to = \"blocked\"
blocked-route = true
";
    let class = MachineClass::from_toml(source).expect("the cross-feature runbook is valid");
    let freezing = class
        .transitions()
        .iter()
        .filter(|transition| transition.freezes_goal())
        .count();
    assert_eq!(freezing, 1, "ETB-003: the goal freeze survives the parse");
    let blocked = class
        .transitions()
        .iter()
        .filter(|transition| transition.is_blocked_route())
        .count();
    assert_eq!(blocked, 1, "PGE-006: the blocked route survives the parse");

    let intake = class.phases().get("intake").expect("intake phase");
    let guard = intake.guards().first().expect("the pinned guard survives");
    assert_eq!(guard.name(), "command_exit");
    assert!(
        guard.is_exempt(),
        "ETB-001: the exemption marking a toolchain probe survives the parse"
    );

    // A blocked route deliberately confers no inbound edge, so the Run fixture
    // is the ordinary spine: one initial Phase, one freeze, one pinned probe.
    let project = Project::with_runbook(
        "cross-feature",
        "\
[phases.intake]
prompt = \"Intake.\"
guards = [{ kind = \"command_exit\", program = \"rustc\", args = [\"--version\"], expected = 0, exempt = true }]

[phases.build]
prompt = \"Build.\"

[[transitions]]
from = \"intake\"
to = \"build\"
freeze = \"goal\"
",
    );
    // ETB-003: the freeze needs a goal bundle to record.
    fs::create_dir_all(project.path(".arca/goal")).expect("create the goal bundle");
    fs::write(project.path(".arca/goal/spec.md"), "# goal\n").expect("write the goal");

    let mut scheduler = Scheduler::open(project.path("")).expect("open the project");
    scheduler.start().expect("start the Run");
    let outcome = scheduler
        .step(StepRequest::new("probed"))
        .expect("the exempt probe is evaluated, not refused for want of a pin");
    assert!(
        format!("{outcome:?}").contains("Advanced"),
        "ETB-001: an exempt command guard still passes through the typed path"
    );
}

/// The specification's guard-kind rows and the Engine's vocabulary are the same
/// set; t-054 proves it against dispatch, this proves it against the type.
#[test]
fn the_typed_vocabulary_is_the_specifications() {
    let spec = spec_field_sets().keys().cloned().collect::<BTreeSet<_>>();
    let typed = GuardKind::VOCABULARY
        .iter()
        .map(|kind| (*kind).to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        spec, typed,
        "TRP-002: the closed enum and the specification table must be one list"
    );
}
