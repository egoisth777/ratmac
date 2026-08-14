# Issue specification

Dispositions below record the author's proposed decision; P1 confirms or revises them at
integration.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `GPH-001` | Every contract gate is exercised by at least one fixture with a past: a fixture carrying the kind of history that gate walks (archived records citing an older freeze for the record contract, archived issue bundles and tickets for the intake contract, receipts from an earlier run for the completion and sensitivity gates), and the gate's expected verdict on that fixture is stated by the check, pass or refuse. | accepted | The record contract was unpassable on any repository with history and green for months, because every fixture was fresh. Measured cause, not speculation: i-029's summary names it, and no other gate has a fixture that could catch the same class. | |
| `GPH-002` | The working rules carry the fixture-with-a-past requirement forward: a ticket that adds or amends a contract gate lists, in its Merge Gate, the fixture-with-a-past check that exercises it, the same way hidden-lane coverage is already listed. A gate landed without one is a review refusal, not a style note. | accepted | A one-time fixture sweep decays as gates are added; the blind spot returns with the next gate unless the rule binds at the same place the other per-ticket guarantees bind. | |
| `GPH-003` | This repository's own history is the regression floor: at least one check per contract gate runs the gate against this repository as it stands, not only against built fixtures, and the expected verdict is recorded in the check. | accepted | The stalled Run was the only "check" that ever ran a gate on real history. This repository is the one fixture whose past is guaranteed to keep growing, and `EDNV-004` already proves the pattern works. | |

## Acceptance criteria

- For each contract gate, a named check builds or addresses a repository with a past and
  states the gate's expected verdict on it.
- A new gate cannot reach a green Merge Gate without such a check; the working rules say so
  where Merge Gate contents are defined.
- No gate's verdict on any existing fixture changes: this issue adds coverage, never behavior.
