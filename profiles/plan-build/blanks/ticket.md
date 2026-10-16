---
ticket-id: "{{ticket-id}}"
residual-ids:
  - "{{residual-id}}"
# Deliberate-damage evidence lives solely in these gap records' mutation-kill
# lists (the bundle's schemas/residual.toml defines the shape); the ticket
# points and duplicates no evidence bytes.
behavior-refs:
  - "{{path/to/spec.md#anchor}}"
design-refs:
  - "{{path/to/design.md#anchor}}"
planned-test-refs:
  - "{{planned-test-id}}"
# The ticket's declared checks as three tag lists of opaque check ids; a
# checker reads these, never prose (the bundle's schemas/ticket.toml defines
# the fields).
focused-tests:
  - "{{test-suite-or-function}}"
hidden-lanes:
  - "{{hidden-id}}"
quality-commands:
  - "{{command}}"
dependencies:
  - "{{ticket-id}}"
status: "{{status}}"
# The human-readable mark you write when an authorized `rtm hold`
# pauses the Run, together with status "held". The Engine never reads it back.
# The short hash of the commit that made the ticket green (see the bundle's
# terms.md, "edition"). Filled in at closure; a ticket is passed by evidence,
# never by status.
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

Public coverage manifest entries. Category must map to one of the six adversarial lanes
(the bundle's schemas/ticket.toml lists them):
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
