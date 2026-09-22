# Issue test plan

## Verification

All checks below are planned, not observed results.

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `WRSV-005-01` | [`WRS-005`](spec.md#requirement-records) | List one top-level Run and two children with distinct bindings; each row contains the correct address, parent/class, binding, State, and status in stable order. |
| `WRSV-005-02` | [`WRS-005`](spec.md#requirement-records) | List retired, malformed, and missing-record entries and damaged ledgers; identify uncertainty rather than omit entries or invent top-level ownership. |
| `WRSV-005-03` | [`WRS-005`](spec.md#requirement-records) | Repeat the roster under runbook drift and from a linked checkout; derive the same Engine-owned identities without evaluating guards or writing files. |
| `WRSV-005-04` | [`WRS-005`](spec.md#requirement-records) | Missing/invalid addresses still fail and never select a sole Run; opaque binding control characters cannot forge extra rows. |

## Existing test candidates

`test/qa/tests/t059_run_residency.rs`, `test/qa/tests/t066_spawn_ledger.rs`, `test/qa/tests/t078_resolved_root_reporting.rs`, plus missing-address command fixtures.

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
