//! DRD-001..DRD-007: diagnosis as data.
//!
//! `rtm doctor` reports one **finding** per defect - a stable code, a
//! severity, a location, and a message - so an agent can repair a runbook
//! from the report instead of guessing at prose. The codes are tabled once,
//! in `.arca/runbook-spec.md`; this module emits them and documents nothing.
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
use std::path::Path;

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

    /// The `RB*` code tabled in `.arca/runbook-spec.md`.
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

/// Diagnose the runbook at `path`, reading it and nothing else.
///
/// The result is sorted, so two runs over the same bytes produce the same
/// report (DRD-006).
pub fn diagnose(path: &Path) -> Vec<Finding> {
    let shown = path.to_string_lossy().replace('\\', "/");
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            return vec![Finding::error(
                "RB101",
                shown.clone(),
                format!("cannot read {shown}: {error}"),
            )]
        }
    };
    let class = match MachineClass::from_toml(&source) {
        Ok(class) => class,
        // DRD-007: the parser named the defect; the doctor relays it rather
        // than re-classifying prose it did not produce.
        Err(error) => {
            return vec![Finding::error(
                error.code(),
                error.location().to_owned(),
                error.message().to_owned(),
            )]
        }
    };

    let mut findings = Vec::new();
    inspect_graph(&class, &mut findings);
    lint_guards(&class, &mut findings);
    audit_ownership(&class, &shown, &mut findings);
    findings.sort();
    findings
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

/// The human report: one line per finding, in the order they are held.
pub fn render_report(findings: &[Finding]) -> String {
    let mut out = String::new();
    for finding in findings {
        out.push_str(&finding.to_string());
        out.push('\n');
    }
    if findings.is_empty() {
        out.push_str("No findings.\n");
    }
    out
}

/// The machine-readable report (DRD-006): the same findings, as JSON.
pub fn render_json(findings: &[Finding]) -> String {
    let mut out = String::from("{\n  \"findings\": [");
    for (index, finding) in findings.iter().enumerate() {
        out.push_str(if index == 0 { "\n" } else { ",\n" });
        out.push_str(&format!(
            "    {{\"code\": \"{}\", \"severity\": \"{}\", \"location\": {}, \"message\": {}}}",
            finding.code(),
            finding.severity(),
            quote(finding.location()),
            quote(finding.message())
        ));
    }
    if !findings.is_empty() {
        out.push('\n');
        out.push_str("  ");
    }
    out.push_str("]\n}\n");
    out
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
/// route (PGE-006), so a Phase reachable only through one is not reachable.
fn inspect_graph(class: &MachineClass, findings: &mut Vec<Finding>) {
    let phases = class.phases();
    if phases.is_empty() {
        findings.push(Finding::error(
            "RB201",
            "phases",
            "the runbook declares no Phase",
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
                "this transition leaves and enters the same Phase, so taking it makes no progress",
            ));
        }
    }

    let mut inbound = BTreeMap::new();
    let mut outbound: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for name in phases.keys() {
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
            "no initial Phase: every Phase has an inbound transition, so a Run has nowhere to start",
        )),
        1 => {}
        _ => {
            for name in &initial {
                findings.push(Finding::error(
                    "RB203",
                    format!("phase {name:?}"),
                    format!(
                        "several initial Phases ({}); a Run starts in exactly one",
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
        for name in phases.keys() {
            if !reached.contains(name.as_str()) {
                findings.push(Finding::error(
                    "RB204",
                    format!("phase {name:?}"),
                    format!("unreachable from the initial Phase {entry:?}"),
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
                    "one of {} terminal Phases; one ending is the ordinary shape, several usually mean a missing edge",
                    terminal.len()
                ),
            ));
        }
    }
}

/// DRD-002: what a guard's verdict actually rests on.
fn lint_guards(class: &MachineClass, findings: &mut Vec<Finding>) {
    let root = Path::new(".");
    for (name, definition) in class.phases() {
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
