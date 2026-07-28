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
| `.arca/goal/index.md` | updated | Reverse link to `i-011-runbook-spec` under "Integrated Machine Class as first-class data". |
| `.arca/goal/ubi-lang.md` | updated | This issue's terms folded in; the `Exit Guard` entry now routes to the runbook specification instead of enumerating kinds. |
| `.arca/goal/spec.md` | updated | This issue's accepted requirement records, each linking back here. |
| `.arca/goal/design.md` | updated | Accepted mechanics recorded under "Machine Class made first-class". |
| `.arca/goal/test-list.md` | updated | This issue's verification checks. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | — |
| `none` | unaffected | — |
