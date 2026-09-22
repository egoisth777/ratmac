# Issue specification

## Source and observed gap

`src/blocked.rs::plan_hold` and `apply_hold` both call `scheduler.machine().blocked_route_for`; `src/scheduler.rs::resolve_state_scope` already selects the durable child class for step and status.

The exact selected wish is **A held child Run should be looked up in its own class** in [Wishlist](../../wishlist.md), attributed to t-090 hidden-lane observation, filed 2026-08-10. Billy's 2026-09-22 Workflow dispatch authorizes preparing this issue; integration was accepted on 2026-09-22.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WRS-002` | Both planning and applying an addressed hold resolve the blocked route in the Run's recorded owning class. A child whose current State declares a blocked route can be held, even when its State name overlaps the parent or a sibling class; another class's route never substitutes for its own. The hold preserves Run identity, records blocked status and the opaque blocker only in Engine-owned state, and leaves the parent, siblings, workflow files, pins, and previous evidence unchanged. Confirmation, terminal refusal, containment, stale-plan checks, and write-free refusals remain enforced. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements) |

## Constraints

Existing [product requirements](../../goal/spec.md) and [working rules](../../schema.md) remain authoritative until explicit integration. Historical records keep their bytes. The Engine alone writes runtime state; a contributor's claim never authorizes motion or proves completion.

## Dependencies

Independent Engine repair. It precedes exercising pause routes on the Plan-Build child class in the cycle-pause issue.
