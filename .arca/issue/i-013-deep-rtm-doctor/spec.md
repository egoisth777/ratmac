# Issue specification

Dispositions below record the author's proposed decision; P1 confirms or revises them at integration.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `DRD-001` | Doctor runs the real parser (i-012's typed MachineClass), not a bare `toml::Value` walk. | accepted | A doctor with its own reader disagrees with the machine it diagnoses. | `.arca/goal/spec.md` (proposed, P1) |
| `DRD-002` | Graph checks: unique initial phase, reachability of every state, dead ends, duplicate edges. | accepted | A well-formed file can still describe a broken machine. | `.arca/goal/spec.md` (proposed, P1) |
| `DRD-003` | Guard lint: unknown kind, per-kind required/forbidden fields, unpinned non-exempt `command_exit`, warning on agent-writable guards. | accepted | The defect classes we have already met, mechanized. | `.arca/goal/spec.md` (proposed, P1) |
| `DRD-004` | The existing `ownership::audit_ownership` (PGE-004) is wired into doctor, not duplicated. | accepted | The enforcer exists; doctor is its natural caller. | `.arca/goal/spec.md` (proposed, P1) |
| `DRD-005` | `rtm doctor <path>` validates an arbitrary runbook file, not only the repo's own. | accepted | The authoring loop drafts outside the live location. | `.arca/goal/spec.md` (proposed, P1) |
| `DRD-006` | Machine-readable diagnostics: stable codes, `--json` output. | accepted | The repair loop (i-014) must parse findings, not scrape prose. | `.arca/goal/spec.md` (proposed, P1) |
| `DRD-007` | Differentiated exit codes: clean, warnings-only, and errors are distinguishable to a caller. | accepted | Scripts branch on exit codes, not on text. | `.arca/goal/spec.md` (proposed, P1) |
