//! WEB-006 (t-134): `rtm endpoint`, the child Engine protocol endpoint.
//!
//! The parent Engine launches this command in the child checkout with the
//! Engine-minted invocation address, the complete invocation lineage, and
//! the dedicated receipt pipe's write end. It admits at most one child Run
//! per invocation address - exactly the `rtm start` path in this checkout -
//! and writes one strict terminal receipt, only after persisting and
//! reading back the child Run's terminal fact, to the receipt pipe.

use crate::cli::CliError;
use crate::nested::ReceiptEnvelope;
use crate::root::{Displayed, Roots};
use std::fs;
use std::path::{Path, PathBuf};

/// The parsed command surface.
pub(crate) struct EndpointRequest {
    pub(crate) invocation: String,
    pub(crate) lineage: String,
    pub(crate) descriptor: u64,
}

/// Parse the three required options. Argument defects are the CLI layer's
/// exit 2 under the `endpoint:` prefix.
pub(crate) fn parse_arguments(args: &[String]) -> Result<EndpointRequest, CliError> {
    let value_of =
        |slot: &mut Option<String>, key: &str, value: Option<&String>| -> Result<(), CliError> {
            if slot.is_some() {
                return Err(CliError::refusal(format!("endpoint: {key} given twice")));
            }
            let Some(value) = value else {
                return Err(CliError::refusal(format!("endpoint: {key} needs a value")));
            };
            *slot = Some(value.clone());
            Ok(())
        };
    let mut invocation: Option<String> = None;
    let mut lineage: Option<String> = None;
    let mut descriptor: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--invocation" => {
                value_of(&mut invocation, "--invocation", args.get(index + 1))?;
                index += 2;
            }
            "--lineage" => {
                value_of(&mut lineage, "--lineage", args.get(index + 1))?;
                index += 2;
            }
            "--receipt-fd" => {
                value_of(&mut descriptor, "--receipt-fd", args.get(index + 1))?;
                index += 2;
            }
            other => {
                return Err(CliError::refusal(format!(
                    "endpoint: unexpected argument {other:?}"
                )));
            }
        }
    }
    let invocation = invocation
        .filter(|value| !value.is_empty())
        .ok_or_else(|| CliError::refusal("endpoint: --invocation is required"))?;
    let lineage = lineage
        .filter(|value| !value.is_empty())
        .ok_or_else(|| CliError::refusal("endpoint: --lineage is required"))?;
    let descriptor_text = descriptor
        .filter(|value| !value.is_empty())
        .ok_or_else(|| CliError::refusal("endpoint: --receipt-fd is required"))?;
    let descriptor = descriptor_text.parse::<u64>().map_err(|_| {
        CliError::refusal(format!(
            "endpoint: --receipt-fd {descriptor_text} is not a non-negative integer"
        ))
    })?;
    Ok(EndpointRequest {
        invocation,
        lineage,
        descriptor,
    })
}

/// One operational refusal under the endpoint prefix: exit 1, stdout empty.
fn refuse(detail: impl std::fmt::Display) -> CliError {
    CliError::new(format!("endpoint: {detail}"))
}

/// The canonical Engine root spelling: `fs::canonicalize`, forward slashes,
/// the verbatim prefix kept.
pub(crate) fn canonical_root(engine_root: &Path) -> Result<String, String> {
    let canonical = fs::canonicalize(engine_root)
        .map_err(|error| format!("cannot canonicalize {}: {error}", engine_root.displayed()))?;
    Ok(crate::root::displayed(canonical))
}

/// One parsed invocation record, the strict six-field shape.
struct InvocationRecord {
    #[allow(dead_code)]
    invocation: String,
    parent_root: String,
    program: String,
    run: String,
}

const RECORD_FIELDS: [&str; 6] = [
    "invocation",
    "parent-root",
    "program",
    "run",
    "class-pin",
    "engine",
];

fn parse_invocation_record(path: &Path) -> Result<InvocationRecord, String> {
    let shown = path.displayed();
    let source = fs::read_to_string(path)
        .map_err(|error| format!("the invocation record {shown} is unreadable: {error}"))?;
    let value: toml::Value = source
        .parse()
        .map_err(|error| format!("the invocation record {shown} is not valid TOML: {error}"))?;
    let table = value
        .as_table()
        .ok_or_else(|| format!("the invocation record {shown} must be a table"))?;
    for key in table.keys() {
        if !RECORD_FIELDS.contains(&key.as_str()) {
            return Err(format!(
                "the invocation record {shown} declares an unknown key {key:?}"
            ));
        }
    }
    let mut fields = Vec::with_capacity(RECORD_FIELDS.len());
    for key in RECORD_FIELDS {
        let text = table
            .get(key)
            .and_then(toml::Value::as_str)
            .ok_or_else(|| {
                format!("the invocation record {shown} is missing the non-empty string {key:?}")
            })?;
        // The four identity fields must name their facts; the two digests
        // may legitimately read empty in a repaired or planted record.
        if text.is_empty() && !matches!(key, "class-pin" | "engine") {
            return Err(format!(
                "the invocation record {shown} is missing the non-empty string {key:?}"
            ));
        }
        fields.push(text.to_owned());
    }
    let mut fields = fields.into_iter();
    let mut next = || fields.next().expect("six fields collected");
    Ok(InvocationRecord {
        invocation: next(),
        parent_root: next(),
        program: next(),
        run: next(),
    })
}

fn render_invocation_record(
    invocation: &str,
    parent_root: &str,
    program: &str,
    run: &str,
    class_pin: &str,
    engine: &str,
) -> String {
    format!(
        "invocation = {}\nparent-root = {}\nprogram = {}\nrun = {}\nclass-pin = {}\nengine = {}\n",
        crate::ledger::toml_string(invocation),
        crate::ledger::toml_string(parent_root),
        crate::ledger::toml_string(program),
        crate::ledger::toml_string(run),
        crate::ledger::toml_string(class_pin),
        crate::ledger::toml_string(engine)
    )
}

/// The highest-numbered archived authorization record's SHA-256, when a
/// governed child's terminal sequence is above zero.
fn authorization_digest(run_dir: &Path) -> String {
    let Ok(entries) = fs::read_dir(run_dir.join("authorizations")) else {
        return String::new();
    };
    let mut highest: Option<(u64, PathBuf)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if let Ok(number) = stem.parse::<u64>() {
            if highest.as_ref().is_none_or(|(prior, _)| number > *prior) {
                highest = Some((number, path));
            }
        }
    }
    highest
        .and_then(|(_, path)| crate::pin::sha256_file(&path).ok())
        .unwrap_or_default()
}

/// Count the child's transition-log records naming the Run.
fn terminal_sequence(engine_root: &Path, run_id: &str) -> usize {
    let Ok(source) = fs::read_to_string(engine_root.join("log.md")) else {
        return 0;
    };
    source.lines().filter(|line| line.contains(run_id)).count()
}

/// The engine identity digest this endpoint pins: its own executable's
/// SHA-256.
fn engine_digest() -> Result<String, CliError> {
    let exe = std::env::current_exe().map_err(|error| {
        refuse(format_args!(
            "cannot resolve the running executable: {error}"
        ))
    })?;
    crate::pin::sha256_file(&exe)
        .map_err(|error| refuse(format_args!("cannot hash the running executable: {error}")))
}

/// The driver the parent's declaration names for this program, read from
/// the parent's runbook at the penultimate lineage entry's canonical root.
/// The runbook is authored state, not runtime state: reading it is the one
/// way the endpoint can learn what the parent declared.
fn declared_driver(parent_root: &str, program: &str, checkout: &Path) -> Option<String> {
    let native = parent_root.replace('/', std::path::MAIN_SEPARATOR_STR);
    let engine_root = Path::new(&native);
    let source = fs::read_to_string(engine_root.join("ratmac.toml")).ok()?;
    let class = crate::machine::MachineClass::from_toml(&source).ok()?;
    let parent_checkout = engine_root.parent()?;
    let own = fs::canonicalize(checkout).ok()?;
    let mut exact_seen = false;
    let mut exact = None;
    let mut fallback = None;
    let mut consider = |guard: &crate::machine::GuardKind| {
        let crate::machine::GuardKind::NestedRun {
            program: declared,
            root,
            driver,
            ..
        } = guard
        else {
            return;
        };
        if declared != program {
            return;
        }
        // The guard whose declared root resolves to this very checkout is
        // the parent's declaration for this program here - driver or none,
        // it suppresses any other match; another match is only the
        // first-identity fallback.
        let resolved = crate::machine::resolve_nested_root(parent_checkout, root);
        if fs::canonicalize(&resolved).ok().as_deref() == Some(own.as_path()) {
            exact_seen = true;
            exact = driver.clone();
        } else if !exact_seen && fallback.is_none() {
            fallback = driver.clone();
        }
    };
    for definition in class.states().values() {
        for guard in definition.guards() {
            consider(guard);
        }
    }
    for child_class in class.classes().values() {
        for definition in child_class.states().values() {
            for guard in definition.guards() {
                consider(guard);
            }
        }
    }
    if exact_seen {
        exact
    } else {
        fallback
    }
}

/// The terminal fact, as the record reads it: `passed` or `failed` on a
/// present record; `abandoned` when the Run was retired while this endpoint
/// held its address.
struct TerminalFact {
    outcome: String,
}

/// One stable terminal read: an unguarded peek decides whether to wait at
/// all, then the fact and the terminal sequence are read under the child
/// Run's own lock - every transition holds that claim across its record
/// replace and its history append, so today's deliberate State-first order
/// can never yield a sequence counted before the terminal transition's own
/// history record exists.
///
/// A peek error falls through to the locked read (only a successful
/// non-terminal peek ends the wait), and a denied lock acquire is retried
/// on the wait cadence: Windows leaves a just-released lock path
/// delete-pending, and every open in that window fails with
/// ERROR_ACCESS_DENIED, so the child's own `rtm step` release can deny
/// this endpoint's poll. The retry is bounded by the lock module's
/// `WAIT_TIMEOUT` measured from the first denial in this call, so a
/// lasting denial still refuses with its last error instead of hanging
/// the unbounded terminal wait.
fn terminal_snapshot(
    engine_root: &Path,
    run_id: &str,
) -> Result<Option<(TerminalFact, usize)>, CliError> {
    if let Ok(None) = read_terminal(engine_root, run_id) {
        return Ok(None);
    }
    let mut deadline = None;
    let lock = loop {
        match crate::lock::RunLock::try_acquire(
            engine_root,
            run_id,
            crate::lock::TraceDomain::Silent,
        ) {
            Ok(Some(lock)) => break lock,
            // A transition holds the claim; its history append completes
            // first.
            Ok(None) => return Ok(None),
            Err(error) => {
                let deadline = deadline
                    .get_or_insert_with(|| std::time::Instant::now() + crate::lock::WAIT_TIMEOUT);
                if std::time::Instant::now() >= *deadline {
                    return Err(refuse(error.to_string()));
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
    };
    let terminal = read_terminal(engine_root, run_id)?;
    let sequence = terminal_sequence(engine_root, run_id);
    drop(lock);
    Ok(terminal.map(|fact| (fact, sequence)))
}

fn read_terminal(engine_root: &Path, run_id: &str) -> Result<Option<TerminalFact>, CliError> {
    let record = engine_root.join("runs").join(run_id).join("run.toml");
    match fs::metadata(&record) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // A retired child keeps only its spawn-ledger; the invocation
            // record is gone with the rest. The named resumable outcome is
            // the abandoned receipt carrying the id recorded at admission.
            return Ok(Some(TerminalFact {
                outcome: "abandoned".to_owned(),
            }));
        }
        Err(error) => {
            return Err(refuse(format_args!(
                "cannot inspect the child Run Record {}: {error}",
                record.displayed()
            )))
        }
    }
    let state = crate::state::StateStore::for_engine_root(engine_root, run_id)
        .load()
        .map_err(|error| refuse(error.to_string()))?;
    match state.status {
        crate::model::Status::Passed => Ok(Some(TerminalFact {
            outcome: "passed".to_owned(),
        })),
        crate::model::Status::Failed => Ok(Some(TerminalFact {
            outcome: "failed".to_owned(),
        })),
        _ => Ok(None),
    }
}

/// Launch the declared driver in the child checkout and hold its verdict to
/// the terminal rule: a driver that exits without a terminal child Run
/// refuses, its output bounded.
fn launch_driver(
    checkout: &Path,
    driver: &str,
    engine_root: &Path,
    run_id: &str,
) -> Result<(), CliError> {
    let resolved = crate::pin::resolve_program(checkout, driver).map_err(|reason| {
        refuse(format_args!(
            "the declared driver {driver:?} is not pinnable: {reason}"
        ))
    })?;
    let mut driver = std::process::Command::new(&resolved);
    driver
        .current_dir(checkout)
        .env_remove(crate::diag::TRACE_VARIABLE);
    crate::diag::nested_child_stdio(&mut driver);
    let output = driver
        .output()
        .map_err(|error| refuse(format_args!("cannot launch the declared driver: {error}")))?;
    let code = output.status.code().unwrap_or(-1);
    let state = crate::state::StateStore::for_engine_root(engine_root, run_id)
        .load()
        .map_err(|error| refuse(error.to_string()))?;
    let terminal = matches!(
        state.status,
        crate::model::Status::Passed | crate::model::Status::Failed
    );
    if code == 0 && terminal {
        return Ok(());
    }
    Err(refuse(format_args!(
        "the driver exited {code} while run {run_id} is {} at {}; {}",
        state.status,
        state.state,
        crate::scheduler::bounded_diagnostic(&output.stderr, &output.stdout)
    )))
}

/// The bytes this endpoint writes to the pipe: the honest envelope, unless
/// the QA-only fault seam replaces it.
fn faulted_receipt(envelope: &ReceiptEnvelope) -> Vec<u8> {
    #[cfg(feature = "test-fault-injection")]
    if let Ok(fault) = std::env::var("RATMAC_TEST_ENDPOINT_FAULT") {
        let honest = envelope.encode();
        return match fault.as_str() {
            "receipt-none" => Vec::new(),
            "receipt-partial" => honest[..honest.len() / 2].to_vec(),
            "receipt-duplicate" => {
                let mut doubled = honest.clone();
                doubled.extend_from_slice(&honest);
                doubled
            }
            _ => honest,
        };
    }
    #[cfg(not(feature = "test-fault-injection"))]
    let _ = envelope;
    envelope.encode()
}

/// One admitted-or-resumed child Run.
struct Admitted {
    run_id: String,
    governed: bool,
    resumed: bool,
    class_pin: String,
    engine: String,
}

/// Run the endpoint. Every operational refusal is exit 1 with stdout empty;
/// success writes the receipt, prints nothing, and exits 0.
pub(crate) fn run(request: EndpointRequest, roots: &Roots) -> Result<i32, CliError> {
    // 1. Agreement.
    let inherited = match std::env::var("RATMAC_LINEAGE") {
        Ok(value) => value,
        Err(_) => {
            return Err(refuse(
                "RATMAC_LINEAGE is absent; a nested endpoint requires the inherited lineage",
            ))
        }
    };
    if inherited != request.lineage {
        return Err(refuse(
            "the inherited RATMAC_LINEAGE disagrees with the protocol lineage; refusing before admission",
        ));
    }

    // 2. Lineage - the checkout's own declaration is needed to judge the
    // chain's last entry, so the runbook parse precedes the grammar check.
    let checkout = roots.invoking_checkout_root();
    let engine_root = roots.engine_root();
    let class = crate::machine::MachineClass::load_from_checkout(checkout).map_err(|error| {
        refuse(format_args!(
            "parse .ratmac/ratmac.toml [{}]: {error}",
            error.code()
        ))
    })?;
    let own_root = canonical_root(engine_root).map_err(refuse)?;
    let own_program = class.program().map(str::to_owned).ok_or_else(|| {
        refuse("this checkout declares no program identity; a nested endpoint requires one")
    })?;
    let chain = crate::lineage::parse_carrier(&request.lineage)
        .map_err(|defect| refuse(format_args!("the lineage is malformed: {defect}")))?;
    // The same bound the parent guard applies, over the whole observed
    // clause: the named entry and the chain share one diagnostic bound, so
    // a hostile lineage - wide identities included - can never bloat a
    // refusal, here or there.
    let chain_text = crate::lineage::display(&chain);
    let bounded = |observed: String| crate::Scheduler::bounded_observed(&observed);
    if let Some(entry) = crate::lineage::repeated_identity(&chain) {
        return Err(refuse(format_args!(
            "observed {}; expected every identity distinct",
            bounded(format!(
                "lineage repeats the declared program identity {:?} ({chain_text})",
                entry.identity
            ))
        )));
    }
    if let Some(entry) = crate::lineage::repeated_root(&chain) {
        return Err(refuse(format_args!(
            "observed {}; expected every root distinct",
            bounded(format!(
                "lineage repeats the canonical Engine root {} ({chain_text})",
                entry.root
            ))
        )));
    }
    if chain.len() > crate::lineage::DEFAULT_MAX_DEPTH {
        return Err(refuse(format_args!(
            "observed {}; expected depth at most {}",
            bounded(format!(
                "lineage depth {} exceeds the permitted maximum {} ({chain_text})",
                chain.len(),
                crate::lineage::DEFAULT_MAX_DEPTH
            )),
            crate::lineage::DEFAULT_MAX_DEPTH
        )));
    }
    let own = chain
        .last()
        .expect("a parsed chain holds at least two entries");
    if own.identity != own_program || own.root != own_root {
        return Err(refuse(format_args!(
            "observed {}; expected this checkout's {}@{}",
            crate::Scheduler::bounded_observed(&format!(
                "lineage entry {}@{}",
                own.identity, own.root
            )),
            own_program,
            own_root
        )));
    }
    let parent_root = chain[chain.len() - 2].root.clone();

    // 3. Admission, exactly the rtm start path in this checkout. The pause
    // companions are validated before any admission write, and the pipe is
    // secluded before anything launches - no driver, or any descendant of
    // one, can receive it.
    crate::nested::validate_pause_companions(&crate::nested::ENDPOINT_PAUSE)
        .map_err(|detail| CliError::new(format!("endpoint: {detail}")))?;
    nested_seclude(request.descriptor)?;
    let policy = crate::ignore_policy::install(checkout, engine_root)
        .map_err(|refusal| refuse(refusal.to_string()))?;
    let admitted = match admit(&request, checkout, engine_root, &parent_root) {
        Ok(admitted) => admitted,
        Err(error) => {
            policy.rollback.rollback();
            return Err(error);
        }
    };
    crate::nested::hold_pause_if_requested(&crate::nested::ENDPOINT_PAUSE, "after-admit")
        .map_err(|detail| CliError::new(format!("endpoint: {detail}")))?;

    // 5. The driver: launched on a fresh admission whenever the parent's
    // declaration names one, and on resume only while the Run is not yet
    // terminal.
    let mut snapshot = terminal_snapshot(engine_root, &admitted.run_id)?;
    if snapshot.is_none() || !admitted.resumed {
        if let Some(driver) = declared_driver(&parent_root, &own_program, checkout) {
            launch_driver(checkout, &driver, engine_root, &admitted.run_id)?;
            snapshot = terminal_snapshot(engine_root, &admitted.run_id)?;
        }
    }

    // 6. The terminal fact.
    let (terminal, sequence) = match snapshot {
        Some(snapshot) => snapshot,
        None => loop {
            std::thread::sleep(std::time::Duration::from_millis(20));
            match terminal_snapshot(engine_root, &admitted.run_id)? {
                Some(snapshot) => break snapshot,
                None => continue,
            }
        },
    };

    // 7. Read back, then emit.
    crate::nested::hold_pause_if_requested(&crate::nested::ENDPOINT_PAUSE, "after-terminal")
        .map_err(|detail| CliError::new(format!("endpoint: {detail}")))?;
    let sequence = sequence.to_string();
    let (policy_digest, enrollment_digest) = if admitted.governed {
        let declaration = class
            .review()
            .expect("a governed admission parsed its declaration");
        let enrollment = crate::review::read_enrollment(
            engine_root,
            declaration,
            &crate::Scheduler::project_scope(engine_root),
        )
        .map_err(refuse)?;
        (
            declaration.policy_digest().to_owned(),
            enrollment.manifest_sha256,
        )
    } else {
        (String::new(), String::new())
    };
    let authorization = if admitted.governed && terminal.outcome != "abandoned" && sequence != "0" {
        authorization_digest(&crate::Scheduler::runs_dir_at(engine_root).join(&admitted.run_id))
    } else {
        String::new()
    };
    let envelope = ReceiptEnvelope {
        invocation: request.invocation.clone(),
        program: own_program,
        root: own_root,
        run: admitted.run_id.clone(),
        class_pin: admitted.class_pin.clone(),
        engine: admitted.engine.clone(),
        terminal_sequence: sequence,
        outcome: terminal.outcome,
        governance: if admitted.governed {
            "governed"
        } else {
            "ungoverned"
        }
        .to_owned(),
        policy_digest,
        enrollment_digest,
        authorization_digest: authorization,
    };
    let bytes = faulted_receipt(&envelope);
    crate::nested::write_receipt(request.descriptor, &bytes)
        .map_err(|error| refuse(format_args!("cannot write the receipt: {error}")))?;
    crate::nested::hold_pause_if_requested(&crate::nested::ENDPOINT_PAUSE, "after-receipt")
        .map_err(|detail| CliError::new(format!("endpoint: {detail}")))?;
    Ok(0)
}

fn nested_seclude(descriptor: u64) -> Result<(), CliError> {
    crate::nested::seclude_receipt_descriptor(descriptor).map_err(|error| {
        refuse(format_args!(
            "the receipt descriptor cannot be secluded: {error}"
        ))
    })
}

/// Admit or resume the child Run for this invocation address, writing the
/// artifacts in the restartable order: the invocation record first, the
/// remaining admission artifacts, and `run.toml` last.
fn admit(
    request: &EndpointRequest,
    checkout: &Path,
    engine_root: &Path,
    parent_root: &str,
) -> Result<Admitted, CliError> {
    // The start path: runbook parse and pin.
    let snapshot = crate::Scheduler::load_runbook_snapshot(checkout, checkout, engine_root)
        .map_err(|error| refuse(error.to_string()))?;
    let class = snapshot.class();
    let declaration = class.review().cloned();
    let governed = declaration.is_some();
    // The t-129 enrollment boundary, before any mint.
    if let Some(declaration) = declaration.as_ref() {
        crate::Scheduler::require_admission_origin(engine_root, declaration).map_err(|detail| {
            refuse(format_args!(
                "the class \"runbook\" declares a review policy; admission requires its \
                 enrolled trust origin: {detail}"
            ))
        })?;
    }
    let own_program = class.program().map(str::to_owned).ok_or_else(|| {
        refuse("this checkout declares no program identity; a nested endpoint requires one")
    })?;
    let engine = engine_digest()?;

    // Scan the roster strictly for this address's record.
    let mut found: Option<InvocationRecord> = None;
    let mut second: Option<String> = None;
    for run_id in crate::Scheduler::run_roster_at(engine_root) {
        let path = crate::Scheduler::runs_dir_at(engine_root)
            .join(&run_id)
            .join("invocation.toml");
        if !path.is_file() {
            continue;
        }
        let record = parse_invocation_record(&path).map_err(refuse)?;
        if record.invocation == request.invocation && record.parent_root == parent_root {
            if found.is_some() {
                second = Some(run_id);
                break;
            }
            found = Some(record);
        }
    }
    if second.is_some() {
        return Err(refuse(format_args!(
            "two Runs record the invocation {}; repair the records, then retry",
            request.invocation
        )));
    }

    // The declared driver, resolved and pinned like a gate artifact, joins
    // the deterministic artifact set before any write.
    let driver = declared_driver(parent_root, &own_program, checkout);

    // The initial Run Record the start path computes.
    let machine = crate::Scheduler::graph_of(class);
    let state =
        crate::Scheduler::initial_state_of(&machine).map_err(|error| refuse(error.to_string()))?;
    let initial_status = if machine.has_ordinary_outgoing(state.as_str()) {
        crate::model::Status::Planned
    } else {
        crate::model::Status::Passed
    };
    let goal_baseline = match snapshot.roots().resolve("goal") {
        Ok(goal) => crate::goal::revision(&goal)
            .map_err(|error| refuse(format_args!("read declared goal root: {error}")))?,
        Err(_) => None,
    };

    // Reserve or reuse the Run under the child root's own locks.
    let root_lock =
        crate::lock::RootLock::acquire(engine_root).map_err(|error| refuse(error.to_string()))?;
    // Exactly the start path after its root lock: recheck the inexpensive
    // flat-residue fact, then prove the runbook has not moved since it was
    // parsed, before minting changes the roster.
    crate::Scheduler::start_admission_rechecks(&snapshot, checkout, engine_root)
        .map_err(|error| refuse(error.to_string()))?;
    let run_id = match found.as_ref() {
        Some(record) => record.run.clone(),
        None => {
            let log_path = crate::Scheduler::transition_log_path(engine_root)
                .map_err(|error| refuse(error.to_string()))?;
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
                .map_err(|error| refuse(format_args!("open log.md: {error}")))?;
            crate::mint::next(engine_root).map_err(|error| refuse(error.to_string()))?
        }
    };
    let run_dir = crate::Scheduler::runs_dir_at(engine_root).join(&run_id);
    // A resumed Run can meet a lingering peer endpoint's release window -
    // on Windows the just-released lock path is delete-pending and every
    // open fails ERROR_ACCESS_DENIED - so a denied acquire retries on the
    // lock module's wait timeout, bound measured from the first denial,
    // before refusing with the last error.
    let mut deadline = None;
    let run_lock = loop {
        match crate::lock::RunLock::try_acquire(
            engine_root,
            &run_id,
            crate::lock::TraceDomain::Silent,
        ) {
            Ok(Some(lock)) => break lock,
            Ok(None) => {
                return Err(refuse(format_args!(
                    "the child Run lock for {run_id} is held; retry once it frees"
                )))
            }
            Err(error) => {
                let deadline = deadline
                    .get_or_insert_with(|| std::time::Instant::now() + crate::lock::WAIT_TIMEOUT);
                if std::time::Instant::now() >= *deadline {
                    return Err(refuse(error.to_string()));
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
    };
    if found.is_none() {
        if run_dir.exists() {
            return Err(refuse(format_args!(
                "the minted address {run_id} is already occupied"
            )));
        }
        // `create_dir_all` brings `runs/` into being, exactly as the
        // start path's directory walk does; the Run directory itself is
        // then created directly and filled in the restartable order below,
        // so an interruption leaves at most an artifact-less directory a
        // retry completes.
        fs::create_dir_all(crate::Scheduler::runs_dir_at(engine_root))
            .map_err(|error| refuse(format_args!("create the runs directory: {error}")))?;
    }

    // A resumed record must still name this checkout's declared identity.
    if let Some(record) = found.as_ref() {
        if record.program != own_program {
            return Err(refuse(format_args!(
                "the launched program declares identity {own_program:?}; the invocation declares {:?}",
                record.program
            )));
        }
    }

    // The deterministic artifact set, in the restartable order.
    let mut evidence = crate::pin::Evidence::load(&run_dir);
    if let Some(identity) = crate::pin::engine_identity() {
        evidence.set_engine(identity);
    }
    if let Some(driver) = driver.as_deref() {
        pin_driver(checkout, driver, &mut evidence)?;
    }
    evidence.goal_baseline = goal_baseline;
    evidence.goal_frozen = None;
    evidence.runbook_sha256 = Some(snapshot.pin().to_owned());
    let record = crate::model::RunState {
        state: state.as_str().to_owned(),
        status: initial_status,
        goal_revision: String::new(),
        input_revision: String::new(),
        output_revision: String::new(),
        active_refs: Vec::new(),
        blocker: String::new(),
    };
    let invocation_record = render_invocation_record(
        &request.invocation,
        parent_root,
        &own_program,
        &run_id,
        snapshot.pin(),
        &engine,
    )
    .into_bytes();
    let evidence_bytes = evidence.render().into_bytes();
    let governance_bytes = declaration
        .as_ref()
        .map(|declared| declared.governance_bytes())
        .unwrap_or_default();
    let record_bytes =
        crate::state::StateStore::serialize(&record).map_err(|error| refuse(error.to_string()))?;

    root_lock
        .ensure_current()
        .map_err(|error| refuse(error.to_string()))?;
    run_lock
        .ensure_current()
        .map_err(|error| refuse(error.to_string()))?;
    if found.is_none() {
        // A fresh mint writes its artifacts directly, exactly as the
        // start path does: the invocation record first, `run.toml` last,
        // so an interrupted admission never leaves an admitted Run with
        // missing artifacts. A later creation failure removes the
        // half-made directory but deliberately leaves its reserved
        // ordinal.
        let admission = (|| -> Result<(), CliError> {
            fs::create_dir(&run_dir)
                .map_err(|error| refuse(format_args!("create run directory {run_id}: {error}")))?;
            // The invocation-to-Run binding is durable before any other
            // artifact.
            write_admission_artifact(&run_dir, "invocation.toml", &invocation_record)?;
            write_admission_artifact(&run_dir, "evidence.toml", &evidence_bytes)?;
            if governed {
                write_admission_artifact(
                    &run_dir,
                    crate::review::GOVERNANCE_FILE,
                    &governance_bytes,
                )?;
            }
            reserve_spawn_ledger(&run_dir, &run_id)?;
            root_lock
                .ensure_current()
                .map_err(|error| refuse(error.to_string()))?;
            run_lock
                .ensure_current()
                .map_err(|error| refuse(error.to_string()))?;
            write_admission_artifact(&run_dir, "run.toml", &record_bytes)
        })();
        if let Err(admission_error) = admission {
            // Mirror the start path's incomplete-mint cleanup: require both
            // locks still name this mutation before deleting anything, and
            // a cleanup failure is refused, never swallowed.
            let cleanup: Result<(), String> = (|| {
                root_lock
                    .ensure_current()
                    .map_err(|error| error.to_string())?;
                run_lock
                    .ensure_current()
                    .map_err(|error| error.to_string())?;
                fs::remove_dir_all(&run_dir).map_err(|error| {
                    format!(
                        "remove incomplete run directory {}: {error}",
                        run_dir.displayed()
                    )
                })
            })();
            if let Err(cleanup_error) = cleanup {
                return Err(refuse(format_args!(
                    "the admission failed ({admission_error}) and its cleanup could not \
                     complete: {cleanup_error}"
                )));
            }
            return Err(admission_error);
        }
    } else {
        // A resumed Run may already have advanced past its admission bytes;
        // the invocation-to-Run binding still proves it names this very
        // invocation, and only an interrupted admission (a missing
        // artifact) is completed, never overwritten.
        write_admission_artifact(&run_dir, "invocation.toml", &invocation_record)?;
        complete_missing_admission_artifacts(
            &run_dir,
            &evidence_bytes,
            governed.then_some(&governance_bytes),
            &record_bytes,
        )?;
        reserve_spawn_ledger(&run_dir, &run_id)?;
    }
    Ok(Admitted {
        run_id,
        governed,
        resumed: found.is_some(),
        class_pin: snapshot.pin().to_owned(),
        engine,
    })
}

/// Resolve and pin the declared driver like a gate artifact (ETB-001),
/// refusing a pinned driver whose bytes moved.
fn pin_driver(
    checkout: &Path,
    driver: &str,
    evidence: &mut crate::pin::Evidence,
) -> Result<(), CliError> {
    let resolved = crate::pin::resolve_program(checkout, driver).map_err(|reason| {
        refuse(format_args!(
            "the declared driver {driver:?} is not pinnable: {reason}"
        ))
    })?;
    let sha256 = crate::pin::sha256_file(&resolved)
        .map_err(|error| refuse(format_args!("hash the declared driver: {error}")))?;
    let identity = crate::pin::Identity {
        resolved: crate::root::displayed(&resolved),
        sha256,
        ..crate::pin::Identity::default()
    };
    match evidence.gate(driver) {
        Some(pinned) if *pinned == identity => {}
        Some(pinned) => {
            return Err(refuse(format_args!(
                "the declared driver {driver:?} hashes a different digest than the pinned one \
                 ({pinned}); refusing to launch it"
            )))
        }
        None => evidence.record_gate(driver, identity),
    }
    Ok(())
}

/// Keep a present byte-identical artifact, write a missing one, and refuse
/// on a conflicting byte, naming the path.
fn write_admission_artifact(run_dir: &Path, name: &str, expected: &[u8]) -> Result<(), CliError> {
    let path = run_dir.join(name);
    match fs::read(&path) {
        Ok(present) => {
            if present != expected {
                return Err(refuse(format_args!(
                    "admission cannot resume: {} holds bytes this admission did not write",
                    path.displayed()
                )));
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            crate::observe::operation("record");
            fs::write(&path, expected)
                .map_err(|error| refuse(format_args!("write {}: {error}", path.displayed())))
        }
        Err(error) => Err(refuse(format_args!("read {}: {error}", path.displayed()))),
    }
}

/// Complete an interrupted admission: write only the artifacts a prior
/// admission left missing, and never overwrite bytes a resumed Run - which
/// may have advanced - carries.
fn complete_missing_admission_artifacts(
    run_dir: &Path,
    evidence_bytes: &[u8],
    governance_bytes: Option<&[u8]>,
    record_bytes: &[u8],
) -> Result<(), CliError> {
    write_missing_artifact(run_dir, "evidence.toml", evidence_bytes)?;
    if let Some(bytes) = governance_bytes {
        write_missing_artifact(run_dir, crate::review::GOVERNANCE_FILE, bytes)?;
    }
    write_missing_artifact(run_dir, "run.toml", record_bytes)?;
    Ok(())
}

fn write_missing_artifact(run_dir: &Path, name: &str, expected: &[u8]) -> Result<(), CliError> {
    let path = run_dir.join(name);
    if path.exists() {
        return Ok(());
    }
    crate::observe::operation("record");
    fs::write(&path, expected)
        .map_err(|error| refuse(format_args!("write {}: {error}", path.displayed())))
}

/// Reserve the empty spawn-ledger path a fresh mint reserves. A present
/// ledger is this Run's own record of its children and is never rewritten.
fn reserve_spawn_ledger(run_dir: &Path, run_id: &str) -> Result<(), CliError> {
    let path = run_dir.join("spawn-ledger");
    if path.exists() {
        return Ok(());
    }
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map(|_| ())
        .map_err(|error| refuse(format_args!("reserve spawn-ledger under {run_id}: {error}")))
}
