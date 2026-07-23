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
    let root = std::env::temp_dir().join(format!("arca-scheduler-t027-{nonce}"));
    let arca = root.join(".arca");
    fs::create_dir_all(&arca).expect("create isolated .arca directory");
    for file in ["ratmac.toml", "state.toml", "log.md"] {
        fs::copy(fixture_root().join(".arca").join(file), arca.join(file))
            .expect("copy comment fixture");
    }
    root
}

#[test]
fn ratmac_comments_are_absent_from_phase_prompt() -> Result<(), Box<dyn Error>> {
    let root = copy_fixture_to_temp();
    let scheduler = Scheduler::open(&root)?;
    let status = scheduler.status()?;
    let prompt = status.phase_prompt();
    let rendered = prompt.as_str();

    assert!(rendered.contains("Build the artifact."));
    assert!(rendered.contains("files_exact"));
    assert!(rendered.contains("artifacts"));
    assert!(rendered.contains("marker.txt"));

    for comment in [
        "COMMENT_BEFORE_PHASE",
        "COMMENT_INLINE_PHASE",
        "COMMENT_INLINE_PROMPT",
        "COMMENT_INLINE_NEXT",
        "COMMENT_BEFORE_GUARD",
        "COMMENT_INLINE_GUARD",
        "COMMENT_KIND",
        "COMMENT_PATH",
        "COMMENT_ENTRIES",
        "COMMENT_OTHER_PHASE",
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
