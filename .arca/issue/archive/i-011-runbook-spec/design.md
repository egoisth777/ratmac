# Issue design

## Proposed mechanics

Write the runbook spec as a section (or routed file) under `.arca/goal/` at P1 — exact home decided at
integration. Shape: (1) file format and machine-class schema, field by field; (2) guard-kind table — one row
per kind: semantics, required fields, forbidden fields, exemptions (e.g. pinned `command_exit`); (3)
ownership rules as a short contract (machine-owned vs agent-writable, consequence of violation); (4) a
back-reference table mapping spec statements to R-002/R-003/R-011. Source material: current parser and
scheduler behavior read from `src/`, not invented — where code and intended meaning disagree, the
disagreement becomes a requirement decision at P1, not a silent choice.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
