# Engine-namespace split

```yaml
issue-id: "i-024-engine-namespace-split"
provenance: "Human promotion, 2026-08-06, of the Engine-namespace split wish (Billy, 2026-08-03) recorded in .arca/wishlist.md, with the cross-worktree evidence gathered while scoping run recursion and parallel child Runs."
status: "integrated"
ideal-shape-property: "Self-hosted"
```

## Summary

The Engine has one `.ratmac/` root at the primary checkout root. It holds the Machine
Class, runs, the durable id mint record, locks, the Engine transition log, and receipts;
no file under `.arca/` is Engine-written or mechanically read except through a declared
root name. The workflow folders that contract guards read become runbook data instead of
Rust literals.

A linked worktree uses Git worktree metadata to resolve the primary checkout's `.ratmac/`
rather than creating its own; without Git, `.ratmac/` resolves at the current checkout
root. That gives every worktree one roster, one id namespace, and one lock domain,
eliminating the duplicate-ordinal and invisible-child-Run failures of checkout-local
runtime state.

Git ignores runtime files under `.ratmac/`, while the Machine Class and receipts at
`.ratmac/evidence/<run-id>/` stay tracked. Run state can therefore never enter a ticket
branch or a merge, and run-scoped receipt paths let parallel child Runs merge without
collision.

Nothing migrates itself: a pre-split layout makes every entry point refuse and print
instructions.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## P1 disposition

On 2026-08-06, `ENS-001`..`ENS-010` were accepted into the goal's integrated
engine-namespace requirements, its decision record, glossary, orientation, and
verification list. `ENS-011` and `ENS-012` were accepted into the working authority as
[schema requirements](../../../../schema.md#ens-011--current-engine-addresses), so they bind
at integration and mint no goal row, no gap record, and no ticket.

One ask was revised at integration: the Machine Class is read from the invoking
checkout's own `.ratmac/ratmac.toml`, while runtime resolves to the primary checkout's
`.ratmac/`. A worktree that edits its runbook therefore refuses on pin drift instead of
silently having no effect.

The complete bundle was archived. The single Engine root, resolved by every worktree,
advances **Self-hosted**.
