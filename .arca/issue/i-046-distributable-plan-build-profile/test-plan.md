# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `WCPV-019` | [WCP-007](spec.md#requirement-records) | Export and initialize into a clean repository without .arca/schema.md or inherited prompt; drive intake, gap analysis, work-item checks, review, and proven completion from shipped instructions and Engine output. End-to-end fixture modeled on t092_plan_build_runbook.rs; inventory every opened dependency. |
| `WCPV-020` | [WCP-007](spec.md#requirement-records) | Remove or corrupt each required schema/template/checker and select an incompatible profile format; initialization or its first relevant guard refuses by artifact name without a false pass. Validate against the actual shipped manifest, not a separately invented test list. |
| `WCPV-021` | [WCP-007](spec.md#requirement-records) | Existing target files are preserved on collision or interrupted export; two exports of the same version contain the same bytes; runtime ownership/ignore boundaries remain valid. Reuse t104_operator_skill.rs atomic-write and non-overwrite test patterns. |

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
