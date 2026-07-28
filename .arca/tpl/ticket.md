---
ticket-id: "{{ticket-id}}"
residual-ids:
  - "{{residual-id}}"
behavior-refs:
  - "{{path/to/spec.md#anchor}}"
design-refs:
  - "{{path/to/design.md#anchor}}"
planned-test-refs:
  - "{{planned-test-id}}"
dependencies:
  - "{{ticket-id}}"
status: "{{status}}"
# PGE-006: set only by an authorized `rtm hold`, together with status "held".
# Units and git (schema.md): the short hash of the commit that made the ticket
# green. Filled in at closure; a ticket is passed by evidence, never by status.
landed-commit: "{{short-hash}}"
blocker-ref: "{{blocker-issue-folder-or-residual}}"
---

# Ticket: {{ticket-id}}

## P4 Apparent Test Plan

Public mapping from explicit goal behavior to an executable test and oracle.

| Apparent Test ID | Goal Contract Ref | Fixture Ref | Executable Target | Oracle |
| ---------------- | ----------------- | ----------- | ----------------- | ------ |
| `{{planned-test-id}}` | `{{path/to/contract.md#anchor}}` | `{{fixture-ref}}` | `{{executable-target}}` | `{{observable-oracle}}` |

## P5 Hidden Test Public Coverage Manifest

Public coverage manifest entries. Category must map to one of the six adversarial lanes:
1. `Regression`
2. `Input/Routing`
3. `Lifecycle/Model`
4. `Durability/Recovery`
5. `Output/Filesystem`
6. `Cross-Feature`

Assess every lane before generating private bodies. `not-applicable` requires
a public rationale and must not be used to hide missing contract coverage.

| Lane | Assessment | Rationale | Hidden IDs |
| ---- | ---------- | --------- | ---------- |
| `{{category}}` | `{{covered\|not-applicable}}` | `{{assessment-rationale}}` | `{{hidden-ids-or-none}}` |

| Hidden ID | Goal Contract Ref | Derivation | Category | Oracle | Owner |
| --------- | ----------------- | ---------- | -------- | ------ | ----- |
| `{{hidden-id}}` | `{{path/to/contract.md#anchor}}` | `{{why-the-oracle-follows-from-the-contract}}` | `{{category}}` | `{{oracle}}` | `{{owner}}` |
