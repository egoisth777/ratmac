# Issue specification

Dispositions below record the author's proposed decision; P1 confirms or revises them at
integration. `NRR` expands to **Namespace row rulings** and is this issue's stable
requirement-ID prefix, defined in [ubi-lang.md](ubi-lang.md).

Both asks were forks a human owns. Billy ruled both on 2026-08-10 and P1 records the rulings
below; the wording of each ask is unchanged from the filing.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `NRR-001` | One ruling settles where the held fact lives. Either the hold's whole record moves into Engine-owned state, so the Engine neither writes nor reads a contributor's ticket file to learn that a Run is held and the working rules stop mechanizing the ticket-file edit, or the goal row admits a single named exception for that one annotation and says who owns it. | accepted | Ruled by Billy on 2026-08-10, and ruled wider than the fork as filed: "rtm is a generic state machine runner, in some runbooks there are no even concepts of `ticket`". The held fact moves into Engine-owned state, and the Engine loses the work-item concept - no Engine rule, argument, message, refusal, or path may presume a work-item document, its fields, or its shape. Recording that work is paused is Run state; annotating a contributor's file is a shop action. | [goal NRR-001](../../../goal/spec.md#integrated-namespace-row-ruling-requirements) |
| `NRR-002` | One ruling settles who may name a retired folder in Engine source. Either the goal's no-literal check admits exactly one named exception, names its owner, and says how the check states that exception, or the residue detection moves out of Engine source into runbook data so the check can hold literally. | accepted | Ruled by Billy on 2026-08-10, taking the author's recommendation: admit the one named exception. A project still carrying the retired folder cannot declare that folder in its own runbook - the folder *is* the residue - so moving detection into runbook data asks the broken project to describe its own breakage. | [goal NRR-002](../../../goal/spec.md#integrated-namespace-row-ruling-requirements) |

## Out of scope

No code, no test, and no folder layout changes here. Renaming the machine position is a
separate issue and does not settle either fork. Which ticket carries the accepted ruling is
decided at planning, not here.
