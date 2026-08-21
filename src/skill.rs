//! AOP-003/AOP-004: the engine writes its own operator skill.
//!
//! `rtm skill <path>` writes the thin `ratmac-operator` skill folder - one
//! `SKILL.md` carrying the writing engine's identity stamp plus a
//! `references/` folder that deepens the entry document - and refuses rather
//! than overwriting anything. It is the scaffold's discipline carried from
//! one file to one folder: checks before the first byte, exactly one folder
//! at the caller's path, no directories beyond it.
//!
//! The content teaches only invariants - the operating loop and the
//! never-touch rules. It enumerates no flags and quotes no command output:
//! everything current is reached by running the engine, never by copying
//! what it printed. The identity stamp is computed from the running
//! executable at write time, the same digest the argument-free doctor
//! reports (DFP-001), so a stale skill is detectable by comparing stamps.
//!
//! The write is atomic in placement: the folder is built at a hidden
//! sibling and moved into place by one rename, so an interrupted write can
//! never leave a half folder at the caller's path - only a named leftover
//! beside it, which the next attempt refuses and names for the operator to
//! remove.

use std::path::{Path, PathBuf};

/// The suffix of the hidden sibling an in-progress write builds in before
/// the one rename moves it to the caller's path. Fixed per target so an
/// interrupted write is detectable by name: the leftover never sits at the
/// requested path, and the next attempt refuses naming it.
const PARTIAL_SUFFIX: &str = "rtm-skill-partial";

/// Why a skill folder was not written. Every variant leaves the tree
/// untouched.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SkillRefusal {
    /// Something already lives at the requested path.
    Occupied(String),
    /// The requested path's parent directory does not exist. The skill
    /// write creates exactly one folder and no directories, so the parent
    /// is the caller's call to make, not the Engine's.
    NoParent(String),
    /// An interrupted earlier write left its partial sibling behind. The
    /// Engine never deletes what it cannot vouch finished, so the operator
    /// removes the named leftover and runs the command again.
    Leftover(String),
    /// A live pre-split artifact prevents any Engine entry from acting.
    Preflight(String),
    /// The write itself failed; any partial sibling was removed.
    Unwritable(String, String),
}

impl std::fmt::Display for SkillRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Occupied(path) => write!(
                formatter,
                "skill: {path} already exists; the skill write never overwrites - choose a new path or delete that one yourself"
            ),
            Self::NoParent(path) => write!(
                formatter,
                "skill: the directory for {path} does not exist; the skill write creates exactly one folder and no directories"
            ),
            Self::Leftover(path) => write!(
                formatter,
                "skill: an interrupted write left {path} behind; no partial folder ever sits at the requested path - remove the leftover yourself and run the command again"
            ),
            Self::Preflight(reason) => formatter.write_str(reason),
            Self::Unwritable(path, error) => {
                write!(formatter, "skill: cannot write {path}: {error}")
            }
        }
    }
}

/// Write the skill folder at `path`, or refuse.
///
/// The checks come before the first byte, so a refusal is a refusal: no
/// partial folder, no created directory, nothing to clean up. The write
/// itself builds the whole folder at a hidden sibling of `path` and moves
/// it into place with one rename, so an interrupted write leaves the
/// caller's path absent and a named leftover beside it - never a half
/// folder that passes for the skill.
pub fn write_skill(path: &Path) -> Result<(), SkillRefusal> {
    let project_root = crate::root::addressed_project_root(path);
    crate::Scheduler::refuse_flat_residue(&project_root)
        .map_err(|error| SkillRefusal::Preflight(error.to_string()))?;
    let shown = crate::root::displayed(path);
    if path.exists() {
        return Err(SkillRefusal::Occupied(shown));
    }
    let parent = match path.parent() {
        // A bare folder name is written into the current directory.
        Some(parent) if parent.as_os_str().is_empty() => Path::new("."),
        Some(parent) => parent,
        None => return Err(SkillRefusal::NoParent(shown)),
    };
    if !parent.is_dir() {
        return Err(SkillRefusal::NoParent(shown));
    }
    let Some(name) = path.file_name() else {
        return Err(SkillRefusal::NoParent(shown));
    };
    let partial: PathBuf = parent.join(format!(
        ".{}.{}",
        crate::root::component(name),
        PARTIAL_SUFFIX
    ));
    if partial.exists() {
        return Err(SkillRefusal::Leftover(crate::root::displayed(&partial)));
    }
    let Some(stamp) = crate::pin::engine_identity().map(|identity| identity.sha256) else {
        return Err(SkillRefusal::Unwritable(
            shown,
            "cannot resolve the running engine's identity".to_owned(),
        ));
    };
    if let Err(error) = write_partial(&partial, skill_files(&stamp)) {
        let _ = std::fs::remove_dir_all(&partial);
        return Err(SkillRefusal::Unwritable(shown, error.to_string()));
    }
    match std::fs::rename(&partial, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = std::fs::remove_dir_all(&partial);
            Err(SkillRefusal::Unwritable(shown, error.to_string()))
        }
    }
}

/// Die mid-write, exactly as an interrupted process dies, when the QA-only
/// fault seam asks for it: after the entry document lands, before the
/// references. Death runs no cleanup, so whatever the write built so far
/// stays where it was being built - the interruption the durability lanes
/// judge. Compiled only with `test-fault-injection`, which the QA harnesses
/// enable; production builds use the no-op stub below.
#[cfg(feature = "test-fault-injection")]
fn abort_mid_write_if_requested(name: &str) {
    if name == "SKILL.md"
        && std::env::var("RATMAC_TEST_SKILL_FAULT").ok().as_deref() == Some("after-entry")
    {
        std::process::abort();
    }
}

#[cfg(not(feature = "test-fault-injection"))]
fn abort_mid_write_if_requested(_name: &str) {}

/// Build the complete folder at `partial`: every file at its final name,
/// so the rename into place exposes the whole skill at once.
fn write_partial(partial: &Path, files: Vec<(String, String)>) -> std::io::Result<()> {
    std::fs::create_dir(partial)?;
    for (name, body) in files {
        let target = partial.join(&name);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(target, body)?;
        abort_mid_write_if_requested(&name);
    }
    Ok(())
}

/// The one folder the skill write produces, in write order: the entry
/// document and its references.
fn skill_files(stamp: &str) -> Vec<(String, String)> {
    vec![
        ("SKILL.md".to_owned(), skill_md(stamp)),
        ("references/loop.md".to_owned(), LOOP_REFERENCE.to_owned()),
        (
            "references/refusal-codes.md".to_owned(),
            REFUSAL_CODES_REFERENCE.to_owned(),
        ),
        (
            "references/never-touch.md".to_owned(),
            NEVER_TOUCH_REFERENCE.to_owned(),
        ),
    ]
}

/// The entry document: the whole loop, the never-touch rules, and the
/// identity stamp of the engine that wrote this copy. It names no flags
/// and quotes no render; references only deepen what is here.
fn skill_md(stamp: &str) -> String {
    format!(
        r#"---
name: ratmac-operator
description: Operate the ratmac run engine when a .ratmac/ directory exists in the project or a request mentions rtm, a runbook, a Run, stepping, or receipts. Teaches the operating loop and the never-touch rules only; everything current is read from the engine's own output by running its commands.
---

# ratmac operator

This skill teaches only invariant behavior: the loop that operates a Run,
the never-touch rules, and where each fact comes from. It names no flags
and quotes no command output. Anything current - the verbs, the wording,
the roster, a version's shape - is reached by running the engine and
reading what it prints.

Written by the engine itself. Its identity stamp, the sha256 the
argument-free doctor reports for the running executable:

    {stamp}

## The operating loop

1. Orient through the engine's own report: run `rtm status` and read it.
   Run addressing is taught by the engine's own refusal when it is needed.
2. Read the State Prompt the engine renders: the authored prompt of the
   current state names the work; do that work, not a guessed substitute.
3. Place the artifacts the pending guards declare: a guard's line names
   the exact artifact it reads. Place each artifact where the runbook
   declares it and nowhere else.
4. Step the Run: run `rtm step`. A refused step is safe to repeat once
   the named gap is closed.
5. Branch on the refusal's stable code: the code names the failure
   family, and the refusal itself names the repair. Follow the repair;
   never argue with or work around a refusal.

The loop ends when the engine's render deliberately teaches nothing more:
a terminal Run. Read the final report there.

## Never-touch rules

- Never write under the engine root. `.ratmac` and everything beneath it
  is engine-owned - run records, the transition log, locks, evidence,
  receipts. You never write under the engine root; the engine does.
- Never edit the runbook to make a guard pass. The runbook is
  human-authored, reviewed input; a guard that fails is telling the truth
  about the work.
- Never fabricate an artifact, a receipt, or engine output. Produce the
  real artifact, then let the engine observe it.
- Reach everything current through engine output: run the command and
  read what it renders. Never quote a render from memory into a decision.

## References

- `references/loop.md` deepens each turn of the loop.
- `references/refusal-codes.md` deepens branching on refusal codes.
- `references/never-touch.md` deepens the never-touch rules.
"#,
        stamp = stamp
    )
}

const LOOP_REFERENCE: &str = r#"# The operating loop in depth

Each turn of the loop reads one render and performs at most one act.
The engine decides what is current; the operator follows.

## Orient

Begin every session and every unfamiliar situation by running the
engine's status command for the Run being operated and reading the whole
render: the state, its lifecycle position, the blocker if any, and the
guards waiting on work. If you do not yet know which Run, run the
command without an address and read the refusal: it lists the roster and
names the one way to address a Run.

## Read the prompt

The render carries the current state's authored prompt. That prose is
the work order: it names what the state exists to produce. Do not infer
a different task from the ticket trail alone; the prompt is the
machine's voice about its own state.

## Place the artifacts

A waiting guard names the artifact it reads and the shape it expects.
Place the real artifact at the declared path, beneath a declared root,
produced by actually doing the work. A guard that still fails after the
artifact exists is reporting a real mismatch: read the observed and
expected facts it names and close that exact gap.

## Step

One act per turn: advance the Run with the engine's step command. A
refused step changes nothing and is safe to repeat; re-run it only after
the named gap is closed. Never advance a Run by any other means.

## Branch on the refusal code

Every refusal carries a stable code. The code, not the prose around it,
is the branch key:

- a guard refusal means the work is not done yet: close the named gap,
  then step again;
- an addressing refusal means the Run was named wrong: re-address from
  the roster the refusal lists;
- a terminal refusal means the Run is finished: read its record and
  report; no act remains.

When a render teaches nothing further, the Run is terminal. Report the
final render's facts and stop.
"#;

/// Branching on refusal codes in depth: the code is the key, the render
/// names the repair, and codes are read live - never memorized.
const REFUSAL_CODES_REFERENCE: &str = r#"# Branching on refusal codes

A refusal is information, not an obstacle. Every refusal the engine
renders carries a stable code that names its failure family, and the
refusal itself names the repair for that code. The invariant is the
discipline:

1. Read the code from the refusal render. The code is stable across
   versions; the wording around it is not, so never branch on wording.
2. Look for the repair the same render names. The engine teaches the
   next legitimate act for the state it rendered; run exactly that act.
3. If the render names no act, the Run is terminal or the refusal admits
   no engine verb: read the record and report to the human who owns the
   decision.

Repeated refusals with the same code mean the same gap is still open.
Re-running a refused step is safe: a refusal writes nothing, so the
retry begins from an unchanged Run. Never loop a refusal more times
without changing the world between attempts - place the artifact,
produce the output, or ask the human, then step again.

Codes are never memorized here. The engine that renders a refusal also
names its repair; a different version may add, rename, or retire codes,
and the operator that reads the living render is never wrong about the
current set.
"#;

/// The never-touch rules in depth: the engine root, the runbook,
/// artifacts and receipts, and what current means.
const NEVER_TOUCH_REFERENCE: &str = r#"# The never-touch rules in depth

The engine owns its records; the operator owns the work. The boundary
is the engine root.

## The engine root

Never write under the engine root: the project's `.ratmac` directory and
everything beneath it. That tree holds run records, the transition log,
locks, run evidence, and receipts - all engine-written. Editing, adding,
or deleting anything there - by editor, script, or version-control
command - can corrupt a Run in ways the engine will honestly refuse to
guess about. If a task seems to require writing there, the task is
really asking for an engine command; run the engine and let it write.

## The runbook

The runbook is human-authored, reviewed input. Never edit it to make a
guard pass, never add a transition to route around work, never adjust a
declared path to match where a file happens to sit. If the runbook is
wrong, say so to the human who owns it; the engine's doctor command
diagnoses a runbook read-only and reports by stable code.

## Artifacts and receipts

Place artifacts only where the runbook declares them, beneath declared
roots, and only by producing the real work. Never fabricate an artifact,
never write a receipt for work not done, never copy engine output into a
file to satisfy a guard - the guard reads the artifact, and the record
of the work must be the work.

## What is current

Everything current - commands, wording, rosters, codes, versions - is
read from engine output at the moment it is needed, by running the
engine. Never quote a remembered render, never restate a help text, and
never teach a future operator from a paste: run the command and read
what it prints.
"#;
