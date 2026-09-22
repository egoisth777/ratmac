# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `WCPV-004` | [WCP-002](spec.md#requirement-records) | Run migrated live lanes individually and through the same aggregate runner; every previous lane identifier and requirement remains represented. Candidate: extend t108_lane_sweep.rs and migration coverage inventories. |
| `WCPV-005` | [WCP-002](spec.md#requirement-records) | Run independent fixtures concurrently with conflicting environment values, working directories, and fault points; each sees only its own inputs and restores its files. Candidate: shared-runner behavioral tests in test/qa support. |
| `WCPV-006` | [WCP-002](spec.md#requirement-records) | Apply representative faults previously killed by the migrated lanes; the same assertions still fail, and expiry/history bytes remain unchanged. Compare pre/post lane inventories and archive snapshots; record sweep/build observations. |

These are planned checks, not results. Executable work enters its approved ticket before tests or implementation are authored.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../schema.md#wcp-002--shared-private-test-support). |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../schema.md#wcp-002--shared-private-test-support). |
| `.arca/goal/spec.md` | unaffected | [Accepted authority](../../schema.md#wcp-002--shared-private-test-support). |
| `.arca/goal/design.md` | updated | [Accepted authority](../../schema.md#wcp-002--shared-private-test-support). |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../schema.md#wcp-002--shared-private-test-support). |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change during filing. |
| `.arca/schema.md` | updated | [Accepted authority](../../schema.md#wcp-002--shared-private-test-support). |
| `.arca/wishlist.md` | unaffected | Original wish retained unchanged until its complete desired end is proved. |
| `.ratmac/ratmac.toml` | unaffected | No active Run contract is edited during filing. |
