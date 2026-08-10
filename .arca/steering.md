# Steering

Read me before the goal bundle. This file is direction: what ratmac is for,
the bets behind it, and the lines no goal, issue, or ticket may cross. When
direction changes, this file changes **first**; `.arca/goal/`,
`.arca/issue/`, and `.arca/ticket/` re-align to it, in that order. It binds
contributors (people and agents) choosing what to build next; it never
overrides `.arca/goal/spec.md` on what the running program does.

## What we are building

ratmac (`rtm`) is a small Rust engine that runs agent work as an explicit
state machine. A Machine Class - a runbook, plain TOML data - declares States,
prompts, guards, and transitions; the Engine
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
   starting with this repository's **Plan-Build Runbook**, the Machine
   Class for its P1-P5 cycle. The tool's first governed project is its own
   construction, and `rtm status` answers "where are we".
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
the forecast route from the landed routing, delivery, completion, and
composition contracts to the Self-hosted property in Ideal shape: the
**Plan-Build Runbook** running this repository's P1-P5 cycle through
ratmac. Each item names an
entry condition - what must already be landed or ruled before selecting it
at P1 is safe - and an exit - the direction-level fact its integration would
make true. Conditions forecast what selection would require and deliver;
they select nothing.

1. **The engine-namespace split** - promoted and integrated as
   `i-024-engine-namespace-split` (`ENS-001`-`ENS-012`): the files the Engine
   owns or consumes (the runbook, the runs roster, the locks, the receipts) now
   live under the engine-named root `.ratmac/`, and the arca folder roots the
   contract guards read became runbook data, retiring most of the
   hard-coded-path debt (R-016). Exit - one owner per root - is reached except
   for the roots-table ticket `t-076`, which is held: two of its rows
   contradict the rules that mechanize them, and the ruling waits in
   `i-026-namespace-row-rulings`. Entry for finishing it: a human rules where a
   held ticket's fact lives and who may name a retired folder in Engine source.
   Until then `src/` still carries one declared `.arca` exception and the gap
   records `res-106` and `res-113` stay unproven.
2. **The Plan-Build Runbook** - the cycle-as-runbook issue
   (`i-015-cycle-as-runbook`), waiting whole in the deferred buffer:
   self-hosting this repository's P1-P5 loop. Entry: routing, delivery,
   completion, and composition are all integrated and landed, because the
   runbook consumes all four; and the engine-namespace split above, because
   the Plan-Build Runbook welds in whatever namespace exists when it is
   authored. The four contracts are landed; the split is the remaining entry
   condition. Exit - the Self-hosted property itself: the Plan-Build Runbook's
   Machine Class declares the cycle and is doctor-clean, `rtm status`
   answers "where are we" with the tree-derived lookup demoted to a
   no-live-Run fallback, the landing line is Engine-appended while a Run is
   live, and no gate's verdict rests on content writable by the agent under
   test. Only when that exit holds does ratmac replace the manual P1-P5
   control this file describes - never before, and never by declaring it
   here.
   The carrier also owns (`PCR-001`) the dirty-tree refusal before every
   deliberate-damage step and the intake gate's acceptance of
   working-authority requirement headings. Those responsibilities and the
   entry conditions are unchanged.
   The State-not-Phase wish is no longer a forecast: Billy promoted it on
   2026-08-10, it entered as `i-025-state-vocabulary` (`SVC-001`-`SVC-010`),
   and it is the current sprint. That satisfies the ordering this item always
   demanded - the machine position is renamed before the Plan-Build Runbook is
   authored, so the shop's own runbook is written once, in `states`, and never
   needs a format migration.
3. **The `failed`-outcome contract** - a blocked, independent side path that
   names the concrete Engine-observable failure event granting the third
   terminal. Entry: a human answers the failure-event fork below with that
   concrete event; a judgment value, a transition input, and a guard refusal
   do not qualify. Until that ruling, this item is blocked. Exit: `failed`
   has exactly one Engine-observable trigger. It is independent because the
   Plan-Build Runbook does not depend on it, so it blocks no
   critical-path item; it becomes relevant only if the runbook's terminal
   vocabulary turns out to need the third outcome.

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

Derived record. Regenerated wholesale at P1 close from the signed issue set;
never hand-edited, never a progress report. Stage lives in the tree.

Freeze stamp: pending - planning step 2 records the P1 integration HEAD and the
canonical goal revision computed by `src/goal.rs::revision` over the checked-out
goal. P1 opened 2026-08-10 on the promoted State-not-Phase wish.

A sprint starts when enough issues have collected to be worth integrating
into the goal, and runs the cycle - plan, then build - until the gap check
comes back clean.

This sprint: rename the machine position from `Phase` to `State` across every
live surface, after separating the three words that "state" was carrying at
once - the graph position, the file the Engine writes for a Run, and the whole
live instance - and move nothing but names.

Signed issue set: i-025-state-vocabulary. `SVC-001` through `SVC-008` are
accepted product requirements; `SVC-009` and `SVC-010` are accepted
working-authority requirements resolving to headings in `.arca/schema.md`. The
carrying Ideal-shape property is **Authored, not imitated** - the written
schema is the only way the format is meant to be learned, so its words must
describe the machine that exists; **Refusals are branchable** is served too,
because pre-cutover residue gets its own stable code while every existing code
keeps its identity.

Route - an ordered dependency list, one why per edge. It says what depends
on what, never when.

1. Settle the three words - State, Run Record, Run, with `status` untouched -
   in the authorities before any file moves; first because a rename onto an
   occupied word would re-create the collision the wish was filed about.
2. Format and Run Record surfaces - the runbook's `states` tables and the
   Run Record at `.ratmac/runs/<run-id>/run.toml` with its `state` field;
   after the words are settled because both are the words written down on disk.
3. Residue refusal and diagnostics - a pre-cutover `phases` runbook or Run
   Record refuses before any read, join, parse, or write with its own new
   code, while every pre-existing code keeps its exact identity; after the new
   surfaces exist because a refusal must name the repair the new format wants.
4. Caller-visible text and the live-surface audit - State Prompt, `rtm status`,
   the human doctor report, `--json` findings, and refusal text, proven by an
   audit whose history allowlist is enumerated; after the surfaces so the audit
   measures the finished cutover rather than a half-renamed tree.
5. Behavior-unchanged proof, then re-gap and close - the existing suites pass
   with their meanings intact, then every `SVC` row is classified from fresh
   evidence.

Endpoint: no live product surface names the machine position `Phase`. The
runbook declares `states`, a Run is recorded in `.ratmac/runs/<run-id>/run.toml`
with a `state` field, pre-cutover artifacts refuse and instruct without
migrating, diagnostic codes keep their identity, archived history keeps its
bytes under an enumerated allowlist, and no routing, guard, lock, mint, spawn,
join, hold, abandon, completion, receipt, or exit-code behavior has moved.

Held: the roots-table ticket `t-076` is paused with `blocker-ref`
`i-026-namespace-row-rulings` - two engine-namespace rows contradict the rules
that mechanize them, and only a human can rule where the held fact lives and
who may name a retired folder in Engine source. Its gap records `res-106` and
`res-113` stay unproven until that issue is selected.

Deferred: the Plan-Build Runbook (`i-015-cycle-as-runbook`) now waits only on
this cutover and on `i-026`; it still carries the machine enforcement of the
discard guard - the dirty-tree refusal before any deliberate-damage step
and the intake gate's acceptance of working-authority requirement headings
(`PCR-001`, extended 2026-08-03); the `failed`-outcome contract awaits a
concrete Engine-observable event. Also still deferred: a git-state guard
kind and extraction of hard-coded `.arca/issue|ticket|residual|goal` paths
from the Engine (R-016 debt).

## Advisor conclusion — 2026-08-03

The just-closed safe-deliberate-damage sprint ended at Idle: its
working-authority rules landed, the final gap check found no product gaps,
and there are no current product tickets or live Runs. That is a clean
cutover point because namespace work would not have to move active product
work or an in-flight Run.

The engine-namespace split is the next promotion target. It is the only
unblocked item on the critical path and directly gates the deferred
cycle-as-runbook issue. Today path literals in Rust bake this repository's
`.arca/issue`, `.arca/ticket`, `.arca/residual`, and `.arca/goal` layout into
the Engine. If the cycle is authored first, its runbook will freeze those
paths, so splitting later means re-pathing the shop's own governing runbook
as well as the Engine. Cutting over now means doing that work once. The
authored Machine Class identity above now says only runbook and plain TOML:
it removes `.arca/ratmac.toml` and deliberately names no replacement root.

The State-not-Phase wish remains unordered and unpromoted. If a human
promotes it, settle it before cycle-as-runbook, or that runbook will require
a later format migration. No engine-split issue is minted here. The next
concrete action is to author its complete pending issue bundle for human P1
disposition.

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
