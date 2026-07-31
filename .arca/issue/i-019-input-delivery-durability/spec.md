# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `FDC-003` | One live verdict record belongs to the addressed Run and current Phase. An external evidence reviewer authors the strict record with exactly one transition input plus rationale; the Engine makes no judgment of its own. After readiness guards pass, the Engine validates the record, atomically renames it into immutable Run evidence, thereby clearing the live slot, and only then writes the successor State File. Distinct monotonic evidence names preserve repeated visits. An interruption after consumption but before state advance leaves the old Phase awaiting a fresh verdict; archived input never replays. | accepted | The consume-before-advance human ruling makes stale-input replay impossible while preserving external judgment. Accepted after `FDC-001` at planning step 1 on 2026-07-30. | [goal `FDC-003`](../../goal/spec.md#integrated-input-delivery-and-durability-requirements); mechanics in [goal design](../../goal/design.md#durable-transition-input-delivery-fdc-003); [goal checks](../../goal/test-list.md#integrated-input-delivery-and-durability-verification) |

## Boundaries

- Depends on the legal input list and labelled-edge contract in the input-routed-transition issue ([i-016-fsm-doctrine-convergence](../i-016-fsm-doctrine-convergence/spec.md)).
- The accepted live slot remains `.arca/runs/<id>/verdict.toml`; it is absent when empty. The accepted strict fields are `phase`, `input`, and `rationale`, and consumed records move under that Run's `verdicts/` evidence sequence. These mechanics are fixed in goal design.
- This is not a silent rename of the accepted `Verdict slot`; it gives the reserved `verdict.toml` location a contract and lifecycle.
- Does not define signer identity, witnessed verdicts, or human approval. Those remain deferred.
- Does not define judge independence; requirements `FDC-009` and `FDC-010` remain solely in the machine-composition issue ([i-018-machine-composition](../i-018-machine-composition/spec.md)).

## Split record

On 2026-07-30 Billy split the doctrine-convergence seed into atomic concerns. This issue became the sole pending home of `FDC-003`; the seed keeps the adversarial-review ledger and decision history as evidence, while the input-routed-transition issue retains `FDC-001` and the Run-completion issue ([i-020-run-completion](../i-020-run-completion/spec.md)) owns `FDC-002`. Requirement identifiers were not renumbered.
