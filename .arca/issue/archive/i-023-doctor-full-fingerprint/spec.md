# Issue specification


## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `DFP-001` | Argument-free `rtm doctor` reports the complete 64-character lowercase hexadecimal SHA-256 of the exact executable being run and remains write-free. Executable selection, pin/trust behavior, state reporting, Runbook findings, and `--json` behavior remain unchanged. | accepted | A truncated identity cannot unambiguously identify the executable whose behavior is being diagnosed. | [goal DFP-001](../../../goal/spec.md#integrated-full-doctor-executable-fingerprint-requirement) |
