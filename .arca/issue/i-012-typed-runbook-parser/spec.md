# Issue specification

Dispositions below record the author's proposed decision; P1 confirms or revises them at integration.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `TRP-001` | One typed parse: a serde-typed MachineClass is the only reader of runbook toml; the second parse in `scheduler.rs` is removed and the scheduler consumes the typed value. | accepted | Two readers of one file drift by construction. | `.arca/goal/spec.md` (proposed, P1) |
| `TRP-002` | Guard kinds are a typed enum (GuardKind); an unknown kind is a typed parse error, never a skipped guard. | accepted | A skipped guard is a silently weaker machine. | `.arca/goal/spec.md` (proposed, P1) |
| `TRP-003` | Per-kind field validation at parse time: required and forbidden fields per guard kind, per the runbook spec (RBS-002). | accepted | Catch shape errors where they enter, not at run time. | `.arca/goal/spec.md` (proposed, P1) |
| `TRP-004` | Guards are retained through the parse: every guard written in the runbook is present on the typed MachineClass. | accepted | Dropping guards is the failure mode this route exists to end. | `.arca/goal/spec.md` (proposed, P1) |
| `TRP-005` | Missing or unreadable runbook is a named refusal surfaced to the caller; never a silent `MachineGraph::default()`. | accepted | An empty machine that runs is worse than a refusal that explains. | `.arca/goal/spec.md` (proposed, P1) |
| `TRP-006` | R-002/R-003/R-011 semantics are preserved; existing default and opt-in lanes stay green. | accepted | Refactor, not behavior change — decided behavior moves only via issues. | `.arca/goal/spec.md` (proposed, P1) |
