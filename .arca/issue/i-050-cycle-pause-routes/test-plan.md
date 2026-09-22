# Issue test plan

## Verification

All checks below are planned, not observed results.

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `WRSV-003-01` | [`WRS-003`](spec.md#requirement-records) | Drive a fresh copy of the shipped Runbook to each of the five working cycle States and three working ticket States; a correctly confirmed hold retains its address and State and records its blocker. |
| `WRSV-003-02` | [`WRS-003`](spec.md#requirement-records) | Check exactly one blocked route per working State and none on rest/done; doctor accepts the runbook and ordinary traversal still chooses only ordinary edges. |
| `WRSV-003-03` | [`WRS-003`](spec.md#requirement-records) | Unconfirmed hold, missing/outside blocker, and terminal hold refuse without changing runtime or workflow bytes. |
| `WRSV-003-04` | [`WRS-003`](spec.md#requirement-records) | Retain original evidence and pin bytes after each hold; prove a pre-change Run rejects drift rather than silently acquiring the new routes. |
| `WRSV-003-05` | [`WRS-003`](spec.md#requirement-records) | For parent and child States, provide exact human resolution authorization for the current Run and recorded blocker, with a valid WRA-001 signature under a governed policy, and satisfy existing entry prerequisites. The same address, class, State, pins, and prior receipts remain; executing status returns without advancing and history records the authorized resolution. A resumed parent can subsequently spawn a properly checked child, and a resumed child can later complete through fresh normal checks. |
| `WRSV-003-06` | [`WRS-003`](spec.md#requirement-records) | Missing, wrong, replayed, or differently bound resolution authorization; a missing or invalid governed signature; failed existing prerequisite; wrong class; non-held or terminal Run; stale plan; and changed pin each refuse with byte-identical records, receipts, ledgers, and history. Editing blocker prose or removing its file never substitutes for authorization. Identical valid authorization produces the same decision despite irrelevant blocker prose changes: the Engine does not adjudicate its external resolution. |
| `WRSV-003-07` | [`WRS-003`](spec.md#requirement-records) | Source changes while paused leave old completion receipts stale after authorized resume; completion refuses until fresh proof exists. An accepted rule-fix amendment alone leaves the Run blocked and cannot authorize resume or a transition. |
| `WRSV-003-08` | [`WRS-003`](spec.md#requirement-records) | Inject interruption before and after hold/resume journal publication and race competing motion. A mutating retry restores the old action or finishes the committed action exactly once; readers report pending reconciliation without repairing. History and pause occurrences are never rewritten, post-publication failure reports committed/recovery-pending honestly, and the public Run Record keeps its seven fields. |
| `WRSV-003-09` | [`WRS-003`](spec.md#requirement-records) | Hold, validly resume, and hold again at the identical State and blocker. The second occurrence strictly increases; the first ratmac-resume-v1 signature fails. Changing the held-record digest, context, or occurrence fails independently, as does using a review, approval, or recovery-purpose signature; a fresh exactly bound resume signature passes. |

## Existing test candidates

`test/qa/tests/t092_plan_build_runbook.rs`, `test/qa/tests/t093_stage_answer.rs`, `test/qa/tests/t087_no_work_item.rs`, and the corresponding private traversal lanes.

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
