# Issue test plan

## Verification

Renumbered at the 2026-08-10 integration to match the accepted checks in the goal test list
(`.arca/goal/test-list.md`, Integrated Plan-Build Runbook verification) one for one. The row
for `PCR-004` is gone because that ask was rejected, and the row for `PCR-006` is gone because
that ask was dropped at review on 2026-07-28; three rows are new, for the asks the integration
accepted after splitting `PCR-001`'s extension.

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| PCRV-001 | PCR-001 | `.ratmac/ratmac.toml` parses through `MachineClass::from_toml` and declares the cycle's stages and edges; a Run started on it reaches the ticket turns and then the rest State by stepping alone, with no rule supplied outside the file. |
| PCRV-002 | PCR-002 | The machine has exactly one initial State and exactly one terminal rest State; while a sprint Run is live the addressed report names its stage, and the tree-derived lookup appears only as the labelled no-live-Run fallback. |
| PCRV-003 | PCR-003 | A machine check distinguishes a landed work item from an open one on the real ticket root's contents, with no prose input; seeding one unproven gap record flips exactly one item to open. |
| PCRV-004 | PCR-005 | `rtm doctor` exits `0` on `.ratmac/ratmac.toml`; a test pins the exit code so a later gate cannot reintroduce an `RB302` shape unnoticed. `ownership::audit_ownership` over the runbook's prompts and guard contracts returns no violation. |
| PCRV-005 | PCR-007 | Two child turns spawned with different bound addresses are each graded against their own receipts; the other turn's receipts satisfy neither, the runbook file carries no identifier, and declaring both address forms or naming an unsupplied binding refuses under its own code without writing. |
| PCRV-006 | PCR-008 | The intake gate passes an accepted ask resolving only to a working-authority requirement heading, passes one resolving only to a goal row, and refuses one resolving to neither, naming the ask and both places it looked. |
| PCRV-007 | PCR-009 | Stepping into the deliberate-damage stage refuses while a tracked file carries an uncommitted change, names observed against expected, and leaves State and Status untouched; committing the change makes the same step succeed. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/spec.md` | proposed | This issue's accepted requirement records, each linking back here. |
| `.arca/goal/design.md` | proposed | Accepted mechanics for the Plan-Build Runbook, the bound address, the open-work-item predicate, and the dirty-tree refusal, recorded as `ADR-0015`. |
| `.arca/goal/test-list.md` | proposed | This issue's verification checks. |
| `.arca/goal/ubi-lang.md` | proposed | Plan-Build Runbook, Cycle stage, Open work item, Bound address, No-live-Run fallback. |
| `.arca/goal/index.md` | proposed | Reverse link to this issue. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `.arca/schema.md` | proposed | The ticket archive rule is already in force and settles `PCR-003`'s route; no landing-line instruction is added, because `PCR-004` was rejected and the landing line stays a human act. Working rules change only at integration, never from a ticket. |
| `.arca/index.md` | proposed | The stage-derivation lookup becomes the no-Run fallback once `rtm status` answers the question. |
| `.arca/runbook-spec.md` | unaffected | The Plan-Build Runbook is written against it; it defines nothing new here. |
| `AGENTS.md` | unaffected | — |
