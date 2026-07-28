# Deep rtm doctor

```yaml
issue-id: "i-013-deep-rtm-doctor"
provenance: "User request, 2026-07-27 - steering.md Current sprint, route item 2; cut at loop entry (task ledger #2)"
status: "pending"
```

## Summary

Deepen `rtm doctor` from a surface check into a real validator: run the actual parser (i-012) instead of a
bare `toml::Value` walk, add graph checks and guard lint, wire in the existing ownership audit (PGE-004),
accept arbitrary file paths, and emit machine-readable diagnostics with stable codes and differentiated exit
codes — the foundation the authoring loop (i-014) repairs against.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |
