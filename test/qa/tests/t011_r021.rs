use std::str::FromStr;

use ratmac::graph::State;
use ratmac::model::{Run, Runs, Status};

fn fixture_phase(path: &str) -> String {
    let source = std::fs::read_to_string(path).expect("run fixture exists");
    let value: toml::Value = source.parse().expect("run fixture is valid TOML");
    value
        .get("phase")
        .and_then(toml::Value::as_str)
        .expect("run fixture has a phase")
        .to_owned()
}

#[test]
fn r021_model_represents_multiple_runs() {
    let phase_a = fixture_phase(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/t011-two-runs/run-a.toml"
    ));
    let phase_b = fixture_phase(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/t011-two-runs/run-b.toml"
    ));
    assert_ne!(phase_a, phase_b);

    let run_a = Run::new(
        State::new(phase_a),
        Status::from_str("planned").expect("planned is a valid lifecycle status"),
    );
    let run_b = Run::new(
        State::new(phase_b),
        Status::from_str("planned").expect("planned is a valid lifecycle status"),
    );

    let mut runs = Runs::new();
    runs.push(run_a);
    runs.push(run_b);
    assert_eq!(runs.len(), 2);

    let states: Vec<_> = runs.iter().map(|run| run.phase().as_str()).collect();
    assert_eq!(states, ["prepare", "review"]);
}
