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

// Depth of active `record`-quiet scopes on this thread: an authorization
// verification re-reads the archive and evidence the motion's earlier
// stages already observed, so its scans add no `record` mark of their own
// - the landing authorization record is the one `record` mark a governed
// motion adds. Thread-local so concurrent in-process invocations from
// parallel test threads never drop each other's marks.
thread_local! {
    static RECORD_QUIET: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// One scope in which `record` operations are not observed.
pub(crate) struct RecordQuiet;

impl Drop for RecordQuiet {
    fn drop(&mut self) {
        RECORD_QUIET.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

/// Enter a `record`-quiet scope for the remainder of this guard's life.
#[must_use]
pub(crate) fn quiet_records() -> RecordQuiet {
    RECORD_QUIET.with(|depth| depth.set(depth.get() + 1));
    RecordQuiet
}

/// Record that the Engine performed one operation of `kind`.
#[cfg(feature = "test-fault-injection")]
pub(crate) fn operation(kind: &str) {
    use std::io::Write;
    if kind == "record" && RECORD_QUIET.with(|depth| depth.get()) > 0 {
        return;
    }
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
