# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| PCRV-001 | PCR-001 | `.arca/ratmac.toml` parses through `MachineClass::from_toml` and declares the P1-P5 Phases and edges; a Run started on it reaches the P4/P5 loop and the rest state by stepping alone, with no rule supplied outside the file. |
| PCRV-002 | PCR-002 | On this repository, `rtm status` names the stage the tree implies, proven by fixtures for each stage (pending issue, frozen goal with stale residuals, unproven residual without a ticket, executing ticket, all clean). |
| PCRV-003 | PCR-003 | A machine check distinguishes a landed ticket from an executing one on the real `.arca/ticket/` contents, with no prose input; seeding one unproven residual flips exactly one ticket to open. |
| PCRV-004 | PCR-004 | `ownership::audit_ownership` over the cycle runbook's prompts and guard contracts returns no violation, and the landing line is appended by an `rtm` command whose test proves the file is Scheduler-written. |
| PCRV-005 | PCR-005 | `rtm doctor` exits `0` on `.arca/ratmac.toml`; a test pins the exit code so a later gate cannot reintroduce an `RB302` shape unnoticed. |
| PCRV-006 | PCR-006 | With the roots declared in the runbook, the contract and freeze guards read them from the parsed class; a fixture project rooted elsewhere passes the same gates with no `.arca` literal in `src/`. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/spec.md` | proposed | This issue's accepted requirement records, each linking back here. |
| `.arca/goal/design.md` | proposed | Accepted mechanics for the cycle runbook, the append command, the open-ticket predicate, and the path extraction. |
| `.arca/goal/test-list.md` | proposed | This issue's verification checks. |
| `.arca/goal/ubi-lang.md` | proposed | Cycle runbook, Open ticket, Landing append. |
| `.arca/goal/index.md` | proposed | Reverse link to this issue. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `.arca/schema.md` | proposed | The ticket archive rule, if P1 picks that route for `PCR-003`; the landing-line instruction, if `PCR-004` routes the append through `rtm`. Working rules change only at integration, never from a ticket. |
| `.arca/index.md` | proposed | The stage-derivation lookup becomes the no-Run fallback once `rtm status` answers the question. |
| `.arca/runbook-spec.md` | unaffected | The cycle runbook is written against it; it defines nothing new here. |
| `AGENTS.md` | unaffected | — |
