# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| TRPV-001 | TRP-002 | Fixture runbook with an unknown guard kind: parse returns a typed error naming the kind and location; no machine is built. |
| TRPV-002 | TRP-003 | Fixtures violating per-kind required/forbidden fields: each refused with an error naming kind + field. |
| TRPV-003 | TRP-001 | Exactly one toml read of the runbook in `src/` (grep); `scheduler.rs` contains no toml parsing and consumes the typed MachineClass. |
| TRPV-004 | TRP-004 | Guard-count fixture: every guard authored in the runbook is present on the parsed MachineClass, all kinds. |
| TRPV-005 | TRP-005 | Runbook path absent/unreadable: `rtm status`/`rtm run` surface a named refusal; no code path yields `MachineGraph::default()` for a missing file. |
| TRPV-006 | TRP-006 | Default + opt-in lanes green (baseline 156 passed / 0 failed / 1 ignored); R-002/R-003/R-011 tests unchanged and passing. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | Reverse link to `i-012-typed-runbook-parser` under "Integrated Machine Class as first-class data". |
| `.arca/goal/ubi-lang.md` | updated | This issue's terms folded in; the `Exit Guard` entry now routes to the runbook specification instead of enumerating kinds. |
| `.arca/goal/spec.md` | updated | This issue's accepted requirement records, each linking back here. |
| `.arca/goal/design.md` | updated | Accepted mechanics recorded under "Machine Class made first-class". |
| `.arca/goal/test-list.md` | updated | This issue's verification checks. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | — |
| `none` | unaffected | — |
