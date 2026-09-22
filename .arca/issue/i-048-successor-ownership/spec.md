# Issue specification

## Source and observed gap

`src/scheduler.rs::Scheduler::respawn` releases the minted successor's Run lock, retires its predecessor, and only afterward appends the successor ledger entry. The historical source locations in the wish describe this same inherited interval; they are provenance, not today's line numbers.

The exact selected wish is **Record a spawned successor before it can be retired** in [Wishlist](../../wishlist.md), attributed to Advisor, 2026-08-06. Billy's 2026-09-22 Workflow dispatch authorizes preparing this issue; integration was accepted on 2026-09-22.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WRS-001` | A spawned replacement becomes available to public Run operations only after its owning parent ledger durably records its identity, class, bindings, workspace, and superseded Run. Concurrent step, spawn, hold, abandon, join, and roster reads must never observe an admitted successor without that ownership. Root-before-Run lock order and root-lock-free guard evaluation remain unchanged. Failed or interrupted replacement preserves the predecessor or records a complete recoverable replacement; an ambiguous write never causes deletion of potentially owned evidence, and identifiers are never reused. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements) |

## Constraints

Existing [product requirements](../../goal/spec.md) and [working rules](../../schema.md) remain authoritative until explicit integration. Historical records keep their bytes. The Engine alone writes runtime state; a contributor's claim never authorizes motion or proves completion.

## Dependencies

No dependency on another new issue. Coordinate with rule-fix recovery if it reuses replacement or publication helpers.
