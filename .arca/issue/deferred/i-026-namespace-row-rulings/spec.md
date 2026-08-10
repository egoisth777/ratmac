# Issue specification

Dispositions below record the author's proposed decision; P1 confirms or revises them at
integration. `NRR` expands to **Namespace row rulings** and is this issue's stable
requirement-ID prefix, defined in [ubi-lang.md](ubi-lang.md).

Both asks are forks a human owns, so both are proposed `deferred`. Each rationale names the
author's recommendation so the ruling costs one sentence to give.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `NRR-001` | One ruling settles where the held fact lives. Either the hold's whole record moves into Engine-owned state, so the Engine neither writes nor reads a contributor's ticket file to learn that a Run is held and the working rules stop mechanizing the ticket-file edit, or the goal row admits a single named exception for that one annotation and says who owns it. | deferred | Recommendation: move the fact into Engine-owned state. One owner per file is the structural rule the whole namespace split bought, and a contributor file that doubles as the Engine's index is the last two-writer file left. The cost is that the working rules and the blocked-route row both need rewording, which is exactly why it is a ruling and not a repair. | |
| `NRR-002` | One ruling settles who may name a retired folder in Engine source. Either the goal's no-literal check admits exactly one named exception, names its owner, and says how the check states that exception, or the residue detection moves out of Engine source into runbook data so the check can hold literally. | deferred | Recommendation: admit the one named exception. A project still carrying the retired folder cannot declare that folder in its own runbook — the folder *is* the residue — so moving detection into runbook data asks the broken project to describe its own breakage. The exception already exists, is declared once under its own name, and its check fails on a second literal or a rename. | |

## Out of scope

No code, no test, and no folder layout changes here. Renaming the machine position is a
separate issue and does not settle either fork. Which ticket carries the accepted ruling is
decided at planning, not here.
