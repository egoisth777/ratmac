# The completion gate reads declared data, not prose shape

```yaml
issue-id: "i-032-completion-gate-reads-declared-data"
provenance: "Wishlist, `The completion gate should not know what a ticket is either` - Billy's 2026-08-10 ruling applied where it was not yet carried; remainder identified after t-089 landed PCR-007"
status: "deferred"
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
become declared data behind the one typed reader, and the prose parsers are
deleted.

## History

- 2026-08-13: filed from the wishlist by the DESIGNER; dispositions are the
  author's proposal, P1 confirms or revises at integration.
