//! Deterministic file-barrier guard for lock-split QA.
//!
//! The caller supplies one marker and one release path through environment
//! variables. The guard publishes its marker before it waits for the release,
//! so a test can use the files as a handshake rather than a scheduling delay.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_BARRIER_TIMEOUT: Duration = Duration::from_secs(30);

fn required_path(name: &str) -> Result<PathBuf, String> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing required {name} environment variable"))
}

fn barrier_timeout() -> Result<Duration, String> {
    match std::env::var("RATMAC_QA_BARRIER_TIMEOUT_MILLIS") {
        Ok(value) => value
            .parse::<u64>()
            .map(Duration::from_millis)
            .map_err(|error| format!("parse RATMAC_QA_BARRIER_TIMEOUT_MILLIS {value:?}: {error}")),
        Err(std::env::VarError::NotPresent) => Ok(DEFAULT_BARRIER_TIMEOUT),
        Err(error) => Err(format!("read RATMAC_QA_BARRIER_TIMEOUT_MILLIS: {error}")),
    }
}

fn main() -> ExitCode {
    let marker = match required_path("RATMAC_QA_BARRIER_MARKER") {
        Ok(path) => path,
        Err(error) => {
            eprintln!("lock-barrier: {error}");
            return ExitCode::from(64);
        }
    };
    let release = match required_path("RATMAC_QA_BARRIER_RELEASE") {
        Ok(path) => path,
        Err(error) => {
            eprintln!("lock-barrier: {error}");
            return ExitCode::from(64);
        }
    };
    let timeout_marker = match required_path("RATMAC_QA_BARRIER_TIMEOUT_MARKER") {
        Ok(path) => path,
        Err(error) => {
            eprintln!("lock-barrier: {error}");
            return ExitCode::from(64);
        }
    };
    let timeout = match barrier_timeout() {
        Ok(timeout) => timeout,
        Err(error) => {
            eprintln!("lock-barrier: {error}");
            return ExitCode::from(64);
        }
    };

    if let Some(parent) = marker.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            eprintln!(
                "lock-barrier: create marker directory {}: {error}",
                parent.display()
            );
            return ExitCode::from(65);
        }
    }
    if let Err(error) = fs::write(&marker, format!("pid={}", std::process::id())) {
        eprintln!("lock-barrier: write marker {}: {error}", marker.display());
        return ExitCode::from(65);
    }

    let deadline = Instant::now() + timeout;
    loop {
        if release.is_file() {
            return ExitCode::SUCCESS;
        }
        if Instant::now() >= deadline {
            let _ = fs::write(
                &timeout_marker,
                "release file did not arrive before deadline\n",
            );
            eprintln!(
                "lock-barrier: timed out after {timeout:?} waiting for {}",
                release.display()
            );
            return ExitCode::from(2);
        }
        thread::yield_now();
    }
}
