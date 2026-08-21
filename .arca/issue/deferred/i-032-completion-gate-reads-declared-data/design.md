# Issue design

## Proposed mechanics

This file is incoming evidence: integrated mechanics remain authoritative
only in the accepted forward authority.

**1. The declaration is three front-matter fields, read by the reader that
already exists.** Tickets already carry structured YAML front matter that the
Engine reads through one typed reader (`planned-test-refs`, `residual-ids`,
`dependencies`). The declaration extends that surface, replacing the two
prose parsers and the shape match:

- `planned-test-refs` - already declared data; stays, and becomes the *only*
  source of focused checks (today `crate::receipt::planned_tests` is already
  field-based).
- `hidden-lane-ids` - new list field; replaces the `HT-nnn-nn` token-shape
  harvest of `hidden_lane_ids()` (`src/completion.rs:134`). Ids are opaque
  strings; the Engine carries no lane-id grammar.
- `merge-gate-commands` - new list field; replaces the `## Merge Gate`
  heading split and backtick scan of `merge_gate_commands()`
  (`src/completion.rs:150`). Commands are opaque strings the receipts must
  match verbatim.

`declared_checks` takes the three parsed lists, not raw markdown. Order and
dedup rules are unchanged: focused, then hidden, then quality; a duplicate id
across fields is a refusal (CGD-002), not a silent skip as today.

**2. The human-readable sections stay; the schema stops being load-bearing.**
The `## Merge Gate` section and the hidden-coverage manifest remain in the
working rules as prose for reviewers. The cutover means a mismatch between
prose and declaration is caught the only way it ever honestly was: the
declared set is what the gate proves, and review reads the prose. The working
rules (`.arca/schema.md` ticket template) gain the two fields; live tickets
are all archived, so no live ticket needs rewriting - the fixture pair in the
test plan proves equivalence on a representative ticket instead.

**3. What is deleted.** `hidden_lane_ids()`, `merge_gate_commands()`,
`backticked()` (if unused after), and the raw-markdown signature of
`declared_checks`. `gate_completion_at` reads the item once through the typed
front-matter reader and hands the parsed declaration down.

**4. What is deliberately out of scope.**

- The receipt format, evidence paths, freshness digests, and stray-receipt
  refusals (CGD-003 pins them unchanged).
- The sensitivity gate (`PGE-003`): `planned_tests` is already declared data.
- Non-markdown work items beyond proving the gate does not require headings:
  a full non-file work-item concept is a later issue.
- Any repair of archived tickets: frozen provenance, never edited.

## Open decision for P1

Whether a declaration field present-but-empty (`hidden-lane-ids: []`) means
"this item declares no hidden lanes" (legal, lane count zero) or is refused.
Proposed: legal - an explicit empty list is a declaration, absence of all
three fields is the only "declares nothing" refusal. P1 confirms.
