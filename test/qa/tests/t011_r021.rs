use std::str::FromStr;

use ratmac::graph::State;
use ratmac::model::{Run, Runs, Status};

fn fixture_state(path: &str) -> String {
    let source = std::fs::read_to_string(path).expect("run fixture exists");
    let value: toml::Value = source.parse().expect("run fixture is valid TOML");
    value
        .get("state")
        .and_then(toml::Value::as_str)
        .expect("run fixture has a state")
        .to_owned()
}

#[test]
fn r021_model_represents_multiple_runs() {
    let state_a = fixture_state(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/t011-two-runs/run-a.toml"
    ));
    let state_b = fixture_state(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/t011-two-runs/run-b.toml"
    ));
    assert_ne!(state_a, state_b);

    let run_a = Run::new(
        State::new(state_a),
        Status::from_str("planned").expect("planned is a valid lifecycle status"),
    );
    let run_b = Run::new(
        State::new(state_b),
        Status::from_str("planned").expect("planned is a valid lifecycle status"),
    );

    let mut runs = Runs::new();
    runs.push(run_a);
    runs.push(run_b);
    assert_eq!(runs.len(), 2);

    let states: Vec<_> = runs.iter().map(|run| run.state().as_str()).collect();
    assert_eq!(states, ["prepare", "review"]);
}
