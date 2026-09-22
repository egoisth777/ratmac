# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WCP-005` | Before deliberate damage, record the exact bytes and effective checkout conversion settings for every declared tracked mutation target. A restore is accepted only after recomputing each restored target's digest and matching its frozen digest, with index and tracked-tree state also verified. Refuse before damage when effective line-ending or checkout filters cannot reproduce those frozen bytes; never silently change global or repository Git settings. A clean Git status alone cannot pass a restore whose bytes differ, and a refusal names the affected path and corrective action. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../schema.md#wcp-005--byte-verified-restores) |

## Evidence and boundaries

[Snapshot support](../../../test/qa/src/snapshot.rs) supplies SHA-256 file digests. [Damage safety rules](../../schema.md#sdc-002--damage-only-from-a-checkpoint) require checkpoint restoration and clean status but do not specify byte-rehash acceptance. The wishlist records the historical line-ending counterexample.

Existing files are observed evidence. Proposed checks below have not been executed and are not satisfaction evidence.

## Assumptions

Use SHA-256 manifests already familiar to snapshot evidence. Verify the effective checkout behavior for declared targets, including attributes and filters, instead of assuming core.autocrlf alone describes it. Keep goal-fingerprint normalization separate: restored source bytes must remain exact.

## Dependencies

Implemented with the checkpoint-only restore mechanism; review-before-proofs establishes the frozen reviewed source it verifies.

## Open decisions

No unresolved disposition remains. The requirement was accepted on 2026-09-22; the linked forward authority governs implementation and its adopted assumptions.
