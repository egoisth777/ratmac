# Turn housekeeping is one lifecycle, not a remembered order

```yaml
issue-id: "i-035-turn-housekeeping-automation"
provenance: "Wishlist 'Automate ticket and cycle housekeeping end to end' (Advisor, 2026-08-03), plus recurrence evidence from the 2026-08-21 sprint: git worktree remove --force again destroyed a green turn's only hidden-lane crate (t-102's gitignored test-hidden/t-102/, before copy-back; recovered only because the authoring context was still alive) - the same loss class as t-076 (.arca/log.md:310) under a rule that already fixes the order in prose (.arca/schema.md, Units and git: cycle-end git discipline)"
status: "integrated"
```

## Summary

Every build turn repeats the same choreography by hand: open a linked worktree
on a branch named after the work item, copy the untracked lanes root in, work
the turn, then at green — merge into `main`, copy the lanes back, remove the
worktree and branch, stamp the item record's landed commit, append the log
line, and re-run the lanes once from `main`. The working rules already fix
this order in prose ([Units and git](../../../schema.md#units-and-git)):
copy-back second, "before any removal, because the new crate is gitignored
and committed nowhere, so removing the worktree first destroys its only
copy", removal third, the `main` rerun last.

The prose has now failed twice in practice. On 2026-08-06 the t-076 turn ran
`git worktree remove --force` before copy-back and destroyed
`.arca-private/t-076/` — no backup survived
([log.md:310](../../../log.md)). On 2026-08-21 the same slip destroyed the
t-102 turn's `test-hidden/t-102/` crate; recovery succeeded only because the
authoring context still held the bytes — luck, not a mechanism. That sprint
ran four consecutive turns entirely by hand
([log.md:433-436](../../../log.md): t-102 through t-105, each ending "turn
passed"), so each landing was a fresh occasion for the same end-of-turn
slip, and the log's landing lines record the merges without recording the
steps between them — the choreography leaves no durable trace a reviewer
could audit.

This issue proposes mechanizing the turn lifecycle after the shape the trial
lifecycle already proved (`TWL-001..010`): one open command with the lanes
carried in, one close command that completes the fixed order with removal
last, a refusal that blocks any destructive removal while it holds the only
copy of an artifact, and a dry run with recovery commands — ownership
mirroring the trial verbs. The asks are phrased over work items and declared
data (`NRR-001`, `PCR-007`); tickets are this repository's instance, not the
concept.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## History

- 2026-08-24: filed from the wishlist by the DESIGNER, carrying the
  2026-08-21 t-102 recurrence as fresh evidence. Dispositions are the
  author's proposal; P1 confirms or revises at integration.
- 2026-08-24: P1 integrated all four asks as accepted goal rows
  THK-001..THK-004; the command-boundary, lifecycle-home, and
  recovery-ownership choices are recorded at ADR-0018 in the goal design.
