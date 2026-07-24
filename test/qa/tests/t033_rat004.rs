use std::fs;
use std::path::{Path, PathBuf};

fn qa_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn qa_uses_canonical_ratmac_rtm_identity() {
    let qa = qa_root();
    let source = fs::read_to_string(qa.join("src/lib.rs")).expect("QA helper source must exist");
    assert!(source.contains("use ratmac::"));
    assert!(source.contains("test_only_rtm_writes_state_file"));
    assert!(!source.contains("arca-scheduler"));
    assert!(!source.contains("run_schd"));

    let tests = qa.join("tests");
    for entry in fs::read_dir(&tests).expect("QA tests directory must exist") {
        let path = entry.expect("QA test entry must be readable").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        // These targets intentionally probe rejected legacy spellings and metadata absence.
        if matches!(
            name,
            "t031_rat002.rs" | "t032_rat003.rs" | "t033_rat004.rs" | "t035_rat006.rs"
        ) {
            continue;
        }
        let text = fs::read_to_string(&path).expect("QA test source must be readable");
        assert!(
            !text.contains("arca-scheduler"),
            "stale project identity in {name}"
        );
        assert!(
            !text.contains("run_schd"),
            "stale helper identity in {name}"
        );
        assert!(
            !text.contains("\"schd\""),
            "stale command invocation in {name}"
        );
    }
}
