# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `WCPV-010` | [WCP-004](spec.md#requirement-records) | A review rejection invokes no expensive proof command; repair and approval precede the first private-lane command. Behavioral command trace in a minimal workflow fixture, not only document scanning. |
| `WCPV-011` | [WCP-004](spec.md#requirement-records) | One unchanged approved snapshot earns its private/damage proof set once after review while still executing every required restored-green rerun and declared post-merge verification from main. Instrument separate proof-earning, restored-green, and main-verification events; removing any mandatory rerun must fail this check. |
| `WCPV-012` | [WCP-004](spec.md#requirement-records) | An edit after review or proof invalidates the corresponding evidence and requires a renewed review/proof; missing private lanes still refuse. Reuse snapshot and completion-freshness fixture machinery. |

These are planned checks, not results. Executable work enters its approved ticket before tests or implementation are authored.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../schema.md#wcp-004--review-before-proofs). |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../schema.md#wcp-004--review-before-proofs). |
| `.arca/goal/spec.md` | unaffected | [Accepted authority](../../schema.md#wcp-004--review-before-proofs). |
| `.arca/goal/design.md` | updated | [Accepted authority](../../schema.md#wcp-004--review-before-proofs). |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../schema.md#wcp-004--review-before-proofs). |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change during filing. |
| `.arca/schema.md` | updated | [Accepted authority](../../schema.md#wcp-004--review-before-proofs). |
| `.arca/wishlist.md` | unaffected | Original wish retained unchanged until its complete desired end is proved. |
| `.ratmac/ratmac.toml` | unaffected | No active Run contract is edited during filing. |
