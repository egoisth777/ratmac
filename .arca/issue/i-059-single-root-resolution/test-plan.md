# Issue test plan

## Verification

These are proposed checks, not executed results. Each check references the observable contract in [the specification](spec.md#requirement-records).

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| One call per route (`WEBV-017`) | `WEB-005` | A scripted resolver counts exactly one call for each single-project public command, human/structured doctor report, addressed status, and public library entry; repeated-root mutations fail the count even when both answers match. |
| Divergent resolver (`WEBV-018`) | `WEB-005` | Make a hypothetical second call return another runtime root or fail. The operation, report, refusal, and any allowed write all use the first root; no later call occurs. |
| Worktree and target scope (`WEBV-019`) | `WEB-005` | Exercise primary checkout, linked worktree, child workspace, no-Git fallback, addressed doctor/scaffold targets, and distinct explicit projects. Each distinct project resolves once; invoking runbook bytes remain local. |
| Interface guard (`WEBV-020`) | `WEB-005` | A compile-fail or visibility test prevents an internal context-bound handler from calling the discovery constructor. A focused scan may supplement, but never replaces, behavioral resolver-call proof. |

## Goal/Test File Traces

Integration was accepted on 2026-09-22; the table below records the authority changes. Product goal/design/tests and root-module interfaces need integration; dependent residue and nesting work should use the context established here.

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
