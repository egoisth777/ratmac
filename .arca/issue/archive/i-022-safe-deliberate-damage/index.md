# Safe deliberate-damage checks

```yaml
issue-id: "i-022-safe-deliberate-damage"
provenance: "Billy, 2026-08-03 - human promotion of the safe deliberate-damage wish, captured in .arca/wishlist.md after the composition-format build turn (t-064) lost an unsaved completed fix to a discard command run to undo deliberate test damage"
ideal-shape-property: "One writer, append-only"
status: "integrated"
```

## Summary

A deliberate-damage check breaks the code on purpose, briefly, to prove a named test kills it. In the
composition-format build turn (`t-064`, archived at
[.arca/ticket/archive/t-064.md](../../../ticket/archive/t-064.md)), undoing such a break with
`git checkout -- src/machine.rs` while the completed green implementation of that file was still
unsaved destroyed the only copy - the file was never staged, so git held no blob - and the work had
to be reconstructed from its tests.

This issue makes that loss impossible by rule, without banning version-control restoration: look
before any discard; save or park unsaved completed work first; run damage only from an ephemeral
safety commit and restore from that checkpoint; write the kills into the owning gap record only
after the observed failure; and fold the checkpoint into the one green landing, so permanent ticket
history stays exactly red-then-green. It binds the manual cycle run in this repository today;
machine enforcement stays owned by the deferred cycle-as-runbook issue
([i-015](../../deferred/i-015-cycle-as-runbook/index.md)).

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## P1 disposition

- 2026-08-03: `SDC-001`-`SDC-004` accepted into the working authority - the new
  "[Deliberate damage and discard safety](../../../schema.md#deliberate-damage-and-discard-safety)"
  section of `.arca/schema.md`, with the P1/P5/Units/Evidence-receipts alignment, four glossary
  entries in `.arca/dict.md`, and the residual and ticket blanks mechanizing the single evidence
  home (`mutation-kill` in the gap record; the ticket only points). No goal row, gap record, or
  ticket was minted: this pass added the working-authority branch to planning step 1, and these
  asks resolve to requirement-ID headings there. `SDC-005` is a `duplicate`: the deferred
  cycle-as-runbook issue owns future automation, and its first ask (`PCR-001`) was extended to
  name the dirty-tree refusal and the intake gate's working-authority references. The provenance
  correction landed in the same pass - the recorded loss is the `t-064` turn, and the new rules
  explicitly supersede that ticket's frozen backup-copy lesson while its archived bytes stay
  untouched. **One writer, append-only** is the carrying Ideal-shape property: one evidence home
  with one writer, no claim predating its check, no history silently destroyed.
