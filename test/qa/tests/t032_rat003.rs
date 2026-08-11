//! PT-032-01 / HT-032-01..04: canonical executable routing and diagnostics.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const CLI_RATMAC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/cli/.ratmac/ratmac.toml"
));

struct Project {
    root: PathBuf,
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn project() -> Project {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("ratmac-t032-{}-{stamp}", std::process::id()));
    fs::create_dir_all(root.join(".ratmac")).expect("create isolated .ratmac directory");
    fs::write(root.join(".ratmac/ratmac.toml"), CLI_RATMAC).expect("write CLI fixture");
    Project { root }
}

fn rtm(project: &Path, args: &[&str]) -> Output {
    Command::new(ratmac_qa::engine_bin!())
        .args(args)
        .current_dir(project)
        .output()
        .expect("rtm executable must run")
}

#[test]
fn rtm_cli_surface() {
    let project = project();

    let help = rtm(&project.root, &["--help"]);
    assert!(help.status.success(), "rtm --help failed: {help:?}");
    let help_stdout = String::from_utf8_lossy(&help.stdout);
    assert!(
        help_stdout.contains("Usage: rtm"),
        "help must identify rtm: {help_stdout}"
    );
    assert!(
        !help_stdout.contains("schd"),
        "help must not advertise schd: {help_stdout}"
    );

    let start = rtm(&project.root, &["start"]);
    assert!(start.status.success(), "rtm start failed: {start:?}");
    // FDC-004: start mints a run under the plural path; commands address it.
    let run_id = std::fs::read_dir(project.root.join(".ratmac/runs"))
        .expect("list the runs roster")
        .map(|entry| entry.expect("roster entry is readable"))
        .find(|entry| entry.path().is_dir())
        .expect("the started run appears on the roster")
        .file_name()
        .to_string_lossy()
        .into_owned();
    assert!(project
        .root
        .join(".ratmac/runs")
        .join(&run_id)
        .join("run.toml")
        .exists());
    let status = rtm(&project.root, &["status", "--run", &run_id]);
    assert!(status.status.success(), "rtm status failed: {status:?}");
    let status_stdout = String::from_utf8_lossy(&status.stdout);
    assert!(status_stdout.contains("State: prepare"));
    assert!(status_stdout.contains("Prepare the run."));

    let refused = rtm(&project.root, &["step", "--run", &run_id]);
    assert!(
        refused.status.success(),
        "guard refusal should be reported: {refused:?}"
    );
    let refused_stdout = String::from_utf8_lossy(&refused.stdout);
    assert!(
        refused_stdout.contains("rtm: step refused"),
        "refusal must identify rtm: {refused_stdout}"
    );
    assert!(!refused_stdout.contains("schd"));

    let invalid = rtm(&project.root, &["unknown"]);
    assert!(!invalid.status.success(), "unknown command must fail");
    let invalid_stderr = String::from_utf8_lossy(&invalid.stderr);
    assert!(
        invalid_stderr.starts_with("rtm: "),
        "error must identify rtm: {invalid_stderr}"
    );
    assert!(!invalid_stderr.contains("schd"));

    let legacy = rtm(&project.root, &["schd", "status"]);
    assert!(
        !legacy.status.success(),
        "legacy command spelling must be rejected"
    );
    let legacy_stderr = String::from_utf8_lossy(&legacy.stderr);
    assert!(
        legacy_stderr.starts_with("rtm: "),
        "legacy rejection must identify rtm: {legacy_stderr}"
    );
    assert!(!legacy_stderr.contains("schd"));
}
