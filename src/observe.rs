//! WEBV-014: the operation observer.
//!
//! Builds with `test-fault-injection` append one line - the operation kind,
//! then a newline - to the file named by `RATMAC_TEST_OPERATION_LOG` at each
//! documented operation boundary: `roster`, `record`, `ledger`, `runbook`,
//! `blocker`, `target`, `lock`. Unset or empty, or a build without the
//! feature, reads and writes nothing extra.
//!
//! The residue inspection is not an operation: its existence checks, its
//! runbook byte scan, and its listing of `runs/` for pre-cutover records
//! never pass through here, so a residue refusal leaves the log absent. The
//! call sites below therefore sit in the operation code paths, after the
//! residue preflight has already passed.

/// Record that the Engine performed one operation of `kind`.
#[cfg(feature = "test-fault-injection")]
pub(crate) fn operation(kind: &str) {
    use std::io::Write;
    let Some(log) = std::env::var_os("RATMAC_TEST_OPERATION_LOG").filter(|log| !log.is_empty())
    else {
        return;
    };
    let written = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .and_then(|mut file| file.write_all(format!("{kind}\n").as_bytes()));
    written.unwrap_or_else(|error| {
        panic!("RATMAC_TEST_OPERATION_LOG cannot record operation {kind}: {error}")
    });
}

/// A build without `test-fault-injection` observes nothing.
#[cfg(not(feature = "test-fault-injection"))]
pub(crate) fn operation(_kind: &str) {}
