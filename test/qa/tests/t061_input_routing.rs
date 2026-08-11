//! t-061 / FDC-001: transition input is the only branch selector.

use std::fs;

use ratmac::graph::MachineGraph;
use ratmac::machine::MachineClass;
use ratmac::{Scheduler, StepRequest};

fn parse(source: &str) -> MachineClass {
    MachineClass::from_toml(source).unwrap_or_else(|error| {
        panic!(
            "valid input-routed Machine Class refused as {} at {}: {error}",
            error.code(),
            error.location()
        )
    })
}

fn code(source: &str) -> &'static str {
    MachineClass::from_toml(source)
        .expect_err("malformed input contract must refuse")
        .code()
}

fn branch(states: &str, transitions: &str) -> String {
    format!(
        "[states.review]\nprompt = \"Review.\"\n{states}\n\
         [states.approve]\nprompt = \"Approve.\"\n\n\
         [states.rework]\nprompt = \"Rework.\"\n\n\
         {transitions}"
    )
}

#[test]
fn typed_input_contract_is_retained() {
    let source = branch(
        "inputs = [\"approve\", \"rework\"]",
        "[[transitions]]\nfrom = \"review\"\nto = \"approve\"\ninput = \"approve\"\n\n\
         [[transitions]]\nfrom = \"review\"\nto = \"rework\"\ninput = \"rework\"\n",
    );
    let class = parse(&source);
    assert_eq!(
        class.states()["review"].inputs(),
        Some(&["approve".to_owned(), "rework".to_owned()][..])
    );
    assert_eq!(
        class
            .transitions()
            .iter()
            .map(|edge| edge.input())
            .collect::<Vec<_>>(),
        vec![Some("approve"), Some("rework")]
    );

    let straight = parse(
        "[states.a]\nprompt = \"A.\"\n[states.b]\nprompt = \"B.\"\n\n\
         [[transitions]]\nfrom = \"a\"\nto = \"b\"\n",
    );
    assert_eq!(straight.states()["a"].inputs(), None);
    assert_eq!(straight.transitions()[0].input(), None);
}

#[test]
fn invalid_input_contracts_refuse_with_stable_codes() {
    let empty = branch(
        "inputs = []",
        "[[transitions]]\nfrom = \"review\"\nto = \"approve\"\n",
    );
    assert_eq!(code(&empty), "RB208");

    let duplicate_list = branch(
        "inputs = [\"approve\", \"approve\"]",
        "[[transitions]]\nfrom = \"review\"\nto = \"approve\"\ninput = \"approve\"\n\n\
         [[transitions]]\nfrom = \"review\"\nto = \"rework\"\ninput = \"approve\"\n",
    );
    assert_eq!(code(&duplicate_list), "RB208");

    let no_list = branch(
        "",
        "[[transitions]]\nfrom = \"review\"\nto = \"approve\"\n\n\
         [[transitions]]\nfrom = \"review\"\nto = \"rework\"\n",
    );
    assert_eq!(code(&no_list), "RB209");
    let uncovered_value = branch(
        "inputs = [\"approve\", \"rework\", \"escalate\"]",
        "[[transitions]]\nfrom = \"review\"\nto = \"approve\"\ninput = \"approve\"\n\n\
         [[transitions]]\nfrom = \"review\"\nto = \"rework\"\ninput = \"rework\"\n",
    );
    assert_eq!(code(&uncovered_value), "RB210");

    let missing_coverage = branch(
        "inputs = [\"approve\", \"rework\"]",
        "[[transitions]]\nfrom = \"review\"\nto = \"approve\"\ninput = \"approve\"\n\n\
         [[transitions]]\nfrom = \"review\"\nto = \"rework\"\ninput = \"revise\"\n",
    );
    assert_eq!(code(&missing_coverage), "RB212");

    let duplicate_coverage = branch(
        "inputs = [\"approve\", \"rework\"]",
        "[[transitions]]\nfrom = \"review\"\nto = \"approve\"\ninput = \"approve\"\n\n\
         [[transitions]]\nfrom = \"review\"\nto = \"rework\"\ninput = \"approve\"\n",
    );
    assert_eq!(code(&duplicate_coverage), "RB211");

    let mixed = branch(
        "inputs = [\"approve\", \"rework\"]",
        "[[transitions]]\nfrom = \"review\"\nto = \"approve\"\ninput = \"approve\"\n\n\
         [[transitions]]\nfrom = \"review\"\nto = \"rework\"\n",
    );
    assert_eq!(code(&mixed), "RB212");

    let blocked_label = branch(
        "inputs = [\"approve\", \"rework\"]",
        "[[transitions]]\nfrom = \"review\"\nto = \"approve\"\ninput = \"approve\"\n\n\
         [[transitions]]\nfrom = \"review\"\nto = \"rework\"\ninput = \"rework\"\n\n\
         [[transitions]]\nfrom = \"review\"\nto = \"rework\"\nblocked-route = true\ninput = \"hold\"\n",
    );
    assert_eq!(code(&blocked_label), "RB213");
}

#[test]
fn selection_is_input_only_and_prompt_discloses_values() {
    let source = branch(
        "inputs = [\"approve\", \"rework\"]",
        "[[transitions]]\nfrom = \"review\"\nto = \"rework\"\ninput = \"rework\"\n\n\
         [[transitions]]\nfrom = \"review\"\nto = \"approve\"\ninput = \"approve\"\n",
    );
    let class = parse(&source);
    let graph = MachineGraph::new(class.states().keys().cloned(), class.transitions().to_vec());
    assert_eq!(
        graph
            .transition_for_input("review", Some("approve"))
            .expect("approve route")
            .to()
            .as_str(),
        "approve"
    );
    assert_eq!(
        graph
            .transition_for_input("review", Some("rework"))
            .expect("rework route")
            .to()
            .as_str(),
        "rework"
    );
    assert!(graph.transition_for_input("review", None).is_none());
    assert!(graph
        .transition_for_input("review", Some("foreign"))
        .is_none());

    let root = std::env::temp_dir().join(format!(
        "ratmac-t061-prompt-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".ratmac")).expect("create prompt project");
    fs::write(root.join(".ratmac/ratmac.toml"), &source).expect("write branching machine class");
    let mut scheduler = Scheduler::open(&root).expect("open branching project");
    scheduler.start().expect("start branching Run");
    let rendered = scheduler
        .status()
        .expect("read branching status")
        .state_prompt()
        .to_string();
    assert!(rendered.starts_with("Review."));
    assert!(rendered.contains("Legal transition inputs:\n- approve\n- rework"));
    assert!(!rendered.contains("Approve."));
    assert!(!rendered.contains("Rework."));
    assert!(!rendered.contains("to ="));

    let straight_root = std::env::temp_dir().join(format!(
        "ratmac-t061-straight-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = fs::remove_dir_all(&straight_root);
    fs::create_dir_all(straight_root.join(".ratmac")).expect("create straight project");
    fs::write(
        straight_root.join(".ratmac/ratmac.toml"),
        "[states.a]\nprompt = \"A.\"\n[states.b]\nprompt = \"B.\"\n\
         [[transitions]]\nfrom = \"a\"\nto = \"b\"\n",
    )
    .expect("write straight machine class");
    let mut straight_scheduler = Scheduler::open(&straight_root).expect("open straight project");
    straight_scheduler.start().expect("start straight Run");
    let outcome = straight_scheduler
        .step(StepRequest::new("ready"))
        .expect("step straight line");
    assert!(
        format!("{outcome}").contains("advanced: a -> b"),
        "straight-line step must need no transition input: {outcome}"
    );
    fs::remove_dir_all(straight_root).expect("remove straight project");
    fs::remove_dir_all(root).expect("remove prompt project");
}
