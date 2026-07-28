use std::env;
use std::io::{self, Write};

fn main() {
    let project_root = match env::current_dir() {
        Ok(root) => root,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "rtm: current directory: {error}");
            std::process::exit(1);
        }
    };
    let args = env::args().skip(1).collect::<Vec<_>>();
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    if args.first().is_some_and(|arg| arg == "schd") {
        let _ = writeln!(stderr, "rtm: unsupported command; invoke rtm");
        std::process::exit(1);
    }
    match ratmac::cli::run_from(args, project_root, &mut stdout) {
        Ok(0) => {}
        // DRD-004: the diagnosis carries its verdict in the exit code.
        Ok(code) => {
            let _ = stdout.flush();
            std::process::exit(code);
        }
        Err(error) => {
            let _ = writeln!(stderr, "rtm: {error}");
            std::process::exit(error.exit_code());
        }
    }
}
