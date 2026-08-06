# Issue design

## Proposed mechanics

### 1. One Engine root, one owner

    <primary-checkout>/.ratmac/       one shared Engine root per repository
      ratmac.toml                     tracked Machine Class
      evidence/<run-id>/              tracked receipts
      runs/<run-id>/state.toml        Git-ignored State File per Run
      runs/<run-id>/...               Git-ignored verdicts and spawn ledger
      mint.toml                       Git-ignored highest Run id ever issued
      locks/root.lock                 Git-ignored minting and roster or ledger mutation
      locks/<run-id>.lock             Git-ignored motion on one Run
      log.md                          Git-ignored Engine transition log

    <checkout>/.arca/                 workflow only: goal, issue, ticket, residual, steering

In a primary checkout the Engine root is `<checkout>/.ratmac/`. A linked worktree uses
Git worktree metadata to locate the primary checkout and resolves that checkout's
`.ratmac/` rather than creating a checkout-local root; without Git, the Engine resolves
`<current-checkout>/.ratmac/`. All worktrees therefore share one roster, one id namespace,
and one lock domain. `rtm status` and `rtm doctor` print the resolved Engine root. The
Machine Class is read from that resolved root, so a linked worktree uses the primary
checkout's runbook and evaluates its Run's pin against that runbook's hash.

Git ignores the runtime files under `.ratmac/`—`runs/`, `mint.toml`, `locks/`, and
`log.md`—while `ratmac.toml` and receipts under `.ratmac/evidence/<run-id>/` stay tracked.
Run state therefore cannot enter a ticket branch or a merge. Run-scoped tracked receipt
paths keep two parallel child Runs from writing the same receipt filename and colliding on
merge.

### 2. Minting and locking

`mint.toml` records the highest ordinal ever issued. Minting takes the root lock, reads
the record, advances it, writes the new Run directory, and releases the lock. Ordinals
stay human-legible because respawn and abandon confirmations are typed by a person.

Motion takes the per-Run lock only. Guard evaluation can run for minutes, so it must never
hold the root lock, otherwise parallel child Runs serialize behind one another. When both
locks are needed the order is root before Run.

### 3. Child workspace binding

    rtm spawn --run <parent-id> --class <name> --workspace <path>

The path is canonicalized and written into the child's ledger entry. Absent, the child
inherits the parent's workspace. The child Run's guards and file reads resolve against the
recorded workspace, while its machine position lives in the shared Engine root. The Engine
records topology and never creates, moves, or removes a worktree; a wrong path is recorded
honestly and refused at guard evaluation, not silently repaired.

### 4. Workflow roots as runbook data

    [roots]
    issue    = ".arca/issue"
    ticket   = ".arca/ticket"
    residual = ".arca/residual"
    goal     = ".arca/goal"

Guards name a root instead of a path. Static validation rejects an undeclared root name,
an absent root path, and any root that overlaps the Engine root, each with its own `RB`
code. This retires the `R-016` hard-coded-path debt: after the split, `src/` contains no
`.arca` literal, and a second project can point the same Engine at its own folders.

Editing `[roots]` changes the runbook hash, so live Runs refuse on pin drift. That is
`FDC-005` behaving correctly, and the refusal message must say so.

### 5. Migration by refusal

A pre-split live artifact — `.arca/ratmac.toml`, `.arca/runs/`, `.arca/rtm.lock`, or a flat
`.arca/state.toml` — makes every entry point refuse and print the exact operator steps. The
Engine moves nothing, matching the accepted `FDC-005` posture. Archived receipts under
`.arca/evidence/` are inert history and are not residue; new receipts live only at tracked
`.ratmac/evidence/<run-id>/`. This repository is hand-migrated once, before its first
post-split invocation.

### 6. Relationship to accepted requirements

- `FDC-004` — the canonical run path spelling moves to the Engine root, and "one id
  namespace" is restated at repository scope instead of checkout scope. Run addressing by
  `--run <id>` and the nesting of verdict and spawn-ledger paths under the Run directory are
  unchanged.
- `FDC-005` — the pin stays hash-only; a second residue class is added with the identical
  refuse-and-instruct posture.
- `FDC-006` — never-reuse becomes durable through the mint record instead of depending on
  retired directories remaining listed.
- `FDC-011` — the ledger workspace field goes from always empty to populated when bound.
- `R-016`, `R-024`..`R-026`, `ADR-0008` — path spellings and the "project-level under
  `.arca/`" clause are superseded; the transition log gains a single Engine writer in
  `.ratmac/` while `.arca/log.md` becomes human-only.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
