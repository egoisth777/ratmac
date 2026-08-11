//! t-056 / DRD-001..DRD-007: a doctor that names defects by code.
//!
//! PT-056-01 `each_defect_class_reports_its_code`
//! PT-056-02 `json_output_is_parsable_and_deterministic`
//! PT-056-03 `exit_codes_are_differentiated`
//! PT-056-04 `parse_refusal_is_rendered_as_a_finding`
//! PT-056-05 `ownership_violations_surface_through_the_doctor`
//! PT-056-06 `arbitrary_path_is_diagnosed_read_only`
//! HT-056-01 `the_environment_report_survives_the_deepening`
//! HT-056-02 `hostile_arguments_refuse_by_name`
//! HT-056-03 `a_blocked_route_confers_no_reachability`
//! HT-056-04 `every_invocation_is_write_free`
//! HT-056-05 `emitted_codes_and_documented_codes_are_one_set`
//! HT-056-06 `the_projects_own_runbook_passes_its_own_doctor`
//!
//! Findings are data: a stable code, a severity, a location, a message. The
//! doctor diagnoses through the t-055 parser and keeps no reader of its own.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use ratmac::cli;
use ratmac::doctor::{self, Severity};
use ratmac::machine::MachineClass;
use ratmac_qa::json::Json;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A throwaway directory holding runbooks to diagnose.
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
            "ratmac-t057-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create bench");
        Self { root }
    }

    /// A runbook file at `<bench>/<name>.toml`.
    fn runbook(&self, name: &str, source: &str) -> PathBuf {
        let path = self.root.join(format!("{name}.toml"));
        fs::write(&path, source).expect("write runbook");
        path
    }

    /// A project root whose `.ratmac/ratmac.toml` holds `source`.
    fn project(&self, name: &str, source: &str) -> PathBuf {
        let root = self.root.join(name);
        fs::create_dir_all(root.join(".ratmac")).expect("create project");
        fs::write(root.join(".ratmac/ratmac.toml"), source).expect("write machine class");
        root
    }
}

/// The codes the doctor reports for one runbook source.
fn codes_for(bench: &Bench, name: &str, source: &str) -> BTreeSet<String> {
    let path = bench.runbook(name, source);
    doctor::diagnose(&path)
        .iter()
        .map(|finding| finding.code().to_owned())
        .collect()
}

/// Run `rtm` and return (exit code, output).
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

/// Every code documented in the specification's diagnostics table.
fn documented_codes() -> BTreeSet<String> {
    let text = fs::read_to_string(repo_root().join(".arca/runbook-spec.md"))
        .expect("read .arca/runbook-spec.md");
    let start = text
        .find("## Diagnostics")
        .expect("the specification tables the diagnostics");
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
            && first[2..].chars().all(|c| c.is_ascii_digit())
        {
            codes.insert(first.to_owned());
        }
    }
    assert!(
        codes.len() >= 20,
        "the specification must document the diagnostic vocabulary, found {codes:?}"
    );
    codes
}

/// One runbook per defect class, with the code it must produce.
fn defect_catalogue() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "rb601",
            "RB601",
            "[roots]\nwork = \"../outside\"\n\n[states.a]\nprompt = \"p\"\n",
        ),
        (
            "rb602",
            "RB602",
            "[states.a]\nprompt = \"p\"\nguards = [{ kind = \"files_exact\", root = \"work\", path = \"out\" }]\n",
        ),
        ("rb102", "RB102", "this is not = = toml\n"),
        ("rb103", "RB103", "[states.a]\nprompt = \"p\"\nextra = 1\n"),
        ("rb104", "RB104", "status = \"planned\"\n[states.a]\nprompt = \"p\"\n"),
        ("rb105", "RB105", "[states.a]\n"),
        (
            "rb106",
            "RB106",
            "[states.a]\nprompt = \"p\"\nguards = [{ kind = \"nope\" }]\n",
        ),
        (
            "rb107",
            "RB107",
            "[states.a]\nprompt = \"p\"\nguards = [{ kind = \"intake_contract\", path = \"x\" }]\n",
        ),
        (
            "rb108",
            "RB108",
            "[states.a]\nprompt = \"p\"\n[[transitions]]\nfrom = \"a\"\nto = \"ghost\"\n",
        ),
        (
            "rb109",
            "RB109",
            "[states.a]\nprompt = \"p\"\n[[transitions]]\nfrom = \"a\"\nto = \"a\"\nfreeze = \"tree\"\n",
        ),
        ("rb110", "RB110", "[states.a]\nprompt = 42\n"),
        ("rb111", "RB111", "[phases.a]\nprompt = \"p\"\n"),
        ("rb201", "RB201", "[states]\n"),
        (
            "rb202",
            "RB202",
            "[states.a]\nprompt = \"p\"\n[states.b]\nprompt = \"p\"\n[[transitions]]\nfrom = \"a\"\nto = \"b\"\n[[transitions]]\nfrom = \"b\"\nto = \"a\"\n",
        ),
        (
            "rb203",
            "RB203",
            "[states.a]\nprompt = \"p\"\n[states.b]\nprompt = \"p\"\n",
        ),
        (
            "rb204",
            "RB204",
            "[states.a]\nprompt = \"p\"\n[states.b]\nprompt = \"p\"\n[states.c]\nprompt = \"p\"\n[states.d]\nprompt = \"p\"\n\
             [[transitions]]\nfrom = \"a\"\nto = \"b\"\n[[transitions]]\nfrom = \"c\"\nto = \"d\"\n[[transitions]]\nfrom = \"d\"\nto = \"c\"\n",
        ),
        (
            "rb205",
            "RB205",
            "[states.a]\nprompt = \"p\"\ninputs = [\"b\", \"c\"]\n[states.b]\nprompt = \"p\"\n[states.c]\nprompt = \"p\"\n\
             [[transitions]]\nfrom = \"a\"\nto = \"b\"\ninput = \"b\"\n\
             [[transitions]]\nfrom = \"a\"\nto = \"c\"\ninput = \"c\"\n",
        ),
        (
            "rb206",
            "RB206",
            "[states.a]\nprompt = \"p\"\n[states.b]\nprompt = \"p\"\n\
             [[transitions]]\nfrom = \"a\"\nto = \"b\"\n\
             [[transitions]]\nfrom = \"b\"\nto = \"a\"\nblocked-route = true\n\
             [[transitions]]\nfrom = \"b\"\nto = \"a\"\nblocked-route = true\n",
        ),
        (
            "rb207",
            "RB207",
            "[states.a]\nprompt = \"p\"\n[[transitions]]\nfrom = \"a\"\nto = \"a\"\n",
        ),
        (
            "rb208",
            "RB208",
            "[states.a]\nprompt = \"p\"\ninputs = []\n",
        ),
        (
            "rb209",
            "RB209",
            "[states.a]\nprompt = \"p\"\n[states.b]\nprompt = \"p\"\n[states.c]\nprompt = \"p\"\n\
             [[transitions]]\nfrom = \"a\"\nto = \"b\"\n\
             [[transitions]]\nfrom = \"a\"\nto = \"c\"\n",
        ),
        (
            "rb210",
            "RB210",
            "[states.a]\nprompt = \"p\"\ninputs = [\"b\", \"c\", \"d\"]\n[states.b]\nprompt = \"p\"\n[states.c]\nprompt = \"p\"\n\
             [[transitions]]\nfrom = \"a\"\nto = \"b\"\ninput = \"b\"\n\
             [[transitions]]\nfrom = \"a\"\nto = \"c\"\ninput = \"c\"\n",
        ),
        (
            "rb211",
            "RB211",
            "[states.a]\nprompt = \"p\"\ninputs = [\"b\", \"c\"]\n[states.b]\nprompt = \"p\"\n[states.c]\nprompt = \"p\"\n\
             [[transitions]]\nfrom = \"a\"\nto = \"b\"\ninput = \"b\"\n\
             [[transitions]]\nfrom = \"a\"\nto = \"c\"\ninput = \"b\"\n",
        ),
        (
            "rb212",
            "RB212",
            "[states.a]\nprompt = \"p\"\n[states.b]\nprompt = \"p\"\n\
             [[transitions]]\nfrom = \"a\"\nto = \"b\"\ninput = \"foreign\"\n",
        ),
        (
            "rb213",
            "RB213",
            "[states.a]\nprompt = \"p\"\n[states.b]\nprompt = \"p\"\n\
             [[transitions]]\nfrom = \"a\"\nto = \"b\"\n\
             [[transitions]]\nfrom = \"b\"\nto = \"a\"\nblocked-route = true\ninput = \"hold\"\n",
        ),
        (
            "rb214",
            "RB214",
            "[states.entry]\nprompt = \"p\"\n[states.a]\nprompt = \"p\"\n\
             [states.b]\nprompt = \"p\"\n\
             [[transitions]]\nfrom = \"entry\"\nto = \"a\"\n\
             [[transitions]]\nfrom = \"a\"\nto = \"b\"\n\
             [[transitions]]\nfrom = \"b\"\nto = \"a\"\n",
        ),
        (
            "rb301",
            "RB301",
            "[states.a]\nprompt = \"p\"\nguards = [{ kind = \"command_exit\", program = \"no-such-program-anywhere\", expected = 0 }]\n",
        ),
        (
            "rb302",
            "RB302",
            "[states.a]\nprompt = \"p\"\nguards = [{ kind = \"files_exact\", path = \"out\" }]\n",
        ),
        (
            "rb401",
            "RB401",
            "[states.a]\nprompt = \"Write .ratmac/runs/run-1/run.toml when you are done.\"\n",
        ),
        (
            "rb501",
            "RB501",
            "classes = 1\n[states.a]\nprompt = \"p\"\n",
        ),
        (
            "rb502",
            "RB502",
            "[classes.c]\nbindings = 1\n\n[classes.c.states.x]\nprompt = \"p\"\n\n[states.a]\nprompt = \"p\"\n",
        ),
        (
            "rb503",
            "RB503",
            "[classes.c.states.x]\nprompt = \"p\"\n\n[states.a]\nprompt = \"p\"\nspawns = 1\n",
        ),
        (
            "rb504",
            "RB504",
            "[states.a]\nprompt = \"p\"\n[[states.a.spawns]]\nclass = \"ghost\"\nname = \"g\"\n",
        ),
        (
            "rb505",
            "RB505",
            "[classes.c.bindings.ticket]\nrequired = true\n\n[classes.c.states.x]\nprompt = \"p\"\n\n[states.a]\nprompt = \"p\"\n[[states.a.spawns]]\nclass = \"c\"\nname = \"n\"\n",
        ),
        (
            "rb506",
            "RB506",
            "[states.a]\nprompt = \"p\"\nguards = [{ kind = \"join\", require = \"any_passed\" }]\n",
        ),
    ]
}

/// PT-056-01 / DRD-001, DRD-002, DRD-003: every defect class reports its
/// documented code, at the location where the defect lives.
#[test]
fn each_defect_class_reports_its_code() {
    let bench = Bench::new("catalogue");
    for (name, code, source) in defect_catalogue() {
        let path = bench.runbook(name, source);
        let findings = doctor::diagnose(&path);
        let codes = findings
            .iter()
            .map(|finding| finding.code().to_owned())
            .collect::<BTreeSet<_>>();
        assert!(
            codes.contains(code),
            "DRD-001: {name} must report {code}, reported {codes:?}"
        );
        for finding in &findings {
            assert!(
                !finding.location().is_empty(),
                "DRD-001: {} must name a location",
                finding.code()
            );
            assert!(
                !finding.message().is_empty(),
                "DRD-001: {} must carry a message",
                finding.code()
            );
        }
    }

    // RB101 is about the file, not its contents.
    let missing = bench.root.join("not-here.toml");
    let findings = doctor::diagnose(&missing);
    assert_eq!(
        findings
            .iter()
            .map(|finding| finding.code())
            .collect::<Vec<_>>(),
        vec!["RB101"],
        "DRD-001: an absent runbook is exactly one finding"
    );

    // A location is a place an author can go to: the state, the guard, or the
    // transition that carries the defect.
    let path = bench.runbook(
        "located",
        "[states.build]\nprompt = \"p\"\nguards = [{ kind = \"nope\" }]\n",
    );
    let finding = doctor::diagnose(&path)
        .into_iter()
        .find(|finding| finding.code() == "RB106")
        .expect("the unknown kind is reported");
    assert!(
        finding.location().contains("build"),
        "DRD-001: the location must name the state: {}",
        finding.location()
    );
}

/// PT-056-02 / DRD-006: the machine-readable output is parsable and stable.
#[test]
fn json_output_is_parsable_and_deterministic() {
    let bench = Bench::new("json");
    let path = bench.runbook(
        "defective",
        "[states.a]\nprompt = \"p\"\nguards = [{ kind = \"files_exact\", path = \"out\" }]\n[states.b]\nprompt = \"p\"\n",
    );
    let root = repo_root();
    let shown = path.to_string_lossy().into_owned();
    let (first_code, first) = rtm(&root, &["doctor", "--json", &shown]);
    let (second_code, second) = rtm(&root, &["doctor", "--json", &shown]);
    assert_eq!(first, second, "DRD-006: two runs must be byte-identical");
    assert_eq!(first_code, second_code);

    let value = Json::parse(&first).unwrap_or_else(|error| {
        panic!("DRD-006: --json must emit parsable JSON: {error}\n{first}")
    });
    let findings = value
        .as_object()
        .and_then(|object| object.get("findings"))
        .and_then(Json::as_array)
        .expect("DRD-006: the document carries a findings array");
    assert!(
        !findings.is_empty(),
        "DRD-006: a defective runbook must report findings: {first}"
    );
    for finding in findings {
        for key in ["code", "severity", "location", "message"] {
            let field = finding
                .field(key)
                .unwrap_or_else(|| panic!("DRD-006: every finding carries {key}: {first}"));
            assert!(
                !field.is_empty(),
                "DRD-006: {key} must not be empty: {first}"
            );
        }
        let severity = finding.field("severity").unwrap_or_default();
        assert!(
            severity == "error" || severity == "warning",
            "DRD-006: severity is error or warning, not {severity:?}"
        );
    }
    assert_eq!(
        value.field("exit-code").map(str::to_owned),
        None,
        "DRD-006: the exit code is the process's, not a JSON field to be trusted"
    );
}

/// PT-056-03 / DRD-007: clean, warning, and error runbooks exit differently.
#[test]
fn exit_codes_are_differentiated() {
    let bench = Bench::new("exit");
    let root = repo_root();

    let clean = bench.runbook(
        "clean",
        "[states.a]\nprompt = \"Do the work.\"\n[states.b]\nprompt = \"Done.\"\n[[transitions]]\nfrom = \"a\"\nto = \"b\"\n",
    );
    let warning = bench.runbook(
        "warning",
        "[states.a]\nprompt = \"Do the work.\"\nguards = [{ kind = \"files_exact\", path = \"out\" }]\n[states.b]\nprompt = \"Done.\"\n[[transitions]]\nfrom = \"a\"\nto = \"b\"\n",
    );
    let error = bench.runbook("error", "[states.a]\nprompt = 42\n");

    for (path, expected, label) in [
        (&clean, 0, "a clean runbook"),
        (&warning, 1, "warnings only"),
        (&error, 2, "any error"),
    ] {
        let shown = path.to_string_lossy().into_owned();
        let (code, report) = rtm(&root, &["doctor", &shown]);
        assert_eq!(
            code, expected,
            "DRD-007: {label} must exit {expected}: {report}"
        );
        let (json_code, _) = rtm(&root, &["doctor", "--json", &shown]);
        assert_eq!(
            json_code, expected,
            "DRD-007: --json must not change the verdict for {label}"
        );
    }
}

/// PT-056-04 / DRD-001: a parse refusal becomes a finding, and the doctor
/// keeps no reader of its own.
#[test]
fn parse_refusal_is_rendered_as_a_finding() {
    let bench = Bench::new("parse");
    let source =
        "[states.a]\nprompt = \"p\"\nguards = [{ kind = \"files_exact\", contains = \"x\" }]\n";
    let path = bench.runbook("refused", source);

    let parse_error = ratmac::machine::MachineClass::from_toml(source)
        .expect_err("the parser refuses this runbook");
    let findings = doctor::diagnose(&path);
    assert_eq!(findings.len(), 1, "a parse refusal is one finding");
    let finding = &findings[0];
    assert_eq!(finding.severity(), Severity::Error);
    assert!(
        finding.message().contains(&parse_error.to_string())
            || parse_error.to_string().contains(finding.message()),
        "DRD-001: the doctor must name the defect the parser named:\n  parser: {parse_error}\n  doctor: {}",
        finding.message()
    );

    let source = fs::read_to_string(repo_root().join("src/doctor.rs")).expect("read src/doctor.rs");
    for forbidden in [
        "parse::<toml::Value>",
        "toml::Value =",
        "toml::from_str",
        "toml::map::Map",
    ] {
        assert!(
            !source.contains(forbidden),
            "DRD-001: the doctor must not walk runbook TOML itself ({forbidden})"
        );
    }
}

/// PT-056-05 / DRD-004: an ownership violation is a finding like any other,
/// carrying the message the audit produced.
#[test]
fn ownership_violations_surface_through_the_doctor() {
    let bench = Bench::new("ownership");
    let path = bench.runbook(
        "owned",
        "[states.a]\nprompt = \"Record your progress in .ratmac/runs/run-1/run.toml before you finish.\"\n",
    );
    let finding = doctor::diagnose(&path)
        .into_iter()
        .find(|finding| finding.code() == "RB401")
        .expect("DRD-004: the ownership audit must reach the doctor");
    assert_eq!(finding.severity(), Severity::Error);
    assert!(
        finding.message().contains("run.toml"),
        "DRD-004: the finding must carry the audit's own message: {}",
        finding.message()
    );

    let source = fs::read_to_string(&path).expect("read ownership fixture");
    let class = MachineClass::from_toml(&source).expect("parse ownership fixture");
    let shown = path.to_string_lossy().replace('\\', "/");
    let violations = ratmac::ownership::audit_ownership(&ratmac::ownership::runbook_instructions(
        &class, &shown,
    ))
    .expect_err("the audit refuses this prompt");
    assert!(
        violations
            .iter()
            .any(|violation| finding.message().contains(&violation.to_string())),
        "DRD-004: the doctor must not paraphrase the audit: {}",
        finding.message()
    );
}

/// PT-056-06 / DRD-005: any runbook path can be diagnosed, and diagnosing
/// writes nothing.
#[test]
fn arbitrary_path_is_diagnosed_read_only() {
    let bench = Bench::new("path");
    let outside = bench.runbook("outside", "[states.a]\nprompt = 42\n");
    let shown = outside.to_string_lossy().into_owned();
    let before = fs::read(&outside).expect("read the runbook");

    let root = repo_root();
    let (code, report) = rtm(&root, &["doctor", &shown]);
    assert_eq!(code, 2, "an error-bearing runbook exits 2: {report}");
    assert!(
        report.contains("RB110"),
        "DRD-005: the report names the code: {report}"
    );
    assert_eq!(
        before,
        fs::read(&outside).expect("read the runbook again"),
        "DRD-005: diagnosing writes nothing"
    );

    // Argument-free doctor keeps its environment report (ORS-002).
    let project = bench.project(
        "inside",
        "[states.a]\nprompt = \"Do the work.\"\n[states.b]\nprompt = \"Done.\"\n[[transitions]]\nfrom = \"a\"\nto = \"b\"\n",
    );
    let (code, report) = rtm(&project, &["doctor"]);
    assert_eq!(code, 0, "a clean project exits 0: {report}");
    for line in ["Engine:", "Runbook:", "Run Record:", "Next:"] {
        assert!(
            report.contains(line),
            "ORS-002: the environment report must keep {line}: {report}"
        );
    }
}

/// HT-056-01 (Regression): the ORS-002 contract survives in idle, active, and
/// corrupt projects.
#[test]
fn the_environment_report_survives_the_deepening() {
    let bench = Bench::new("ors002");
    let runbook = "[states.a]\nprompt = \"Do the work.\"\n[states.b]\nprompt = \"Done.\"\n[[transitions]]\nfrom = \"a\"\nto = \"b\"\n";

    let idle = bench.project("idle", runbook);
    let (_, report) = rtm(&idle, &["doctor"]);
    assert!(
        report.contains("Next:"),
        "idle doctor is actionable: {report}"
    );

    // FDC-004: the State File resides in the Engine run directory.
    let active = bench.project("active", runbook);
    fs::create_dir_all(active.join(".ratmac/runs/run-001")).expect("create run directory");
    fs::write(
        active.join(".ratmac/runs/run-001/run.toml"),
        "state = \"a\"\nstatus = \"executing\"\ngoal_revision = \"\"\ninput_revision = \"\"\noutput_revision = \"\"\nactive_refs = []\nblocker = \"\"\n",
    )
    .expect("write state");
    let (_, report) = rtm(&active, &["doctor"]);
    assert!(
        report.contains("state: a"),
        "an active Run is reported: {report}"
    );

    let corrupt = bench.project("corrupt", runbook);
    fs::create_dir_all(corrupt.join(".ratmac/runs/run-001")).expect("create run directory");
    fs::write(
        corrupt.join(".ratmac/runs/run-001/run.toml"),
        "not = = toml\n",
    )
    .expect("write corrupt state");
    let (_, report) = rtm(&corrupt, &["doctor"]);
    assert!(
        report.contains("unreadable"),
        "a corrupt State File is named, not fatal: {report}"
    );
}

/// HT-056-02 (Input/Routing): hostile arguments refuse by name, exit 2, and
/// none of them panics.
#[test]
fn hostile_arguments_refuse_by_name() {
    let bench = Bench::new("hostile");
    let root = repo_root();
    let good = bench
        .runbook("good", "[states.a]\nprompt = \"p\"\n")
        .to_string_lossy()
        .into_owned();
    let directory = bench.root.to_string_lossy().into_owned();

    for args in [
        vec!["doctor", "no-such-file.toml"],
        vec!["doctor", directory.as_str()],
        vec!["doctor", good.as_str(), good.as_str()],
        vec!["doctor", "--verbose"],
        vec!["doctor", "--fix"],
        vec!["doctor", "--json", "--json"],
    ] {
        let (code, report) = rtm(&root, &args);
        assert_eq!(code, 2, "DRD-005: {args:?} must exit 2: {report}");
        assert!(
            !report.trim().is_empty(),
            "DRD-005: {args:?} must say what is wrong"
        );
    }
}

/// HT-056-03 (Lifecycle/Model): reachability agrees with `rtm step` routing -
/// a blocked route is not a way in.
#[test]
fn a_blocked_route_confers_no_reachability() {
    let bench = Bench::new("blocked");
    let codes = codes_for(
        &bench,
        "blocked-route",
        "[states.a]\nprompt = \"p\"\n[states.b]\nprompt = \"p\"\n[states.held]\nprompt = \"p\"\n\
         [[transitions]]\nfrom = \"a\"\nto = \"b\"\n[[transitions]]\nfrom = \"b\"\nto = \"held\"\nblocked-route = true\n",
    );
    assert!(
        codes.contains("RB203"),
        "DRD-002: a state reachable only through a blocked route is a second initial state, not a reachable one: {codes:?}"
    );
}

/// HT-056-04 (Durability/Recovery): every invocation, including the failing
/// ones, leaves the tree byte-identical.
#[test]
fn every_invocation_is_write_free() {
    let bench = Bench::new("readonly");
    let project = bench.project(
        "tree",
        "[states.a]\nprompt = \"p\"\nguards = [{ kind = \"files_exact\", path = \"out\" }]\n",
    );
    let runbook = project.join(".ratmac/ratmac.toml");
    let shown = runbook.to_string_lossy().into_owned();

    let snapshot = |root: &Path| {
        let mut entries = Vec::new();
        collect(root, &mut entries);
        entries
    };
    let before = snapshot(&project);
    for args in [
        vec!["doctor"],
        vec!["doctor", "--json"],
        vec!["doctor", shown.as_str()],
        vec!["doctor", "--json", shown.as_str()],
        vec!["doctor", "no-such-file.toml"],
    ] {
        let _ = rtm(&project, &args);
        assert_eq!(
            before,
            snapshot(&project),
            "DRD-005: {args:?} must write nothing"
        );
    }
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

/// HT-056-05 (Output/Filesystem): the emitted vocabulary and the documented
/// vocabulary are one set - nothing undocumented, nothing unreachable.
#[test]
fn emitted_codes_and_documented_codes_are_one_set() {
    let bench = Bench::new("vocabulary");
    let mut emitted = BTreeSet::new();
    emitted.extend(
        doctor::diagnose(&bench.root.join("absent.toml"))
            .iter()
            .map(|finding| finding.code().to_owned()),
    );
    for (name, _, source) in defect_catalogue() {
        emitted.extend(codes_for(&bench, name, source));
    }
    for (name, source) in [
        (
            "rb603",
            "[roots]\nwork = \"missing\"\n\n[states.a]\nprompt = \"p\"\n",
        ),
        (
            "rb604",
            "[roots]\nwork = \".ratmac\"\n\n[states.a]\nprompt = \"p\"\n",
        ),
    ] {
        let project = bench.project(name, source);
        emitted.extend(
            doctor::diagnose(&project.join(".ratmac/ratmac.toml"))
                .iter()
                .map(|finding| finding.code().to_owned()),
        );
    }
    let documented = documented_codes();
    assert_eq!(
        emitted, documented,
        "DRD-006: every emitted code must be documented and every documented code reachable\n  emitted only: {:?}\n  documented only: {:?}",
        emitted.difference(&documented).collect::<Vec<_>>(),
        documented.difference(&emitted).collect::<Vec<_>>()
    );
}
/// HT-056-06 (Cross-Feature): the project's own runbook passes its own doctor,
/// or reports only warnings this ticket accepted in writing.
#[test]
fn the_projects_own_runbook_passes_its_own_doctor() {
    let findings = doctor::diagnose(&repo_root().join(".ratmac/ratmac.toml"));
    let errors = findings
        .iter()
        .filter(|finding| finding.severity() == Severity::Error)
        .map(|finding| {
            format!(
                "{} {} {}",
                finding.code(),
                finding.location(),
                finding.message()
            )
        })
        .collect::<Vec<_>>();
    assert!(
        errors.is_empty(),
        "DRD-001: the project's own runbook must carry no error findings: {errors:?}"
    );
    for finding in &findings {
        assert_eq!(
            finding.code(),
            "RB302",
            "the only accepted warning is the agent-writable guard the runbook declares on purpose: {} {}",
            finding.code(),
            finding.message()
        );
    }
}
