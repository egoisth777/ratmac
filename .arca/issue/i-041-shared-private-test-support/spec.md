# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WCP-002` | Current private regression lanes use one maintained support surface for fixture construction, process execution, snapshots, and fault setup, with isolated process environments and filesystem state. One aggregate runner accounts for every declared live lane or valid expiry without silent skips. A documented current-versus-history boundary preserves archived evidence and every migrated lane's identity, requirement, and assertion strength. The migration covers existing live duplication, names any retained exception, and measures full-sweep growth rather than merely adding a helper unused by current lanes. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../schema.md#wcp-002--shared-private-test-support) |

## Evidence and boundaries

[Baseline support](../../../test/qa/src/baseline.rs), [snapshot support](../../../test/qa/src/snapshot.rs), and [lane sweep](../../../tools/sweep_lanes.py) already exist. The [lane roster](../../../.ratmac/lanes.toml) accounts for landed crates; this is a consolidation task, not a missing-runner claim.

Existing files are observed evidence. Proposed checks below have not been executed and are not satisfaction evidence.

## Assumptions

Extend the existing Rust test/qa support and existing lane sweep; do not add another runner or rewrite archived evidence. Migrate current non-expired crates in bounded tickets, using a shared build output only where package/features and process isolation remain correct.

## Dependencies

Contributor-tools lane should settle ownership first. Preserve landed-lane sweep, recovery, and expiry contracts.

## Open decisions

No unresolved disposition remains. The requirement was accepted on 2026-09-22; the linked forward authority governs implementation and its adopted assumptions.
