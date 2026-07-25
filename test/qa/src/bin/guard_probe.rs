//! Deterministic guard program for `command_exit` diagnostics tests.
//!
//! Usage:
//! - `guard-probe say <message>`     emit `<message>` on stderr, exit 1
//! - `guard-probe flood <bytes>`     emit `<bytes>` bytes of stderr, exit 1
//! - `guard-probe silent`            emit nothing, exit 1
//! - `guard-probe binary`            emit invalid UTF-8 and a NUL, exit 1
//! - `guard-probe partial`           emit stderr, then die without flushing more
//! - `guard-probe pass`              emit nothing, exit 0

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = args.first().map(String::as_str).unwrap_or("pass");
    let mut stderr = std::io::stderr();

    match mode {
        "say" => {
            let message = args.get(1).cloned().unwrap_or_default();
            let _ = writeln!(stderr, "{message}");
            ExitCode::from(1)
        }
        "flood" => {
            let total: usize = args
                .get(1)
                .and_then(|value| value.parse().ok())
                .unwrap_or(65_536);
            // Deterministic filler with a recognizable tail marker.
            let mut emitted = 0usize;
            while emitted < total {
                let line = format!("filler {emitted:08}\n");
                let _ = stderr.write_all(line.as_bytes());
                emitted += line.len();
            }
            let _ = writeln!(stderr, "TAIL-MARKER-END");
            ExitCode::from(1)
        }
        "silent" => ExitCode::from(1),
        "binary" => {
            let _ = stderr.write_all(&[0xff, 0xfe, 0x00, b'x', 0x80, b'\n']);
            ExitCode::from(1)
        }
        "partial" => {
            let _ = stderr.write_all(b"partial diagnostic before death\n");
            let _ = stderr.flush();
            std::process::exit(3);
        }
        _ => ExitCode::SUCCESS,
    }
}
