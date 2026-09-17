# Issue design

## Proposed mechanics

1. **One lifecycle entry point, after the trial shape (`THK-001`, `THK-004`).**
   The trial lifecycle already owns its choreography end to end through one
   entry point with `status`/`start` verbs, check-before-first-write,
   compare-and-delete rollback, and printed recovery commands
   ([tools/trial.ps1](../../../../tools/trial.ps1); `TWL-001..003`, `TWL-006`,
   `TWL-009`). The turn lifecycle is the same problem one lane over: propose
   `open` and `close` beside a `status` dry run, invoked from the primary
   checkout under the same ownership table (`TWL-010`). Whether this ships as
   a sibling repo-local tool, a runbook-declared lifecycle the Engine drives,
   or an Engine subcommand is the open command-boundary choice named in the
   specification; the wish defers it to a human, and nothing below depends
   on the answer.

2. **Open carries the lanes in (`THK-001`).** Identity is deterministic and
   computed before mutation: branch `<item-id>-<slug>`, worktree a sibling
   directory, item id arriving as the Run's opaque binding (`PCR-007`) —
   never parsed. The lanes root, trunk branch, per-lane skip list, stamp
   field, and landing log are read from the runbook's typed declarations
   (the `[roots]` table t-076 landed), so a different project's cycle
   reuses the mechanism by declaring its own names. Precondition refusal
   mirrors `TWL-001`: zero mutation on any refusal.

3. **Close is the fixed order, removal last, resumable (`THK-002`).** The
   close verb starts where the turn's internal order ends — the green
   landing already exists per `SDC-003`, so close composes with the
   deliberate-damage discipline and changes none of it. Close then runs the
   prose duty mechanically: merge, verified copy-back, stamp, log line,
   worktree and branch removal, trunk lanes rerun. Each step's completion is
   recorded in Engine-adjacent state the dry run also reads, which is what
   makes resume exact: a re-invocation after interruption diffs recorded
   progress against the plan and continues from the first uncompleted step.

4. **The only-copy refusal (`THK-003`).** Before any removal, the close (or
   an explicit remove) inventories the worktree's untracked content against
   the primary checkout; anything that exists only in the worktree blocks
   the removal by name. This is the one guard that must also catch a human
   reaching for raw Git, so the proposal is that removal is only reachable
   through the lifecycle at all: the mechanics refuse to register the
   worktree as removable while the inventory is non-empty, and the refusal
   text carries the copy-back that clears it. A caller who truly wants the
   bytes gone declares the artifact obsolete by name — an explicit act, not
   a flag.

This file is incoming evidence. Integrated mechanics remain authoritative
only in the accepted forward authority.
