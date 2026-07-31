# State File path correction

```yaml
issue-id: "i-021-state-file-path-correction"
provenance: "Manual plan-build observation, 2026-07-30 — the integrated residency requirement moved each State File under its addressed Run, but R-024 and R-025 still claim that State File is flat under .arca"
ideal-shape-property: "One writer, append-only"
status: "integrated"
```

## Summary

The frozen goal contradicts itself. `FDC-004` makes `.arca/runs/<id>/state.toml` canonical, while inherited `R-024` and `R-025` still require `.arca/state.toml`.

This issue authorizes a goal correction only. It adds no capability and mints no replacement requirement: `FDC-004` already owns the path.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirement disposition | [Specification](spec.md) |
| Proposed correction | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## P1 disposition

- 2026-07-30: `SPC-001` was integrated as a duplicate correction to `FDC-004`, not as a new requirement. The goal marks the inherited flat State File clauses superseded and retains one canonical per-Run path contract. **One writer, append-only** is the carrying Ideal-shape property.
