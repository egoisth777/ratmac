//! t-072 / ENS-003, ENS-004: one repository-wide Run namespace.
//!
//! ENSV-004 `run_created_in_primary_is_addressable_from_linked_worktree`
//! ENSV-005 `mint_record_never_reissues_a_deleted_run_id`
//!
//! A Git repository has one Engine runtime at its primary checkout.  Its
//! linked worktrees address that same Run roster while retaining independently
//! checked-out runbooks.  The durable mint record keeps allocation monotonic
//! even if an individual Run directory is removed out of band.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// One top-level parent with a declared reviewer child.  The child has a real
/// transition to its terminal State so the linked worktree can step it before
/// the primary checkout evaluates the parent's `join` guard.
const COMPOSED_RUNBOOK: &str = r#"
[classes.reviewer.bindings.ticket]
required = true

[classes.reviewer.states.review]
prompt = "Review the delegated ticket."

[classes.reviewer.states.approved]
prompt = "Approved."

[[classes.reviewer.transitions]]
from = "review"
to = "approved"

[states.plan]
prompt = "Plan."

[states.delegate]
prompt = "Delegate and wait."
guards = [{ kind = "join", require = "all_passed", min = 1 }]

[[states.delegate.spawns]]
class = "reviewer"
name = "review"
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

struct GitFixture {
    sandbox: PathBuf,
    primary: PathBuf,
    linked: Option<PathBuf>,
}

impl GitFixture {
    fn new(label: &str) -> Self {
        let sandbox = std::env::temp_dir().join(format!(
            "ratmac-t072-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock must be after the Unix epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&sandbox);

        let primary = sandbox.join("primary");
        fs::create_dir_all(primary.join(".ratmac")).expect("create primary Engine tree");
        fs::write(primary.join(".ratmac/ratmac.toml"), COMPOSED_RUNBOOK)
            .expect("write committed Machine Class");

        git_success(&primary, &["init"]);
        git_success(&primary, &["config", "core.autocrlf", "false"]);
        git_success(&primary, &["config", "user.email", "qa@example.invalid"]);
        git_success(&primary, &["config", "user.name", "Ratmac QA"]);
        git_success(&primary, &["add", "--", ".ratmac/ratmac.toml"]);
        git_success(&primary, &["commit", "-m", "fixture base"]);

        Self {
            sandbox,
            primary,
            linked: None,
        }
    }

    fn add_linked_worktree(&mut self) -> PathBuf {
        let linked = self.sandbox.join("linked");
        let output = Command::new("git")
            .args(["worktree", "add", "-b", "t072-linked"])
            .arg(&linked)
            .current_dir(&self.primary)
            .output()
            .expect("git worktree add must be executable");
        assert!(
            output.status.success(),
            "git worktree add failed: {}",
            combined(&output)
        );
        self.linked = Some(linked.clone());
        linked
    }
}

impl Drop for GitFixture {
    fn drop(&mut self) {
        if let Some(linked) = &self.linked {
            let _ = Command::new("git")
                .args(["worktree", "remove", "--force"])
                .arg(linked)
                .current_dir(&self.primary)
                .output();
        }
        let _ = fs::remove_dir_all(&self.sandbox);
    }
}

fn git(root: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("Git must be executable for this fixture")
}

fn git_success(root: &Path, args: &[&str]) {
    let output = git(root, args);
    assert!(
        output.status.success(),
        "git {args:?} failed in {}: {}",
        root.display(),
        combined(&output)
    );
}

fn rtm_at(root: &Path, args: &[&str]) -> Output {
    Command::new(ratmac_qa::engine_bin!())
        .args(args)
        .current_dir(root)
        .output()
        .expect("invoke built rtm binary")
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn minted_id(output: &Output, wording: &str) -> String {
    let text = combined(output);
    text.split(wording)
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or_else(|| panic!("rtm output must contain {wording:?}: {text}"))
        .to_owned()
}

/// Verify the public canonical spelling while extracting its comparable
/// ordinal.  `run-1000` remains canonical because it is what `run-{n:03}`
/// renders once the ordinal reaches four digits.
fn canonical_ordinal(id: &str) -> u64 {
    let digits = id
        .strip_prefix("run-")
        .unwrap_or_else(|| panic!("Run id must start with `run-`: {id:?}"));
    assert!(
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()),
        "Run id must end in decimal digits: {id:?}"
    );
    let ordinal = digits
        .parse::<u64>()
        .unwrap_or_else(|error| panic!("Run id ordinal must fit u64: {id:?}: {error}"));
    assert!(ordinal > 0, "Run id ordinal must be positive: {id:?}");
    assert_eq!(
        id,
        format!("run-{ordinal:03}"),
        "Run id must use the canonical `run-NNN` spelling"
    );
    ordinal
}

/// The resolved primary Engine root's direct Run-directory names are the
/// shared roster.
fn roster_at(primary: &Path) -> Vec<String> {
    let runs = primary.join(".ratmac/runs");
    assert!(
        runs.is_dir(),
        "ENSV: the primary Engine root must expose a .ratmac/runs/ roster at {}",
        runs.display()
    );
    let mut roster: Vec<String> = fs::read_dir(&runs)
        .expect("primary roster is listable")
        .map(|entry| entry.expect("roster entry is readable"))
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    roster.sort();
    roster
}

fn ledger_child_ids(path: &Path) -> Vec<String> {
    let text = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "spawn ledger must be readable at {}: {error}",
            path.display()
        )
    });
    let value: toml::Value = text
        .parse()
        .unwrap_or_else(|error| panic!("spawn ledger must be valid TOML: {error}\n{text}"));
    value["children"]
        .as_array()
        .expect("spawn ledger must contain a children array")
        .iter()
        .map(|entry| {
            entry["id"]
                .as_str()
                .expect("each spawn-ledger entry must name its child id")
                .to_owned()
        })
        .collect()
}

/// Read the durable record without depending on a private Scheduler
/// representation.
fn mint_record_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "ENSV-005: durable mint record missing at {}: {error}",
            path.display()
        )
    })
}

/// The durable record has one numeric high-water key, not a Run-id string or
/// a directory-derived allocation hint.
fn mint_record_highest(path: &Path) -> u64 {
    let text = mint_record_text(path);
    let document: toml::Value = text.parse().unwrap_or_else(|error| {
        panic!(
            "ENSV-005: durable mint record at {} must be valid TOML: {error}\n{text}",
            path.display()
        )
    });
    let table = document
        .as_table()
        .expect("ENSV-005: durable mint record must be a top-level table");
    assert_eq!(
        table.len(),
        1,
        "ENSV-005: durable mint record has exactly one key: {text}"
    );
    let highest = table
        .get("highest")
        .and_then(toml::Value::as_integer)
        .expect("ENSV-005: durable mint record has numeric highest = <u64>");
    u64::try_from(highest)
        .expect("ENSV-005: durable mint record highest must be a non-negative u64")
}

/// ENSV-004: `rtm spawn` in the primary checkout, `rtm step --run <child>` in
/// a linked worktree, and the parent's join-driving `rtm step --run <parent>`
/// in the primary all address the same child Run under one shared roster.
#[test]
fn run_created_in_primary_is_addressable_from_linked_worktree() {
    let mut fixture = GitFixture::new("cross-worktree");
    let linked = fixture.add_linked_worktree();

    let start = rtm_at(&fixture.primary, &["start"]);
    let start_text = combined(&start);
    assert!(
        start.status.success(),
        "primary start succeeds: {start_text}"
    );
    let parent = minted_id(&start, "started run ");
    canonical_ordinal(&parent);

    let enter_delegate = rtm_at(&fixture.primary, &["step", "--run", &parent]);
    let delegate_text = combined(&enter_delegate);
    assert!(
        enter_delegate.status.success(),
        "primary step into the spawning State succeeds: {delegate_text}"
    );
    assert!(
        delegate_text.contains("Delegate and wait."),
        "step renders the declared spawning State Prompt: {delegate_text}"
    );

    let spawn = rtm_at(
        &fixture.primary,
        &[
            "spawn",
            "review",
            "--run",
            &parent,
            "--bind",
            "ticket=ENSV-004",
        ],
    );
    let spawn_text = combined(&spawn);
    assert!(
        spawn.status.success(),
        "primary spawn succeeds: {spawn_text}"
    );
    let child = minted_id(&spawn, "spawned run ");
    canonical_ordinal(&child);
    assert_ne!(child, parent, "the spawned child receives its own Run id");
    assert!(
        spawn_text.contains(&format!(".ratmac/runs/{child}/")),
        "spawn reports the child Run directory: {spawn_text}"
    );

    let child_dir = fixture.primary.join(".ratmac/runs").join(&child);
    assert!(
        child_dir.join("run.toml").is_file(),
        "ENSV-004: spawn creates the child only under the primary .ratmac/runs/ tree"
    );
    assert_eq!(
        roster_at(&fixture.primary),
        vec![parent.clone(), child.clone()],
        "ENSV-004: the primary roster has one parent and exactly one child, not a duplicate allocation"
    );
    let ledger = fixture
        .primary
        .join(".ratmac/runs")
        .join(&parent)
        .join("spawn-ledger");
    assert_eq!(
        ledger_child_ids(&ledger),
        vec![child.clone()],
        "ENSV-004: the parent ledger has one entry naming the sole child"
    );
    assert!(
        !linked.join(".ratmac/runs").exists(),
        "ENSV-004: the linked worktree must not receive a private .ratmac/runs/ tree"
    );

    let child_step = rtm_at(&linked, &["step", "--run", &child]);
    let child_step_text = combined(&child_step);
    assert!(
        child_step.status.success(),
        "ENSV-004: linked step must address primary child {child}: {child_step_text}"
    );
    assert!(
        child_step_text.contains("Approved."),
        "linked step renders the child terminal State Prompt: {child_step_text}"
    );
    let child_state = fs::read_to_string(child_dir.join("run.toml"))
        .expect("the primary child State File remains readable after linked step");
    assert!(
        child_state.contains("state = \"approved\"") && child_state.contains("\"passed\""),
        "ENSV-004: the linked step changes the one primary child State File: {child_state}"
    );
    assert_eq!(
        roster_at(&fixture.primary),
        vec![parent.clone(), child.clone()],
        "ENSV-004: linked motion must not mint a checkout-local or duplicate child"
    );
    assert_eq!(
        ledger_child_ids(&ledger),
        vec![child.clone()],
        "ENSV-004: linked motion preserves the one primary ledger entry"
    );
    assert!(
        !linked.join(".ratmac/runs").exists(),
        "ENSV-004: linked motion must not create a local Run roster"
    );

    // There is no separate `rtm join` verb: `rtm step --run <parent>` evaluates
    // the declared `join` guard and advances the parent when the child passed.
    let join = rtm_at(&fixture.primary, &["step", "--run", &parent]);
    let join_text = combined(&join);
    assert!(
        join.status.success(),
        "ENSV-004: primary join step must see linked child {child}: {join_text}"
    );
    assert!(
        join_text.contains("Done."),
        "the joined parent renders its terminal State Prompt: {join_text}"
    );
    let parent_state = fs::read_to_string(
        fixture
            .primary
            .join(".ratmac/runs")
            .join(&parent)
            .join("run.toml"),
    )
    .expect("the primary parent State File remains readable after join");
    assert!(
        parent_state.contains("state = \"done\""),
        "ENSV-004: the primary join advances the original parent: {parent_state}"
    );
    assert_eq!(
        roster_at(&fixture.primary),
        vec![parent.clone(), child.clone()],
        "ENSV-004: spawn, linked child step, and primary join identify one shared child in one roster"
    );
    assert_eq!(
        ledger_child_ids(&ledger),
        vec![child],
        "ENSV-004: primary join preserves the one ledger entry that names the shared child"
    );
    assert!(
        !linked.join(".ratmac/runs").exists(),
        "ENSV-004: no operation may leave a linked-worktree .ratmac/runs/ tree"
    );
}

/// ENSV-005: allocation reads durable history rather than treating the current
/// Run-directory listing as the whole namespace.
#[test]
fn mint_record_never_reissues_a_deleted_run_id() {
    let fixture = GitFixture::new("mint-record");

    let first_start = rtm_at(&fixture.primary, &["start"]);
    let first_text = combined(&first_start);
    assert!(
        first_start.status.success(),
        "first start succeeds: {first_text}"
    );
    let first = minted_id(&first_start, "started run ");
    let first_ordinal = canonical_ordinal(&first);
    let mint_path = fixture.primary.join(".ratmac/mint.toml");
    let first_record = mint_record_text(&mint_path);
    assert_eq!(
        mint_record_highest(&mint_path),
        first_ordinal,
        "ENSV-005: first mint record must persist numeric high-water ordinal {first_ordinal}"
    );

    let deleted_dir = fixture.primary.join(".ratmac/runs").join(&first);
    assert!(
        deleted_dir.is_dir(),
        "the first start must create exactly the directory that this fixture removes"
    );
    fs::remove_dir_all(&deleted_dir).expect("remove only the first Run directory");
    assert!(
        !deleted_dir.exists(),
        "the fixture deleted only the first Run directory before the second mint"
    );
    assert_eq!(
        mint_record_text(&mint_path),
        first_record,
        "removing a Run directory must not erase or rewrite the durable mint record"
    );
    assert!(
        roster_at(&fixture.primary).is_empty(),
        "the deleted Run directory was the only roster entry"
    );

    let second_start = rtm_at(&fixture.primary, &["start"]);
    let second_text = combined(&second_start);
    assert!(
        second_start.status.success(),
        "second start succeeds after the directory deletion: {second_text}"
    );
    let second = minted_id(&second_start, "started run ");
    let second_ordinal = canonical_ordinal(&second);
    assert!(
        second_ordinal > first_ordinal,
        "ENSV-005: deleting {first} must not make its id available again; second mint was {second}"
    );
    assert_ne!(
        second, first,
        "ENSV-005: the deleted id must never be reissued"
    );

    let second_record = mint_record_text(&mint_path);
    assert_eq!(
        mint_record_highest(&mint_path),
        second_ordinal,
        "ENSV-005: mint record must advance to numeric high-water ordinal {second_ordinal}"
    );
    assert_ne!(
        second_record, first_record,
        "ENSV-005: mint record bytes must advance with the second allocation"
    );
    assert_eq!(
        roster_at(&fixture.primary),
        vec![second],
        "ENSV-005: only the newly minted Run directory is present after the deletion"
    );
}
