# Issue design

## Proposed mechanics

- Use the existing mint and ledger boundaries in `src/scheduler.rs`, `src/ledger.rs`, and `src/lock.rs`; make publication one root-locked decision instead of releasing an admitted successor before ownership is durable.

- Prepare the replacement and validate the predecessor before destructive retirement. Keep any uncommitted replacement unavailable to public motion; distinguish absent, complete, and indeterminate ledger writes without guessing. Recovery must finish or safely cancel the same recorded operation rather than minting duplicates.

- Retain the confirmed `respawn <run id>` contract, child workspace and binding inheritance, append-only ledger history, and the one-level spawn cap. Do not hold the root lock while running guards.

## Scope and assumptions

These are minimal proposed mechanics under Billy's 2026-09-22 instruction to proceed without questions. They authorize no source edit, runtime mutation, historical rewrite, or completed-work claim in this issue-creation stage.

No dependency on another new issue. Coordinate with rule-fix recovery if it reuses replacement or publication helpers.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.

