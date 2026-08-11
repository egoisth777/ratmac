use std::fs;
use std::path::{Path, PathBuf};

use ratmac::Scheduler;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/r029-phase-scope")
}

fn copy_fixture(project: &Path) {
    fs::create_dir_all(project.join(".ratmac")).expect("temporary project should be creatable");
    let fixture = fixture_root().join(".ratmac");
    fs::copy(
        fixture.join("ratmac.toml"),
        project.join(".ratmac/ratmac.toml"),
    )
    .expect("phase-scope fixture should be copied");
    // FDC-004: the State File resides in the addressed run's directory.
    let run_dir = project.join(".ratmac/runs/run-001");
    fs::create_dir_all(&run_dir).expect("create run directory");
    fs::copy(fixture.join("run.toml"), run_dir.join("run.toml"))
        .expect("phase-scope fixture should be copied");
}

#[test]
fn phase_prompt_excludes_other_phases_and_graph() {
    let project = std::env::temp_dir().join(format!("ratmac-t025-r029-{}", std::process::id()));
    if project.exists() {
        fs::remove_dir_all(&project).expect("stale phase-scope directory should be removable");
    }
    copy_fixture(&project);

    let scheduler =
        Scheduler::open_run(&project, "run-001").expect("phase-scope fixture should open");
    let report = scheduler
        .status()
        .expect("active state should be reportable");
    let prompt = report.phase_prompt().as_str();

    assert!(
        prompt.contains("Build the selected artifact only."),
        "current phase prose must remain available"
    );
    for guard_kind in ["files_exact", "file_contains", "command_exit"] {
        assert!(
            prompt.contains(guard_kind),
            "current phase guard {guard_kind} must remain available"
        );
    }
    for other_phase in ["build-review", "build-done"] {
        assert!(
            !prompt.contains(other_phase),
            "prompt must not disclose non-selected phase {other_phase}"
        );
    }
    for graph_marker in ["flowchart", "graph", "transitions", "->"] {
        assert!(
            !prompt.contains(graph_marker),
            "prompt must not disclose graph marker {graph_marker}"
        );
    }

    fs::remove_dir_all(project).expect("phase-scope temporary project should be cleaned up");
}
