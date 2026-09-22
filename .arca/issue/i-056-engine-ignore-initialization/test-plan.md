# Issue test plan

## Verification

These are proposed checks, not executed results. Each check references the observable contract in [the specification](spec.md#requirement-records).

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| Clean initialization (`WEBV-005`) | `WEB-002` | Initialize an empty repository; git check-ignore confirms each runtime path is ignored and the runbook, evidence, and ignore-policy file are not. No runbook, Run, commit, or global configuration is created. |
| Repeat and operator rules (`WEBV-006`) | `WEB-002` | Repeat initialization twice and compare all bytes. Compatible root/local rules survive unchanged; conflicting blanket ignores, negations, already-tracked runtime, unreadable rules, and unwritable destination refuse with no partial files. Failed index/rule inspection in an available repository refuses; it is never downgraded to no-Git success. |
| Start and worktrees (`WEBV-007`) | `WEB-002` | A first start establishes protection before minting; a refused start restores prior bytes. Primary and linked checkouts share protected runtime while preserving each invoking runbook/evidence. A no-Git directory has a local policy without a false verified claim. |
| Interruption and concurrency (`WEBV-008`) | `WEB-002` | Fail the staged write and final replacement; compare before/after trees. Concurrent initialization yields one complete policy, never duplicates or lost operator content. |

## Goal/Test File Traces

Integration was accepted on 2026-09-22; the table below records the authority changes. Product goal, command help/operator skill, runtime layout, and the initialization guidance need integration; scaffold's existing one-file behavior stays explicit.

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
