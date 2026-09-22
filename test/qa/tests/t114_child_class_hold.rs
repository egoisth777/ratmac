//! WRS-002: hold plans and writes use the addressed child's own class.

use ratmac::blocked::{apply_hold, plan_hold, HoldRequest};
use ratmac::model::{RunState, Status};
use ratmac::{Scheduler, StepRequest};
use ratmac_qa::tempgit::TempRepo;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const BLOCKER: &str = "work/blocker.txt";

struct Fixture {
    repo: TempRepo,
    parent: String,
    child: String,
    sibling: String,
    workspace: PathBuf,
}

fn runbook(same_name: bool, child_route: bool) -> String {
    let parent_work = if same_name { "work" } else { "delegate" };
    let child_hold = if child_route {
        "[[classes.worker.transitions]]\nfrom = 'work'\nto = 'prep'\nblocked-route = true\n"
    } else {
        ""
    };
    format!(
        r#"
[roots]
issue = "work"

[states.plan]
prompt = "Prepare parent."
[states.{parent_work}]
prompt = "Spawn children."
[[states.{parent_work}.spawns]]
name = "worker"
class = "worker"
[[states.{parent_work}.spawns]]
name = "sibling"
class = "sibling"
[states.parent_done]
prompt = "Parent complete."
[[transitions]]
from = "plan"
to = "{parent_work}"
[[transitions]]
from = "{parent_work}"
to = "parent_done"
[[transitions]]
from = "{parent_work}"
to = "plan"
blocked-route = true

[classes.worker.states.prep]
prompt = "Prepare child."
[classes.worker.states.work]
prompt = "Child work."
[classes.worker.states.child_done]
prompt = "Child complete."
[[classes.worker.transitions]]
from = "prep"
to = "work"
[[classes.worker.transitions]]
from = "work"
to = "child_done"
{child_hold}

[classes.sibling.states.sibling_prep]
prompt = "Prepare sibling."
[classes.sibling.states.work]
prompt = "Sibling work with the same name."
[classes.sibling.states.sibling_done]
prompt = "Sibling complete."
[[classes.sibling.transitions]]
from = "sibling_prep"
to = "work"
[[classes.sibling.transitions]]
from = "work"
to = "sibling_done"
[[classes.sibling.transitions]]
from = "work"
to = "sibling_prep"
blocked-route = true

[classes.nohold.states.prep]
prompt = "Prepare a class with no pause route."
[classes.nohold.states.work]
prompt = "Same State name, no blocked route."
[classes.nohold.states.done]
prompt = "Done."
[[classes.nohold.transitions]]
from = "prep"
to = "work"
[[classes.nohold.transitions]]
from = "work"
to = "done"
"#
    )
}

impl Fixture {
    fn new(label: &str, same_name: bool, child_route: bool, separate_workspace: bool) -> Self {
        let repo = TempRepo::new(&format!("t114-{label}"));
        repo.write(".ratmac/ratmac.toml", &runbook(same_name, child_route));
        repo.write(BLOCKER, "Opaque blocker; its content is not a gate.\n");
        repo.write("work/item.md", "Contributor-owned original bytes.\n");
        repo.write(
            "outside.txt",
            "This exists outside every declared workflow root.\n",
        );
        let workspace = if separate_workspace {
            repo.write("child-area/work/blocker.txt", "Child-only blocker.\n");
            repo.write("child-area/work/item.md", "Child contributor bytes.\n");
            repo.write("work/primary-only.txt", "Must not be read for the child.\n");
            repo.root().join("child-area")
        } else {
            repo.root().to_path_buf()
        };
        let parent = Scheduler::open(repo.root())
            .expect("valid composed runbook opens")
            .start()
            .expect("parent starts")
            .id()
            .expect("started Run has an address")
            .to_owned();
        step(repo.root(), &parent);
        let child = Scheduler::spawn_to_with_workspace(
            repo.root(),
            &parent,
            "worker",
            &BTreeMap::new(),
            Some(&workspace),
        )
        .expect("worker child spawns with valid ownership and workspace");
        let sibling = Scheduler::spawn_to(repo.root(), &parent, "sibling", &BTreeMap::new())
            .expect("independent sibling spawns");
        step(repo.root(), &child);
        step(repo.root(), &sibling);
        let fixture = Self {
            repo,
            parent,
            child,
            sibling,
            workspace,
        };
        for id in [&fixture.child, &fixture.sibling] {
            assert_eq!(fixture.record(id).state, "work");
            assert_eq!(fixture.record(id).status, Status::Planned);
        }
        fixture
    }

    fn root(&self) -> &Path {
        self.repo.root()
    }

    fn record(&self, id: &str) -> RunState {
        let text = fs::read_to_string(self.root().join(".ratmac/runs").join(id).join("run.toml"))
            .expect("read complete Engine-produced record");
        toml::from_str(&text).expect("Engine produced a valid Run Record")
    }

    fn request(&self, id: &str, blocker: &str) -> HoldRequest {
        HoldRequest {
            run: Some(id.to_owned()),
            blocker: Some(blocker.to_owned()),
            confirmation: Some(format!("hold {id}")),
        }
    }

    fn unchanged_refusal(&self, request: &HoldRequest, expected: &str) {
        let before = snapshot(self.root());
        let error = plan_hold(self.root(), request).expect_err("invalid hold must refuse");
        assert!(error.reason.contains(expected), "wrong refusal: {error}");
        assert_eq!(
            snapshot(self.root()),
            before,
            "refused planning wrote files"
        );
    }

    fn successful_hold(&self, id: &str, destination: &str) {
        let before = snapshot(self.root());
        let mut expected_record = self.record(id);
        expected_record.state = destination.to_owned();
        expected_record.status = Status::Blocked;
        expected_record.blocker = BLOCKER.to_owned();
        let plan = plan_hold(self.root(), &self.request(id, BLOCKER))
            .expect("a route in the addressed Run's own class must produce a plan");
        assert_eq!(snapshot(self.root()), before, "planning must be read-only");
        apply_hold(self.root(), &plan).expect("own-class plan must also apply in that class");
        let record = self.record(id);
        assert_eq!(
            record.state, destination,
            "hold selected another class's route"
        );
        assert_eq!(record.status, Status::Blocked);
        assert_eq!(record.blocker, BLOCKER);
        assert_eq!(record, expected_record, "hold changed unrelated Run fields");
        assert_only_hold_changed(self.root(), id, &before);
    }
}

fn step(root: &Path, id: &str) {
    Scheduler::open_run(root, id)
        .expect("valid addressed Run opens")
        .step(StepRequest::new("fixture prerequisite is complete"))
        .expect("ordinary fixture transition passes");
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, at: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(at).expect("snapshot directory is readable") {
            let entry = entry.expect("snapshot entry is readable");
            if at == root && entry.file_name() == ".git" {
                continue;
            }
            let path = entry.path();
            if entry.file_type().expect("read file kind").is_dir() {
                visit(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root)
                        .expect("snapshot stays local")
                        .to_path_buf(),
                    fs::read(&path).expect("snapshot file is readable"),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}

fn assert_only_hold_changed(root: &Path, id: &str, before: &BTreeMap<PathBuf, Vec<u8>>) {
    let mut expected = before.clone();
    let mut after = snapshot(root);
    let record = PathBuf::from(format!(".ratmac/runs/{id}/run.toml"));
    let log = PathBuf::from(".ratmac/log.md");
    assert_ne!(after.get(&record), expected.get(&record));
    assert!(
        after[&log].starts_with(&expected[&log]),
        "history prefix was rewritten"
    );
    assert!(
        after[&log].len() > expected[&log].len(),
        "hold must append history"
    );
    for path in [record, log] {
        expected.remove(&path);
        after.remove(&path);
    }
    assert_eq!(
        after, expected,
        "hold changed a parent, sibling, pin, ledger, or workflow file"
    );
}

#[test]
fn wrsv_002_01() {
    let fixture = Fixture::new("child-only-route", false, true, false);
    // The top-level graph has no State called work at all.
    fixture.successful_hold(&fixture.child, "prep");
}

#[test]
fn wrsv_002_02() {
    let fixture = Fixture::new("overlapping-names", true, true, false);
    // Parent work -> plan, worker work -> prep, sibling work -> sibling_prep.
    // A wrong planner either applies its wrong destination or is refused by
    // the correct apply boundary; a wrong apply boundary rejects a correct plan.
    fixture.successful_hold(&fixture.child, "prep");
    fixture.successful_hold(&fixture.sibling, "sibling_prep");
    assert_eq!(fixture.record(&fixture.parent).state, "work");
    assert_eq!(fixture.record(&fixture.parent).status, Status::Planned);
}

#[test]
fn wrsv_002_03() {
    let fixture = Fixture::new("refusal-controls", true, true, false);
    // Establish that the shared confirmation and blocker setup can really hold.
    fixture.successful_hold(&fixture.parent, "plan");
    let mut unconfirmed = fixture.request(&fixture.child, BLOCKER);
    unconfirmed.confirmation = None;
    fixture.unchanged_refusal(&unconfirmed, "unconfirmed");
    unconfirmed.confirmation = Some(format!("hold {}", fixture.sibling));
    fixture.unchanged_refusal(&unconfirmed, "does not match");

    let plan = plan_hold(fixture.root(), &fixture.request(&fixture.child, BLOCKER))
        .expect("valid child planning reaches the route boundary");
    step(fixture.root(), &fixture.child);
    assert_eq!(fixture.record(&fixture.child).status, Status::Passed);
    let before = snapshot(fixture.root());
    let refusal = apply_hold(fixture.root(), &plan).expect_err("a plan predating motion is stale");
    assert!(refusal.reason.contains("stale"), "{refusal}");
    assert_eq!(snapshot(fixture.root()), before);
    fixture.unchanged_refusal(&fixture.request(&fixture.child, BLOCKER), "terminal");

    let damaged = Fixture::new("malformed-ownership", true, true, false);
    let ledger = damaged
        .root()
        .join(".ratmac/runs")
        .join(&damaged.parent)
        .join("spawn-ledger");
    fs::write(&ledger, "children = [ malformed ownership\n").expect("plant malformed ledger");
    damaged.unchanged_refusal(&damaged.request(&damaged.child, BLOCKER), "ledger");

    // HoldPlan is opaque. Forge its ownership context, not its private fields:
    // an issued plan must not borrow a same-named parent's route on apply.
    let forged = Fixture::new("ownership-changed-after-plan", true, true, false);
    let plan = plan_hold(forged.root(), &forged.request(&forged.child, BLOCKER))
        .expect("obtain the originally valid opaque plan");
    let ledger = forged
        .root()
        .join(".ratmac/runs")
        .join(&forged.parent)
        .join("spawn-ledger");
    let mut document: toml::Value = fs::read_to_string(&ledger)
        .expect("read generated ledger")
        .parse()
        .expect("generated ledger parses");
    let entries = document["children"]
        .as_array_mut()
        .expect("ledger children");
    let entry = entries
        .iter_mut()
        .find(|entry| entry["id"].as_str() == Some(&forged.child))
        .expect("ledger names the addressed child");
    entry["class"] = toml::Value::String("nohold".to_owned());
    fs::write(
        &ledger,
        toml::to_string(&document).expect("serialize forged fixture"),
    )
    .expect("replace only fixture ownership context");
    let before = snapshot(forged.root());
    let refusal = apply_hold(forged.root(), &plan)
        .expect_err("apply must not borrow the parent's route after ownership changes");
    assert!(refusal.reason.contains("route"), "{refusal}");
    assert_eq!(
        snapshot(forged.root()),
        before,
        "rejected forged context wrote files"
    );

    let no_route = Fixture::new("parent-route-is-not-child-route", true, false, false);
    no_route.successful_hold(&no_route.parent, "plan");
    no_route.unchanged_refusal(
        &no_route.request(&no_route.child, BLOCKER),
        "no blocked route",
    );
}

#[test]
fn wrsv_002_04() {
    let fixture = Fixture::new("bound-child-workspace", false, true, true);
    assert_ne!(fixture.workspace, fixture.root());
    // The primary-only blocker exists, but not in the child's recorded workspace.
    fixture.unchanged_refusal(
        &fixture.request(&fixture.child, "work/primary-only.txt"),
        "does not resolve",
    );
    fixture.unchanged_refusal(
        &fixture.request(&fixture.child, "../work/blocker.txt"),
        "must stay",
    );
    fixture.repo.write(
        "child-area/outside.txt",
        "Existing outside declared root.\n",
    );
    fixture.unchanged_refusal(
        &fixture.request(&fixture.child, "outside.txt"),
        "outside every",
    );
    fixture.successful_hold(&fixture.child, "prep");
    // The sibling still completes normally and cannot subsequently be held.
    step(fixture.root(), &fixture.sibling);
    assert_eq!(fixture.record(&fixture.sibling).status, Status::Passed);
    fixture.unchanged_refusal(&fixture.request(&fixture.sibling, BLOCKER), "terminal");
}
