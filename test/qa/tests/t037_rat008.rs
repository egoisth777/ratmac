//! PT-037-01: complete RAT-008 acceptance proof for the ratmac/rtm cutover.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const LEGACY_PRODUCT: &str = concat!("arca", "-scheduler");
const LEGACY_COMMAND: &str = concat!("sc", "hd");

fn repo_root() -> PathBuf {
    std::env::var_os("RATMAC_ACCEPTANCE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .canonicalize()
                .expect("workspace root must resolve")
        })
}

fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rtm"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("rtm executable must run")
}

fn temporary_project(root: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_nanos();
    let project = std::env::temp_dir().join(format!("ratmac-t037-{}-{stamp}", std::process::id()));
    fs::create_dir_all(project.join(".ratmac")).expect("create isolated .ratmac directory");
    fs::copy(
        root.join("test/qa/fixtures/rebrand-smoke/.ratmac/ratmac.toml"),
        project.join(".ratmac/ratmac.toml"),
    )
    .expect("copy isolated Machine Class fixture");
    project
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if matches!(name, ".git" | ".arca-private" | "target") {
        return;
    }
    let metadata = fs::symlink_metadata(path).expect("audit path must be readable");
    if metadata.is_dir() {
        for entry in fs::read_dir(path).expect("audit directory must be readable") {
            collect_files(&entry.expect("audit entry must be readable").path(), files);
        }
    } else if metadata.is_file() {
        files.push(path.to_owned());
    }
}

fn path_matches(pattern: &str, relative: &str) -> bool {
    pattern
        .strip_suffix("/**")
        .map_or(pattern == relative, |prefix| {
            relative == prefix || relative.starts_with(&format!("{prefix}/"))
        })
}

fn active_reference_audit(root: &Path) {
    let allowlist = fs::read_to_string(root.join("test/qa/fixtures/rebrand-audit/allowlist.tsv"))
        .expect("rebrand audit allowlist must be readable");
    let rules: Vec<_> = allowlist
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|line| {
            let fields: Vec<_> = line.splitn(3, '\t').collect();
            assert_eq!(
                fields.len(),
                3,
                "each allowlist row needs path, token, reason"
            );
            assert!(
                !fields[2].trim().is_empty(),
                "allowlist reasons must be explicit"
            );
            (fields[0], fields[1], fields[2])
        })
        .collect();
    assert!(!rules.is_empty(), "allowlist must contain active rules");
    let mut used = vec![false; rules.len()];
    let mut files = Vec::new();
    collect_files(root, &mut files);
    let mut violations = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(root)
            .expect("audit file must be under workspace root")
            .to_string_lossy()
            .replace('\\', "/");
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (line_no, line) in text.lines().enumerate() {
            let product = line.contains(LEGACY_PRODUCT);
            let command = line.contains(LEGACY_COMMAND);
            if !product && !command {
                continue;
            }
            let mut allowed = false;
            for (index, (pattern, token, _)) in rules.iter().enumerate() {
                if !path_matches(pattern, &relative) {
                    continue;
                }
                let token_matches = *token == "both"
                    || (*token == LEGACY_PRODUCT && product)
                    || (*token == LEGACY_COMMAND && command);
                if token_matches {
                    used[index] = true;
                    allowed = true;
                }
            }
            if !allowed {
                violations.push(format!("{relative}:{}: {line}", line_no + 1));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "unallowlisted active legacy references:\n{}",
        violations.join("\n")
    );
    let unused: Vec<_> = rules
        .iter()
        .enumerate()
        .filter(|(index, _)| !used[*index])
        .map(|(_, (_, _, reason))| (*reason).to_owned())
        .collect();
    assert!(
        unused.is_empty(),
        "stale allowlist entries: {}",
        unused.join("; ")
    );
    assert!(
        root.join(".arca/log.md").is_file(),
        "transition log must remain present"
    );
    assert!(
        root.join(".arca/ticket/archive").is_dir(),
        "archived ticket history must remain present"
    );
}

fn metadata_and_paths(root: &Path) {
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--locked",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .output()
        .expect("cargo metadata must run");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata = String::from_utf8(output.stdout).expect("metadata must be UTF-8");
    assert!(metadata.contains(r#""name":"ratmac""#));
    assert!(metadata.contains(r#""name":"ratmac-qa""#));
    assert!(metadata.contains("src/bin/rtm.rs") || metadata.contains(r#"src\\bin\\rtm.rs"#));
    assert!(!metadata.contains(&format!(r#""name":"{}""#, LEGACY_PRODUCT)));
    assert!(!metadata.contains(&format!(r#""name":"{}""#, LEGACY_COMMAND)));
    assert!(root.join("src/bin/rtm.rs").is_file());
    assert!(!root.join("src/bin/schd.rs").exists());

    let lock = fs::read_to_string(root.join("Cargo.lock")).expect("Cargo.lock must be readable");
    assert!(lock.starts_with("# This file is automatically @generated by Cargo."));
    assert!(lock.contains("name = \"ratmac\""));
    assert!(lock.contains("name = \"ratmac-qa\""));
    assert!(!lock.contains(&format!("name = \"{LEGACY_PRODUCT}\"")));
    assert!(!lock.contains(&format!("name = \"{LEGACY_COMMAND}\"")));
    let ignore = fs::read_to_string(root.join(".gitignore")).expect(".gitignore must be readable");
    assert!(ignore.lines().any(|line| line.trim() == "target"));
}

#[test]
fn full_rebrand_acceptance() {
    let root = repo_root();
    active_reference_audit(&root);
    metadata_and_paths(&root);

    let project = temporary_project(&root);
    let engine = project.join(".ratmac");
    let help = run(&project, &["--help"]);
    assert!(help.status.success(), "rtm --help failed: {help:?}");
    let help_stdout = String::from_utf8_lossy(&help.stdout);
    assert!(help_stdout.contains("Usage: rtm"));
    assert!(!help_stdout.contains(LEGACY_COMMAND));

    let start = run(&project, &["start"]);
    assert!(start.status.success(), "rtm start failed: {start:?}");
    // FDC-004: the started run's State File resides under the plural path.
    let run_id = fs::read_dir(engine.join("runs"))
        .expect("list the runs roster")
        .map(|entry| entry.expect("roster entry is readable"))
        .find(|entry| entry.path().is_dir())
        .expect("the started run appears on the roster")
        .file_name()
        .to_string_lossy()
        .into_owned();
    let state_path = engine.join("runs").join(&run_id).join("run.toml");
    let class_before = fs::read(engine.join("ratmac.toml")).expect("read Machine Class");
    let state_before = fs::read(&state_path).expect("read state");
    let log_before = fs::read(engine.join("log.md")).expect("read transition log");
    let status = run(&project, &["status", "--run", &run_id]);
    assert!(status.status.success(), "rtm status failed: {status:?}");
    assert!(String::from_utf8_lossy(&status.stdout).contains("Phase: prepare"));

    let legacy_lock = engine.join("schd.lock");
    fs::write(&legacy_lock, b"operator-held legacy lock\n").expect("write legacy-lock fixture");
    let commands: [&[&str]; 3] = [
        &["status", "--run", run_id.as_str()],
        &["step", "--run", run_id.as_str()],
        &["start"],
    ];
    for args in commands {
        let refused = run(&project, args);
        assert!(
            !refused.status.success(),
            "legacy lock must refuse {args:?}"
        );
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert!(stderr.contains("refusing to run"));
        assert!(stderr.contains(LEGACY_COMMAND));
        assert!(stderr.contains("migrate or remove"));
    }
    assert!(legacy_lock.is_file());
    assert!(!engine.join("locks/root.lock").exists());
    assert_eq!(class_before, fs::read(engine.join("ratmac.toml")).unwrap());
    assert_eq!(state_before, fs::read(&state_path).unwrap());
    assert_eq!(log_before, fs::read(engine.join("log.md")).unwrap());
    fs::remove_file(&legacy_lock).expect("remove isolated legacy-lock fixture");

    let legacy = run(&project, &[LEGACY_COMMAND, "status"]);
    assert!(!legacy.status.success());
    assert!(String::from_utf8_lossy(&legacy.stderr).starts_with("rtm: "));
    assert!(!String::from_utf8_lossy(&legacy.stderr).contains(LEGACY_COMMAND));

    let refused = run(&project, &["step", "--run", &run_id]);
    assert!(
        refused.status.success(),
        "guard refusal is a reported result"
    );
    assert!(String::from_utf8_lossy(&refused.stdout).contains("rtm: step refused"));
    fs::create_dir(project.join("artifacts")).expect("create guard directory");
    fs::write(project.join("artifacts/required.txt"), b"ready\n").expect("write guard artifact");
    let stepped = run(&project, &["step", "--run", &run_id]);
    assert!(stepped.status.success(), "passing step failed: {stepped:?}");
    let final_state = fs::read_to_string(&state_path).expect("read final state");
    assert!(final_state.contains("state = \"review\""));
    let final_log = fs::read_to_string(engine.join("log.md")).expect("read final log");
    assert_eq!(
        final_log.matches("- Transition: prepare -> review").count(),
        1
    );
    assert_eq!(class_before, fs::read(engine.join("ratmac.toml")).unwrap());
    assert!(!engine.join("locks/root.lock").exists());
    let invalid = run(&project, &["unknown"]);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).starts_with("rtm: "));
    let _ = fs::remove_dir_all(project);
}
