use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
type AuthorizationArguments = (Vec<String>, Vec<(String, String)>, Option<String>, usize);

use crate::root::Displayed;
use crate::{Scheduler, StepOutcome, StepRequest};

pub use crate::root::entries::run_from;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliError {
    message: String,
    exit_code: i32,
}

impl CliError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: 1,
        }
    }

    /// A usage refusal: the caller asked for something the interface does not
    /// offer. DRD-004 reserves `2` for "this cannot be diagnosed as asked".
    pub(crate) fn refusal(message: impl Into<String>) -> Self {
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
        "init" => {
            "Usage: rtm init\n\nInstall the Engine-root tracking policy before any Run is admitted: .ratmac/.gitignore carries the four runtime lines /runs/, /mint.toml, /locks/, and /log.md, each exactly once, while the runbook and evidence/ stay trackable. Existing bytes are preserved; a conflicting effective rule, a runtime file already in Git's index, or a failed repository inspection refuses and changes nothing. Idempotent and concurrency-safe: repeating it writes nothing, and no Run, runbook, commit, or configuration is created. rtm start runs the same policy step before admitting a Run.\n"
        }
        "start" => {
            "Usage: rtm start\n\nA human may invoke rtm start directly. The Main-Agent may invoke it only after explicit human Run-start sign-off for the current target project. A Subagent never invokes any rtm command and only reads state.\n"
        }
        "status" => {
            "Usage: rtm status --run <id>\n\nReport the named Run without changing it. Run addressing is always required; a missing --run refuses and prints the roster (the listing of .ratmac/runs/).\n"
        }
        "step" => {
            "Usage: rtm step --run <id> [--review <label>:<sig>]... [--approve <sig>] [--help]\n\nAdvance exactly the named Run. Run addressing is always required; a missing --run refuses and prints the roster. Only the Main-Agent or a human invokes rtm step. Subagents only read state and never invoke rtm.\n"
        }
        "hold" => {
            "Usage: rtm hold --run <id> --blocker <reference beneath a declared root> --confirm \"hold <id>\" [--review <label>:<sig>]... [--approve <sig>]\n\nA human confirms pausing a Run that is blocked for an out-of-scope reason. The Run Record records the pause and its blocker reference, and the named Run routes along the Runbook's blocked route; it stays not-passed until a human resumes it. The blocker is an opaque reference: rtm checks that it exists beneath a declared root and judges nothing else about it. The confirmation phrase is typed at invocation; it is never read from a file.\n"
        }
        "intent" => {
            "Usage: rtm intent --run <id> [--hold --blocker <ref> | --resume | --recover [--enrollment <path>]] [--review <label>:<sig>]... --out <path> [--help]\n\nRender the canonical intent a governed Run's next motion binds, for review and approval signatures to be taken over. The bytes cover the motion's every varying fact; reviewers sign them under the review namespace and the Approver signs them plus the review-set digest under the approval namespace. --hold renders the blocked-route motion instead of the ordinary one; --resume renders the canonical resolution intent of a held Run, the bytes one ratmac-resume-v1 signature by the designated Approver covers. The render writes one file at the caller-named --out, never under the Engine root, and changes no Run state.\n"
        }
        "resume" => {
            "Usage: rtm resume --run <id> --blocker <reference beneath a declared root> --confirm \"resume <id>\" [--resolution <sig>] [--help]\n\nA human resumes a held Run when its blocker clears. The given blocker must equal the recorded blocker exactly; the Engine verifies the authorization and its binding and never judges whether the external cause was fixed - editing or deleting the referenced record substitutes for nothing. The confirmation phrase is typed at invocation and never read from a file. Under a declared review policy the resume must also carry one ratmac-resume-v1 signature by the designated Approver over the canonical intent (rtm intent --run <id> --resume).\n"
        }
        "recover" => {
            "Usage: rtm recover --run <id> [--enrollment <path>] --decision <sig> --confirm \"recover <id>\" [--help]\n\nA human amends a Run whose rule bytes drifted: one ratmac-recover-v1 signature by the designated Approver over the canonical recover intent (rtm intent --run <id> --recover) adopts the corrected authority as an append-only amendment under the Engine root. The Run keeps its address, State, class, bindings, workspace, and original pins; an amendment neither resumes a paused Run nor approves a transition. A legacy Run without previously enrolled recovery trust requires an independently enrolled external authority (--enrollment); the builder may neither generate it nor substitute task authorization for it. An interrupted publication is reconciled by repeating this command exactly once. The signed decision is verified under the prior trusted policy or a previously enrolled external authority, never the proposed replacement, and a confirmation phrase alone authorizes nothing. Adopting the amendment invalidates the affected proof; later motion needs its own fresh checks.\n"
        }
        "abandon" => {
            "Usage: rtm abandon --run <id> --confirm \"abandon <run id>\"\n\nA human retires a broken Run: rtm records a terminal abandoned event, then retires the admission state, the Run evidence, and the lock so a fresh Run can start. The confirmation phrase names the addressed run id (FDC-007), is typed at invocation, and is never read from a file. Retiring only a leftover lock - no Run admitted anywhere - omits --run and confirms with \"abandon <project directory name>\"; that phrase never retires a Run, and while any Run is admitted abandon without --run refuses and lists the roster. No bypass flag exists - a stale lock is retired through this path.\n"
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
        "skill" => {
            "Usage: rtm skill <path>\n\nWrite the thin ratmac-operator skill folder (SKILL.md plus references) at a path that does not exist yet. The skill write creates exactly one folder, never overwrites, and creates no directories. SKILL.md carries the identity stamp of the engine that wrote it. The skill itself teaches only invariants - the operating loop and the never-touch rules - and reaches everything current by running the engine and reading its output.\n"
        }
        "endpoint" => {
            "Usage: rtm endpoint --invocation <address> --lineage <hex> --receipt-fd <n>\n\nThe machine-facing nested-invocation entry (WEB-006), not for human use. The parent Engine launches it in the child checkout with the Engine-minted invocation address, the complete invocation lineage, and the dedicated receipt pipe's write end. The protocol lineage and the inherited RATMAC_LINEAGE environment must agree byte for byte; a repeated declared program identity or canonical Engine root, a malformed lineage, or an over-deep chain refuses naming the chain before any Run is admitted. At most one child Run exists per invocation address. On success it writes one strict terminal receipt - only after persisting and reading back the child Run's terminal fact - to the receipt pipe, prints nothing, and exits 0.\n"
        }
        _ => general_usage(),
    }
}

/// The one public route table (WEB-004): the verbs the general usage's
/// `Commands:` line lists are exactly the commands `rtm` dispatches, and a
/// verb added to dispatch is added here.
const COMMANDS: [&str; 15] = [
    "init", "start", "status", "step", "hold", "intent", "resume", "recover", "abandon", "spawn",
    "respawn", "doctor", "scaffold", "skill", "endpoint",
];

/// The general usage, rendered from the route table so the table stays its
/// one source.
fn general_usage() -> &'static str {
    static USAGE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    USAGE.get_or_init(|| {
        format!(
            "Usage: rtm <command> [options]\n\nCommands: {}\n",
            COMMANDS.join(", ")
        )
    })
}

fn command_index(args: &[String]) -> usize {
    usize::from(args.first().is_some_and(|arg| arg == "rtm"))
}

fn is_help(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

/// The roster line printed by every run-addressing refusal: the listing of
/// the resolved `.ratmac/runs/`, read off artifacts.
fn roster_line(roots: &crate::root::Roots) -> Result<Roster, CliError> {
    Scheduler::refuse_flat_residue_with_roots(roots)
        .map_err(|error| CliError::refusal(error.to_string()))?;
    Ok(roster_line_at(roots.engine_root()))
}

/// AOP-002: wrap a hard error that halted a status/step render with the
/// next-act table's line for the stable surface the error names, so the
/// rendering still ends in one truthful `next:` line.
fn hard_error(command: &str, error: crate::state::StateError) -> CliError {
    CliError::new(format!(
        "{command}: {error}\n{}",
        crate::teach::error_next(&error)
    ))
}

/// A rendered roster: the `runs: ...` summary a refusal's first line
/// carries, and the descriptive rows (WRS-005) that follow that line.
struct Roster {
    line: String,
    rows: String,
}

/// Render a roster whose Engine root has already been resolved and preflighted.
fn roster_line_at(engine_root: &Path) -> Roster {
    let roster = Scheduler::run_roster_at(engine_root);
    Roster {
        line: format!(
            "runs: {}",
            crate::roster::summary(
                engine_root,
                &roster,
                "none (.ratmac/runs/ lists no run; rtm start mints one)"
            )
        ),
        rows: crate::roster::rows(engine_root, &roster),
    }
}

/// FDC-004: resolve the `--run <id>` a command must carry. A missing,
/// empty, duplicated, non-canonical, escaping, or unknown value refuses and
/// prints the roster; caller input is validated before any path join and the
/// refusal changes nothing.
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
                    let roster = roster_line_at(roots.engine_root());
                    return Err(CliError::refusal(format!(
                        "{command}: --run given twice; address exactly one run; {}{}\n{}",
                        roster.line,
                        roster.rows,
                        crate::teach::addressing_next(roots.engine_root())
                    )));
                }
                let Some(value) = args.get(index + 1) else {
                    let roster = roster_line_at(roots.engine_root());
                    return Err(CliError::refusal(format!(
                        "{command}: --run needs a run id; {}{}\n{}",
                        roster.line,
                        roster.rows,
                        crate::teach::addressing_next(roots.engine_root())
                    )));
                };
                run = Some(value.clone());
                index += 2;
            }
            other => {
                return Err(CliError::refusal(format!(
                    "{command}: unexpected argument {other:?}; usage: rtm {command} --run <id>\n{}",
                    crate::teach::addressing_next(roots.engine_root())
                )));
            }
        }
    }
    let Some(id) = run.filter(|id| !id.is_empty()) else {
        let roster = roster_line_at(roots.engine_root());
        return Err(CliError::refusal(format!(
            "{command}: run addressing is always required — pass --run <id>; {}{}\n{}",
            roster.line,
            roster.rows,
            crate::teach::addressing_next(roots.engine_root())
        )));
    };
    Scheduler::validate_run_address_at(roots.engine_root(), &id).map_err(|error| {
        CliError::refusal(format!(
            "{command}: {error}\n{}",
            crate::teach::addressing_next(roots.engine_root())
        ))
    })?;
    Ok(id)
}

/// Run the CLI inside one invocation context; the path-taking `run_from`
/// lives in `src/root.rs`. Each project the command addresses is resolved
/// by `invocation` the first time it is needed, and every handler below is
/// handed that context rather than a path.
///
/// WEB-004 fixes the order: splitting the arguments into command and target,
/// Engine-root resolution, and the residue inspection are the only things
/// that may precede a recognized command's refusal for retired-layout
/// residue - never the command's own option validation, runbook parsing,
/// roster reads, blocker lookup, target checks, or writes.
pub(crate) fn run<I, S, W>(
    args: I,
    invocation: &crate::root::Invocation,
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
    let command_index = command_index(&args);
    let Some(command) = args.get(command_index) else {
        writer.write_all(help("").as_bytes())?;
        return Ok(0);
    };
    let command_args = &args[command_index + 1..];

    // RAT-003: the retired spelling is refused by name before any help,
    // resolution, inspection, read, or write, and its refusal never spells
    // the retired name. The binary and the library answer identically.
    if command == "schd" {
        return Err(CliError::new("unsupported command; invoke rtm"));
    }

    // A first token the route table does not list is an unknown command: a
    // pure usage response that resolves, inspects, reads, and writes
    // nothing.
    if !COMMANDS.contains(&command.as_str()) {
        if is_help(&args) {
            writer.write_all(help("").as_bytes())?;
            return Ok(0);
        }
        return Err(unsupported(command));
    }

    // The one pure per-command help form: a recognized command followed by
    // exactly one `--help` or `-h` and nothing else. Nothing is resolved,
    // inspected, read, or written.
    if command_args.len() == 1 && matches!(command_args[0].as_str(), "--help" | "-h") {
        writer.write_all(help(command).as_bytes())?;
        return Ok(0);
    }

    // One preflight before any handler: the invoking checkout with its
    // shared primary root first, every command.
    let roots = invocation.checkout();
    Scheduler::refuse_flat_residue_with_roots(&roots).map_err(|error| match command.as_str() {
        "status" => hard_error("status", error),
        _ => CliError::new(format!("{command}: {error}")),
    })?;

    // Then the project the arguments explicitly address, when the
    // non-validating target scan identifies one.
    match command.as_str() {
        "doctor" | "scaffold" | "skill" => {
            if let Some(target) = command_args.iter().find(|arg| !arg.starts_with('-')) {
                let project =
                    invocation.project(&crate::root::addressed_project_root(Path::new(target)));
                Scheduler::refuse_flat_residue_with_roots(&project)
                    .map_err(|error| CliError::refusal(error.to_string()))?;
            }
        }
        "spawn" => {
            if let Some(workspace) = sole_workspace(command_args) {
                Scheduler::refuse_spawn_workspace_residue(&roots, Path::new(workspace))
                    .map_err(|error| CliError::new(format!("spawn: {error}")))?;
            }
        }
        _ => {}
    }

    // Help among any other operational argument cannot skip the preflight
    // above; once that has passed, the command's help is the whole answer
    // and nothing else runs.
    if is_help(command_args) {
        writer.write_all(help(command).as_bytes())?;
        return Ok(0);
    }

    match command.as_str() {
        "init" => init(command_args, &roots, writer),
        "doctor" => doctor(command_args, invocation, writer),
        "status" => {
            status(command_args, &roots, writer)?;
            Ok(0)
        }
        "scaffold" => scaffold(command_args, invocation, writer),
        "skill" => skill(command_args, invocation, writer),
        "endpoint" => endpoint(command_args, &roots, writer),
        "hold" => {
            hold(command_args, &roots, writer)?;
            Ok(0)
        }
        "intent" => intent(command_args, &roots, invocation, writer),
        "resume" => {
            resume(command_args, &roots, writer)?;
            Ok(0)
        }
        "recover" => {
            recover(command_args, &roots, invocation, writer)?;
            Ok(0)
        }
        "abandon" => {
            abandon(command_args, &roots, writer)?;
            Ok(0)
        }
        "spawn" => {
            spawn(command_args, &roots, writer)?;
            Ok(0)
        }
        "respawn" => {
            respawn(command_args, &roots, writer)?;
            Ok(0)
        }
        "start" => start(command_args, &roots, writer),
        "step" => step(command_args, &roots, writer),
        _ => unreachable!("the route table lists every dispatched command"),
    }
}

/// WEB-004: the non-validating spawn target scan. A `--workspace` value
/// addresses a project only when the option occurs exactly once and its next
/// argument exists, is non-empty, and does not begin with `--`; anything
/// else - repeated, valueless, or empty - addresses no project, so only the
/// invoking checkout is inspected and the ordinary usage error follows.
fn sole_workspace(args: &[String]) -> Option<&str> {
    let mut workspace: Option<&str> = None;
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--workspace" {
            let value = args.get(index + 1)?;
            if workspace.is_some() || value.is_empty() || value.starts_with("--") {
                return None;
            }
            workspace = Some(value);
        }
        index += 1;
    }
    workspace
}

/// WEB-002 (t-121): install the Engine-root tracking policy, the same step
/// `Scheduler::start` runs before admitting a Run. The three stdout lines
/// carry the install fact, the verification fact, and the next legitimate
/// action; no runbook, Run, or Git state is created.
fn init<W: Write>(
    args: &[String],
    roots: &crate::root::Roots,
    writer: &mut W,
) -> Result<i32, CliError> {
    if !args.is_empty() {
        return Err(CliError::refusal("init accepts no arguments"));
    }
    let outcome =
        crate::ignore_policy::install(roots.invoking_checkout_root(), roots.engine_root())
            .map_err(|refusal| CliError::new(format!("init: {refusal}")))?;
    let policy = crate::ignore_policy::policy_path(roots.engine_root()).displayed();
    let installed = if outcome.installed {
        format!("rtm: installed the Engine-root tracking policy at {policy}")
    } else {
        format!("rtm: Engine-root tracking policy already present at {policy}")
    };
    let verified = if outcome.verified {
        "rtm: verified: runs/, mint.toml, locks/, and log.md ignored; .gitignore, ratmac.toml, \
         and evidence/ trackable"
    } else {
        "rtm: repository-level ignore rules were not verified: Git is absent or the directory \
         is not inside a Git repository"
    };
    writeln!(writer, "{installed}")?;
    writeln!(writer, "{verified}")?;
    // WEBV-014: the runbook existence check that picks the `next:` line is
    // a target operation.
    crate::observe::operation("target");
    let next = if crate::root::checkout_machine_class_path(roots.invoking_checkout_root()).exists()
    {
        "next: rtm start"
    } else {
        "next: rtm scaffold .ratmac/ratmac.toml"
    };
    writeln!(writer, "{next}")?;
    Ok(0)
}

/// AAL-003/LGS-001: mint the project's next Run from the invoking checkout.
fn start<W: Write>(
    args: &[String],
    roots: &crate::root::Roots,
    writer: &mut W,
) -> Result<i32, CliError> {
    if !args.is_empty() {
        return Err(CliError::new(
            "start accepts no run-id or extra arguments".to_owned(),
        ));
    }
    let mut scheduler = Scheduler::open_with_roots(roots)
        .map_err(|error| CliError::new(format!("start: {error}")))?;
    let run = scheduler
        .start()
        .map_err(|error| CliError::new(format!("start: {error}")))?;
    if let Some(id) = run.id() {
        writeln!(writer, "rtm: started run {id} at .ratmac/runs/{id}/")?;
    }
    Ok(0)
}

/// WRA-001: split `--review <label>:<sig>` and `--approve <sig>` options -
/// space or `=` form - out of a command's arguments, leaving every other
/// argument in place for the command's own parse. The values are paths and
/// labels as typed; the authorization boundary interprets them.
fn split_authorization_args(
    command: &str,
    args: &[String],
) -> Result<AuthorizationArguments, CliError> {
    let review_option = |value: &str| -> Result<(String, String), CliError> {
        let (label, signature) = value.split_once(':').ok_or_else(|| {
            CliError::new(format!(
                "{command}: --review {value:?} is not shaped <label>:<signature file>"
            ))
        })?;
        if label.is_empty() || signature.is_empty() {
            return Err(CliError::new(format!(
                "{command}: --review {value:?} names no label or no signature file"
            )));
        }
        Ok((label.to_owned(), signature.to_owned()))
    };
    let mut rest = Vec::new();
    let mut reviews = Vec::new();
    let mut approve: Option<String> = None;
    let mut approval_count: usize = 0;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--review" => {
                let Some(value) = args.get(index + 1).filter(|value| !value.starts_with("--"))
                else {
                    return Err(CliError::new(format!(
                        "{command}: --review needs <label>:<signature file>"
                    )));
                };
                reviews.push(review_option(value)?);
                index += 2;
            }
            other if other.starts_with("--review=") => {
                reviews.push(review_option(
                    other.strip_prefix("--review=").unwrap_or_default(),
                )?);
                index += 1;
            }
            "--approve" => {
                let Some(value) = args.get(index + 1).filter(|value| !value.starts_with("--"))
                else {
                    return Err(CliError::new(format!(
                        "{command}: --approve needs the approver's signature file"
                    )));
                };
                // A repeated --approve is judged at the verification
                // boundary, not here: the last value is carried and the
                // count is reported.
                if approve.replace(value.clone()).is_none() {
                    approval_count = 1;
                } else {
                    approval_count += 1;
                }
                index += 2;
            }
            other if other.starts_with("--approve=") => {
                let value = other
                    .strip_prefix("--approve=")
                    .unwrap_or_default()
                    .to_owned();
                if approve.replace(value).is_none() {
                    approval_count = 1;
                } else {
                    approval_count += 1;
                }
                index += 1;
            }
            other => {
                rest.push(other.to_owned());
                index += 1;
            }
        }
    }
    Ok((rest, reviews, approve, approval_count))
}

/// WRA-001: whether the addressed Run carries its recorded governance
/// marker. Presence alone decides governance.
fn run_is_governed(roots: &crate::root::Roots, id: &str) -> bool {
    roots
        .engine_root()
        .join("runs")
        .join(id)
        .join(crate::review::GOVERNANCE_FILE)
        .is_file()
}

/// FDC-004/R-019: advance exactly the addressed Run one ordinary motion.
fn step<W: Write>(
    args: &[String],
    roots: &crate::root::Roots,
    writer: &mut W,
) -> Result<i32, CliError> {
    let command = "step";
    let (rest, reviews, approve, approval_count) = split_authorization_args(command, args)?;
    let id = addressed_run_with_roots(command, &rest, roots)?;
    // WRA-001: the authorization options address a review policy. On a Run
    // with none recorded they address nothing, which is an invocation
    // refusal, never a motion refusal.
    if (!reviews.is_empty() || approve.is_some()) && !run_is_governed(roots, &id) {
        return Err(CliError::refusal(
            "step: --review and --approve address no declared review policy",
        ));
    }
    let mut scheduler = Scheduler::open_run_for_motion_with_roots(roots, &id)
        .map_err(|error| hard_error(command, error))?;
    let mut request = StepRequest::new("");
    request.reviews = reviews;
    request.approve = approve;
    request.approval_count = approval_count;
    let outcome = scheduler
        .step(request)
        .map_err(|error| step_state_error(&id, error))?;
    if let StepOutcome::Refused { .. } = &outcome {
        writeln!(writer, "rtm: {outcome}")?;
        if let Some(next) = crate::teach::step_refusal_next(&id, &outcome) {
            writeln!(writer, "{next}")?;
        }
    } else {
        let report = scheduler
            .status()
            .map_err(|error| hard_error("status", error))?;
        writer.write_all(report.state_prompt().as_str().as_bytes())?;
        writer.write_all(b"\n")?;
        if let Some(next) = crate::teach::status_next(&id, report.state.status) {
            writeln!(writer, "{next}")?;
        }
    }
    Ok(0)
}

/// WEB-006 (t-134): the machine-facing nested-invocation entry. It prints
/// nothing: a refusal names itself under the `endpoint:` prefix, success
/// speaks only through the receipt pipe.
fn endpoint<W: Write>(
    args: &[String],
    roots: &crate::root::Roots,
    _writer: &mut W,
) -> Result<i32, CliError> {
    let request = crate::endpoint::parse_arguments(args)?;
    crate::endpoint::run(request, roots)
}

/// A step motion error renders through the next-act table, except the
/// parent guard's pause-seam refusals, which keep the retry as their next
/// act: the pause left a resumable ledger state, not a Run defect.
fn step_state_error(id: &str, error: crate::state::StateError) -> CliError {
    let text = error.to_string();
    if text.contains("RATMAC_TEST_NESTED_PAUSE companion") || text.contains("injected nested pause")
    {
        return CliError::new(format!("step: {text}\nnext: rtm step --run {id}"));
    }
    hard_error("step", error)
}

fn unsupported(command: &str) -> CliError {
    CliError::new(format!("unsupported command or option: {command}"))
}

/// Report one addressed Run through the one root resolution used to open it.
fn status<W: Write>(
    args: &[String],
    roots: &crate::root::Roots,
    writer: &mut W,
) -> Result<(), CliError> {
    Scheduler::refuse_flat_residue_with_roots(roots)
        .map_err(|error| hard_error("status", error))?;
    let id = addressed_run_with_roots("status", args, roots)?;
    // WRS-006: a passed Run reads as history before any runbook-dependent
    // open, so a runbook that moved on cannot refuse or reword it.
    if let Some(history) =
        Scheduler::history_with_roots(roots, &id).map_err(|error| hard_error("status", error))?
    {
        writeln!(
            writer,
            "Engine root: {}",
            crate::root::displayed(roots.engine_root())
        )?;
        writeln!(writer, "{history}")?;
        return Ok(());
    }
    let scheduler =
        Scheduler::open_run_with_roots(roots, &id).map_err(|error| hard_error("status", error))?;
    let report = scheduler
        .status()
        .map_err(|error| hard_error("status", error))?;
    scheduler
        .write_engine_root(writer)
        .map_err(|error| CliError::new(format!("status: {error}")))?;
    writeln!(writer, "{report}")?;
    writer.write_all(report.state_prompt().as_str().as_bytes())?;
    writer.write_all(b"\n")?;
    if let Some(next) = crate::teach::status_next(&id, report.state.status) {
        writeln!(writer, "{next}")?;
    }
    // WRS-003: a reader reports a pending committed pause action and
    // repairs nothing.
    if crate::pause::has_pending_entry(&roots.engine_root().join("runs").join(&id)) {
        writeln!(
            writer,
            "Recovery pending: a committed pause action for this Run awaits reconciliation; \
             the next mutating command on this Run finishes it exactly once."
        )?;
    }
    // WRS-004: a reader reports a committed amendment whose history event
    // is missing and repairs nothing.
    if let Some((run_id, sequence)) = crate::recover::pending_amendment(
        roots.engine_root(),
        &roots.engine_root().join("runs").join(&id),
    ) {
        writeln!(
            writer,
            "Amendment pending: sequence {sequence} for this Run is committed with its history \
             event missing; a repeated rtm recover --run {run_id} reconciles it exactly once."
        )?;
    }
    Ok(())
}

/// PGE-006: the human-confirmed blocked route.
///
/// Every condition is checked before the first write, so a refusal leaves
/// Scheduler-owned files byte-identical.
fn hold<W: Write>(
    args: &[String],
    roots: &crate::root::Roots,
    writer: &mut W,
) -> Result<(), CliError> {
    let (split_rest, reviews, approve, approval_count) = split_authorization_args("hold", args)?;
    let args: &[String] = &split_rest;
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
                    let roster = roster_line(roots)?;
                    return Err(CliError::refusal(format!(
                        "hold: --run needs a run id; {}{}",
                        roster.line, roster.rows
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

    // WRA-001: on a roster-resolved Run with no recorded review policy the
    // authorization options address nothing - an invocation refusal before
    // the plan's own checks change it. A run id that resolves nowhere falls
    // through to the plan's own addressing refusals.
    let offered = !reviews.is_empty() || approve.is_some();
    // The roster is consulted only when authorization options are offered:
    // an ungoverned invocation logs exactly what it logs today, and the
    // roster read is an operation-log mark of its own.
    let id_on_roster = if offered {
        run.as_deref()
            .filter(|id| !id.is_empty())
            .map(|id| {
                Scheduler::run_roster_with_roots(roots)
                    .map(|roster| roster.iter().any(|entry| entry == id))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    } else {
        false
    };
    let governed = id_on_roster && run_is_governed(roots, run.as_deref().unwrap_or_default());
    if offered && id_on_roster && !governed {
        return Err(CliError::refusal(
            "hold: --review and --approve address no declared review policy",
        ));
    }
    let request = crate::blocked::HoldRequest {
        blocker,
        confirmation,
        run,
    };
    let approval_count = approval_count.max(usize::from(approve.is_some()));
    let options = crate::review::AuthorizationOptions {
        reviews,
        approve,
        approval_count,
    };
    let plan = if governed {
        crate::blocked::plan_authorized_hold_in(roots, &request, &options).map_err(hold_failure)?
    } else {
        crate::blocked::plan_hold_in(roots, &request).map_err(hold_failure)?
    };
    crate::blocked::apply_hold_in(roots, &plan).map_err(hold_failure)?;
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

/// WRS-003: render one hold refusal - an unchanged refusal keeps today's
/// family line; a committed action reports recovery pending instead, never
/// a false unchanged refusal.
fn hold_failure(refusal: crate::blocked::HoldRefusal) -> CliError {
    if refusal.committed {
        CliError::new(format!(
            "hold: committed with recovery pending: {}; a later mutating command finishes it \
             exactly once",
            refusal.reason
        ))
    } else {
        CliError::new(format!("hold refused; {refusal}"))
    }
}

/// WRS-003: render one resume refusal, exactly as the hold family renders
/// its own.
fn resume_failure(refusal: crate::blocked::ResumeRefusal) -> CliError {
    if refusal.committed {
        CliError::new(format!(
            "resume: committed with recovery pending: {}; a later mutating command finishes it \
             exactly once",
            refusal.reason
        ))
    } else {
        CliError::new(format!("resume refused; {refusal}"))
    }
}

/// WRS-003: the human-confirmed resume of a held Run.
///
/// Every condition is checked before the first write, so a refusal leaves
/// Scheduler-owned files byte-identical.
fn resume<W: Write>(
    args: &[String],
    roots: &crate::root::Roots,
    writer: &mut W,
) -> Result<(), CliError> {
    let mut blocker: Option<String> = None;
    let mut confirmation: Option<String> = None;
    let mut run: Option<String> = None;
    let mut resolution: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--run" => {
                let Some(value) = args
                    .get(index + 1)
                    .filter(|value| !value.starts_with("--"))
                    .cloned()
                else {
                    let roster = roster_line(roots)?;
                    return Err(CliError::refusal(format!(
                        "resume: --run needs a run id; {}{}",
                        roster.line, roster.rows
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
                                "resume: --blocker needs a reference beneath a declared root",
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
                            CliError::new("resume: --confirm needs the exact confirmation phrase")
                        })?,
                );
                index += 2;
            }
            "--resolution" => {
                resolution = Some(
                    args.get(index + 1)
                        .filter(|value| !value.starts_with("--"))
                        .cloned()
                        .ok_or_else(|| {
                            CliError::new("resume: --resolution needs the signature file's path")
                        })?,
                );
                index += 2;
            }
            other => {
                return Err(CliError::new(format!(
                    "resume: unexpected extra argument {other}"
                )))
            }
        }
    }

    // WRS-003: a resolution signature addresses nothing on a Run with no
    // recorded review policy - an invocation refusal before the plan's own
    // checks change it, mirroring hold's authorization options. A run id
    // that resolves nowhere falls through to the plan's addressing
    // refusals.
    let offered = resolution.is_some();
    let id_on_roster = if offered {
        run.as_deref()
            .filter(|id| !id.is_empty())
            .map(|id| {
                Scheduler::run_roster_with_roots(roots)
                    .map(|roster| roster.iter().any(|entry| entry == id))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    } else {
        false
    };
    let governed = id_on_roster && run_is_governed(roots, run.as_deref().unwrap_or_default());
    if offered && id_on_roster && !governed {
        return Err(CliError::refusal(
            "resume: --resolution addresses no declared review policy",
        ));
    }
    let request = crate::blocked::ResumeRequest {
        blocker,
        confirmation,
        run,
        resolution: if governed { resolution } else { None },
    };
    let plan = crate::blocked::plan_resume_in(roots, &request).map_err(|refusal| {
        if refusal.committed {
            CliError::new(format!(
                "resume: committed with recovery pending: {}; a later mutating command finishes \
                 it exactly once",
                refusal.reason
            ))
        } else {
            CliError::new(format!("resume refused; {refusal}"))
        }
    })?;
    crate::blocked::apply_resume_in(roots, &plan).map_err(resume_failure)?;
    writeln!(
        writer,
        "rtm: run {} resumed against {}; occurrence {} cleared",
        plan.run_id(),
        plan.blocker(),
        plan.occurrence()
    )?;
    writeln!(
        writer,
        "The Run executes again at the same State; later motion still requires its normal \
         fresh checks."
    )?;
    Ok(())
}

/// One path spelled with no `.` or `..` components: `.` drops out and
/// `..` cancels the component before it, so prefix tests answer for paths
/// whose parents do not exist to canonicalize.
fn lexical(path: &Path) -> std::path::PathBuf {
    let mut parts: Vec<std::ffi::OsString> = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if parts.pop().is_none() {
                    parts.push(std::ffi::OsString::from(".."));
                }
            }
            other => parts.push(other.as_os_str().to_owned()),
        }
    }
    let mut result = std::path::PathBuf::new();
    for part in parts {
        result.push(part);
    }
    result
}

/// WRS-004 (t-132): the human-confirmed rule fix recovery.
///
/// Every argument shape and check is verified before the first write, so a
/// refusal leaves Engine-owned files byte-identical; the one durable
/// interruption - an amendment committed with its history event pending -
/// reports the committed state and reconciles on the repeated command.
fn recover<W: Write>(
    args: &[String],
    roots: &crate::root::Roots,
    _invocation: &crate::root::Invocation,
    writer: &mut W,
) -> Result<i32, CliError> {
    let mut run: Option<String> = None;
    let mut decision: Option<String> = None;
    let mut enrollment: Option<String> = None;
    let mut confirmation: Option<String> = None;
    let mut rest: Vec<String> = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--run" => {
                let Some(value) = args
                    .get(index + 1)
                    .filter(|value| !value.starts_with("--"))
                    .cloned()
                else {
                    let roster = roster_line(roots)?;
                    return Err(CliError::refusal(format!(
                        "recover: --run needs a run id; {}{}",
                        roster.line, roster.rows
                    )));
                };
                run = Some(value);
                index += 2;
            }
            "--decision" => {
                decision = Some(
                    args.get(index + 1)
                        .filter(|value| !value.starts_with("--"))
                        .cloned()
                        .ok_or_else(|| {
                            CliError::new(
                                "recover: --decision needs a path to the signed recovery \
                                 decision",
                            )
                        })?,
                );
                index += 2;
            }
            "--enrollment" => {
                enrollment = Some(
                    args.get(index + 1)
                        .filter(|value| !value.starts_with("--"))
                        .cloned()
                        .ok_or_else(|| {
                            CliError::new(
                                "recover: --enrollment needs a path to the externally enrolled \
                                 authority",
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
                            CliError::new("recover: --confirm needs the exact confirmation phrase")
                        })?,
                );
                index += 2;
            }
            other => {
                rest.push(other.to_owned());
                index += 1;
            }
        }
    }
    if let Some(stray) = rest.first() {
        return Err(CliError::new(format!(
            "recover: unexpected extra argument {stray}"
        )));
    }
    let run = run.unwrap_or_default();
    // WRS-004: an --enrollment addresses a missing recovery trust only; a
    // governed Run carries its prior trusted policy.
    if enrollment.is_some() && run_is_governed(roots, &run) {
        return Err(CliError::refusal(
            "recover: --enrollment addresses no missing recovery trust",
        ));
    }
    let request = crate::recover::RecoverRequest {
        enrollment: enrollment.map(std::path::PathBuf::from),
        decision: decision.map(std::path::PathBuf::from),
        confirmation,
        run: Some(run.clone()),
    };
    let render = |failure: crate::recover::RecoverFailure| {
        if failure.committed() {
            CliError::new(format!("recover: {}", failure.detail()))
        } else if failure.hard_failure() {
            // Lock acquisition and wait failures keep today's hard
            // `<verb>: ...` family with its next line.
            hard_error(
                "recover",
                crate::state::StateError::new(failure.detail().to_string()),
            )
        } else {
            CliError::new(format!("recover refused; {}", failure.detail()))
        }
    };
    let plan = crate::recover::plan_recover_in(roots, &request).map_err(render)?;
    let applied = crate::recover::apply_recover_in(roots, &plan).map_err(render)?;
    writeln!(
        writer,
        "rtm: run {} amended at sequence {}; the corrected authority is effective",
        applied.run_id(),
        applied.sequence()
    )?;
    writeln!(
        writer,
        "The Run keeps its address, State, class, bindings, workspace, and original pins; \
         an amendment neither resumes a paused Run nor approves a transition."
    )?;
    Ok(0)
}

/// WRA-001: render the canonical intent of a governed Run's next motion.
///
/// The render is read-only for the Engine: it resolves the motion a
/// signature would authorize, derives its canonical bytes, and writes them
/// to the caller-named `--out`, which may never address inside the Engine
/// root.
fn intent<W: Write>(
    args: &[String],
    roots: &crate::root::Roots,
    _invocation: &crate::root::Invocation,
    writer: &mut W,
) -> Result<i32, CliError> {
    let refused = |detail: String| CliError::new(format!("intent refused; {detail}"));
    let mut reviews: Vec<(String, String)> = Vec::new();
    let review_pair = |value: &str| -> Result<(String, String), CliError> {
        let (label, signature) = value.split_once(':').ok_or_else(|| {
            CliError::new(format!(
                "intent: --review {value:?} is not shaped <label>:<signature file>"
            ))
        })?;
        if label.is_empty() || signature.is_empty() {
            return Err(CliError::new(format!(
                "intent: --review {value:?} names no label or no signature file"
            )));
        }
        Ok((label.to_owned(), signature.to_owned()))
    };
    let mut hold = false;
    let mut resume = false;
    let mut recover = false;
    let mut enrollment: Option<String> = None;
    let mut blocker: Option<String> = None;
    let mut out: Option<String> = None;
    let mut rest: Vec<String> = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--run" => {
                rest.push(args[index].clone());
                if let Some(value) = args.get(index + 1) {
                    rest.push(value.clone());
                }
                index += 2;
            }
            "--hold" => {
                hold = true;
                index += 1;
            }
            "--resume" => {
                resume = true;
                index += 1;
            }
            "--recover" => {
                recover = true;
                index += 1;
            }
            "--enrollment" => {
                enrollment = Some(
                    args.get(index + 1)
                        .filter(|value| !value.starts_with("--"))
                        .cloned()
                        .ok_or_else(|| {
                            CliError::new(
                                "intent: --enrollment needs a path to the externally enrolled \
                                 authority",
                            )
                        })?,
                );
                index += 2;
            }
            "--blocker" => {
                blocker = Some(
                    args.get(index + 1)
                        .filter(|value| !value.starts_with("--"))
                        .cloned()
                        .ok_or_else(|| {
                            CliError::new(
                                "intent: --blocker needs a reference beneath a declared root",
                            )
                        })?,
                );
                index += 2;
            }
            "--out" => {
                out = Some(
                    args.get(index + 1)
                        .filter(|value| !value.starts_with("--"))
                        .cloned()
                        .ok_or_else(|| {
                            CliError::new("intent: --out needs the intent file's path")
                        })?,
                );
                index += 2;
            }
            "--review" => {
                let Some(value) = args.get(index + 1).filter(|value| !value.starts_with("--"))
                else {
                    return Err(CliError::new(
                        "intent: --review needs <label>:<signature file>",
                    ));
                };
                reviews.push(review_pair(value)?);
                index += 2;
            }
            other if other.starts_with("--review=") => {
                reviews.push(review_pair(
                    other.strip_prefix("--review=").unwrap_or_default(),
                )?);
                index += 1;
            }
            other => {
                rest.push(other.to_owned());
                index += 1;
            }
        }
    }

    // WRS-003: the two intent purposes name different intents.
    if hold && resume {
        return Err(refused(
            "--hold and --resume name different intents; pass exactly one".to_owned(),
        ));
    }
    // WRS-004: the recover purpose is its own intent.
    if recover && hold {
        return Err(refused(
            "--recover and --hold name different intents; pass exactly one".to_owned(),
        ));
    }
    if recover && resume {
        return Err(refused(
            "--recover and --resume name different intents; pass exactly one".to_owned(),
        ));
    }
    if recover && !reviews.is_empty() {
        return Err(refused("--review addresses no recovery intent".to_owned()));
    }
    if !recover && enrollment.is_some() {
        return Err(refused(
            "--enrollment addresses no recovery intent".to_owned(),
        ));
    }
    let id = addressed_run_with_roots("intent", &rest, roots)?;
    // WRA-001: governance is recorded fact; an ungoverned Run has no
    // canonical intent to render - except the recover purpose, whose own
    // trust boundary answers for a legacy Run's external enrollment.
    if !recover && !run_is_governed(roots, &id) {
        return Err(refused(format!(
            "run {id} resolves in no review-governed class; the canonical intent exists \
             only under a declared review policy"
        )));
    }
    // WRS-004: the recover render is the one intent that opens a drifted
    // runbook: the recovery open never compares the recorded pin.
    let scheduler = if recover {
        Scheduler::open_run_for_recover_with_roots(roots, &id)
            .map_err(|error| hard_error("intent", error))?
    } else {
        Scheduler::open_run_for_motion_with_roots(roots, &id)
            .map_err(|error| hard_error("intent", error))?
    };
    let state = scheduler
        .load_state()
        .map_err(|error| hard_error("intent", error))?;
    if recover {
        // WRS-004: the recover intent renders for a live Run in any
        // non-terminal status, held included; its own family's refusals
        // answer the rest.
    } else if resume {
        // WRS-003: the resolution intent exists only for a held Run; the
        // refusal is the resume family's own not-held detail.
        if state.status != crate::model::Status::Blocked {
            return Err(refused(format!(
                "run {id} is not held (status {}); nothing to resume",
                state.status
            )));
        }
    } else if state.status == crate::model::Status::Passed {
        // Today's terminal refusal, in the guard's own shape, so the
        // render's refusals never invent a second wording.
        return Err(refused(
            crate::scheduler::guard_failure(
                "terminal",
                state.state.clone(),
                "run is terminal (status passed): no transition may proceed",
                "a live, non-terminal Run",
            )
            .to_string(),
        ));
    } else if state.status == crate::model::Status::Blocked {
        return Err(refused(format!(
            "run {id} is already blocked against {}",
            if state.blocker.is_empty() {
                "an unnamed blocker"
            } else {
                state.blocker.as_str()
            }
        )));
    }
    if hold && blocker.is_none() {
        return Err(refused(
            "hold has no blocker link: pass --blocker <reference beneath a declared root>"
                .to_owned(),
        ));
    }
    let Some(out_as_given) = out else {
        return Err(refused(
            "--out <path> is required; the canonical intent is written to the caller-named \
             file"
                .to_owned(),
        ));
    };
    // A relative `--out` resolves against the process's working
    // directory, like any file the caller names - never a silent
    // repository-relative rebasing.
    let candidate = std::path::Path::new(&out_as_given);
    let resolved = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        let cwd = std::env::current_dir().map_err(|error| {
            refused(format!(
                "--out {out_as_given:?} cannot be resolved against the process directory: \
                 {error}"
            ))
        })?;
        cwd.join(candidate)
    };
    let engine_root = roots.engine_root();
    // A relative `--out` is judged under both bases the caller may mean:
    // the process directory the write lands in, and the invoking checkout
    // the string may name. Either basis inside the Engine root, or an
    // existing directory under either, refuses.
    let checkout_basis = if candidate.is_absolute() {
        resolved.clone()
    } else {
        roots.invoking_checkout_root().join(candidate)
    };
    let inside_engine_root = |target: &Path| -> bool {
        let parent = target.parent().unwrap_or(Path::new("."));
        match (fs::canonicalize(parent), fs::canonicalize(engine_root)) {
            (Ok(parent), Ok(engine)) => parent.starts_with(engine),
            // A parent the host cannot canonicalize - one that does not
            // exist yet - still answers lexically, with `.` and `..`
            // components resolved first: `<root>/sub/../.ratmac/x.bin`
            // must not slip past the prefix test.
            _ => lexical(target).starts_with(lexical(engine_root)),
        }
    };
    if [&resolved, &checkout_basis]
        .iter()
        .any(|basis| inside_engine_root(basis))
    {
        return Err(refused(format!(
            "--out {out_as_given:?} resolves inside the Engine root; the canonical intent \
             never lands there"
        )));
    }
    if resolved.is_dir() || checkout_basis.is_dir() {
        return Err(refused(format!(
            "--out {out_as_given:?} names an existing directory; the canonical intent renders \
             to a file"
        )));
    }
    // The render runs under the Run lock, today's acquire-with-deadline, and
    // holds it through the --out publish: a peer motion cannot interleave.
    let run_lock = crate::lock::RunLock::acquire(engine_root, &id).map_err(|error| {
        // A lock wait names a live peer motion on this very run: the next
        // act is that run's status, exactly as the next-act table renders
        // run-scoped hard errors.
        let text = error.to_string();
        if text.starts_with("lock wait expired") {
            CliError::new(format!("intent: {text}\nnext: rtm status --run {id}"))
        } else {
            hard_error("intent", error)
        }
    })?;
    let blocker = blocker.unwrap_or_default();
    let (mut facts, _enrollment) = if recover {
        let inner = scheduler
            .recover_view(
                engine_root,
                &id,
                enrollment.as_deref().map(std::path::Path::new),
            )
            .map_err(|error| hard_error("intent", error))?;
        (inner.map_err(refused)?, None)
    } else {
        let outcome = if resume {
            scheduler
                .resume_view(engine_root, &id)
                .map_err(|error| hard_error("intent", error))?
        } else {
            scheduler
                .intent_view(engine_root, &id, hold, &blocker)
                .map_err(|error| hard_error("intent", error))?
        };
        let (facts, enrollment) = outcome.map_err(refused)?;
        (facts, Some(enrollment))
    };
    // With review signatures offered, the render binds their review-set
    // digest: that is the approval intent the Approver signs.
    // WRS-003: the resume purpose renders the canonical resolution
    // intent WRS-003 pins; its review-set digest stays empty, so
    // offered review signatures never bind it.
    if !reviews.is_empty() && !resume && !recover {
        let mut offered = Vec::new();
        for (label, signature) in &reviews {
            let bytes = fs::read(signature).map_err(|error| {
                hard_error(
                    "intent",
                    crate::state::StateError::new(format!(
                        "cannot read the review signature {signature:?}: {error}"
                    )),
                )
            })?;
            offered.push((label.clone(), bytes));
        }
        facts.review_set_digest = crate::review::review_set_digest(&offered);
    }
    let engine_identity = crate::review::engine_identity_sha256()
        .map_err(|detail| hard_error("intent", crate::state::StateError::new(detail)))?;
    let bytes = facts.canonical_bytes(&Scheduler::project_scope(engine_root), &engine_identity);
    // Publish through a same-directory temporary sibling and one rename, so
    // no reader ever sees a half-written intent file.
    let file_name = resolved
        .file_name()
        .map(crate::root::component)
        .unwrap_or_else(|| "intent".to_owned());
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_nanos())
        .unwrap_or_default();
    let temporary =
        resolved.with_file_name(format!(".{file_name}.tmp-{}-{stamp}", std::process::id()));
    if let Err(error) =
        fs::write(&temporary, &bytes).and_then(|()| fs::rename(&temporary, &resolved))
    {
        let _ = fs::remove_file(&temporary);
        return Err(hard_error(
            "intent",
            crate::state::StateError::new(format!("cannot write {out_as_given:?}: {error}")),
        ));
    }
    let digest = crate::review::sha256_hex(&bytes);
    // The echoed --out is the argument exactly as given in the displayed
    // spelling - the one renderer's separators, never canonicalized or
    // made absolute.
    let echoed = crate::root::displayed(std::path::Path::new(&out_as_given));
    writeln!(
        writer,
        "rtm: canonical {} intent for run {id} written to {echoed} (sha256 {digest})",
        facts.purpose
    )?;
    drop(run_lock);
    Ok(0)
}

/// PGE-007: safe human-confirmed Run abandonment.
///
/// The confirmation phrase is checked before the first write; the retirement
/// itself is all-or-nothing.
/// FDC-007: ordinary checked motion. No confirmation phrase; the Scheduler
/// checks the parent's State and the declared spawn before any write.
fn spawn<W: Write>(
    args: &[String],
    roots: &crate::root::Roots,
    writer: &mut W,
) -> Result<(), CliError> {
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
                    let roster = roster_line(roots)?;
                    return Err(CliError::refusal(format!(
                        "spawn: --run needs a run id; {}{}",
                        roster.line, roster.rows
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
        let roster = roster_line(roots)?;
        return Err(CliError::refusal(format!(
            "spawn requires --run <parent id>; {}{}",
            roster.line, roster.rows
        )));
    };
    let child = Scheduler::spawn_to_with_roots(
        roots,
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
fn respawn<W: Write>(
    args: &[String],
    roots: &crate::root::Roots,
    writer: &mut W,
) -> Result<(), CliError> {
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
                    let roster = roster_line(roots)?;
                    return Err(CliError::refusal(format!(
                        "respawn: --run needs a run id; {}{}",
                        roster.line, roster.rows
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
    let successor = Scheduler::respawn_with_roots(roots, &request)
        .map_err(|error| CliError::new(format!("respawn refused; {error}")))?;
    writeln!(
        writer,
        "rtm: run superseded; successor run {successor} minted at .ratmac/runs/{successor}/"
    )?;
    Ok(())
}

fn abandon<W: Write>(
    args: &[String],
    roots: &crate::root::Roots,
    writer: &mut W,
) -> Result<(), CliError> {
    let project_root = roots.named_checkout();
    let mut confirmation: Option<String> = None;
    let mut run: Option<String> = None;
    // WRS-007: the address resolves first, wherever --run sits, and is
    // validated before any phrase-specific hint, so every refusal below
    // names the phrase for the right target and never chooses a Run.
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--run" {
            let Some(value) = args
                .get(index + 1)
                .filter(|value| !value.starts_with("--") && !value.trim().is_empty())
                .cloned()
            else {
                // WRS-007: a missing or blank address teaches the addressed
                // usage and chooses no roster entry for the caller.
                let roster = roster_line(roots)?;
                return Err(CliError::refusal(format!(
                    "abandon: --run needs a run id; {}; a Run is retired only by its own phrase: {}{}",
                    roster.line,
                    crate::abandon::ADDRESSED_USAGE,
                    roster.rows
                )));
            };
            if run.is_some() {
                let roster = roster_line(roots)?;
                return Err(CliError::refusal(format!(
                    "abandon: --run given twice; address exactly one run; {}; a Run is retired only by its own phrase: {}{}",
                    roster.line,
                    crate::abandon::ADDRESSED_USAGE,
                    roster.rows
                )));
            }
            run = Some(value);
            index += 2;
        } else {
            index += 1;
        }
    }
    let target = || crate::abandon::resolve_target_in(roots, run.as_deref());
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--run" => {
                index += 2;
            }
            "--confirm" => {
                let Some(value) = args
                    .get(index + 1)
                    .filter(|value| !value.starts_with("--"))
                    .cloned()
                else {
                    return Err(CliError::new(match target() {
                        Ok(target) => format!(
                            "abandon: --confirm needs the exact confirmation phrase {:?}; {}",
                            target.phrase(project_root),
                            target.guidance(project_root)
                        ),
                        Err(refusal) => {
                            format!(
                                "abandon: --confirm needs the exact confirmation phrase; {refusal}"
                            )
                        }
                    }));
                };
                confirmation = Some(value);
                index += 2;
            }
            other => {
                let next = match target() {
                    Ok(target) => target.guidance(project_root),
                    Err(refusal) => refusal.to_string(),
                };
                return Err(CliError::new(format!(
                    "abandon: unsupported option {other}; the options are --run <id> and --confirm <phrase>; {next}"
                )));
            }
        }
    }

    let request = crate::abandon::AbandonRequest { confirmation, run };
    let plan = crate::abandon::plan_abandon_in(roots, &request)
        .map_err(|refusal| CliError::new(format!("abandon refused; {refusal}")))?;
    crate::abandon::apply_abandon_in(roots, &plan)
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
fn doctor<W: Write>(
    args: &[String],
    invocation: &crate::root::Invocation,
    writer: &mut W,
) -> Result<i32, CliError> {
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
        let roots = invocation.project(&crate::root::addressed_project_root(target));
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

    let roots = invocation.checkout();
    Scheduler::refuse_flat_residue_with_roots(&roots)
        .map_err(|error| CliError::new(format!("doctor: {error}")))?;
    let runbook_path = roots.machine_class_path();
    let mut diagnosis = crate::doctor::diagnose_with_roots(&runbook_path, &roots);
    // WRS-003: a reader reports a pending committed pause action and
    // repairs nothing - one warning finding per pending Run.
    diagnosis.extend(crate::doctor::pending_pause_findings(roots.engine_root()));
    diagnosis.extend(crate::doctor::pending_amendment_findings(
        roots.engine_root(),
    ));
    if json {
        write_findings(&diagnosis, true, writer)?;
        return Ok(diagnosis.exit_code());
    }

    environment_report(&roots, &diagnosis, writer)?;
    write_findings(&diagnosis, false, writer)?;
    Ok(diagnosis.exit_code())
}

/// AAL-002: write one runbook at a path that does not exist yet.
fn scaffold<W: Write>(
    args: &[String],
    invocation: &crate::root::Invocation,
    writer: &mut W,
) -> Result<i32, CliError> {
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
    let project = invocation.project(&crate::root::addressed_project_root(path));
    crate::scaffold::write_scaffold_in(path, &project)
        .map_err(|refusal| CliError::refusal(refusal.to_string()))?;
    writeln!(
        writer,
        "Wrote {}. Diagnose it with `rtm doctor --json {}`.",
        crate::root::displayed(path),
        crate::root::displayed(path)
    )?;
    Ok(0)
}

/// AOP-003: write one skill folder at a path that does not exist yet.
fn skill<W: Write>(
    args: &[String],
    invocation: &crate::root::Invocation,
    writer: &mut W,
) -> Result<i32, CliError> {
    const USAGE: &str = "skill takes exactly one path";
    let mut target: Option<&str> = None;
    for arg in args {
        if arg.starts_with('-') {
            return Err(CliError::refusal(format!(
                "skill: unknown option {arg:?}; {USAGE}"
            )));
        }
        if target.is_some() {
            return Err(CliError::refusal(format!(
                "skill: one path at a time; {USAGE}"
            )));
        }
        target = Some(arg);
    }
    let Some(target) = target else {
        return Err(CliError::refusal(format!("skill: no path given; {USAGE}")));
    };
    let path = Path::new(target);
    let project = invocation.project(&crate::root::addressed_project_root(path));
    crate::skill::write_skill_in(path, &project)
        .map_err(|refusal| CliError::refusal(refusal.to_string()))?;
    writeln!(
        writer,
        "Wrote {}. Its SKILL.md carries the identity stamp of the engine that wrote it.",
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

    // ECP-003: the running engine's own channel and provenance, stamped at
    // build time by the channel-aware bootstrap; a binary built without it
    // honestly reports none.
    let own = crate::pin::engine_identity();
    let (own_channel, own_source) = own
        .map(|identity| (identity.channel, identity.source_commit))
        .unwrap_or((None, None));
    writeln!(
        writer,
        "Engine provenance: channel={} source-commit={}",
        own_channel.as_deref().unwrap_or("none"),
        own_source.as_deref().unwrap_or("none")
    )?;

    // ECP-003: the channel/provenance row - what stable resolves to here,
    // offline, or why it refuses; then any live Run driven off-pin.
    let project_root = roots.invoking_checkout_root();
    match crate::channel::resolve_channel(project_root, "stable") {
        Ok(resolution) => writeln!(
            writer,
            "Engine channel: stable is {} at {}",
            resolution.edition, resolution.commit
        )?,
        Err(reason) => writeln!(writer, "Engine channel: {reason}")?,
    }
    for finding in crate::channel::live_run_findings(project_root, roots.engine_root()) {
        writeln!(writer, "Engine channel finding: {finding}")?;
    }

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
        let records = crate::roster::Records::read(engine_root, &roster);
        for id in &roster {
            let state_path = runs_dir.join(id).join("run.toml");
            let shown = format!(".ratmac/runs/{id}/run.toml");
            if state_path.is_file() {
                crate::observe::operation("record");
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
                let absence = match records.retirement(id) {
                    crate::roster::Retirement::Recorded => format!("run {id} is retired"),
                    crate::roster::Retirement::NotRecorded => {
                        format!("no retirement is recorded for run {id}")
                    }
                    crate::roster::Retirement::Unknown(reason) => {
                        format!("retirement unknown: {reason}")
                    }
                };
                writeln!(writer, "Run Record: {shown} (absent \u{2014} {absence})")?;
            }
        }
        // WRS-001: a spawn-ledger entry whose address never reached the
        // roster is a recorded, unadmitted successor. The doctor reports
        // the same facts its roster row renders - its recorded role and
        // its absence - lock-free and writing nothing.
        for (id, absence) in records.absent_recorded_children(&roster) {
            writeln!(
                writer,
                "Run Record: .ratmac/runs/{id}/run.toml (absent \u{2014} {absence})"
            )?;
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

#[cfg(test)]
mod lexical_tests {
    use super::lexical;
    use std::path::Path;

    #[test]
    fn dots_resolve_and_dotdots_cancel() {
        assert_eq!(lexical(Path::new("a/./b")), Path::new("a/b"));
        assert_eq!(lexical(Path::new("a/b/../c")), Path::new("a/c"));
        assert_eq!(
            lexical(Path::new("r/sub/../.ratmac/newdir/x.bin")),
            Path::new("r/.ratmac/newdir/x.bin")
        );
        assert_eq!(lexical(Path::new("../outside")), Path::new("../outside"));
    }
}
