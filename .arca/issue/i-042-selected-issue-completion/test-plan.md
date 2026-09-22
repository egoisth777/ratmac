# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `WCPV-007` | [WCP-003](spec.md#requirement-records) | An isolated self-development run cannot pass using the historic writable-marker shape, unrelated issue receipts, a missing red check, or stale green evidence. Candidate: extend t092_plan_build_runbook.rs with selected-issue binding fixtures. |
| `WCPV-008` | [WCP-003](spec.md#requirement-records) | A working agent's forged review record or wrong reviewer is refused; an authorized independent review bound to the tested snapshot can pass. Requires the reviewer-authority issue's real trust-boundary fixtures, not wording tests. |
| `WCPV-009` | [WCP-003](spec.md#requirement-records) | Changing source, issue selection, requirements, or check declarations after review invalidates completion; an authorized reviewed red/green sequence ends in an Engine-written passed fact. Candidate: completion and child-reviewer behavioral scenarios. |

These are planned checks, not results. Executable work enters its approved ticket before tests or implementation are authored.

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
| `AGENTS.md` | unaffected | No change during filing. |
| `.arca/schema.md` | unaffected | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/wishlist.md` | unaffected | Original wish retained unchanged until its complete desired end is proved. |
| `.ratmac/ratmac.toml` | unaffected | No active Run contract is edited during filing. |
