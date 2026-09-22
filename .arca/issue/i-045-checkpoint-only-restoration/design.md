# Issue design

## Proposed mechanics

Reuse turn ownership and declared roots. Bind the current item's reviewed source manifest to its safety commit and fail if the selected checkpoint does not contain that exact green snapshot. The restore verb consumes this recorded checkpoint, never a caller-supplied backup path or revision. Validate all targets and preserve unrelated work before touching any file; invoke Git restore with the explicit checkpoint for both index and worktree. Then invoke the byte-verification contract. Recovery reports the same checkpoint and exact allowed command without copy-back fallbacks.

The mechanics implement [WCP-006](spec.md#requirement-records) and are tested by the [verification plan](test-plan.md#verification).

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.

