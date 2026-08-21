# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `CGDV-001` | `CGD-001` | A fixture pair: one ticket carrying today's prose sections plus the declared fields, one carrying only the declared fields with the headings renamed or absent. Both gate identically, and the declared set equals what the prose parser produced before the cutover (equivalence proven at the cutover commit, then the prose comparison is retired with the parser). |
| `CGDV-002` | `CGD-001` | `src/completion.rs` source check: no `## Merge Gate` literal, no `HT-` shape match, no backtick scan survives; the source names the three fields and nothing prose-shaped. |
| `CGDV-003` | `CGD-002` | A declaration with a non-list field, an empty-string entry, and a duplicate id across fields each refuse naming the field and entry, writing nothing; an item declaring none of the three fields keeps the `declares no checks` refusal verbatim. |
| `CGDV-004` | `CGD-003` | The existing t048/t089 contract-gate and receipt suites pass unchanged on this repository as it stands; receipt-defect refusal wording is byte-identical before and after. |
| `CGDV-005` | `CGD-001` | A work item file with declared fields and no markdown heading anywhere passes the gate when its receipts are green and fresh. |

## Integration traces

| Trace | Where it lands |
| :--- | :--- |
| The declared-check contract | `PGE-005`'s row in the goal specification |
| The two new field names | Working-rules ticket template in `.arca/schema.md` |
| The deleted prose parsers | `src/completion.rs`, cut in the owning ticket |
