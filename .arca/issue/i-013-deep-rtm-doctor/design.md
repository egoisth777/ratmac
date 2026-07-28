# Issue design

## Proposed mechanics

Doctor calls the i-012 load path first; parse refusals become diagnostics rather than panics. On a successful
parse, run graph checks over the typed MachineClass (initial-phase uniqueness, reachability walk, dead-end
and duplicate-edge detection), then guard lint against the spec's guard-kind table (i-011/RBS-002), then
`ownership::audit_ownership`. Each finding is `{code, severity, location, message}`; `--json` emits the list
verbatim, human output formats the same list — one source of findings, two renderings. Exit codes proposed:
0 clean, 1 warnings only, 2 errors (parse refusal = error). Codes live in one table in the doctor module and
in the spec's diagnostics section, checked against each other by a test. Depends on i-011 (vocabulary) and
i-012 (parser); the authoring loop (i-014) consumes the `--json` shape.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
