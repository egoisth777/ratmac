# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `RCRV-001` | `RCR-001` | The working rules state the stamp-landing order (stamp landing follows the merged green landing; the hash is derived by resolving the landed tip), and a scan finds no live rule instructing a record to carry the hash of a commit it lands inside. |
| `RCRV-002` | `RCR-002` | On this repository: before the allowlist the check refuses naming exactly the rotted citations (11 at filing — res-116/117, res-124/125, res-147..153); with the enumerated allowlist it passes; every allowlist row matches a real record, and deleting a matched row's record fails that row as stale. |
| `RCRV-003` | `RCR-002` | A fixture repository: a record citing a commit that a later amend orphans refuses, naming the record and the hash; the same record citing the amended (reachable) commit passes; a record citing a hash that never existed refuses; a record citing a commit held only by a tag passes. |
| `RCRV-004` | `RCR-003` | A fixture amend after stamping: one stamp step re-derives the record's `implementation-revision` and the ticket's `landed-commit` in the same change; the residual archive move refuses while either cites an unresolvable commit outside the allowlist and passes once re-pointed. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | Integrated records-cite-resolving-commits section at P1 |
| `.arca/goal/ubi-lang.md` | updated | Resolving citation, stamp landing |
| `.arca/goal/spec.md` | updated | RCR-001..RCR-003 |
| `.arca/goal/design.md` | updated | ADR-0019 records the carrier and reachability decisions |
| `.arca/goal/test-list.md` | updated | RCRV-001..RCRV-004 |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | |
| `.arca/schema.md` | unaffected at integration | `RCR-001`'s stamp-landing order lands as working-rules text with the ticket that proves it (`ADR-0019`) |
| `.arca/steering.md` | updated | Current sprint regenerated at P1 close; this issue advances the Every-boundary-machine-checked Ideal-shape property |
