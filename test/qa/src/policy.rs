//! ORS-001: the caller policy audit.
//!
//! One policy, stated on every active caller-facing surface:
//! a human may invoke argument-free `rtm start`; the Main-Agent may invoke it
//! only after explicit human Run-start sign-off for the current target
//! project; a Subagent never invokes any `rtm` command.
//!
//! The audit is deliberately two-sided: a surface must state all three
//! clauses, and no surface may retain the superseded user-only or
//! blanket never-agent-start wording.

use std::fs;
use std::path::Path;

/// One caller-facing text the policy must appear on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicySurface {
    pub name: String,
    pub text: String,
}

/// A surface that fails the audit, and why.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyViolation {
    pub surface: String,
    pub reason: String,
}

/// Wording retired by ORS-001. Any occurrence is a violation.
pub const RETIRED_PHRASES: [&str; 6] = [
    "user-only",
    "user only",
    "never agent-initiated",
    "never agent initiated",
    "not agent-initiated",
    "agents must not start",
];

/// Read a surface from a repository file, resolving a pointer file to what it
/// points at: `AGENTS.md` is a one-line pointer, so its policy content is the
/// document it names.
pub fn surface_from_file(repo_root: &Path, relative: &str) -> PolicySurface {
    let path = repo_root.join(relative);
    let text = fs::read_to_string(&path).unwrap_or_default();
    let resolved = if text.len() < 400 {
        pointer_target(&text)
            .and_then(|target| fs::read_to_string(repo_root.join(target)).ok())
            .map(|target_text| format!("{text}\n{target_text}"))
            .unwrap_or(text)
    } else {
        text
    };
    PolicySurface {
        name: relative.to_owned(),
        text: resolved,
    }
}

/// Extract `path` from the first `[label](path)` markdown link in `text`.
fn pointer_target(text: &str) -> Option<String> {
    let start = text.find("](")? + 2;
    let end = text[start..].find(')')? + start;
    let target = text[start..end].trim();
    (!target.is_empty() && !target.starts_with("http"))
        .then(|| target.trim_start_matches("./").to_owned())
}

/// Audit every surface for the one caller policy.
pub fn audit_caller_policy(surfaces: &[PolicySurface]) -> Result<(), Vec<PolicyViolation>> {
    let mut violations = Vec::new();

    for surface in surfaces {
        let text = surface.text.to_ascii_lowercase();

        for phrase in RETIRED_PHRASES {
            if text.contains(phrase) {
                violations.push(PolicyViolation {
                    surface: surface.name.clone(),
                    reason: format!("retains retired wording {phrase:?}"),
                });
            }
        }

        if !(text.contains("human may invoke") && text.contains("rtm start")) {
            violations.push(PolicyViolation {
                surface: surface.name.clone(),
                reason: "does not state that a human may invoke argument-free rtm start".to_owned(),
            });
        }
        if !(text.contains("main-agent") && text.contains("sign-off")) {
            violations.push(PolicyViolation {
                surface: surface.name.clone(),
                reason: "does not state that the Main-Agent may start only after explicit human Run-start sign-off"
                    .to_owned(),
            });
        }
        if !(text.contains("subagent never invokes") || text.contains("subagent never runs")) {
            violations.push(PolicyViolation {
                surface: surface.name.clone(),
                reason: "does not state that a Subagent never invokes rtm".to_owned(),
            });
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}
