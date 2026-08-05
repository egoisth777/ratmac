# Issue specification

The disposition below is the proposed P1 decision; P1 confirms or revises it before this pending bundle
can be integrated.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `DFP-001` | Argument-free `rtm doctor` reports the complete 64-character lowercase hexadecimal SHA-256 of the exact executable being run and remains write-free. Executable selection, pin/trust behavior, state reporting, Runbook findings, and `--json` behavior remain unchanged. | accepted | A truncated identity cannot unambiguously identify the executable whose behavior is being diagnosed. | Proposed P1 destination: a new `DFP-001` record in `.arca/goal/spec.md`, constrained by existing `ORS-002` and `DRD-005`. |
