# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `NRRV-001` | `NRR-001` | The ruling is verifiable only once given: whichever branch is taken, the surviving rule set contains no sentence that contradicts another about who writes the held fact, and the accepting ticket carries the executable check for the chosen branch. Until the ruling, the evidence is the recorded contradiction in the roots-table ticket and its gap record. |
| `NRRV-002` | `NRR-002` | Likewise: after the ruling, the no-literal check and the residue-refusal row agree, and the check states its exception, if any, by name. Until the ruling, the evidence is the recorded contradiction in the same ticket and its gap record. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | unaffected | Both asks are deferred; no goal row moves yet. |
| `.arca/goal/ubi-lang.md` | unaffected | Both asks are deferred; no goal row moves yet. |
| `.arca/goal/spec.md` | unaffected | Both asks are deferred; no goal row moves yet. |
| `.arca/goal/design.md` | unaffected | Both asks are deferred; no goal row moves yet. |
| `.arca/goal/test-list.md` | unaffected | Both asks are deferred; no goal row moves yet. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | Names no held fact and no folder literal. |
| `.arca/schema.md` | unaffected | Its blocked-route wording is one branch of `NRR-001` and moves only if that branch is chosen. |
