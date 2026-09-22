# Run nested programs safely

```yaml
issue-id: "i-060-safe-nested-invocations"
provenance: "Wishlist: A runbook node that runs a skill itself powered by a ratmac runbook composes safely: each level owns its own Engine root, the parent consumes only the child Run's terminal receipt as evidence, and runaway recursion (a skill chain that reaches itself again) refuses by name instead of looping; Billy authorized all issues and wishes, then confirmed Workflow dispatch on 2026-09-22 with 'Yes, dispatch'."
status: "integrated"
```

## Summary

A ratmac-powered program may invoke another independently rooted ratmac-powered program safely. Each owns its state, the parent accepts only a verified child completion receipt, and recursion refuses before another process or Run is admitted.

Source: [wishlist](../../wishlist.md), exact entry **A runbook node that runs a skill itself powered by a ratmac runbook composes safely: each level owns its own Engine root, the parent consumes only the child Run's terminal receipt as evidence, and runaway recursion (a skill chain that reaches itself again) refuses by name instead of looping**. The user's 2026-09-22 dispatch promotes this work for the planning pass. This bundle proposes a ruling; it does not claim integration or implementation.

Ideal-shape properties advanced: **Generic engine, Every boundary machine-checked, and One writer, append-only** in [steering](../../steering.md#ideal-shape).

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |


## Planning decision

Accepted on 2026-09-22 under Billy's all-wishes request and explicit workflow dispatch.
Advances the steering's Generic engine, Every boundary machine-checked, or
One writer, append-only properties through its stated outcome. Accepted assumptions
are recorded in the forward authority. Integration makes no implementation claim.
