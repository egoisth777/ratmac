# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `FDC-002` | The Engine writes `passed` when `rtm start` begins in a terminal Phase or `rtm step` arrives at one. Explicit abandonment writes a durable terminal event before active state is retired; `abandoned` is never a surviving State File value. Guard refusal remains non-terminal and leaves Run state unchanged. No path writes `failed` until a later issue defines a concrete Engine-observable failure event. | accepted | Billy selected the deferred bundle on 2026-08-03; completion is the next stratum because composition joins and the cycle runbook consume the Engine-written terminal fact. | [goal FDC-002](../../../goal/spec.md#integrated-run-completion-requirements) |

## Boundaries

- Independent of the input-routed-transition issue ([i-016-fsm-doctrine-convergence](../i-016-fsm-doctrine-convergence/spec.md)) and input-delivery issue ([i-019-input-delivery-durability](../i-019-input-delivery-durability/spec.md)).
- Builds on the integrated Run-residency contract only for the addressed State File and Run evidence location.
- Does not define `failed`. Steering already forecasts that later contract; no duplicate failure issue is created by this cut.
- Does not turn guard refusal into failure, because refusal must leave state untouched.

## Split record

On 2026-07-30 Billy split the doctrine-convergence seed into atomic concerns. This issue became the sole pending home of `FDC-002`; the seed keeps the adversarial-review ledger and decision history as evidence, while the input-routed-transition issue retains `FDC-001` and the input-delivery issue owns `FDC-003`. Requirement identifiers were not renumbered.
