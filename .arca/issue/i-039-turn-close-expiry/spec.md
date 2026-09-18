# Issue specification

Disposition confirmed at the 2026-09-18 planning pass under Billy's explicit
request to fix ticket-close expiry first. The executable contract resolves
to the goal; the working rules carry the matching contributor procedure.
`TCE` means Turn-close expiry, defined in [ubi-lang.md](ubi-lang.md).

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| TCE-001 | After merge, verified lane copy-back, stamping, logging, and worktree and branch removal, this repository's ticket-close confirmation freshly checks every declared landed crate using the sweep's existing expiry rules. It succeeds only when every crate passes or carries a valid explicit expiry marker, reports those outcomes separately, and refuses an unexpired failure, missing or stray crate, malformed marker, or verification error. It never creates, renews, or removes an expiry marker. Existing per-lane declarations remain compatible. A failed final confirmation can be retried without repeating completed close mutations, with completed landing evidence required before a branchless retry. | accepted | The recovery turns stopped at deliberately expired crates even though the full sweep proved all current lanes green. Reuse the established policy instead of teaching the turn tool another marker parser. | [Turn-close expiry requirements](../../goal/spec.md#integrated-turn-close-expiry-requirements) |
