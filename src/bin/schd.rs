use std::env;
use std::io::{self, Write};

fn main() {
    let project_root = match env::current_dir() {
        Ok(root) => root,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "schd: current directory: {error}");
            std::process::exit(1);
        }
    };
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    if let Err(error) =
        arca_scheduler::cli::run_from(env::args().skip(1), project_root, &mut stdout)
    {
        let _ = writeln!(stderr, "schd: {error}");
        std::process::exit(1);
    }
}
