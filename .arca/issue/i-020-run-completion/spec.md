# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `FDC-002` | The Engine writes `passed` when `rtm start` begins in a terminal state or `rtm step` arrives at one, where a terminal state has no ordinary outgoing edge. Explicit abandonment writes a durable terminal event before active state is retired; `abandoned` is never a surviving State File value. Guard refusal remains non-terminal and leaves Run state unchanged. No path writes `failed` until a later issue defines a concrete, Engine-observable failure event; this issue adds no failure command and no per-runbook terminal vocabulary. | proposed | Completion and abandonment are Engine-owned lifecycle facts, independent of how a transition was selected. Provenance: human rulings for adversarial-review findings `AR-04` and `AR-05`, plus Billy's 2026-07-29 human ruling deferring `failed`, recorded in the evidence seed's [design](../i-016-fsm-doctrine-convergence/design.md). Billy's 2026-07-30 cut made this lifecycle boundary independently reviewable without changing those rulings. | — (pending planning step 1) |

## Boundaries

- Independent of the input-routed-transition issue ([i-016-fsm-doctrine-convergence](../i-016-fsm-doctrine-convergence/spec.md)) and input-delivery issue ([i-019-input-delivery-durability](../i-019-input-delivery-durability/spec.md)).
- Builds on the integrated Run-residency contract only for the addressed State File and Run evidence location.
- Does not define `failed`. Steering already forecasts that later contract; no duplicate failure issue is created by this cut.
- Does not turn guard refusal into failure, because refusal must leave state untouched.

## Split record

On 2026-07-30 Billy split the doctrine-convergence seed into atomic concerns. This issue became the sole pending home of `FDC-002`; the seed keeps the adversarial-review ledger and decision history as evidence, while the input-routed-transition issue retains `FDC-001` and the input-delivery issue owns `FDC-003`. Requirement identifiers were not renumbered.
