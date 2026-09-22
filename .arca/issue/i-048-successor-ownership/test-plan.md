# Issue test plan

## Verification

All checks below are planned, not observed results.

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `WRSV-001-01` | [`WRS-001`](spec.md#requirement-records) | Pause replacement at every publication boundary and race abandon, spawn, step, hold, join, and roster reads; no child is usable or described as top-level before its ledger entry exists. |
| `WRSV-001-02` | [`WRS-001`](spec.md#requirement-records) | Inject ledger append, predecessor retirement, and restart failures; each outcome retains the original or one fully owned recoverable successor, never an orphan or lost proof. |
| `WRSV-001-03` | [`WRS-001`](spec.md#requirement-records) | Repeat recovery and race two replacements of one predecessor; no duplicate live replacement, reused identifier, or deadlock. |
| `WRSV-001-04` | [`WRS-001`](spec.md#requirement-records) | Keep inherited workspace, class, bindings, superseded link, confirmation refusal, and root-lock-free guard execution tests green. |

## Existing test candidates

`test/qa/tests/t065_motion_authorization.rs`, `test/qa/tests/t066_spawn_ledger.rs`, `test/qa/tests/t074_child_workspace.rs`; use the existing fault-injection test feature for deterministic process races.

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
