use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use ratmac::Scheduler;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/r012-toml-comments")
}

fn copy_fixture_to_temp() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is after the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("ratmac-t027-{nonce}"));
    let engine = root.join(".ratmac");
    fs::create_dir_all(&engine).expect("create isolated .ratmac directory");
    for file in ["ratmac.toml", "log.md"] {
        fs::copy(fixture_root().join(".ratmac").join(file), engine.join(file))
            .expect("copy comment fixture");
    }
    // FDC-004: the State File resides in the addressed run's directory.
    let run_dir = engine.join("runs/run-001");
    fs::create_dir_all(&run_dir).expect("create run directory");
    fs::copy(
        fixture_root().join(".ratmac/run.toml"),
        run_dir.join("run.toml"),
    )
    .expect("copy comment fixture");
    root
}

#[test]
fn ratmac_comments_are_absent_from_state_prompt() -> Result<(), Box<dyn Error>> {
    let root = copy_fixture_to_temp();
    let scheduler = Scheduler::open_run(&root, "run-001")?;
    let status = scheduler.status()?;
    let prompt = status.state_prompt();
    let rendered = prompt.as_str();

    assert!(rendered.contains("Build the artifact."));
    assert!(rendered.contains("files_exact"));
    assert!(rendered.contains("artifacts"));
    assert!(rendered.contains("marker.txt"));

    for comment in [
        "COMMENT_BEFORE_STATE",
        "COMMENT_INLINE_STATE",
        "COMMENT_INLINE_PROMPT",
        "COMMENT_INLINE_NEXT",
        "COMMENT_BEFORE_GUARD",
        "COMMENT_INLINE_GUARD",
        "COMMENT_KIND",
        "COMMENT_PATH",
        "COMMENT_ENTRIES",
        "COMMENT_OTHER_STATE",
    ] {
        assert!(
            !rendered.contains(comment),
            "comment leaked into prompt: {comment}"
        );
    }
    assert!(!rendered.contains("Review the artifact."));

    fs::remove_dir_all(root)?;
    Ok(())
}
