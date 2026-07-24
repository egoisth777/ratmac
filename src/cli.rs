use std::fmt;
use std::io::{self, Write};
use std::path::Path;

use crate::{Scheduler, StepOutcome, StepRequest};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliError {
    message: String,
}

impl CliError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CliError {}

impl From<io::Error> for CliError {
    fn from(error: io::Error) -> Self {
        Self::new(format!("write CLI output: {error}"))
    }
}

/// Render the policy-bearing help text for a supported command.
pub fn help(command: impl AsRef<str>) -> &'static str {
    match command.as_ref() {
        "start" => {
            "Usage: rtm start\n\nStart is user-only. Loop entry is never agent-initiated; agents must not start a Run.\n"
        }
        "status" => "Usage: rtm status\n\nReport the active Run without changing it.\n",
        "step" => {
            "Usage: rtm step [--help]\n\nOnly the Main-Agent or a human invokes rtm step. Subagents only read state and never invoke rtm.\n"
        }
        _ => "Usage: rtm <command> [options]\n\nCommands: start, status, step\n",
    }
}

fn command_index(args: &[String]) -> usize {
    usize::from(args.first().is_some_and(|arg| arg == "rtm"))
}

fn is_help(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

/// Run the CLI from supplied arguments without spawning a process.
///
/// `status` and `step` deliberately accept no run-id: both operate on the
/// active Run selected by `Scheduler::open`. Extra positional or flagged
/// arguments are rejected before opening the Scheduler, so they cannot
/// retarget or mutate any Run.
pub fn run_from<I, S, W>(
    args: I,
    project_root: impl AsRef<Path>,
    writer: &mut W,
) -> Result<(), CliError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    W: Write,
{
    let args = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect::<Vec<_>>();
    let project_root = project_root.as_ref().to_path_buf();
    let command_index = command_index(&args);
    let Some(command) = args.get(command_index) else {
        writer.write_all(help("").as_bytes())?;
        return Ok(());
    };
    let command_args = &args[command_index + 1..];

    if is_help(&args) {
        writer.write_all(help(command).as_bytes())?;
        return Ok(());
    }

    if matches!(command.as_str(), "start" | "status" | "step") {
        if !command_args.is_empty() {
            return Err(CliError::new(format!(
                "{command} accepts no run-id or extra arguments"
            )));
        }

        let mut scheduler = Scheduler::open(&project_root)
            .map_err(|error| CliError::new(format!("{command}: {error}")))?;
        if command == "start" {
            scheduler
                .start()
                .map_err(|error| CliError::new(format!("start: {error}")))?;
            return Ok(());
        }
        match command.as_str() {
            "status" => {
                let report = scheduler
                    .status()
                    .map_err(|error| CliError::new(format!("status: {error}")))?;
                writeln!(writer, "{report}")?;
                writer.write_all(report.phase_prompt().as_str().as_bytes())?;
                writer.write_all(b"\n")?;
            }
            "step" => {
                let outcome = scheduler
                    .step(StepRequest::new(""))
                    .map_err(|error| CliError::new(format!("step: {error}")))?;
                if matches!(outcome, StepOutcome::Refused { .. }) {
                    writeln!(writer, "rtm: {outcome}")?;
                } else {
                    let report = scheduler
                        .status()
                        .map_err(|error| CliError::new(format!("status: {error}")))?;
                    writer.write_all(report.phase_prompt().as_str().as_bytes())?;
                    writer.write_all(b"\n")?;
                }
            }
            _ => unreachable!(),
        }
        return Ok(());
    }

    Err(CliError::new(format!(
        "unsupported command or option: {}",
        args.get(command_index).unwrap_or(&String::new())
    )))
}
