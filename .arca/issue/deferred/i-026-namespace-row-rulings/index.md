# Two engine-namespace rows contradict the rules that mechanize them

```yaml
issue-id: "i-026-namespace-row-rulings"
provenance: "The road back from the still-open roots-table build turn: the ticket about the runbook roots table (`t-076`) and both of its gap records state that a planning-pass ruling is required before their remaining clauses can be met (.arca/ticket/t-076.md, .arca/residual/res-106.md, .arca/residual/res-113.md). Billy also filed the same two observations as wishes on 2026-08-06."
status: "deferred"
```

## Summary

Two goal rows from the engine-namespace split cannot be satisfied as written,
because each one contradicts another rule that is already in force. Neither is a
coding defect; each needs one ruling that says which side moves.

1. **Where the held fact lives.** The goal says the Engine writes no file under
   the working folder, while the working rules and the blocked-route requirement
   both say that an authorized hold marks the ticket file held. Today the Engine
   does write that file, and it also re-reads it to learn that a Run is held, so
   a contributor's file is both a write target and the Engine's index.
2. **Who may name a retired folder in Engine source.** The goal's check asks that
   the Engine source contain no literal naming the pre-split folder, while
   another row requires every entry point to refuse a project that still carries
   that folder — which cannot be detected without naming it once. One named
   exception survives in the source today, pinned by its own check.

This issue asks for the two rulings and nothing else. It proposes no code.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |
