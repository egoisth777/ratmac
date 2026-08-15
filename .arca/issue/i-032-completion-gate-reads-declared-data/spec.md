# Issue specification

Dispositions below record the author's proposed decision; P1 confirms or
revises them at integration.

`CGD` is this issue's stable requirement-ID prefix - **Completion from
Declared Data** - defined in [ubi-lang.md](ubi-lang.md).

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `CGD-001` | The checks the completion gate demands for an addressed item are declared data read through one typed reader: three explicit front-matter list fields on the item - focused tests, hidden lanes, quality commands - each taken verbatim as an opaque check id. The gate performs no heading split, no token-shape match, and no backtick scan; `declared_checks` takes the parsed declaration, never raw markdown. | pending | The gate must consume the same kind of input every other guard consumes - declared fields - not the shape of a contributor's prose. Verbatim-opaque ids keep the Engine free of any `HT-nnn-nn` or command grammar. | - |
| `CGD-002` | A malformed declaration is a hard, named refusal: a declared field that is present but not a list of non-empty strings, or a duplicate id across the three fields, names the field and the offending entry and passes nothing. An item declaring none of the three fields keeps today's honest refusal - it declares no checks, so completion would prove nothing. | pending | Silent tolerance is how the heading split failed: a renamed heading declared nothing without saying so. Unknown or malformed input is a refusal, never a guess (`R-011`'s discipline applied to the declaration). | - |
| `CGD-003` | Every guarantee `PGE-005` carries per receipt is unchanged: green, self-consistent, fresh, declared, no stray receipt beside the declared set, and the receipt's item field must match the addressed binding. Only the source of the declared-check list moves; the receipt format, the evidence path keyed by the Engine-minted Run id, and the refusal wording for receipt defects stay as they are. | pending | This issue relocates the gate's input; it must not weaken the gate. The per-receipt verification is the part that already works. | - |

## Acceptance criteria

- `src/completion.rs` contains no `## Merge Gate` split, no `HT-` shape
  match, and no backtick scan; `rg "Merge Gate|split|backtick"` over the gate
  finds only the typed declaration reader.
- A ticket whose front matter declares the three lists gates exactly as
  today's prose-parsed ticket did (the live tickets' declared sets are
  byte-equal before and after the cutover, proven by a fixture pair).
- A declaration that is present but malformed refuses naming the field and
  entry, and writes nothing.
- A work item that is not markdown but carries the declared fields passes the
  gate; the gate never requires the item to hold any heading.
- Every check `PGE-003` and `PGE-005` already carry per receipt still passes
  on this repository as it stands.
