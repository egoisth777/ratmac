# Issue design

## Proposed mechanics

Four parts, in dependency order. This file is incoming evidence: integrated mechanics remain authoritative
only in the accepted forward authority.

**1. Unblock the ownership contradiction (`PCR-004`).** The cycle's Phase Prompts must survive the doctor's
ownership pass, so the landing line cannot be an instruction to edit `.arca/log.md`. The schema already
names the shape of the answer - an explicit `rtm` command that performs the Scheduler-owned append itself.
The smallest version is one command that takes the line's content from the agent and writes it, so the
prompt says "record the landing" and points at a command rather than a path. Alternative considered and
recorded for P1: declare `.arca/log.md` agent-writable and drop it from the Scheduler-owned set - cheaper,
but it deletes the property that history is machine-owned, which is why `PGE-004` exists.

**2. Make "open" mechanical (`PCR-003`).** Two candidates. (a) Extend the authorized archive move from
issues to tickets: a ticket whose residuals are all `satisfied` and whose `landed-commit` is recorded may
move to `.arca/ticket/archive/`, and "open" means "present in `.arca/ticket/`". (b) Keep every ticket in
place and derive "open" from evidence: a ticket is open while any residual it owns is not `satisfied`.
(b) needs no new rule and no move, and it is already how the 2026-07-24 close reasoned in practice; (a)
matches the issue rule and keeps the directory small. P1 picks one - the requirement is that a guard, not a
reader, can tell the difference.

**3. Free the paths (`PCR-006`).** The gate kinds that hard-code `.arca/*` (`intake_contract`,
`record_contract`, and the goal freeze check) take their roots as guard fields declared in the runbook, with
the current values as the defaults so nothing changes for an existing runbook. This is what lets the cycle
runbook say which directories it governs, and it is the difference between a generic engine and one with a
methodology compiled in.

**4. Write the cycle runbook (`PCR-001`, `PCR-002`, `PCR-005`).** Phases follow the working rules already
written down: intake and integration (P1), freeze and gap check (P2), cut tickets (P3), tests-first (P4),
implementation and close (P5), with the loop edge from P5 back to P4 while tickets remain and the exit edge
to a rest state when the gap check comes back clean. Guards are the ones that already exist -
`intake_contract` at the P1 exit, `record_contract` and the goal freeze at P2, ticket ownership and
acyclicity at P3, `sensitivity_receipts` at P4, `completion_gate` at P5 - which is the honest test of
whether the closed guard vocabulary is sufficient. Where it is not, the finding is a new guard-kind issue,
not a widening of scope here. `rtm status` then answers the stage question, and `.arca/index.md`'s lookup
table is demoted to the no-Run fallback it already claims to be.

**Sequencing note.** Parts 1-3 are prerequisites discovered by trying to write part 4 in the abstract; each
is small and independently provable. Part 4 is the only one that produces the endpoint, and it is worth
nothing if the first three are skipped - a cycle runbook that fails its own ownership lint, cannot say
whether a ticket is open, and duplicates paths the Engine already knows is a demonstration, which is what
exists today.

## Non-goals

- No new guard kind is introduced by this issue. If the cycle cannot be expressed in the closed vocabulary,
  that discovery becomes its own issue.
- No change to the P1-P5 rules themselves. This encodes the working rules as they stand; changing them is a
  steering pivot, not an encoding task.
- No agent-spawning, no process management, no scheduling: the Run is still stepped by a caller.
