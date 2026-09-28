//! t-122 / WRS-001: successor ownership.
//!
//! A spawned replacement becomes usable only after its owning parent ledger
//! durably records it. The checks pause a ledger-recorded respawn at the
//! publication boundaries (`RATMAC_TEST_RESPAWN_PAUSE`), inject the fault
//! boundaries (`RATMAC_TEST_RESPAWN_FAULT`), kill paused invocations for the
//! recovery table, race two respawns of one predecessor, and keep the
//! unchanged completed-state behavior green. Every wait is bounded and every
//! spawned child is killed and reaped on every exit path.

use ratmac_qa::support;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

/// A parent machine that spawns `rev` children of class `reviewer` bound to a
/// required `ticket`, with a join-guarded out-edge. The reviewer class is
/// steppable (`review` -> `finished`), so a successor is a live, ordinary Run,
/// and it declares a blocked route, so `rtm hold` reaches its Run lock.
const RUNBOOK: &str = r#"
[roots]
issue = ".arca/issue"

[classes.reviewer.bindings.ticket]
required = true

[classes.reviewer.states.review]
prompt = "Review the delegated ticket."

[classes.reviewer.states.finished]
prompt = "Review finished."

[classes.reviewer.states.parked]
prompt = "Parked against a blocker."

[[classes.reviewer.transitions]]
from = "review"
to = "finished"

[[classes.reviewer.transitions]]
from = "review"
to = "parked"
blocked-route = true

[[classes.reviewer.transitions]]
from = "finished"
to = "parked"

[states.plan]
prompt = "Plan."

[states.delegate]
prompt = "Delegate and wait."
guards = [{ kind = "join", require = "all_passed", min = 1 }]

[[states.delegate.spawns]]
class = "reviewer"
name = "rev"
bind = ["ticket"]

[states.done]
prompt = "Done."

[[transitions]]
from = "plan"
to = "delegate"

[[transitions]]
from = "delegate"
to = "done"
"#;

/// The pause seam's companions, as the contract names them.
const PAUSE_VAR: &str = "RATMAC_TEST_RESPAWN_PAUSE";
const PAUSE_MARKER_VAR: &str = "RATMAC_TEST_RESPAWN_PAUSE_MARKER";
const PAUSE_RELEASE_VAR: &str = "RATMAC_TEST_RESPAWN_PAUSE_RELEASE";
const PAUSE_TIMEOUT_VAR: &str = "RATMAC_TEST_RESPAWN_PAUSE_TIMEOUT_MILLIS";
/// The fault seam.
const FAULT_VAR: &str = "RATMAC_TEST_RESPAWN_FAULT";
/// Where a feature build records one operation kind per line.
const OP_LOG_VAR: &str = "RATMAC_TEST_OPERATION_LOG";

/// A pause long enough that every 5 s lock wait in a boundary inspection
/// expires while the paused invocation still holds its locks.
const PAUSE_MILLIS: &str = "120000";

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn create(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t122-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".arca/goal")).expect("create fixture goal tree");
        fs::create_dir_all(root.join(".ratmac")).expect("create fixture Engine tree");
        fs::create_dir_all(root.join("src")).expect("create fixture source tree");
        fs::create_dir_all(root.join(".arca/issue/i-777-blocker"))
            .expect("create fixture blocker tree");
        fs::write(root.join(".arca/goal/spec.md"), "# Fixture goal\n").expect("write fixture goal");
        fs::write(root.join(".ratmac/ratmac.toml"), RUNBOOK).expect("write fixture machine class");
        fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").expect("write fixture source");
        fs::write(
            root.join(".arca/issue/i-777-blocker/spec.md"),
            "# i-777\n\n```yaml\nstatus: \"pending\"\n```\n",
        )
        .expect("write fixture blocker");
        Self { root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        self.rtm_env(args, &[])
    }

    fn rtm_env(&self, args: &[&str], envs: &[(&str, &str)]) -> Output {
        let mut command = support::command(ratmac_qa::engine_bin!(), &self.root);
        command.args(args);
        for (key, value) in envs {
            command.env(key, value);
        }
        command.output().expect("invoke rtm")
    }

    /// A respawn of `predecessor`, paused at `boundary`, as a guarded child.
    fn spawn_paused_respawn(
        &self,
        boundary: &str,
        marker: &Path,
        release: &Path,
        predecessor: &str,
    ) -> ChildGuard {
        let confirm = format!("respawn {predecessor}");
        let mut command = support::command(ratmac_qa::engine_bin!(), &self.root);
        command
            .env(PAUSE_VAR, boundary)
            .env(PAUSE_MARKER_VAR, marker)
            .env(PAUSE_RELEASE_VAR, release)
            .env(PAUSE_TIMEOUT_VAR, PAUSE_MILLIS)
            .args(["respawn", "--run", predecessor, "--confirm", &confirm]);
        ChildGuard::spawn(&mut command)
    }

    fn start(&self) -> String {
        let output = self.rtm(&["start"]);
        let text = combined(&output);
        assert!(output.status.success(), "start succeeds: {text}");
        id_after(&text, "started run ")
    }

    fn step(&self, id: &str) -> Output {
        self.rtm(&["step", "--run", id])
    }

    fn spawn_rev(&self, parent: &str, ticket: &str) -> String {
        let bind = format!("ticket={ticket}");
        let output = self.rtm(&["spawn", "rev", "--run", parent, "--bind", &bind]);
        let text = combined(&output);
        assert!(output.status.success(), "spawn succeeds: {text}");
        id_after(&text, "spawned run ")
    }

    fn respawn(&self, predecessor: &str) -> Output {
        let confirm = format!("respawn {predecessor}");
        self.rtm(&["respawn", "--run", predecessor, "--confirm", &confirm])
    }

    fn runs_dir(&self) -> PathBuf {
        self.root.join(".ratmac/runs")
    }

    fn run_dir(&self, id: &str) -> PathBuf {
        self.runs_dir().join(id)
    }

    fn record_path(&self, id: &str) -> PathBuf {
        self.run_dir(id).join("run.toml")
    }

    fn ledger_path(&self, parent: &str) -> PathBuf {
        self.run_dir(parent).join("spawn-ledger")
    }

    fn ledger_text(&self, parent: &str) -> String {
        fs::read_to_string(self.ledger_path(parent)).expect("the parent spawn-ledger is readable")
    }

    fn mint_text(&self) -> String {
        fs::read_to_string(self.root.join(".ratmac/mint.toml")).expect("mint.toml is readable")
    }

    /// The `.ratmac/runs/` directory members, sorted.
    fn roster(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(self.runs_dir()) else {
            return Vec::new();
        };
        let mut ids: Vec<String> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        ids.sort();
        ids
    }

    /// Every file under `.ratmac/`, keyed by relative path, with exact bytes.
    fn engine_snapshot(&self) -> BTreeMap<String, Vec<u8>> {
        let mut files = BTreeMap::new();
        fn walk(base: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
            let Ok(entries) = fs::read_dir(dir) else {
                return;
            };
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    walk(base, &path, files);
                } else {
                    let relative = path
                        .strip_prefix(base)
                        .expect("snapshot path is below the Engine root")
                        .to_string_lossy()
                        .replace('\\', "/");
                    files.insert(
                        relative,
                        fs::read(&path).expect("snapshot entry is readable"),
                    );
                }
            }
        }
        walk(
            &self.root.join(".ratmac"),
            &self.root.join(".ratmac"),
            &mut files,
        );
        files
    }

    /// The Engine root in its one displayed spelling (forward slashes).
    fn engine_root(&self) -> String {
        self.root
            .join(".ratmac")
            .to_string_lossy()
            .replace('\\', "/")
    }

    /// The displayed parent spawn-ledger path, as refusals spell it.
    fn ledger_display(&self, parent: &str) -> String {
        format!("{}/runs/{parent}/spawn-ledger", self.engine_root())
    }

    /// The bare `rtm status` roster surface: first line, rows, and all.
    fn status_surface(&self) -> String {
        combined(&self.rtm(&["status"]))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn id_after(text: &str, marker: &str) -> String {
    text.split(marker)
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or_else(|| panic!("output names the minted id after {marker:?}: {text}"))
        .to_owned()
}

/// A spawned child that is killed and reaped on every exit path, so no lock
/// outlives the test.
struct ChildGuard {
    child: std::process::Child,
}

impl ChildGuard {
    fn spawn(command: &mut Command) -> Self {
        command.stdin(std::process::Stdio::null());
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        Self {
            child: command.spawn().expect("spawn rtm child"),
        }
    }

    /// Kill and reap the child now.
    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// Collect the child's exit, failing with `what` after `seconds`.
    fn wait_bounded(&mut self, seconds: u64, what: &str) -> Output {
        let deadline = Instant::now() + Duration::from_secs(seconds);
        loop {
            if let Ok(Some(status)) = self.child.try_wait() {
                let stdout = read_pipe(self.child.stdout.as_mut());
                let stderr = read_pipe(self.child.stderr.as_mut());
                return Output {
                    status,
                    stdout,
                    stderr,
                };
            }
            assert!(
                Instant::now() < deadline,
                "WRSV-001: {what} within {seconds}s"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.kill();
    }
}

fn read_pipe(pipe: Option<&mut impl std::io::Read>) -> Vec<u8> {
    let mut bytes = Vec::new();
    if let Some(pipe) = pipe {
        std::io::Read::read_to_end(pipe, &mut bytes).expect("read child output");
    }
    bytes
}

/// Wait until `path` is a file, bounded, failing with `what`.
fn wait_for_file(path: &Path, seconds: u64, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while !path.is_file() {
        assert!(
            Instant::now() < deadline,
            "WRSV-001: {what} within {seconds}s: {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// An owned marker/release pair for one paused invocation.
fn marker_pair(label: &str) -> (PathBuf, PathBuf) {
    let base = std::env::temp_dir().join(format!(
        "ratmac-t122-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    (
        base.with_extension("marker"),
        base.with_extension("release"),
    )
}

/// A live parent in `delegate` with one ledger-recorded child: the parent's
/// spawn-ledger holds the child's entry, and the child is the predecessor
/// every respawn below supersedes.
fn recorded_child(fixture: &Fixture, ticket: &str) -> (String, String) {
    let parent = fixture.start();
    let step = fixture.step(&parent);
    assert!(
        step.status.success(),
        "the parent steps into the spawning State: {}",
        combined(&step)
    );
    let child = fixture.spawn_rev(&parent, ticket);
    (parent, child)
}

/// Parse a ledger as TOML and return its `[[children]]` entries.
fn entries(text: &str) -> Vec<toml::value::Table> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    let value: toml::Value = text.parse().expect("the spawn ledger is valid TOML");
    let table = value.as_table().expect("ledger top level is a table");
    let Some(children) = table.get("children") else {
        return Vec::new();
    };
    children
        .as_array()
        .expect("children is an array of tables")
        .iter()
        .map(|entry| {
            entry
                .as_table()
                .expect("each ledger entry is a table")
                .clone()
        })
        .collect()
}

fn field<'a>(entry: &'a toml::value::Table, key: &str) -> &'a str {
    entry
        .get(key)
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("entry field {key} is a string: {entry:?}"))
}

fn abandoned(entry: &toml::value::Table) -> bool {
    entry
        .get("abandoned")
        .and_then(toml::Value::as_bool)
        .expect("entry field abandoned is a bool")
}

fn binding<'a>(entry: &'a toml::value::Table, name: &str) -> &'a str {
    entry
        .get("bind")
        .and_then(toml::Value::as_table)
        .unwrap_or_else(|| panic!("entry carries a bind table: {entry:?}"))
        .get(name)
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("bind value {name} is a string: {entry:?}"))
}

/// The recorded successor of `predecessor` in its parent's ledger, if any.
///
/// Only a successor entry carries `supersedes`; an ordinary spawned entry
/// has no such key, so the link is read optionally, never as a required
/// field.
fn supersedes_id(entry: &toml::value::Table) -> Option<&str> {
    entry.get("supersedes").and_then(toml::Value::as_str)
}

fn recorded_successor(fixture: &Fixture, parent: &str, predecessor: &str) -> Option<String> {
    entries(&fixture.ledger_text(parent))
        .into_iter()
        .find(|entry| supersedes_id(entry) == Some(predecessor))
        .map(|entry| field(&entry, "id").to_owned())
}

/// One roster row, as the descriptive roster renders it.
fn row(id: &str, fields: &[&str]) -> String {
    format!("  {id}: {}", fields.join("; "))
}

fn child_role(parent: &str, ticket: &str) -> String {
    format!("child of {parent}; class reviewer; bind ticket=\"{ticket}\"")
}

/// Assert every addressing refusal for `successor` names it, quotes the
/// roster without it, and writes nothing.
fn successor_is_unaddressable(fixture: &Fixture, parent: &str, predecessor: &str, successor: &str) {
    let before = fixture.engine_snapshot();
    let summary = format!("runs: {parent}, {predecessor}");

    let step = fixture.rtm(&["step", "--run", successor]);
    let step_text = stderr(&step);
    let step_lines: Vec<&str> = step_text.lines().collect();
    assert_eq!(
        step_lines.first().copied(),
        Some(
            format!("rtm: step: run id \"{successor}\" is not an exact roster member; {summary}")
                .as_str(),
        ),
        "WRSV-001-01: step refuses the unadmitted successor by address before any lock"
    );

    let hold = fixture.rtm(&[
        "hold",
        "--run",
        successor,
        "--blocker",
        ".arca/issue/i-777-blocker",
        "--confirm",
        &format!("hold {successor}"),
    ]);
    assert!(
        !hold.status.success()
            && stderr(&hold).starts_with(&format!(
                "rtm: hold refused; run id \"{successor}\" is not an exact roster member; {summary}"
            )),
        "WRSV-001-01: hold refuses the unadmitted successor by address: {}",
        combined(&hold)
    );

    let spawn = fixture.rtm(&["spawn", "rev", "--run", successor, "--bind", "ticket=t-9"]);
    assert!(
        !spawn.status.success()
            && stderr(&spawn).starts_with(&format!(
                "rtm: spawn: run id \"{successor}\" is not an exact roster member; {summary}"
            )),
        "WRSV-001-01: spawn refuses the unadmitted successor by address: {}",
        combined(&spawn)
    );

    let respawn = fixture.rtm(&[
        "respawn",
        "--run",
        successor,
        "--confirm",
        &format!("respawn {successor}"),
    ]);
    assert!(
        !respawn.status.success()
            && stderr(&respawn).starts_with(&format!(
                "rtm: respawn refused; respawn names no run: \"{successor}\" is not on the roster; {summary}"
            )),
        "WRSV-001-01: respawn refuses the unadmitted successor by address: {}",
        combined(&respawn)
    );

    let status = fixture.rtm(&["status", "--run", successor]);
    assert!(
        !status.status.success()
            && stderr(&status).starts_with(&format!(
                "rtm: status: run id \"{successor}\" is not an exact roster member; {summary}"
            )),
        "WRSV-001-01: status refuses the unadmitted successor by address: {}",
        combined(&status)
    );

    assert_eq!(
        fixture.engine_snapshot(),
        before,
        "WRSV-001-01: the addressing refusals write nothing"
    );
}

/// Assert the lock wait expired naming `lock_path`, with today's owner-token
/// diagnostic, and nothing was written.
fn assert_wait_expired(output: &Output, first_line_prefix: &str, lock_path: &str) {
    assert!(
        !output.status.success(),
        "WRSV-001-01: a held lock makes the invocation refuse: {}",
        combined(output)
    );
    assert_eq!(
        stdout(output),
        String::new(),
        "WRSV-001-01: a wait-expired refusal writes no stdout"
    );
    assert!(
        stderr(output).starts_with(&format!(
            "{first_line_prefix}lock wait expired: {lock_path}; owner token: ratmac-lock-v1; pid="
        )),
        "WRSV-001-01: the refusal is today's wait-expired text naming the held lock: {}",
        stderr(output)
    );
}

/// WRSV-001-01: pause a ledger-recorded respawn at every publication boundary
/// and race the roster readers, successor addressing, the join guard, step and
/// hold on the predecessor, the root-domain writers, and a second respawn;
/// the successor is never usable or rendered top-level before its entry
/// exists, and the released invocation completes the recorded operation.
#[test]
fn wrsv_001_01() {
    // Positive control: an unpause respawn of a ledger-recorded child
    // completes with today's success line and completed bytes.
    let control = Fixture::create("01-control");
    let (_control_parent, control_child) = recorded_child(&control, "t-102");
    let control_respawn = control.respawn(&control_child);
    assert!(
        control_respawn.status.success(),
        "WRSV-001-01 control: the respawn succeeds: {}",
        combined(&control_respawn)
    );
    let control_successor = id_after(&stdout(&control_respawn), "successor run ");
    assert_eq!(
        stdout(&control_respawn),
        format!(
            "rtm: run superseded; successor run {control_successor} minted at \
             .ratmac/runs/{control_successor}/\n"
        ),
        "WRSV-001-01 control: the success line is one LF-terminated line"
    );
    assert_eq!(stderr(&control_respawn), String::new());
    drop(control);

    // B1: the entry is durable and the predecessor still live.
    let fixture = Fixture::create("01-b1");
    let (parent, predecessor) = recorded_child(&fixture, "t-102");
    let (marker, release) = marker_pair("01-b1");
    let ledger_before = fixture.ledger_text(&parent);
    let mut paused = fixture.spawn_paused_respawn("after-record", &marker, &release, &predecessor);
    wait_for_file(
        &marker,
        15,
        "WRSV-001-01: the after-record pause publishes its marker (the \
         RATMAC_TEST_RESPAWN_PAUSE seam must stop the respawn after the entry \
         is durable and before the predecessor is retired)",
    );
    assert_eq!(
        fs::read_to_string(&marker).expect("the pause marker is readable"),
        "after-record\n",
        "WRSV-001-01: the marker carries the boundary name"
    );

    let successor = recorded_successor(&fixture, &parent, &predecessor)
        .expect("B1: the successor entry is durable in the parent ledger");
    assert_eq!(successor, "run-003", "B1: the successor takes the next id");

    // The B1 disk state: entry durable, predecessor live, no successor
    // directory, ordinal reserved.
    assert!(
        fixture.record_path(&predecessor).is_file(),
        "B1: the predecessor keeps its admission state"
    );
    assert!(
        !fixture.run_dir(&successor).exists(),
        "B1: no successor directory exists before admission"
    );
    assert!(
        fixture.mint_text().contains("highest = 3"),
        "B1: the successor ordinal is reserved: {}",
        fixture.mint_text()
    );
    let b1_entries = entries(&fixture.ledger_text(&parent));
    assert_eq!(b1_entries.len(), 2, "B1: one successor entry is appended");
    assert_eq!(field(&b1_entries[0], "id"), predecessor);
    assert!(!abandoned(&b1_entries[0]), "B1: the predecessor is live");
    assert_eq!(field(&b1_entries[1], "id"), successor);
    assert_eq!(field(&b1_entries[1], "supersedes"), predecessor);
    assert!(
        !abandoned(&b1_entries[1]),
        "B1: the successor entry is live"
    );

    // The lock-free record readers: the summary lists no successor, the
    // predecessor keeps its ordinary live row, and the successor renders its
    // recorded role and nothing more - never top-level.
    let surface = fixture.status_surface();
    assert!(
        surface.contains(&format!("runs: {parent}, {predecessor}\n")),
        "B1: the roster summary lists no successor: {surface}"
    );
    assert!(
        surface.contains(&row(
            &predecessor,
            &[
                &child_role(&parent, "t-102"),
                "state review; status planned"
            ]
        )),
        "B1: the predecessor's row is its ordinary live row: {surface}"
    );
    let b1_successor_row = row(
        &successor,
        &[
            &child_role(&parent, "t-102"),
            &format!("supersedes {predecessor}"),
            &format!(
                "not on the roster ({}/runs/{successor} is absent)",
                fixture.engine_root()
            ),
        ],
    );
    assert!(
        surface.contains(&b1_successor_row),
        "B1: the successor renders its recorded role and absence: {surface}"
    );
    assert!(
        !surface.contains(&format!("{successor}: top-level")),
        "B1: the successor is never rendered top-level: {surface}"
    );
    let doctor = fixture.rtm(&["doctor"]);
    assert!(
        doctor.status.success(),
        "B1: doctor exits 0: {}",
        combined(&doctor)
    );
    let doctor_text = combined(&doctor);
    assert!(
        doctor_text.contains(&format!(
            "Run Record: .ratmac/runs/{predecessor}/run.toml (state: review)"
        )),
        "B1: doctor reports the live predecessor: {doctor_text}"
    );
    assert!(
        doctor_text.lines().any(|line| {
            line.contains(&successor)
                && line.contains("absent")
                && line.contains(&format!("child of {parent}; class reviewer"))
        }),
        "B1: doctor reports the recorded successor absent with its role: {doctor_text}"
    );

    // The successor is exactly as usable as an id never minted.
    successor_is_unaddressable(&fixture, &parent, &predecessor, &successor);

    // The join guard reads the ledger lock-free and refuses naming the
    // recorded-but-absent successor.
    let state_before_join = fs::read(fixture.record_path(&predecessor))
        .expect("the predecessor State File is readable");
    let join = fixture.step(&parent);
    assert!(
        combined(&join).contains(&format!(
            "ledger children missing on disk: {successor}; the expected set never silently shrinks"
        )),
        "B1: the join guard refuses naming the recorded successor not on disk: {}",
        combined(&join)
    );
    assert_eq!(
        fs::read(fixture.record_path(&predecessor))
            .expect("the predecessor State File is readable"),
        state_before_join,
        "B1: the refused join writes nothing"
    );

    // Step and hold on the predecessor wait on its held Run lock and refuse
    // with today's wait-expired text, writing nothing.
    let run_lock = format!("{}/locks/runs/{predecessor}.lock", fixture.engine_root());
    let step_wait = fixture.step(&predecessor);
    assert_wait_expired(&step_wait, "rtm: step: ", &run_lock);
    let hold_wait = fixture.rtm(&[
        "hold",
        "--run",
        &predecessor,
        "--blocker",
        ".arca/issue/i-777-blocker",
        "--confirm",
        &format!("hold {predecessor}"),
    ]);
    assert_wait_expired(&hold_wait, "rtm: hold refused; ", &run_lock);
    assert_eq!(
        fs::read(fixture.record_path(&predecessor))
            .expect("the predecessor State File is readable"),
        state_before_join,
        "B1: the wait-expired step and hold write nothing"
    );

    // The root-domain writers wait at the root lock and refuse, writing
    // nothing; so does a second respawn of the predecessor.
    let root_lock = format!("{}/locks/root.lock", fixture.engine_root());
    let b1_snapshot = fixture.engine_snapshot();
    let b1_ledger = fixture.ledger_text(&parent);
    let abandon_wait = fixture.rtm(&[
        "abandon",
        "--run",
        &predecessor,
        "--confirm",
        &format!("abandon {predecessor}"),
    ]);
    assert_wait_expired(&abandon_wait, "rtm: abandon refused; ", &root_lock);
    let spawn_wait = fixture.rtm(&["spawn", "rev", "--run", &parent, "--bind", "ticket=t-9"]);
    assert_wait_expired(&spawn_wait, "rtm: spawn: ", &root_lock);
    let start_wait = fixture.rtm(&["start"]);
    assert_wait_expired(&start_wait, "rtm: start: ", &root_lock);
    let second_wait = fixture.respawn(&predecessor);
    assert_wait_expired(&second_wait, "rtm: respawn refused; ", &root_lock);
    assert_eq!(
        fixture.engine_snapshot(),
        b1_snapshot,
        "B1: the wait-expired writers and the second respawn write nothing"
    );
    assert_eq!(
        fixture.ledger_text(&parent),
        b1_ledger,
        "B1: the ledger is unchanged through every wait"
    );

    // Release: the paused invocation completes the recorded operation.
    fs::write(&release, "release\n").expect("write the pause release");
    let finished = paused.wait_bounded(30, "WRSV-001-01: the released respawn completes");
    assert!(
        finished.status.success(),
        "B1 -> B3: the released respawn succeeds: {}",
        combined(&finished)
    );
    assert_eq!(
        stdout(&finished),
        format!(
            "rtm: run superseded; successor run {successor} minted at \
             .ratmac/runs/{successor}/\n"
        ),
        "B1 -> B3: the success line names the recorded successor"
    );
    assert_eq!(stderr(&finished), String::new());

    // B3: the completed state. The predecessor is retired, the successor is
    // admitted, usable, and owned by the durable entry.
    assert!(
        !fixture.record_path(&predecessor).exists(),
        "B3: the predecessor's admission state is retired"
    );
    assert!(
        fixture.run_dir(&predecessor).join("spawn-ledger").is_file(),
        "B3: the predecessor keeps its reserved directory with its spawn-ledger"
    );
    let successor_record =
        fs::read_to_string(fixture.record_path(&successor)).expect("B3: the successor is admitted");
    assert!(
        successor_record.contains("state = \"review\"")
            && successor_record.contains("status = \"planned\""),
        "B3: the successor is minted from the recorded class: {successor_record}"
    );
    let status_successor = fixture.rtm(&["status", "--run", &successor]);
    assert!(
        status_successor.status.success(),
        "B3: the successor is usable: {}",
        combined(&status_successor)
    );
    let status_predecessor = fixture.rtm(&["status", "--run", &predecessor]);
    assert!(
        !status_predecessor.status.success()
            && combined(&status_predecessor).contains(&format!(
                "run {predecessor} is terminal: its admission state is retired"
            )),
        "B3: the predecessor refuses as retired: {}",
        combined(&status_predecessor)
    );
    let b3_surface = fixture.status_surface();
    assert!(
        b3_surface.contains(&row(
            &predecessor,
            &[&child_role(&parent, "t-102"), "retired"]
        )),
        "B3: the predecessor's row ends retired: {b3_surface}"
    );
    assert!(
        b3_surface.contains(&row(
            &successor,
            &[
                &child_role(&parent, "t-102"),
                &format!("supersedes {predecessor}"),
                "state review; status planned"
            ]
        )),
        "B3: the successor's row reads its role then its State and status: {b3_surface}"
    );
    let join_b3 = fixture.step(&parent);
    assert!(
        !combined(&join_b3).contains("ledger children missing on disk"),
        "B3: the join guard judges the successor's own status: {}",
        combined(&join_b3)
    );
    let step_successor = fixture.step(&successor);
    assert!(
        step_successor.status.success(),
        "B3: step acts on the successor's recorded class: {}",
        combined(&step_successor)
    );
    let cap = fixture.rtm(&["spawn", "rev", "--run", &successor, "--bind", "ticket=t-9"]);
    assert!(
        !cap.status.success()
            && combined(&cap).contains(&format!(
                "run {successor} is a ledger-recorded child; composition is capped at one level"
            )),
        "B3: spawn refuses the successor by the one-level cap: {}",
        combined(&cap)
    );
    let supersede = fixture.respawn(&successor);
    assert!(
        supersede.status.success(),
        "B3: respawn may supersede the successor: {}",
        combined(&supersede)
    );

    // B2: the predecessor retired and the successor not yet admitted.
    let b2 = Fixture::create("01-b2");
    let (b2_parent, b2_predecessor) = recorded_child(&b2, "t-102");
    let (b2_marker, b2_release) = marker_pair("01-b2");
    let mut b2_paused =
        b2.spawn_paused_respawn("after-retire", &b2_marker, &b2_release, &b2_predecessor);
    wait_for_file(
        &b2_marker,
        15,
        "WRSV-001-01: the after-retire pause publishes its marker (the \
         RATMAC_TEST_RESPAWN_PAUSE seam must stop the respawn after the \
         predecessor is retired and before the successor is admitted)",
    );
    assert_eq!(
        fs::read_to_string(&b2_marker).expect("the pause marker is readable"),
        "after-retire\n",
        "WRSV-001-01: the marker carries the boundary name"
    );
    let b2_successor = recorded_successor(&b2, &b2_parent, &b2_predecessor)
        .expect("B2: the successor entry is durable");
    assert!(
        !b2.record_path(&b2_predecessor).exists(),
        "B2: the predecessor is retired"
    );
    assert!(
        !b2.run_dir(&b2_successor).exists(),
        "B2: the successor is not yet admitted"
    );
    let b2_entries = entries(&b2.ledger_text(&b2_parent));
    assert_eq!(field(&b2_entries[0], "id"), b2_predecessor);
    assert!(
        abandoned(&b2_entries[0]),
        "B2: the predecessor's entry carries the abandoned mark"
    );
    let b2_surface = b2.status_surface();
    assert!(
        b2_surface.contains(&row(
            &b2_predecessor,
            &[&child_role(&b2_parent, "t-102"), "retired"]
        )),
        "B2: the predecessor's row keeps its role fields and ends retired: {b2_surface}"
    );
    assert!(
        b2_surface.contains(&row(
            &b2_successor,
            &[
                &child_role(&b2_parent, "t-102"),
                &format!("supersedes {b2_predecessor}"),
                &format!(
                    "not on the roster ({}/runs/{b2_successor} is absent)",
                    b2.engine_root()
                ),
            ]
        )),
        "B2: the successor renders the same row as at B1: {b2_surface}"
    );
    let b2_doctor = combined(&b2.rtm(&["doctor"]));
    assert!(
        b2_doctor.contains(&format!(
            "Run Record: .ratmac/runs/{b2_predecessor}/run.toml (absent — run {b2_predecessor} is retired)"
        )),
        "B2: doctor reports the predecessor retired: {b2_doctor}"
    );
    successor_is_unaddressable(&b2, &b2_parent, &b2_predecessor, &b2_successor);
    let b2_join = b2.step(&b2_parent);
    assert!(
        combined(&b2_join).contains(&format!(
            "ledger children missing on disk: {b2_successor}; the expected set never silently shrinks"
        )),
        "B2: the join guard still refuses naming the recorded successor: {}",
        combined(&b2_join)
    );
    assert_wait_expired(
        &b2.step(&b2_predecessor),
        "rtm: step: ",
        &format!("{}/locks/runs/{b2_predecessor}.lock", b2.engine_root()),
    );
    assert_wait_expired(
        &b2.respawn(&b2_predecessor),
        "rtm: respawn refused; ",
        &format!("{}/locks/root.lock", b2.engine_root()),
    );
    fs::write(&b2_release, "release\n").expect("write the pause release");
    let b2_finished = b2_paused.wait_bounded(30, "WRSV-001-01: the released B2 respawn completes");
    assert!(
        b2_finished.status.success()
            && stdout(&b2_finished)
                == format!(
                    "rtm: run superseded; successor run {b2_successor} minted at \
                 .ratmac/runs/{b2_successor}/\n"
                ),
        "B2 -> B3: the released respawn admits the recorded successor: {}",
        combined(&b2_finished)
    );
    assert!(
        b2.record_path(&b2_successor).is_file(),
        "B2 -> B3: the recorded successor is admitted"
    );

    // The append-only ledger kept every prior byte through both boundaries.
    assert!(
        fixture
            .ledger_text(&parent)
            .starts_with(&ledger_before.replacen("abandoned = false", "abandoned = true", 1)),
        "the ledger keeps every prior byte as a prefix"
    );
}

/// WRSV-001-02: inject each append, retirement, and admission fault; every
/// refusal keeps the exact stderr and exit, leaves the predecessor or one
/// fully recorded recoverable successor on disk, and never deletes evidence.
#[test]
fn wrsv_001_02() {
    // Positive control: the unpause respawn completes.
    let control = Fixture::create("02-control");
    let (_control_parent, control_child) = recorded_child(&control, "t-102");
    let control_respawn = control.respawn(&control_child);
    assert!(
        control_respawn.status.success(),
        "WRSV-001-02 control: the respawn succeeds: {}",
        combined(&control_respawn)
    );
    drop(control);

    // ledger-append: the AbsentAfterError row.
    let absent = Fixture::create("02-absent");
    let (absent_parent, absent_child) = recorded_child(&absent, "t-102");
    let absent_before = absent.ledger_text(&absent_parent);
    let refused = absent.rtm_env(
        &[
            "respawn",
            "--run",
            &absent_child,
            "--confirm",
            &format!("respawn {absent_child}"),
        ],
        &[(FAULT_VAR, "ledger-append")],
    );
    assert!(
        !refused.status.success() && stdout(&refused).is_empty(),
        "WRSV-001-02 ledger-append: the refusal exits 1 with empty stdout: {}",
        combined(&refused)
    );
    assert_eq!(
        stderr(&refused),
        format!(
            "rtm: respawn refused; respawn cannot record the successor entry for run-003: \
             injected successor-ledger append failure; nothing was admitted and {absent_child} \
             keeps its admission state; the reserved id run-003 is never reused; repeat this \
             command to respawn with a fresh id\n"
        ),
        "WRSV-001-02 ledger-append: the AbsentAfterError refusal text"
    );
    assert!(
        absent.record_path(&absent_child).is_file(),
        "WRSV-001-02 ledger-append: the predecessor stays live"
    );
    assert_eq!(
        absent.ledger_text(&absent_parent),
        absent_before,
        "WRSV-001-02 ledger-append: the ledger bytes are unchanged"
    );
    assert!(
        !absent.run_dir("run-003").exists(),
        "WRSV-001-02 ledger-append: no successor directory exists"
    );
    assert!(
        absent.mint_text().contains("highest = 3"),
        "WRSV-001-02 ledger-append: the ordinal stays reserved"
    );
    let repeat = absent.respawn(&absent_child);
    assert!(
        repeat.status.success() && stdout(&repeat).contains("successor run run-004"),
        "WRSV-001-02 ledger-append: the repeat respawns with a fresh id: {}",
        combined(&repeat)
    );

    // ledger-append-committed: the RecordedAfterError row - a warning, then
    // the ordinary completion.
    let committed = Fixture::create("02-committed");
    let (committed_parent, committed_child) = recorded_child(&committed, "t-102");
    let warned = committed.rtm_env(
        &[
            "respawn",
            "--run",
            &committed_child,
            "--confirm",
            &format!("respawn {committed_child}"),
        ],
        &[(FAULT_VAR, "ledger-append-committed")],
    );
    assert!(
        warned.status.success(),
        "WRSV-001-02 ledger-append-committed: the run proceeds: {}",
        combined(&warned)
    );
    assert_eq!(
        stdout(&warned),
        "rtm: run superseded; successor run run-003 minted at .ratmac/runs/run-003/\n",
        "WRSV-001-02 ledger-append-committed: the success line prints"
    );
    assert_eq!(
        stderr(&warned),
        format!(
            "warning: successor ledger append for run-003 reported injected \
             successor-ledger append failure, but {} records the complete entry; treating the \
             successor as committed\n",
            committed.ledger_display(&committed_parent)
        ),
        "WRSV-001-02 ledger-append-committed: today's warning text"
    );
    assert!(
        !committed.record_path(&committed_child).exists()
            && committed.record_path("run-003").is_file(),
        "WRSV-001-02 ledger-append-committed: steps 2 and 3 completed"
    );

    // ledger-append-partial: the Indeterminate row - bytes left as they are.
    let partial = Fixture::create("02-partial");
    let (partial_parent, partial_child) = recorded_child(&partial, "t-102");
    let partial_before = partial.ledger_text(&partial_parent);
    let indeterminate = partial.rtm_env(
        &[
            "respawn",
            "--run",
            &partial_child,
            "--confirm",
            &format!("respawn {partial_child}"),
        ],
        &[(FAULT_VAR, "ledger-append-partial")],
    );
    assert!(
        !indeterminate.status.success() && stdout(&indeterminate).is_empty(),
        "WRSV-001-02 ledger-append-partial: the refusal exits 1: {}",
        combined(&indeterminate)
    );
    assert_eq!(
        stderr(&indeterminate),
        format!(
            "rtm: respawn refused; respawn cannot determine whether its successor entry \
             committed for run-003: injected successor-ledger append failure; nothing was \
             admitted and {partial_child} keeps its admission state; the ledger {} was left \
             exactly as it now reads; inspect it before retrying\n",
            partial.ledger_display(&partial_parent)
        ),
        "WRSV-001-02 ledger-append-partial: the Indeterminate refusal text"
    );
    let partial_after = partial.ledger_text(&partial_parent);
    assert!(
        partial_after.starts_with(&partial_before) && partial_after != partial_before,
        "WRSV-001-02 ledger-append-partial: the partial bytes remain, never deleted"
    );
    assert!(
        partial.record_path(&partial_child).is_file() && !partial.run_dir("run-003").exists(),
        "WRSV-001-02 ledger-append-partial: the predecessor stays live, no successor directory"
    );
    // A human repairs the ledger; the repeat then records a fresh successor.
    fs::write(partial.ledger_path(&partial_parent), &partial_before).expect("repair the ledger");
    let repaired = partial.respawn(&partial_child);
    assert!(
        repaired.status.success() && stdout(&repaired).contains("successor run run-004"),
        "WRSV-001-02 ledger-append-partial: after repair the repeat respawns freshly: {}",
        combined(&repaired)
    );

    // retire: the retirement refusal row - the complete recoverable
    // replacement is recorded.
    let retire = Fixture::create("02-retire");
    let (retire_parent, retire_child) = recorded_child(&retire, "t-102");
    let retired_refusal = retire.rtm_env(
        &[
            "respawn",
            "--run",
            &retire_child,
            "--confirm",
            &format!("respawn {retire_child}"),
        ],
        &[(FAULT_VAR, "retire")],
    );
    assert!(
        !retired_refusal.status.success() && stdout(&retired_refusal).is_empty(),
        "WRSV-001-02 retire: the refusal exits 1: {}",
        combined(&retired_refusal)
    );
    assert_eq!(
        stderr(&retired_refusal),
        format!(
            "rtm: respawn refused; respawn interrupted retiring {retire_child}: injected \
             respawn fault at retire; the successor run-003 is recorded in {} and not yet \
             admitted; repeat this respawn command to finish it\n",
            retire.ledger_display(&retire_parent)
        ),
        "WRSV-001-02 retire: the retirement-refusal text"
    );
    assert!(
        retire.record_path(&retire_child).is_file(),
        "WRSV-001-02 retire: the predecessor is live and unretired"
    );
    assert_eq!(
        recorded_successor(&retire, &retire_parent, &retire_child),
        Some("run-003".to_owned()),
        "WRSV-001-02 retire: the complete recoverable replacement is recorded"
    );
    assert!(
        !retire.run_dir("run-003").exists(),
        "WRSV-001-02 retire: no successor directory exists"
    );
    let retire_repeat = retire.respawn(&retire_child);
    assert!(
        retire_repeat.status.success()
            && stdout(&retire_repeat)
                == "rtm: run superseded; successor run run-003 minted at .ratmac/runs/run-003/\n",
        "WRSV-001-02 retire: the repeat finishes the recorded successor: {}",
        combined(&retire_repeat)
    );
    assert!(
        !retire.record_path(&retire_child).exists() && retire.record_path("run-003").is_file(),
        "WRSV-001-02 retire: the repeat retired the predecessor and admitted run-003"
    );

    // admit: the admission failure row - retired predecessor, durable entry,
    // nothing deleted.
    let admit = Fixture::create("02-admit");
    let (admit_parent, admit_child) = recorded_child(&admit, "t-102");
    let admit_refusal = admit.rtm_env(
        &[
            "respawn",
            "--run",
            &admit_child,
            "--confirm",
            &format!("respawn {admit_child}"),
        ],
        &[(FAULT_VAR, "admit")],
    );
    assert!(
        !admit_refusal.status.success() && stdout(&admit_refusal).is_empty(),
        "WRSV-001-02 admit: the refusal exits 1: {}",
        combined(&admit_refusal)
    );
    assert_eq!(
        stderr(&admit_refusal),
        format!(
            "rtm: respawn refused; respawn cannot admit the recorded successor run-003: \
             injected respawn fault at admit; {admit_child} is retired and {} records run-003; \
             repeat this respawn command to finish it\n",
            admit.ledger_display(&admit_parent)
        ),
        "WRSV-001-02 admit: the admission-failure text"
    );
    assert!(
        !admit.record_path(&admit_child).exists(),
        "WRSV-001-02 admit: the predecessor is retired"
    );
    assert_eq!(
        recorded_successor(&admit, &admit_parent, &admit_child),
        Some("run-003".to_owned()),
        "WRSV-001-02 admit: the entry stays durable"
    );
    let admit_repeat = admit.respawn(&admit_child);
    assert!(
        admit_repeat.status.success()
            && stdout(&admit_repeat)
                == "rtm: run superseded; successor run run-003 minted at .ratmac/runs/run-003/\n",
        "WRSV-001-02 admit: the repeat admits the same id, idempotently: {}",
        combined(&admit_repeat)
    );
    assert!(
        admit.record_path("run-003").is_file(),
        "WRSV-001-02 admit: the recorded successor is admitted after the repeat"
    );
}

/// WRSV-001-03: repeat recovery after a kill at each boundary and after a
/// human abandonment, and race two respawns of one predecessor: one recorded
/// successor, no reused id, no deadlock.
#[test]
fn wrsv_001_03() {
    // Positive control: the unpause respawn completes.
    let control = Fixture::create("03-control");
    let (_control_parent, control_child) = recorded_child(&control, "t-102");
    assert!(
        control.respawn(&control_child).status.success(),
        "WRSV-001-03 control: the respawn succeeds"
    );
    drop(control);

    // A kill at B1 leaves the predecessor live with the recorded successor;
    // the repeated command finishes that successor, never a duplicate.
    let killed = Fixture::create("03-kill-b1");
    let (killed_parent, killed_child) = recorded_child(&killed, "t-102");
    let (marker, release) = marker_pair("03-kill-b1");
    let mut paused = killed.spawn_paused_respawn("after-record", &marker, &release, &killed_child);
    wait_for_file(
        &marker,
        15,
        "WRSV-001-03: the after-record pause publishes its marker (the kill \
         fixture needs the RATMAC_TEST_RESPAWN_PAUSE seam)",
    );
    let killed_successor = recorded_successor(&killed, &killed_parent, &killed_child)
        .expect("B1: the successor entry is durable before the kill");
    assert!(
        killed.record_path(&killed_child).is_file(),
        "B1: the predecessor is live at the kill boundary"
    );
    paused.kill();
    let recovered = killed.respawn(&killed_child);
    assert!(
        recovered.status.success()
            && stdout(&recovered)
                == format!(
                    "rtm: run superseded; successor run {killed_successor} minted at \
                     .ratmac/runs/{killed_successor}/\n"
                ),
        "WRSV-001-03: the repeat finishes the recorded successor the killed \
         invocation left, with the same id: {}",
        combined(&recovered)
    );
    assert!(
        !killed.record_path(&killed_child).exists()
            && killed.record_path(&killed_successor).is_file(),
        "WRSV-001-03: the recovery retired the predecessor and admitted the successor"
    );
    assert_eq!(
        entries(&killed.ledger_text(&killed_parent))
            .iter()
            .filter(|entry| supersedes_id(entry) == Some(killed_child.as_str()))
            .count(),
        1,
        "WRSV-001-03: exactly one recorded successor of the predecessor"
    );

    // A kill at B1 handed to a human abandonment: the abandon succeeds as
    // today, and the repeat then admits the recorded successor.
    let human = Fixture::create("03-human");
    let (human_parent, human_child) = recorded_child(&human, "t-102");
    let (human_marker, human_release) = marker_pair("03-human");
    let mut human_paused =
        human.spawn_paused_respawn("after-record", &human_marker, &human_release, &human_child);
    wait_for_file(
        &human_marker,
        15,
        "WRSV-001-03: the after-record pause publishes its marker for the human route",
    );
    let human_successor = recorded_successor(&human, &human_parent, &human_child)
        .expect("B1: the successor entry is durable before the kill");
    human_paused.kill();
    let abandoned_predecessor = human.rtm(&[
        "abandon",
        "--run",
        &human_child,
        "--confirm",
        &format!("abandon {human_child}"),
    ]);
    assert!(
        abandoned_predecessor.status.success()
            && combined(&abandoned_predecessor).contains("Run abandoned"),
        "WRSV-001-03: the human abandonment of the live predecessor succeeds: {}",
        combined(&abandoned_predecessor)
    );
    let human_repeat = human.respawn(&human_child);
    assert!(
        human_repeat.status.success()
            && stdout(&human_repeat)
                == format!(
                    "rtm: run superseded; successor run {human_successor} minted at \
                     .ratmac/runs/{human_successor}/\n"
                ),
        "WRSV-001-03: the repeat finds the predecessor retired with the recorded \
         successor unadmitted and admits it: {}",
        combined(&human_repeat)
    );
    assert!(
        human.record_path(&human_successor).is_file(),
        "WRSV-001-03: the recorded successor is admitted after the human route"
    );

    // Two concurrent respawns of one predecessor: the root lock serializes
    // the decision; the loser refuses wait-expired and writes nothing.
    let race = Fixture::create("03-race");
    let (race_parent, race_child) = recorded_child(&race, "t-102");
    let (race_marker, race_release) = marker_pair("03-race");
    let mut winner =
        race.spawn_paused_respawn("after-record", &race_marker, &race_release, &race_child);
    wait_for_file(
        &race_marker,
        15,
        "WRSV-001-03: the winning respawn pauses at after-record",
    );
    let race_successor =
        recorded_successor(&race, &race_parent, &race_child).expect("the winner recorded once");
    let race_snapshot = race.engine_snapshot();
    let mut loser = ChildGuard::spawn(support::command(ratmac_qa::engine_bin!(), &race.root).args(
        [
            "respawn",
            "--run",
            &race_child,
            "--confirm",
            &format!("respawn {race_child}"),
        ],
    ));
    let lost = loser.wait_bounded(
        30,
        "WRSV-001-03: the second respawn refuses at the root-lock deadline",
    );
    assert!(
        !lost.status.success() && stdout(&lost).is_empty(),
        "WRSV-001-03: the second respawn refuses: {}",
        combined(&lost)
    );
    assert!(
        stderr(&lost).starts_with(&format!(
            "rtm: respawn refused; lock wait expired: {}/locks/root.lock; owner token: \
             ratmac-lock-v1; pid=",
            race.engine_root()
        )),
        "WRSV-001-03: the loser refuses with today's wait-expired text at the root \
         lock: {}",
        stderr(&lost)
    );
    assert_eq!(
        race.engine_snapshot(),
        race_snapshot,
        "WRSV-001-03: the losing respawn writes nothing"
    );
    fs::write(&race_release, "release\n").expect("write the pause release");
    let won = winner.wait_bounded(30, "WRSV-001-03: the winning respawn completes");
    assert!(
        won.status.success()
            && stdout(&won)
                == format!(
                    "rtm: run superseded; successor run {race_successor} minted at \
                     .ratmac/runs/{race_successor}/\n"
                ),
        "WRSV-001-03: the winner completes the one recorded successor: {}",
        combined(&won)
    );
    assert_eq!(
        race.roster(),
        vec![
            race_parent.clone(),
            race_child.clone(),
            race_successor.clone()
        ],
        "WRSV-001-03: one recorded successor, no reused id, no duplicate"
    );
    assert_eq!(
        entries(&race.ledger_text(&race_parent))
            .iter()
            .filter(|entry| supersedes_id(entry) == Some(race_child.as_str()))
            .count(),
        1,
        "WRSV-001-03: exactly one successor entry of the predecessor"
    );

    // A second respawn parked at the root lock meets the winner's completed
    // state when the winner is released well inside the 5 s deadline: today's
    // already-terminal refusal, writing nothing beyond the winner's result.
    let early = Fixture::create("03-early-release");
    let (early_parent, early_child) = recorded_child(&early, "t-102");
    let (early_marker, early_release) = marker_pair("03-early-release");
    let mut early_winner =
        early.spawn_paused_respawn("after-record", &early_marker, &early_release, &early_child);
    wait_for_file(
        &early_marker,
        15,
        "WRSV-001-03: the winning respawn pauses at after-record for the early release",
    );
    let early_successor =
        recorded_successor(&early, &early_parent, &early_child).expect("the winner recorded once");
    let mut early_loser = ChildGuard::spawn(
        support::command(ratmac_qa::engine_bin!(), &early.root).args([
            "respawn",
            "--run",
            &early_child,
            "--confirm",
            &format!("respawn {early_child}"),
        ]),
    );
    // Park the loser at the root lock, then release the winner well inside
    // the loser's 5 s deadline.
    std::thread::sleep(Duration::from_millis(500));
    fs::write(&early_release, "release\n").expect("write the pause release");
    let early_won =
        early_winner.wait_bounded(30, "WRSV-001-03: the early-released winner completes");
    assert!(
        early_won.status.success()
            && stdout(&early_won)
                == format!(
                    "rtm: run superseded; successor run {early_successor} minted at \
                     .ratmac/runs/{early_successor}/\n"
                ),
        "WRSV-001-03: the winner completes the one recorded successor: {}",
        combined(&early_won)
    );
    let winner_result = early.engine_snapshot();
    let early_lost = early_loser.wait_bounded(
        15,
        "WRSV-001-03: the loser refuses after the winner's early release",
    );
    assert!(
        !early_lost.status.success() && stdout(&early_lost).is_empty(),
        "WRSV-001-03: the loser meets the winner's completed state: {}",
        combined(&early_lost)
    );
    assert_eq!(
        stderr(&early_lost),
        format!(
            "rtm: respawn refused; run {early_child} is already terminal: its admission \
             state is retired; nothing to supersede\n"
        ),
        "WRSV-001-03: after the winner completed, the loser prints today's \
         already-terminal refusal"
    );
    assert_eq!(
        early.engine_snapshot(),
        winner_result,
        "WRSV-001-03: the loser writes nothing beyond the winner's result"
    );

    // A successor a human has since abandoned is no recorded successor: the
    // repeated respawn of its retired predecessor meets today's
    // already-terminal refusal and the durable tree stays byte-identical.
    let post = Fixture::create("03-abandoned-successor");
    let (post_parent, post_child) = recorded_child(&post, "t-102");
    let completed = post.respawn(&post_child);
    assert!(
        completed.status.success(),
        "WRSV-001-03: the initial respawn succeeds: {}",
        combined(&completed)
    );
    let post_successor = recorded_successor(&post, &post_parent, &post_child)
        .expect("the successor entry is recorded");
    let retire_successor = post.rtm(&[
        "abandon",
        "--run",
        &post_successor,
        "--confirm",
        &format!("abandon {post_successor}"),
    ]);
    assert!(
        retire_successor.status.success(),
        "WRSV-001-03: the human abandons the successor: {}",
        combined(&retire_successor)
    );
    let after_abandon = post.engine_snapshot();
    assert!(
        !post.record_path(&post_successor).exists(),
        "WRSV-001-03: the abandoned successor stays retired"
    );
    let repeat = post.respawn(&post_child);
    assert!(
        !repeat.status.success() && stdout(&repeat).is_empty(),
        "WRSV-001-03: the repeat after the successor's abandonment refuses: {}",
        combined(&repeat)
    );
    assert_eq!(
        stderr(&repeat),
        format!(
            "rtm: respawn refused; run {post_child} is already terminal: its admission \
             state is retired; nothing to supersede\n"
        ),
        "WRSV-001-03: an abandoned successor is no recorded successor, so the \
         repeat meets today's already-terminal refusal"
    );
    assert_eq!(
        post.engine_snapshot(),
        after_abandon,
        "WRSV-001-03: the refused repeat leaves the durable tree byte-identical"
    );
    assert!(
        !post.record_path(&post_successor).exists(),
        "WRSV-001-03: no run.toml is recreated for the abandoned successor"
    );
    let successor_entry = entries(&post.ledger_text(&post_parent))
        .into_iter()
        .find(|entry| field(entry, "id") == post_successor)
        .expect("the successor entry is durable");
    assert!(
        abandoned(&successor_entry),
        "WRSV-001-03: the successor's ledger entry keeps its abandoned mark"
    );
}

/// WRSV-001-04: keep the unchanged behavior green - the inherited workspace,
/// class, and bindings, the supersedes link, the confirmation refusal twins
/// with untouched state, the unchanged top-level path, and the operation
/// log's kinds and stage order.
#[test]
fn wrsv_001_04() {
    let fixture = Fixture::create("04-kept");
    let (parent, child) = recorded_child(&fixture, "t-404");
    let before = fixture.ledger_text(&parent);
    let spawn_entries = entries(&before);
    let recorded_workspace = field(&spawn_entries[0], "workspace").to_owned();

    // The confirmation refusal twins: byte-identical refusals, untouched
    // state, and no mint.
    let engine_before = fixture.engine_snapshot();
    let twin_a = fixture.rtm(&["respawn", "--run", &child]);
    let twin_b = fixture.rtm(&["respawn", "--run", &child]);
    assert_eq!(
        (twin_a.status.code(), stdout(&twin_a), stderr(&twin_a)),
        (twin_b.status.code(), stdout(&twin_b), stderr(&twin_b)),
        "WRSV-001-04: the missing-confirmation twins are byte-identical"
    );
    assert_eq!(
        stderr(&twin_a),
        format!(
            "rtm: respawn refused; respawn is unconfirmed: a human must type --confirm \
             \"respawn {child}\"\n"
        ),
        "WRSV-001-04: the missing-confirmation refusal text"
    );
    let wrong_a = fixture.rtm(&["respawn", "--run", &child, "--confirm", "respawn run-999"]);
    let wrong_b = fixture.rtm(&["respawn", "--run", &child, "--confirm", "respawn run-999"]);
    assert_eq!(
        (wrong_a.status.code(), stdout(&wrong_a), stderr(&wrong_a)),
        (wrong_b.status.code(), stdout(&wrong_b), stderr(&wrong_b)),
        "WRSV-001-04: the mismatched-confirmation twins are byte-identical"
    );
    assert_eq!(
        stderr(&wrong_a),
        format!(
            "rtm: respawn refused; respawn is unconfirmed: confirmation \"respawn run-999\" \
             does not match the required phrase \"respawn {child}\"\n"
        ),
        "WRSV-001-04: the mismatched-confirmation refusal text"
    );
    assert_eq!(
        fixture.engine_snapshot(),
        engine_before,
        "WRSV-001-04: the refusal twins change no Engine byte, mint included"
    );

    // The completed respawn inherits the recorded workspace, class, and
    // bindings, and appends the supersedes link after the preserved prefix.
    let respawned = fixture.respawn(&child);
    assert!(
        respawned.status.success(),
        "WRSV-001-04: the confirmed respawn succeeds: {}",
        combined(&respawned)
    );
    let successor = id_after(&stdout(&respawned), "successor run ");
    let after = fixture.ledger_text(&parent);
    assert!(
        after.starts_with(&before.replacen("abandoned = false", "abandoned = true", 1)),
        "WRSV-001-04: the ledger is append-only"
    );
    let recorded = entries(&after);
    assert_eq!(recorded.len(), 2, "one successor entry is appended");
    let entry = &recorded[1];
    assert_eq!(field(entry, "id"), successor);
    assert_eq!(
        field(entry, "class"),
        "reviewer",
        "the successor keeps the recorded class"
    );
    assert_eq!(
        binding(entry, "ticket"),
        "t-404",
        "the successor inherits the recorded binding values"
    );
    assert_eq!(
        field(entry, "workspace"),
        recorded_workspace,
        "the successor inherits the recorded workspace"
    );
    assert_eq!(
        field(entry, "supersedes"),
        child,
        "the entry records the superseded id"
    );
    assert!(!field(entry, "spawned_at").is_empty());
    assert!(!abandoned(entry), "the successor entry is live");
    let record =
        fs::read_to_string(fixture.record_path(&successor)).expect("the successor is admitted");
    assert!(
        record.contains("state = \"review\"") && record.contains("status = \"planned\""),
        "the successor is minted from the recorded class: {record}"
    );

    // The already-terminal refusal on the completed state.
    let terminal = fixture.respawn(&child);
    assert!(
        !terminal.status.success() && stdout(&terminal).is_empty(),
        "WRSV-001-04: respawning the retired predecessor refuses: {}",
        combined(&terminal)
    );
    assert_eq!(
        stderr(&terminal),
        format!(
            "rtm: respawn refused; run {child} is already terminal: its admission state is \
             retired; nothing to supersede\n"
        ),
        "WRSV-001-04: today's already-terminal text"
    );

    // A top-level predecessor stays on today's path: no ledger entry, the
    // successor minted and rendered top-level, the predecessor retired.
    let top_ledger_before = fixture.ledger_text(&parent);
    let top = fixture.respawn(&parent);
    assert!(
        top.status.success(),
        "WRSV-001-04: the top-level respawn succeeds: {}",
        combined(&top)
    );
    let top_successor = id_after(&stdout(&top), "successor run ");
    assert_eq!(
        fixture.ledger_text(&parent),
        top_ledger_before,
        "WRSV-001-04: a top-level predecessor records no successor entry"
    );
    assert!(
        !fixture.record_path(&parent).exists() && fixture.record_path(&top_successor).is_file(),
        "WRSV-001-04: the top-level predecessor is retired, its successor admitted"
    );
    let top_surface = fixture.status_surface();
    assert!(
        top_surface.contains(&format!("{top_successor}: top-level;")),
        "WRSV-001-04: the top-level successor renders top-level: {top_surface}"
    );

    // `rtm respawn --help` keeps its route text.
    let help = fixture.rtm(&["respawn", "--help"]);
    assert!(
        help.status.success() && stdout(&help).contains("Usage: rtm respawn --run <id> --confirm"),
        "WRSV-001-04: the respawn help text is unchanged: {}",
        combined(&help)
    );

    // The operation log: only the documented kinds, the validation marks
    // before the first lock, and the stage order - successor-append ledger
    // marks first, records after the last lock, the reserved spawn-ledger
    // ledger mark last.
    let log_tree = support::TempTree::new("t122-04-oplog").expect("own the log tree");
    let observed_project = Fixture::create("04-oplog");
    let (_observed_parent, observed_child) = recorded_child(&observed_project, "t-102");
    let log = log_tree.join("respawn.log");
    let observed = observed_project.rtm_env(
        &[
            "respawn",
            "--run",
            &observed_child,
            "--confirm",
            &format!("respawn {observed_child}"),
        ],
        &[(OP_LOG_VAR, log.to_str().expect("the log path is UTF-8"))],
    );
    assert!(
        observed.status.success(),
        "WRSV-001-04: the observed respawn succeeds: {}",
        combined(&observed)
    );
    let log_text = fs::read_to_string(&log).expect("the operation log is readable");
    let lines: Vec<&str> = log_text.lines().collect();
    assert!(
        !lines.is_empty(),
        "WRSV-001-04: the respawn records its operations"
    );
    for line in &lines {
        assert!(
            matches!(
                *line,
                "roster" | "record" | "ledger" | "runbook" | "target" | "lock"
            ),
            "WRSV-001-04: the log records only the documented kinds, never `{line}`: {lines:?}"
        );
    }
    for required in ["lock", "ledger", "record"] {
        assert!(
            lines.contains(&required),
            "WRSV-001-04: the log records the `{required}` kind: {lines:?}"
        );
    }
    let first_lock = lines
        .iter()
        .position(|line| *line == "lock")
        .expect("the log records a lock mark");
    assert!(
        lines[..first_lock]
            .iter()
            .any(|line| matches!(*line, "roster" | "ledger" | "runbook" | "target")),
        "WRSV-001-04: no lock line precedes the validation marks, target included: {lines:?}"
    );
    let last_lock = lines
        .iter()
        .rposition(|line| *line == "lock")
        .expect("the log records a lock mark");
    assert!(
        lines[last_lock + 1..].contains(&"record"),
        "WRSV-001-04: the retirement and admission records follow the recording \
         step's locks: {lines:?}"
    );
    assert_eq!(
        lines.last(),
        Some(&"ledger"),
        "WRSV-001-04: the successor's reserved spawn-ledger mark is last: {lines:?}"
    );

    // A validation refusal precedes the first lock: its log holds no lock.
    let refusal_log = log_tree.join("refusal.log");
    let refused = observed_project.rtm_env(
        &[
            "respawn",
            "--run",
            "run-099",
            "--confirm",
            "respawn run-099",
        ],
        &[(
            OP_LOG_VAR,
            refusal_log.to_str().expect("the log path is UTF-8"),
        )],
    );
    assert!(
        !refused.status.success(),
        "WRSV-001-04: the unknown-run refusal refuses: {}",
        combined(&refused)
    );
    let refusal_text = fs::read_to_string(&refusal_log).expect("the refusal log is readable");
    let refusal_lines: Vec<&str> = refusal_text.lines().collect();
    assert!(
        !refusal_lines.contains(&"lock"),
        "WRSV-001-04: every validation refusal precedes the first lock: {refusal_lines:?}"
    );
}
