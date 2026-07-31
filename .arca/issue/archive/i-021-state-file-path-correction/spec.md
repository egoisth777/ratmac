# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `SPC-001` | The goal describes the State File at `.arca/runs/<id>/state.toml`; the flat ownership clause keeps only the Machine Class, Transition Log, and invocation lock under `.arca/`. | duplicate | `FDC-004` already defines canonical per-Run residency. This corrects stale inherited wording instead of creating a second path contract. | [goal `R-024`, `R-025`, `FDC-004`](../../../goal/spec.md#integrated-run-residency-requirements); [goal path mechanics](../../../goal/design.md#state-layout--project-level-plus-per-run-files-adr-0008-superseded-in-part-by-fdc-004) |

`SPC` expands to **State Path Correction**. This issue adds no product requirement; planning step 1 dispositioned `SPC-001` as a duplicate of `FDC-004` and corrected the accepted goal in place.
