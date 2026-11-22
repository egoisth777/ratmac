# Steering

Read [prd.md](prd.md) first: it is the product definition, and this file
follows it. Read me next, before the goal bundle. This file is direction: the
properties every issue advances, and the lines no goal, issue, or ticket may
cross. When direction changes, this file changes **first**; `.arca/goal/`,
`.arca/issue/`, and `.arca/ticket/` re-align to it, in that order. Only a
change to the product definition comes earlier: prd.md lands first, then
this file. It binds contributors (people and agents) choosing what to build
next; it never overrides `.arca/goal/spec.md` on what the running program
does.

## What we are building

Removed on 2026-11-22 at Billy's direction. The product definition is [prd.md](prd.md).

## Ideal shape

Direction, never evidence: no residual cites it and no ticket is cut from it.
Every issue folded in at P1 names the property it advances; one that advances
none is rejected, or it is a pivot.

The properties an issue names are now the core problem and the three core
missions in [prd.md](prd.md), by their exact names. This section lists them
and adds none of its own; a pivot that changes them starts in prd.md, which
only Billy changes.

- [Core problem](prd.md#core-problem)
- [Mission 1 - a reliable local engine](prd.md#mission-1---a-reliable-local-engine)
- [Mission 2 - parallel traversal, orchestration, and execution](prd.md#mission-2---parallel-traversal-orchestration-and-execution)
- [Mission 3 - a language that agents already know](prd.md#mission-3---a-language-that-agents-already-know)

## Thesis

Removed on 2026-11-22 at Billy's direction. The product definition is [prd.md](prd.md).

## Invariants

These survive every goal change. A goal, issue, or ticket that needs to break
one is a direction change and starts here.

1. Only `rtm` writes run state; contributors and agents read.
2. The Machine Class is data, read-only at runtime. The Engine carries no
   project knowledge (R-016): anything project-specific belongs in the
   runbook, not in Rust.
3. Guards judge artifacts, not narration. A failing guard refuses, reports,
   and leaves state untouched (R-017/R-020).
4. Executed guards run pinned code (ETB-001) unless declared `exempt`, and
   refusals surface the gate's own diagnostics (ETB-002).
5. Schema is strict: unknown keys rejected (R-011); `status` never appears in
   a runbook (R-002/R-003) - status is runtime, owned by the Engine.
6. History is append-only; the goal is frozen per Run and drift is caught by
   content hash.
7. Deterministic and offline: no network, no installs, no hidden global state.

## Non-goals

Removed on 2026-11-22 at Billy's direction. The product definition is [prd.md](prd.md).

## Horizon

Removed on 2026-11-22 at Billy's direction. The product definition is [prd.md](prd.md).

Kept from the removed forecast: the State-not-Phase rename.

The line above stays only because a test fixture (`test/qa/fixtures/rebrand-audit/allowlist.tsv`) needs its retired word in this file; removing it is program-lane work that needs a ticket.

## Open questions

Forks not yet decided. Each one is a choice of mechanism, never of
destination: every property above holds whichever way it goes. This section
binds nothing - nothing here is chosen and no work is cut from it. An answer
leaves the section: it lands as a property above, as a requirement in the
goal, or as a new issue, and the question is deleted here. A fork nobody
wrote down is drift.

- **Catch it, or stop it?** Today the ownership check reads the runbook: it
  catches a runbook that *tells* an agent to write engine-owned files, and
  nothing stops an agent that simply writes them. Either ratmac stays a gate
  that refuses afterwards, or the surrounding harness gains hooks that make
  the write impossible. Undecided. One branch is already closed: a check the
  Engine computes from the file it is checking cannot catch a deliberate
  writer, because whoever edits `.ratmac/runs/<run-id>/run.toml` recomputes the same
  unkeyed digest over it. Detecting an intentional writer needs an
  authenticator anchored outside what that writer can reach - which is the
  harness branch under another name. A self-contained digest is worth having
  for accidental damage and staleness, and it is not an integrity boundary;
  claiming otherwise would make run state look machine-owned while it is not.
- **What concrete event grants `failed`?** The outcome has a separate future
  home after the Run-completion cut, but its trigger is still undecided. It
  must be an event the Engine can observe directly. A guard refusal cannot
  qualify because refusal leaves state unchanged, and a transition input
  cannot qualify because richer work outcomes belong in judgments and
  evidence. Every Ideal-shape property holds whichever qualifying event is
  eventually chosen.

## Current sprint

Derived at planning close on 2026-10-27 from Billy's 2026-10-26 wish selection
and his 2026-10-27 live-run pin decision. Freeze stamp:
`git:e7499db117a3b45a9f873bf066d449d9080fd069` +
`goal-sha256:v2:fa3e740c95a958e84039a791a10fdd8f67ca94042f56be0744c662d282ae2c52`.
The stable edition-009 Engine froze this accepted bundle for run-060.

The signed batch contains 12 accepted issues, advancing Every boundary
machine-checked, Self-hosted, Refusals are branchable, and One writer,
append-only. A test, a private lane, or a command gives the same answer
whatever runs beside it, wherever Cargo puts its build output, and however
briefly another program holds a file. Seven working-authority requirements
bind contributor tests, test support, and tools; seven product requirements
bind the Engine and the project-local bootstrap. Every one needs executable
proof.

Accepted issue set:

- [build-message test Engine](issue/archive/i-063-build-message-test-engine/index.md) (SVE-001).
- [Cargo-built bootstrap Engine](issue/archive/i-064-cargo-built-bootstrap-engine/index.md) (SVE-002).
- [same lane verdict](issue/archive/i-065-same-lane-verdict/index.md) (SVE-003, SVE-007, SVE-008).
- [unique test scratch](issue/archive/i-066-unique-test-scratch/index.md) (SVE-004).
- [whole pause marker](issue/archive/i-067-whole-pause-marker/index.md) (SVE-005).
- [case-scoped hold barrier](issue/archive/i-068-case-scoped-hold-barrier/index.md) (SVE-006).
- [repeated run refusal](issue/archive/i-069-repeated-run-refusal/index.md) (WTC-001).
- [unreadable record refusal](issue/archive/i-070-unreadable-record-refusal/index.md) (WTC-002).
- [skill move retry](issue/archive/i-071-skill-move-retry/index.md) (WTC-003).
- [abandon release window](issue/archive/i-072-abandon-release-window/index.md) (WTC-004).
- [endpoint release window](issue/archive/i-073-endpoint-release-window/index.md) (WTC-005).
- [live run pin check](issue/archive/i-074-live-run-pin-check/index.md) (LRP-001).

Route — accepted issue dependencies, not progress or implementation status:

- same lane verdict follows build-message test Engine because a private lane
  launches only the executable the shared test helper reports: lane builds stay
  private (SVE-007) depends on SVE-001, and the run-private freeze and ports
  folders (SVE-008) depend on both.
- case-scoped hold barrier follows whole pause marker because its check learns
  that the armed case is parked from the marker's whole boundary line
  (SVE-006 depends on SVE-005), and both rewrite the same cycle-pause check.
- live run pin check lands after Cargo-built bootstrap Engine because both edit
  the same lines of `tools/rtm.ps1` and the same bootstrap suite; neither needs
  the other.
- skill move retry, abandon release window, and endpoint release window land
  after whole pause marker and case-scoped hold barrier because their new pauses
  and faults follow those two test-isolation rules; neither rule is a
  prerequisite.
- The stable driver retains its exact pinned runbook during this batch: no
  requirement here changes the runbook, and no Run pin is rewritten.
