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
work only by going through P1 like any other issue.

1. **Input-routed transitions** - the input-routed-transition issue
   (`i-016-fsm-doctrine-convergence`): each branching state declares a closed
   legal input list, each ordinary outgoing edge maps one value, and the
   Engine selects by state plus input while the doctor checks complete,
   unique coverage. First because runs are plural and addressed but each
   still walks first-edge-wins; every day that survives, agents route by
   convention the Engine cannot check.
2. **Input delivery and durability** - the input-delivery issue
   (`i-019-input-delivery-durability`): one external judgment supplies one
   transition input to the addressed Run and state; the Engine atomically
   consumes it into immutable evidence before advancing. After selection
   because delivery has no meaning until the class declares the legal values
   and their destinations.
3. **Run completion** - the Run-completion issue
   (`i-020-run-completion`): `passed` on entering a terminal state, durable
   abandonment before active-state retirement, and guard refusal kept
   non-terminal. Independent of routing, but before composition because joins
   need an Engine-written terminal fact they can read.
4. **The `failed`-outcome contract** - a later, separate issue must name the
   concrete Engine-observable failure event that grants the third terminal.
   It does not fold back into input routing or Run completion: neither a
   judgment value nor a guard refusal is failure.
5. **Machine composition** - the machine-composition issue
   (`i-018-machine-composition`): spawn and join semantics, the spawn-ledger
   content contract, recursion depth. After routing, delivery, and completion
   because spawned children must return durable facts that route and
   terminate the parent.
6. **The cycle as the real runbook** - the cycle-as-runbook issue
   (`i-015-cycle-as-runbook`): self-hosting this repository's P1-P5 loop as
   a ratmac runbook. Last because it consumes selection, delivery,
   completion, and composition; hosting it earlier would encode
   first-edge-wins into the shop's own loop.

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
- **Where does the spawn-ledger content contract live?** Run residency
  reserved the per-run spawn-ledger slot by name and left its content
  undefined; the machine-composition issue (`i-018-machine-composition`) is
  the named home but does not yet define it. Either that issue's spec is
  extended to carry the contract, or a new small issue carries it alone.
  Undecided; every property above holds either way.

## Current sprint

Derived record. Regenerated wholesale at P1 close from the signed issue set;
never hand-edited, never a progress report. Stage lives in the tree.

Freeze stamp: goal git HEAD `89071f1` — planning step 1 closed 2026-07-29; the
goal integration landing was authorized and recorded on 2026-07-30.

A sprint starts when enough issues have collected to be worth integrating
into the goal, and runs the cycle - plan, then build - until the gap check
comes back clean.

This sprint: give every Run a durable address of its own.

Signed issue set: i-017-run-residency.

Route - an ordered dependency list, one why per edge. It says what depends on
what, never when.

0. i-017 Run residency (`FDC-004`-`FDC-006`) - the plural `runs` path, one id
   namespace with `--run <id>` always required, a hash-only runbook pin, a
   flat-layout residue that refuses instead of migrating, and run ids never
   reused after abandon. No edges: it is the whole of this sprint, and it
   depends on nothing else signed here.

Endpoint: with the route complete, two Runs of the same project work side by
side without either one reaching the other's files, and a finished Run's
record cannot be overwritten by whoever works next.

Deferred, each awaiting an earlier stratum of the 2026-07-29 split rather than
a decision: the verdict-routed execution core (`i-016-fsm-doctrine-convergence`,
stated in terms of the per-Run verdict slot residency defines), machine
composition (`i-018-machine-composition`, which gives a contract to the
spawn-ledger location residency only reserves), and the cycle as the real
runbook (`i-015-cycle-as-runbook`, which needs all three under it). Also still
deferred from the previous pass: a git-state guard kind; extracting the
hard-coded `.arca/issue|ticket|residual|goal` paths from the Engine (R-016
debt).

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
