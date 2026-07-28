# Single typed runbook parser

```yaml
issue-id: "i-012-typed-runbook-parser"
provenance: "User request, 2026-07-27 - steering.md Current sprint, route item 1; cut at loop entry (task ledger #1)"
status: "integrated"
```

## Summary

Unify runbook parsing into one serde-typed parse: a typed MachineClass with a GuardKind enum and per-kind
field validation, guards retained through the parse, and the second parse in `scheduler.rs` removed. A
missing or unreadable runbook becomes a named refusal instead of a silent `MachineGraph::default()`.
Implements the runbook spec (i-011); preserves R-002/R-003/R-011 semantics.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Integration

Folded into the goal on 2026-07-28 (P1). Every requirement disposition in [spec.md](spec.md) was confirmed `accepted`.

| Goal artifact | What it now carries |
| :--- | :--- |
| [Goal specification](../../../goal/spec.md) | Requirement records `TRP-001`–`TRP-006` |
| [Goal design](../../../goal/design.md) | One reader for the runbook, closed `GuardKind` enum, guards retained, missing runbook refused by name |
| [Goal test list](../../../goal/test-list.md) | Checks `TRPV-001`–`TRPV-006` |
| [Goal ubiquitous language](../../../goal/ubi-lang.md) | Typed parse and Named refusal |
| [Goal index](../../../goal/index.md) | Reverse link to this issue |
