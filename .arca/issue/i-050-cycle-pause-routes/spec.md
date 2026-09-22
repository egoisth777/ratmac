# Issue specification

## Source and observed gap

`.ratmac/ratmac.toml` has no `blocked-route = true` transition. Its working cycle States are intake, gap-check, cut-tickets, ticket-turns, and close; its working ticket States are tests, implement, and damage. A blocked self-route alone is insufficient: current step propagates blocked status, completion refuses a held Run, and spawn refuses a blocked parent. The capability therefore includes an explicit resume path.

The exact selected wish is **The cycle declares no human-authorized pause** in [Wishlist](../../wishlist.md), attributed to t-093 hidden-lane observation, filed 2026-08-10. Billy's 2026-09-22 Workflow dispatch authorizes preparing this issue; integration was accepted on 2026-09-22.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WRS-003` | The shipped Plan-Build Runbook declares exactly one human-confirmed blocked route for each nonterminal cycle and ticket State, preserving that State as its pause destination. A valid hold keeps the Run address, evidence, child ownership, and pins while recording blocked status and a linked blocker in Engine-owned state. Resume verifies exact human resolution authorization bound to that Run, its currently recorded opaque blocker, held-record digest, and unique hold occurrence, together with its unchanged owning class, State, pins, and existing entry prerequisites; under a governed policy the resolution authorization must also be signed through WRA-001. The Engine verifies authorization, not whether an external cause has actually been fixed. It then atomically clears the paused fact and restores executing status without advancing, retaining the hold, blocker, and resolution authorization in history; resume refusal before publication leaves persisted records and history unchanged; an interrupted committed resume reports recovery pending and is completed before further mutation. Resume neither refreshes receipts nor approves a transition: later completion and spawning still require their normal fresh checks. Ordinary step never takes a blocked route, terminal States declare none, and no pause fabricates passing proof. Activation happens only at a compatible cycle boundary and never by rewriting a live Run's pin. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements) |

## Constraints

The accepted [product requirements](../../goal/spec.md) and [working rules](../../schema.md) are authoritative. Historical records keep their bytes. The Engine alone writes runtime state; a contributor's claim never authorizes motion or proves completion.

## Dependencies

Depends on child-class hold for ticket-stage pauses and WRA-001 for signed resolution authorization under a governed policy. Rule-fix recovery is separate and required only when continuing after a pinned rule itself changes. Accepting an authority amendment under that recovery contract neither resumes this paused Run nor approves its next transition; resume requires its own exact resolution authorization and current prerequisite proof. The blocker remains opaque under NRR-001: document contents or disappearance cannot establish its resolution.
