# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| MCV-001 | FDC-007 - FDC-010 | Every ask traces to its decision record in the seed's design ([design.md](../i-016-fsm-doctrine-convergence/design.md) - Adopted defaults for FDC-007, FDC-008, FDC-010; Individual human rulings for FDC-009) and to the research sections it condenses (`.arca/research/re-ratmac-FSM/05-invocation-join.md`, `07-conceptual-model.md`). |
| MCV-002 | FDC-007 | `spawn` proceeds with no confirmation phrase; `respawn` and abandon-with-run-id refuse without a phrase naming the run id and proceed with one, the refusals recorded as behavioral evidence. |
| MCV-003 | FDC-008 | A runbook with a cycle whose every phase carries a receipt- or contract-class guarded out-edge passes the termination check; removing that edge from one phase on the cycle fails it by kind membership. |
| MCV-004 | FDC-009 | A runbook carrying the class and spawn tables parses without the format-restriction refusal the review cites (`RBS-004`); the `blocked-route` spelling (hyphen) is accepted as canonical. |
| MCV-005 | FDC-010 | Child-as-reviewer works in the first increment; no witnessed verdict verb exists in the surface, and its deferral is recorded, not silently dropped. |
| MCV-006 | FDC-007 - FDC-010 | The recursion-depth open question in [spec.md](spec.md) remains recorded and unresolved; no ask, ticket, or mechanic here answers it. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | proposed | Reverse link to this issue. |
| `.arca/goal/ubi-lang.md` | proposed | Spawn ledger, child-as-reviewer, witnessed verdict verb - if integration coins them goal-side. |
| `.arca/goal/spec.md` | proposed | This issue's accepted requirement records, each linking back here. |
| `.arca/goal/design.md` | proposed | Whatever composition mechanics P1 accepts from the settled research files. |
| `.arca/goal/test-list.md` | proposed | This issue's verification checks. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | — |
| `.arca/schema.md` | proposed | The spawn/respawn confirmation-phrase surface (FDC-007) touches the working rules' authorization surface; working rules change only at integration, never from this issue directly. |
