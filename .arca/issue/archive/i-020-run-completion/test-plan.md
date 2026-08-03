# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `FDCV-011` | `FDC-002` | `rtm start` writes `passed` when the initial state has no ordinary outgoing edge. |
| `FDCV-012` | `FDC-002` | `rtm step` writes `passed` when its selected transition arrives at a state with no ordinary outgoing edge. |
| `FDCV-013` | `FDC-002` | Confirmed abandonment writes one durable terminal event before retiring active state; `abandoned` never survives as a State File value. |
| `FDCV-014` | `FDC-002` | No Engine path writes the deferred `failed` outcome, and guard refusal leaves Run state byte-identical rather than marking it terminal. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | proposed | Add the accepted Run-completion contract and link this issue at planning step 1. |
| `.arca/goal/ubi-lang.md` | proposed | Define terminal state, Passed Run, abandoned event, and the deferred failed outcome. |
| `.arca/goal/spec.md` | proposed | Move `FDC-002`'s accepted forward authority to this issue. |
| `.arca/goal/design.md` | proposed | Define the accepted completion and terminal-event write points. |
| `.arca/goal/test-list.md` | proposed | Adopt `FDCV-011` through `FDCV-014` or their accepted successors. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No contributor-authority change. |
| `.arca/runbook-spec.md` | proposed | State that terminality is structural and that lifecycle status remains Engine-owned rather than a runbook key. |
| `.arca/runbook-authoring.md` | proposed | Explain terminal states by linking to the runbook-format single source of truth; define no schema facts here. |
