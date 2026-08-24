# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `LNRV-001` | `LNR-001` | The sweep over this repository reports one verdict per crate for `t-058`..`t-105` plus a total, and names a crate removed from the folder rather than silently skipping it. |
| `LNRV-002` | `LNR-001` | The first sweep run separates the 2026-08-21 crates from the 2026-08-10 findings: `t-102`..`t-105` read `pass` against today's Engine, and every other verdict is `red` with lane ids or `expired` with an edition - never a bare, unexplained failure. |
| `LNRV-003` | `LNR-002` | A crate whose lanes refuse with no marker reads `red`; writing a marker naming an edition flips its verdict to `expired` and moves it out of both the pass count and the red count; `t-078` and `t-079` read `expired` once marked, never `red`. |
| `LNRV-004` | `LNR-003` | A close whose report names a red, unexpired lane refuses; the same close passes once the lane is green or marked; the Machine Class diff that wires it adds a `command_exit`-class guard and nothing else. |
| `LNRV-005` | `LNR-001`, `LNR-002` | A tree snapshot around a full sweep is byte-identical apart from the report artifact; marking and un-marking change exactly the marker's own bytes. |

## Goal/Test File Traces

Statuses describe the tree at filing; the P1 pass that decides each ask
updates them.

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | unaffected | - |
| `.arca/goal/ubi-lang.md` | unaffected | - |
| `.arca/goal/spec.md` | unaffected | - |
| `.arca/goal/design.md` | unaffected | - |
| `.arca/goal/test-list.md` | unaffected | - |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | - |
| `.arca/schema.md` | unaffected | - |
