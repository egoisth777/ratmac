---
ticket-id: "{{ticket-id}}"
residual-ids:
  - "{{residual-id}}"
# SDC-003 (schema.md, "Deliberate damage and discard safety"): deliberate-damage
# evidence lives solely in these gap records' mutation-kill lists; the ticket
# points and duplicates no evidence bytes.
behavior-refs:
  - "{{path/to/spec.md#anchor}}"
design-refs:
  - "{{path/to/design.md#anchor}}"
planned-test-refs:
  - "{{planned-test-id}}"
dependencies:
  - "{{ticket-id}}"
status: "{{status}}"
# PGE-006/NRR-001: the human-readable mark you write when an authorized `rtm hold`
# pauses the Run, together with status "held". The Engine never reads it back.
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
