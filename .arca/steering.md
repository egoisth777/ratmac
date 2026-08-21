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
4. **Ticket tags in the runbook's ticket format** - promoted and integrated
   at the 2026-08-21 planning pass as the revised completion-gate issue
   (`i-032`, `CGD-001`/`CGD-002` accepted into the working authority,
   `CGD-003` deferred with the bundle): the ticket blank and the schema's
   Ticket check tags rules carry the three tag lists, and the sprint measures
   the two executable clauses (a tag-reading checker; a malformed-list
   refusal). The Engine-side cutover remains the deferred ask, selectable at
   a later planning pass once the format is proven on real tickets.
5. **The edition ledger recording order** - minted and integrated in the same
   pass as `i-034` (`ELR-001`-`ELR-003`) after the stable engine refused to
   build at its own tagged commit: the recording landing follows the tag, the
   stable bootstrap resolves from the invoking checkout and builds a clean
   tagged tree (`ELR-002`, measured this sprint), and the false
   exactly-at-an-edition start claim is retired without a new restriction.

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

Freeze stamp: `git:03eacee` +
`goal-sha256:5d03d8cd5a62dc1f7849d4debee3022d20f0d137bade7c4173876153f393eb4e`
- the P1 integration HEAD and the canonical goal revision recorded by the
Engine at the run-013 freeze. P1 opened and closed 2026-08-21 on one signed
batch of three: the agent-operator-protocol issue (`i-033`, accepted), the
revised completion-gate issue (`i-032`, `CGD-001`/`CGD-002` accepted,
`CGD-003` deferred), and the edition-ledger-recording issue (`i-034`,
accepted). The gap check and every ticket are cut against this stamp.

Route - ordered dependencies of the signed sprint, one why per edge:

1. The self-describing CLI (`AOP-001`, `AOP-002`, from `i-033`) precedes the
   operator skill (`AOP-003`, `AOP-004`, same issue) - the skill points at
   the CLI's own output for everything current, so the output it points at
   must exist first.
2. The stable-bootstrap split (`ELR-002`, from `i-034`) depends on nothing in
   this sprint and unblocks every future sprint's driver build.
3. The tag-reading ticket checker (`CGD-001`, `CGD-002`, from `i-032`)
   depends on nothing in this sprint; its tag format landed at integration.
