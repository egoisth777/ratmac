# Steering

Read me before the goal bundle. This file is direction: what ratmac is for,
the bets behind it, and the lines no goal, issue, or ticket may cross. When
direction changes, this file changes **first**; `.arca/goal/`,
`.arca/issue/`, and `.arca/ticket/` re-align to it, in that order. It binds
contributors (people and agents) choosing what to build next; it never
overrides `.arca/goal/spec.md` on what the running program does.

## What we are building

ratmac (`rtm`) is a small Rust engine that runs agent work as an explicit
state machine. A Machine Class - a runbook at `.arca/ratmac.toml`, plain TOML
data - declares phases, prompts, guards, and transitions; the Engine
instantiates it into a Run and is the only writer of run state. Progress is
proven by machine-checked guards over artifacts on disk, never by an agent's
claim.

## Ideal shape

Where this is going, independent of any sprint. Prose only: no requirement
IDs, no dates, no ordering - Horizon orders, the gap check measures, this
section only says what the finished system is like. Direction, never
evidence: no residual cites it and no ticket is cut from it. Every issue
folded in at P1 names the property it advances; one that advances none is
rejected, or it is a pivot and this section changes first.

1. **Self-hosted.** The process a project follows is a runbook `rtm` runs -
   starting with this repository's own P1-P5 cycle. The tool's first governed
   project is its own construction, and `rtm status` answers "where are we".
2. **Every boundary machine-checked.** In a governed process no stage advances
   on narration. Every boundary that matters has a guard reading artifacts.
3. **Authored, not imitated.** An agent writes a runbook from the written
   schema and repairs it by diagnostic code until the doctor is clean. Nobody
   learns the format by copying an example.
4. **Refusals are branchable.** Every refusal carries a stable code, a
   location, and the repair. A caller branches on the code, never on prose.
5. **Generic engine.** No project knowledge in Rust. A second project adopts
   ratmac by writing a runbook, changing no code.
6. **One writer, append-only.** Run state has exactly one writer and history
   only grows, so the record cannot be rewritten by whoever is working.

## Thesis

- Agents are unreliable narrators of their own progress. Trust comes from the
  environment - deterministic guards, receipts, digests - not self-report.
- Process-as-data beats process-as-prompt: a runbook you can parse, lint, and
  diff outlives any prompt phrasing.
- Refusal over guessing: when identity, schema, or state cannot be verified,
  the Engine names the mismatch and stops. It never repairs silently.

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

- Not a CI system, task queue, or scheduler-as-a-service.
- Not an agent framework: it calls no models; it gates whatever does.
- Not multi-tenant: one repository, local disk. Runs are plural and uncapped
  within that one repository - concurrency is not tenancy. (The former "one Run
  at a time" clause was the v1 cap ADR-0007 itself marked liftable; the
  2026-07-29 sign-off accepting uncapped multi-run lifted it, and this clause
  moved first so the goal never contradicts what binds harder.)

## Horizon

An authored ordering of directions beyond the current sprint, in direction
and issue terms only. Binds nothing; nothing here is chosen; an item enters
work only by going through P1 like any other issue. The ordering below is
the forecast route from the landed routing, delivery, and completion
contracts - and the composition sprint now in flight - to the Self-hosted
property in Ideal shape: this repository's own
P1-P5 cycle run as a ratmac runbook. Each item names an entry condition -
what must already be landed or ruled before selecting it at P1 is safe - and
an exit - the direction-level fact its integration would make true.
Conditions forecast what selection would require and deliver; they select
nothing.

1. **The `failed`-outcome contract** - a later, separate issue that names
   the concrete Engine-observable failure event granting the third
   terminal. Entry: the failure-event fork below is answered - neither a
   judgment value nor a guard refusal qualifies. Exit: `failed` has
   exactly one Engine-observable trigger. Off the critical path: neither
   the machine-composition issue nor the cycle-as-runbook issue names it
   as a dependency, so it may land beside composition and blocks nothing;
   it becomes urgent only if the cycle runbook's terminal vocabulary
   turns out to need the third outcome.
2. **The engine-namespace split** - a future issue, not yet minted: the
   files the Engine owns or consumes (the runbook, the runs roster, the
   lock, the receipts) move out of `.arca/` to an engine-named root, and
   the arca folder roots the contract guards read become runbook data,
   retiring the hard-coded-path debt (R-016) the current sprint's
   Deferred list already names. Entry: none among these items - it
   consumes neither routing, delivery, completion, nor composition, so
   it may land beside composition and blocks nothing on the critical
   path. Exit: one owner per root - everything under the engine's root
   is Engine-written or Engine-consumed, `src/` carries no `.arca/`
   literal, and a second project runs `rtm` without an `.arca/` folder
   existing. Before the cycle because a self-hosting runbook freezes
   every path it reads: splitting afterwards would re-path the live
   machine governing this repository and churn the namespace twice.
3. **The cycle as the real runbook** - the cycle-as-runbook issue
   (`i-015-cycle-as-runbook`), waiting whole in the deferred buffer:
   self-hosting this repository's P1-P5 loop. Entry: routing, delivery,
   completion, and composition are all integrated and landed, because the
   cycle consumes all four; and the engine-namespace split above, because
   the cycle runbook welds in whatever namespace exists when it is
   authored; hosting it earlier would encode
   first-edge-wins into the shop's own loop. Exit - the Self-hosted
   property itself: `.arca/ratmac.toml` declares the real cycle and is
   doctor-clean, `rtm status` answers "where are we" with the
   tree-derived lookup demoted to a no-live-Run fallback, the landing
   line is Engine-appended while a Run is live, and no gate's verdict
   rests on content writable by the agent under test. Only when that exit
   holds does ratmac replace the manual P1-P5 control this file
   describes - never before, and never by declaring it here.

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
  writer, because whoever edits `.arca/state.toml` recomputes the same
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

Derived record. Regenerated wholesale at P1 close from the signed issue set;
never hand-edited, never a progress report. Stage lives in the tree.

Freeze stamp: goal git HEAD `e9dbf46`, goal SHA-256
`c96ba1541352a9261b8bfe23d2cf41d34049f8642c9cd750902bf40f4ed8e7c6` — planning step 1 closed 2026-08-03.

A sprint starts when enough issues have collected to be worth integrating
into the goal, and runs the cycle - plan, then build - until the gap check
comes back clean.

This sprint: one Run creates and consumes other Runs - composition as
checked ordinary motion.

Signed issue set: i-018-machine-composition.

Route - an ordered dependency list, one why per edge. It says what depends on
what, never when.

1. FDC-009 format extension - the runbook format grows the class and spawn
   tables (and the `join` guard kind's fields) with the doctor's static
   checks; first because every other composition surface reads declared
   classes and spawn names from the format authority.
2. FDC-007 spawn + FDC-011 ledger - `rtm spawn` as checked ordinary motion
   appending the Scheduler-owned ledger entry; after the format because a
   spawn table must exist to check against; the ledger fixes the expected
   set the join will read.
3. Join evaluation + FDC-008 termination - the join guard passes only on
   the Engine-written terminal facts of every non-abandoned ledger child,
   and the doctor's cycle check demands a receipt- or contract-guarded exit
   on every cycle Phase; after spawn/ledger because the join judges the
   ledger.
4. FDC-007 respawn and abandon-with-run-id + FDC-012 cap - phrase-confirmed
   supersession recording the superseded id, and the one-level refusal at
   the spawn boundary; after spawn because both act on ledger-recorded
   children.
5. FDC-010 child-as-reviewer - a spawned child produces the judgment a
   parent's branching Phase consumes; last because it composes spawn, join,
   and the landed verdict delivery without new machinery.

Endpoint: a parent Run parked on its spawn/join Phase finishes on the
durable terminal facts its children's Engines wrote - spawn, join, respawn,
and the cap all refuse or pass by name, with no human courier between
machines.

Deferred: the real cycle runbook (`i-015-cycle-as-runbook`) follows the
engine-namespace split; the `failed`-outcome contract awaits a concrete
Engine-observable event. Also still deferred: a git-state guard kind and
extraction of hard-coded `.arca/issue|ticket|residual|goal` paths from the
Engine (R-016 debt).

## How direction flows

| File | Question it answers | Changes when |
| :--- | :--- | :--- |
| `steering.md` | Why, and where to | direction pivots (rare, first) |
| `.arca/goal/` - the goal bundle | What must become true | re-derived from steering; frozen per Run |
| `.arca/issue/` | What is wrong or missing | on discovery, anytime |
| `.arca/ticket/` | What work, provably done | cut from gap records at P3 |
| `.arca/residual/` | Is each requirement proven | every gap check |
| `.arca/log.md` | What happened | append-only, always |

On a pivot: steering -> `goal/` -> issue triage -> tickets. The frozen goal
is never edited while tickets are open; a new issue is the only road back.
