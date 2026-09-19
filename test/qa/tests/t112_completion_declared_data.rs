//! t-112 / CGD-003: completion reads runbook-mapped declarations, never
//! prose shape.
//!
//! CGDV-003 `runbook_selected_fields_drive_the_same_ordered_gate`
//! CGDV-004 `completion_has_no_prose_discovery_boundary`
//! CGDV-005 `declared_list_errors_refuse_before_any_mutation`
//! CGDV-006 `historical_receipt_contract_is_unchanged`
//! CGDV-007 `heading_free_non_markdown_carrier_gates`
//!
//! Every check below drives the shipped `rtm` command inside throwaway
//! fixture roots under the system temporary directory, so nothing here can
//! resolve this repository's own `.ratmac`. Receipts are written with the
//! unchanged receipt utilities (`sha256_text`, `tree_digest`, `check_slug`)
//! in the format the gate already reads, and no Rust surface the cutover
//! will add is referenced: the whole file compiles against the pre-cutover
//! Engine, so its red baseline is behavioral (the declaration mapping group
//! and its guarantees do not exist yet), never a missing API. Every refusal
//! group first demands one fully valid configured execution, so a legacy
//! unknown-key parse refusal cannot masquerade as a passing negative.

use ratmac::completion::{check_slug, tree_digest};
use ratmac::receipt::sha256_text;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A green run's recorded output, the bytes its digest is derived from.
const GREEN: &str = "test result: ok. 3 passed; 0 failed\n";

/// A red baseline run's recorded output for the sensitivity receipt.
const FAILED: &str = "test cgdv606_probe ... FAILED\n";

/// One green, fresh completion receipt set for a `.data` carrier.
const DATA_CHECKS: &[(&str, &str)] = &[
    ("G-907-01", "focused"),
    ("S-907-01", "hidden-lane"),
    ("rtm probe alpha", "quality"),
];

struct Fixture {
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    /// A throwaway project root outside this repository, carrying the
    /// directories one case needs plus the source tree receipts describe.
    fn create(label: &str, dirs: &[&str]) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t112-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in dirs {
            fs::create_dir_all(root.join(dir)).expect("create fixture tree");
        }
        fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").expect("write fixture source");
        Self { root }
    }

    fn write_runbook(&self, runbook: &str) {
        fs::write(self.root.join(".ratmac/ratmac.toml"), runbook).expect("write fixture runbook");
    }

    fn write_file(&self, relative: &str, contents: &str) {
        if let Some(parent) = Path::new(relative).parent() {
            fs::create_dir_all(self.root.join(parent)).expect("create fixture directory");
        }
        fs::write(self.root.join(relative), contents).expect("write fixture file");
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(ratmac_qa::engine_bin!())
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    fn text(&self, args: &[&str]) -> String {
        combined(&self.rtm(args))
    }

    /// `rtm start`, returning the minted run id.
    fn start(&self) -> String {
        let output = self.rtm(&["start"]);
        let text = combined(&output);
        assert!(output.status.success(), "rtm start succeeds: {text}");
        text.split("started run ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .expect("start names the minted run id")
            .to_owned()
    }

    /// `rtm spawn`, binding one name; the spawn word is the declared spawn
    /// name, never the class name.
    fn spawn(&self, parent: &str, name: &str, binding: &str, value: &str) -> String {
        let pair = format!("{binding}={value}");
        let output = self.rtm(&["spawn", name, "--run", parent, "--bind", &pair]);
        let text = combined(&output);
        assert!(output.status.success(), "spawn succeeds: {text}");
        text.split("spawned run ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .expect("spawn names the child run id")
            .to_owned()
    }

    /// One refused `rtm step`, proving the refusal happened.
    fn refuse_step(&self, run: &str) -> String {
        let text = self.text(&["step", "--run", run]);
        assert!(
            text.contains("step refused"),
            "the step refuses; got:\n{text}"
        );
        text
    }

    /// One successful `rtm step`.
    fn pass_step(&self, run: &str) -> String {
        let output = self.rtm(&["step", "--run", run]);
        let text = combined(&output);
        assert!(
            output.status.success() && !text.contains("step refused"),
            "the step succeeds; got:\n{text}"
        );
        text
    }

    fn in_state(&self, run: &str, state: &str) {
        let report = self.text(&["status", "--run", run]);
        assert!(
            report.contains(&format!("State: {state}")),
            "the Run stands in {state}; got:\n{report}"
        );
    }

    fn receipt_path(&self, run: &str, check: &str) -> PathBuf {
        self.root
            .join(format!(".ratmac/evidence/{run}/completion"))
            .join(format!("{}.toml", check_slug(check)))
    }

    /// One completion receipt in the format the gate already reads, fresh
    /// against the source tree as it stands right now.
    fn write_receipt(&self, run: &str, ticket: &str, check: &str, kind: &str, exit: i64) {
        let directory = self.root.join(format!(".ratmac/evidence/{run}/completion"));
        fs::create_dir_all(&directory).expect("create evidence directory");
        let digest =
            tree_digest(&self.root, &["src".to_owned()]).expect("source roots are readable");
        let body = format!(
            "ticket-id = \"{ticket}\"\n\
             check-id = \"{check}\"\n\
             kind = \"{kind}\"\n\
             command = \"cargo --version\"\n\
             working-dir = \".\"\n\
             exit-status = {exit}\n\
             output-sha256 = \"{}\"\n\
             tree-roots = [\"src\"]\n\
             tree-sha256 = \"{digest}\"\n\
             output = \"\"\"\n{GREEN}\"\"\"\n",
            sha256_text(GREEN)
        );
        fs::write(self.receipt_path(run, check), body).expect("write completion receipt");
    }

    /// Replace the addressed Run's whole completion receipt set.
    fn record_all(&self, run: &str, ticket: &str, checks: &[(&str, &str)]) {
        let _ = fs::remove_dir_all(self.root.join(format!(".ratmac/evidence/{run}/completion")));
        for (check, kind) in checks {
            self.write_receipt(run, ticket, check, kind, 0);
        }
    }

    /// One sensitivity receipt, flat in the addressed Run's evidence
    /// directory, naming a test that exists as a runnable fn.
    fn write_sensitivity(&self, run: &str, ticket: &str, planned: &str) {
        let directory = self.root.join(format!(".ratmac/evidence/{run}"));
        fs::create_dir_all(&directory).expect("create evidence directory");
        let body = format!(
            "planned-test-id = \"{planned}\"\n\
             ticket-id = \"{ticket}\"\n\
             kind = \"baseline-failure\"\n\
             command = \"cargo test --offline --workspace\"\n\
             working-dir = \".\"\n\
             test-file = \"lanes/fake.rs\"\n\
             test-name = \"cgdv606_probe\"\n\
             exit-status = 101\n\
             output-sha256 = \"{}\"\n\
             output = \"\"\"\n{FAILED}\"\"\"\n",
            sha256_text(FAILED)
        );
        fs::write(
            directory.join(format!("{}.toml", check_slug(planned))),
            body,
        )
        .expect("write sensitivity receipt");
    }
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Every file under `root` with its bytes, so an absence claim is provable.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(bytes) = fs::read(&path) {
                files.insert(path, bytes);
            }
        }
    }
    files
}

/// Each id's first position in a refusal, so two refusals' derived check
/// orders compare without depending on the surrounding words.
fn order_of(text: &str, ids: &[&str]) -> Vec<usize> {
    ids.iter()
        .map(|id| {
            text.find(id)
                .unwrap_or_else(|| panic!("the refusal names {id}; got:\n{text}"))
        })
        .collect()
}

/// The positions strictly increase: focused first, then hidden, then quality.
fn assert_increasing(order: &[usize], text: &str) {
    for pair in order.windows(2) {
        assert!(
            pair[0] < pair[1],
            "the checks derive focused, then hidden, then quality; got:\n{text}"
        );
    }
}

/// One repository file's text, addressed from this crate's manifest
/// directory.
fn repository(relative: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

/// The plain two-State machine whose exit is one authored guard.
fn gated_runbook(guard: &str) -> String {
    format!(
        "[states.work]\nprompt = \"Work the item.\"\nguards = [{guard}]\n\n\
         [states.done]\nprompt = \"Finish.\"\n\n\
         [[transitions]]\nfrom = \"work\"\nto = \"done\"\n"
    )
}

/// A completion guard whose declaration mapping selects the given fields.
fn mapped_guard(ticket: &str, focused: &str, hidden: &str, quality: &str) -> String {
    format!(
        "{{ kind = \"completion_gate\", ticket = \"{ticket}\", \
         declaration-format = \"front-matter-string-lists\", \
         focused-field = \"{focused}\", hidden-lane-field = \"{hidden}\", \
         quality-field = \"{quality}\" }}"
    )
}

/// A completion guard carrying raw authored TOML after the kind and address,
/// for the machine-shape group cases.
fn raw_guard(ticket: &str, raw: &str) -> String {
    format!("{{ kind = \"completion_gate\", ticket = \"{ticket}\", {raw} }}")
}

/// The declaration carrier of twin A: legacy fields ride along as unselected
/// data, the body has no heading and no backtick.
const TWIN_A: &str = r#"---
ticket-id: t-903
planned-test-refs:
  - "PT-LEGACY-9"
planned-works:
  - "W-903-01"
  - "W-903-02"
secret-lanes:
  - "L-903-01"
commands-of-record:
  - "rtm probe alpha"
---

Plain body prose with no heading and no backtick. The checks live only in the
selected lists above this line.
"#;

/// The declaration carrier of twin B: differently named selected fields in a
/// permuted order, one renamed heading, the same list values.
const TWIN_B: &str = r#"---
ticket-id: t-904
zeta-quality:
  - "rtm probe alpha"
zeta-focused:
  - "W-903-01"
  - "W-903-02"
zeta-hidden:
  - "L-903-01"
---

# Journal

No checklist heading, no lane table, nothing shaped like a Merge Gate.
"#;

/// A carrier whose body carries every prose shape the legacy path scraped:
/// a Merge Gate section, backticked lane-shaped ids, a backticked command,
/// the workflow's own field names, and body-lines shaped like the legacy
/// planned-test list.
const DECOY_CARRIER: &str = r#"---
ticket-id: t-902
works:
  - "W-404-01"
lanes:
  - "L-404-01"
gates:
  - "rtm probe alpha"
---

## Merge Gate

- Quality: `legacy-decoy-command --run` passes.
- Hidden lane `HT-777-01` and its twin `HT-777-02` stay green.

The shop words focused-tests, hidden-lanes, and quality-commands appear here
as prose and nothing else.

planned-test-refs:
  - "PT-BODY-9"
"#;

/// The aged fixture's carrier: the legacy planned list still feeds
/// sensitivity, the mapped lists feed completion.
const AGED_CARRIER: &str = r#"---
ticket-id: t-606
planned-test-refs:
  - "PT-606-01"
works:
  - "W-606-01"
lanes:
  - "L-606-01"
gates:
  - "cargo --version"
---

Plain body prose. Sensitivity reads the planned tests; completion reads the
mapped lists.
"#;

/// A heading-free, non-Markdown carrier behind an arbitrary extension.
const DATA_CARRIER: &str = r#"---
musts:
  - "G-907-01"
shadows:
  - "S-907-01"
commands:
  - "rtm probe alpha"
---
Plain bytes only. No heading, no backtick, no markdown, no ticket concept.
"#;

/// CGDV-003: two runbooks that select differently named field triplets
/// derive the same verbatim ordered checks and the same verdict from
/// carriers holding the same list values, whatever the headings say.
#[test]
fn runbook_selected_fields_drive_the_same_ordered_gate() {
    const IDS: [&str; 4] = ["W-903-01", "W-903-02", "L-903-01", "rtm probe alpha"];

    // Twin A: `planned-works` / `secret-lanes` / `commands-of-record`.
    let a = Fixture::create("order-a", &[".ratmac", "src", ".arca/ticket"]);
    a.write_runbook(&gated_runbook(&mapped_guard(
        ".arca/ticket/t-903.md",
        "planned-works",
        "secret-lanes",
        "commands-of-record",
    )));
    a.write_file(".arca/ticket/t-903.md", TWIN_A);
    let doctor = a.rtm(&["doctor"]);
    assert!(
        doctor.status.success(),
        "the mapped runbook is a valid Machine Class; got:\n{}",
        combined(&doctor)
    );
    let run_a = a.start();
    let missing_a = a.refuse_step(&run_a);
    for id in IDS {
        assert!(
            missing_a.contains(id),
            "the refusal names the declared check {id}; got:\n{missing_a}"
        );
    }
    assert!(
        !missing_a.contains("PT-LEGACY-9"),
        "the legacy planned field is not a completion source; got:\n{missing_a}"
    );
    let order_a = order_of(&missing_a, &IDS);
    assert_increasing(&order_a, &missing_a);

    a.record_all(
        &run_a,
        "t-903",
        &[
            ("W-903-01", "focused"),
            ("W-903-02", "focused"),
            ("L-903-01", "hidden-lane"),
            ("rtm probe alpha", "quality"),
        ],
    );
    a.pass_step(&run_a);
    a.in_state(&run_a, "done");

    // Twin B: `zeta-focused` / `zeta-hidden` / `zeta-quality`, the fields
    // permuted inside the carrier and every heading renamed or removed.
    let b = Fixture::create("order-b", &[".ratmac", "src", ".arca/ticket"]);
    b.write_runbook(&gated_runbook(&mapped_guard(
        ".arca/ticket/t-904.md",
        "zeta-focused",
        "zeta-hidden",
        "zeta-quality",
    )));
    b.write_file(".arca/ticket/t-904.md", TWIN_B);
    let run_b = b.start();
    let missing_b = b.refuse_step(&run_b);
    let order_b = order_of(&missing_b, &IDS);
    assert_increasing(&order_b, &missing_b);
    assert_eq!(
        order_a, order_b,
        "renaming and moving the selected fields changes nothing but the \
         authored mapping; twin A refused:\n{missing_a}\ntwin B refused:\n{missing_b}"
    );

    b.record_all(
        &run_b,
        "t-904",
        &[
            ("W-903-01", "focused"),
            ("W-903-02", "focused"),
            ("L-903-01", "hidden-lane"),
            ("rtm probe alpha", "quality"),
        ],
    );
    b.pass_step(&run_b);
    b.in_state(&run_b, "done");
}

/// CGDV-004: only the runbook-selected lists reach completion - no Merge
/// Gate split, lane-shape match, or backtick scan in production, and the
/// Engine and the QA adapter read one generic reader.
#[test]
fn completion_has_no_prose_discovery_boundary() {
    // One valid configured execution first: the selected checks alone, with
    // receipts for exactly them, complete the ticket.
    let fixture = Fixture::create("prose", &[".ratmac", "src", ".arca/ticket"]);
    fixture.write_runbook(&gated_runbook(&mapped_guard(
        ".arca/ticket/t-902.md",
        "works",
        "lanes",
        "gates",
    )));
    fixture.write_file(".arca/ticket/t-902.md", DECOY_CARRIER);
    let run = fixture.start();
    fixture.record_all(
        &run,
        "t-902",
        &[
            ("W-404-01", "focused"),
            ("L-404-01", "hidden-lane"),
            ("rtm probe alpha", "quality"),
        ],
    );
    fixture.pass_step(&run);
    fixture.in_state(&run, "done");

    // A second Run over the same decoy carrier, with no receipts: the
    // refusal names exactly the selected ids and not one decoy. Had any
    // prose inference survived, the decoy commands and lane shapes would be
    // missing receipts here.
    let dry = fixture.start();
    let refusal = fixture.refuse_step(&dry);
    for id in ["W-404-01", "L-404-01", "rtm probe alpha"] {
        assert!(
            refusal.contains(id),
            "the refusal names the selected check {id}; got:\n{refusal}"
        );
    }
    for decoy in [
        "HT-777-01",
        "HT-777-02",
        "legacy-decoy-command",
        "PT-BODY-9",
    ] {
        assert!(
            !refusal.contains(decoy),
            "no prose shape reaches completion, so {decoy} cannot be named; got:\n{refusal}"
        );
    }

    // The production completion path scrapes no prose: the legacy helpers
    // and the Merge Gate split are gone from `src/completion.rs`.
    let completion = repository("../../src/completion.rs");
    for marker in [
        "merge_gate_commands",
        "hidden_lane_ids",
        "backticked",
        "## Merge Gate",
    ] {
        assert!(
            !completion.contains(marker),
            "the completion path no longer scrapes prose, but {marker} remains in src/completion.rs"
        );
    }

    // One generic reader, owned by the declaration module and reached by the
    // QA adapter - never a second hand-rolled scanner.
    let declaration = repository("../../src/declaration.rs");
    assert!(
        declaration.contains("read_selected_string_lists"),
        "the shared generic reader lives in src/declaration.rs"
    );
    let adapter = repository("src/ticket_tags.rs");
    assert!(
        adapter.contains("read_selected_string_lists"),
        "the QA adapter reaches the one shared reader instead of a second parser"
    );

    // Production names the three receipt kinds and no workflow field.
    for relative in [
        "../../src/completion.rs",
        "../../src/declaration.rs",
        "../../src/machine.rs",
        "../../src/scheduler.rs",
        "../../src/lib.rs",
    ] {
        let source = repository(relative);
        for literal in ["focused-tests", "hidden-lanes", "quality-commands"] {
            assert!(
                !source.contains(literal),
                "production hard-codes no workflow field, but {literal} appears in {relative}"
            );
        }
    }
}

/// The declaration carriers of the refusal group: one field set, one defect
/// per case, all under the mapping the runbook selected.
const TICKET_905: &str = ".arca/ticket/t-905.md";

const CARRIER_SCALAR: &str = r#"---
ticket-id: t-905
planned-works: oops
secret-lanes:
  - "L-905-01"
commands-of-record:
  - "rtm probe alpha"
---
Body.
"#;

const CARRIER_EMPTY_ENTRY: &str = r#"---
ticket-id: t-905
planned-works:
  - "W-905-01"
secret-lanes:
  - ""
commands-of-record:
  - "rtm probe alpha"
---
Body.
"#;

const CARRIER_UNQUOTED: &str = r#"---
ticket-id: t-905
planned-works:
  - "W-905-01"
secret-lanes:
  - bare-word
commands-of-record:
  - "rtm probe alpha"
---
Body.
"#;

const CARRIER_DUP_WITHIN: &str = r#"---
ticket-id: t-905
planned-works:
  - "W-905-01"
  - "W-905-01"
secret-lanes:
  - "L-905-01"
commands-of-record:
  - "rtm probe alpha"
---
Body.
"#;

const CARRIER_DUP_ACROSS: &str = r#"---
ticket-id: t-905
planned-works:
  - "W-905-01"
secret-lanes:
  - "W-905-01"
commands-of-record:
  - "rtm probe alpha"
---
Body.
"#;

const CARRIER_MISSING_PEER: &str = r#"---
ticket-id: t-905
planned-works:
  - "W-905-01"
commands-of-record:
  - "rtm probe alpha"
---
Body.
"#;

const CARRIER_TRUNCATED: &str = "---\nticket-id: t-905\nplanned-works:\n  - \"W-905-01\"\nsecret-lanes:\n  - \"L-905-01\"\ncommands-of-record:\n  - \"rtm probe alpha\"\n";

const CARRIER_ALL_EMPTY: &str = r#"---
ticket-id: t-905
planned-works: []
secret-lanes: []
commands-of-record: []
---
Body.
"#;

const CARRIER_UNSELECTED: &str = r#"---
ticket-id: t-905
notes: "nothing selected here"
---
Body.
"#;

/// CGDV-005: malformed, partial, empty, and duplicate declarations refuse
/// naming the selected field and entry before receipt indexing and without
/// one mutated byte; one empty kind beside real work is legal; the
/// machine-shape group reports RB105, RB110, or RB113.
#[test]
fn declared_list_errors_refuse_before_any_mutation() {
    // One valid configured execution first: real work beside an explicitly
    // empty kind - the `[]` spelling the documented format adds.
    let fixture = Fixture::create("shapes", &[".ratmac", "src", ".arca/ticket"]);
    fixture.write_runbook(&gated_runbook(&mapped_guard(
        TICKET_905,
        "planned-works",
        "secret-lanes",
        "commands-of-record",
    )));
    fixture.write_file(
        TICKET_905,
        "---\nticket-id: t-905\nplanned-works:\n  - \"W-905-01\"\nsecret-lanes: \
         []\ncommands-of-record:\n  - \"rtm probe alpha\"\n---\n\nPlain body prose.\n",
    );
    let doctor = fixture.rtm(&["doctor"]);
    assert!(
        doctor.status.success(),
        "the mapped runbook is a valid Machine Class; got:\n{}",
        combined(&doctor)
    );
    let green = fixture.start();
    fixture.record_all(
        &green,
        "t-905",
        &[("W-905-01", "focused"), ("rtm probe alpha", "quality")],
    );
    fixture.pass_step(&green);
    fixture.in_state(&green, "done");

    // A second Run in the same root: complete green evidence for the first
    // Run satisfies nothing here, and every declaration defect refuses
    // before receipt indexing while mutating nothing.
    let run = fixture.start();
    let cases: &[(&str, &str, &str, &str, &str)] = &[
        (
            "a scalar where a list belongs",
            CARRIER_SCALAR,
            "planned-works",
            "entry \"oops\"",
            "scalar",
        ),
        (
            "an unindented entry in a selected block",
            "---\nplanned-works:\n- \"W-905-01\"\nsecret-lanes: []\ncommands-of-record:\n  - \"rtm probe alpha\"\n---\n",
            "planned-works",
            "entry \"W-905-01\"",
            "an entry with no open list",
        ),
        (
            "an unindented entry after a completed selected list",
            "---\nplanned-works: []\n- \"W-905-01\"\nsecret-lanes: []\ncommands-of-record:\n  - \"rtm probe alpha\"\n---\n",
            "planned-works",
            "entry \"W-905-01\"",
            "an entry with no open list",
        ),
        (
            "an empty entry",
            CARRIER_EMPTY_ENTRY,
            "secret-lanes",
            "entry \"\"",
            "empty entry",
        ),
        (
            "an entry that is not a quoted string",
            CARRIER_UNQUOTED,
            "secret-lanes",
            "entry \"bare-word\"",
            "quoted string",
        ),
        (
            "a duplicate within one list",
            CARRIER_DUP_WITHIN,
            "planned-works",
            "entry \"W-905-01\"",
            "duplicate",
        ),
        (
            "a duplicate across two lists",
            CARRIER_DUP_ACROSS,
            "secret-lanes",
            "entry \"W-905-01\"",
            "duplicate",
        ),
        (
            "a missing selected peer",
            CARRIER_MISSING_PEER,
            "secret-lanes",
            "",
            "missing",
        ),
        (
            "truncated front matter",
            CARRIER_TRUNCATED,
            "",
            "",
            "truncated",
        ),
        (
            "an entirely empty combined set",
            CARRIER_ALL_EMPTY,
            "",
            "",
            "declares no checks",
        ),
        (
            "no selected field at all",
            CARRIER_UNSELECTED,
            "",
            "",
            "declares no checks",
        ),
    ];
    for (label, carrier, field, entry, why) in cases {
        fixture.write_file(TICKET_905, carrier);
        let before = snapshot(&fixture.root);
        let refusal = fixture.refuse_step(&run);
        if !field.is_empty() {
            assert!(
                refusal.contains(*field),
                "{label}: the refusal names the selected field; got:\n{refusal}"
            );
        }
        if !entry.is_empty() {
            assert!(
                refusal.contains(*entry),
                "{label}: the refusal names the offending entry; got:\n{refusal}"
            );
        }
        assert!(
            refusal.contains(*why),
            "{label}: the refusal says why in its stable words ({why}); got:\n{refusal}"
        );
        assert_eq!(
            snapshot(&fixture.root),
            before,
            "{label}: the refusal changed Run record, log, carrier, or evidence bytes"
        );
    }
    fixture.in_state(&run, "work");

    // The refusal precedes receipt indexing: beside a malformed declaration
    // a stray receipt stays unnamed.
    fixture.write_receipt(&run, "t-905", "STRAY-905-9", "focused", 0);
    fixture.write_file(TICKET_905, CARRIER_SCALAR);
    let before = snapshot(&fixture.root);
    let refusal = fixture.refuse_step(&run);
    assert!(
        refusal.contains("planned-works") && refusal.contains("entry \"oops\""),
        "the declaration defect is named before any receipt is read; got:\n{refusal}"
    );
    assert!(
        !refusal.contains("STRAY-905-9"),
        "a declaration refusal precedes receipt indexing, so the stray check stays \
         unnamed; got:\n{refusal}"
    );
    assert_eq!(snapshot(&fixture.root), before, "the refusal wrote nothing");

    // The machine-shape group: partial, mistyped, and bad mapping values
    // refuse under their own codes, and the doctor writes nothing.
    let group: &[(&str, String, &str, &[&str])] = &[
        (
            "a partial group",
            gated_runbook(&raw_guard(
                TICKET_905,
                "declaration-format = \"front-matter-string-lists\"",
            )),
            "RB105",
            &["focused-field"],
        ),
        (
            "a wrong type",
            gated_runbook(&raw_guard(
                TICKET_905,
                "declaration-format = \"front-matter-string-lists\", focused-field = 3, \
                 hidden-lane-field = \"secret-lanes\", quality-field = \"commands-of-record\"",
            )),
            "RB110",
            &["focused-field"],
        ),
        (
            "an unsupported format",
            gated_runbook(&raw_guard(
                TICKET_905,
                "declaration-format = \"prose-scrape\", focused-field = \"planned-works\", \
                 hidden-lane-field = \"secret-lanes\", quality-field = \"commands-of-record\"",
            )),
            "RB113",
            &["prose-scrape"],
        ),
        (
            "a blank mapped name",
            gated_runbook(&raw_guard(
                TICKET_905,
                "declaration-format = \"front-matter-string-lists\", focused-field = \"\", \
                 hidden-lane-field = \"secret-lanes\", quality-field = \"commands-of-record\"",
            )),
            "RB113",
            &["focused-field"],
        ),
        (
            "a name reused for two kinds",
            gated_runbook(&raw_guard(
                TICKET_905,
                "declaration-format = \"front-matter-string-lists\", focused-field = \"twin\", \
                 hidden-lane-field = \"twin\", quality-field = \"commands-of-record\"",
            )),
            "RB113",
            &["twin"],
        ),
    ];
    for (label, runbook, code, needles) in group {
        let shape = Fixture::create("shape", &[".ratmac", "src", ".arca/ticket"]);
        shape.write_runbook(runbook);
        shape.write_file(TICKET_905, CARRIER_UNSELECTED);
        let before = snapshot(&shape.root);
        let doctor = shape.rtm(&["doctor"]);
        let report = combined(&doctor);
        assert!(
            !doctor.status.success(),
            "{label} is not a runbook; got:\n{report}"
        );
        assert!(
            report.contains(*code),
            "{label} refuses under its own code {code}; got:\n{report}"
        );
        for needle in *needles {
            assert!(
                report.contains(*needle),
                "{label} names {needle}; got:\n{report}"
            );
        }
        if *label == "a partial group" {
            assert!(
                !report.contains("hidden-lane-field"),
                "group completeness names the first missing field only; got:\n{report}"
            );
        }
        assert_eq!(
            snapshot(&shape.root),
            before,
            "{label}: the doctor writes nothing"
        );
    }
}

/// CGDV-006: the receipt contract the aged t047/t048/t049 fixtures proved -
/// kinds and bytes, Run-keyed paths, target binding, green, self-consistency,
/// freshness, declared-only receipts, missing/stray/duplicate safeguards,
/// paused-Run precedence, and sensitivity keyed to the planned tests - is
/// unchanged by the mapping.
#[test]
fn historical_receipt_contract_is_unchanged() {
    let fixture = Fixture::create("aged", &[".ratmac", "src", ".arca/ticket", "lanes"]);
    fixture.write_file("lanes/fake.rs", "fn cgdv606_probe() {}\n");
    fixture.write_runbook(&format!(
        "[states.probe]\nprompt = \"Prove the planned tests red.\"\n\
         guards = [{{ kind = \"sensitivity_receipts\", ticket = \
         \".arca/ticket/t-606.md\" }}]\n\n\
         [states.work]\nprompt = \"Work the item.\"\nguards = [{}]\n\n\
         [states.done]\nprompt = \"Finish.\"\n\n\
         [[transitions]]\nfrom = \"probe\"\nto = \"work\"\n\n\
         [[transitions]]\nfrom = \"work\"\nto = \"done\"\n",
        mapped_guard(".arca/ticket/t-606.md", "works", "lanes", "gates")
    ));
    fixture.write_file(".arca/ticket/t-606.md", AGED_CARRIER);
    const CHECKS: &[(&str, &str)] = &[
        ("W-606-01", "focused"),
        ("L-606-01", "hidden-lane"),
        ("cargo --version", "quality"),
    ];

    // One valid configured execution first: sensitivity proven red, then the
    // whole mapped set green and fresh, straight through to done.
    let run = fixture.start();
    fixture.write_sensitivity(&run, "t-606", "PT-606-01");
    fixture.pass_step(&run);
    fixture.in_state(&run, "work");
    fixture.record_all(&run, "t-606", CHECKS);
    fixture.pass_step(&run);
    fixture.in_state(&run, "done");

    // A second Run carries the defect matrix. Sensitivity stays keyed to the
    // planned tests: with no sensitivity receipt the refusal names the
    // planned test, never a mapped id.
    let dry = fixture.start();
    let sensitivity = fixture.refuse_step(&dry);
    assert!(
        sensitivity.contains("PT-606-01") && sensitivity.contains("sensitivity"),
        "sensitivity still reads the planned tests; got:\n{sensitivity}"
    );
    fixture.write_sensitivity(&dry, "t-606", "PT-606-01");
    fixture.pass_step(&dry);
    fixture.in_state(&dry, "work");

    // Missing receipts: the existing wording, the receipt kinds, the
    // Run-keyed expected path, and the focused-hidden-quality order.
    let missing = fixture.refuse_step(&dry);
    let order = order_of(&missing, &["W-606-01", "L-606-01", "cargo --version"]);
    assert_increasing(&order, &missing);
    for phrase in ["focused check", "hidden-lane check", "quality check"] {
        assert!(
            missing.contains(phrase),
            "the refusal names the receipt kind ({phrase}); got:\n{missing}"
        );
    }
    assert!(
        missing.contains(&format!(
            ".ratmac/evidence/{dry}/completion/{}.toml",
            check_slug("W-606-01")
        )),
        "the refusal expects the Run-keyed receipt path; got:\n{missing}"
    );
    assert!(
        !missing.contains("PT-606-01"),
        "a planned test is sensitivity work, not completion work; got:\n{missing}"
    );

    // A stray receipt is named and refuses.
    fixture.record_all(&dry, "t-606", CHECKS);
    fixture.write_receipt(&dry, "t-606", "STRAY-606-9", "focused", 0);
    let before = snapshot(&fixture.root);
    let stray = fixture.refuse_step(&dry);
    assert!(
        stray.contains("STRAY-606-9")
            && stray.contains("claims a check the ticket does not declare"),
        "a receipt for undeclared work refuses in the existing words; got:\n{stray}"
    );
    assert_eq!(snapshot(&fixture.root), before, "the refusal wrote nothing");

    // Two receipts claiming one check refuse in the existing words.
    fixture.record_all(&dry, "t-606", CHECKS);
    let twin = fixture
        .receipt_path(&dry, "W-606-01")
        .with_file_name("w-606-01-again.toml");
    fs::copy(fixture.receipt_path(&dry, "W-606-01"), &twin).expect("duplicate the receipt");
    let duplicate = fixture.refuse_step(&dry);
    assert!(
        duplicate.contains("two receipts claim the same check"),
        "a duplicated receipt refuses in the existing words; got:\n{duplicate}"
    );

    // Target binding: another ticket's receipt satisfies nothing.
    fixture.record_all(&dry, "t-606", CHECKS);
    fixture.write_receipt(&dry, "t-909", "W-606-01", "focused", 0);
    let bound = fixture.refuse_step(&dry);
    assert!(
        bound.contains("W-606-01") && bound.contains("no completion receipt"),
        "a receipt bound to another ticket does not satisfy the check; got:\n{bound}"
    );

    // The receipt kind must match the declared kind.
    fixture.record_all(&dry, "t-606", CHECKS);
    fixture.write_receipt(&dry, "t-606", "L-606-01", "focused", 0);
    let relabeled = fixture.refuse_step(&dry);
    assert!(
        relabeled.contains("records a focused check, but the ticket declares it as hidden-lane"),
        "a relabeled receipt refuses in the existing words; got:\n{relabeled}"
    );

    // A recorded red run is not completion.
    fixture.record_all(&dry, "t-606", CHECKS);
    fixture.write_receipt(&dry, "t-606", "W-606-01", "focused", 101);
    let red = fixture.refuse_step(&dry);
    assert!(
        red.contains("recorded run exited 101"),
        "a red receipt refuses naming the exit status; got:\n{red}"
    );

    // Freshness: a receipt that no longer describes the tree is stale.
    fixture.record_all(&dry, "t-606", CHECKS);
    let recorded =
        tree_digest(&fixture.root, &["src".to_owned()]).expect("source roots are readable");
    fixture.write_file("src/lib.rs", "pub fn fixture() { todo!() }\n");
    let current =
        tree_digest(&fixture.root, &["src".to_owned()]).expect("source roots are readable");
    assert_ne!(recorded, current, "editing the source changes the digest");
    let stale = fixture.refuse_step(&dry);
    assert!(
        stale.contains("is stale") && stale.contains(&recorded) && stale.contains(&current),
        "the refusal shows the recorded and current tree digests; got:\n{stale}"
    );
    fixture.write_file("src/lib.rs", "pub fn fixture() {}\n");

    // Self-consistency: a digest that does not re-derive refuses.
    fixture.record_all(&dry, "t-606", CHECKS);
    let path = fixture.receipt_path(&dry, "W-606-01");
    let source = fs::read_to_string(&path).expect("read the receipt");
    fs::write(&path, source.replace(&sha256_text(GREEN), &"0".repeat(64)))
        .expect("corrupt the digest");
    let derived = fixture.refuse_step(&dry);
    assert!(
        derived.contains("does not re-derive"),
        "the refusal says the digest does not re-derive; got:\n{derived}"
    );

    // The green end state: the same fixture, whole and fresh, completes.
    fixture.record_all(&dry, "t-606", CHECKS);
    fixture.pass_step(&dry);
    fixture.in_state(&dry, "done");

    // Paused-Run precedence: a held Run cannot pass a completion gate even
    // with every receipt green.
    let held = Fixture::create(
        "aged-held",
        &[".ratmac", "src", ".arca/ticket", ".arca/issue"],
    );
    held.write_file(".arca/issue/note.md", "# an out-of-scope blocker\n");
    let guard = mapped_guard(".arca/ticket/t-606.md", "works", "lanes", "gates");
    held.write_runbook(&format!(
        "[roots]\nissue = \".arca/issue\"\n\n\
         [states.prep]\nprompt = \"Prepare.\"\nguards = [{guard}]\n\n\
         [states.work]\nprompt = \"Work.\"\nguards = [{guard}]\n\n\
         [states.done]\nprompt = \"Finish.\"\n\n\
         [[transitions]]\nfrom = \"prep\"\nto = \"work\"\n\n\
         [[transitions]]\nfrom = \"work\"\nto = \"done\"\n\n\
         [[transitions]]\nfrom = \"work\"\nto = \"prep\"\nblocked-route = true\n"
    ));
    held.write_file(".arca/ticket/t-606.md", AGED_CARRIER);
    let paused_run = held.start();
    held.record_all(&paused_run, "t-606", CHECKS);
    held.pass_step(&paused_run);
    held.in_state(&paused_run, "work");
    let confirmation = format!("hold {paused_run}");
    let hold = held.rtm(&[
        "hold",
        "--run",
        &paused_run,
        "--blocker",
        ".arca/issue/note.md",
        "--confirm",
        &confirmation,
    ]);
    let held_text = combined(&hold);
    assert!(
        hold.status.success() && held_text.contains("paused against"),
        "the human-confirmed hold pauses the Run; got:\n{held_text}"
    );
    let paused = held.refuse_step(&paused_run);
    assert!(
        paused.contains("paused")
            && paused.contains(&paused_run)
            && paused.contains(".arca/issue/note.md"),
        "a paused Run cannot pass a completion gate, and the refusal names the Run \
         and its blocker; got:\n{paused}"
    );
}

/// The spawn-driven fixture: one worker class whose completion gate is
/// addressed by a binding over an arbitrary-extension carrier.
const BOUND_RUNBOOK: &str = r#"
[roots]
stock = "stock"

[classes.worker.bindings.widget]
required = true

[classes.worker.states.assemble]
prompt = "Assemble the bound widget."
guards = [{ kind = "completion_gate", root = "stock", ticket-binding = "widget", declaration-format = "front-matter-string-lists", focused-field = "musts", hidden-lane-field = "shadows", quality-field = "commands" }]

[classes.worker.states.done]
prompt = "Done."

[[classes.worker.transitions]]
from = "assemble"
to = "done"

[states.plan]
prompt = "Plan."

[states.delegate]
prompt = "Delegate and wait."
guards = [{ kind = "join", require = "all_passed", min = 1 }]

[[states.delegate.spawns]]
class = "worker"
name = "unit"
bind = ["widget"]

[states.finished]
prompt = "Finished."

[[transitions]]
from = "plan"
to = "delegate"

[[transitions]]
from = "delegate"
to = "finished"
"#;

/// CGDV-007: a heading-free, non-Markdown carrier behind an arbitrary
/// extension gates through literal and bound addresses with no suffix rule,
/// loses nothing when one receipt is missing, and an unmapped guard refuses
/// with mapping guidance before reading an unreadable item.
#[test]
fn heading_free_non_markdown_carrier_gates() {
    // The literal address: a `.data` carrier with no heading and no
    // backtick completes on whole green receipts alone.
    let literal = Fixture::create("data-literal", &[".ratmac", "src", "items"]);
    literal.write_runbook(&gated_runbook(&mapped_guard(
        "items/gadget.data",
        "musts",
        "shadows",
        "commands",
    )));
    literal.write_file("items/gadget.data", DATA_CARRIER);
    let run = literal.start();
    let unread = literal.refuse_step(&run);
    for id in ["G-907-01", "S-907-01", "rtm probe alpha"] {
        assert!(
            unread.contains(id),
            "the gate read the .data carrier and named {id}; got:\n{unread}"
        );
    }
    literal.record_all(&run, "gadget", DATA_CHECKS);
    literal.pass_step(&run);
    literal.in_state(&run, "done");

    // The same fixture with one receipt removed keeps the existing refusal.
    let second = literal.start();
    literal.record_all(&second, "gadget", DATA_CHECKS);
    fs::remove_file(literal.receipt_path(&second, "S-907-01")).expect("remove one receipt");
    let before = snapshot(&literal.root);
    let refusal = literal.refuse_step(&second);
    assert!(
        refusal.contains("S-907-01")
            && refusal.contains(&format!(
                ".ratmac/evidence/{second}/completion/{}.toml",
                check_slug("S-907-01")
            )),
        "the missing receipt refuses by name at its Run-keyed path; got:\n{refusal}"
    );
    assert_eq!(snapshot(&literal.root), before, "the refusal wrote nothing");

    // The bound address: the same carrier reached through a spawn binding,
    // the runbook naming no item.
    let bound = Fixture::create("data-bound", &[".ratmac", "src", "stock"]);
    bound.write_runbook(BOUND_RUNBOOK);
    bound.write_file("stock/gadget.data", DATA_CARRIER);
    let parent = bound.start();
    bound.pass_step(&parent);
    let child = bound.spawn(&parent, "unit", "widget", "gadget.data");
    bound.record_all(&child, "gadget", DATA_CHECKS);
    bound.pass_step(&child);
    bound.in_state(&child, "done");
    let runbook = fs::read_to_string(bound.root.join(".ratmac/ratmac.toml"))
        .expect("read the fixture runbook");
    assert!(
        !runbook.contains("gadget"),
        "the bound runbook names no item; the address lives only in the spawn ledger"
    );

    // An unmapped guard refuses before reading an unreadable item: the
    // refusal names the missing mapping fields, never the absent file, and
    // writes nothing. Omitted mapping stays statically valid, so the doctor
    // keeps its clean exit.
    let unmapped = Fixture::create("data-unmapped", &[".ratmac", "src", "items"]);
    unmapped.write_runbook(&gated_runbook(
        "{ kind = \"completion_gate\", ticket = \"items/vanished.data\" }",
    ));
    let doctor = unmapped.rtm(&["doctor"]);
    assert!(
        doctor.status.success(),
        "an omitted mapping is not a doctor lint; got:\n{}",
        combined(&doctor)
    );
    let orphan = unmapped.start();
    let before = snapshot(&unmapped.root);
    let refusal = unmapped.refuse_step(&orphan);
    for field in [
        "declaration-format",
        "focused-field",
        "hidden-lane-field",
        "quality-field",
    ] {
        assert!(
            refusal.contains(field),
            "the unmapped refusal names the missing mapping field {field}; got:\n{refusal}"
        );
    }
    assert!(
        !refusal.contains("unreadable"),
        "the refusal precedes any read of the addressed item; got:\n{refusal}"
    );
    assert_eq!(
        snapshot(&unmapped.root),
        before,
        "the unmapped refusal created no file and changed no byte"
    );
    assert!(
        !unmapped.root.join("items/vanished.data").exists(),
        "the unreadable item was never created"
    );
}
