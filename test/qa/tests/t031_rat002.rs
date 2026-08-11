use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("QA crate must live below the workspace root")
        .to_path_buf()
}

fn metadata(root: &Path) -> String {
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .output()
        .expect("cargo metadata must be executable");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("cargo metadata must be UTF-8")
}

#[test]
fn canonical_package_and_binary_metadata() {
    let root = workspace_root();
    let metadata = metadata(&root);

    assert!(metadata.contains(r#""name":"ratmac""#));
    assert!(metadata.contains(r#""name":"ratmac","#));
    assert!(metadata.contains("src/bin/rtm.rs") || metadata.contains(r#"src\\bin\\rtm.rs"#));
    assert!(!metadata.contains("src/bin/schd.rs") && !metadata.contains(r#"src\\bin\\schd.rs"#));
    assert!(!metadata.contains(r#""name":"arca-scheduler""#));
    assert!(!metadata.contains(r#""name":"schd""#));

    let qa_manifest = std::fs::read_to_string(root.join("test/qa/Cargo.toml"))
        .expect("QA manifest must be readable");
    assert!(qa_manifest.contains("name = \"ratmac-qa\""));
    // DEB-001: the harness builds the canonical Engine source under its own
    // target name, so it never writes over the shipped `rtm`.
    assert!(qa_manifest.contains("name = \"rtm-qa\""));
    assert!(!qa_manifest.contains("name = \"rtm\""));
    assert!(qa_manifest.contains("path = \"../../src/bin/rtm.rs\""));
    assert!(!qa_manifest.contains("arca-scheduler"));
    assert!(!qa_manifest.contains("schd"));
    let parsed: toml::Value = qa_manifest.parse().expect("QA manifest must be valid TOML");
    assert_eq!(
        parsed["dependencies"]["ratmac"]["path"].as_str(),
        Some("../.."),
        "the QA crate must depend on the canonical root package"
    );
}
