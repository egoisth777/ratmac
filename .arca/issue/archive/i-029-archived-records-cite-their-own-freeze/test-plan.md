# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `ARFV-001` | `ARF-001`, `ARF-003` | A fixture repository holding an archived record that cites an older goal revision, and live records citing the current one, passes the record contract. Removing the archived record's citation entirely, or corrupting it, still refuses. |
| `ARFV-002` | `ARF-001` | A live record citing anything other than the Run's frozen revision still refuses, with the same wording as before. |
| `ARFV-003` | `ARF-002` | A requirement re-judged `missing` from an archived record refuses until that record is back in the active folder, and a `satisfied` claim resting on a requirement no gate mechanizes still refuses. |
| `ARFV-004` | `ARF-001`, `ARF-003` | The gate passes on this repository as it stands - the check that could not be run before this issue. |

## Integration traces

| Trace | Where it lands |
| :--- | :--- |
| The amended requirement | The record-contract row in the goal specification, and the working rules' summary of the two contract gates. |
| The code | The record gate's per-path predicate. |
| The fixture with a past | The contract-gate checks, which build every fixture at the current freeze today. |
