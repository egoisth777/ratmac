# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `FDC-003` | One live verdict record belongs to the addressed Run and current state. An external judge authors the record from evidence; the record carries exactly one transition input value plus rationale, while the Engine makes no judgment of its own. When that value selects a transition, the Engine atomically moves the live record into immutable Run evidence, thereby clearing the live slot, before it writes the successor State File. The evidence name is collision-free for repeated visits to the same state. An interruption after consumption but before state advance leaves the Run at the old state awaiting a fresh verdict; the archived verdict never replays. | proposed | This separates judgment from execution and makes stale-input replay impossible. Provenance: Billy's 2026-07-30 human clarification assigns judgment to the designated external judge and only validation, consumption, and routing to the Engine; the consume-before-advance ordering, atomic rename, and collision-free evidence naming are the human ruling and refinement for adversarial-review finding `AR-06`, recorded in the evidence seed's [design](../i-016-fsm-doctrine-convergence/design.md). The same day's authorized cut made this boundary independently reviewable. | — (pending planning step 1) |

## Boundaries

- Depends on the legal input list and labelled-edge contract in the input-routed-transition issue ([i-016-fsm-doctrine-convergence](../i-016-fsm-doctrine-convergence/spec.md)).
- Does not define exact TOML field names or physical slot spelling. Planning step 1 must reconcile the accepted per-Run `Verdict slot` and current `verdict.toml` reservation with state-specific addressing, then place the accepted format in the runbook-format single source of truth ([runbook-spec.md](../../runbook-spec.md)).
- Does not rename the accepted `Verdict slot` or `verdict.toml`; such a rename would require an explicit accepted-goal, source, and test migration.
- Does not define signer identity, witnessed verdicts, or human approval. Those remain deferred.
- Does not define judge independence; requirements `FDC-009` and `FDC-010` remain solely in the machine-composition issue ([i-018-machine-composition](../i-018-machine-composition/spec.md)).

## Split record

On 2026-07-30 Billy split the doctrine-convergence seed into atomic concerns. This issue became the sole pending home of `FDC-003`; the seed keeps the adversarial-review ledger and decision history as evidence, while the input-routed-transition issue retains `FDC-001` and the Run-completion issue ([i-020-run-completion](../i-020-run-completion/spec.md)) owns `FDC-002`. Requirement identifiers were not renumbered.
