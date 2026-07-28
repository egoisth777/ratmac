# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| AALV-001 | AAL-002 | Scaffold output run through `rtm doctor` exits clean (code 0), enforced by a test so it stays true. |
| AALV-002 | AAL-003 | Seeded-defect drill: a runbook with known defects is repaired to doctor-clean using only the `--json` diagnostics and the instructions doc's code table. |
| AALV-003 | AAL-001 | End-to-end: from scaffold + instructions doc alone (no reading `src/`), a fresh agent or scripted stand-in produces a doctor-clean nontrivial runbook. |
| AALV-004 | AAL-001, AAL-004 | The instructions doc contains no schema definition of its own — every schema statement is a link into the runbook spec (reviewed by grep for guard-kind names). |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | unaffected | — (pending P1) |
| `.arca/goal/ubi-lang.md` | unaffected | — (pending P1) |
| `.arca/goal/spec.md` | unaffected | — (pending P1) |
| `.arca/goal/design.md` | unaffected | — (pending P1) |
| `.arca/goal/test-list.md` | unaffected | — (pending P1) |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | — |
| `none` | unaffected | — |
