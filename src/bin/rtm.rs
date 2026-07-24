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
    if let Err(error) = ratmac::cli::run_from(args, project_root, &mut stdout) {
        let _ = writeln!(stderr, "rtm: {error}");
        std::process::exit(1);
    }
}
