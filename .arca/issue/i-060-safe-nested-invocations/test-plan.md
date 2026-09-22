# Issue test plan

## Verification

These are proposed checks, not executed results. Each check references the observable contract in [the specification](spec.md#requirement-records).

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| Independent success (`WEBV-021`) | `WEB-006` | Drive parent, child, and grandchild fixtures with distinct declared program/class identities and separate roots while all use the same pinned Engine executable. Only the selected current successful terminal receipt delivered directly by the child Engine endpoint permits parent progress; each runtime tree contains only its own records. |
| Receipt rejection (`WEBV-022`) | `WEB-006` | Exit zero without a receipt, writable success marker, forged wrapper output, replayed invocation, wrong Run/root/class/Engine, failed/held/nonterminal outcome, malformed/duplicate/partial receipt, and stale terminal evidence all refuse without parent transition. A wrapper printing a perfectly formed receipt and an agent-writable receipt file cannot substitute for the dedicated endpoint pipe. A driver cannot inherit or write that pipe; governed-child evidence must retain its authenticated policy binding. |
| Cycle and depth (`WEBV-023`) | `WEB-006` | Test direct self-call, A to B to A, path aliases, shared-root linked worktrees, exactly the permitted depth, one beyond it, malformed lineage, and truncated lineage. Refusal names the chain and occurs before child admission or process launch. Add missing inherited lineage, disagreement between protocol/environment lineage, and repeated declared program identity. Distinct programs sharing one Engine executable remain valid. |
| Recovery and coexistence (`WEBV-024`) | `WEB-006` | Interrupt after child admission, child terminal write, receipt delivery, and parent consumption. Retry is idempotent and cannot borrow an older result. Existing same-root spawn cap, root-before-Run locks, pin checks, and ordinary command_exit behavior remain valid. |

## Goal/Test File Traces

Integration was accepted on 2026-09-22; the table below records the authority changes. Product goal, root policy, Machine Class schema, operator guidance, receipt schema, and integration tests require coordinated integration; no child prototype source is imported.

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
