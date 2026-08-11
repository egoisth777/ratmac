use std::fmt;
use std::io::{self, Write};
use std::path::Path;

use crate::{Scheduler, StepOutcome, StepRequest};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliError {
    message: String,
    exit_code: i32,
}

impl CliError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: 1,
        }
    }

    /// A usage refusal: the caller asked for something the interface does not
    /// offer. DRD-004 reserves `2` for "this cannot be diagnosed as asked".
    fn refusal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: 2,
        }
    }

    /// The process exit code this failure deserves.
    pub fn exit_code(&self) -> i32 {
        self.exit_code
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
            "Usage: rtm start\n\nA human may invoke rtm start directly. The Main-Agent may invoke it only after explicit human Run-start sign-off for the current target project. A Subagent never invokes any rtm command and only reads state.\n"
        }
        "status" => {
            "Usage: rtm status --run <id>\n\nReport the named Run without changing it. Run addressing is always required; a missing --run refuses and prints the roster (the listing of .ratmac/runs/).\n"
        }
        "step" => {
            "Usage: rtm step --run <id> [--help]\n\nAdvance exactly the named Run. Run addressing is always required; a missing --run refuses and prints the roster. Only the Main-Agent or a human invokes rtm step. Subagents only read state and never invoke rtm.\n"
        }
        "hold" => {
            "Usage: rtm hold --run <id> --blocker <reference beneath a declared root> --confirm \"hold <id>\"\n\nA human confirms pausing a Run that is blocked for an out-of-scope reason. The Run Record records the pause and its blocker reference, and the named Run routes along the Runbook's blocked route; it stays not-passed until a human resumes it. The blocker is an opaque reference: rtm checks that it exists beneath a declared root and judges nothing else about it. The confirmation phrase is typed at invocation; it is never read from a file.\n"
        }
        "abandon" => {
            "Usage: rtm abandon --run <id> --confirm \"abandon <run id>\"\n\nA human retires a broken Run: rtm records a terminal abandoned event, then retires the admission state, the Run evidence, and the lock so a fresh Run can start. The confirmation phrase names the addressed run id (FDC-007), is typed at invocation, and is never read from a file. Retiring only a leftover lock - no live run anywhere - omits --run and confirms with \"abandon <project directory name>\". No bypass flag exists - a stale lock is retired through this path.\n"
        }
        "spawn" => {
            "Usage: rtm spawn <name> --run <parent id> [--bind name=value ...] [--workspace <path>]\n\nOrdinary checked motion, no confirmation phrase (FDC-007): create a child Run from a class the parent's runbook declares. Legal only while the parent occupies the State declaring that spawn. Each --bind supplies a value for a binding name the spawn declares; --workspace binds the child to an existing directory, or otherwise it inherits the parent's workspace. The entry lands in the parent's spawn ledger (FDC-011). The child is an ordinary Run on the flat roster with its own Run Record and evidence.\n"
        }
        "respawn" => {
            "Usage: rtm respawn --run <id> --confirm \"respawn <run id>\"\n\nA human supersedes a Run: a fresh successor id is minted - never overwriting, the superseded record keeps its address - and the superseded Run is retired by the abandon path. The confirmation phrase names the run id, is typed at invocation, and is never read from a file.\n"
        }
        "doctor" => {
            "Usage: rtm doctor [--json] [runbook path]\n\nRead-only diagnosis: reports the resolved Engine identity, Runbook validity, and runtime state, and names the next legitimate action. Given a path, diagnoses that runbook instead, inside or outside a project. --json emits the findings as data. Exit code: 0 clean, 1 warnings, 2 errors. Writes nothing.\n"
        }
        "scaffold" => {
            "Usage: rtm scaffold <path>\n\nWrite the smallest doctor-clean runbook at a path that does not exist yet. Scaffolding creates exactly one file, never overwrites, and creates no directories. Edit from there and repair with rtm doctor --json <path>; the repair loop is documented in the runbook authoring guide.\n"
        }
        _ => "Usage: rtm <command> [options]\n\nCommands: start, status, step, hold, abandon, spawn, respawn, doctor, scaffold\n",
    }
}

fn command_index(args: &[String]) -> usize {
    usize::from(args.first().is_some_and(|arg| arg == "rtm"))
}

fn is_help(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

/// The roster line printed by every run-addressing refusal: the listing of
/// the resolved `.ratmac/runs/`, read off artifacts.
fn roster_line(project_root: &Path) -> Result<String, CliError> {
    let roots = crate::root::resolve(project_root);
    Scheduler::refuse_flat_residue_with_roots(&roots)
        .map_err(|error| CliError::refusal(error.to_string()))?;
    Ok(roster_line_at(roots.engine_root()))
}

/// Render a roster whose Engine root has already been resolved and preflighted.
fn roster_line_at(engine_root: &Path) -> String {
    let roster = Scheduler::run_roster_at(engine_root);
    if roster.is_empty() {
        "runs: none (.ratmac/runs/ lists no run; rtm start mints one)".to_owned()
    } else {
        format!("runs: {}", roster.join(", "))
    }
}

/// FDC-004: resolve the `--run <id>` a command must carry. A missing,
/// empty, duplicated, non-canonical, escaping, or unknown value refuses and
/// prints the roster; caller input is validated before any path join and the
/// refusal changes nothing.
fn addressed_run(command: &str, args: &[String], project_root: &Path) -> Result<String, CliError> {
    let roots = crate::root::resolve(project_root);
    addressed_run_with_roots(command, args, &roots)
}

/// Resolve an addressed Run through roots already selected for the invocation.
fn addressed_run_with_roots(
    command: &str,
    args: &[String],
    roots: &crate::root::Roots,
) -> Result<String, CliError> {
    let mut run: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--run" => {
                if run.is_some() {
                    return Err(CliError::refusal(format!(
                        "{command}: --run given twice; address exactly one run; {}",
                        roster_line_at(roots.engine_root())
                    )));
                }
                let Some(value) = args.get(index + 1) else {
                    return Err(CliError::refusal(format!(
                        "{command}: --run needs a run id; {}",
                        roster_line_at(roots.engine_root())
                    )));
                };
                run = Some(value.clone());
                index += 2;
            }
            other => {
                return Err(CliError::refusal(format!(
                    "{command}: unexpected argument {other:?}; usage: rtm {command} --run <id>"
                )));
            }
        }
    }
    let Some(id) = run.filter(|id| !id.is_empty()) else {
        return Err(CliError::refusal(format!(
            "{command}: run addressing is always required — pass --run <id>; {}",
            roster_line_at(roots.engine_root())
        )));
    };
    Scheduler::validate_run_address_at(roots.engine_root(), &id)
        .map_err(|error| CliError::refusal(format!("{command}: {error}")))?;
    Ok(id)
}

/// Run the CLI from supplied arguments without spawning a process.
///
/// FDC-004: `status` and `step` act on an existing Run, so `--run <id>` is
/// always required; a missing value refuses and prints the roster (the
/// listing of `.ratmac/runs/`) without touching any Run. `start` takes no
/// run-id: it mints one.
pub fn run_from<I, S, W>(
    args: I,
    project_root: impl AsRef<Path>,
    writer: &mut W,
) -> Result<i32, CliError>
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
        return Ok(0);
    };
    let command_args = &args[command_index + 1..];

    if is_help(&args) {
        writer.write_all(help(command).as_bytes())?;
        return Ok(0);
    }
    if command == "doctor" {
        return doctor(command_args, &project_root, writer);
    }

    if command == "status" {
        status(command_args, &project_root, writer)?;
        return Ok(0);
    }

    if matches!(
        command.as_str(),
        "start" | "step" | "hold" | "abandon" | "spawn" | "respawn" | "scaffold"
    ) {
        Scheduler::refuse_flat_residue(&project_root)
            .map_err(|error| CliError::new(format!("{command}: {error}")))?;
    }

    if command == "scaffold" {
        return scaffold(command_args, writer);
    }

    if command == "hold" {
        hold(command_args, &project_root, writer)?;
        return Ok(0);
    }

    if command == "abandon" {
        abandon(command_args, &project_root, writer)?;
        return Ok(0);
    }

    if command == "spawn" {
        spawn(command_args, &project_root, writer)?;
        return Ok(0);
    }

    if command == "respawn" {
        respawn(command_args, &project_root, writer)?;
        return Ok(0);
    }

    if command == "start" {
        if !command_args.is_empty() {
            return Err(CliError::new(
                "start accepts no run-id or extra arguments".to_owned(),
            ));
        }
        let mut scheduler = Scheduler::open(&project_root)
            .map_err(|error| CliError::new(format!("start: {error}")))?;
        let run = scheduler
            .start()
            .map_err(|error| CliError::new(format!("start: {error}")))?;
        if let Some(id) = run.id() {
            writeln!(writer, "rtm: started run {id} at .ratmac/runs/{id}/")?;
        }
        return Ok(0);
    }

    if command == "step" {
        let id = addressed_run(command, command_args, &project_root)?;
        let mut scheduler = Scheduler::open_run(&project_root, &id)
            .map_err(|error| CliError::new(format!("{command}: {error}")))?;
        let outcome = scheduler
            .step(StepRequest::new(""))
            .map_err(|error| CliError::new(format!("step: {error}")))?;
        if matches!(outcome, StepOutcome::Refused { .. }) {
            writeln!(writer, "rtm: {outcome}")?;
        } else {
            let report = scheduler
                .status()
                .map_err(|error| CliError::new(format!("status: {error}")))?;
            writer.write_all(report.state_prompt().as_str().as_bytes())?;
            writer.write_all(b"\n")?;
        }
        return Ok(0);
    }

    Err(CliError::new(format!(
        "unsupported command or option: {}",
        args.get(command_index).unwrap_or(&String::new())
    )))
}

/// Report one addressed Run through the one root resolution used to open it.
fn status<W: Write>(args: &[String], project_root: &Path, writer: &mut W) -> Result<(), CliError> {
    let roots = crate::root::resolve(project_root);
    Scheduler::refuse_flat_residue_with_roots(&roots)
        .map_err(|error| CliError::new(format!("status: {error}")))?;
    let id = addressed_run_with_roots("status", args, &roots)?;
    // The roots-taking open is required to resolve this invocation once; swapping in the path-taking form is a silent regression no test can catch.
    let scheduler = Scheduler::open_run_with_roots(&roots, &id)
        .map_err(|error| CliError::new(format!("status: {error}")))?;
    let report = scheduler
        .status()
        .map_err(|error| CliError::new(format!("status: {error}")))?;
    scheduler
        .write_engine_root(writer)
        .map_err(|error| CliError::new(format!("status: {error}")))?;
    writeln!(writer, "{report}")?;
    writer.write_all(report.state_prompt().as_str().as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

/// PGE-006: the human-confirmed blocked route.
///
/// Every condition is checked before the first write, so a refusal leaves
/// Scheduler-owned files byte-identical.
fn hold<W: Write>(args: &[String], project_root: &Path, writer: &mut W) -> Result<(), CliError> {
    let mut blocker: Option<String> = None;
    let mut confirmation: Option<String> = None;
    let mut run: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--run" => {
                let Some(value) = args
                    .get(index + 1)
                    .filter(|value| !value.starts_with("--"))
                    .cloned()
                else {
                    return Err(CliError::refusal(format!(
                        "hold: --run needs a run id; {}",
                        roster_line(project_root)?
                    )));
                };
                run = Some(value);
                index += 2;
            }
            "--blocker" => {
                blocker = Some(
                    args.get(index + 1)
                        .filter(|value| !value.starts_with("--"))
                        .cloned()
                        .ok_or_else(|| {
                            CliError::new(
                                "hold: --blocker needs a reference beneath a declared root",
                            )
                        })?,
                );
                index += 2;
            }
            "--confirm" => {
                confirmation = Some(
                    args.get(index + 1)
                        .filter(|value| !value.starts_with("--"))
                        .cloned()
                        .ok_or_else(|| {
                            CliError::new("hold: --confirm needs the exact confirmation phrase")
                        })?,
                );
                index += 2;
            }
            other => {
                return Err(CliError::new(format!(
                    "hold: unexpected extra argument {other}"
                )))
            }
        }
    }

    let request = crate::blocked::HoldRequest {
        blocker,
        confirmation,
        run,
    };
    let plan = crate::blocked::plan_hold(project_root, &request)
        .map_err(|refusal| CliError::new(format!("hold refused; {refusal}")))?;
    crate::blocked::apply_hold(project_root, &plan)
        .map_err(|refusal| CliError::new(format!("hold refused; {refusal}")))?;
    writeln!(
        writer,
        "rtm: run {} paused against {}; routed {} -> {}",
        plan.run_id(),
        plan.blocker(),
        plan.source_state(),
        plan.to_state()
    )?;
    writeln!(
        writer,
        "The Run is not passed; a human resumes it when the blocker clears."
    )?;
    Ok(())
}

/// PGE-007: safe human-confirmed Run abandonment.
///
/// The confirmation phrase is checked before the first write; the retirement
/// itself is all-or-nothing.
/// FDC-007: ordinary checked motion. No confirmation phrase; the Scheduler
/// checks the parent's State and the declared spawn before any write.
fn spawn<W: Write>(args: &[String], project_root: &Path, writer: &mut W) -> Result<(), CliError> {
    let mut name: Option<String> = None;
    let mut run: Option<String> = None;
    let mut bindings: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    let mut workspace: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--bind" => {
                let pair = args
                    .get(index + 1)
                    .filter(|value| !value.starts_with("--"))
                    .cloned()
                    .ok_or_else(|| {
                        CliError::new("spawn: --bind needs a value shaped name=value".to_owned())
                    })?;
                let (bind_name, bind_value) = pair.split_once('=').ok_or_else(|| {
                    CliError::new(format!("spawn: --bind {pair:?} is not shaped name=value"))
                })?;
                let bind_name = bind_name.trim();
                if bind_name.is_empty() {
                    return Err(CliError::new(format!(
                        "spawn: --bind {pair:?} names no binding"
                    )));
                }
                if bindings
                    .insert(bind_name.to_owned(), bind_value.to_owned())
                    .is_some()
                {
                    return Err(CliError::new(format!(
                        "spawn: --bind names {bind_name:?} twice"
                    )));
                }
                index += 2;
            }
            "--workspace" => {
                if workspace.is_some() {
                    return Err(CliError::new("spawn: --workspace given twice".to_owned()));
                }
                workspace = Some(
                    args.get(index + 1)
                        .filter(|value| !value.starts_with("--") && !value.is_empty())
                        .cloned()
                        .ok_or_else(|| {
                            CliError::new("spawn: --workspace needs a directory path".to_owned())
                        })?,
                );
                index += 2;
            }
            "--run" => {
                let Some(value) = args
                    .get(index + 1)
                    .filter(|value| !value.starts_with("--"))
                    .cloned()
                else {
                    return Err(CliError::refusal(format!(
                        "spawn: --run needs a run id; {}",
                        roster_line(project_root)?
                    )));
                };
                run = Some(value);
                index += 2;
            }
            other if name.is_none() && !other.starts_with("--") => {
                name = Some(other.to_owned());
                index += 1;
            }
            other => {
                return Err(CliError::new(format!(
                    "spawn: unsupported option {other}; usage: rtm spawn <name> --run <parent id>"
                )))
            }
        }
    }
    let name = name.ok_or_else(|| {
        CliError::new(
            "spawn needs the declared spawn name: rtm spawn <name> --run <parent id>".to_owned(),
        )
    })?;
    let Some(run) = run else {
        return Err(CliError::refusal(format!(
            "spawn requires --run <parent id>; {}",
            roster_line(project_root)?
        )));
    };
    let child = Scheduler::spawn_to_with_workspace(
        project_root,
        &run,
        &name,
        &bindings,
        workspace.as_deref().map(Path::new),
    )
    .map_err(|error| CliError::new(format!("spawn: {error}")))?;
    writeln!(
        writer,
        "rtm: spawned run {child} (spawn {name}) from run {run}; the child is an ordinary Run at .ratmac/runs/{child}/"
    )?;
    Ok(())
}

/// FDC-007/FDC-006: human-confirmed supersession by a phrase naming the
/// superseded run id.
fn respawn<W: Write>(args: &[String], project_root: &Path, writer: &mut W) -> Result<(), CliError> {
    let mut confirmation: Option<String> = None;
    let mut run: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--run" => {
                let Some(value) = args
                    .get(index + 1)
                    .filter(|value| !value.starts_with("--"))
                    .cloned()
                else {
                    return Err(CliError::refusal(format!(
                        "respawn: --run needs a run id; {}",
                        roster_line(project_root)?
                    )));
                };
                run = Some(value);
                index += 2;
            }
            "--confirm" => {
                confirmation = Some(args.get(index + 1)
                    .filter(|value| !value.starts_with("--"))
                    .cloned()
                    .ok_or_else(|| {
                    CliError::new(
                        "respawn: --confirm needs the exact confirmation phrase \"respawn <run id>\""
                            .to_owned(),
                    )
                })?);
                index += 2;
            }
            other => {
                return Err(CliError::new(format!(
                    "respawn: unsupported option {other}; usage: rtm respawn --run <id> --confirm \"respawn <run id>\""
                )))
            }
        }
    }
    let request = crate::RespawnRequest { run, confirmation };
    let successor = Scheduler::respawn(project_root, &request)
        .map_err(|error| CliError::new(format!("respawn refused; {error}")))?;
    writeln!(
        writer,
        "rtm: run superseded; successor run {successor} minted at .ratmac/runs/{successor}/"
    )?;
    Ok(())
}

fn abandon<W: Write>(args: &[String], project_root: &Path, writer: &mut W) -> Result<(), CliError> {
    let mut confirmation: Option<String> = None;
    let mut run: Option<String> = None;
    // The addressed run resolves first, wherever --run sits, so every
    // refusal below names the phrase for the right target.
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--run" {
            let Some(value) = args
                .get(index + 1)
                .filter(|value| !value.starts_with("--"))
                .cloned()
            else {
                return Err(CliError::refusal(format!(
                    "abandon: --run needs a run id; {}",
                    roster_line(project_root)?
                )));
            };
            run = Some(value);
            index += 2;
        } else {
            index += 1;
        }
    }
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--run" => {
                index += 2;
            }
            "--confirm" => {
                confirmation = Some(
                    args.get(index + 1)
                        .filter(|value| !value.starts_with("--"))
                        .cloned()
                        .ok_or_else(|| {
                            CliError::new(format!(
                                "abandon: --confirm needs the exact confirmation phrase {:?}",
                                crate::abandon::required_phrase(project_root, run.as_deref())
                            ))
                        })?,
                );
                index += 2;
            }
            other => {
                return Err(CliError::new(format!(
                    "abandon: unsupported option {other}; the only option is --confirm {:?}",
                    crate::abandon::required_phrase(project_root, run.as_deref())
                )))
            }
        }
    }

    let request = crate::abandon::AbandonRequest { confirmation, run };
    let plan = crate::abandon::plan_abandon(project_root, &request)
        .map_err(|refusal| CliError::new(format!("abandon refused; {refusal}")))?;
    crate::abandon::apply_abandon(project_root, &plan)
        .map_err(|refusal| CliError::new(format!("abandon refused; {refusal}")))?;
    match plan.state.as_deref() {
        Some(state) => writeln!(
            writer,
            "rtm: Run abandoned at state {state}; admission state, Run evidence, and lock retired. A fresh rtm start may begin."
        )?,
        None => writeln!(
            writer,
            "rtm: retirement completed; the leftover lock is retired and no Run remains."
        )?,
    }
    Ok(())
}

/// ORS-002: read-only diagnosis. Reports the resolved Engine identity,
/// Runbook presence/validity, and runtime state. Writes nothing.
/// DRD-005: diagnose the project, or any runbook path the caller names.
///
/// The argument-free form keeps its ORS-002 environment report and appends the
/// findings; a path diagnoses that file alone. Either way the command writes
/// nothing and its exit code carries the verdict.
fn doctor<W: Write>(args: &[String], project_root: &Path, writer: &mut W) -> Result<i32, CliError> {
    const USAGE: &str = "doctor accepts --json and one runbook path";
    let mut json = false;
    let mut target: Option<&str> = None;
    for arg in args {
        if arg == "--json" {
            if json {
                return Err(CliError::refusal(format!(
                    "doctor: --json given twice; {USAGE}"
                )));
            }
            json = true;
        } else if arg.starts_with('-') {
            return Err(CliError::refusal(format!(
                "doctor: unknown option {arg:?}; {USAGE}"
            )));
        } else if target.is_some() {
            return Err(CliError::refusal(format!(
                "doctor: one runbook path at a time; {USAGE}"
            )));
        } else {
            target = Some(arg);
        }
    }

    if let Some(target) = target {
        let target = Path::new(target);
        let project_root = crate::root::addressed_project_root(target);
        let roots = crate::root::resolve(&project_root);
        Scheduler::refuse_flat_residue_with_roots(&roots)
            .map_err(|error| CliError::refusal(error.to_string()))?;
        // A path that cannot be read is a diagnosis (`RB101`), not a usage
        // error: an authoring loop asks about paths that do not exist yet, and
        // it reads codes, not refusals.
        let diagnosis = crate::doctor::diagnose_with_roots(target, &roots);
        if !json {
            diagnosis.write_engine_root(writer)?;
        }
        write_findings(&diagnosis, json, writer)?;
        return Ok(diagnosis.exit_code());
    }

    let roots = crate::root::resolve(project_root);
    Scheduler::refuse_flat_residue_with_roots(&roots)
        .map_err(|error| CliError::new(format!("doctor: {error}")))?;
    let runbook_path = roots.machine_class_path();
    // The roots-taking diagnosis is required to resolve this invocation once; swapping in the path-taking form is a silent regression no test can catch.
    let diagnosis = crate::doctor::diagnose_with_roots(&runbook_path, &roots);
    if json {
        write_findings(&diagnosis, true, writer)?;
        return Ok(diagnosis.exit_code());
    }

    environment_report(&roots, &diagnosis, writer)?;
    write_findings(&diagnosis, false, writer)?;
    Ok(diagnosis.exit_code())
}

/// AAL-002: write one runbook at a path that does not exist yet.
fn scaffold<W: Write>(args: &[String], writer: &mut W) -> Result<i32, CliError> {
    const USAGE: &str = "scaffold takes exactly one path";
    let mut target: Option<&str> = None;
    for arg in args {
        if arg.starts_with('-') {
            return Err(CliError::refusal(format!(
                "scaffold: unknown option {arg:?}; {USAGE}"
            )));
        }
        if target.is_some() {
            return Err(CliError::refusal(format!(
                "scaffold: one path at a time; {USAGE}"
            )));
        }
        target = Some(arg);
    }
    let Some(target) = target else {
        return Err(CliError::refusal(format!(
            "scaffold: no path given; {USAGE}"
        )));
    };
    let path = Path::new(target);
    crate::scaffold::write_scaffold(path)
        .map_err(|refusal| CliError::refusal(refusal.to_string()))?;
    writeln!(
        writer,
        "Wrote {}. Diagnose it with `rtm doctor --json {}`.",
        crate::root::displayed(path),
        crate::root::displayed(path)
    )?;
    Ok(0)
}

fn write_findings<W: Write>(
    diagnosis: &crate::doctor::Diagnosis,
    json: bool,
    writer: &mut W,
) -> Result<(), CliError> {
    if json {
        writer.write_all(diagnosis.render_json().as_bytes())?;
    } else {
        writer.write_all(diagnosis.render_report().as_bytes())?;
    }
    Ok(())
}

/// ORS-002: the environment report - Engine identity, Runbook, State, and the
/// next legitimate action.
fn environment_report<W: Write>(
    roots: &crate::root::Roots,
    diagnosis: &crate::doctor::Diagnosis,
    writer: &mut W,
) -> Result<(), CliError> {
    let engine_path = std::env::current_exe()
        .map_err(|error| CliError::new(format!("resolve Engine binary: {error}")))?;
    let engine_hash = sha256_file(&engine_path)
        .map_err(|error| CliError::new(format!("hash Engine binary: {error}")))?;
    writeln!(
        writer,
        "Engine: {} (sha256: {})",
        crate::root::displayed(&engine_path),
        engine_hash
    )?;

    diagnosis.write_engine_root(writer)?;
    let runbook_path = roots.machine_class_path();

    if runbook_path.is_file() {
        match std::fs::read_to_string(&runbook_path) {
            // TRP-001: the doctor judges the runbook with the parser that runs
            // it, never with a looser second reader.
            Ok(source) => match crate::machine::MachineClass::from_toml(&source) {
                Ok(_) => writeln!(writer, "Runbook: .ratmac/ratmac.toml (valid)")?,
                Err(error) => writeln!(writer, "Runbook: .ratmac/ratmac.toml (INVALID: {error})")?,
            },
            Err(error) => writeln!(writer, "Runbook: .ratmac/ratmac.toml (unreadable: {error})")?,
        }
    } else {
        writeln!(
            writer,
            "Runbook: .ratmac/ratmac.toml (absent — no Machine Class declared)"
        )?;
    }

    // FDC-004: listing the resolved .ratmac/runs/ is the roster; each Run's
    // Run Record lives in its own directory.
    let engine_root = roots.engine_root();
    let roster = crate::Scheduler::run_roster_at(engine_root);
    let runs_dir = engine_root.join("runs");
    if roster.is_empty() {
        writeln!(
            writer,
            "Run Record: .ratmac/runs/ (empty — no Run on the roster)"
        )?;
        writeln!(
            writer,
            "Next: .ratmac/ratmac.toml is the human-authored Machine Class; \
             .ratmac/runs/<id>/run.toml is Scheduler-owned runtime state created only by \
             `rtm start`, which mints the run id. To begin a Run, invoke `rtm start`; \
             address it afterwards with --run <id>."
        )?;
    } else {
        for id in roster {
            let state_path = runs_dir.join(&id).join("run.toml");
            let shown = format!(".ratmac/runs/{id}/run.toml");
            if state_path.is_file() {
                match std::fs::read_to_string(&state_path) {
                    Ok(source) => {
                        if let Ok(table) = source.parse::<toml::Value>() {
                            let state = table
                                .get("state")
                                .and_then(toml::Value::as_str)
                                .unwrap_or("unknown");
                            writeln!(writer, "Run Record: {shown} (state: {state})")?;
                        } else {
                            writeln!(writer, "Run Record: {shown} (present, unreadable)")?;
                        }
                    }
                    Err(_) => {
                        writeln!(writer, "Run Record: {shown} (present, unreadable)")?;
                    }
                }
            } else {
                writeln!(writer, "Run Record: {shown} (absent — run {id} is retired)")?;
            }
        }
    }

    Ok(())
}

fn sha256_file(path: &Path) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
