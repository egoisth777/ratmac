# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WEB-005` | An Engine invocation resolves each distinct addressed project exactly once and carries that immutable result through residue checks, address selection, scheduler binding, workspace validation, command execution, and reports. The invoking checkout still supplies its own runbook; linked worktrees still share the primary runtime root. A context-bound handler cannot substitute a path-taking resolver or an independently rendered root. Public convenience entry points each establish one context and use context-taking internals. A test can count resolutions and make a second resolution return a different answer, proving every route uses the original value. Distinct explicitly addressed projects may each have one cached result; workspace membership checks must not re-resolve an already known Engine root. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements) |
