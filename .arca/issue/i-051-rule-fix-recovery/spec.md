# Issue specification

## Source and observed gap

The wishlist records that run-002 carried three approved tickets and was abandoned when the archived-record rule was wrong; the fix in archived issue i-029 could not restore its position. `src/pin.rs::Evidence` stores original Engine, gate, goal, and runbook identities; existing ordinary operations reject changed pins rather than authorizing a repair.

The exact selected wish is **A Run parked on a rule defect survives that rule's fix** in [Wishlist](../../wishlist.md), attributed to run-002 postmortem, filed 2026-08-10. Billy's 2026-09-22 Workflow dispatch authorizes preparing this issue; integration was accepted on 2026-09-22.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WRS-004` | An explicitly authorized Engine recovery can continue a nonterminal Run stopped by a corrected rule without changing its Run address, owning class, bindings, workspace, or current State, deleting its proof, retiring it, or rewriting its original pins. Authorization is verified under the prior trusted policy or a previously enrolled recovery authority, never the proposed replacement policy. The signed authorization binds the Run, full old and new authority identities, and every enumerated change; a confirmation phrase alone cannot authorize replacement. Recovery cannot enroll its own approver. A Run lacking previously enrolled recovery trust requires explicit independent external enrollment before recovery is eligible. The recovery durably records the identities, affected rule, verified authorization, and evidence boundary as a forward-only amendment. It validates the corrected class and State and independently rechecks all affected prerequisites and proof; stale receipts or agent-authored claims never establish success. Missing authorization, incompatible corrections, terminal Runs, corrupt evidence, and interruption leave the old authority effective or one complete recoverable amendment, never a partial silent repin. An amendment does not resume a paused Run or approve a transition. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements) |

## Constraints

The accepted [product requirements](../../goal/spec.md) and [working rules](../../schema.md) are authoritative. Historical records keep their bytes. The Engine alone writes runtime state; a contributor's claim never authorizes motion or proves completion.

## Dependencies

Requires the authorized-review requirement WRA-001 in issue i-062, extended to authorize authority replacement using prior trusted policy or independently enrolled recovery authority. Keep independent from respawn: recovery preserves the original Run address. Child-class hold is required only for the composed-child recovery tests. A Run without the required external trust enrollment is ineligible until an authorized independent operator provides it; the builder may neither generate that trust nor treat user task authorization as signed recovery evidence.
