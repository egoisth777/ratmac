# Issue specification

Dispositions below were confirmed at P1 on 2026-07-28; every ask was accepted and now lives in the goal specification.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `DRD-001` | Doctor runs the real parser (i-012's typed MachineClass), not a bare `toml::Value` walk. | accepted | A doctor with its own reader disagrees with the machine it diagnoses. | [Goal specification](../../../goal/spec.md#integrated-deep-doctor-requirements) |
| `DRD-002` | Graph checks: unique initial phase, reachability of every state, dead ends, duplicate edges. | accepted | A well-formed file can still describe a broken machine. | [Goal specification](../../../goal/spec.md#integrated-deep-doctor-requirements) |
| `DRD-003` | Guard lint: unknown kind, per-kind required/forbidden fields, unpinned non-exempt `command_exit`, warning on agent-writable guards. | accepted | The defect classes we have already met, mechanized. | [Goal specification](../../../goal/spec.md#integrated-deep-doctor-requirements) |
| `DRD-004` | The existing `ownership::audit_ownership` (PGE-004) is wired into doctor, not duplicated. | accepted | The enforcer exists; doctor is its natural caller. | [Goal specification](../../../goal/spec.md#integrated-deep-doctor-requirements) |
| `DRD-005` | `rtm doctor <path>` validates an arbitrary runbook file, not only the repo's own. | accepted | The authoring loop drafts outside the live location. | [Goal specification](../../../goal/spec.md#integrated-deep-doctor-requirements) |
| `DRD-006` | Machine-readable diagnostics: stable codes, `--json` output. | accepted | The repair loop (i-014) must parse findings, not scrape prose. | [Goal specification](../../../goal/spec.md#integrated-deep-doctor-requirements) |
| `DRD-007` | Differentiated exit codes: clean, warnings-only, and errors are distinguishable to a caller. | accepted | Scripts branch on exit codes, not on text. | [Goal specification](../../../goal/spec.md#integrated-deep-doctor-requirements) |
