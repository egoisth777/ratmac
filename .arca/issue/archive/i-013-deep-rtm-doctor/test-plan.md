# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| DRDV-001 | DRD-002, DRD-003 | One fixture runbook per defect class; doctor reports each with its documented code at the right location. |
| DRDV-002 | DRD-006 | `--json` output parses; codes for identical fixtures are byte-identical across two runs. |
| DRDV-003 | DRD-007 | Clean fixture exits 0; warning-only fixture exits with the warning code; error fixture with the error code — three distinct values observed. |
| DRDV-004 | DRD-001 | Doctor's findings on a parse-refused file match the parser's refusal (same defect, doctor renders it as a diagnostic, no independent toml walk in the doctor module). |
| DRDV-005 | DRD-004 | An ownership violation fixture is reported via doctor; the finding originates from `ownership::audit_ownership` (no duplicated audit logic). |
| DRDV-006 | DRD-005 | `rtm doctor <tempfile path>` validates a file outside the repo's live runbook location. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | Reverse link to `i-013-deep-rtm-doctor` under "Integrated Machine Class as first-class data". |
| `.arca/goal/ubi-lang.md` | updated | This issue's terms folded in; the `Exit Guard` entry now routes to the runbook specification instead of enumerating kinds. |
| `.arca/goal/spec.md` | updated | This issue's accepted requirement records, each linking back here. |
| `.arca/goal/design.md` | updated | Accepted mechanics recorded under "Machine Class made first-class". |
| `.arca/goal/test-list.md` | updated | This issue's verification checks. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | — |
| `none` | unaffected | — |
