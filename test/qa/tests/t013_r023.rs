use ratmac::cli::run_from;
use std::fs;
use std::path::{Path, PathBuf};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/t013-active-run-cli")
}

fn copy_fixture(project: &Path) {
    fs::create_dir_all(project.join(".arca")).expect("temporary project should be creatable");
    let fixture = fixture_root().join(".arca");
    for name in ["ratmac.toml", "state.toml", "log.md"] {
        fs::copy(fixture.join(name), project.join(".arca").join(name))
            .expect("T-09 fixture file should be copied");
    }
}

fn run_schd(project: &Path, args: &[&str]) -> bool {
    let mut argv = vec!["schd"];
    argv.extend_from_slice(args);
    let mut sink = Vec::new();
    run_from(argv, project, &mut sink).is_ok()
}

fn state_phase(path: &Path) -> String {
    let value: toml::Value = fs::read_to_string(path)
        .expect("active state should be readable")
        .parse()
        .expect("active state fixture should be valid TOML");
    value["phase"]
        .as_str()
        .expect("active state must contain a phase")
        .to_owned()
}

#[test]
fn step_and_status_target_active_run_without_run_id() {
    let project =
        std::env::temp_dir().join(format!("arca-scheduler-t013-r023-{}", std::process::id()));
    if project.exists() {
        fs::remove_dir_all(&project).expect("stale T-09 directory should be removable");
    }
    copy_fixture(&project);

    let state_path = project.join(".arca/state.toml");
    let before_status = fs::read(&state_path).expect("active state should be readable");
    let status = run_schd(&project, &["status"]);
    assert!(status, "status without run-id should target active Run");
    assert_eq!(
        fs::read(&state_path).expect("active state should remain readable"),
        before_status,
        "status must remain read-only"
    );

    let step = run_schd(&project, &["step"]);
    assert!(step, "step without run-id should target active Run");
    assert_eq!(
        state_phase(&state_path),
        "done",
        "no-run-id step must advance the canonical active Run"
    );

    let after_step = fs::read(&state_path).expect("active state should remain readable");
    for args in [["status", "stale"], ["step", "stale"]] {
        let rejected = run_schd(&project, &args);
        assert!(!rejected, "{} with a run-id must be rejected", args[0]);
        assert_eq!(
            fs::read(&state_path).expect("active state should remain readable"),
            after_step,
            "rejected run-id form must not retarget or mutate the active Run"
        );
    }

    fs::remove_dir_all(project).expect("T-09 temporary project should be cleaned up");
}
