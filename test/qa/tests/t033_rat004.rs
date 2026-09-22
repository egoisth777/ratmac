use std::path::{Path, PathBuf};

fn qa_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn qa_uses_canonical_ratmac_rtm_identity() {
    let qa = qa_root();
    let repository = qa.join("../..");
    let selected =
        ratmac_qa::audit_files::select(&repository, &[]).expect("select indexed QA sources");
    let source = std::str::from_utf8(
        &selected
            .entries
            .iter()
            .find(|entry| entry.path == Path::new("test/qa/src/lib.rs"))
            .expect("QA helper source is indexed")
            .bytes,
    )
    .expect("QA helper source is text");
    assert!(source.contains("use ratmac::"));
    assert!(source.contains("test_only_rtm_writes_state_file"));
    assert!(!source.contains("arca-scheduler"));
    assert!(!source.contains("run_schd"));

    for entry in selected.entries {
        let path = &entry.path;
        if path.parent() != Some(Path::new("test/qa/tests")) {
            continue;
        }
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
            "t031_rat002.rs"
                | "t032_rat003.rs"
                | "t033_rat004.rs"
                | "t034_rat005.rs"
                | "t035_rat006.rs"
        ) {
            continue;
        }
        let text = std::str::from_utf8(&entry.bytes).expect("QA test source must be text");
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
