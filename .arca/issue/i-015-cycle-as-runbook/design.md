# Issue design

## Proposed mechanics

This file is incoming evidence: integrated mechanics remain authoritative only in the accepted forward
authority.

**1. Per-ticket gating with a read-only runbook (`PCR-007`) - the open decision.** `sensitivity_receipts`
and `completion_gate` each require a literal `ticket` field, and the runbook is read-only at runtime with no
interpolation. Four candidates, recorded for P1 to choose between rather than settled here:

- **(a) Coarse loop.** The P4/P5 loop becomes one Phase whose exit guard is `record_contract`, which names
  no ticket. **Rejected at review:** `record_contract` checks planning-record shape only - one residual per
  requirement, evidence behind `satisfied`, one owning ticket per gap, acyclic dependencies, complete
  sections. It checks no receipt. Choosing it silently drops the per-ticket guarantees `PGE-003` and
  `PGE-005` already carry as accepted requirements, which is a regression dressed as a simplification.
- **(b) Run per ticket.** `rtm start` once per ticket; the runbook names that ticket. Keeps the gates,
  but makes the runbook a per-ticket artifact and multiplies Runs, and the cycle's own P1-P3 stages then sit
  outside any Run.
- **(c) Bind the gate target from `active_refs`.** The State File already carries `active_refs` as one of
  its seven Scheduler-written fields (R-025, ADR-0008), and the fixtures show its intended content is
  exactly this - `["ticket-r020"]`, `["R-025", "T-07", "T-15"]`. `Scheduler::step` loads the state before it
  calls `guard_failures`, so the active refs are already in scope one call above the guard dispatch. The
  runbook then declares the *role* ("the active ticket") instead of an id, an explicit `rtm` input sets the
  active ref, and no ticket id ever enters the file. This adds no dimension to the machine graph:
  `ADR-0001` fixes what transitions may branch on - Phase, never `status` - and the State File already
  holds six fields besides `phase`. **Cheapest of the three that keep the gates.**
- **(d) New machine state plus field interpolation in the runbook format.** Rejected: it makes the runbook
  a template language and reopens `R-013`.

Recommended for P1: **(c)**, with the scope question being whether setting the active ref is a new `rtm`
verb or a field of an existing one.

**2. The landing append (`PCR-004`).** With a Run live, `.arca/log.md` belongs to `rtm`, and no Phase
Prompt may instruct an agent to write it (`RB401`). Smallest resolution: one `rtm` command that takes the
line's content and performs the Scheduler-owned append, so the prompt says "record the landing" and names a
command instead of a path. Alternative recorded for P1: declare `.arca/log.md` agent-writable and drop it
from the Scheduler-owned set - cheaper, but it deletes the property that history is machine-owned, which is
why `PGE-004` exists.

**3. Make "open" mechanical (`PCR-003`).** Two candidates. (a) Extend the authorized archive move from
issues to tickets: a ticket whose residuals are all `satisfied` and whose `landed-commit` is recorded may
move to `.arca/ticket/archive/`, and "open" means "present in `.arca/ticket/`". (b) Keep every ticket in
place and derive "open" from evidence: a ticket is open while any residual it owns is not `satisfied`.
(b) needs no new rule and no move, and it is already how the 2026-07-24 close reasoned in practice; (a)
matches the issue rule and keeps the directory small. P1 picks one - the requirement is that a guard, not a
reader, can tell the difference.

**4. Write the cycle runbook (`PCR-001`, `PCR-002`, `PCR-005`).** Phases follow the working rules already
written down: intake and integration (P1), freeze and gap check (P2), cut tickets (P3), tests-first (P4),
implementation and close (P5), with the loop edge from P5 back to P4 while tickets remain and the exit edge
to a rest state when the gap check comes back clean. Guards are the ones that already exist -
`intake_contract` at the P1 exit, `record_contract` and the goal freeze at P2, ticket ownership and
acyclicity at P3, `sensitivity_receipts` at P4, `completion_gate` at P5 - which is the honest test of
whether the closed guard vocabulary is sufficient. Where it is not, the finding is a new guard-kind issue,
not a widening of scope here. `rtm status` then answers the stage question, and `.arca/index.md`'s lookup
table is demoted to the no-Run fallback it already claims to be.

**Open: Run lifetime.** Whether the cycle Run is perpetual (started once, a rest Phase between sprints, a
permanent `.arca/state.toml`) or per-sprint (`rtm start` at P1, finished at the clean gap check) is
undecided and decides `PCR-002`'s reach: only the perpetual shape lets `rtm status` answer between sprints
and so actually retires the second oracle. Recorded as an open decision, not assumed.

**Sequencing note.** Parts 1-3 are prerequisites discovered by trying to write part 4; each is small and
independently provable. Part 4 is the only one that produces the endpoint, and it is worth nothing if the
first three are skipped - a cycle runbook that cannot name the ticket it is gating, fails its own ownership
lint, and cannot say whether a ticket is open is a demonstration, which is what exists today.

## Non-goals

- No new guard kind is introduced by this issue. If the cycle cannot be expressed in the closed vocabulary,
  that discovery becomes its own issue.
- No change to the P1-P5 rules themselves. This encodes the working rules as they stand; changing them is a
  steering pivot, not an encoding task.
- No extraction of the Engine's hard-coded `.arca/*` paths (R-016): dropped at review, still deferred in
  `.arca/steering.md`.
- No agent-spawning, no process management, no scheduling: the Run is still stepped by a caller.
