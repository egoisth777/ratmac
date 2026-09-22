# Issue test plan

## Verification

All checks below are planned, not observed results.

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `WRSV-006-01` | [`WRS-006`](spec.md#requirement-records) | Finish a parent and child, then change, remove, and corrupt the runbook; status still reports each persisted passed Run with no changed prompt and no file mutation. |
| `WRSV-006-02` | [`WRS-006`](spec.md#requirement-records) | Record checksums of records, evidence, receipts, ledger abandoned flags, and logs before repeated status; every byte remains identical. |
| `WRSV-006-03` | [`WRS-006`](spec.md#requirement-records) | Perform the same drift on a nonterminal Run; current status/motion still refuses, while terminal step and hold remain prohibited. |
| `WRSV-006-04` | [`WRS-006`](spec.md#requirement-records) | Malformed Run Record, invalid address, unknown Run, and unreadable evidence produce honest named diagnostics without bypassing residue checks or implying verification. |

## Existing test candidates

`test/qa/tests/t060_runbook_pin.rs`, `test/qa/tests/t063_run_completion.rs`, `test/qa/tests/t069_child_reviewer.rs`, and command status rendering fixtures.

Select exact test ownership and independent private coverage when cutting tickets. Public contract assertions must detect the observed wrong behavior before implementation; private checks cover the changed boundary and its failure paths.

## Goal/Test File Traces

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
| `AGENTS.md` | unaffected | No change proposed. |
| `.arca/schema.md` | unaffected | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/runbook-spec.md` | unaffected | Any accepted format change must be specified before implementation. |
