# Issue test plan

## Verification

All checks below are planned, not observed results.

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `WRSV-002-01` | [`WRS-002`](spec.md#requirement-records) | Hold a child through a route declared only in its class and read back its blocked status, route destination, and blocker. |
| `WRSV-002-02` | [`WRS-002`](spec.md#requirement-records) | Give parent, child, and sibling identical State names with different destinations. Inspect the plan's child destination, then apply and read the child record; independently replace either lookup with the parent's graph and require this check to fail. |
| `WRSV-002-03` | [`WRS-002`](spec.md#requirement-records) | A child lacking a route refuses despite a same-named parent's route during planning and when applying a forged public plan. Malformed ownership, stale plans, and unconfirmed requests leave all files unchanged. |
| `WRSV-002-04` | [`WRS-002`](spec.md#requirement-records) | Use a child workspace distinct from the invoking checkout and prove blocker containment there; preserve terminal refusal and no workflow-file writes. |

## Existing test candidates

`test/qa/tests/t087_no_work_item.rs`, `test/qa/tests/t069_child_reviewer.rs`, `test/qa/tests/t074_child_workspace.rs`, and the current private t-090 observation.

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
