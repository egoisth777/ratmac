# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WEB-002` | A documented, idempotent initialization command installs the Engine-root tracking policy before any Run can be admitted: runs/, mint.toml, locks/, and log.md beneath the resolved Engine root are ignored, while ratmac.toml and evidence/ remain eligible for tracking. Initialization preserves unrelated bytes and compatible operator rules, refuses an incompatible effective rule with the exact path and repair, and leaves no partial initialization on failure. Already-indexed runtime files and failures inspecting an available repository's index or effective ignore rules refuse without mutation; they are not treated as absent Git. Repeating it with the same effective policy is a byte-identical no-op. Linked worktrees protect the shared runtime root and their own tracked authoring surfaces. A fresh project's first start cannot create unprotected runtime; no command silently stages, commits, changes global Git configuration, or overwrites a runbook. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements) |
