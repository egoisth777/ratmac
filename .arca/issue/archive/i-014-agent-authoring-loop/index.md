# Agent authoring loop

```yaml
issue-id: "i-014-agent-authoring-loop"
provenance: "User request, 2026-07-27 - steering.md Current sprint, route item 3; cut at loop entry (task ledger #3)"
status: "integrated"
```

## Summary

Make runbooks writable by agents on purpose instead of by imitation: an agent-facing schema instructions doc
(so runbooks are written against the spec, not guessed from examples), scaffold/template output as a valid
starting point, and a write → doctor → repair loop driven by the doctor's machine-readable diagnostics.
Depends on the parser (i-012) and doctor (i-013); cites the runbook spec (i-011), never restates it.

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
| [Goal specification](../../../goal/spec.md) | Requirement records `AAL-001`–`AAL-004` |
| [Goal design](../../../goal/design.md) | `rtm scaffold <path>` and `.arca/runbook-authoring.md` keyed to the diagnostic codes |
| [Goal test list](../../../goal/test-list.md) | Checks `AALV-001`–`AALV-004` |
| [Goal ubiquitous language](../../../goal/ubi-lang.md) | Scaffold and Authoring loop |
| [Goal index](../../../goal/index.md) | Reverse link to this issue |
