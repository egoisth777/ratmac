//! AOI-001: record a reviewable-snapshot manifest beside an evidence record.
//!
//! Usage: `snapshot-manifest <out-path> <root>... [-- <exception>...]`
//!
//! Every file under the declared roots must be tracked or staged; an
//! unreviewable path is refused by name unless it is declared as an
//! exception. On success the manifest — path, tracking state, SHA-256 — is
//! written to `<out-path>` so a reviewer can reconstruct the claim.

use ratmac_qa::snapshot::record_snapshot;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut split = args.splitn(2, |arg| arg == "--");
    let declared: Vec<String> = split.next().unwrap_or(&[]).to_vec();
    let exceptions: Vec<String> = split.next().unwrap_or(&[]).to_vec();

    let Some((out_path, roots)) = declared.split_first() else {
        eprintln!("usage: snapshot-manifest <out-path> <root>... [-- <exception>...]");
        return ExitCode::FAILURE;
    };
    if roots.is_empty() {
        eprintln!("snapshot-manifest: declare at least one evidence root");
        return ExitCode::FAILURE;
    }

    let repo_root = std::env::current_dir().expect("current directory");
    let root_refs: Vec<&str> = roots.iter().map(String::as_str).collect();
    let exception_refs: Vec<&str> = exceptions.iter().map(String::as_str).collect();

    match record_snapshot(&repo_root, &root_refs, &exception_refs) {
        Ok(manifest) => {
            let out = Path::new(out_path);
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).expect("create manifest directory");
            }
            fs::write(out, manifest.render()).expect("write manifest");
            println!(
                "snapshot-manifest: {} rows over {} -> {out_path}",
                manifest.rows.len(),
                root_refs.join(", ")
            );
            ExitCode::SUCCESS
        }
        Err(violations) => {
            eprintln!("snapshot-manifest: refused; evidence would not be reviewable");
            for violation in violations {
                eprintln!("  {}: {}", violation.path, violation.reason);
            }
            ExitCode::FAILURE
        }
    }
}
