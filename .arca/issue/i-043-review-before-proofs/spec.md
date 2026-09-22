# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WCP-004` | The build order obtains an independent review of the proposed source and tests before executing the expensive private-lane and deliberate-damage proof set. The accepted review identifies the exact reviewed snapshot. A rejected review returns to repair without running that proof set. Later source or oracle changes invalidate the affected review and proofs rather than claiming a once-only guarantee over changed bytes. A clean reviewed snapshot earns one complete private/damage proof set after review. This once-only rule applies to earning that unchanged proof, not to mandatory restored-green checks after each mutation or the declared post-merge verification from main; both remain required. Later source or oracle repair still requires renewed affected proof. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../schema.md#wcp-004--review-before-proofs) |

## Evidence and boundaries

[Working step P5](../../schema.md#the-steps-p1p5) currently places short review after private lanes and damage checks. [Snapshot support](../../../test/qa/src/snapshot.rs) already records file digests and tracking state. Historical repeated review failures are provenance from the wishlist, not new test results.

Existing files are observed evidence. Proposed checks below have not been executed and are not satisfaction evidence.

## Assumptions

Move the existing short review earlier and bind it to snapshot evidence, rather than removing review or relaxing hidden lanes. Final evidence checking may still catch defects; a necessary repair legitimately requires fresh affected proofs.

## Dependencies

Compose with checkpoint-only restoration and selected-issue completion. This order does not settle the separate reviewer identity mechanism.

## Open decisions

No unresolved disposition remains. The requirement was accepted on 2026-09-22; the linked forward authority governs implementation and its adopted assumptions.
