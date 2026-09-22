# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| WRAV-001 | [WRA-001](spec.md#requirement-records) | An allowed independent group review plus its Approver authorizes exactly one fresh normal transition and one blocked transition. |
| WRAV-002 | [WRA-001](spec.md#requirement-records) | Builder-only review, an outsider, a non-approving member, a forged signer field, a missing signature, and an insufficient count all refuse with state and evidence byte-identical. |
| WRAV-003 | [WRA-001](spec.md#requirement-records) | Changes to work, policy, Run, State, target, selected input, or sequence invalidate the same otherwise-valid approval. |
| WRAV-004 | [WRA-001](spec.md#requirement-records) | Replaying consumed approval and interrupting at each evidence/state boundary never authorizes a second transition or loses accepted history. |
| WRAV-005 | [WRA-001](spec.md#requirement-records) | Malformed policy refuses at parsing/diagnosis; historical Runs retain truthful no-policy provenance; live governed workflow refuses without its independent trust anchor. |
| WRAV-006 | [WRA-001](spec.md#requirement-records) | The production signature verifier accepts independently enrolled fixture identities but rejects self-enrollment, a builder key under a second label, the builder acting as Approver after an outside review, a review signature reused as approval, a substituted policy or verifier, and an authority amendment signed only by its proposed replacement authority. |

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
| `AGENTS.md` | unaffected | Existing working-rule entry point remains. |
| `.arca/schema.md` | unaffected | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
