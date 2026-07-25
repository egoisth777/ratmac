# Phase gates verify contracts and route blockers honestly

```yaml
issue-id: "i-007-contract-verifying-gates"
provenance: "Observed branch recovery evidence, 2026-07-24: the abandoned, uncommitted self-hosted Runbook experiment (formerly branch feat/ratmac-rombook-test at baseline e68bc51) and its independent advisor review. Observed there: the intake gate accepted five filenames plus a status; the residual and ticket gates trusted status fields; the P4 gate matched a free-text log line containing the ticket id plus expected-red or sensitivity wording and any test filename containing the ticket number; the P5 gate passed once statuses were relabeled, without running or verifying the promised tests; the P4 prompt told agents to append to the Scheduler-owned log; and an honestly blocked executing ticket had no route out of P5; and the broken active Run itself had no authorized abandonment path, so retiring its admission state to start fresh required discarding the worktree by hand outside RTM. The branch and its untracked artifacts are discarded, so the findings are restated here without links to them."
status: "integrated"
```

## Summary

In the discarded run, the mechanized P1–P5 gates checked shapes and statuses, not the work: an issue could claim `integrated` without its requirement IDs existing in the goal; a residual could claim `satisfied` without evidence; P4 sensitivity was proven by a matching prose line in the log plus a suitably named test file; and P5 routed on relabeled statuses without ever running or verifying the ticket's promised focused, regression, hidden-lane, and quality checks. The same prompts created an ownership contradiction by directing agents to append hole-poking notes to the Scheduler-owned append-only log. Finally, when a real out-of-scope failure blocked a ticket, the loop had only completion-shaped routes, so the honest state — executing ticket, partial residual — could neither park, intake its blocker, nor return to planning. One level up, a Run broken beyond repair had no terminal route at all: nothing could retire the active Run’s admission state without hand-editing Scheduler-owned files. This issue makes every phase gate verify its phase's contract with executable evidence, moves agent-authored evidence into agent-writable artifacts, adds a human-authorized blocked route, and adds safe human-authorized Run abandonment so a broken active Run can be retired honestly instead of stranded or hand-edited. These requirements bind the re-attempted self-hosted plan-build loop on this branch; where no such gate exists yet, the requirement is unmet, never vacuously satisfied. Dispositions in the specification record the author's proposed decision; P1 confirms or revises them at integration.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Integration

Folded into the goal on 2026-07-24 (P1). Every requirement disposition in [spec.md](spec.md) was confirmed `accepted`.

| Goal artifact | What it now carries |
| :--- | :--- |
| [Goal specification](../../../current/spec.md) | Requirement records `PGE-001`–`PGE-007` |
| [Goal design](../../../current/design.md) | The accepted mechanics for this issue |
| [Goal test list](../../../current/test-list.md) | Checks `PGEV-002`–`PGEV-009` |
| [Goal ubiquitous language](../../../current/ubi-lang.md) | This issue's terms |
| [Goal index](../../../current/index.md) | Reverse link to this issue |
