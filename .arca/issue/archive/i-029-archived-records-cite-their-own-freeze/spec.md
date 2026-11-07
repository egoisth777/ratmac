# Issue specification

Dispositions below record the author's proposed decision; P1 confirms or revises them at
integration. `ARF` expands to **Archived record freeze** and is this issue's stable
requirement-ID prefix, defined in [ubi-lang.md](ubi-lang.md).

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `ARF-001` | The record contract's frozen-revision check binds live records only. Archived records are still counted for presence and uniqueness - one record per requirement across active and archive together, no satisfaction by absence - but each cites the goal revision frozen when it was judged, and citing an older revision is not a defect. | accepted | The gate is unpassable as written for any repository whose goal has ever changed: measured on this repository, 127 archived records are reported and no live record is at fault. Editing them to today's revision is forbidden by the archive rule, so the only alternative to amending the requirement is never running the gate on real history. | |
| `ARF-002` | A stale claim is still caught. An archived record whose requirement is re-judged `missing` or `partial` moves back to the active folder, where the live check applies in full, and the gate still refuses a `satisfied` claim resting on a requirement no gate mechanizes. | accepted | The frozen-revision equality was doing real work - it stopped a record from claiming satisfaction against a goal that has since moved. That protection has to survive the amendment, and the existing visible-move rule already carries it. | |
| `ARF-003` | The checks that prove the record contract exercise a repository with history: at least one fixture carries an archived record citing an older revision, so this defect class cannot return under a green suite. | accepted | Every existing check builds a fixture with no past, which is exactly why a permanently unpassable gate looked green for months. | |

## Out of scope

- No change to which folder a record lives in, to the archive move itself, or to the
  one-record-per-requirement count.
- No relaxation of no-satisfaction-by-absence.
- The edition requirements are a separate sprint and only need this issue to run the gate
  on this repository at all.
