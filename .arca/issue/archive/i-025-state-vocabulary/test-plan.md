# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `SVCV-001` | `SVC-001`, `SVC-002` | A written check over the settled vocabulary: the working rules, the glossary, and the runbook specification each define State, Run Record, and Run separately, and no live document defines two of them with the same word. |
| `SVCV-002` | `SVC-003` | A runbook written with `states`, `[[states.<name>.spawns]]`, and `[classes.<name>.states]` parses, passes the doctor clean, and runs a Run from start to a terminal State; every other key keeps its behavior. |
| `SVCV-003` | `SVC-003`, `SVC-005` | A runbook that declares `phases` refuses with the new dedicated diagnostic naming the rename and the repair, not the generic unknown-key error, and no Run, Run Record, or evidence file is created. |
| `SVCV-004` | `SVC-004` | A started Run writes `.ratmac/runs/<run-id>/run.toml` whose first field is `state`; the seven-field strict parse, the atomic replacement, and the refusal on a corrupt record are unchanged, and no file at the pre-cutover name is created. |
| `SVCV-005` | `SVC-005` | With a pre-cutover Run Record planted, every public entry point refuses before its first read, path join, parse, or write, names the artifact and the repair, and leaves both the addressed project and the invoking checkout byte-identical. |
| `SVCV-006` | `SVC-006` | The doctor's documented code table equals the Engine's table; each pre-existing defect still reports its original code, and the pre-cutover runbook residue reports a code not previously in use. |
| `SVCV-007` | `SVC-002` | Every caller-visible surface — the State Prompt, `rtm status`, the human doctor report, `--json` findings, and refusal text — names the position State, and an executable scan finds no live occurrence of the old word in those surfaces. |
| `SVCV-008` | `SVC-007` | Routing, guard evaluation, freeze and drift, locking, minting, spawning, joining, holding, abandoning, completion, receipts, and exit codes are proven unchanged by the existing suites passing with their meanings intact after the rename. |
| `SVCV-009` | `SVC-008` | The audit passes over the whole tracked tree with an enumerated allowlist; removing any allowlist entry makes it fail on that history, and planting the old word in a live surface makes it fail there. |
| `SVCV-010` | `SVC-009`, `SVC-010` | The working rules, orientation, glossary, runbook specification, authoring instructions, steering, and blank forms read in the settled vocabulary, and the glossary states that renaming a term never rewrites history. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | `SVC-002` |
| `.arca/goal/ubi-lang.md` | updated | `SVC-001`, `SVC-002`, `SVC-004` |
| `.arca/goal/spec.md` | updated | `SVC-001` through `SVC-008` |
| `.arca/goal/design.md` | updated | `SVC-001` through `SVC-008` |
| `.arca/goal/test-list.md` | updated | `SVCV-001` through `SVCV-009` |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | Points at the schema; it names no machine position of its own. |
| `.arca/schema.md` | updated | `SVC-009`, `SVC-010` land as working-authority headings; the State section names the Run Record. |
| `.arca/index.md` | updated | `SVC-009` — orientation and the paths table name the Run Record and its field. |
| `.arca/dict.md` | updated | `SVC-009`, `SVC-010` — the `Phase` entry is replaced by State and Run Record entries, and the renaming rule states that history keeps its bytes. |
| `.arca/runbook-spec.md` | updated | `SVC-002`, `SVC-003`, `SVC-005`, `SVC-006` — the format authority carries the new table names, the State Prompt, and the new diagnostic row. |
| `.arca/runbook-authoring.md` | updated | `SVC-003`, `SVC-005` — the authoring loop writes `states` and repairs the new code. |
| `.arca/steering.md` | updated | `SVC-009` — the authored identity names States, and the sprint record is regenerated at P1 close. |
| `.arca/tpl/state.toml` | updated | `SVC-004`, `SVC-009` — the blank Run Record form moves with the file and its field. |
