# Issue test plan

## Verification

These are proposed checks, not executed results. Each check references the observable contract in [the specification](spec.md#requirement-records).

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| Public-route matrix (`WEBV-013`) | `WEB-004` | For start, status, step, hold, abandon, spawn, respawn, doctor, scaffold, skill, and any new operational verbs, each residue class defeats malformed options and returns its named repair. Library wrappers follow the same rule. Explicitly include initialization, recovery, review/authorization, and nested launch/receipt endpoints; newly exposed verbs fail the coverage check until included. |
| No operational observation (`WEBV-014`) | `WEB-004` | Instrument roster reads, runbook parse, blocker lookup, target checks, command execution, and writes. With residue, none runs after only necessary discovery/inspection; all file snapshots remain equal. |
| Addressed and ambiguous targets (`WEBV-015`) | `WEB-004` | Cover residue only at invoking root, shared root, and a valid explicit target; malformed extra options still report it first. Missing or ambiguous targets inspect only safely identified roots and then report usage. |
| Clean and help routes (`WEBV-016`) | `WEB-004` | Without residue ordinary valid behavior and argument errors remain, except the explicitly narrowed help rule. No arguments, a sole --help/-h, and recognized-command-plus-only-help stay pure help even with residue. Unknown commands stay write-free usage refusals. Help mixed with any other operational argument cannot skip preflight. |

## Goal/Test File Traces

Integration was accepted on 2026-09-22; the table below records the authority changes. Product goal and public entry-point test matrix need integration, with a precise exception replacing literally impossible before-any-read wording.

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/goal/spec.md` | updated | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/goal/design.md` | updated | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change in this intake bundle. |
| `.arca/schema.md` | unaffected | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/runbook-spec.md` | unaffected | Integration must update this authority if the accepted design adds or changes declared data. |
