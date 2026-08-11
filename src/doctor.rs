//! DRD-001..DRD-007: diagnosis as data.
//!
//! `rtm doctor` reports one **finding** per defect - a stable code, a
//! severity, a location, and a message - so an agent can repair a runbook
//! from the report instead of guessing at prose. The codes are tabled in the
//! runbook specification; this module emits them and documents nothing.
//!
//! Four passes run in order, and each one only runs on what the pass before
//! it produced:
//!
//! 1. **Parse** - through `MachineClass`, the one reader (TRP-001). A refusal
//!    is a finding carrying the parser's own code, location, and message; the
//!    later passes need a parsed class, so a refusal ends the diagnosis.
//! 2. **Graph** - entry, reachability, and edge shape.
//! 3. **Guard lint** - what a guard's verdict actually rests on.
//! 4. **Ownership** - PGE-004, through the existing audit.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::machine::{GuardKind, MachineClass};

/// How much a finding matters. Errors make a runbook unusable; warnings make
/// it weaker than it looks.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Severity {
    Warning,
    Error,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One defect, named the same way every time.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Finding {
    code: &'static str,
    severity: Severity,
    location: String,
    message: String,
}

impl Finding {
    fn new(
        code: &'static str,
        severity: Severity,
        location: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        // ENS-010: a finding is not normalized here. A message may name a
        // runbook identifier that legitimately contains a backslash, and a
        // report that rewrote it could not be matched back to the runbook.
        // Whoever renders a path into a message calls `root::displayed`.
        Self {
            code,
            severity,
            location: location.into(),
            message: message.into(),
        }
    }

    fn error(code: &'static str, location: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(code, Severity::Error, location, message)
    }

    fn warning(
        code: &'static str,
        location: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(code, Severity::Warning, location, message)
    }

    /// The `RB*` code tabled in the runbook specification.
    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// Where to go to fix it.
    pub fn location(&self) -> &str {
        &self.location
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {}: {}: {}",
            self.code, self.severity, self.location, self.message
        )
    }
}

/// Findings paired with the exact Engine root used to diagnose them.
pub(crate) struct Diagnosis {
    findings: Vec<Finding>,
    engine_root: PathBuf,
}

impl Diagnosis {
    /// Render this diagnosis's selected Engine root for a human report.
    pub(crate) fn write_engine_root<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writeln!(
            writer,
            "Engine root: {}",
            crate::root::displayed(&self.engine_root)
        )
    }

    /// Render this diagnosis's findings as the human-readable report.
    pub(crate) fn render_report(&self) -> String {
        let mut out = String::new();
        for finding in &self.findings {
            out.push_str(&finding.to_string());
            out.push('\n');
        }
        if self.findings.is_empty() {
            out.push_str("No findings.\n");
        }
        out
    }

    /// Render this diagnosis's Engine root and findings as JSON.
    pub(crate) fn render_json(&self) -> String {
        let engine_root = crate::root::displayed(&self.engine_root);
        let mut out = format!(
            "{{\n  \"engine_root\": {},\n  \"findings\": [",
            quote(&engine_root)
        );
        for (index, finding) in self.findings.iter().enumerate() {
            out.push_str(if index == 0 { "\n" } else { ",\n" });
            out.push_str(&format!(
                "    {{\"code\": \"{}\", \"severity\": \"{}\", \"location\": {}, \"message\": {}}}",
                finding.code(),
                finding.severity(),
                quote(finding.location()),
                quote(finding.message())
            ));
        }
        if !self.findings.is_empty() {
            out.push('\n');
            out.push_str("  ");
        }
        out.push_str("]\n}\n");
        out
    }

    pub(crate) fn exit_code(&self) -> i32 {
        exit_code(&self.findings)
    }
}

/// Diagnose a runbook through the roots selected for its addressed project.
pub fn diagnose(path: &Path) -> Vec<Finding> {
    let project_root = crate::root::addressed_project_root(path);
    let roots = crate::root::resolve(&project_root);
    diagnose_with_roots(path, &roots).findings
}

/// Diagnose a runbook through roots already selected by the caller.
pub(crate) fn diagnose_with_roots(path: &Path, roots: &crate::root::Roots) -> Diagnosis {
    let shown = crate::root::displayed(path);
    if let Err(error) = crate::Scheduler::refuse_flat_residue_with_roots(roots) {
        return Diagnosis {
            findings: vec![Finding::error(
                error.code().unwrap_or("RB101"),
                shown.clone(),
                error.to_string(),
            )],
            engine_root: roots.engine_root().to_path_buf(),
        };
    }
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            return Diagnosis {
                findings: vec![Finding::error(
                    "RB101",
                    shown.clone(),
                    format!("cannot read {shown}: {error}"),
                )],
                engine_root: roots.engine_root().to_path_buf(),
            }
        }
    };
    let class = match MachineClass::from_toml(&source) {
        Ok(class) => class,
        // DRD-007: the parser named the defect; the doctor relays it rather
        // than re-classifying prose it did not produce.
        Err(error) => {
            return Diagnosis {
                findings: vec![Finding::error(
                    error.code(),
                    error.location().to_owned(),
                    error.message().to_owned(),
                )],
                engine_root: roots.engine_root().to_path_buf(),
            }
        }
    };

    let mut findings = Vec::new();
    if runbook_workspace(path).is_some() {
        if let Err(error) =
            class.validate_roots(roots.invoking_checkout_root(), roots.engine_root())
        {
            findings.push(Finding::error(
                error.code(),
                format!("roots {:?}", error.role()),
                error.message().to_owned(),
            ));
        }
    }
    inspect_graph(&class, &mut findings);
    audit_termination(&class, &mut findings);
    lint_guards(&class, &mut findings);
    audit_ownership(&class, &shown, &mut findings);
    findings.sort();
    Diagnosis {
        findings,
        engine_root: roots.engine_root().to_path_buf(),
    }
}

/// Return a workspace only for the conventional tracked runbook location.
/// A caller may diagnose an arbitrary standalone file, which has no
/// repository context in which named roots could be validated.
fn runbook_workspace(path: &Path) -> Option<std::path::PathBuf> {
    let directory = path.parent()?;
    if crate::root::component(path.file_name()?) != MachineClass::FILE_NAME
        || crate::root::component(directory.file_name()?) != ".ratmac"
    {
        return None;
    }
    directory.parent().map(Path::to_path_buf)
}

/// The process exit code for a report: `0` clean, `1` warnings only, `2` any
/// error (DRD-004).
pub fn exit_code(findings: &[Finding]) -> i32 {
    if findings
        .iter()
        .any(|finding| finding.severity() == Severity::Error)
    {
        2
    } else if findings.is_empty() {
        0
    } else {
        1
    }
}

/// JSON string escaping, by the letter of the grammar.
fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if (control as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// DRD-002: entry, reachability, and edge shape.
///
/// Only ordinary transitions carry routing: `rtm step` never takes a blocked
/// route (PGE-006), so a State reachable only through one is not reachable.
fn inspect_graph(class: &MachineClass, findings: &mut Vec<Finding>) {
    let states = class.states();
    if states.is_empty() {
        findings.push(Finding::error(
            "RB201",
            "states",
            "the runbook declares no State",
        ));
        return;
    }

    let ordinary = class
        .transitions()
        .iter()
        .filter(|transition| !transition.is_blocked_route())
        .collect::<Vec<_>>();

    let mut seen = BTreeSet::new();
    for transition in class.transitions() {
        let from = transition.from().as_str();
        let to = transition.to().as_str();
        let edge = (
            from.to_owned(),
            to.to_owned(),
            transition.input().map(str::to_owned),
            transition.is_blocked_route(),
        );
        let route = format!("transition {from:?} -> {to:?}");
        if !seen.insert(edge) {
            findings.push(Finding::warning(
                "RB206",
                route.clone(),
                "this edge is declared more than once; the duplicate adds no route",
            ));
        }
        if from == to {
            findings.push(Finding::warning(
                "RB207",
                route,
                "this transition leaves and enters the same State, so taking it makes no progress",
            ));
        }
    }

    let mut inbound = BTreeMap::new();
    let mut outbound: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for name in states.keys() {
        inbound.insert(name.as_str(), 0_usize);
        outbound.insert(name.as_str(), Vec::new());
    }
    for transition in &ordinary {
        let from = transition.from().as_str();
        let to = transition.to().as_str();
        if let Some(count) = inbound.get_mut(to) {
            *count += 1;
        }
        if let Some(targets) = outbound.get_mut(from) {
            targets.push(to);
        }
    }

    let initial = inbound
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(name, _)| *name)
        .collect::<Vec<_>>();
    match initial.len() {
        0 => findings.push(Finding::error(
            "RB202",
            "transitions",
            "no initial State: every State has an inbound transition, so a Run has nowhere to start",
        )),
        1 => {}
        _ => {
            for name in &initial {
                findings.push(Finding::error(
                    "RB203",
                    format!("phase {name:?}"),
                    format!(
                        "several initial States ({}); a Run starts in exactly one",
                        initial
                            .iter()
                            .map(|name| format!("{name:?}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
            }
        }
    }

    if let [entry] = initial.as_slice() {
        let mut reached = BTreeSet::new();
        let mut stack = vec![*entry];
        while let Some(name) = stack.pop() {
            if !reached.insert(name) {
                continue;
            }
            for next in outbound.get(name).into_iter().flatten() {
                stack.push(next);
            }
        }
        for name in states.keys() {
            if !reached.contains(name.as_str()) {
                findings.push(Finding::error(
                    "RB204",
                    format!("phase {name:?}"),
                    format!("unreachable from the initial State {entry:?}"),
                ));
            }
        }
    }

    let terminal = outbound
        .iter()
        .filter(|(_, targets)| targets.is_empty())
        .map(|(name, _)| *name)
        .collect::<Vec<_>>();
    if terminal.len() > 1 {
        for name in &terminal {
            findings.push(Finding::warning(
                "RB205",
                format!("phase {name:?}"),
                format!(
                    "one of {} terminal States; one ending is the ordinary shape, several usually mean a missing edge",
                    terminal.len()
                ),
            ));
        }
    }
}

/// FDC-008: cycle termination as guard-kind membership (RB214).
///
/// Every State on a cycle over ordinary edges must carry at least one
/// receipt-class (`sensitivity_receipts`, `completion_gate`) or
/// contract-class (`intake_contract`, `record_contract`) guard: that guard
/// gates the State's ordinary out-edges, so kind membership alone proves the
/// cycle can end. Nothing is executed, and a blocked route satisfies
/// nothing - `rtm step` never takes one.
fn audit_termination(class: &MachineClass, findings: &mut Vec<Finding>) {
    let graph = crate::graph::MachineGraph::new(
        class.states().keys().map(String::as_str),
        class.transitions().to_vec(),
    );
    for cycle in graph.ordinary_cycles() {
        let offenders = cycle
            .iter()
            .filter(|phase| {
                class
                    .states()
                    .get(phase.as_str())
                    .is_none_or(|definition| !definition.guards().iter().any(guard_terminates))
            })
            .map(|phase| format!("{:?}", phase.as_str()))
            .collect::<Vec<_>>();
        if offenders.is_empty() {
            continue;
        }
        let mut route = cycle
            .iter()
            .map(|phase| format!("{:?}", phase.as_str()))
            .collect::<Vec<_>>()
            .join(" -> ");
        route.push_str(" -> ");
        route.push_str(&format!("{:?}", cycle[0].as_str()));
        let (noun, verb) = if offenders.len() == 1 {
            ("State", "carries")
        } else {
            ("States", "carry")
        };
        findings.push(Finding::error(
            "RB214",
            format!("cycle {route}"),
            format!(
                "{noun} {} on this cycle {verb} no receipt- or contract-class \
                 guarded out-edge, so nothing statically proves the cycle \
                 terminates",
                offenders.join(", ")
            ),
        ));
    }
}

/// The guard-kind classes whose membership satisfies termination (FDC-008).
fn guard_terminates(kind: &GuardKind) -> bool {
    matches!(
        kind,
        GuardKind::SensitivityReceipts { .. }
            | GuardKind::CompletionGate { .. }
            | GuardKind::IntakeContract
            | GuardKind::RecordContract
    )
}

/// DRD-002: what a guard's verdict actually rests on.
fn lint_guards(class: &MachineClass, findings: &mut Vec<Finding>) {
    let root = Path::new(".");
    for (name, definition) in class.states() {
        for (index, guard) in definition.guards().iter().enumerate() {
            let location = format!("phase {name:?} guard {index}");
            match guard {
                GuardKind::CommandExit {
                    program, exempt, ..
                } => {
                    // ETB-001: a non-exempt gate command must be pinnable, and
                    // a command that cannot be resolved cannot be pinned.
                    if !exempt {
                        if let Err(reason) = crate::pin::resolve_program(root, program) {
                            findings.push(Finding::error(
                                "RB301",
                                location,
                                format!("{program:?} is not pinnable: {reason}"),
                            ));
                        }
                    }
                }
                GuardKind::FilesExact { path, .. } | GuardKind::FileContains { path, .. } => {
                    if !crate::ownership::is_scheduler_owned_path(path) {
                        findings.push(Finding::warning(
                            "RB302",
                            location,
                            format!(
                                "this guard's verdict rests on {path:?}, which the agent under test can write"
                            ),
                        ));
                    }
                }
                GuardKind::SensitivityReceipts { .. }
                | GuardKind::CompletionGate { .. }
                | GuardKind::IntakeContract
                | GuardKind::RecordContract
                | GuardKind::Join { .. } => {}
            }
        }
    }
}

/// DRD-003: the PGE-004 audit, reported like any other pass.
fn audit_ownership(class: &MachineClass, shown: &str, findings: &mut Vec<Finding>) {
    let instructions = crate::ownership::runbook_instructions(class, shown);
    if let Err(violations) = crate::ownership::audit_ownership(&instructions) {
        for violation in violations {
            findings.push(Finding::error(
                "RB401",
                violation.source.clone(),
                violation.to_string(),
            ));
        }
    }
}
