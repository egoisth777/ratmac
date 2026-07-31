# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `FDCV-015` | `FDC-003` | Interruption before the verdict rename leaves the Run in the old state with the live verdict record intact. |
| `FDCV-016` | `FDC-003` | Interruption after the rename but before the State File write leaves the Run in the old state requiring a fresh verdict; the archived verdict cannot replay. |
| `FDCV-017` | `FDC-003` | Interruption after the State File write leaves the Run correctly advanced, with the verdict already archived and its live slot cleared. |
| `FDCV-018` | `FDC-003` | The Engine extracts one transition input value from the external judge's record and never authors or substitutes that value itself. A malformed record or value outside the current state's legal list refuses without changing Run state or consuming the record. |
| `FDCV-019` | `FDC-003` | Repeated visits to one state archive verdicts under distinct immutable evidence names; no visit overwrites or replays earlier evidence. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | proposed | Add the accepted input-delivery contract and link this issue at planning step 1. |
| `.arca/goal/ubi-lang.md` | proposed | Distinguish the legal input list, transition input value, live verdict record, and archived verdict without silently renaming the accepted Verdict slot. |
| `.arca/goal/spec.md` | proposed | Move `FDC-003`'s accepted forward authority to this issue. |
| `.arca/goal/design.md` | proposed | Define the accepted record shape, address derivation, and atomic consume-before-advance mechanics. |
| `.arca/goal/test-list.md` | proposed | Adopt `FDCV-015` through `FDCV-019` or their accepted successors. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No contributor-authority change. |
| `.arca/runbook-spec.md` | proposed | If the accepted live record or state input list adds Machine Class keys, define those keys and diagnostics only in the runbook-format single source of truth. |
| `.arca/runbook-authoring.md` | proposed | Add one repair row per accepted diagnostic code; do not define schema facts here. |
