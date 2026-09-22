# Issue test plan

## Verification

All checks below are planned, not observed results.

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `WRSV-004-01` | [`WRS-004`](spec.md#requirement-records) | Reproduce an aged-record rule refusal, preserve the original Run and evidence, repair the rule through an approved amendment, then progress the same Run from its retained State using fresh checks. |
| `WRSV-004-02` | [`WRS-004`](spec.md#requirement-records) | Wrong Run/identity confirmation, absent authorization, changed class/workspace, missing State, terminal Run, and corrupt prior evidence each refuse unchanged. |
| `WRSV-004-03` | [`WRS-004`](spec.md#requirement-records) | Inject interruption before final amendment rename, after rename, and during history append. Temporary preparation leaves the old head effective; final publication selects the new validated head. The command reports committed/history-pending after publication; retry reconciles one missing event without a second amendment or duplicate history. Readers validate without repair, and original pins remain byte-identical. |
| `WRSV-004-04` | [`WRS-004`](spec.md#requirement-records) | Changed rule or source makes old receipts insufficient; forged approval text and replayed verdicts cannot advance the Run; a child retains its parent ledger ownership throughout. |
| `WRSV-004-05` | [`WRS-004`](spec.md#requirement-records) | A candidate that removes a failing guard or replaces reviewer keys cannot authorize itself through a phrase or signature under its new policy. An independently signed exact amendment under the prior policy is the positive control; omitted changes, substituted identities, and stale chain tips refuse unchanged. |
| `WRSV-004-06` | [`WRS-004`](spec.md#requirement-records) | A legacy Run lacking enrolled recovery authority refuses despite caller-supplied keys, generated signatures, or writable approval files. Independent external enrollment supplies the explicit prerequisite; recovery never enrolls its own approver and ordinary builder access cannot replace that enrollment. |
| `WRSV-004-07` | [`WRS-004`](spec.md#requirement-records) | A fully authorized amendment preserves blocked status, hold history, and original pins, and does not consume a verdict or create passing receipts. Separate resume and transition authorization are still required; stale proof from a removed or changed guard cannot count as current completion evidence. |
| `WRSV-004-08` | [`WRS-004`](spec.md#requirement-records) | Parse a two-entry version-1 amendment chain and reject each missing, unknown, or duplicate field, missing sequence, noncanonical filename, broken predecessor digest, changed original-pin digest, mismatched signed bytes, or unenumerated authority change. Temporary preparation is ignored, final malformed members refuse, and each successful motion uses only the complete latest verified head. |
| `WRSV-004-09` | [`WRS-004`](spec.md#requirement-records) | Keep source bytes identical while committing an authorized authority amendment. A receipt or approval bound to the previous effective head still refuses; fresh proof and separately authorized motion bound to the new head can pass. The older receipts, original pin bytes, and earlier amendments remain unchanged historical evidence. |

## Existing test candidates

New focused recovery integration suite using `test/qa/tests/t060_runbook_pin.rs`, archived-freeze fixtures in `test/qa/tests/t048_contract_gates.rs`, `test/qa/tests/t065_motion_authorization.rs`, and existing receipt freshness fixtures.

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
