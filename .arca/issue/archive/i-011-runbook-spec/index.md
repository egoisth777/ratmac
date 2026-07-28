# Runbook spec — write down what a runbook IS

```yaml
issue-id: "i-011-runbook-spec"
provenance: "User request, 2026-07-27 - steering.md Current sprint, route item 0; cut at loop entry (task ledger #4)"
status: "integrated"
```

## Summary

Write down what a runbook IS — file format, machine-class schema, guard-kind vocabulary, ownership rules — as
prose, before more code embodies it. Today the definition lives implicitly in the parser and scheduler; every
other route item (parser i-012, doctor i-013, authoring loop i-014) implements this spec, so it must exist
first and be the single authority they cite.

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
| [Goal specification](../../../goal/spec.md) | Requirement records `RBS-001`–`RBS-005` |
| [Goal design](../../../goal/design.md) | Home of the specification (`.arca/runbook-spec.md`, shop lane, outside the frozen bundle) and single-authority rule |
| [Goal test list](../../../goal/test-list.md) | Checks `RBSV-001`–`RBSV-005` |
| [Goal ubiquitous language](../../../goal/ubi-lang.md) | Runbook specification and Guard-kind vocabulary |
| [Goal index](../../../goal/index.md) | Reverse link to this issue |
