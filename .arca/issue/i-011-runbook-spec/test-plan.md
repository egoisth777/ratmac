# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| RBSV-001 | RBS-002 | Every guard kind appearing in `src/` has a spec entry; every spec entry appears in code — the two lists match (grep GuardKind variants vs spec table). |
| RBSV-002 | RBS-004 | Parser/doctor/authoring issue texts and their landed docs cite spec sections; no second definition of any schema term outside the spec. |
| RBSV-003 | RBS-005 | R-002/R-003/R-011 each traceable to a spec statement; existing lanes stay green (baseline 156 passed / 0 failed). |
| RBSV-004 | RBS-003 | Ownership rules in the spec name the same surfaces ownership::audit_ownership enforces; no rule without an enforcer or an explicit "prose-only" mark. |

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
