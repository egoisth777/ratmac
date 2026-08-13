# Issue design

## Proposed mechanics

### 1. The probe (`EDN-002`)

One read-only version-control command, in the guard kind this repository's runbook already
uses for the safety-commit check:

```toml
{ kind = "command_exit", program = "git", args = [
  "describe", "--exact-match", "--match", "edition-*", "HEAD",
], expected = 0 }
```

Measured before the guard was proposed: with no edition tag the command exits `128`
("No names found, cannot describe anything"); at `edition-001` it exits `0` and prints
`edition-001`. Any non-zero refuses, so the guard needs no exit-code table of its own.

It belongs on the `close` stage's guard list, beside the record contract that already checks
the gap records. Ordering falls out naturally: the sprint's landings are committed, the person
cuts the edition, and the step into `rest` then passes. A sprint that forgets is stopped at the
last stage with a refusal that names the missing tag, which is cheaper than discovering months
later that no base was marked.

Tags are shared across linked worktrees - they live in the common git directory, not the
per-worktree one - so the guard reads the same answer from a ticket worktree as from `main`.

### 2. The bar (`EDN-001`) is written, not probed

The guard proves one fact: a tag named `edition-*` points at the commit being left. Everything
else in `EDN-001` - green suite, clean formatting, zero lints, resolving links, `rtm doctor`
exit `0`, clean tree, cycle at rest - is proven by the gates the cycle already runs before
`close` is reachable, and is recorded in the tag's own message.

**Stated limit, not papered over:** the probe cannot tell an annotated tag from a lightweight
one, and cannot read whether the message is honest. Two candidates were weighed and the
ruling belongs to whoever takes the ticket:

- **Accept the limit and name it** in the gap record: the tag is a receipt a human writes at a
  point where the machine has already refused every alternative, and the surrounding gates are
  the real evidence. Recommended - it adds nothing and hides nothing.
- **A second check that reads the tag object's type.** Provable, but it needs either a second
  guard whose program is a shell pipeline - which the guard vocabulary deliberately does not
  accept - or a contributor tool, which is a bigger change than the fact it buys.

### 3. Bootstrap

`edition-001` was cut at `18bc304` on 2026-08-10, before this bundle existed, because that
commit already met the whole bar: the i-015 sprint had just closed, every gap record read
`satisfied` and was archived, no ticket was open, and the gates were green. Cutting it first
also keeps this issue's own sprint honest - it starts from an edition, exactly as `EDN-002`
will require of every sprint after it.

This sprint therefore ends by cutting `edition-002`, and that step is the first live exercise
of the new guard.

### Rejected: a guard on the `intake` stage instead

Checking "the base was an edition" when leaving `intake` looks more direct - it guards the
entry rather than the exit. It is refused: by the time `intake` is left, the tree has already
moved off the base, so the guard would be asking a question about a commit that is no longer
`HEAD` and would have to be handed the base by interpolation the runbook does not have. The
`close` guard reaches the same end - every sprint bounded by editions - while asking only about
the commit in front of it.

### Rejected: teaching the engine about tags

A dedicated guard kind (`version_control_tag`, say) would read better in the runbook. It is
refused for this issue: it puts version-control knowledge inside a generic engine to save one
line of TOML in one project's runbook, and the closed guard vocabulary is deliberately hard to
grow.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted
forward authority.
