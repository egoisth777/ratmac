# Issue test plan

## Verification

`CGDV-001` and `CGDV-002` remain the landed workflow-check proofs from
`CGD-001` and `CGD-002`. The Engine-cutover checks begin at `CGDV-003`.

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `CGDV-003` | `CGD-003` | A fixture supplies the three field selections through runbook data. Renaming those fields and moving the same list values, while renaming or removing every Markdown heading, derives the same ordered focused, hidden, and quality set and produces the same gate verdict. |
| `CGDV-004` | `CGD-003` | The completion path contains no `## Merge Gate` split, `HT-` shape match, or backtick scan. Rust names the three generic receipt kinds but none of this workflow's fields; focused, hidden, and quality declarations reach the gate without a prose-parser fallback. |
| `CGDV-005` | `CGD-003` | A non-list value, empty-string entry, and duplicate each refuse naming the field and entry with no mutation. An explicitly empty list for one kind is legal; absent selected fields and an entirely empty combined set both retain the existing `declares no checks` refusal rather than passing. |
| `CGDV-006` | `CGD-003` | Historical contract-gate and receipt fixtures pass without archive edits. Receipt kind and format, Run-keyed path, target binding, green/self-consistency/freshness/declared checks, missing/stray/duplicate safeguards, sensitivity behavior, and every receipt-defect refusal remain byte-for-byte compatible. |
| `CGDV-007` | `CGD-003` | A non-Markdown declaration carrier with no heading or backtick syntax gates successfully when its focused, hidden, and quality receipts are complete and green; deleting a required receipt still produces the existing refusal. |

## Integration traces

| Trace | Where it lands |
| :--- | :--- |
| The declared-check contract | [`CGD-003`](../../goal/spec.md#integrated-completion-from-declared-data-requirements) in the goal specification |
| Current workflow field names | `CGD-001` in `.arca/schema.md`; the generic field-selection and migration mechanism waits for the upcoming goal design decision |
| The deleted prose parsers | `src/completion.rs`, cut in the owning ticket |
