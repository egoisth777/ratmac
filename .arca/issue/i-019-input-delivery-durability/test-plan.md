# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `FDCV-015` | `FDC-003` | Interruption before the verdict rename leaves the Run in the old state with the live verdict record intact. |
| `FDCV-016` | `FDC-003` | Interruption after the rename but before the State File write leaves the Run in the old state requiring a fresh verdict; the archived verdict cannot replay. |
| `FDCV-017` | `FDC-003` | Interruption after the State File write leaves the Run correctly advanced, with the verdict already archived and its live slot cleared. |
| `FDCV-018` | `FDC-003` | The Engine extracts one transition input value from the external judge's record and never authors or substitutes that value itself. A malformed record or value outside the current state's legal list refuses without changing Run state or consuming the record. |
| `FDCV-019` | `FDC-003` | Repeated visits to one state archive verdicts under distinct immutable evidence names; no visit overwrites or replays earlier evidence. |

Accepted successor checks:

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `FDCV-020` | `FDC-003` | Start leaves the live slot absent; a branch without a record refuses after guards without changing State File or evidence. |
| `FDCV-021` | `FDC-003` | Recorded ordering proves all readiness guards precede verdict read/consume and archive rename precedes State File replacement. |
| `FDCV-022` | `FDC-003` | A live record at a straight-line Phase refuses untouched; with an empty slot, straight-line movement reads no verdict. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | integrated | Names the accepted input-delivery contract and carrying Ideal-shape property. |
| `.arca/goal/ubi-lang.md` | integrated | Distinguishes the legal list, transition input, live verdict record, and archived verdict without renaming the Verdict slot. |
| `.arca/goal/spec.md` | integrated | Accepts `FDC-003` with this issue as forward authority. |
| `.arca/goal/design.md` | integrated | Defines the strict record, absent-empty slot, archive sequence, and consume-before-advance ordering. |
| `.arca/goal/test-list.md` | integrated | Preserves `FDCV-015` through `FDCV-019` and adds accepted successor checks `FDCV-020` through `FDCV-022`. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No contributor-authority change. |
| `.arca/runbook-spec.md` | unaffected | The live verdict record is Run input, not a Machine Class key; `inputs` and `input` belong to the routing ticket's coordinated format update. |
| `.arca/runbook-authoring.md` | unaffected | No distinct delivery diagnostic code or runbook schema fact is added here. |
