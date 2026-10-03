use std::io::Write;

fn main() {
    // The current-directory read is the only thing that may precede the
    // trace latch; its failure renders through the one stderr owner.
    let project_root = match std::env::current_dir() {
        Ok(root) => root,
        Err(error) => {
            ratmac::diag::diagnostic(format_args!("rtm: current directory: {error}"));
            std::process::exit(1);
        }
    };
    // t-128: latch `RATMAC_TRACE` once, before any command dispatch. An
    // invalid value refuses before any work - before help, the residue
    // preflight, every lock, and every write - so nothing runs.
    if let Err(value) = ratmac::diag::latch_from_environment() {
        ratmac::diag::invalid_toggle(&value);
        std::process::exit(2);
    }
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut stdout = std::io::stdout().lock();
    match ratmac::cli::run_from(args, project_root, &mut stdout) {
        Ok(0) => {}
        // DRD-004: the diagnosis carries its verdict in the exit code.
        Ok(code) => {
            let _ = stdout.flush();
            std::process::exit(code);
        }
        Err(error) => {
            ratmac::diag::diagnostic(format_args!("rtm: {error}"));
            std::process::exit(error.exit_code());
        }
    }
}
