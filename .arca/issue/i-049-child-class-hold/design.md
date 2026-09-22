# Issue design

## Proposed mechanics

- Expose or reuse one owning-scope route lookup from the Scheduler instead of reading its top-level graph in `src/blocked.rs`.

- Resolve from the durable ledger class during planning and revalidate under the addressed Run lock during apply. Use the child's recorded workspace for blocker containment.

- A missing class, missing own route, ambiguous ownership, or forged/stale plan refuses by name before writing; do not guess from matching State names.

## Scope and assumptions

These are minimal proposed mechanics under Billy's 2026-09-22 instruction to proceed without questions. They authorize no source edit, runtime mutation, historical rewrite, or completed-work claim in this issue-creation stage.

Independent Engine repair. It precedes exercising pause routes on the Plan-Build child class in the cycle-pause issue.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.

