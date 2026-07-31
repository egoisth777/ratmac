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
verb or a field of an existing one, and with one constraint that (c) is unsound without.

**Constraint on (c) - a set active ref selects what the gates judge.** If the active ref is only ever
*set* and never *derived*, then whatever writes `.arca/state.toml` chooses which ticket
`sensitivity_receipts` and `completion_gate` grade. Point it at a ticket whose residuals are already
`satisfied` and both gates pass while the real work stays unproven - and this needs no bad intent, a
stale ref left over from the previous loop turn does it. So (c) must derive the active ticket from the
tree and treat the stored value as a cache: on every step the Engine recomputes the derivation, and a
stored ref that disagrees is a refusal with its own code, never a silent override. A test has to pin
that refusal, or the mechanism is a hole with a receipt attached.

**Two predicates, not one - and residual status cannot drive the build loop.** An earlier draft of this
paragraph proposed one derivation over the tree serving part 1, part 3, and part 4 at once, with a ticket
open while any residual it owns is unproven. That is wrong on the working rules' own timing.
`.arca/schema.md` writes residual records at P2 and reruns the gap check only once no tickets are left, so
residual status is a constant for the whole length of the P4/P5 loop: the ticket that just landed keeps
every unproven residual it started with, stays the head of the ordered set, and the loop never advances.
Recomputing "proven" earlier, before the ticket's own completion gate has run, breaks it the other way -
the head moves off a ticket whose tests are not written yet, since a run can be green simply because the
previous ticket's tests are the only ones that exist.

So the cycle needs two predicates with different clocks and different owners:

- **Ticket complete** - per ticket, recomputed now, and what advances the build loop. The gate that judges
  it already exists and `src/completion.rs` states its own limit: "The gate writes nothing: a refusal
  leaves the ticket executing and its residuals unproven." Its inputs are receipts under the evidence
  directory `src/receipt.rs` calls agent-writable, so a receipt is evidence the gate *verifies* and never
  the signal that the loop moved - reading it as the signal would put the loop's position back in the hands
  of whatever writes the evidence, which is the hole two corrections above. The machine-owned signal is the
  Scheduler's own successful transition: `Scheduler::step` runs the guards, writes `.arca/state.toml`, then
  appends one history line, rolling both back together if the append fails, so the two never disagree. A
  ticket is complete when the Engine recorded that it left the P5 Phase with that ticket active - or when
  the ticket took an authorized archive move, the alternative `PCR-003` already offers.

  **This needs one small engine change, and P1 should scope it.** The appended line is exactly
  `- Transition: <from> -> <to>`: it names no ticket, so today's history cannot say *which* ticket
  advanced. The transition entry has to carry the active ref it advanced before the build loop's head is
  derivable from Engine-owned records at all.
- **Gap remains** - per requirement, recomputed only at a gap check, and the condition on the P2/P3 edges:
  more tickets to cut, or rest. This one is residual status, and it belongs to nothing else.

Ordering inside the build loop stays the tickets' declared dependencies with the ticket number breaking
ties - deterministic because `record_contract` already proves that graph acyclic.

**A gap in the working rules, for P1 to settle.** `.arca/schema.md` never says who flips a residual to
`satisfied` during the build loop, yet `.arca/log.md` shows landings doing exactly that. A machine cannot
read a step nobody wrote down. Either the rules gain that step explicitly, or residual status is declared a
gap-check-only judgment and ticket completion carries its own evidence - the second is what the two
predicates above assume, and it is the reason they are two.

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
