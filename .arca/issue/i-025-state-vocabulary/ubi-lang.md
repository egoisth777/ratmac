# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| `SVC` | **State vocabulary cutover** — this issue's stable requirement-ID prefix. |
| State | Where a Run currently sits in its machine graph. The thing a runbook declares, a transition moves between, and a guard is attached to. Replaces the word `Phase` in every live surface. |
| Run Record | The single file the Engine writes for one Run, holding that Run's `state`, its `status`, the revisions in play, and its blocker. Replaces the term `State File`. |
| Run | One live instance of a machine: its Run Record, its evidence, its lock, and its spawn ledger together. Unchanged in meaning; named here so the three ideas stay apart. |
| `status` | The Run's lifecycle value — `planned`, `executing`, `blocked`, `passed`, or `failed`. Unchanged in meaning, values, and ownership; it is never a position in the graph. |
| State Prompt | The prose a State declares plus the generated list of its Exit Guards and, for a branching State, its legal input values. Replaces the term `Phase Prompt`. |
| Pre-cutover residue | A runbook that still declares `phases`, or a Run Record that still carries the old field or the old filename. It is refused with instructions, never migrated in place. |
