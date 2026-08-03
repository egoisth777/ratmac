# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| MCV-001 | FDC-007 - FDC-010 | Every ask traces to its decision record in the seed's design ([design.md](../i-016-fsm-doctrine-convergence/design.md) - Adopted defaults for FDC-007, FDC-008, FDC-010; Individual human rulings for FDC-009) and to the research sections it condenses (`.arca/research/re-ratmac-FSM/05-invocation-join.md`, `07-conceptual-model.md`). |
| MCV-002 | FDC-007 | `spawn` proceeds with no confirmation phrase; `respawn` and abandon-with-run-id refuse without a phrase naming the run id and proceed with one, the refusals recorded as behavioral evidence. |
| MCV-003 | FDC-008 | A runbook with a cycle whose every phase carries a receipt- or contract-class guarded out-edge passes the termination check; removing that edge from one phase on the cycle fails it by kind membership. |
| MCV-004 | FDC-009 | A runbook carrying the class and spawn tables parses without the format-restriction refusal the review cites (`RBS-004`); the `blocked-route` spelling (hyphen) is accepted as canonical. |
| MCV-005 | FDC-010 | Child-as-reviewer works in the first increment; no witnessed verdict verb exists in the surface, and its deferral is recorded, not silently dropped. |
| MCV-006 | FDC-012 | The recursion-depth ruling is recorded with its provenance (2026-08-03, Billy); a spawn addressed to a Run that is itself a ledger-recorded child refuses naming the one-level cap. |
| MCV-007 | FDC-011 | The spawn ledger at the reserved per-run path is Scheduler-owned and append/annotate-only: spawn appends an entry with the recorded fields; abandon flips only the abandoned mark; respawn appends the successor entry naming the superseded id; a ledger entry whose child Run is missing on disk makes the join refuse naming that child. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | integrated | "Integrated machine composition" section links back to this issue. |
| `.arca/goal/ubi-lang.md` | integrated | Spawn, Spawn ledger, Respawn, Join, Child-as-reviewer, Recursion depth cap, Witnessed verdict verb coined goal-side. |
| `.arca/goal/spec.md` | integrated | `FDC-007`-`FDC-012` under "Integrated machine-composition requirements", each Source cell linking back to [spec.md](spec.md). |
| `.arca/goal/design.md` | integrated | "Machine composition (FDC-007-FDC-012)", integrated from this issue's [design.md](design.md) and the settled research files. |
| `.arca/goal/test-list.md` | integrated | `MCV-001`-`MCV-007` under "Integrated machine-composition verification". |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | — |
| `.arca/schema.md` | integrated | The working rules' authorization surface states the split at integration: `spawn` is ordinary motion; `respawn` and abandon-with-run-id take phrases naming the run id. |
