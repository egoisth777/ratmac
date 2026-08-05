# Issue design

## Proposed mechanics

Retain the existing executable-selection path and hash the selected executable exactly as today; widen only
the argument-free human report’s rendered SHA-256 value from its abbreviated form to the complete digest.
Do not alter pin/trust handling, runtime-state reporting, Runbook diagnosis or findings, or the `--json`
path.

The regression calculates an independent SHA-256 over the exact test-built executable invoked by doctor and
compares that value with the rendered digest. It must not derive its expected value from the report helper or
accept a matching prefix. A before-and-after filesystem snapshot proves the invocation remains write-free.

The archived trial is evidence only, not an implementation source. The main-first implementation and its
regression are authored afresh; trial implementation bytes are not copied, merged, or cherry-picked.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
