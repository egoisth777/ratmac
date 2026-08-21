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
   hard-coded-path debt (R-016). Exit - one owner per root - is reached: Billy
   ruled on 2026-08-10 (`NRR-001`, `NRR-002`) that the Engine has no work-item
   concept and that one retired folder name may be declared once in Engine
   source under its own owner, the roots-table ticket `t-076` landed, `t-087`
   removed the Engine's last write under a workflow root, and the gap records
   `res-106` and `res-113` are proven. What `src/` still carries is that one
   named, owned `.arca` exception - the residue detector's own literal.
2. **The Plan-Build Runbook** - promoted and integrated as
   `i-015-cycle-as-runbook` (`PCR-001`, `PCR-002`, `PCR-003`, `PCR-005`,
   `PCR-007`, `PCR-008`, `PCR-009`): Billy selected it from the deferred
   buffer on 2026-08-10, every entry condition having landed - routing,
   delivery, completion, composition, the engine-namespace split, and the
   State-not-Phase rename that let the shop's own runbook be written once, in
   `states`. It is the current sprint, so its exit is measured there and not
   forecast here. One ask did not survive integration: the landing line stays
   a human act, because the working rules made history human-only and the
   no-work-item ruling forbids the Engine writing under a workflow root.
3. **The `failed`-outcome contract** - a blocked, independent side path that
   names the concrete Engine-observable failure event granting the third
   terminal. Entry: a human answers the failure-event fork below with that
   concrete event; a judgment value, a transition input, and a guard refusal
   do not qualify. Until that ruling, this item is blocked. Exit: `failed`
   has exactly one Engine-observable trigger. It is independent because the
   Plan-Build Runbook does not depend on it, so it blocks no
   critical-path item; it becomes relevant only if the runbook's terminal
   vocabulary turns out to need the third outcome.
4. **Ticket tags in the runbook's ticket format** - Billy's 2026-08-18 ruling
   on the issue about the completion gate reading declared data
   (`i-032-completion-gate-reads-declared-data`): the declared-checks question
   is a workflow matter, not an Engine matter. The ticket format the
   Plan-Build Runbook owns gains explicit tag lists (focused tests, hidden
   lanes, quality commands) as first-class ticket fields, and the completion
   story follows from that format. Entry: the next planning pass (P1) revises
   `i-032`'s pending asks to this framing - the ticket blank
   (`.arca/tpl/ticket.md`) and the schema's ticket rules carry the tags; no
   Engine cutover is presumed. Exit: a ticket declares its checks as tags the
   workflow defines, and no checker needs to read a ticket's prose to learn
   them.

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

Freeze stamp: `git:4ac18a1` +
`goal-sha256:74ccde5aefbf2a5b5c4a773e6438ddb20f1d6c123d095f5bb3dc51b2e73e8440`
- the P1 integration HEAD and the canonical goal revision computed by
`src/goal.rs::revision` over the checked-out goal. P1 opened and closed
2026-08-10 on the cycle-as-runbook issue; the gap check and every ticket are
cut against this stamp.

A sprint starts when enough issues have collected to be worth integrating
into the goal, and runs the cycle - plan, then build - until the gap check
comes back clean.

This sprint: run the shop's own process on the engine. The Machine Class this
repository ships is still a demonstration that builds a file called
`release.txt`, while the process the shop actually follows lives as prose and
"where are we" is answered by a person reading a table. Replace the
demonstration with the real cycle, and make the engine able to carry it
without learning what a ticket is.

Signed issue set: i-015-cycle-as-runbook. `PCR-001`, `PCR-002`, `PCR-003`,
`PCR-005`, `PCR-007`, `PCR-008`, and `PCR-009` are accepted product
requirements; `PCR-004` is rejected. The carrying Ideal-shape property is
**Self-hosted** - the tool's first governed project is its own construction.
**Every boundary machine-checked** is served with it: a stage that cannot be
expressed as a guard over artifacts is a stage the shop was advancing on
narration. **Generic engine** binds the shape of the answer: the cycle's own
identifiers stay in the runbook and out of Rust.

Route - an ordered dependency list, one why per edge. It says what depends
on what, never when.

1. Teach the intake gate that an accepted ask may resolve to a
   working-authority requirement heading (`PCR-008`). First, because the gate
   refuses this repository's own tree until it does, and every later stage
   runs against that tree.
2. Give a receipt-class guard the bound address (`PCR-007`). Before the
   runbook is authored, because the runbook cannot name its ticket turns
   until the address exists.
3. Add the refusal that stops deliberate damage on an uncommitted tree
   (`PCR-009`). Independent of the two above and of the runbook; it earns its
   place in this sprint because the runbook is where it is declared.
4. Make the open-work-item predicate a machine check over the real records
   (`PCR-003`). After the gate work, because it reads the same roots.
5. Author the Plan-Build Runbook itself and prove the doctor exits clean on
   it (`PCR-001`, `PCR-005`). Last of the building work, because it consumes
   every piece above and is worth nothing without them.
6. Demote the tree-derived stage lookup to the labelled no-live-Run fallback
   (`PCR-002`). After the runbook exists, because until then the tree is the
   only oracle and demoting it would leave the question unanswered.

Endpoint: `rtm doctor` exits `0` on this repository's Machine Class; a Run
started on it advances through intake, gap check, cutting the work items, the
ticket turns, close, and rest by starting and stepping alone; the addressed
report names the stage while that Run is live; no guard's verdict rests on
content the agent under test can write; and no identifier for a work item
appears in the runbook file.

Not in this sprint, and named so the absence is legible: the landing line
stays a human act (`PCR-004`, rejected); a repository-state guard kind would
be needed for the dirty-tree refusal to see a file that was never added, and
the closed vocabulary cannot express "no gap remains", so the branch out of
the gap-check stage is a declared transition input judged by the records the
record gate reads. The `failed`-outcome contract still awaits a concrete
Engine-observable event. Extraction of the hard-coded workflow paths from the
Engine (R-016 debt) stays deferred, as does the completion gate's remaining
habit of parsing a contributor's document for the checks it declares.

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
