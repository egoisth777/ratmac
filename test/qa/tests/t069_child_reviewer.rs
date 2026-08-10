//! t-069 / FDC-010: judge independence lands child-as-reviewer first.
//!
//! PT-069-01 `child_reviews_and_parent_routes_without_courier`
//! PT-069-02 `no_witnessed_verdict_verb_exists`

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A composed machine whose parent `delegate` State both spawns the reviewer
/// and branches on the reviewer's transition input, behind the join.
const REVIEWER_RUNBOOK: &str = r#"
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
inputs = ["approve", "rework"]
guards = [{ kind = "join", require = "all_passed", min = 1 }]

[[states.delegate.spawns]]
class = "reviewer"
name = "rev"
bind = ["ticket"]

[states.done]
prompt = "Done."

[states.rework]
prompt = "Rework."

[[transitions]]
from = "plan"
to = "delegate"

[[transitions]]
from = "delegate"
to = "done"
input = "approve"

[[transitions]]
from = "delegate"
to = "rework"
input = "rework"
"#;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn create(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ratmac-t069-{label}-{}-{}",
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
        fs::write(root.join(".arca/goal/spec.md"), "# Fixture goal\n").expect("write fixture goal");
        fs::write(root.join(".ratmac/ratmac.toml"), REVIEWER_RUNBOOK)
            .expect("write fixture machine class");
        fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").expect("write fixture source");
        Self { root }
    }

    fn rtm(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rtm"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .expect("invoke rtm")
    }

    fn start(&self) -> String {
        let output = self.rtm(&["start"]);
        let text = combined(&output);
        assert!(output.status.success(), "start succeeds: {text}");
        text.split("started run ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .expect("start names the minted run id")
            .to_owned()
    }

    fn step(&self, id: &str) -> Output {
        self.rtm(&["step", "--run", id])
    }

    fn run_dir(&self, id: &str) -> PathBuf {
        self.root.join(".ratmac/runs").join(id)
    }

    fn state_text(&self, id: &str) -> String {
        fs::read_to_string(self.run_dir(id).join("state.toml")).expect("the State File is readable")
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

/// The sorted file names inside a directory, empty when it is absent.
fn names_in(directory: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// PT-069-01: the child Run performs the review, its Engine writes the
/// durable terminal fact, and the child-authored verdict routes the parent's
/// branch as ordinary transition-input delivery - no human courier between
/// machines, no human-authored byte in the delivery path.
#[test]
fn child_reviews_and_parent_routes_without_courier() {
    let fixture = Fixture::create("courier");
    let parent = fixture.start();
    let step = fixture.step(&parent);
    assert!(
        step.status.success(),
        "step into delegate: {}",
        combined(&step)
    );

    let spawn = fixture.rtm(&["spawn", "rev", "--run", &parent, "--bind", "ticket=t-201"]);
    let spawn_text = combined(&spawn);
    assert!(spawn.status.success(), "spawn succeeds: {spawn_text}");
    let child = spawn_text
        .split("spawned run ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .expect("spawn names the child run id")
        .to_owned();

    // Inside the child's turn: the reviewer authors the parent's transition
    // input, then finishes its own Run. These bytes are the child's - the
    // test stands in for the child machine's agent, not for a human.
    let verdict = format!(
        "phase = \"delegate\"\ninput = \"approve\"\nrationale = \"reviewer {child} approves ticket t-201\"\n"
    );
    fs::write(fixture.run_dir(&parent).join("verdict.toml"), &verdict)
        .expect("the child authors the parent's live verdict");
    let finish = fixture.step(&child);
    assert!(
        finish.status.success(),
        "the child's terminal step: {}",
        combined(&finish)
    );
    let child_state = fixture.state_text(&child);
    assert!(
        child_state.contains("phase = \"approved\"") && child_state.contains("\"passed\""),
        "the child's Engine wrote the durable terminal fact: {child_state}"
    );

    // The parent routes on the child-authored verdict: archive, then advance.
    let route = fixture.step(&parent);
    let route_text = combined(&route);
    assert!(route.status.success(), "the parent routes: {route_text}");
    let parent_state = fixture.state_text(&parent);
    assert!(
        parent_state.contains("phase = \"done\""),
        "the approve input routed delegate -> done: {parent_state}"
    );

    assert!(
        !fixture.run_dir(&parent).join("verdict.toml").exists(),
        "the live slot is consumed"
    );
    let archive = fixture.run_dir(&parent).join("verdicts");
    let archived = names_in(&archive);
    assert_eq!(
        archived.len(),
        1,
        "exactly one archived verdict: {archived:?}"
    );
    let archived_bytes =
        fs::read(archive.join(&archived[0])).expect("the archived verdict is readable");
    assert_eq!(
        archived_bytes,
        verdict.as_bytes(),
        "the archive holds the child's exact bytes"
    );
}

/// PT-069-02: no verb signs or witnesses a verdict - the unknown verbs
/// refuse, the surface lists nothing of the kind, and the deferral is
/// recorded in the goal's glossary and spec rather than silently absent.
#[test]
fn no_witnessed_verdict_verb_exists() {
    let fixture = Fixture::create("deferral");

    for verb in ["witness", "sign"] {
        let refused = fixture.rtm(&[verb]);
        let text = combined(&refused);
        assert!(
            !refused.status.success(),
            "rtm {verb} must refuse, got: {text}"
        );
        assert!(
            text.contains("unsupported command") || text.contains("Usage"),
            "the unknown verb refuses through the ordinary surface: {text}"
        );
        assert!(
            !text.contains("verdict is witnessed") && !text.contains("signed"),
            "nothing witnesses or signs: {text}"
        );
    }

    let usage = combined(&fixture.rtm(&[]));
    assert!(
        !usage.contains("witness") && !usage.contains("sign"),
        "the surface lists no witnessed-verdict verb: {usage}"
    );

    let goal = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.arca/goal");
    let glossary =
        fs::read_to_string(goal.join("ubi-lang.md")).expect("the goal glossary is readable");
    assert!(
        glossary.contains("Witnessed verdict verb") && glossary.contains("deferral is recorded"),
        "the glossary records the deferral"
    );
    let spec = fs::read_to_string(goal.join("spec.md")).expect("the goal spec is readable");
    assert!(
        spec.contains("witnessed verdict verb remains deferred"),
        "the spec records the deferral"
    );
}
