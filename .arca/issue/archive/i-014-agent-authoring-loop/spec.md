# Issue specification

Dispositions below were confirmed at P1 on 2026-07-28; every ask was accepted and now lives in the goal specification.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `AAL-001` | An agent-facing schema instructions doc exists: how to write a runbook against the spec — routed to the runbook spec (i-011), restating nothing. | accepted | Agents currently write runbooks by imitating existing ones; imitation copies accidents. | [Goal specification](../../../goal/spec.md#integrated-authoring-loop-requirements) |
| `AAL-002` | Scaffold output: a generated starting runbook that passes doctor clean. | accepted | Starting from valid turns authoring into editing, which agents do far better. | [Goal specification](../../../goal/spec.md#integrated-authoring-loop-requirements) |
| `AAL-003` | The write → doctor → repair loop consumes the doctor's `--json` diagnostics (i-013/DRD-006); repairs address named codes, not guesses. | accepted | A loop that scrapes prose breaks the first time wording changes. | [Goal specification](../../../goal/spec.md#integrated-authoring-loop-requirements) |
| `AAL-004` | Ordering: this issue builds on i-012 and i-013 and cites i-011; it lands last on the route and duplicates no schema prose. | accepted | Steering route order — spec → parser → doctor ∥ authoring — with authoring's doctor dependency explicit. | [Goal specification](../../../goal/spec.md#integrated-authoring-loop-requirements) |
