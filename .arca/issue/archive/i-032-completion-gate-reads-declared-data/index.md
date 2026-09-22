# The completion gate reads declared data, not prose shape

```yaml
issue-id: "i-032-completion-gate-reads-declared-data"
provenance: "Wishlist, `The completion gate should not know what a ticket is either` - Billy's 2026-08-10 ruling applied where it was not yet carried; remainder identified after t-089 landed PCR-007"
status: "integrated"
```

## Summary

`NRR-001` removed the Engine's work-item concept from the hold, and t-089
(`PCR-007`) removed the literal ticket id from every runbook: a receipt-class
guard now addresses its item through a binding the Run carries, and the
evidence path is keyed by what the Engine mints. What remains is the last
place the Engine still *understands a contributor's document*:
`src/completion.rs` decides what completion must prove by parsing prose
shape - it splits the item's markdown on the `## Merge Gate` heading
(`src/completion.rs:150`), harvests hidden-lane ids by the `HT-nnn-nn` token
shape from anywhere in the file (`src/completion.rs:134`), and treats
backticked fragments with a space as commands.

A gate whose input is the *shape* of an agent-written document is fragile in
both directions: a renamed heading silently declares nothing (and only the
"declares no checks" refusal catches it), and a stray backticked token in
prose becomes a check the worker must now evidence. A generic runner whose
work items are not markdown files cannot use the gate at all. The fix is the
same move the rest of the Engine already made: the checks a Run must prove
come from explicit lists named by the runbook, and the prose parsers are
deleted.

## Selection

Billy delegated issue selection and the next-sprint start. The remaining Engine
cutover (`CGD-003`) was selected because the [tag-reader ticket
(`t-105`)](../../../ticket/archive/t-105.md) and its satisfied gap records
([`res-152`](../../../residual/archive/res-152.md) and
[`res-153`](../../../residual/archive/res-153.md)) prove the tag format, while
the [turn-close ticket's implementation-run review
(`t-111`)](../../../ticket/archive/t-111.md#implementation-run-review) records
the duplicate work of keeping the same checks in tags and prose.

At run-030 intake on 2026-09-18, Billy explicitly approved the remaining
Engine cutover. `CGD-003` is accepted and integrated into the
[forward product goal](../../../goal/spec.md#integrated-completion-from-declared-data-requirements):
declared checks replace prose discovery while the existing receipt contract
stays unchanged. The two sibling asks (`CGD-001` and `CGD-002`) remain accepted
working-authority prerequisites. This planning integration makes no
implementation claim; parser removal, generic field selection, and preserved
receipt behavior remain to be proved.

## History

- 2026-08-13: filed from the wishlist by the DESIGNER; dispositions are the
  author's proposal, P1 confirms or revises at integration.
- 2026-08-21: P1 accepted `CGD-001` and `CGD-002` into the working authority
  and deferred the Engine cutover in `CGD-003`.
- 2026-09-18: after the declared-tag workflow landed and was used on real
  turns, the complete deferred bundle returned to intake for run-030.
- 2026-09-18: Billy explicitly approved `CGD-003`; P1 accepted it into the
  product goal without changing the two earlier accepted dispositions.
