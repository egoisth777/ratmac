//! PT-036-01 / HT-036-01..06: clean-cutover lock policy and compatibility behavior.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

const CLASS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/lock-migration/.arca/ratmac.toml"
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
    let root = std::env::temp_dir().join(format!("ratmac-t036-{}-{stamp}", std::process::id()));
    fs::create_dir_all(root.join(".arca")).expect("create isolated .arca directory");
    fs::write(root.join(".arca/ratmac.toml"), CLASS).expect("write Machine Class fixture");
    Project { root }
}

fn rtm(project: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rtm"))
        .args(args)
        .current_dir(project)
        .output()
        .expect("rtm executable must run")
}

#[test]
fn lock_and_compatibility_policy() {
    let project = Arc::new(project());
    let started = rtm(&project.root, &["start"]);
    assert!(started.status.success(), "rtm start failed: {started:?}");

    let arca = project.root.join(".arca");
    let class_before = fs::read(arca.join("ratmac.toml")).expect("read class before refusal");
    let state_before = fs::read(arca.join("state.toml")).expect("read state before refusal");
    let log_before = fs::read(arca.join("log.md")).expect("read log before refusal");

    // HT-036-02/04/05: legacy state is never deleted, bypassed, or mutated.
    let legacy_lock = arca.join("schd.lock");
    fs::write(&legacy_lock, b"operator-held legacy lock\n").expect("create legacy lock");
    for command in [["status"], ["step"], ["start"]] {
        let refused = rtm(&project.root, &command);
        assert!(
            !refused.status.success(),
            "legacy lock must refuse {command:?}"
        );
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert!(
            stderr.contains("refusing to run"),
            "refusal must be explicit: {stderr}"
        );
        assert!(
            stderr.contains("schd.lock"),
            "refusal must name observed lock: {stderr}"
        );
        assert!(
            stderr.contains("migrate or remove"),
            "refusal must guide migration: {stderr}"
        );
        assert!(legacy_lock.is_file(), "legacy lock must remain untouched");
    }
    assert_eq!(class_before, fs::read(arca.join("ratmac.toml")).unwrap());
    assert_eq!(state_before, fs::read(arca.join("state.toml")).unwrap());
    assert_eq!(log_before, fs::read(arca.join("log.md")).unwrap());
    assert!(
        !arca.join("rtm.lock").exists(),
        "refusal must release no new lock"
    );

    // HT-036-02: clean cutover has no executable schd alias.
    fs::remove_file(&legacy_lock).expect("remove fixture legacy lock after refusal checks");
    let legacy_command = rtm(&project.root, &["schd", "status"]);
    assert!(
        !legacy_command.status.success(),
        "schd command must be rejected"
    );
    let legacy_stderr = String::from_utf8_lossy(&legacy_command.stderr);
    assert!(legacy_stderr.contains("unsupported command"));
    assert!(!legacy_stderr.contains("schd.lock"));

    // HT-036-01/03/06: two real invocations arbitrate one transition.
    fs::create_dir(project.root.join("artifacts")).expect("create guard directory");
    fs::write(project.root.join("artifacts/required.txt"), b"ready\n")
        .expect("write guard artifact");
    let first_root = Arc::clone(&project);
    let first = thread::spawn(move || rtm(&first_root.root, &["step"]));
    let second_root = Arc::clone(&project);
    let second = thread::spawn(move || rtm(&second_root.root, &["step"]));
    let outputs = [
        first.join().expect("first invocation must finish"),
        second.join().expect("second invocation must finish"),
    ];
    assert!(
        outputs.iter().all(|output| output.status.success()),
        "lock arbitration should produce deterministic CLI outcomes: {outputs:?}"
    );
    let state = fs::read_to_string(arca.join("state.toml")).expect("read final state");
    assert!(
        state.contains("phase = \"review\""),
        "exactly one transition must advance the Run: {state}"
    );
    let log = fs::read_to_string(arca.join("log.md")).expect("read transition log");
    assert_eq!(log.matches("- Transition: prepare -> review").count(), 1);
    assert!(
        !arca.join("rtm.lock").exists(),
        "transient canonical lock must be released"
    );
}
