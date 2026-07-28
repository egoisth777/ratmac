# Runbook spec — write down what a runbook IS

```yaml
issue-id: "i-011-runbook-spec"
provenance: "User request, 2026-07-27 - steering.md Current sprint, route item 0; cut at loop entry (task ledger #4)"
status: "pending"
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
