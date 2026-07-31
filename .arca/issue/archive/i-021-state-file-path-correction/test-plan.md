# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `SPCV-001` | `SPC-001`, `FDC-004` | Every inherited flat State File or Run-evidence statement is explicitly superseded; the unsuperseded path contract names `.arca/runs/<id>/state.toml` and `.arca/runs/<id>/evidence.toml`. |
| `SPCV-002` | `SPC-001`, `FDC-004` | Existing residency checks still prove that each addressed Run reads and writes `.arca/runs/<id>/state.toml` and `.arca/runs/<id>/evidence.toml`. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/spec.md` | integrated | `SPC-001` is a duplicate correction to `R-024`, `R-025`, and `FDC-004`. |
| `.arca/goal/design.md` | integrated | Marks the old flat projection superseded and retains canonical residency as authority. |
| `.arca/goal/test-list.md` | unaffected | Existing residency behavior checks remain authoritative. |
