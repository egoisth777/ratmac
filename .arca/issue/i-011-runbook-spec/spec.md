# Issue specification

Dispositions below record the author's proposed decision; P1 confirms or revises them at integration.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `RBS-001` | A prose spec defines what a runbook IS: file format, machine-class shape (states, transitions, phases), required and optional fields. | accepted | The definition currently lives only in code; code cannot be the spec for the code. | `.arca/goal/spec.md` (proposed, P1) |
| `RBS-002` | The guard-kind vocabulary is enumerated in the spec: every kind, its semantics, and its per-kind required/forbidden fields. | accepted | Doctor lint (i-013) and parser validation (i-012) both need one list to check against. | `.arca/goal/spec.md` (proposed, P1) |
| `RBS-003` | Ownership rules are stated in the spec: which files/sections are machine-owned vs agent-writable, and what follows from writing where you may not. | accepted | ownership::audit_ownership (PGE-004) exists in code with no prose authority behind it. | `.arca/goal/spec.md` (proposed, P1) |
| `RBS-004` | The spec is the single authority: parser (i-012), doctor (i-013), and authoring loop (i-014) cite it and do not restate it. | accepted | Restated definitions drift; one authority, routed. | `.arca/goal/spec.md` (proposed, P1) |
| `RBS-005` | Existing semantics R-002/R-003/R-011 are preserved and restated in spec terms with back-references. | accepted | The spec formalizes what is; it must not silently change decided behavior. | `.arca/goal/spec.md` (proposed, P1) |
