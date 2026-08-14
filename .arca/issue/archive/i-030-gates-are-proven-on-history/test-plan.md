# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `GPHV-001` | `GPH-001` | For each contract gate, a named check builds a fixture with a past - archived records at an older freeze, archived bundles, or prior-run receipts as that gate walks - and asserts the gate's stated verdict on it, pass or refuse. |
| `GPHV-002` | `GPH-001` | The aged-fixture builder produces a repository whose archive and active folder cite different freezes, and corrupting the aged half is still refused: age is never a free pass. |
| `GPHV-003` | `GPH-003` | Each contract gate runs against this repository as it stands with a recorded expected verdict, and the checks keep passing as history grows. |
| `GPHV-004` | `GPH-002` | The working rules' Merge Gate section names the fixture-with-a-past requirement, and the ticket that lands each gate check lists it. |

## Integration traces

| Trace | Where it lands |
| :--- | :--- |
| The per-gate coverage rule | The working rules, in the section that defines Merge Gate contents. |
| The aged-fixture builder | The QA harness library, beside the existing contract-gate fixture trees. |
| The per-gate checks | The contract-gate suites, one fixture-with-a-past case each. |
